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
    /// `{string: V}` — v3 maps are string-keyed.
    Map(Box<Ty>),
    /// A named user type.
    Named(String),
    /// An enum value, by enum type name.
    Enum(String),
    /// `T1 | T2 | ...` — a union of two or more distinct members. Stored in a
    /// canonical, flattened, deduplicated form ([`Ty::union`]); a union with
    /// `none` (or any `Unknown` member) collapses to [`Ty::Unknown`], which is
    /// the documented permissive behavior of `T | none`.
    Union(Vec<Ty>),
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
            Ty::Map(v) => format!("{{string: {}}}", v.name()),
            Ty::Named(n) => n.clone(),
            Ty::Enum(n) => n.clone(),
            Ty::Union(ms) => ms.iter().map(Ty::name).collect::<Vec<_>>().join(" | "),
            Ty::Unknown => "unknown".into(),
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
        // `none`/`Unknown` makes the union permissive, exactly as `T | none`
        // has always been. Collapse immediately so the representation never
        // carries `Unknown` as a member.
        if flat.iter().any(|m| matches!(m, Ty::Unknown)) {
            return Ty::Unknown;
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
            Ty::Map(_) => 5,
            Ty::Named(_) | Ty::Enum(_) => 6,
            Ty::Union(_) => 7,
            Ty::Unknown => 8,
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
        match (self, other) {
            (Ty::Union(expected), Ty::Union(actual)) => actual
                .iter()
                .all(|a| expected.iter().any(|e| e.compatible_with(a))),
            (Ty::Union(expected), actual) => expected.iter().any(|e| e.compatible_with(actual)),
            (expected, Ty::Union(actual)) => actual.iter().all(|a| expected.compatible_with(a)),
            (Ty::List(a), Ty::List(b)) => a.compatible_with(b),
            (Ty::Map(a), Ty::Map(b)) => a.compatible_with(b),
            _ => self == other,
        }
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
            Ty::Map(_) => TypeClass::Map,
            Ty::Named(_) | Ty::Enum(_) | Ty::Union(_) | Ty::Unknown => return None,
        })
    }

    /// Whether two types can be ordered with `<`, `<=`, `>`, `>=`.
    ///
    /// This mirrors `Value::comparable_with`: only numeric-numeric, string,
    /// and bool pairs are orderable. Returns `Some(false)` only when it can
    /// prove the comparison is a type error; `None` when a side is `Unknown`
    /// or a union the checker cannot pin to one class.
    #[must_use]
    pub fn orderable_with(&self, other: &Ty) -> Option<bool> {
        if matches!(self, Ty::Unknown | Ty::Union(_)) || matches!(other, Ty::Unknown | Ty::Union(_))
        {
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
            // `none` has no static type; it is the permissive `Unknown`.
            TypeExpr::None => Ty::Unknown,
            TypeExpr::List(inner) => Ty::List(Box::new(Ty::from_expr(inner, types, span)?)),
            TypeExpr::Map(k, v) => {
                let key = Ty::from_expr(k, types, span)?;
                if !matches!(key, Ty::String | Ty::Unknown) {
                    return Err(Diag::new(
                        codes::TYPE_MISMATCH,
                        format!(
                            "Aura maps are string-keyed in this version; `{}` is not a valid key type",
                            key.name()
                        ),
                        span,
                    ));
                }
                Ty::Map(Box::new(Ty::from_expr(v, types, span)?))
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
        })
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
            TypeExpr::None => Ty::Unknown,
            TypeExpr::List(inner) => Ty::List(Box::new(Ty::from_expr_lenient(inner))),
            TypeExpr::Map(_, v) => Ty::Map(Box::new(Ty::from_expr_lenient(v))),
            TypeExpr::Union(members) => {
                Ty::union(members.iter().map(Ty::from_expr_lenient).collect())
            }
            TypeExpr::Named(n) => Ty::Named(n.clone()),
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
            Ty::Map(v) => TypeExpr::Map(Box::new(TypeExpr::String), Box::new(v.to_type_expr()?)),
            Ty::Named(n) => TypeExpr::Named(n.clone()),
            Ty::Enum(n) => TypeExpr::Named(n.clone()),
            Ty::Union(members) => {
                let mut out = Vec::with_capacity(members.len());
                for m in members {
                    out.push(m.to_type_expr()?);
                }
                TypeExpr::Union(out)
            }
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
