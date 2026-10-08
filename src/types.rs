//! Shared type representation and annotation parsing.
//!
//! Aura's contract promises that type annotations are checked before
//! execution. This module provides a *sound* checker: it only reports a
//! mismatch when it is certain about both the expected and the inferred type.
//! When it cannot infer a type, it infers `Unknown`, which is compatible with
//! everything. This means the checker never produces false positives; it
//! reports exactly the mismatches it can prove.
//!
//! It is deliberately not a full inference engine. The point is to make the
//! documented guarantee real for the cases a user can see, without inventing
//! a type system the language did not specify.

use crate::ast::*;
use crate::error::{codes, Diag, Result, Span};

/// The capability family a semantic type belongs to (`LANGUAGE_SPEC.md` §5.4).
///
/// A family answers "what operations are valid here", never "what is this":
/// `[int]` and `string` are both `Sequence` and therefore both iterable and
/// indexable, but they stay distinct semantic types and distinct runtime
/// values. The family is a *projection* of a type, not a replacement for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TypeFamily {
    /// `int`, `float`, `bool`, `string`.
    Scalar,
    /// `[T]`, `string` — capabilities shared by list and string iteration.
    Sequence,
    /// `{K: V}`.
    Mapping,
    /// `{T}` — unordered membership.
    SetLike,
    /// A nominal struct.
    Object,
    /// An enum, a union, `never`, or `none` (an inhabitant-set type).
    Sum,
    /// A function value.
    Callable,
    /// `none`.
    Nullish,
    /// A runtime-only range; it has no annotation spelling.
    RangeLike,
    /// The checker does not know; no family is claimed.
    Unknown,
}

impl TypeFamily {
    /// The stable protocol spelling of this family.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            TypeFamily::Scalar => "scalar",
            TypeFamily::Sequence => "sequence",
            TypeFamily::Mapping => "mapping",
            TypeFamily::SetLike => "set_like",
            TypeFamily::Object => "object",
            TypeFamily::Sum => "sum",
            TypeFamily::Callable => "callable",
            TypeFamily::Nullish => "nullish",
            TypeFamily::RangeLike => "range_like",
            TypeFamily::Unknown => "unknown",
        }
    }
}

/// A type known to the checker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ty {
    /// `int`
    Int,
    /// `float`
    Float,
    /// `bool`
    Bool,
    /// `string`
    String,
    /// `[T]`
    List(Box<Ty>),
    /// `[T; N]` — a fixed-length array of `N` elements each of type `T`. The
    /// length is part of the type identity: `[int; 2]` and `[int; 3]` are
    /// different types, with no length covariance.
    Array(Box<Ty>, u64),
    /// `(T1, T2, ...)` — a fixed-length heterogeneous tuple. Always two or
    /// more members (a one-member `(T,)` is normalized to its member).
    Tuple(Vec<Ty>),
    /// `{T}` — an unordered set of `T` (a `SetLike` capability).
    Set(Box<Ty>),
    /// `{K: V}` — a map from key type `K` to value type `V`. `K` is a
    /// key-capable scalar type (`string`, `int`, `bool`), a union of such
    /// types, a generic parameter that must become key-capable when
    /// instantiated, or `Unknown`.
    Map(Box<Ty>, Box<Ty>),
    /// A named user type.
    Named(String),
    /// An enum value, by enum type name.
    Enum(String),
    /// `never` — the bottom type. No value can result from a `never`
    /// expression: a `return`, `throw`, `break`, or `continue` path, or a
    /// function declared `-> never` that does not return normally. `never`
    /// is assignable to every type and compatible with every type.
    Never,
    /// `none` — the absence value's type. `none` appears as a member of an
    /// optional union (`T | none`); a union containing it stays permissive
    /// for assignment (the historical `T | none` behavior), but the member
    /// is retained so flow narrowing can remove it inside a `!= none` guard
    /// and so an unguarded member access can be diagnosed precisely.
    None,
    /// `T1 | T2 | ...` — a union of two or more distinct members. Stored in a
    /// canonical, flattened, deduplicated form ([`Ty::union`]); a union with
    /// `none` (or any `Unknown` member) collapses to [`Ty::Unknown`], which is
    /// the documented permissive behavior of `T | none`.
    Union(Vec<Ty>),
    /// A generic type parameter in scope, identified by name. Two parameters
    /// with the same name in the same declaration are the same parameter. A
    /// `Param` is only meaningful inside a generic declaration; substitution
    /// replaces it with the bound concrete type before any comparison.
    Param(String),
    /// `Name<A1, A2, ...>` — a parameterised nominal type application. The
    /// name is a declared generic struct, enum, or alias; the arguments are
    /// the type arguments (which may themselves be `Param` or nested `App`).
    /// `Named(n)` is the zero-argument form and the two are kept distinct only
    /// where arity matters.
    App(String, Vec<Ty>),
    /// Anything the checker does not know; compatible with all types.
    Unknown,
}

