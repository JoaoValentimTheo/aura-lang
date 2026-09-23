//! Runtime values.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

/// A runtime value.
#[derive(Clone)]
pub enum Value {
    /// `int`
    Int(i64),
    /// `float`
    Float(f64),
    /// `string`
    Str(Rc<str>),
    /// `bool`
    Bool(bool),
    /// `none`
    None,
    /// `[T]`
    List(Rc<RefCell<Vec<Value>>>),
    /// `{K: V}`
    Map(Rc<RefCell<BTreeMap<String, Value>>>),
    /// A struct instance.
    Instance(Rc<Instance>),
    /// An enum variant.
    Variant(Rc<Variant>),
    /// A first-class function.
    Closure(Rc<crate::run::Closure>),
    /// A native function referenced by name (for pipes like `xs |> len`).
    Native(String),
    /// A range produced by `range(a, b)`.
    Range(Rc<RangeVal>),
}

/// A struct instance.
#[derive(Debug)]
pub struct Instance {
    /// Type name.
    pub ty: String,
    /// Fields in declaration order.
    pub fields: RefCell<Vec<(String, Value)>>,
}

/// An enum variant value.
#[derive(Debug)]
pub struct Variant {
    /// Enum type name.
    pub ty: String,
    /// Variant name.
    pub tag: String,
    /// Payload.
    pub payload: Vec<Value>,
}

/// A lazy integer range.
#[derive(Debug)]
pub struct RangeVal {
    /// Start (inclusive).
    pub start: i64,
    /// End (exclusive).
    pub end: i64,
}

impl RangeVal {
    /// The number of elements, using saturating arithmetic so extreme bounds
    /// never overflow. Ranges are always step 1 in v3.
    #[must_use]
    pub fn len(&self) -> i64 {
        if self.end <= self.start {
            0
        } else {
            self.end.saturating_sub(self.start)
        }
    }

    /// Whether the range contains no elements.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.end <= self.start
    }
}

impl Value {
    /// The `string` value from a `String`.
    #[must_use]
    pub fn str(s: impl Into<String>) -> Value {
        Value::Str(Rc::from(s.into().as_str()))
    }

    /// An empty list.
    #[must_use]
    pub fn list(items: Vec<Value>) -> Value {
        Value::List(Rc::new(RefCell::new(items)))
    }

    /// A runtime type name for diagnostics.
    #[must_use]
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Int(_) => "int",
            Value::Float(_) => "float",
            Value::Str(_) => "string",
            Value::Bool(_) => "bool",
            Value::None => "none",
            Value::List(_) => "list",
            Value::Map(_) => "map",
            Value::Instance(_) => "struct",
            Value::Variant(_) => "enum",
            Value::Closure(_) | Value::Native(_) => "fn",
            Value::Range(_) => "range",
        }
    }

    /// Truthiness: `none` and `false` are falsy; `0` and `""` are falsy too.
    #[must_use]
    pub fn truthy(&self) -> bool {
        match self {
            Value::None => false,
            Value::Bool(b) => *b,
            Value::Int(i) => *i != 0,
            Value::Float(f) => *f != 0.0,
            Value::Str(s) => !s.is_empty(),
            Value::List(l) => !l.borrow().is_empty(),
            Value::Map(m) => !m.borrow().is_empty(),
            _ => true,
        }
    }

    /// Structural equality.
    #[must_use]
    pub fn equals(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::None, Value::None) => true,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::Int(a), Value::Float(b)) | (Value::Float(b), Value::Int(a)) => {
                (*a as f64) == *b
            }
            (Value::Str(a), Value::Str(b)) => a == b,
            (Value::List(a), Value::List(b)) => {
                let (a, b) = (a.borrow(), b.borrow());
                a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| x.equals(y))
            }
            (Value::Map(a), Value::Map(b)) => {
                let (a, b) = (a.borrow(), b.borrow());
                a.len() == b.len() && a.iter().all(|(k, v)| b.get(k).is_some_and(|w| v.equals(w)))
            }
            (Value::Instance(a), Value::Instance(b)) => {
                a.ty == b.ty
                    && a.fields.borrow().len() == b.fields.borrow().len()
                    && a.fields
                        .borrow()
                        .iter()
                        .zip(b.fields.borrow().iter())
                        .all(|((k1, v1), (k2, v2))| k1 == k2 && v1.equals(v2))
            }
            (Value::Variant(a), Value::Variant(b)) => {
                a.tag == b.tag
                    && a.payload.len() == b.payload.len()
                    && a.payload
                        .iter()
                        .zip(b.payload.iter())
                        .all(|(x, y)| x.equals(y))
            }
            (Value::Range(a), Value::Range(b)) => a.start == b.start && a.end == b.end,
            _ => false,
        }
    }

    /// Ordering for `<`, `<=`, `>`, `>=`; `None` if unordered.
    #[must_use]
    pub fn cmp_val(&self, other: &Value) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (Value::Int(a), Value::Int(b)) => Some(a.cmp(b)),
            (Value::Float(a), Value::Float(b)) => a.partial_cmp(b),
            (Value::Int(a), Value::Float(b)) => (*a as f64).partial_cmp(b),
            (Value::Float(a), Value::Int(b)) => a.partial_cmp(&(*b as f64)),
            (Value::Str(a), Value::Str(b)) => Some(a.cmp(b)),
            (Value::Bool(a), Value::Bool(b)) => Some(a.cmp(b)),
            _ => None,
        }
    }

    /// The display form (used by `print` and `to_string`).
    #[must_use]
    pub fn display(&self) -> String {
        self.repr(true)
    }

    /// The debug form (quotes strings inside collections).
    #[must_use]
    pub fn debug_repr(&self) -> String {
        self.repr(false)
    }

    fn repr(&self, top: bool) -> String {
        match self {
            Value::Int(i) => i.to_string(),
            Value::Float(f) => format_float(*f),
            Value::Str(s) => {
                if top {
                    (*s).to_string()
                } else {
                    format!("\"{s}\"")
                }
            }
            Value::Bool(b) => if *b { "true" } else { "false" }.to_string(),
            Value::None => "none".to_string(),
            Value::List(l) => {
                let inner = l
                    .borrow()
                    .iter()
                    .map(Value::debug_repr)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("[{inner}]")
            }
            Value::Map(m) => {
                let inner = m
                    .borrow()
                    .iter()
                    .map(|(k, v)| format!("\"{k}\": {}", v.debug_repr()))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{{{inner}}}")
            }
            Value::Instance(i) => {
                let fields = i
                    .fields
                    .borrow()
                    .iter()
                    .map(|(k, v)| format!("{k}: {}", v.debug_repr()))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{} {{ {fields} }}", i.ty)
            }
            Value::Variant(v) => {
                if v.payload.is_empty() {
                    v.tag.clone()
                } else {
                    let inner = v
                        .payload
                        .iter()
                        .map(Value::debug_repr)
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!("{}({inner})", v.tag)
                }
            }
            Value::Closure(_) | Value::Native(_) => "<fn>".to_string(),
            Value::Range(r) => format!("{}..{}", r.start, r.end),
        }
    }
}

impl std::fmt::Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.debug_repr())
    }
}

/// Canonical float formatting: integral floats keep one decimal.
#[must_use]
pub fn format_float(f: f64) -> String {
    if f.is_nan() {
        return "nan".to_string();
    }
    if f.is_infinite() {
        return if f > 0.0 {
            "inf".to_string()
        } else {
            "-inf".to_string()
        };
    }
    if f == f.trunc() && f.abs() < 1e16 {
        format!("{f:.1}")
    } else {
        format!("{f}")
    }
}
