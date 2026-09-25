//! One authoritative description of every builtin callable.
//!
//! Both the static checker and the runtime consult this registry, so they
//! cannot drift: the checker rejects a call the runtime would reject, and
//! knows the result type where the registry states it.
//!
//! The model is deliberately coarse. It does not encode a full type system;
//! it records exactly what is statically knowable about each callable:
//!
//! * how many arguments it takes,
//! * which type classes each argument accepts,
//! * what type it returns (or that the result is dynamic).
//!
//! `Accepts::Any` and `Returns::Dynamic` are explicit "cannot be known
//! statically" values, not guesses.

use crate::types::Ty;

/// A type class a parameter accepts. This is intentionally coarser than
/// [`Ty`]: many builtins are polymorphic over a family of types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Accepts {
    /// Any value.
    Any,
    /// Exactly one concrete type.
    One(TypeClass),
    /// One of several concrete types.
    AnyOf(&'static [TypeClass]),
}

/// A coarse type class used by [`Accepts`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeClass {
    /// `int`
    Int,
    /// `float`
    Float,
    /// `bool`
    Bool,
    /// `string`
    Str,
    /// `[T]`
    List,
    /// `{string: V}`
    Map,
    /// A `range` value.
    Range,
    /// `none`
    None,
    /// A function/closure.
    Function,
    /// Any other value (struct, enum, range, ...).
    Other,
}

impl TypeClass {
    /// Whether a statically-inferred [`Ty`] definitely belongs to this class.
    /// Returns `None` when the type is `Unknown` and nothing can be decided.
    #[must_use]
    pub fn matches_ty(self, ty: &Ty) -> Option<bool> {
        let is = match (self, ty) {
            (_, Ty::Unknown) => return None,
            // A union satisfies a class when any member does; a union never
            // contains `Unknown` (it collapses to `Unknown`), so the answer is
            // always decidable.
            (_, Ty::Union(members)) => {
                return Some(members.iter().any(|m| self.matches_ty(m) == Some(true)));
            }
            (TypeClass::Int, Ty::Int) => true,
            (TypeClass::Float, Ty::Float) => true,
            (TypeClass::Bool, Ty::Bool) => true,
            (TypeClass::Str, Ty::String) => true,
            (TypeClass::List, Ty::List(_)) => true,
            (TypeClass::Map, Ty::Map(_)) => true,
            (TypeClass::Range, Ty::Named(n)) if n == "range" => true,
            (TypeClass::None, _) => false,
            (TypeClass::Function, _) => false,
            (TypeClass::Other, ty) => {
                matches!(ty, Ty::Named(_) | Ty::Enum(_))
            }
            _ => false,
        };
        Some(is)
    }

    /// The source spelling used in diagnostics.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            TypeClass::Int => "int",
            TypeClass::Float => "float",
            TypeClass::Bool => "bool",
            TypeClass::Str => "string",
            TypeClass::List => "list",
            TypeClass::Map => "map",
            TypeClass::Range => "range",
            TypeClass::None => "none",
            TypeClass::Function => "fn",
            TypeClass::Other => "value",
        }
    }
}

impl Accepts {
    /// Whether an inferred type satisfies this parameter expectation.
    /// `None` means "cannot decide" (the argument type is unknown).
    #[must_use]
    pub fn accepts_ty(self, ty: &Ty) -> Option<bool> {
        match self {
            Accepts::Any => Some(true),
            Accepts::One(c) => c.matches_ty(ty),
            Accepts::AnyOf(cs) => {
                let mut any_unknown = false;
                for c in cs {
                    match c.matches_ty(ty) {
                        Some(true) => return Some(true),
                        Some(false) => {}
                        None => any_unknown = true,
                    }
                }
                if any_unknown {
                    None
                } else {
                    Some(false)
                }
            }
        }
    }