impl Ty {
    /// The source spelling of this type.
    #[must_use]
    pub fn name(&self) -> String {
        match self {
            Ty::Int => "int".into(),
            Ty::Float => "float".into(),
            Ty::Bool => "bool".into(),
            Ty::String => "string".into(),
            Ty::List(t) => format!("[{}]", t.name()),
            Ty::Array(t, n) => format!("[{}; {}]", t.name(), n),
            Ty::Tuple(ts) if ts.len() == 1 => format!("({},)", ts[0].name()),
            Ty::Tuple(ts) => format!(
                "({})",
                ts.iter().map(Ty::name).collect::<Vec<_>>().join(", ")
            ),
            Ty::Set(t) => format!("{{{}}}", t.name()),
            Ty::Map(k, v) => format!("{{{}: {}}}", k.name(), v.name()),
            Ty::Named(n) => n.clone(),
            Ty::Enum(n) => n.clone(),
            Ty::Union(ms) => ms.iter().map(Ty::name).collect::<Vec<_>>().join(" | "),
            Ty::Param(n) => n.clone(),
            Ty::App(n, args) => format!(
                "{}<{}>",
                n,
                args.iter().map(Ty::name).collect::<Vec<_>>().join(", ")
            ),
            Ty::Never => "never".into(),
            Ty::None => "none".into(),
            Ty::Unknown => "unknown".into(),
        }
    }

    /// The nominal head of this type: the declared name of a user type, with
    /// any type arguments discarded. `Box<int>` and `Box<string>` share the
    /// head `Box`; a primitive has no nominal head.
    #[must_use]
    pub fn nominal_head(&self) -> Option<String> {
        match self {
            Ty::Named(n) | Ty::Enum(n) => Some(n.clone()),
            Ty::App(n, _) => Some(n.clone()),
            _ => None,
        }
    }

    /// Whether this type is `none` or a union that contains `none`.
    #[must_use]
    pub fn contains_none(&self) -> bool {
        match self {
            Ty::None => true,
            Ty::Union(ms) => ms.iter().any(Ty::contains_none),
            _ => false,
        }
    }

    /// This type with the `none` member removed: the narrowed type inside a
    /// `!= none` guard. Removing it from a single-member union is the member
    /// itself; removing the last member yields `never` (the branch cannot be
    /// reached). A type without `none` is returned unchanged.
    #[must_use]
    pub fn without_none(&self) -> Ty {
        match self {
            Ty::None => Ty::Never,
            Ty::Union(ms) => Ty::union(
                ms.iter()
                    .filter(|m| !matches!(m, Ty::None))
                    .cloned()
                    .collect(),
            ),
            other => other.clone(),
        }
    }

    /// Whether this type mentions a generic parameter anywhere.
    #[must_use]
    pub fn has_param(&self) -> bool {
        match self {
            Ty::Param(_) => true,
            Ty::List(t) | Ty::Set(t) => t.has_param(),
            Ty::Array(t, _) => t.has_param(),
            Ty::Tuple(ts) => ts.iter().any(Ty::has_param),
            Ty::Map(k, v) => k.has_param() || v.has_param(),
            Ty::Union(ms) => ms.iter().any(Ty::has_param),
            Ty::App(_, args) => args.iter().any(Ty::has_param),
            _ => false,
        }
    }

    /// Substitute generic parameters by name, returning the type with every
    /// [`Ty::Param`] replaced by its binding. A parameter with no binding is
    /// left as-is; callers that must not retain a parameter (a value position)
    /// map the residue to `Unknown` via [`Ty::erase_params`].
    #[must_use]
    pub fn substitute(&self, sigma: &std::collections::HashMap<String, Ty>) -> Ty {
        match self {
            Ty::Param(n) => sigma
                .get(n)
                .cloned()
                .unwrap_or_else(|| Ty::Param(n.clone())),
            Ty::List(t) => Ty::List(Box::new(t.substitute(sigma))),
            Ty::Array(t, n) => Ty::Array(Box::new(t.substitute(sigma)), *n),
            Ty::Tuple(ts) => Ty::Tuple(ts.iter().map(|t| t.substitute(sigma)).collect()),
            Ty::Set(t) => Ty::Set(Box::new(t.substitute(sigma))),
            Ty::Map(k, v) => Ty::Map(Box::new(k.substitute(sigma)), Box::new(v.substitute(sigma))),
            Ty::Union(ms) => Ty::union(ms.iter().map(|m| m.substitute(sigma)).collect()),
            Ty::App(n, args) => Ty::App(
                n.clone(),
                args.iter().map(|a| a.substitute(sigma)).collect(),
            ),
            other => other.clone(),
        }
    }

