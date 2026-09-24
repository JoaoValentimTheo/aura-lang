//! Python interop through PyO3.
//!
//! With the `py` feature, `py_eval("...")` and `py_import("module")` expose
//! CPython to Aura. Values cross the boundary structurally: ints, floats,
//! strings, bools, `none`, lists, and dicts map to their Python equivalents
//! and back.
//!
//! Without the feature, the module is inert and the functions are absent,
//! so the binary links no CPython.

#[cfg(not(feature = "py"))]
use crate::error::{codes, Diag, Span};
use crate::run::Interp;

#[cfg(not(feature = "py"))]
fn unavailable(name: &'static str, span: Span) -> Diag {
    Diag::new(
        codes::PY_UNSUPPORTED,
        format!("`{name}` requires a build with the `py` feature (CPython interop)"),
        span,
    )
}

/// Install Python bridge functions.
///
/// With the `py` feature, the real PyO3 bridge is installed. Without it, the
/// same names are installed as stubs that reject the call with `E5002`, so the
/// static checker and the runtime agree that the functions exist while making
/// the missing capability an explicit, documented diagnostic.
pub fn install(it: &mut Interp) {
    #[cfg(feature = "py")]
    py::install(it);
    #[cfg(not(feature = "py"))]
    {
        for name in ["py_eval", "py_import", "py_call", "py_version"] {
            let owned: &'static str = Box::leak(name.to_string().into_boxed_str());
            it.native(
                name,
                move |_it: &mut Interp, _args: Vec<crate::run::value::Value>, span: Span| {
                    Err(unavailable(owned, span))
                },
            );
        }
    }
}

#[cfg(feature = "py")]
mod py {
    use crate::error::{codes, Diag, Result, Span};
    use crate::run::value::Value;
    use crate::run::Interp;
    use pyo3::prelude::*;
    use pyo3::types::{PyAnyMethods, PyDict, PyInt, PyList, PyModule, PyTuple};
    use std::cell::RefCell;
    use std::collections::BTreeMap;
    use std::rc::Rc;
    use std::sync::Once;

    fn err(msg: impl Into<String>, span: Span) -> Diag {
        Diag::new(codes::PY_ERROR, msg, span)
    }

    static INIT: Once = Once::new();

    fn ensure_init() {
        INIT.call_once(|| {
            Python::attach(|py| {
                let _ = py.version_info();
            });
        });
    }

    /// Python object -> Aura value.
    fn to_value(obj: &Bound<'_, PyAny>) -> Result<Value> {
        to_value_depth(obj, 0)
    }

    /// Bounded-depth conversion so a deeply nested Python object cannot
    /// overflow the native stack. Beyond the bound, the object becomes its
    /// `repr`.
    fn to_value_depth(obj: &Bound<'_, PyAny>, depth: usize) -> Result<Value> {
        if depth >= crate::run::value::MAX_VALUE_DEPTH {
            let repr = obj.repr().map_err(map_pyerr)?.to_string();
            return Ok(Value::str(repr));
        }
        if obj.is_none() {
            return Ok(Value::None);
        }
        if let Ok(b) = obj.extract::<bool>() {
            return Ok(Value::Bool(b));
        }
        // Check `int` *before* `float` and reject values outside `i64` rather
        // than silently demoting them to a `float`, which would lose
        // precision. Python's `int` is arbitrary precision.
        if obj.is_instance_of::<PyInt>() {
            if let Ok(i) = obj.extract::<i64>() {
                return Ok(Value::Int(i));
            }
            let text = obj.str().map_err(map_pyerr)?.to_string();
            return Err(Diag::new(
                codes::OVERFLOW,
                format!("Python integer `{text}` does not fit in Aura's 64-bit `int`"),
                Span::default(),
            ));
        }
        if let Ok(f) = obj.extract::<f64>() {
            return Ok(Value::Float(f));
        }
        if let Ok(s) = obj.extract::<String>() {
            return Ok(Value::str(s));
        }
        if let Ok(list) = obj.cast::<PyList>() {
            let mut out = Vec::new();
            for item in list.iter() {
                out.push(to_value_depth(&item, depth + 1)?);
            }
            return Ok(Value::list(out));
        }
        if let Ok(dict) = obj.cast::<PyDict>() {
            let mut map = BTreeMap::new();
            for (k, v) in dict.iter() {
                // Aura maps are string-keyed. Refuse to stringify arbitrary
                // keys, which would collapse distinct keys (e.g. `1` and
                // `"1"`) into one.
                let Ok(key) = k.extract::<String>() else {
                    let shown = k.str().map_err(map_pyerr)?.to_string();
                    return Err(Diag::new(
                        codes::PY_UNSUPPORTED,
                        format!(
                            "Python dict key `{shown}` is not a string; Aura maps are string-keyed"
                        ),
                        Span::default(),
                    ));
                };
                map.insert(key, to_value_depth(&v, depth + 1)?);
            }
            return Ok(Value::Map(Rc::new(RefCell::new(map))));
        }
        // Fallback: render as a string, so opaque objects remain printable.
        let repr = obj.repr().map_err(map_pyerr)?.to_string();
        Ok(Value::str(repr))
    }