    /// The human-readable list of accepted classes.
    #[must_use]
    pub fn describe(self) -> String {
        match self {
            Accepts::Any => "any value".to_string(),
            Accepts::One(c) => c.name().to_string(),
            Accepts::AnyOf(cs) => {
                let names: Vec<&str> = cs.iter().map(|c| c.name()).collect();
                names.join(", ")
            }
        }
    }
}

/// What a callable returns, as far as the checker can know.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Returns {
    /// A statically known concrete type.
    Ty(Ty),
    /// The result type depends on the arguments or is not modelled; the
    /// checker must not assume anything about it.
    Dynamic,
}

impl Returns {
    /// The checker type of this return, or `Unknown` when dynamic.
    #[must_use]
    pub fn ty(&self) -> Ty {
        match self {
            Returns::Ty(t) => t.clone(),
            Returns::Dynamic => Ty::Unknown,
        }
    }
}

/// One parameter description.
#[derive(Debug, Clone, Copy)]
pub struct Param {
    /// Accepted type classes.
    pub accepts: Accepts,
}

impl Param {
    /// A parameter accepting any value.
    pub const ANY: Param = Param {
        accepts: Accepts::Any,
    };
    /// A parameter accepting exactly `c`.
    pub const fn one(c: TypeClass) -> Param {
        Param {
            accepts: Accepts::One(c),
        }
    }
    /// A parameter accepting any of `cs`.
    pub const fn any_of(cs: &'static [TypeClass]) -> Param {
        Param {
            accepts: Accepts::AnyOf(cs),
        }
    }
}

/// A builtin function signature.
#[derive(Debug, Clone)]
pub struct Signature {
    /// The callable name.
    pub name: &'static str,
    /// Fixed parameters.
    pub params: Vec<Param>,
    /// Minimum number of arguments (for variadic callables).
    pub min_args: usize,
    /// Maximum number of arguments (`usize::MAX` for unbounded variadic).
    pub max_args: usize,
    /// The result type.
    pub returns: Returns,
}

impl Signature {
    /// Check an argument count against this signature; returns a message on
    /// failure. Used by both the checker and the runtime so the wording and
    /// the decision are identical.
    #[must_use]
    pub fn check_arity(&self, count: usize) -> Option<String> {
        check_arity(self.name, self.min_args, self.max_args, count)
    }
}

/// Shared arity check for builtins and methods.
#[must_use]
fn check_arity(name: &str, min: usize, max: usize, count: usize) -> Option<String> {
    if count < min {
        return Some(format!(
            "`{name}` expects at least {min} argument(s), got {count}"
        ));
    }
    if count > max {
        return Some(format!(
            "`{name}` expects at most {max} argument(s), got {count}"
        ));
    }
    None
}

const NUM: &[TypeClass] = &[TypeClass::Int, TypeClass::Float];
const STR_LIST: &[TypeClass] = &[TypeClass::Str, TypeClass::List];

