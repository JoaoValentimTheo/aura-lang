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
            it.native(
                name,
                move |_it: &mut Interp, _args: Vec<crate::run::value::Value>, span: Span| {
                    Err(unavailable(name, span))
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
    use pyo3::types::{
        PyAnyMethods, PyBool, PyDict, PyFloat, PyInt, PyList, PyModule, PyString, PyTuple,
    };
    use std::cell::RefCell;
    use std::collections::{BTreeMap, HashSet};
    use std::rc::Rc;
    use std::sync::Once;

    /// Maximum number of Python container/leaf nodes a single Aura value ->
    /// Python conversion may visit.
    ///
    /// The depth bound alone does not bound *work*: a shared (aliased) Aura
    /// subvalue, or a cyclic one built before it crosses, is re-expanded at
    /// every occurrence, so the conversion grows exponentially and never
    /// returns (AUDIT-4's mechanism, on the Python boundary). This budget
    /// bounds the total nodes visited so the crossing is total; like the depth
    /// bound it rejects rather than silently truncating, so no partial Python
    /// object is ever handed back.
    const MAX_PY_NODES: usize = 1_000_000;

    /// Maximum number of Python nodes a single Python object -> Aura value
    /// conversion may visit, for the same reason: a Python container that
    /// aliases or cycles (`c.append(c)` twice) would otherwise re-expand
    /// without bound. A Python object graph is *untracked* by Aura's `Rc`
    /// value model, so identity here is the Python object's address
    /// (`PyAny::as_ptr`), which is stable for the duration of the conversion.
    const MAX_PY_SOURCE_NODES: usize = 1_000_000;

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

    /// Python object -> Aura value. `span` is the Aura call site, attached to
    /// every diagnostic so a Python-side failure points at the crossing.
    fn to_value(obj: &Bound<'_, PyAny>, span: Span) -> Result<Value> {
        let mut budget = MAX_PY_SOURCE_NODES;
        // Python identity of every container already on the current descent
        // path: a reference back to one of these is a cycle, and is rejected
        // rather than followed (Aura has no way to represent it — `Rc` cannot
        // be satisfied). A reference to a container *off* the path is ordinary
        // sharing and is converted again (correct, and bounded by the node
        // budget).
        let mut path: Vec<usize> = Vec::new();
        to_value_depth(obj, 0, &mut budget, &mut path, span)
    }

    /// Bounded-depth and bounded-node conversion so a deeply nested, aliased,
    /// or cyclic Python object cannot overflow the native stack or expand
    /// exponentially. Beyond depth the object becomes its `repr`; beyond the
    /// node budget the conversion fails with a structured diagnostic.
    fn to_value_depth(
        obj: &Bound<'_, PyAny>,
        depth: usize,
        budget: &mut usize,
        path: &mut Vec<usize>,
        span: Span,
    ) -> Result<Value> {
        if *budget == 0 {
            return Err(Diag::new(
                codes::PY_UNSUPPORTED,
                "Python object has too many nodes to cross into Aura",
                span,
            ));
        }
        *budget -= 1;
        if depth >= crate::run::value::MAX_VALUE_DEPTH {
            // Symmetric with the Aura -> Python direction, which rejects
            // over-depth conversions: a pathologically deep Python container
            // must not silently degrade into a `repr` string with a different
            // type. The documented contract is that both directions bound depth
            // with a diagnostic (`CPYTHON_COMPATIBILITY_TARGET.md`). The
            // shallow opaque-object fallback below is unaffected.
            return Err(Diag::new(
                codes::PY_UNSUPPORTED,
                "Python value is nested too deeply to cross into Aura",
                span,
            ));
        }
        if obj.is_none() {
            return Ok(Value::None);
        }
        // Identity match: `extract::<bool>()` is truthiness-based in PyO3 and
        // would coerce any object with a `__bool__`, so require a genuine
        // `bool` instance (`is_instance_of` accepts a `bool` subclass, which is
        // sound — it is still represented as a `bool`).
        if obj.is_instance_of::<PyBool>() {
            return Ok(Value::Bool(
                obj.extract::<bool>().map_err(|e| map_pyerr(e, span))?,
            ));
        }
        // Check `int` *before* `float` and reject values outside `i64` rather
        // than silently demoting them to a `float`, which would lose
        // precision. Python's `int` is arbitrary precision.
        if obj.is_instance_of::<PyInt>() {
            if let Ok(i) = obj.extract::<i64>() {
                return Ok(Value::Int(i));
            }
            let text = obj.str().map_err(|e| map_pyerr(e, span))?.to_string();
            return Err(Diag::new(
                codes::OVERFLOW,
                format!("Python integer `{text}` does not fit in Aura's 64-bit `int`"),
                span,
            ));
        }
        // Match on *type identity*, not on whether a conversion happens to
        // succeed. `extract::<f64>()` is duck-typed in PyO3: it invokes
        // `__float__` (and `extract::<i64>` invokes `__index__`). A plain user
        // type implementing those would otherwise be silently coerced to
        // `float` (with possible precision loss) instead of taking the
        // documented opaque-object fallback (its `repr`), and a dunder that
        // raises would be silently swallowed. Only genuine `float` and `str`
        // instances convert; everything else falls through to `repr`.
        if obj.is_instance_of::<PyFloat>() {
            return Ok(Value::Float(
                obj.extract::<f64>().map_err(|e| map_pyerr(e, span))?,
            ));
        }
        if obj.is_instance_of::<PyString>() {
            return Ok(Value::str(
                obj.extract::<String>().map_err(|e| map_pyerr(e, span))?,
            ));
        }
        if let Ok(list) = obj.cast::<PyList>() {
            let id = obj.as_ptr() as usize;
            if path.contains(&id) {
                return Err(Diag::new(
                    codes::PY_UNSUPPORTED,
                    "Python object contains a reference cycle, which Aura cannot represent",
                    span,
                ));
            }
            path.push(id);
            let mut out = Vec::with_capacity(list.len());
            for item in list.iter() {
                out.push(to_value_depth(&item, depth + 1, budget, path, span)?);
            }
            path.pop();
            return Ok(Value::list(out));
        }
        if let Ok(dict) = obj.cast::<PyDict>() {
            let id = obj.as_ptr() as usize;
            if path.contains(&id) {
                return Err(Diag::new(
                    codes::PY_UNSUPPORTED,
                    "Python object contains a reference cycle, which Aura cannot represent",
                    span,
                ));
            }
            path.push(id);
            let mut map = BTreeMap::new();
            for (k, v) in dict.iter() {
                // Aura map keys are the key-capable scalars `string`, `int`,
                // and `bool`. Match on *type identity*, exactly as the scalar
                // path above: `extract::<i64>()` is duck-typed (it invokes
                // `__index__`), so a user object implementing `__index__` would
                // otherwise be silently coerced into an `int` key, collapsing
                // distinct keys and reintroducing the precision/identity loss
                // the scalar path rejects (`CPYTHON_COMPATIBILITY_TARGET.md`).
                let key = if k.is_instance_of::<PyBool>() {
                    // Checked before `int`: a Python `bool` is an `int`
                    // subclass, so an integer extraction would otherwise turn
                    // `True` into `1` and lose the key's identity.
                    crate::run::value::MapKey::Bool(
                        k.extract::<bool>().map_err(|e| map_pyerr(e, span))?,
                    )
                } else if k.is_instance_of::<PyInt>() {
                    match k.extract::<i64>() {
                        Ok(i) => crate::run::value::MapKey::Int(i),
                        // An integer key outside `i64` is the same overflow the
                        // scalar path reports, never a silent demotion.
                        Err(_) => {
                            let text = k.str().map_err(|e| map_pyerr(e, span))?.to_string();
                            return Err(Diag::new(
                                codes::OVERFLOW,
                                format!(
                                    "Python integer key `{text}` does not fit in Aura's 64-bit `int`"
                                ),
                                span,
                            ));
                        }
                    }
                } else if k.is_instance_of::<PyString>() {
                    crate::run::value::MapKey::str(
                        k.extract::<String>().map_err(|e| map_pyerr(e, span))?,
                    )
                } else {
                    // Not a key-capable scalar. Use `repr`, not `str`, so the
                    // diagnostic does not invoke a user `__str__` (which could
                    // itself raise or have side effects).
                    let shown = k.repr().map_err(|e| map_pyerr(e, span))?.to_string();
                    return Err(Diag::new(
                        codes::PY_UNSUPPORTED,
                        format!(
                            "Python dict key `{shown}` cannot be an Aura map key; map keys must be `string`, `int`, or `bool`"
                        ),
                        span,
                    ));
                };
                map.insert(key, to_value_depth(&v, depth + 1, budget, path, span)?);
            }
            path.pop();
            return Ok(Value::Map(Rc::new(RefCell::new(map))));
        }
        // Fallback: render as a string, so opaque objects remain printable.
        let repr = obj.repr().map_err(|e| map_pyerr(e, span))?.to_string();
        Ok(Value::str(repr))
    }

    /// Aura value -> Python object. `span` is the Aura call site, attached to
    /// every diagnostic so a conversion failure points at the crossing.
    fn to_py<'py>(py: Python<'py>, v: &Value, span: Span) -> Result<Bound<'py, PyAny>> {
        let mut budget = MAX_PY_NODES;
        // Addresses of the Aura containers on the current descent path, so a
        // cycle is detected by identity and rejected rather than re-expanded
        // forever. Sharing (the same container reachable by two paths) is not
        // a cycle and is converted again, bounded by the node budget.
        let mut path: HashSet<usize> = HashSet::new();
        to_py_depth(py, v, 0, &mut budget, &mut path, span)
    }

    /// Bounded-depth and bounded-node conversion so a deeply nested, aliased,
    /// or cyclic Aura value cannot overflow the native stack or expand
    /// exponentially when crossing into Python. Beyond either bound the value
    /// is rejected rather than risk a host overflow (AUDIT-5).
    fn to_py_depth<'py>(
        py: Python<'py>,
        v: &Value,
        depth: usize,
        budget: &mut usize,
        path: &mut HashSet<usize>,
        span: Span,
    ) -> Result<Bound<'py, PyAny>> {
        if *budget == 0 {
            return Err(Diag::new(
                codes::PY_UNSUPPORTED,
                "value has too many nodes to cross into Python",
                span,
            ));
        }
        *budget -= 1;
        if depth >= crate::run::value::MAX_VALUE_DEPTH {
            return Err(Diag::new(
                codes::PY_UNSUPPORTED,
                "value is nested too deeply to cross into Python",
                span,
            ));
        }
        Ok(match v {
            Value::None => py.None().into_bound(py),
            Value::Bool(b) => b
                .into_pyobject(py)
                .map_err(|e| map_conversion(e, span))?
                .to_owned()
                .into_any(),
            Value::Int(i) => i
                .into_pyobject(py)
                .map_err(|e| map_conversion(e, span))?
                .into_any(),
            Value::Float(f) => f
                .into_pyobject(py)
                .map_err(|e| map_conversion(e, span))?
                .into_any(),
            Value::Str(s) => s
                .to_string()
                .into_pyobject(py)
                .map_err(|e| map_conversion(e, span))?
                .into_any(),
            Value::List(l) => {
                let id = Rc::as_ptr(l) as usize;
                if !path.insert(id) {
                    return Err(Diag::new(
                        codes::PY_UNSUPPORTED,
                        "value contains a reference cycle, which cannot cross into Python",
                        span,
                    ));
                }
                let list = PyList::empty(py);
                for item in l.borrow().iter() {
                    list.append(to_py_depth(py, item, depth + 1, budget, path, span)?)
                        .map_err(|e| map_pyerr(e, span))?;
                }
                path.remove(&id);
                list.into_any()
            }
            Value::Map(m) => {
                let id = Rc::as_ptr(m) as usize;
                if !path.insert(id) {
                    return Err(Diag::new(
                        codes::PY_UNSUPPORTED,
                        "value contains a reference cycle, which cannot cross into Python",
                        span,
                    ));
                }
                let dict = PyDict::new(py);
                for (k, val) in m.borrow().iter() {
                    let key = match k {
                        crate::run::value::MapKey::Str(s) => s
                            .to_string()
                            .into_pyobject(py)
                            .map_err(|e| map_conversion(e, span))?
                            .into_any(),
                        crate::run::value::MapKey::Int(i) => i
                            .into_pyobject(py)
                            .map_err(|e| map_conversion(e, span))?
                            .into_any(),
                        crate::run::value::MapKey::Bool(b) => {
                            PyBool::new(py, *b).to_owned().into_any()
                        }
                    };
                    dict.set_item(key, to_py_depth(py, val, depth + 1, budget, path, span)?)
                        .map_err(|e| map_pyerr(e, span))?;
                }
                path.remove(&id);
                dict.into_any()
            }
            other => {
                return Err(Diag::new(
                    codes::PY_UNSUPPORTED,
                    format!(
                        "value of type {} cannot cross into Python",
                        other.type_name()
                    ),
                    span,
                ))
            }
        })
    }

    fn map_pyerr(e: PyErr, span: Span) -> Diag {
        Diag::new(codes::PY_ERROR, format!("python: {e}"), span)
    }

    /// Convert any `into_pyobject` error (which implements `Into<PyErr>`)
    /// into an Aura diagnostic at the given call site.
    fn map_conversion<E: Into<PyErr>>(e: E, span: Span) -> Diag {
        map_pyerr(e.into(), span)
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
                let result = py
                    .eval(c.as_c_str(), None, None)
                    .map_err(|e| map_pyerr(e, span))?;
                to_value(&result, span)
            })
        });
        it.native("py_import", |_it, args, span| {
            let Some(Value::Str(name)) = args.first() else {
                return Err(err("py_import expects a module name string", span));
            };
            ensure_init();
            Python::attach(|py| {
                let module = PyModule::import(py, name.as_ref()).map_err(|e| map_pyerr(e, span))?;
                // Return a dict of the module's public attributes as strings.
                let mut map = BTreeMap::new();
                for (k, _) in module.dict().iter() {
                    let key = k.str().map_err(|e| map_pyerr(e, span))?.to_string();
                    if !key.starts_with('_') {
                        map.insert(crate::run::value::MapKey::str(key), Value::str("<python>"));
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
                let module =
                    PyModule::import(py, module.as_str()).map_err(|e| map_pyerr(e, span))?;
                let func = module
                    .getattr(attr.as_str())
                    .map_err(|e| map_pyerr(e, span))?;
                let owned: Vec<Bound<'_, PyAny>> = call_args
                    .iter()
                    .map(|v| to_py(py, v, span))
                    .collect::<Result<Vec<_>>>()?;
                let tuple = PyTuple::new(py, owned).map_err(|e| map_pyerr(e, span))?;
                let result = func.call1(&tuple).map_err(|e| map_pyerr(e, span))?;
                to_value(&result, span)
            })
        });
        it.native("py_version", |_it, _args, _span| {
            ensure_init();
            ensure_init();
            Python::attach(|_py| Ok(Value::str(::pyo3::Python::version_str())))
        });
    }
}
