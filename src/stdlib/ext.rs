//! Feature-gated standard library modules: `json`, `regex`, `time`.
//!
//! Each is compiled only when its Cargo feature is on. None of them require
//! Python.

#[cfg(feature = "json")]
pub mod json {
    //! JSON encode/decode.

    use crate::error::{codes, Diag, Span};
    use crate::run::value::Value;
    use crate::run::Interp;
    use std::cell::RefCell;
    use std::collections::BTreeMap;
    use std::rc::Rc;

    fn err(msg: impl Into<String>, span: Span) -> Diag {
        Diag::new(codes::TYPE_MISMATCH, msg, span)
    }

    fn to_json(v: &Value) -> serde_json::Value {
        to_json_depth(v, 0)
    }

    /// Convert an Aura value to JSON, bounded by [`Value::MAX_VALUE_DEPTH`]-style
    /// depth so a cyclic or pathologically deep value cannot overflow the
    /// native stack. A value nested beyond the bound serializes as `null`
    /// (`LANGUAGE_SPEC.md` §31.5).
    fn to_json_depth(v: &Value, depth: usize) -> serde_json::Value {
        if depth >= crate::run::value::MAX_VALUE_DEPTH {
            return serde_json::Value::Null;
        }
        match v {
            Value::None => serde_json::Value::Null,
            Value::Bool(b) => serde_json::Value::Bool(*b),
            Value::Int(i) => serde_json::Value::Number((*i).into()),
            Value::Float(f) => serde_json::Number::from_f64(*f)
                .map_or(serde_json::Value::Null, serde_json::Value::Number),
            Value::Str(s) => serde_json::Value::String(s.to_string()),
            Value::List(l) => serde_json::Value::Array(
                l.borrow()
                    .iter()
                    .map(|x| to_json_depth(x, depth + 1))
                    .collect(),
            ),
            Value::Map(m) => {
                let mut obj = serde_json::Map::new();
                for (k, v) in m.borrow().iter() {
                    obj.insert(k.clone(), to_json_depth(v, depth + 1));
                }
                serde_json::Value::Object(obj)
            }
            Value::Instance(i) => {
                let mut obj = serde_json::Map::new();
                for (k, v) in i.fields.borrow().iter() {
                    obj.insert(k.clone(), to_json_depth(v, depth + 1));
                }
                serde_json::Value::Object(obj)
            }
            Value::Variant(v) => {
                if v.payload.is_empty() {
                    serde_json::Value::String(v.tag.clone())
                } else if v.payload.len() == 1 {
                    to_json_depth(&v.payload[0], depth + 1)
                } else {
                    serde_json::Value::Array(
                        v.payload
                            .iter()
                            .map(|x| to_json_depth(x, depth + 1))
                            .collect(),
                    )
                }
            }
            _ => serde_json::Value::Null,
        }
    }

    fn from_json(j: &serde_json::Value) -> Value {
        from_json_depth(j, 0)
    }

    /// Decode JSON, bounded in depth so a hostile deeply nested document
    /// cannot overflow the native stack. serde_json's parser already limits
    /// nesting before this point; the bound keeps the conversion total.
    fn from_json_depth(j: &serde_json::Value, depth: usize) -> Value {
        if depth >= crate::run::value::MAX_VALUE_DEPTH {
            return Value::None;
        }
        match j {
            serde_json::Value::Null => Value::None,
            serde_json::Value::Bool(b) => Value::Bool(*b),
            serde_json::Value::Number(n) => n
                .as_i64()
                .map_or_else(|| Value::Float(n.as_f64().unwrap_or(0.0)), Value::Int),
            serde_json::Value::String(s) => Value::str(s.clone()),
            serde_json::Value::Array(a) => {
                Value::list(a.iter().map(|x| from_json_depth(x, depth + 1)).collect())
            }
            serde_json::Value::Object(o) => {
                let mut m = BTreeMap::new();
                for (k, v) in o {
                    m.insert(k.clone(), from_json_depth(v, depth + 1));
                }
                Value::Map(Rc::new(RefCell::new(m)))
            }
        }
    }

    /// Install `json_encode` and `json_decode`.
    pub fn install(it: &mut Interp) {
        it.native("json_encode", |_it, args, span| {
            let v = args
                .first()
                .ok_or_else(|| err("json_encode expects a value", span))?;
            Ok(Value::str(
                serde_json::to_string(&to_json(v)).map_err(|e| err(e.to_string(), span))?,
            ))
        });
        it.native("json_decode", |_it, args, span| {
            let Some(Value::Str(s)) = args.first() else {
                return Err(err("json_decode expects a string", span));
            };
            let j: serde_json::Value =
                serde_json::from_str(s).map_err(|e| err(e.to_string(), span))?;
            Ok(from_json(&j))
        });
    }
}