/// Every builtin callable in this build, keyed by name.
///
/// This is the single source of truth. The runtime registers exactly these
/// names, and the checker validates calls against exactly these signatures.
#[must_use]
pub fn builtins() -> &'static [Signature] {
    use std::sync::OnceLock;
    static TABLE: OnceLock<Vec<Signature>> = OnceLock::new();
    TABLE.get_or_init(|| {
        vec![
            // ------------------------------------------------------- core
            Signature {
                name: "print",
                params: vec![],
                min_args: 0,
                max_args: usize::MAX,
                returns: Returns::Ty(Ty::Unknown),
            },
            Signature {
                name: "len",
                params: vec![Param::any_of(&[
                    TypeClass::Str,
                    TypeClass::List,
                    TypeClass::Map,
                    TypeClass::Range,
                ])],
                min_args: 1,
                max_args: 1,
                returns: Returns::Ty(Ty::Int),
            },
            Signature {
                name: "to_string",
                params: vec![Param::ANY],
                min_args: 1,
                max_args: 1,
                returns: Returns::Ty(Ty::String),
            },
            Signature {
                name: "to_int",
                params: vec![Param::ANY],
                min_args: 1,
                max_args: 1,
                returns: Returns::Ty(Ty::Int),
            },
            Signature {
                name: "to_float",
                params: vec![Param::ANY],
                min_args: 1,
                max_args: 1,
                returns: Returns::Ty(Ty::Float),
            },
            Signature {
                name: "range",
                params: vec![Param::one(TypeClass::Int)],
                min_args: 1,
                max_args: 2,
                returns: Returns::Ty(Ty::Named("range".to_string())),
            },
            Signature {
                name: "abs",
                params: vec![Param::any_of(NUM)],
                min_args: 1,
                max_args: 1,
                returns: Returns::Dynamic,
            },
            Signature {
                name: "min",
                params: vec![Param::ANY, Param::ANY],
                min_args: 2,
                max_args: 2,
                returns: Returns::Dynamic,
            },
            Signature {
                name: "max",
                params: vec![Param::ANY, Param::ANY],
                min_args: 2,
                max_args: 2,
                returns: Returns::Dynamic,
            },
            Signature {
                name: "push",
                params: vec![Param::one(TypeClass::List), Param::ANY],
                min_args: 2,
                max_args: 2,
                returns: Returns::Ty(Ty::Unknown),
            },
            Signature {
                name: "keys",
                params: vec![Param::one(TypeClass::Map)],
                min_args: 1,
                max_args: 1,
                returns: Returns::Ty(Ty::List(Box::new(Ty::String))),
            },
            Signature {
                name: "values",
                params: vec![Param::one(TypeClass::Map)],
                min_args: 1,
                max_args: 1,
                returns: Returns::Ty(Ty::List(Box::new(Ty::Unknown))),
            },
            Signature {
                name: "sort",
                params: vec![Param::one(TypeClass::List)],
                min_args: 1,
                max_args: 1,
                returns: Returns::Dynamic,
            },
            Signature {
                name: "reverse",
                params: vec![Param::any_of(STR_LIST)],
                min_args: 1,
                max_args: 1,
                returns: Returns::Dynamic,
            },
            Signature {
                name: "map",
                params: vec![Param::one(TypeClass::List), Param::one(TypeClass::Function)],
                min_args: 2,
                max_args: 2,
                returns: Returns::Ty(Ty::List(Box::new(Ty::Unknown))),
            },
            Signature {
                name: "filter",
                params: vec![Param::one(TypeClass::List), Param::one(TypeClass::Function)],
                min_args: 2,
                max_args: 2,
                returns: Returns::Ty(Ty::List(Box::new(Ty::Unknown))),
            },
            Signature {
                name: "reduce",
                params: vec![
                    Param::one(TypeClass::List),
                    Param::one(TypeClass::Function),
                    Param::ANY,
                ],
                min_args: 3,
                max_args: 3,
                returns: Returns::Dynamic,
            },
            Signature {
                name: "sum",
                params: vec![Param::one(TypeClass::List)],
                min_args: 1,
                max_args: 1,
                returns: Returns::Dynamic,
            },
            Signature {
                name: "assert",
                params: vec![Param::ANY],
                min_args: 1,
                max_args: 2,
                returns: Returns::Ty(Ty::Unknown),
            },
            Signature {
                name: "enumerate",
                params: vec![Param::one(TypeClass::List)],
                min_args: 1,
                max_args: 1,
                returns: Returns::Ty(Ty::List(Box::new(Ty::Unknown))),
            },
            Signature {
                name: "zip",
                params: vec![Param::one(TypeClass::List), Param::one(TypeClass::List)],
                min_args: 2,
                max_args: 2,
                returns: Returns::Ty(Ty::List(Box::new(Ty::Unknown))),
            },
            // ------------------------------------------------- scripting I/O
            Signature {
                name: "read_line",
                params: vec![],
                min_args: 0,
                max_args: 0,
                returns: Returns::Dynamic,
            },
            Signature {
                name: "read_file",
                params: vec![Param::one(TypeClass::Str)],
                min_args: 1,
                max_args: 1,
                returns: Returns::Dynamic,
            },
            Signature {
                name: "write_file",
                params: vec![Param::one(TypeClass::Str), Param::one(TypeClass::Str)],
                min_args: 2,
                max_args: 2,
                returns: Returns::Ty(Ty::Unknown),
            },
            Signature {
                name: "args",
                params: vec![],
                min_args: 0,
                max_args: 0,
                returns: Returns::Ty(Ty::List(Box::new(Ty::String))),
            },
            // -------------------------------------------------- python bridge
            Signature {
                name: "py_eval",
                params: vec![Param::one(TypeClass::Str)],
                min_args: 1,
                max_args: 1,
                returns: Returns::Dynamic,
            },
            Signature {
                name: "py_import",
                params: vec![Param::one(TypeClass::Str)],
                min_args: 1,
                max_args: 1,
                returns: Returns::Dynamic,
            },
            Signature {
                name: "py_call",
                params: vec![],
                min_args: 2,
                max_args: usize::MAX,
                returns: Returns::Dynamic,
            },
            Signature {
                name: "py_version",
                params: vec![],
                min_args: 0,
                max_args: 0,
                returns: Returns::Ty(Ty::String),
            },
            // ------------------------------------------------------ json
            #[cfg(feature = "json")]
            Signature {
                name: "json_encode",
                params: vec![Param::ANY],
                min_args: 1,
                max_args: 1,
                returns: Returns::Ty(Ty::String),
            },
            #[cfg(feature = "json")]
            Signature {
                name: "json_decode",
                params: vec![Param::one(TypeClass::Str)],
                min_args: 1,
                max_args: 1,
                returns: Returns::Dynamic,
            },
            // ----------------------------------------------------- regex
            #[cfg(feature = "regex")]
            Signature {
                name: "regex_match",
                params: vec![Param::one(TypeClass::Str), Param::one(TypeClass::Str)],
                min_args: 2,
                max_args: 2,
                returns: Returns::Ty(Ty::Bool),
            },
            #[cfg(feature = "regex")]
            Signature {
                name: "regex_find",
                params: vec![Param::one(TypeClass::Str), Param::one(TypeClass::Str)],
                min_args: 2,
                max_args: 2,
                returns: Returns::Dynamic,
            },
            #[cfg(feature = "regex")]
            Signature {
                name: "regex_find_all",
                params: vec![Param::one(TypeClass::Str), Param::one(TypeClass::Str)],
                min_args: 2,
                max_args: 2,
                returns: Returns::Ty(Ty::List(Box::new(Ty::String))),
            },
            #[cfg(feature = "regex")]
            Signature {
                name: "regex_replace",
                params: vec![
                    Param::one(TypeClass::Str),
                    Param::one(TypeClass::Str),
                    Param::one(TypeClass::Str),
                ],
                min_args: 3,
                max_args: 3,
                returns: Returns::Ty(Ty::String),
            },
            // ------------------------------------------------------ time
            #[cfg(feature = "time")]
            Signature {
                name: "time_now",
                params: vec![],
                min_args: 0,
                max_args: 0,
                returns: Returns::Dynamic,
            },
            #[cfg(feature = "time")]
            Signature {
                name: "time_unix",
                params: vec![],
                min_args: 0,
                max_args: 0,
                returns: Returns::Ty(Ty::Int),
            },
            #[cfg(feature = "time")]
            Signature {
                name: "sleep_ms",
                params: vec![Param::one(TypeClass::Int)],
                min_args: 1,
                max_args: 1,
                returns: Returns::Ty(Ty::Unknown),
            },
        ]
    })
}

