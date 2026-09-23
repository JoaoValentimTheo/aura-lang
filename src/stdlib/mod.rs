//! The native standard library: core functions and methods.
//!
//! Everything here is implemented directly in Rust. Python is not needed.

pub mod ext;

use std::cell::RefCell;
use std::rc::Rc;

use crate::error::{codes, Diag, Result, Span};
use crate::run::value::{RangeVal, Value};
use crate::run::Interp;

fn err(code: u16, msg: impl Into<String>, span: Span) -> Diag {
    Diag::new(code, msg, span)
}

fn arg<'a>(args: &'a [Value], i: usize, name: &str, span: Span) -> Result<&'a Value> {
    args.get(i).ok_or_else(|| {
        err(
            codes::TYPE_MISMATCH,
            format!("`{name}` is missing argument {}", i + 1),
            span,
        )
    })
}

fn as_int(v: &Value, what: &str, span: Span) -> Result<i64> {
    match v {
        Value::Int(i) => Ok(*i),
        Value::Float(f) if f.fract() == 0.0 => Ok(*f as i64),
        other => Err(err(
            codes::TYPE_MISMATCH,
            format!("`{what}` expects an int, found {}", other.type_name()),
            span,
        )),
    }
}

/// Install core natives.
pub fn install(it: &mut Interp) {
    crate::bridge::install(it);
    #[cfg(feature = "json")]
    ext::json::install(it);
    #[cfg(feature = "regex")]
    ext::regex::install(it);
    #[cfg(feature = "time")]
    ext::time::install(it);
    it.native("print", |it, args, _span| {
        let mut line = String::new();
        for (i, a) in args.iter().enumerate() {
            if i > 0 {
                line.push(' ');
            }
            line.push_str(&a.display());
        }
        writeln_line(it, &line);
        Ok(Value::None)
    });
    it.native("len", |_it, args, span| {
        let v = arg(&args, 0, "len", span)?;
        match v {
            Value::Str(s) => Ok(Value::Int(s.chars().count() as i64)),
            Value::List(l) => Ok(Value::Int(l.borrow().len() as i64)),
            Value::Map(m) => Ok(Value::Int(m.borrow().len() as i64)),
            Value::Range(r) => Ok(Value::Int((r.end - r.start).max(0))),
            other => Err(err(
                codes::TYPE_MISMATCH,
                format!("`len` does not accept {}", other.type_name()),
                span,
            )),
        }
    });
    it.native("to_string", |_it, args, span| {
        Ok(Value::str(arg(&args, 0, "to_string", span)?.display()))
    });
    it.native("to_int", |_it, args, span| {
        let v = arg(&args, 0, "to_int", span)?;
        match v {
            Value::Int(i) => Ok(Value::Int(*i)),
            Value::Float(f) => Ok(Value::Int(*f as i64)),
            Value::Bool(b) => Ok(Value::Int(i64::from(*b))),
            Value::Str(s) => s.trim().parse::<i64>().map(Value::Int).map_err(|_| {
                err(
                    codes::TYPE_MISMATCH,
                    format!("cannot parse \"{s}\" as int"),
                    span,
                )
            }),
            other => Err(err(
                codes::TYPE_MISMATCH,
                format!("`to_int` does not accept {}", other.type_name()),
                span,
            )),
        }
    });
    it.native("to_float", |_it, args, span| {
        let v = arg(&args, 0, "to_float", span)?;
        match v {
            Value::Int(i) => Ok(Value::Float(*i as f64)),
            Value::Float(f) => Ok(Value::Float(*f)),
            Value::Bool(b) => Ok(Value::Float(if *b { 1.0 } else { 0.0 })),
            Value::Str(s) => s.trim().parse::<f64>().map(Value::Float).map_err(|_| {
                err(
                    codes::TYPE_MISMATCH,
                    format!("cannot parse \"{s}\" as float"),
                    span,
                )
            }),
            other => Err(err(
                codes::TYPE_MISMATCH,
                format!("`to_float` does not accept {}", other.type_name()),
                span,
            )),
        }
    });
    it.native("range", |_it, args, span| {
        let (start, end) = match args.len() {
            1 => (0, as_int(&args[0], "range", span)?),
            2 => (
                as_int(&args[0], "range", span)?,
                as_int(&args[1], "range", span)?,
            ),
            n => {
                return Err(err(
                    codes::TYPE_MISMATCH,
                    format!("`range` takes 1 or 2 arguments, got {n}"),
                    span,
                ))
            }
        };
        Ok(Value::Range(Rc::new(RangeVal { start, end })))
    });
    it.native("abs", |_it, args, span| match arg(&args, 0, "abs", span)? {
        Value::Int(i) => i
            .checked_abs()
            .map(Value::Int)
            .ok_or_else(|| err(codes::OVERFLOW, "integer overflow", span)),
        Value::Float(f) => Ok(Value::Float(f.abs())),
        other => Err(err(
            codes::TYPE_MISMATCH,
            format!("`abs` does not accept {}", other.type_name()),
            span,
        )),
    });
    it.native("min", |_it, args, span| {
        let a = arg(&args, 0, "min", span)?;
        let b = arg(&args, 1, "min", span)?;
        match a.cmp_val(b) {
            Some(std::cmp::Ordering::Greater) => Ok(b.clone()),
            _ => Ok(a.clone()),
        }
    });
    it.native("max", |_it, args, span| {
        let a = arg(&args, 0, "max", span)?;
        let b = arg(&args, 1, "max", span)?;
        match a.cmp_val(b) {
            Some(std::cmp::Ordering::Less) => Ok(b.clone()),
            _ => Ok(a.clone()),
        }
    });
    it.native("push", |_it, args, span| {
        let list = arg(&args, 0, "push", span)?;
        let item = arg(&args, 1, "push", span)?.clone();
        match list {
            Value::List(l) => {
                l.borrow_mut().push(item);
                Ok(Value::None)
            }
            other => Err(err(
                codes::TYPE_MISMATCH,
                format!("`push` expects a list, found {}", other.type_name()),
                span,
            )),
        }
    });
    it.native("keys", |_it, args, span| {
        match arg(&args, 0, "keys", span)? {
            Value::Map(m) => Ok(Value::list(m.borrow().keys().map(Value::str).collect())),
            other => Err(err(
                codes::TYPE_MISMATCH,
                format!("`keys` expects a map, found {}", other.type_name()),
                span,
            )),
        }
    });
    it.native("values", |_it, args, span| {
        match arg(&args, 0, "values", span)? {
            Value::Map(m) => Ok(Value::list(m.borrow().values().cloned().collect())),
            other => Err(err(
                codes::TYPE_MISMATCH,
                format!("`values` expects a map, found {}", other.type_name()),
                span,
            )),
        }
    });
    it.native("sort", |_it, args, span| {
        let list = arg(&args, 0, "sort", span)?;
        match list {
            Value::List(l) => {
                let mut out = l.borrow().clone();
                out.sort_by(|a, b| a.cmp_val(b).unwrap_or(std::cmp::Ordering::Equal));
                Ok(Value::list(out))
            }
            other => Err(err(
                codes::TYPE_MISMATCH,
                format!("`sort` expects a list, found {}", other.type_name()),
                span,
            )),
        }
    });
    it.native("reverse", |_it, args, span| {
        match arg(&args, 0, "reverse", span)? {
            Value::List(l) => {
                let mut out = l.borrow().clone();
                out.reverse();
                Ok(Value::list(out))
            }
            Value::Str(s) => Ok(Value::str(s.chars().rev().collect::<String>())),
            other => Err(err(
                codes::TYPE_MISMATCH,
                format!(
                    "`reverse` expects a list or string, found {}",
                    other.type_name()
                ),
                span,
            )),
        }
    });
    it.native("map", |it, args, span| {
        let list = arg(&args, 0, "map", span)?.clone();
        let f = arg(&args, 1, "map", span)?.clone();
        let snapshot = as_list(&list, "map", span)?;
        let mut out = Vec::with_capacity(snapshot.len());
        for item in snapshot {
            out.push(it.call_value_pub(f.clone(), vec![item], span)?);
        }
        Ok(Value::list(out))
    });
    it.native("filter", |it, args, span| {
        let list = arg(&args, 0, "filter", span)?.clone();
        let f = arg(&args, 1, "filter", span)?.clone();
        let snapshot = as_list(&list, "filter", span)?;
        let mut out = Vec::new();
        for item in snapshot {
            let keep = it.call_value_pub(f.clone(), vec![item.clone()], span)?;
            if keep.truthy() {
                out.push(item);
            }
        }
        Ok(Value::list(out))
    });
    it.native("reduce", |it, args, span| {
        let list = arg(&args, 0, "reduce", span)?.clone();
        let f = arg(&args, 1, "reduce", span)?.clone();
        let mut acc = arg(&args, 2, "reduce", span)?.clone();
        let snapshot = as_list(&list, "reduce", span)?;
        for item in snapshot {
            acc = it.call_value_pub(f.clone(), vec![acc, item], span)?;
        }
        Ok(acc)
    });
    it.native("sum", |_it, args, span| {
        let list = as_list(&arg(&args, 0, "sum", span)?.clone(), "sum", span)?;
        let mut int_sum: i64 = 0;
        let mut float_sum: f64 = 0.0;
        let mut any_float = false;
        for v in &list {
            match v {
                Value::Int(i) => {
                    int_sum = int_sum
                        .checked_add(*i)
                        .ok_or_else(|| err(codes::OVERFLOW, "integer overflow in sum", span))?;
                    float_sum += *i as f64;
                }
                Value::Float(f) => {
                    any_float = true;
                    float_sum += *f;
                }
                other => {
                    return Err(err(
                        codes::TYPE_MISMATCH,
                        format!("`sum` cannot add {}", other.type_name()),
                        span,
                    ))
                }
            }
        }
        Ok(if any_float {
            Value::Float(float_sum)
        } else {
            Value::Int(int_sum)
        })
    });
    it.native("assert", |_it, args, span| {
        let cond = arg(&args, 0, "assert", span)?;
        if cond.truthy() {
            Ok(Value::None)
        } else {
            let msg = args
                .get(1)
                .map_or_else(|| "assertion failed".to_string(), Value::display);
            Err(err(codes::FOREIGN, msg, span))
        }
    });
    it.native("enumerate", |_it, args, span| {
        let list = as_list(
            &arg(&args, 0, "enumerate", span)?.clone(),
            "enumerate",
            span,
        )?;
        let pairs = list
            .into_iter()
            .enumerate()
            .map(|(i, v)| Value::list(vec![Value::Int(i as i64), v]))
            .collect();
        Ok(Value::list(pairs))
    });
    it.native("zip", |_it, args, span| {
        let a = as_list(&arg(&args, 0, "zip", span)?.clone(), "zip", span)?;
        let b = as_list(&arg(&args, 1, "zip", span)?.clone(), "zip", span)?;
        let pairs = a
            .into_iter()
            .zip(b)
            .map(|(x, y)| Value::list(vec![x, y]))
            .collect();
        Ok(Value::list(pairs))
    });
}

