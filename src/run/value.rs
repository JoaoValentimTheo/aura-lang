//! Runtime values.

use std::cell::RefCell;
use std::collections::{BTreeMap, HashSet};
use std::rc::Rc;

/// Maximum structural depth traversed by the recursive *display* and *JSON
/// encoding* of a value. This bounds native stack usage so that a cyclic or
/// pathologically deep value cannot crash the host (`LANGUAGE_SPEC.md`
/// §31.5). It is a host-safety guard, not a language type: display elides the
/// remainder past this depth with `…`, and JSON encodes it as `null`.
/// Structural equality is exact and iterative, so it is not bounded by this
/// constant.
pub const MAX_VALUE_DEPTH: usize = 512;

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
    ///
    /// Comparison is exact for every acyclic value and is computed with an
    /// explicit worklist, so a deeply nested value cannot exhaust the native
    /// stack (`LANGUAGE_SPEC.md` §31.5). A pair of containers already under
    /// comparison is assumed equal, which terminates on cyclic values and
    /// gives the expected "unrolls identically" result for them. A list, map,
    /// struct, or variant is equal to itself by identity before any descent,
    /// so equality is reflexive at every depth.
    #[must_use]
    pub fn equals(&self, other: &Value) -> bool {
        let mut stack: Vec<(Value, Value)> = vec![(self.clone(), other.clone())];
        let mut seen: HashSet<(usize, usize)> = HashSet::new();
        while let Some((a, b)) = stack.pop() {
            match (&a, &b) {
                (Value::None, Value::None) => {}
                (Value::Bool(x), Value::Bool(y)) if x == y => {}
                (Value::Int(x), Value::Int(y)) if x == y => {}
                (Value::Float(x), Value::Float(y)) if x == y => {}
                (Value::Int(x), Value::Float(y)) | (Value::Float(y), Value::Int(x))
                    if (*x as f64) == *y => {}
                (Value::Str(x), Value::Str(y)) if x == y => {}
                (Value::List(x), Value::List(y)) => {
                    if Rc::ptr_eq(x, y) {
                        continue;
                    }
                    if !seen.insert((rc_addr(x), rc_addr(y))) {
                        continue;
                    }
                    let (xb, yb) = (x.borrow(), y.borrow());
                    if xb.len() != yb.len() {
                        return false;
                    }
                    for (cx, cy) in xb.iter().zip(yb.iter()) {
                        stack.push((cx.clone(), cy.clone()));
                    }
                }
                (Value::Map(x), Value::Map(y)) => {
                    if Rc::ptr_eq(x, y) {
                        continue;
                    }
                    if !seen.insert((rc_addr(x), rc_addr(y))) {
                        continue;
                    }
                    let (xb, yb) = (x.borrow(), y.borrow());
                    if xb.len() != yb.len() {
                        return false;
                    }
                    for (k, v) in xb.iter() {
                        match yb.get(k) {
                            Some(w) => stack.push((v.clone(), w.clone())),
                            None => return false,
                        }
                    }
                }
                (Value::Instance(x), Value::Instance(y)) => {
                    if Rc::ptr_eq(x, y) {
                        continue;
                    }
                    if !seen.insert((rc_addr(x), rc_addr(y))) {
                        continue;
                    }
                    if x.ty != y.ty {
                        return false;
                    }
                    let (xb, yb) = (x.fields.borrow(), y.fields.borrow());
                    if xb.len() != yb.len() {
                        return false;
                    }
                    for ((k1, v1), (k2, v2)) in xb.iter().zip(yb.iter()) {
                        if k1 != k2 {
                            return false;
                        }
                        stack.push((v1.clone(), v2.clone()));
                    }
                }
                (Value::Variant(x), Value::Variant(y)) => {
                    if Rc::ptr_eq(x, y) {
                        continue;
                    }
                    if !seen.insert((rc_addr(x), rc_addr(y))) {
                        continue;
                    }
                    if x.tag != y.tag || x.payload.len() != y.payload.len() {
                        return false;
                    }
                    for (p, q) in x.payload.iter().zip(y.payload.iter()) {
                        stack.push((p.clone(), q.clone()));
                    }
                }
                (Value::Range(x), Value::Range(y)) => {
                    if x.start != y.start || x.end != y.end {
                        return false;
                    }
                }
                // Functions compare by identity: a closure is equal only to
                // itself, and two natives are equal when they name the same
                // builtin.
                (Value::Closure(x), Value::Closure(y)) => {
                    if !Rc::ptr_eq(x, y) {
                        return false;
                    }
                }
                (Value::Native(x), Value::Native(y)) => {
                    if x != y {
                        return false;
                    }
                }
                _ => return false,
            }
        }
        true
    }

    /// Whether values of these two types can be ordered at all, even if a
    /// particular pair (such as two NaNs) is unordered.
    #[must_use]
    pub fn comparable_with(&self, other: &Value) -> bool {
        matches!(
            (self, other),
            (Value::Int(_), Value::Int(_))
                | (Value::Float(_), Value::Float(_))
                | (Value::Int(_), Value::Float(_))
                | (Value::Float(_), Value::Int(_))
                | (Value::Str(_), Value::Str(_))
                | (Value::Bool(_), Value::Bool(_))
        )
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
        self.repr(true, 0)
    }

    /// The debug form (quotes strings inside collections).
    #[must_use]
    pub fn debug_repr(&self) -> String {
        self.repr(false, 0)
    }

    fn repr(&self, top: bool, depth: usize) -> String {
        // A cyclic or very deep value is rendered truncated rather than
        // recursing until the native stack overflows (`LANGUAGE_SPEC.md`
        // §31.5). `…` marks the elided remainder.
        if depth >= MAX_VALUE_DEPTH {
            return "…".to_string();
        }
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
                    .map(|v| v.repr(false, depth + 1))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("[{inner}]")
            }
            Value::Map(m) => {
                let inner = m
                    .borrow()
                    .iter()
                    .map(|(k, v)| format!("\"{k}\": {}", v.repr(false, depth + 1)))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{{{inner}}}")
            }
            Value::Instance(i) => {
                let fields = i
                    .fields
                    .borrow()
                    .iter()
                    .map(|(k, v)| format!("{k}: {}", v.repr(false, depth + 1)))
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
                        .map(|x| x.repr(false, depth + 1))
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

/// The address of an `Rc` allocation, for the equality cycle-detection set.
fn rc_addr<T>(rc: &Rc<T>) -> usize {
    Rc::as_ptr(rc) as usize
}

/// Move the direct children of a uniquely-owned container into `out` so it can
/// be torn down iteratively. A container with more than one owner (an alias or
/// an `Rc` cycle) is left untouched: its `Rc` drop merely decrements.
fn take_children(v: &mut Value, out: &mut Vec<Value>) {
    match v {
        Value::List(rc) => {
            if let Some(items) = Rc::get_mut(rc) {
                out.append(&mut items.borrow_mut());
            }
        }
        Value::Map(rc) => {
            if let Some(map) = Rc::get_mut(rc) {
                for val in map.borrow_mut().values_mut() {
                    out.push(std::mem::replace(val, Value::None));
                }
            }
        }
        Value::Instance(rc) => {
            if let Some(inst) = Rc::get_mut(rc) {
                for (_, val) in inst.fields.get_mut().iter_mut() {
                    out.push(std::mem::replace(val, Value::None));
                }
            }
        }
        Value::Variant(rc) => {
            if let Some(var) = Rc::get_mut(rc) {
                out.append(&mut var.payload);
            }
        }
        _ => {}
    }
}

/// Drop a value without recursing on its structure.
///
/// The derived recursive drop overflows the native stack for a deeply nested
/// or cyclic value, aborting the process — a §31.5 host-safety violation.
/// This teardown takes each uniquely-owned container's children into an
/// explicit worklist and drops them level by level. `Rc` cycles (a container
/// reachable from itself) cannot be uniquely owned, so they are leaked rather
/// than followed, which is the standard, memory-safe `Rc` behavior.
impl Drop for Value {
    fn drop(&mut self) {
        let mut stack: Vec<Value> = Vec::new();
        take_children(self, &mut stack);
        while let Some(mut v) = stack.pop() {
            take_children(&mut v, &mut stack);
        }
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
