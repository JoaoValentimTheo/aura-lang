//! Python interop through PyO3.
//!
//! With the `py` feature, `py_eval("...")` and `py_import("module")` expose
//! CPython to Aura. Values cross the boundary structurally: ints, floats,
//! strings, bools, `none`, lists, and dicts map to their Python equivalents
//! and back.
//!
//! Without the feature, the module is inert and the functions are absent,
//! so the binary links no CPython.

use crate::run::Interp;

/// Install Python bridge functions if the feature is enabled.
pub fn install(it: &mut Interp) {
    #[cfg(feature = "py")]
    py::install(it);
    #[cfg(not(feature = "py"))]
    let _ = it;
}

#[cfg(feature = "py")]
mod py {
    use crate::error::{codes, Diag, Result, Span};
    use crate::run::value::Value;
    use crate::run::Interp;
    use pyo3::prelude::*;
    use pyo3::types::{PyAnyMethods, PyDict, PyList, PyModule, PyTuple};
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
        if obj.is_none() {
            return Ok(Value::None);
        }
        if let Ok(b) = obj.extract::<bool>() {
            return Ok(Value::Bool(b));
        }
        if let Ok(i) = obj.extract::<i64>() {
            return Ok(Value::Int(i));
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
                out.push(to_value(&item)?);
            }
            return Ok(Value::list(out));
        }
        if let Ok(dict) = obj.cast::<PyDict>() {
            let mut map = BTreeMap::new();
            for (k, v) in dict.iter() {
                let key = k.str().map_err(map_pyerr)?.to_string();
                map.insert(key, to_value(&v)?);
            }
            return Ok(Value::Map(Rc::new(RefCell::new(map))));
        }
        // Fallback: render as a string, so opaque objects remain printable.
        let repr = obj.repr().map_err(map_pyerr)?.to_string();
        Ok(Value::str(repr))
    }

    /// Aura value -> Python object.
    fn to_py<'py>(py: Python<'py>, v: &Value) -> Result<Bound<'py, PyAny>> {
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
                    list.append(to_py(py, item)?).map_err(map_pyerr)?;
                }
                list.into_any()
            }
            Value::Map(m) => {
                let dict = PyDict::new(py);
                for (k, val) in m.borrow().iter() {
                    dict.set_item(k, to_py(py, val)?).map_err(map_pyerr)?;
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