/// Extract a list snapshot or produce a diagnostic.
fn as_list(v: &Value, what: &str, span: Span) -> Result<Vec<Value>> {
    match v {
        Value::List(l) => Ok(l.borrow().clone()),
        other => Err(err(
            codes::TYPE_MISMATCH,
            format!("`{what}` expects a list, found {}", other.type_name()),
            span,
        )),
    }
}

fn writeln_line(it: &mut Interp, line: &str) {
    use std::io::Write;
    let _ = writeln!(it.stdout, "{line}");
}

/// Method dispatch for strings, lists, and maps.
pub fn method(
    it: &mut Interp,
    recv: &Value,
    name: &str,
    args: Vec<Value>,
    span: Span,
) -> Result<Value> {
    match recv {
        Value::Str(s) => string_method(it, s, name, args, span),
        Value::List(l) => list_method(it, l, name, args, span),
        Value::Map(m) => map_method(it, m, name, args, span),
        Value::Range(r) => match name {
            "len" => Ok(Value::Int((r.end - r.start).max(0))),
            _ => Err(no_method("range", name, span)),
        },
        other => Err(no_method(other.type_name(), name, span)),
    }
}

fn no_method(ty: &str, name: &str, span: Span) -> Diag {
    err(
        codes::UNDEFINED,
        format!("{ty} has no method `{name}`"),
        span,
    )
}