    /// Aura value -> Python object.
    fn to_py<'py>(py: Python<'py>, v: &Value) -> Result<Bound<'py, PyAny>> {
        to_py_depth(py, v, 0)
    }

    /// Bounded-depth conversion so a deeply nested Aura value cannot overflow
    /// the native stack when crossing into Python. Beyond the bound, the
    /// value is rejected rather than risk a host overflow.
    fn to_py_depth<'py>(py: Python<'py>, v: &Value, depth: usize) -> Result<Bound<'py, PyAny>> {
        if depth >= crate::run::value::MAX_VALUE_DEPTH {
            return Err(Diag::new(
                codes::PY_UNSUPPORTED,
                "value is nested too deeply to cross into Python",
                Span::default(),
            ));
        }
        Ok(match v {
            Value::None => py.None().into_bound(py),
            Value::Bool(b) => b
                .into_pyobject(py)
                .map_err(map_conversion)?
                .to_owned()
                .into_any(),
            Value::Int(i) => i.into_pyobject(py).map_err(map_conversion)?.into_any(),
            Value::Float(f) => f.into_pyobject(py).map_err(map_conversion)?.into_any(),
            Value::Str(s) => s
                .to_string()
                .into_pyobject(py)
                .map_err(map_conversion)?
                .into_any(),
            Value::List(l) => {
                let list = PyList::empty(py);
                for item in l.borrow().iter() {
                    list.append(to_py_depth(py, item, depth + 1)?)
                        .map_err(map_pyerr)?;
                }
                list.into_any()
            }
            Value::Map(m) => {
                let dict = PyDict::new(py);
                for (k, val) in m.borrow().iter() {
                    dict.set_item(k, to_py_depth(py, val, depth + 1)?)
                        .map_err(map_pyerr)?;
                }
                dict.into_any()
            }
            other => {
                return Err(Diag::new(
                    codes::PY_UNSUPPORTED,
                    format!(
                        "value of type {} cannot cross into Python",
                        other.type_name()
                    ),
                    Span::default(),
                ))
            }
        })
    }

    fn map_pyerr(e: PyErr) -> Diag {
        Diag::new(codes::PY_ERROR, format!("python: {e}"), Span::default())
    }

    /// Convert any `into_pyobject` error (which implements `Into<PyErr>`)
    /// into an Aura diagnostic.
    fn map_conversion<E: Into<PyErr>>(e: E) -> Diag {
        map_pyerr(e.into())
    }

    pub(super) fn install(it: &mut Interp) {
        it.native("py_eval", |_it, args, span| {
            let Some(Value::Str(code)) = args.first() else {
                return Err(err("py_eval expects a string of Python code", span));
            };
            ensure_init();
            Python::attach(|py| {
                let c = std::ffi::CString::new(&**code)
                    .map_err(|_| err("Python code contains a NUL byte", span))?;
                let result = py.eval(c.as_c_str(), None, None).map_err(map_pyerr)?;
                to_value(&result)
            })
        });
        it.native("py_import", |_it, args, span| {
            let Some(Value::Str(name)) = args.first() else {
                return Err(err("py_import expects a module name string", span));
            };
            ensure_init();
            Python::attach(|py| {
                let module = PyModule::import(py, name.as_ref()).map_err(map_pyerr)?;
                // Return a dict of the module's public attributes as strings.
                let mut map = BTreeMap::new();
                for (k, _) in module.dict().iter() {
                    let key = k.str().map_err(map_pyerr)?.to_string();
                    if !key.starts_with('_') {
                        map.insert(key, Value::str("<python>"));
                    }
                }
                Ok(Value::Map(Rc::new(RefCell::new(map))))
            })
        });
        it.native("py_call", |_it, args, span| {
            // py_call(module, attr, args...)
            let module = match args.first() {
                Some(Value::Str(s)) => s.to_string(),
                _ => return Err(err("py_call expects (module, attribute, ...)", span)),
            };
            let attr = match args.get(1) {
                Some(Value::Str(s)) => s.to_string(),
                _ => return Err(err("py_call expects (module, attribute, ...)", span)),
            };
            let call_args = args.get(2..).unwrap_or(&[]);
            ensure_init();
            Python::attach(|py| {
                let module = PyModule::import(py, module.as_str()).map_err(map_pyerr)?;
                let func = module.getattr(attr.as_str()).map_err(map_pyerr)?;
                let owned: Vec<Bound<'_, PyAny>> = call_args
                    .iter()
                    .map(|v| to_py(py, v))
                    .collect::<Result<Vec<_>>>()?;
                let tuple = PyTuple::new(py, owned).map_err(map_pyerr)?;
                let result = func.call1(&tuple).map_err(map_pyerr)?;
                to_value(&result)
            })
        });
        it.native("py_version", |_it, _args, _span| {
            ensure_init();
            ensure_init();
            Python::attach(|_py| Ok(Value::str(::pyo3::Python::version_str())))
        });
    }
}