#[cfg(feature = "regex")]
pub mod regex {
    //! Regular expressions backed by the `regex` crate.

    use crate::error::{codes, Diag, Result, Span};
    use crate::run::value::Value;
    use crate::run::Interp;
    use ::regex::Regex;

    fn err(msg: impl Into<String>, span: Span) -> Diag {
        Diag::new(codes::TYPE_MISMATCH, msg, span)
    }

    fn compile(pat: &str, span: Span) -> Result<Regex> {
        Regex::new(pat).map_err(|e| err(format!("invalid regex: {e}"), span))
    }

    /// Install regex functions.
    pub fn install(it: &mut Interp) {
        it.native("regex_match", |_it, args, span| {
            let (Some(Value::Str(pat)), Some(Value::Str(text))) = (args.first(), args.get(1))
            else {
                return Err(err("regex_match expects (pattern, text)", span));
            };
            Ok(Value::Bool(compile(pat, span)?.is_match(text)))
        });
        it.native("regex_find", |_it, args, span| {
            let (Some(Value::Str(pat)), Some(Value::Str(text))) = (args.first(), args.get(1))
            else {
                return Err(err("regex_find expects (pattern, text)", span));
            };
            Ok(compile(pat, span)?
                .find(text)
                .map_or(Value::None, |m| Value::str(m.as_str())))
        });
        it.native("regex_find_all", |_it, args, span| {
            let (Some(Value::Str(pat)), Some(Value::Str(text))) = (args.first(), args.get(1))
            else {
                return Err(err("regex_find_all expects (pattern, text)", span));
            };
            let re = compile(pat, span)?;
            Ok(Value::list(
                re.find_iter(text).map(|m| Value::str(m.as_str())).collect(),
            ))
        });
        it.native("regex_replace", |_it, args, span| {
            let (Some(Value::Str(pat)), Some(Value::Str(text)), Some(Value::Str(rep))) =
                (args.first(), args.get(1), args.get(2))
            else {
                return Err(err(
                    "regex_replace expects (pattern, text, replacement)",
                    span,
                ));
            };
            Ok(Value::str(
                compile(pat, span)?
                    .replace_all(text, rep.as_ref())
                    .into_owned(),
            ))
        });
    }
}

#[cfg(feature = "time")]
pub mod time {
    //! Time and date functions.
    //!
    //! These obtain their clock through the interpreter's host capability
    //! boundary, so a browser host with no clock reports `E5002` rather than
    //! reaching for the operating system.

    use crate::error::{codes, Diag, Span};
    use crate::run::value::Value;
    use crate::run::Interp;
    use std::cell::RefCell;
    use std::collections::BTreeMap;
    use std::rc::Rc;

    fn err(msg: impl Into<String>, span: Span) -> Diag {
        Diag::new(codes::TYPE_MISMATCH, msg, span)
    }

    /// Install time functions.
    pub fn install(it: &mut Interp) {
        it.native("time_now", |it, _args, span| {
            let now = it.host().now_local().map_err(|e| e.into_diag(span))?;
            let mut m = BTreeMap::new();
            m.insert("year".to_string(), Value::Int(i64::from(now.year)));
            m.insert("month".to_string(), Value::Int(i64::from(now.month)));
            m.insert("day".to_string(), Value::Int(i64::from(now.day)));
            m.insert("hour".to_string(), Value::Int(i64::from(now.hour)));
            m.insert("minute".to_string(), Value::Int(i64::from(now.minute)));
            m.insert("second".to_string(), Value::Int(i64::from(now.second)));
            m.insert("unix".to_string(), Value::Int(now.unix));
            Ok(Value::Map(Rc::new(RefCell::new(m))))
        });
        it.native("time_unix", |it, _args, span| {
            Ok(Value::Int(
                it.host().now_unix().map_err(|e| e.into_diag(span))?,
            ))
        });
        it.native("sleep_ms", |it, args, span| {
            let Some(Value::Int(ms)) = args.first() else {
                return Err(err(
                    "sleep_ms expects an integer number of milliseconds",
                    span,
                ));
            };
            let Ok(ms) = u64::try_from(*ms) else {
                return Err(err("sleep_ms expects a non-negative integer", span));
            };
            it.host_mut().sleep_ms(ms).map_err(|e| e.into_diag(span))?;
            Ok(Value::None)
        });
    }
}