fn string_method(
    _it: &mut Interp,
    s: &Rc<str>,
    name: &str,
    args: Vec<Value>,
    span: Span,
) -> Result<Value> {
    match name {
        "len" => Ok(Value::Int(s.chars().count() as i64)),
        "upper" | "up" => Ok(Value::str(s.to_uppercase())),
        "lower" | "down" => Ok(Value::str(s.to_lowercase())),
        "trim" => Ok(Value::str(s.trim().to_string())),
        "contains" => {
            let needle = arg(&args, 0, "contains", span)?;
            match needle {
                Value::Str(n) => Ok(Value::Bool(s.contains(&**n))),
                other => Err(err(
                    codes::TYPE_MISMATCH,
                    format!("contains expects a string, found {}", other.type_name()),
                    span,
                )),
            }
        }
        "starts_with" => match arg(&args, 0, "starts_with", span)? {
            Value::Str(n) => Ok(Value::Bool(s.starts_with(&**n))),
            _ => Err(err(
                codes::TYPE_MISMATCH,
                "starts_with expects a string",
                span,
            )),
        },
        "ends_with" => match arg(&args, 0, "ends_with", span)? {
            Value::Str(n) => Ok(Value::Bool(s.ends_with(&**n))),
            _ => Err(err(
                codes::TYPE_MISMATCH,
                "ends_with expects a string",
                span,
            )),
        },
        "split" => {
            let sep = arg(&args, 0, "split", span)?;
            match sep {
                Value::Str(sep) => {
                    let parts: Vec<Value> = if sep.is_empty() {
                        s.chars().map(|c| Value::str(c.to_string())).collect()
                    } else {
                        s.split(&**sep).map(Value::str).collect()
                    };
                    Ok(Value::list(parts))
                }
                other => Err(err(
                    codes::TYPE_MISMATCH,
                    format!(
                        "split expects a string separator, found {}",
                        other.type_name()
                    ),
                    span,
                )),
            }
        }
        "replace" => {
            let from = match arg(&args, 0, "replace", span)? {
                Value::Str(v) => v.clone(),
                _ => return Err(err(codes::TYPE_MISMATCH, "replace expects strings", span)),
            };
            let to = match arg(&args, 1, "replace", span)? {
                Value::Str(v) => v.clone(),
                _ => return Err(err(codes::TYPE_MISMATCH, "replace expects strings", span)),
            };
            Ok(Value::str(s.replace(from.as_ref(), to.as_ref())))
        }
        "chars" => Ok(Value::list(
            s.chars().map(|c| Value::str(c.to_string())).collect(),
        )),
        _ => Err(no_method("string", name, span)),
    }
}

