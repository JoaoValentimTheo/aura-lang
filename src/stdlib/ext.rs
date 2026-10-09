//! Feature-gated standard library modules: `json`, `regex`, `time`.
//!
//! Each is compiled only when its Cargo feature is on. None of them require
//! Python.

#[cfg(feature = "json")]
pub mod json {
    //! JSON encode/decode.

    use crate::error::{codes, Diag, Span};
    /// The compiler result type for the typed-decode path (the module's bare
    /// `Result` is the two-parameter `std::result::Result`).
    type AuraResult<T> = crate::error::Result<T>;
    use crate::run::value::Value;
    use crate::run::Interp;
    use std::cell::RefCell;
    use std::collections::BTreeMap;
    use std::rc::Rc;

    fn err(msg: impl Into<String>, span: Span) -> Diag {
        Diag::new(codes::TYPE_MISMATCH, msg, span)
    }

    fn to_json(v: &Value) -> Result<serde_json::Value, String> {
        let mut budget = crate::run::value::RenderBudget::new();
        to_json_depth(v, 0, &mut budget)
    }

    /// Convert an Aura value to JSON, bounded by [`Value::MAX_VALUE_DEPTH`]-style
    /// depth *and* a total-node budget so a cyclic or pathologically deep value
    /// cannot overflow the native stack or expand exponentially. A value nested
    /// beyond either bound serializes as `null` (`LANGUAGE_SPEC.md` §31.6).
    ///
    /// A JSON object's keys are strings by the JSON standard. A string-keyed
    /// Aura map encodes as a JSON object unchanged. A map with a non-string key
    /// type (or any non-string key in an otherwise untyped map) is a
    /// deterministic conversion error: stringifying the key would collapse
    /// distinct keys (`1` and `"1"`) into one, which the language refuses to do
    /// silently.
    ///
    /// The same anti-collapse rule governs the value itself: a value whose
    /// *kind* has no exact JSON representation — an enum variant, a `range`, a
    /// function, or a non-finite float (`nan`/`inf`) — is a deterministic
    /// conversion error, never a silent approximation. `null` is the encoding
    /// of `none` (and of the documented over-depth truncation), so
    /// approximating an unrepresentable value as `null` would make
    /// `json_decode(json_encode(x))` collapse distinct kinds (`0..3`, a
    /// closure, `nan`, and `none`) into `none`. Flattening a variant instead
    /// collapses distinct variants of one enum (`A(1)` and `B(1)`) into the
    /// same JSON value, exactly the identity loss the map-key rule refuses.
    /// The kinds JSON *can* represent exactly — scalars, `none`, lists,
    /// string-keyed maps, and structs — encode unchanged.
    fn to_json_depth(
        v: &Value,
        depth: usize,
        budget: &mut crate::run::value::RenderBudget,
    ) -> Result<serde_json::Value, String> {
        use crate::run::value::MapKey;
        if depth >= crate::run::value::MAX_VALUE_DEPTH || !budget.take() {
            return Ok(serde_json::Value::Null);
        }
        Ok(match v {
            Value::None => serde_json::Value::Null,
            Value::Bool(b) => serde_json::Value::Bool(*b),
            Value::Int(i) => serde_json::Value::Number((*i).into()),
            Value::Float(f) => {
                let Some(n) = serde_json::Number::from_f64(*f) else {
                    return Err(format!(
                        "json_encode cannot represent the non-finite float `{f}`; JSON numbers are finite, and `null` is the encoding of `none`"
                    ));
                };
                serde_json::Value::Number(n)
            }
            Value::Str(s) => serde_json::Value::String(s.to_string()),
            Value::List(l) => {
                let mut out = Vec::with_capacity(l.borrow().len());
                for x in l.borrow().iter() {
                    out.push(to_json_depth(x, depth + 1, budget)?);
                }
                serde_json::Value::Array(out)
            }
            // Array, Tuple, and Set encode to a JSON array (a serialization,
            // not an identity-preserving round-trip: dynamic decode yields a
            // List). Set order is deterministic (ascending member order).
            Value::Array(l) => {
                let mut out = Vec::with_capacity(l.borrow().len());
                for x in l.borrow().iter() {
                    out.push(to_json_depth(x, depth + 1, budget)?);
                }
                serde_json::Value::Array(out)
            }
            Value::Tuple(t) => {
                let mut out = Vec::with_capacity(t.len());
                for x in t.iter() {
                    out.push(to_json_depth(x, depth + 1, budget)?);
                }
                serde_json::Value::Array(out)
            }
            Value::Set(s) => {
                let mut out = Vec::with_capacity(s.borrow().len());
                for k in s.borrow().iter() {
                    out.push(to_json_depth(&k.to_value(), depth + 1, budget)?);
                }
                serde_json::Value::Array(out)
            }
            Value::Map(m) => {
                let mut obj = serde_json::Map::new();
                for (k, v) in m.borrow().iter() {
                    let MapKey::Str(s) = k else {
                        return Err(format!(
                            "json_encode cannot represent a map with a `{}` key; JSON object keys are strings",
                            k.type_name()
                        ));
                    };
                    obj.insert(s.to_string(), to_json_depth(v, depth + 1, budget)?);
                }
                serde_json::Value::Object(obj)
            }
            Value::Instance(i) => {
                let mut obj = serde_json::Map::new();
                for (k, v) in i.fields.borrow().iter() {
                    obj.insert(k.clone(), to_json_depth(v, depth + 1, budget)?);
                }
                serde_json::Value::Object(obj)
            }
            Value::Variant(v) => {
                return Err(format!(
                    "json_encode cannot represent the `{}` variant `{}{}`; JSON has no enum kind, and flattening it would collapse distinct variants",
                    v.ty,
                    v.tag,
                    if v.payload.is_empty() { "()" } else { "(…)" }
                ));
            }
            Value::Range(r) => {
                return Err(format!(
                    "json_encode cannot represent the `range` `{}..{}`; JSON has no range kind",
                    r.start, r.end
                ));
            }
            Value::Closure(_) | Value::Native(_) => {
                return Err(format!(
                    "json_encode cannot represent a `{}`; JSON has no function kind",
                    v.type_name()
                ));
            }
        })
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
                    m.insert(
                        crate::run::value::MapKey::str(k),
                        from_json_depth(v, depth + 1),
                    );
                }
                Value::Map(Rc::new(RefCell::new(m)))
            }
        }
    }

    /// Validate and convert a JSON document against a declared Aura
    /// [`TypeExpr`], producing a *typed* value (Keystone §22).
    ///
    /// This is the semantic core of `json_decode_as`: unlike `json_decode`
    /// (which is permissive and returns a dynamic value), this checks the
    /// document against the declared shape and reports a precise `E4031` for a
    /// mismatch. The domains stay distinguishable: a malformed document, a
    /// missing required field, a wrong field type, an optional `T | none`
    /// field, a collection element mismatch, and a nested mismatch each carry
    /// a specific message.
    ///
    /// Bounds: recursion follows the declared type (finite, from the program)
    /// and the document (bounded by serde_json's nesting limit), so a hostile
    /// document cannot cause unbounded recursion beyond the JSON depth limit.
    fn decode_as(
        it: &Interp,
        j: &serde_json::Value,
        ty: &crate::ast::TypeExpr,
        path: &str,
        span: Span,
    ) -> AuraResult<Value> {
        use crate::ast::TypeExpr;
        match ty {
            TypeExpr::Union(members) => {
                // A union accepts the value if any member accepts it. `T | none`
                // additionally accepts JSON null (and only then narrows to none
                // for that member), which is the optional-field contract.
                if members.iter().any(|m| matches!(m, TypeExpr::None)) {
                    if matches!(j, serde_json::Value::Null) {
                        return Ok(Value::None);
                    }
                }
                for m in members {
                    if matches!(m, TypeExpr::None) {
                        continue;
                    }
                    if decode_as(it, j, m, path, span).is_ok() {
                        return decode_as(it, j, m, path, span);
                    }
                }
                Err(decode_err(
                    format!("`{path}` does not match `{}`", ty.name()),
                    span,
                ))
            }
            TypeExpr::Int => match j {
                serde_json::Value::Number(n) => n
                    .as_i64()
                    .map(Value::Int)
                    .ok_or_else(|| decode_err(format!("`{path}` is not an int"), span)),
                _ => Err(decode_err(format!("`{path}` is not an int"), span)),
            },
            TypeExpr::Float => match j {
                serde_json::Value::Number(n) => n
                    .as_f64()
                    .map(Value::Float)
                    .ok_or_else(|| decode_err(format!("`{path}` is not a float"), span)),
                _ => Err(decode_err(format!("`{path}` is not a float"), span)),
            },
            TypeExpr::String => match j {
                serde_json::Value::String(s) => Ok(Value::str(s.clone())),
                _ => Err(decode_err(format!("`{path}` is not a string"), span)),
            },
            TypeExpr::Bool => match j {
                serde_json::Value::Bool(b) => Ok(Value::Bool(*b)),
                _ => Err(decode_err(format!("`{path}` is not a bool"), span)),
            },
            TypeExpr::None => {
                if matches!(j, serde_json::Value::Null) {
                    Ok(Value::None)
                } else {
                    Err(decode_err(format!("`{path}` is not none"), span))
                }
            }
            TypeExpr::List(inner) => match j {
                serde_json::Value::Array(items) => {
                    let mut out = Vec::with_capacity(items.len());
                    for (i, item) in items.iter().enumerate() {
                        out.push(decode_as(it, item, inner, &format!("{path}[{i}]"), span)?);
                    }
                    Ok(Value::list(out))
                }
                _ => Err(decode_err(
                    format!("`{path}` is not a list of `{}`", inner.name()),
                    span,
                )),
            },
            TypeExpr::Array(elem, n) => match j {
                serde_json::Value::Array(items) => {
                    if items.len() as u64 != *n {
                        return Err(decode_err(
                            format!(
                                "`{path}` has {} element(s) but `{}` requires exactly {n}",
                                items.len(),
                                ty.name()
                            ),
                            span,
                        ));
                    }
                    let mut out = Vec::with_capacity(items.len());
                    for (i, item) in items.iter().enumerate() {
                        out.push(decode_as(it, item, elem, &format!("{path}[{i}]"), span)?);
                    }
                    Ok(Value::array(out))
                }
                _ => Err(decode_err(
                    format!("`{path}` is not an array of `{}`", elem.name()),
                    span,
                )),
            },
            TypeExpr::Tuple(members) => match j {
                serde_json::Value::Array(items) => {
                    if items.len() != members.len() {
                        return Err(decode_err(
                            format!(
                                "`{path}` has {} element(s) but `{}` requires exactly {}",
                                items.len(),
                                ty.name(),
                                members.len()
                            ),
                            span,
                        ));
                    }
                    let mut out = Vec::with_capacity(items.len());
                    for (i, (item, m)) in items.iter().zip(members).enumerate() {
                        out.push(decode_as(it, item, m, &format!("{path}[{i}]"), span)?);
                    }
                    Ok(Value::tuple(out))
                }
                _ => Err(decode_err(
                    format!("`{path}` is not a tuple for `{}`", ty.name()),
                    span,
                )),
            },
            TypeExpr::Set(inner) => match j {
                serde_json::Value::Array(items) => {
                    let mut members = std::collections::BTreeSet::new();
                    for (i, item) in items.iter().enumerate() {
                        let v = decode_as(it, item, inner, &format!("{path}[{i}]"), span)?;
                        let Some(k) = crate::run::value::MapKey::from_value(&v) else {
                            return Err(decode_err(
                                format!(
                                    "`{path}[{i}]` is not a valid set element for `{}`",
                                    ty.name()
                                ),
                                span,
                            ));
                        };
                        // A duplicate JSON element is a shape/data mismatch for
                        // strict external validation (§34): the source carried
                        // two pieces of data, and silently collapsing them would
                        // discard information.
                        if !members.insert(k) {
                            return Err(decode_err(
                                format!(
                                    "`{path}[{i}]` is a duplicate element; `{}` may not receive duplicate JSON members",
                                    ty.name()
                                ),
                                span,
                            ));
                        }
                    }
                    Ok(Value::set(members))
                }
                _ => Err(decode_err(
                    format!("`{path}` is not a set of `{}`", inner.name()),
                    span,
                )),
            },
            TypeExpr::Map(k, v) => match j {
                serde_json::Value::Object(o) => {
                    let mut m = std::collections::BTreeMap::new();
                    for (key, item) in o {
                        // JSON object keys are strings; a non-string key type
                        // is a mismatch rather than a silent coercion.
                        if !matches!(k.as_ref(), TypeExpr::String) {
                            return Err(decode_err(
                                format!(
                                    "`{path}` has non-string keys but `{}` requires `{}`",
                                    ty.name(),
                                    k.name()
                                ),
                                span,
                            ));
                        }
                        let value = decode_as(it, item, v, &format!("{path}.{key}"), span)?;
                        m.insert(crate::run::value::MapKey::str(key), value);
                    }
                    Ok(Value::Map(std::rc::Rc::new(std::cell::RefCell::new(m))))
                }
                _ => Err(decode_err(format!("`{path}` is not a map"), span)),
            },
            TypeExpr::Named(name) => {
                // A nominal struct: require an object, every declared field,
                // and no field outside the declaration.
                if let Some(fields) = it.struct_fields(name) {
                    let serde_json::Value::Object(o) = j else {
                        return Err(decode_err(
                            format!("`{path}` is not an object for struct `{name}`"),
                            span,
                        ));
                    };
                    let declared: Vec<&String> = fields.keys().collect();
                    for key in o.keys() {
                        if !fields.contains_key(key.as_str()) {
                            return Err(decode_err(
                                format!(
                                    "`{path}.{key}` is not a field of `{name}`; declared fields are {}",
                                    declared
                                        .iter()
                                        .map(|f| format!("`{f}`"))
                                        .collect::<Vec<_>>()
                                        .join(", ")
                                ),
                                span,
                            ));
                        }
                    }
                    let field_order: Vec<String> =
                        it.struct_field_names(name).cloned().unwrap_or_default();
                    let mut values = Vec::with_capacity(field_order.len());
                    for fname in &field_order {
                        // The runtime registry and the declared-field map are
                        // built from the same struct declaration, but a name
                        // would rather be treated as a decode mismatch than
                        // panic if they ever disagreed.
                        let Some(fty) = fields.get(fname) else {
                            return Err(decode_err(
                                format!("`{path}.{fname}` has no declared type in `{name}`"),
                                span,
                            ));
                        };
                        match o.get(fname) {
                            Some(raw) => {
                                let v = decode_as(it, raw, fty, &format!("{path}.{fname}"), span)?;
                                values.push((fname.clone(), v));
                            }
                            None => {
                                // A missing field is allowed only when the
                                // declared type accepts `none` (an optional
                                // `T | none` field), which decodes to `none`.
                                if accepts_none(fty) {
                                    values.push((fname.clone(), Value::None));
                                } else {
                                    return Err(decode_err(
                                        format!(
                                            "`{path}.{fname}` is required by `{name}` but missing"
                                        ),
                                        span,
                                    ));
                                }
                            }
                        }
                    }
                    Ok(Value::Instance(std::rc::Rc::new(
                        crate::run::value::Instance {
                            ty: name.clone(),
                            fields: std::cell::RefCell::new(values),
                        },
                    )))
                } else {
                    Err(decode_err(
                        format!("unknown type `{name}` for typed decode"),
                        span,
                    ))
                }
            }
            other => Err(decode_err(
                format!("`{}` is not a decodable type", other.name()),
                span,
            )),
        }
    }

    /// Whether a declared type admits `none` (a union containing it).
    fn accepts_none(ty: &crate::ast::TypeExpr) -> bool {
        use crate::ast::TypeExpr;
        match ty {
            TypeExpr::None => true,
            TypeExpr::Union(ms) => ms.iter().any(accepts_none),
            _ => false,
        }
    }

    /// Decode JSON text against an already-parsed [`TypeExpr`], producing a
    /// typed value. Shared by `json_decode_as`'s runtime path: the canonical
    /// parser resolves `json_decode_as(text, Pokemon)` to a [`TypeExpr`] at
    /// parse time, so the runtime never re-parses a type spelling
    /// (`LANGUAGE_SPEC.md` §22).
    pub fn decode_typed(
        it: &Interp,
        text: &str,
        ty: &crate::ast::TypeExpr,
        span: Span,
    ) -> AuraResult<Value> {
        let j: serde_json::Value = serde_json::from_str(text)
            .map_err(|e| Diag::new(codes::DECODE_MISMATCH, format!("malformed JSON: {e}"), span))?;
        decode_as(it, &j, ty, &ty.name(), span)
    }

    fn decode_err(msg: String, span: Span) -> Diag {
        Diag::new(codes::DECODE_MISMATCH, msg, span)
    }

    /// Install `json_encode`, `json_decode`, and `json_decode_as`.
    pub fn install(it: &mut Interp) {
        it.native("json_encode", |_it, args, span| {
            let v = args
                .first()
                .ok_or_else(|| err("json_encode expects a value", span))?;
            let json = to_json(v).map_err(|e| err(e, span))?;
            Ok(Value::str(
                serde_json::to_string(&json).map_err(|e| err(e.to_string(), span))?,
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
        // Typed decode: `json_decode_as(text, "TypeName")`. The type is named
        // by its declared nominal spelling; the runtime registry resolves it to
        // the declaration's fields and types, and the document is validated
        // against them (`E4031` on any mismatch).
        it.native("json_decode_as", |it, args, span| {
            let (Some(Value::Str(s)), Some(Value::Str(type_name))) = (args.first(), args.get(1))
            else {
                return Err(err("json_decode_as expects (text, type_name)", span));
            };
            let j: serde_json::Value = serde_json::from_str(s).map_err(|e| {
                Diag::new(codes::DECODE_MISMATCH, format!("malformed JSON: {e}"), span)
            })?;
            // The type is supplied as its source spelling. It is parsed with
            // the compiler's own type grammar (`[User]`, `{string: int}`,
            // `User | none`), so this path grows no second type language.
            let ty = crate::parse::parse_type(type_name).map_err(|e| {
                decode_err(
                    format!("`{type_name}` is not a valid type spelling: {}", e.message),
                    span,
                )
            })?;
            decode_as(it, &j, &ty, type_name, span)
        });
    }
}

#[cfg(feature = "http-api")]
pub mod http {
    //! HTTP capability surface (Keystone §21).
    //!
    //! The evaluator builds a typed [`crate::host::HttpRequest`] and asks the
    //! installed host for the capability. A host without network authority
    //! returns `E5002`; a transport failure is `E4020`; a response body over
    //! the limit is `E4020`. The request/response are ordinary Aura values: a
    //! response is a map with `status`, `headers`, and `body`, so
    //! `response["status"]` and typed JSON decoding compose without special
    //! syntax.

    use crate::error::{codes, Diag, Span};
    use crate::host::{HttpRequest, HTTP_METHODS, MAX_HTTP_TIMEOUT_MS};
    use crate::run::value::{MapKey, Value};
    use crate::run::Interp;
    use std::cell::RefCell;
    use std::collections::BTreeMap;
    use std::rc::Rc;

    fn err(code: u16, msg: impl Into<String>, span: Span) -> Diag {
        Diag::new(code, msg, span)
    }

    fn string_arg(args: &[Value], i: usize, name: &str, span: Span) -> Result<String, Diag> {
        match args.get(i) {
            Some(Value::Str(s)) => Ok(s.to_string()),
            _ => Err(err(
                codes::TYPE_MISMATCH,
                format!("`{name}` expects a string argument {}", i + 1),
                span,
            )),
        }
    }

    /// Build a response map: `{status, headers, body}`.
    fn response_value(response: &crate::host::HttpResponse) -> Value {
        let mut m: BTreeMap<MapKey, Value> = BTreeMap::new();
        m.insert(
            MapKey::str("status"),
            Value::Int(i64::from(response.status)),
        );
        m.insert(MapKey::str("body"), Value::str(response.body.clone()));
        // The raw body bytes as a list of ints (0..=255). This is the
        // binary-safe view: `body` is a text convenience and may be a lossy
        // rendering for non-UTF-8 payloads.
        m.insert(
            MapKey::str("body_bytes"),
            Value::List(Rc::new(RefCell::new(
                response
                    .body_bytes
                    .iter()
                    .map(|b| Value::Int(i64::from(*b)))
                    .collect(),
            ))),
        );
        // Header materialization (`HTTP_ARCHITECTURE.md`):
        //
        //   * `headers` is a map keyed by the lowercased name, so a lookup is
        //     case-insensitive. A name that occurred once maps to its value; a
        //     name that occurred more than once maps to a *list* of every
        //     occurrence in order, so a repeated header (notably
        //     `Set-Cookie`) is never destroyed by comma-joining — commas are
        //     only valid for a subset of headers and never for `Set-Cookie`.
        //   * `header_lines` is the flat, ordered `[[name, value], …]` list,
        //     preserving duplicates and original casing for a consumer that
        //     needs the exact wire shape.
        let mut headers: BTreeMap<MapKey, Value> = BTreeMap::new();
        for (k, v) in &response.headers {
            let key = MapKey::str(k.to_ascii_lowercase());
            match headers.get(&key) {
                None => {
                    headers.insert(key, Value::str(v.clone()));
                }
                Some(Value::Str(prev)) => {
                    let list = vec![Value::str(prev.to_string()), Value::str(v.clone())];
                    headers.insert(key, Value::List(Rc::new(RefCell::new(list))));
                }
                Some(Value::List(items)) => {
                    items.borrow_mut().push(Value::str(v.clone()));
                }
                Some(_) => {}
            }
        }
        m.insert(
            MapKey::str("headers"),
            Value::Map(Rc::new(RefCell::new(headers))),
        );
        m.insert(
            MapKey::str("header_lines"),
            Value::List(Rc::new(RefCell::new(
                response
                    .headers
                    .iter()
                    .map(|(k, v)| {
                        Value::List(Rc::new(RefCell::new(vec![
                            Value::str(k.clone()),
                            Value::str(v.clone()),
                        ])))
                    })
                    .collect(),
            ))),
        );
        Value::Map(Rc::new(RefCell::new(m)))
    }

    /// Validate the method/URL and build the typed request. Shared by the
    /// synchronous and resumable paths so validation never diverges.
    fn build_request(
        method: &str,
        url: &str,
        headers: &[(String, String)],
        body: Option<String>,
        timeout_ms: u64,
        span: Span,
    ) -> Result<HttpRequest, Diag> {
        let method = method.to_ascii_uppercase();
        if !HTTP_METHODS.contains(&method.as_str()) {
            return Err(err(
                codes::TYPE_MISMATCH,
                format!(
                    "`{method}` is not a supported HTTP method; expected one of {}",
                    HTTP_METHODS.join(", ")
                ),
                span,
            ));
        }
        Ok(HttpRequest {
            method,
            url: url.to_string(),
            headers: headers.to_vec(),
            body,
            timeout_ms: timeout_ms.min(MAX_HTTP_TIMEOUT_MS),
        })
    }

    /// Perform a request through the host capability (synchronous path).
    ///
    /// A host that reports [`crate::host::HostError::Pending`] (a resumable
    /// substrate) is not reachable here: the resumable native intercepts that
    /// before calling this. This remains for the native provider and for a
    /// direct host that answers synchronously.
    #[allow(dead_code)]
    fn request(
        it: &Interp,
        method: &str,
        url: &str,
        headers: &[(String, String)],
        body: Option<String>,
        timeout_ms: u64,
        span: Span,
    ) -> Result<Value, Diag> {
        let request = build_request(method, url, headers, body, timeout_ms, span)?;
        let response = it
            .host()
            .http_request(&request)
            .map_err(|e| e.into_diag(span))?;
        Ok(response_value(&response))
    }

    /// The resumable request step (D1): validate, then either take the host's
    /// synchronous response (native provider) or suspend with the typed
    /// request as a [`PendingEffect::Http`]. On resume, build the Aura value
    /// from the returned parts exactly as the synchronous path does, so a
    /// browser response and a native response are indistinguishable to the
    /// language.
    fn request_step(
        it: &mut Interp,
        method: &str,
        url: &str,
        headers: &[(String, String)],
        body: Option<String>,
        timeout_ms: u64,
        span: Span,
    ) -> Result<crate::run::NativeOutcome, Diag> {
        let request = build_request(method, url, headers, body, timeout_ms, span)?;
        match it.host().http_request(&request) {
            Ok(response) => Ok(crate::run::NativeOutcome::Done(response_value(&response))),
            Err(crate::host::HostError::Pending(effect)) => {
                Ok(crate::run::NativeOutcome::Suspend {
                    effect,
                    wait: Box::new(HttpWait { span }),
                })
            }
            Err(e) => Err(e.into_diag(span)),
        }
    }

    /// The parked continuation of an HTTP request: on resume it builds the
    /// response value from the effect result.
    struct HttpWait {
        span: Span,
    }

    impl crate::run::NativeResume for HttpWait {
        fn resume(
            self: Box<Self>,
            _it: &mut Interp,
            _result: Result<Value, Diag>,
        ) -> Result<crate::run::NativeOutcome, Diag> {
            // A callback result is never expected on an HTTP token.
            Err(err(
                codes::INTERNAL,
                "an HTTP effect token received a callback result",
                self.span,
            ))
        }

        fn resume_effect(
            self: Box<Self>,
            _it: &mut Interp,
            result: Result<Value, Diag>,
        ) -> Result<crate::run::NativeOutcome, Diag> {
            // The effect value is already the Aura response map (built by the
            // embedder from the transport response via the shared encoder, or
            // by a native host); a failure propagates as a diagnostic
            // attributed to the *call site*, exactly like a synchronous
            // `Host::http_request` failure, so `try`/`catch`/`finally` and the
            // source span match the native path.
            result
                .map(crate::run::NativeOutcome::Done)
                .map_err(|mut d| {
                    if d.span == Span::default() {
                        d.span = self.span;
                    }
                    d
                })
        }
    }

    /// Parse the optional headers argument: `[[ "k", "v" ], …]`.
    fn parse_headers(
        args: &[Value],
        i: usize,
        name: &str,
        span: Span,
    ) -> Result<Vec<(String, String)>, Diag> {
        let Some(Value::List(items)) = args.get(i) else {
            return Err(err(
                codes::TYPE_MISMATCH,
                format!("`{name}` expects a list of `[key, value]` header pairs"),
                span,
            ));
        };
        let items = items.borrow();
        let mut out = Vec::with_capacity(items.len());
        for (idx, item) in items.iter().enumerate() {
            let Value::List(pair) = item else {
                return Err(err(
                    codes::TYPE_MISMATCH,
                    format!("`{name}` header {idx} is not a `[key, value]` pair"),
                    span,
                ));
            };
            let pair = pair.borrow();
            if pair.len() != 2 {
                return Err(err(
                    codes::TYPE_MISMATCH,
                    format!("`{name}` header {idx} must have exactly two elements"),
                    span,
                ));
            }
            let (Value::Str(k), Value::Str(v)) = (&pair[0], &pair[1]) else {
                return Err(err(
                    codes::TYPE_MISMATCH,
                    format!("`{name}` header {idx} must be `[string, string]`"),
                    span,
                ));
            };
            out.push((k.to_string(), v.to_string()));
        }
        Ok(out)
    }

    /// Install the HTTP functions.
    ///
    /// Both are **resumable natives** (D1): a host that answers synchronously
    /// (the native `ureq` provider) completes in one step, and a host that
    /// suspends (the browser Worker) parks the machine on
    /// [`PendingEffect::Http`](crate::host::PendingEffect::Http) with the typed
    /// request. Validation happens exactly once, before either path.
    pub fn install(it: &mut Interp) {
        // `http_request(method, url)` / `http_request(method, url, options)`
        // where options is a map with optional `headers`, `body`, `timeout_ms`.
        it.native_resumable("http_request", |it, args, span| {
            let method = string_arg(&args, 0, "http_request", span)?;
            let url = string_arg(&args, 1, "http_request", span)?;
            let (headers, body, timeout_ms) = match args.get(2) {
                None => (Vec::new(), None, MAX_HTTP_TIMEOUT_MS),
                Some(Value::Map(opts)) => {
                    let opts = opts.borrow();
                    let headers = match opts.get(&MapKey::str("headers")) {
                        None => Vec::new(),
                        Some(_) => {
                            // A map value is not a positional list argument, so
                            // rebuild the list path from a synthetic vector.
                            let synthetic = vec![opts
                                .get(&MapKey::str("headers"))
                                .cloned()
                                .unwrap_or(Value::None)];
                            parse_headers(&synthetic, 0, "http_request", span)?
                        }
                    };
                    let body = match opts.get(&MapKey::str("body")) {
                        Some(Value::Str(b)) => Some(b.to_string()),
                        Some(Value::None) | None => None,
                        Some(_) => {
                            return Err(err(
                                codes::TYPE_MISMATCH,
                                "`http_request` option `body` must be a string",
                                span,
                            ));
                        }
                    };
                    let timeout_ms = match opts.get(&MapKey::str("timeout_ms")) {
                        Some(Value::Int(t)) if *t > 0 => (*t).unsigned_abs(),
                        Some(Value::Int(_)) => {
                            return Err(err(
                                codes::TYPE_MISMATCH,
                                "`http_request` option `timeout_ms` must be positive",
                                span,
                            ));
                        }
                        Some(_) => {
                            return Err(err(
                                codes::TYPE_MISMATCH,
                                "`http_request` option `timeout_ms` must be an int",
                                span,
                            ));
                        }
                        None => MAX_HTTP_TIMEOUT_MS,
                    };
                    (headers, body, timeout_ms)
                }
                Some(_) => {
                    return Err(err(
                        codes::TYPE_MISMATCH,
                        "`http_request` third argument must be an options map",
                        span,
                    ));
                }
            };
            request_step(it, &method, &url, &headers, body, timeout_ms, span)
        });
        // Convenience GET: `http_get(url)`.
        it.native_resumable("http_get", |it, args, span| {
            let url = string_arg(&args, 0, "http_get", span)?;
            request_step(it, "GET", &url, &[], None, MAX_HTTP_TIMEOUT_MS, span)
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
            let k = crate::run::value::MapKey::str;
            m.insert(k("year"), Value::Int(i64::from(now.year)));
            m.insert(k("month"), Value::Int(i64::from(now.month)));
            m.insert(k("day"), Value::Int(i64::from(now.day)));
            m.insert(k("hour"), Value::Int(i64::from(now.hour)));
            m.insert(k("minute"), Value::Int(i64::from(now.minute)));
            m.insert(k("second"), Value::Int(i64::from(now.second)));
            m.insert(k("unix"), Value::Int(now.unix));
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
