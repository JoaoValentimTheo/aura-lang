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
    /// Anything the checker does not know; compatible with all types.
    Unknown,
}

impl Ty {
    /// The source spelling of this type.
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
            Ty::Unknown => "unknown".into(),
        }
    }

    /// Whether this type is compatible with `other` as an assignment target.
    /// `Unknown` is compatible with everything; `int` and `float` are
    /// *not* interchangeable (the contract forbids implicit coercion).
    #[must_use]
    pub fn compatible_with(&self, other: &Ty) -> bool {
        if matches!(self, Ty::Unknown) || matches!(other, Ty::Unknown) {
            return true;
        }
        match (self, other) {
            (Ty::List(a), Ty::List(b)) => a.compatible_with(b),
            (Ty::Map(a), Ty::Map(b)) => a.compatible_with(b),
            _ => self == other,
        }
    }

    /// The coarse built-in type class of this type, or `None` when it is
    /// `Unknown` or a user type with no method table.
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
            Ty::Named(_) | Ty::Enum(_) | Ty::Unknown => return None,
        })
    }

    /// Whether two types can be ordered with `<`, `<=`, `>`, `>=`.
    ///
    /// This mirrors `Value::comparable_with`: only numeric-numeric, string,
    /// and bool pairs are orderable. Returns `Some(false)` only when it can
    /// prove the comparison is a type error; `None` when a side is `Unknown`.
    #[must_use]
    pub fn orderable_with(&self, other: &Ty) -> Option<bool> {
        if matches!(self, Ty::Unknown) || matches!(other, Ty::Unknown) {
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
            TypeExpr::Optional(inner) => {
                // `T | none` accepts T or none; model as Unknown at this
                // precision level, which is compatible with both.
                Ty::from_expr(inner, types, span)?;
                Ty::Unknown
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
            TypeExpr::List(inner) => Ty::List(Box::new(Ty::from_expr_lenient(inner))),
            TypeExpr::Map(_, v) => Ty::Map(Box::new(Ty::from_expr_lenient(v))),
            TypeExpr::Optional(_) => Ty::Unknown,
            TypeExpr::Named(n) => Ty::Named(n.clone()),
        }
    }
}