fn list_method(
    it: &mut Interp,
    l: &Rc<RefCell<Vec<Value>>>,
    name: &str,
    args: Vec<Value>,
    span: Span,
) -> Result<Value> {
    match name {
        "len" => Ok(Value::Int(l.borrow().len() as i64)),
        "push" => {
            let item = arg(&args, 0, "push", span)?.clone();
            l.borrow_mut().push(item);
            Ok(Value::None)
        }
        "pop" => Ok(l.borrow_mut().pop().unwrap_or(Value::None)),
        "first" => Ok(l.borrow().first().cloned().unwrap_or(Value::None)),
        "last" => Ok(l.borrow().last().cloned().unwrap_or(Value::None)),
        "join" => {
            let sep = match arg(&args, 0, "join", span)? {
                Value::Str(s) => s.clone(),
                other => {
                    return Err(err(
                        codes::TYPE_MISMATCH,
                        format!(
                            "join expects a string separator, found {}",
                            other.type_name()
                        ),
                        span,
                    ))
                }
            };
            let parts: Vec<String> = l.borrow().iter().map(Value::display).collect();
            Ok(Value::str(parts.join(&*sep)))
        }
        "contains" => {
            let needle = arg(&args, 0, "contains", span)?;
            Ok(Value::Bool(l.borrow().iter().any(|v| v.equals(needle))))
        }
        "sort" => {
            let mut out = l.borrow().clone();
            out.sort_by(|a, b| a.cmp_val(b).unwrap_or(std::cmp::Ordering::Equal));
            Ok(Value::list(out))
        }
        "reverse" => {
            let mut out = l.borrow().clone();
            out.reverse();
            Ok(Value::list(out))
        }
        "map" => {
            let f = arg(&args, 0, "map", span)?.clone();
            let snapshot = l.borrow().clone();
            let mut out = Vec::with_capacity(snapshot.len());
            for item in snapshot {
                out.push(it.call_value_pub(f.clone(), vec![item], span)?);
            }
            Ok(Value::list(out))
        }
        "filter" => {
            let f = arg(&args, 0, "filter", span)?.clone();
            let snapshot = l.borrow().clone();
            let mut out = Vec::new();
            for item in snapshot {
                let keep = it.call_value_pub(f.clone(), vec![item.clone()], span)?;
                if keep.truthy() {
                    out.push(item);
                }
            }
            Ok(Value::list(out))
        }
        "reduce" => {
            let f = arg(&args, 0, "reduce", span)?.clone();
            let mut acc = arg(&args, 1, "reduce", span)?.clone();
            let snapshot = l.borrow().clone();
            for item in snapshot {
                acc = it.call_value_pub(f.clone(), vec![acc, item], span)?;
            }
            Ok(acc)
        }
        _ => Err(no_method("list", name, span)),
    }
}

fn map_method(
    _it: &mut Interp,
    m: &Rc<RefCell<std::collections::BTreeMap<String, Value>>>,
    name: &str,
    args: Vec<Value>,
    span: Span,
) -> Result<Value> {
    match name {
        "len" => Ok(Value::Int(m.borrow().len() as i64)),
        "get" => {
            let k = match arg(&args, 0, "get", span)? {
                Value::Str(s) => s.to_string(),
                other => other.display(),
            };
            Ok(m.borrow().get(&k).cloned().unwrap_or(Value::None))
        }
        "has" => {
            let k = match arg(&args, 0, "has", span)? {
                Value::Str(s) => s.to_string(),
                other => other.display(),
            };
            Ok(Value::Bool(m.borrow().contains_key(&k)))
        }
        "keys" => Ok(Value::list(m.borrow().keys().map(Value::str).collect())),
        "values" => Ok(Value::list(m.borrow().values().cloned().collect())),
        "remove" => {
            let k = match arg(&args, 0, "remove", span)? {
                Value::Str(s) => s.to_string(),
                other => other.display(),
            };
            Ok(m.borrow_mut().remove(&k).unwrap_or(Value::None))
        }
        _ => Err(no_method("map", name, span)),
    }
}