    /// Replace every unbound generic parameter with `Unknown`, so a type that
    /// reaches a value position never retains a parameter.
    #[must_use]
    pub fn erase_params(&self, sigma: &std::collections::HashMap<String, Ty>) -> Ty {
        match self {
            Ty::Param(n) => sigma.get(n).cloned().unwrap_or(Ty::Unknown),
            Ty::List(t) => Ty::List(Box::new(t.erase_params(sigma))),
            Ty::Array(t, n) => Ty::Array(Box::new(t.erase_params(sigma)), *n),
            Ty::Tuple(ts) => Ty::Tuple(ts.iter().map(|t| t.erase_params(sigma)).collect()),
            Ty::Set(t) => Ty::Set(Box::new(t.erase_params(sigma))),
            Ty::Map(k, v) => Ty::Map(
                Box::new(k.erase_params(sigma)),
                Box::new(v.erase_params(sigma)),
            ),
            Ty::Union(ms) => Ty::union(ms.iter().map(|m| m.erase_params(sigma)).collect()),
            Ty::App(n, args) => Ty::App(
                n.clone(),
                args.iter().map(|a| a.erase_params(sigma)).collect(),
            ),
            other => other.clone(),
        }
    }

    /// Build a normalized tuple type from members. Arity is preserved exactly,
    /// including a one-element tuple `(T,)`, which stays a distinct `Tuple`
    /// identity (it is indexable and iterable, unlike a bare `T`). Only an
    /// empty member list is degenerate and maps to `Unknown`.
    #[must_use]
    pub fn tuple(members: Vec<Ty>) -> Ty {
        if members.is_empty() {
            Ty::Unknown
        } else {
            Ty::Tuple(members)
        }
    }

    /// Build a normalized union type from members.
    ///
    /// Normalization is deterministic and total: nested unions are flattened,
    /// duplicate members are removed, and the result is sorted into a
    /// canonical order so that `int | float` and `float | int` are the same
    /// type. A union with a single remaining member is that member. Because
    /// `none` has no static type (it is the permissive [`Ty::Unknown`]), a
    /// union containing it collapses to `Unknown` — the existing, documented
    /// behavior of `T | none`, generalized.
    #[must_use]
    pub fn union(members: Vec<Ty>) -> Ty {
        let mut flat = Vec::new();
        for m in members {
            flatten_into(m, &mut flat);
        }
        // An unresolved member (`Unknown`) still makes the union permissive,
        // exactly as it has always been: collapse immediately so the
        // representation never carries `Unknown` as a member. `none` is
        // different: it is a known member and is retained for narrowing.
        if flat.iter().any(|m| matches!(m, Ty::Unknown)) {
            return Ty::Unknown;
        }
        // `never` is the bottom type and is absorbed by any union (§4.3):
        // there is no value of `int | never` that is not an `int`. A union
        // of only `never` members is `never`.
        flat.retain(|m| !matches!(m, Ty::Never));
        if flat.is_empty() {
            return Ty::Never;
        }
        // Deduplicate, then sort into a canonical, human-friendly order:
        // primitives first in their declaration order, then compounds and
        // named types by spelling. This makes `int | float` and `float | int`
        // the same type and keeps the common unions spelled as written.
        flat.sort_by_cached_key(|t| (t.rank(), t.name()));
        flat.dedup_by(|a, b| a == b);
        match flat.len() {
            0 => Ty::Unknown,
            1 => flat.pop().unwrap_or(Ty::Unknown),
            _ => Ty::Union(flat),
        }
    }

    /// A stable canonical sort rank for primitive types, so a normalized
    /// union reads in declaration order.
    fn rank(&self) -> u8 {
        match self {
            Ty::Int => 0,
            Ty::Float => 1,
            Ty::Bool => 2,
            Ty::String => 3,
            Ty::List(_) => 4,
            Ty::Array(_, _) => 4,
            Ty::Tuple(_) => 4,
            Ty::Set(_) => 4,
            Ty::Map(_, _) => 5,
            Ty::Named(_) | Ty::Enum(_) => 6,
            Ty::Union(_) => 7,
            Ty::Param(_) => 8,
            Ty::App(_, _) => 9,
            Ty::Never => 10,
            Ty::None => 11,
            Ty::Unknown => 12,
        }
    }