/// Look up a builtin signature by name.
#[must_use]
pub fn builtin(name: &str) -> Option<&'static Signature> {
    builtins().iter().find(|s| s.name == name)
}

/// A method on a built-in receiver kind.
#[derive(Debug, Clone)]
pub struct MethodSig {
    /// The method name.
    pub name: &'static str,
    /// The receiver type class.
    pub receiver: TypeClass,
    /// Parameter expectations (excluding the receiver).
    pub params: Vec<Param>,
    /// Minimum argument count.
    pub min_args: usize,
    /// Maximum argument count.
    pub max_args: usize,
    /// Result type.
    pub returns: Returns,
}

impl MethodSig {
    /// Check an argument count against this method signature; returns a
    /// message on failure. Shared by the checker and the runtime.
    #[must_use]
    pub fn check_arity(&self, count: usize) -> Option<String> {
        check_arity(self.name, self.min_args, self.max_args, count)
    }
}

/// Methods available on built-in receiver kinds.
///
/// Structs and enums have no methods in this version; calling one is an
/// error the checker can prove.
#[must_use]
pub fn methods() -> &'static [MethodSig] {
    use std::sync::OnceLock;
    static TABLE: OnceLock<Vec<MethodSig>> = OnceLock::new();
    TABLE.get_or_init(|| {
        vec![
            // ------------------------------------------------- string
            MethodSig {
                name: "len",
                receiver: TypeClass::Str,
                params: vec![],
                min_args: 0,
                max_args: 0,
                returns: Returns::Ty(Ty::Int),
            },
            MethodSig {
                name: "upper",
                receiver: TypeClass::Str,
                params: vec![],
                min_args: 0,
                max_args: 0,
                returns: Returns::Ty(Ty::String),
            },
            MethodSig {
                name: "lower",
                receiver: TypeClass::Str,
                params: vec![],
                min_args: 0,
                max_args: 0,
                returns: Returns::Ty(Ty::String),
            },
            MethodSig {
                name: "trim",
                receiver: TypeClass::Str,
                params: vec![],
                min_args: 0,
                max_args: 0,
                returns: Returns::Ty(Ty::String),
            },
            MethodSig {
                name: "contains",
                receiver: TypeClass::Str,
                params: vec![Param::one(TypeClass::Str)],
                min_args: 1,
                max_args: 1,
                returns: Returns::Ty(Ty::Bool),
            },
            MethodSig {
                name: "starts_with",
                receiver: TypeClass::Str,
                params: vec![Param::one(TypeClass::Str)],
                min_args: 1,
                max_args: 1,
                returns: Returns::Ty(Ty::Bool),
            },
            MethodSig {
                name: "ends_with",
                receiver: TypeClass::Str,
                params: vec![Param::one(TypeClass::Str)],
                min_args: 1,
                max_args: 1,
                returns: Returns::Ty(Ty::Bool),
            },
            MethodSig {
                name: "split",
                receiver: TypeClass::Str,
                params: vec![Param::one(TypeClass::Str)],
                min_args: 1,
                max_args: 1,
                returns: Returns::Ty(Ty::List(Box::new(Ty::String))),
            },
            MethodSig {
                name: "replace",
                receiver: TypeClass::Str,
                params: vec![Param::one(TypeClass::Str), Param::one(TypeClass::Str)],
                min_args: 2,
                max_args: 2,
                returns: Returns::Ty(Ty::String),
            },
            MethodSig {
                name: "chars",
                receiver: TypeClass::Str,
                params: vec![],
                min_args: 0,
                max_args: 0,
                returns: Returns::Ty(Ty::List(Box::new(Ty::String))),
            },
            // --------------------------------------------------- list
            MethodSig {
                name: "len",
                receiver: TypeClass::List,
                params: vec![],
                min_args: 0,
                max_args: 0,
                returns: Returns::Ty(Ty::Int),
            },
            MethodSig {
                name: "push",
                receiver: TypeClass::List,
                params: vec![Param::ANY],
                min_args: 1,
                max_args: 1,
                returns: Returns::Ty(Ty::Unknown),
            },
            MethodSig {
                name: "pop",
                receiver: TypeClass::List,
                params: vec![],
                min_args: 0,
                max_args: 0,
                returns: Returns::Dynamic,
            },
            MethodSig {
                name: "first",
                receiver: TypeClass::List,
                params: vec![],
                min_args: 0,
                max_args: 0,
                returns: Returns::Dynamic,
            },
            MethodSig {
                name: "last",
                receiver: TypeClass::List,
                params: vec![],
                min_args: 0,
                max_args: 0,
                returns: Returns::Dynamic,
            },
            MethodSig {
                name: "join",
                receiver: TypeClass::List,
                params: vec![Param::one(TypeClass::Str)],
                min_args: 1,
                max_args: 1,
                returns: Returns::Ty(Ty::String),
            },
            MethodSig {
                name: "contains",
                receiver: TypeClass::List,
                params: vec![Param::ANY],
                min_args: 1,
                max_args: 1,
                returns: Returns::Ty(Ty::Bool),
            },
            MethodSig {
                name: "sort",
                receiver: TypeClass::List,
                params: vec![],
                min_args: 0,
                max_args: 0,
                returns: Returns::Dynamic,
            },
            MethodSig {
                name: "reverse",
                receiver: TypeClass::List,
                params: vec![],
                min_args: 0,
                max_args: 0,
                returns: Returns::Dynamic,
            },
            MethodSig {
                name: "map",
                receiver: TypeClass::List,
                params: vec![Param::one(TypeClass::Function)],
                min_args: 1,
                max_args: 1,
                returns: Returns::Ty(Ty::List(Box::new(Ty::Unknown))),
            },
            MethodSig {
                name: "filter",
                receiver: TypeClass::List,
                params: vec![Param::one(TypeClass::Function)],
                min_args: 1,
                max_args: 1,
                returns: Returns::Ty(Ty::List(Box::new(Ty::Unknown))),
            },
            MethodSig {
                name: "reduce",
                receiver: TypeClass::List,
                params: vec![Param::one(TypeClass::Function), Param::ANY],
                min_args: 2,
                max_args: 2,
                returns: Returns::Dynamic,
            },
            // ----------------------------------------------------- map
            MethodSig {
                name: "len",
                receiver: TypeClass::Map,
                params: vec![],
                min_args: 0,
                max_args: 0,
                returns: Returns::Ty(Ty::Int),
            },
            MethodSig {
                name: "get",
                receiver: TypeClass::Map,
                params: vec![Param::one(TypeClass::Str)],
                min_args: 1,
                max_args: 1,
                returns: Returns::Dynamic,
            },
            MethodSig {
                name: "has",
                receiver: TypeClass::Map,
                params: vec![Param::one(TypeClass::Str)],
                min_args: 1,
                max_args: 1,
                returns: Returns::Ty(Ty::Bool),
            },
            MethodSig {
                name: "keys",
                receiver: TypeClass::Map,
                params: vec![],
                min_args: 0,
                max_args: 0,
                returns: Returns::Ty(Ty::List(Box::new(Ty::String))),
            },
            MethodSig {
                name: "values",
                receiver: TypeClass::Map,
                params: vec![],
                min_args: 0,
                max_args: 0,
                returns: Returns::Ty(Ty::List(Box::new(Ty::Unknown))),
            },
            MethodSig {
                name: "remove",
                receiver: TypeClass::Map,
                params: vec![Param::one(TypeClass::Str)],
                min_args: 1,
                max_args: 1,
                returns: Returns::Dynamic,
            },
            // ---------------------------------------------------- range
            MethodSig {
                name: "len",
                receiver: TypeClass::Range,
                params: vec![],
                min_args: 0,
                max_args: 0,
                returns: Returns::Ty(Ty::Int),
            },
        ]
    })
}

/// Look up a method by receiver class and name.
#[must_use]
pub fn method(receiver: TypeClass, name: &str) -> Option<&'static MethodSig> {
    methods()
        .iter()
        .find(|m| m.receiver == receiver && m.name == name)
}

/// Look up a method by name across all receivers, for diagnostics.
#[must_use]
pub fn method_exists_anywhere(name: &str) -> bool {
    methods().iter().any(|m| m.name == name)
}