    /// Whether this type is compatible with `other` as an assignment target.
    /// `Unknown` is compatible with everything; `int` and `float` are
    /// *not* interchangeable (the contract forbids implicit coercion).
    ///
    /// `self` is the expected type, `other` the actual. A union accepts an
    /// actual value when some member accepts it; an actual union satisfies an
    /// expected type only when every member does.
    #[must_use]
    pub fn compatible_with(&self, other: &Ty) -> bool {
        if matches!(self, Ty::Unknown) || matches!(other, Ty::Unknown) {
            return true;
        }
        // A generic parameter is compatible with anything at the boundary where
        // it appears as the *expected* type: the caller's actual type is what
        // binds it. As the *actual* type it is likewise permissive, because the
        // caller supplies its binding. Substitution has normally removed it
        // before comparison; this keeps an unresolved parameter from making a
        // sound `false` decision.
        if matches!(self, Ty::Param(_)) || matches!(other, Ty::Param(_)) {
            return true;
        }
        // Optionality (`T | none`) is a real type relation, not a
        // permissiveness escape hatch (§4.3, §5.2): `T` is assignable to
        // `T | none` and `none` is assignable to `T | none`, but a `T | none`
        // value is **not** assignable to `T` without narrowing. A `none`
        // expectation accepts only `none`; the bottom `never` is handled by
        // the rules below. Widening to a union expectation is handled by the
        // union arms of the structural match.
        if matches!(self, Ty::None) {
            return matches!(other, Ty::None | Ty::Never);
        }
        // `never` is the bottom type (§4.3): a `never` value is acceptable
        // wherever any type is expected, and only a `never` (or an `Unknown`)
        // value satisfies a `never` expectation.
        if matches!(self, Ty::Never) {
            return matches!(other, Ty::Never | Ty::Unknown);
        }
        if matches!(other, Ty::Never) {
            return true;
        }
        match (self, other) {
            (Ty::Union(expected), Ty::Union(actual)) => actual
                .iter()
                .all(|a| expected.iter().any(|e| e.compatible_with(a))),
            (Ty::Union(expected), actual) => expected.iter().any(|e| e.compatible_with(actual)),
            (expected, Ty::Union(actual)) => actual.iter().all(|a| expected.compatible_with(a)),
            (Ty::List(a), Ty::List(b)) => a.compatible_with(b),
            // An Array is compatible only with an Array of the same length and
            // a compatible element type. There is no implicit List→Array
            // conversion and no length covariance (§11–§18).
            (Ty::Array(a, n), Ty::Array(b, m)) => n == m && a.compatible_with(b),
            // A Tuple is compatible with a Tuple of the same arity and
            // element-wise compatible member types.
            (Ty::Tuple(a), Ty::Tuple(b)) => {
                a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.compatible_with(y))
            }
            // A Set is compatible with a Set of a compatible element type.
            (Ty::Set(a), Ty::Set(b)) => a.compatible_with(b),
            // Map key and value are both invariant under this conservative
            // relation: a map is assignable only when its key type is
            // compatible in both directions (equal up to `Unknown`) and its
            // value type is compatible. Without key-level agreement a
            // `{string: V}` value could be read as an `{int: V}` and indexed
            // with the wrong key kind, so the key is checked, not ignored.
            (Ty::Map(ek, ev), Ty::Map(ak, av)) => ek.compatible_with(ak) && ev.compatible_with(av),
            // A generic application matches an application with the same
            // nominal head and arity, element-wise. A bare `Named` matches an
            // `App` of the same head only when the application has no
            // arguments, so a wrong arity is not silently accepted here.
            (Ty::App(a, xa), Ty::App(b, xb)) => {
                a == b
                    && xa.len() == xb.len()
                    && xa.iter().zip(xb).all(|(x, y)| x.compatible_with(y))
            }
            (Ty::Named(a), Ty::App(b, xb)) => a == b && xb.is_empty(),
            (Ty::App(a, xa), Ty::Named(b)) => a == b && xa.is_empty(),
            _ => self == other,
        }
    }

    /// Whether this type may be used as a map key.
    ///
    /// A key must have deterministic, stable identity compatible with Aura's
    /// equality: it must be a key-capable scalar (`string`, `int`, `bool`), a
    /// union whose every member is key-capable, or a placeholder that the
    /// checker cannot yet pin down (`Unknown`, or a generic `Param` that must
    /// itself resolve to a key-capable type when instantiated). `float` is not
    /// key-capable: `NaN` has no total order, `0.0 == -0.0` holds while their
    /// bit patterns differ, and Aura's cross-type equality relates `1` and
    /// `1.0`, so no float key can satisfy "equal keys are the same key".
    /// Containers, structs, enums, and functions are not key-capable: they
    /// have no total structural order, may be mutable, and may be cyclic.
    #[must_use]
    pub fn is_key_capable(&self) -> bool {
        match self {
            Ty::Int | Ty::Bool | Ty::String | Ty::Unknown | Ty::Param(_) => true,
            // `never` is vacuously key-capable: no value can exist to key a map.
            Ty::Never => true,
            Ty::Union(members) => members.iter().all(Ty::is_key_capable),
            _ => false,
        }
    }

    /// The diagnostic for a type that cannot be a map key.
    #[must_use]
    pub fn key_type_error(&self, span: Span) -> Diag {
        Diag::new(
            codes::TYPE_MISMATCH,
            format!(
                "type `{}` cannot be used as a map key; map keys must be `string`, `int`, or `bool`",
                self.name()
            ),
            span,
        )
    }

    /// The coarse built-in type class of this type, or `None` when it is
    /// `Unknown`, a union, or a user type with no method table.
    #[must_use]
    pub fn type_class(&self) -> Option<crate::stdlib::signatures::TypeClass> {
        use crate::stdlib::signatures::TypeClass;
        Some(match self {
            Ty::Int => TypeClass::Int,
            Ty::Float => TypeClass::Float,
            Ty::Bool => TypeClass::Bool,
            Ty::String => TypeClass::Str,
            Ty::List(_) => TypeClass::List,
            Ty::Array(_, _) => TypeClass::Array,
            Ty::Tuple(_) => TypeClass::Tuple,
            Ty::Set(_) => TypeClass::Set,
            Ty::Map(_, _) => TypeClass::Map,
            Ty::Named(_) | Ty::Enum(_) | Ty::Union(_) | Ty::Unknown => return None,
            Ty::Param(_) | Ty::App(_, _) | Ty::Never | Ty::None => return None,
        })
    }

    /// The capability families of this type (`LANGUAGE_SPEC.md` §5.4).
    ///
    /// This is a *projection*: it classifies which operations the type
    /// supports, and never replaces the semantic type. Two types in one family
    /// (a list and a string; a struct and a string-keyed map) remain distinct
    /// types with distinct equality, display, and JSON behavior. A type may
    /// belong to more than one family: a `string` is `Scalar` and `Sequence`.
    ///
    /// `App` is a parameterized nominal application whose declaration decides
    /// whether it is an object or a sum; without the declaration registry the
    /// family is not claimed. `Never` is the empty sum, `none` the nullish
    /// unit, and `Union` an untagged sum.
    #[must_use]
    pub fn families(&self) -> &'static [TypeFamily] {
        match self {
            Ty::Int | Ty::Float | Ty::Bool => &[TypeFamily::Scalar],
            Ty::String => &[TypeFamily::Scalar, TypeFamily::Sequence],
            Ty::List(_) => &[TypeFamily::Sequence],
            Ty::Array(_, _) => &[TypeFamily::Sequence],
            Ty::Tuple(_) => &[TypeFamily::Sequence],
            Ty::Set(_) => &[TypeFamily::SetLike],
            Ty::Map(_, _) => &[TypeFamily::Mapping],
            // A range is the dynamic `Ty::Named("range")` marker; it is a
            // `RangeLike` capability, never a nominal `Object` (§5.4).
            Ty::Named(n) if n == "range" => &[TypeFamily::RangeLike],
            Ty::Named(_) => &[TypeFamily::Object],
            Ty::Enum(_) | Ty::Union(_) | Ty::Never => &[TypeFamily::Sum],
            Ty::None => &[TypeFamily::Nullish],
            Ty::Param(_) | Ty::App(_, _) | Ty::Unknown => &[TypeFamily::Unknown],
        }
    }

    /// The primary capability family, the first of [`Ty::families`].
    #[must_use]
    pub fn family(&self) -> TypeFamily {
        self.families()
            .first()
            .copied()
            .unwrap_or(TypeFamily::Unknown)
    }

    /// The canonical runtime value kind a value of this type has
    /// (`LANGUAGE_SPEC.md` §5.4 level 3), or `"unknown"` when the static type
    /// does not pin one kind (a union, a generic parameter, an unresolved
    /// application, the checker's `Unknown`).
    #[must_use]
    pub fn value_kind(&self) -> &'static str {
        match self {
            Ty::Int => "int",
            Ty::Float => "float",
            Ty::Bool => "bool",
            Ty::String => "string",
            Ty::List(_) => "list",
            Ty::Array(_, _) => "array",
            Ty::Tuple(_) => "tuple",
            Ty::Set(_) => "set",
            Ty::Map(_, _) => "map",
            Ty::Named(n) if n == "range" => "range",
            Ty::Named(_) => "struct",
            Ty::Enum(_) => "enum",
            Ty::None => "none",
            Ty::Never => "never",
            Ty::Union(_) => "union",
            Ty::Param(_) | Ty::App(_, _) | Ty::Unknown => "unknown",
        }
    }

    /// Whether two types can be ordered with `<`, `<=`, `>`, `>=`.
    ///
    /// This mirrors `Value::comparable_with`: only numeric-numeric, string,
    /// and bool pairs are orderable. Returns `Some(false)` only when it can
    /// prove the comparison is a type error; `None` when a side is `Unknown`
    /// or a union the checker cannot pin to one class.
    #[must_use]
    pub fn orderable_with(&self, other: &Ty) -> Option<bool> {
        if matches!(
            self,
            Ty::Unknown | Ty::Union(_) | Ty::Param(_) | Ty::App(_, _) | Ty::Never
        ) || matches!(
            other,
            Ty::Unknown | Ty::Union(_) | Ty::Param(_) | Ty::App(_, _) | Ty::Never
        ) {
            return None;
        }
        Some(matches!(
            (self, other),
            (Ty::Int, Ty::Int)
                | (Ty::Float, Ty::Float)
                | (Ty::Int, Ty::Float)
                | (Ty::Float, Ty::Int)
                | (Ty::String, Ty::String)
                | (Ty::Bool, Ty::Bool)
        ))
    }

    /// Convert a written type expression to a checker type, validating that
    /// named types exist.
    ///
    /// # Errors
    /// Returns `E3002` for an unknown type name.
    pub fn from_expr(
        t: &TypeExpr,
        types: &std::collections::HashMap<String, String>,
        span: Span,
    ) -> Result<Ty> {
        Ok(match t {
            TypeExpr::Int => Ty::Int,
            TypeExpr::Float => Ty::Float,
            TypeExpr::Bool => Ty::Bool,
            TypeExpr::String => Ty::String,
            // `none` is the absence type: a member of an optional union
            // (`T | none`). It is retained so flow narrowing can remove it
            // and access checks can diagnose a possible `none` precisely
            // (§4.3, §5.2).
            TypeExpr::None => Ty::None,
            // `never` is the bottom type (§4.3): no value can result.
            TypeExpr::Never => Ty::Never,
            TypeExpr::List(inner) => Ty::List(Box::new(Ty::from_expr(inner, types, span)?)),
            TypeExpr::Array(inner, n) => {
                Ty::Array(Box::new(Ty::from_expr(inner, types, span)?), *n)
            }
            TypeExpr::Tuple(members) => {
                let mut tys = Vec::with_capacity(members.len());
                for m in members {
                    tys.push(Ty::from_expr(m, types, span)?);
                }
                Ty::tuple(tys)
            }
            TypeExpr::Set(inner) => {
                let elem = Ty::from_expr(inner, types, span)?;
                if !elem.is_key_capable() {
                    return Err(elem.key_type_error(span));
                }
                Ty::Set(Box::new(elem))
            }
            TypeExpr::Map(k, v) => {
                let key = Ty::from_expr(k, types, span)?;
                if !key.is_key_capable() {
                    return Err(key.key_type_error(span));
                }
                Ty::Map(Box::new(key), Box::new(Ty::from_expr(v, types, span)?))
            }
            TypeExpr::Union(members) => {
                let mut tys = Vec::with_capacity(members.len());
                for m in members {
                    tys.push(Ty::from_expr(m, types, span)?);
                }
                Ty::union(tys)
            }
            TypeExpr::Named(n) => match types.get(n).map(String::as_str) {
                Some("enum") => Ty::Enum(n.clone()),
                Some(_) => Ty::Named(n.clone()),
                None => {
                    return Err(Diag::new(
                        codes::UNKNOWN_TYPE,
                        format!("unknown type `{n}`"),
                        span,
                    ))
                }
            },
            TypeExpr::App(n, args) => {
                match types.get(n).map(String::as_str) {
                    Some("enum") | Some("struct") | Some("alias") => {}
                    // A generic application to a parameter name (`T<int>`) or
                    // an unknown head is a type error here; the checker resolves
                    // parameters and reports the precise diagnostic.
                    Some(_) => {}
                    None => {
                        return Err(Diag::new(
                            codes::UNKNOWN_TYPE,
                            format!("unknown type `{n}`"),
                            span,
                        ))
                    }
                }
                let mut tys = Vec::with_capacity(args.len());
                for a in args {
                    tys.push(Ty::from_expr(a, types, span)?);
                }
                Ty::App(n.clone(), tys)
            }
        })
    }

    /// Build a runtime overload-selection type from a written annotation,
    /// erasing generic type parameters to `Unknown`. The runtime carries no
    /// static element type and a parameter is a compile-time placeholder, so a
    /// parameter position accepts any runtime value (`LANGUAGE_SPEC.md` §36).
    #[must_use]
    pub fn from_expr_erased(t: &TypeExpr, params: &[String]) -> Ty {
        let mut ty = Ty::from_expr_lenient(t);
        ty.erase_named_params(params);
        ty
    }

    /// Replace every named generic parameter (a bare `Named` whose spelling is
    /// in `params`) with `Unknown`, recursively. The lenient conversion records
    /// a parameter as a `Named`, so erasure works on that spelling.
    fn erase_named_params(&mut self, params: &[String]) {
        match self {
            Ty::Named(n) if params.iter().any(|p| p == n) => *self = Ty::Unknown,
            Ty::List(t) | Ty::Set(t) => t.erase_named_params(params),
            Ty::Array(t, _) => t.erase_named_params(params),
            Ty::Tuple(ts) => {
                for t in ts.iter_mut() {
                    t.erase_named_params(params);
                }
            }
            Ty::Map(k, v) => {
                k.erase_named_params(params);
                v.erase_named_params(params);
            }
            Ty::Union(ms) => {
                for m in ms.iter_mut() {
                    m.erase_named_params(params);
                }
            }
            Ty::App(_, args) => {
                for a in args.iter_mut() {
                    a.erase_named_params(params);
                }
            }
            _ => {}
        }
    }

    /// Convert a written type expression to a checker type *without*
    /// validating named types.
    ///
    /// Used during hoisting, before user types are fully collected. Unknown
    /// names become `Ty::Named`, which the checker treats as opaque. This is
    /// intentionally unsound for diagnostics but sound for the "compatible
    /// with everything" fallback the checker relies on for user types.
    #[must_use]
    pub fn from_expr_lenient(t: &TypeExpr) -> Ty {
        match t {
            TypeExpr::Int => Ty::Int,
            TypeExpr::Float => Ty::Float,
            TypeExpr::Bool => Ty::Bool,
            TypeExpr::String => Ty::String,
            TypeExpr::None => Ty::None,
            TypeExpr::Never => Ty::Never,
            TypeExpr::List(inner) => Ty::List(Box::new(Ty::from_expr_lenient(inner))),
            TypeExpr::Array(inner, n) => Ty::Array(Box::new(Ty::from_expr_lenient(inner)), *n),
            TypeExpr::Tuple(members) => {
                Ty::tuple(members.iter().map(Ty::from_expr_lenient).collect())
            }
            TypeExpr::Set(inner) => Ty::Set(Box::new(Ty::from_expr_lenient(inner))),
            TypeExpr::Map(k, v) => Ty::Map(
                Box::new(Ty::from_expr_lenient(k)),
                Box::new(Ty::from_expr_lenient(v)),
            ),
            TypeExpr::Union(members) => {
                Ty::union(members.iter().map(Ty::from_expr_lenient).collect())
            }
            TypeExpr::Named(n) => Ty::Named(n.clone()),
            TypeExpr::App(n, args) => {
                Ty::App(n.clone(), args.iter().map(Ty::from_expr_lenient).collect())
            }
        }
    }

    /// Convert a checker type back to a written type expression, when it can be
    /// represented. `Unknown` (and any union containing it) has no source
    /// spelling and becomes `None`. Used to persist an inferred binding type in
    /// the REPL so a later submission keeps the same nominal type information a
    /// single module would have.
    #[must_use]
    pub fn to_type_expr(&self) -> Option<TypeExpr> {
        Some(match self {
            Ty::Int => TypeExpr::Int,
            Ty::Float => TypeExpr::Float,
            Ty::Bool => TypeExpr::Bool,
            Ty::String => TypeExpr::String,
            Ty::List(inner) => TypeExpr::List(Box::new(inner.to_type_expr()?)),
            Ty::Array(inner, n) => TypeExpr::Array(Box::new(inner.to_type_expr()?), *n),
            Ty::Tuple(ts) => {
                let mut members = Vec::with_capacity(ts.len());
                for t in ts {
                    members.push(t.to_type_expr()?);
                }
                TypeExpr::Tuple(members)
            }
            Ty::Set(inner) => TypeExpr::Set(Box::new(inner.to_type_expr()?)),
            Ty::Map(k, v) => {
                TypeExpr::Map(Box::new(k.to_type_expr()?), Box::new(v.to_type_expr()?))
            }
            Ty::Named(n) => TypeExpr::Named(n.clone()),
            Ty::Enum(n) => TypeExpr::Named(n.clone()),
            Ty::App(n, args) => {
                let mut out = Vec::with_capacity(args.len());
                for a in args {
                    out.push(a.to_type_expr()?);
                }
                TypeExpr::App(n.clone(), out)
            }
            Ty::Union(members) => {
                let mut out = Vec::with_capacity(members.len());
                for m in members {
                    out.push(m.to_type_expr()?);
                }
                TypeExpr::Union(out)
            }
            // A parameter has a source spelling (its name) only inside its
            // declaration; outside, it cannot be persisted, so it is absent.
            Ty::Param(n) => TypeExpr::Named(n.clone()),
            Ty::Never => TypeExpr::Never,
            Ty::None => TypeExpr::None,
            Ty::Unknown => return None,
        })
    }
}

/// Flatten a type into `out`, splicing nested unions into their members.
fn flatten_into(ty: Ty, out: &mut Vec<Ty>) {
    match ty {
        Ty::Union(members) => {
            for m in members {
                flatten_into(m, out);
            }
        }
        other => out.push(other),
    }
}

/// One overload candidate: its parameter types in order. A `None` entry is an
/// unannotated parameter, which accepts any argument.
pub type OverloadParams = Vec<Option<Ty>>;

/// The result of resolving a call against a set of overloads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OverloadResolution {
    /// No candidate accepts the argument types.
    NoMatch,
    /// Every candidate here accepts the arguments and they are equally
    /// specific, so the call is ambiguous. The indices are the tied
    /// candidates, in declaration order.
    Ambiguous(Vec<usize>),
    /// Exactly one most-specific candidate accepts the arguments.
    Selected(usize),
}

/// Resolve a call against `candidates` by argument types.
///
/// This is the single overload selector shared by the checker and the runtime,
/// so they cannot disagree. The rules (`LANGUAGE_SPEC.md` §15.7):
///
/// * identity is `name + ordered input types` — the return type is never used;
/// * a candidate is *viable* when its arity matches and every annotated
///   parameter is compatible with the corresponding argument type;
/// * among viable candidates the most *specific* wins, where an exact type
///   match is more specific than a merely compatible one (a union or
///   `Unknown` match), which is more specific than an unannotated parameter;
/// * a tie between equally specific viable candidates is ambiguous.
///
/// Unannotated parameters contribute no specificity, so an annotated overload
/// is preferred over an unannotated one when both accept. When the argument
/// type is itself `Unknown`, no candidate can be proven an exact match, so
/// annotated and unannotated candidates all score via the compatible tier and
/// a genuinely tied set is reported ambiguous rather than guessed.
#[must_use]
pub fn resolve_overload(candidates: &[OverloadParams], actual: &[Ty]) -> OverloadResolution {
    let mut viable: Vec<(usize, u32)> = Vec::new();
    for (index, params) in candidates.iter().enumerate() {
        if params.len() != actual.len() {
            continue;
        }
        if !params
            .iter()
            .zip(actual)
            .all(|(expected, arg)| match expected {
                Some(e) => e.compatible_with(arg),
                None => true,
            })
        {
            continue;
        }
        let score: u32 = params
            .iter()
            .zip(actual)
            .map(|(expected, arg)| match expected {
                // Exact (non-union, non-unknown) match: most specific.
                Some(e) if e == arg => 2,
                // Compatible but not identical (union, or `Unknown` argument).
                Some(_) => 1,
                // Unannotated parameter: least specific.
                None => 0,
            })
            .sum();
        viable.push((index, score));
    }
    if viable.is_empty() {
        return OverloadResolution::NoMatch;
    }
    let best = viable.iter().map(|(_, s)| *s).max().unwrap_or(0);
    let winners: Vec<usize> = viable
        .iter()
        .filter(|(_, s)| *s == best)
        .map(|(i, _)| *i)
        .collect();
    if winners.len() == 1 {
        OverloadResolution::Selected(winners[0])
    } else {
        OverloadResolution::Ambiguous(winners)
    }
}
