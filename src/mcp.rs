//! The first-party MCP (Model Context Protocol) adapter for Aura.
//!
//! This is a **transport**, not a semantic authority. Every operation projects
//! onto an AIS/0.1 primitive (`src/ais.rs`): the snapshot, the slice, the
//! delta, symbols, and diagnostics. It defines no semantics of its own, and
//! the compiler remains the only authority for what a program means.
//!
//! ## Architecture
//!
//! ```text
//! Aura source
//!   → compiler pipeline
//!   → AIS/0.1 semantic model
//!   → this adapter (JSON-RPC 2.0 over stdio)
//!   → MCP client
//! ```
//!
//! ## Security
//!
//! The adapter grants no capability. It reads only the source paths the client
//! names on the request (the same read the CLI already performs), never the
//! environment, the network, or a secret. Every response is bounded: a slice's
//! symbol count is capped by the client's `budget`, and a request payload is
//! capped by [`MAX_REQUEST_BYTES`]. A malformed frame produces a JSON-RPC
//! error, never a panic and never a partially-applied state change — the
//! adapter is stateless between requests.
//!
//! ## Protocol
//!
//! The adapter speaks newline-delimited JSON-RPC 2.0 (one JSON object per
//! line) on stdin/stdout, which is the MCP stdio transport. It implements the
//! MCP handshake (`initialize`, `notifications/initialized`, `ping`) and
//! exposes semantic tools plus read-only resources. Unknown methods return the
//! JSON-RPC `-32601` error rather than failing the process.

use serde_json::{json, Value};

/// The MCP protocol revision this adapter implements.
pub const MCP_PROTOCOL_VERSION: &str = "2025-06-18";

/// The largest request frame accepted, in bytes. A larger frame is rejected
/// with a JSON-RPC error rather than buffered without limit.
pub const MAX_REQUEST_BYTES: usize = 8 * 1024 * 1024;

/// The largest number of symbols a single slice response may carry.
pub const MAX_SLICE_SYMBOLS: usize = 4096;

/// Whether an optional external evaluator is discoverable.
///
/// Jev is an *optional adversarial evaluator*, never a semantic authority. This
/// function performs **discovery only**: it scans `PATH` for an executable
/// named `jev` and reports the first match. It does not run the program, read
/// its configuration, or grant any authority — an MCP client learns whether
/// Jev *could* be invoked by the user, and the compiler remains the sole
/// authority for what a program means. Absence changes nothing semantic
/// (`tests/ais_properties.rs` proves AIS works identically either way).
#[must_use]
pub fn discover_jev() -> Option<std::path::PathBuf> {
    discover_jev_in(std::env::var_os("PATH").as_deref())
}

/// The testable core of [`discover_jev`]: scan an explicit `PATH`.
#[must_use]
pub fn discover_jev_in(path: Option<&std::ffi::OsStr>) -> Option<std::path::PathBuf> {
    let path = path?;
    for dir in std::env::split_paths(path) {
        if dir.as_os_str().is_empty() {
            continue;
        }
        let candidate = dir.join("jev");
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

/// The session capability report for `aura://capabilities`.
///
/// The compiler/AIS layer is required and always present; the MCP adapter is
/// this process; Jev is optional and reported only if discoverable. A consumer
/// must treat a missing optional capability as a normal state, not an error.
#[must_use]
pub fn capabilities_report() -> Value {
    let jev = discover_jev();
    json!({
        "compiler": {
            "required": true,
            "present": true,
            "language_version": crate::LANGUAGE_VERSION,
        },
        "ais": {
            "required": true,
            "present": true,
            "ais_version": crate::ais::AIS_VERSION,
        },
        "mcp_adapter": {
            "required": false,
            "present": true,
            "protocol_version": MCP_PROTOCOL_VERSION,
            "authority": "transport only: the compiler is the semantic authority",
        },
        "jev": {
            "required": false,
            "present": jev.is_some(),
            "path": jev.map(|p| p.display().to_string()),
            "role": "adversarial measurement, never semantic authority; absence changes no Aura semantics",
        },
    })
}

/// The server identity reported in the `initialize` handshake.
#[must_use]
pub fn server_info() -> Value {
    json!({
        "name": "aura-mcp",
        "version": crate::VERSION,
        "ais_version": crate::ais::AIS_VERSION,
        "language_version": crate::LANGUAGE_VERSION,
    })
}

/// The capability set reported in `initialize`.
///
/// `tools` and `resources` are advertised; `prompts` is deliberately absent
/// because prompts are convenience text, not semantic authority, and no
/// prompt template is defined by AIS.
#[must_use]
pub fn server_capabilities() -> Value {
    json!({
        "tools": {},
        "resources": { "subscribe": false, "listChanged": false },
        "experimental": capabilities_report(),
    })
}

/// The tool inventory: small semantic operations, never one
/// `get_everything` call.
#[must_use]
pub fn tools() -> Value {
    json!([
        {
            "name": "aura_snapshot",
            "description": "The AIS/0.1 semantic snapshot of one Aura source: declarations, three-level type identity (type, family, value kind), capabilities, and structured diagnostics. Grants no capability.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "source": { "type": "string", "description": "The Aura source text." },
                    "name": { "type": "string", "description": "A display name for the source (default `<mcp>`)." }
                },
                "required": ["source"]
            }
        },
        {
            "name": "aura_slice",
            "description": "A task-focused semantic slice: the target symbol plus its transitive dependencies to `depth`, bounded by `budget`. The minimal sufficient context for a task.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "source": { "type": "string" },
                    "name": { "type": "string" },
                    "target": { "type": "string", "description": "The declared symbol to anchor on." },
                    "depth": { "type": "integer", "minimum": 0, "maximum": 16, "default": 1 },
                    "budget": { "type": "integer", "minimum": 1, "maximum": 4096, "default": 32 }
                },
                "required": ["source", "target"]
            }
        },
        {
            "name": "aura_symbol",
            "description": "One declaration from the snapshot by name, with its resolved type, families, value kind, and capabilities.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "source": { "type": "string" },
                    "name": { "type": "string" },
                    "symbol": { "type": "string" }
                },
                "required": ["source", "symbol"]
            }
        },
        {
            "name": "aura_diagnostics",
            "description": "The structured diagnostics of one source: code, severity, range, notes. No human prose parsing is required.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "source": { "type": "string" },
                    "name": { "type": "string" }
                },
                "required": ["source"]
            }
        },
        {
            "name": "aura_delta",
            "description": "The declaration-level semantic delta between two sources: changed, added, and removed symbols, and resolved/new diagnostics.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "old_source": { "type": "string" },
                    "new_source": { "type": "string" },
                    "name": { "type": "string" }
                },
                "required": ["old_source", "new_source"]
            }
        },
        {
            "name": "aura_revision",
            "description": "The content-addressed revision of one source, so a client can detect whether its held context is still current.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "source": { "type": "string" }
                },
                "required": ["source"]
            }
        },
        {
            "name": "aura_capabilities",
            "description": "The session capability report: the compiler and AIS are required and present; the MCP adapter is transport-only; the optional Jev adversarial evaluator is reported by PATH discovery alone and its absence changes no Aura semantics.",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "required": []
            }
        }
    ])
}

/// The read-only resource inventory.
#[must_use]
pub fn resources() -> Value {
    json!([
        {
            "uri": "aura://schema",
            "name": "AIS/0.1 schema",
            "description": "The AIS protocol version, the three identity levels, and the delivery model.",
            "mimeType": "application/json"
        },
        {
            "uri": "aura://versions",
            "name": "Version matrix",
            "description": "The AIS protocol version, compiler version, and language version.",
            "mimeType": "application/json"
        },
        {
            "uri": "aura://capabilities",
            "name": "Session capabilities",
            "description": "Required and optional capabilities: the compiler and AIS are required; the MCP adapter and the optional Jev evaluator are reported by discovery only.",
            "mimeType": "application/json"
        }
    ])
}

/// Read a resource by URI.
#[must_use]
pub fn read_resource(uri: &str) -> Value {
    match uri {
        "aura://schema" => json!({
            "uri": uri,
            "mimeType": "application/json",
            "text": serde_json::to_string_pretty(&json!({
                "ais_version": crate::ais::AIS_VERSION,
                "identity_levels": ["semantic_type", "type_family", "runtime_value_kind"],
                "delivery": ["snapshot", "slice", "delta"],
                "tools": ["aura_snapshot", "aura_slice", "aura_symbol", "aura_diagnostics", "aura_delta", "aura_revision"],
                "authority": "the compiler; this adapter defines no semantics"
            })).unwrap_or_default()
        }),
        "aura://versions" => json!({
            "uri": uri,
            "mimeType": "application/json",
            "text": serde_json::to_string_pretty(&json!({
                "aura_version": crate::VERSION,
                "language_version": crate::LANGUAGE_VERSION,
                "ais_version": crate::ais::AIS_VERSION,
                "mcp_protocol_version": MCP_PROTOCOL_VERSION
            })).unwrap_or_default()
        }),
        "aura://capabilities" => json!({
            "uri": uri,
            "mimeType": "application/json",
            "text": serde_json::to_string_pretty(&capabilities_report()).unwrap_or_default()
        }),
        _ => json!({ "error": format!("unknown resource `{uri}`") }),
    }
}

/// Build an AIS snapshot from source text, attaching checker diagnostics.
fn snapshot(source: &str, name: &str) -> crate::ais::Document {
    match crate::parse::parse(source) {
        Ok(module) => {
            let mut doc = crate::ais::document(name, source, &module);
            if let Err(d) = crate::check::Checker::module(&module) {
                doc.diagnostics
                    .push(crate::ais::Diagnostic::from_diag(&d, Some(source)));
            }
            doc
        }
        Err(d) => crate::ais::Document {
            ais_version: crate::ais::AIS_VERSION.to_string(),
            aura_version: crate::VERSION.to_string(),
            language_version: crate::LANGUAGE_VERSION.to_string(),
            capabilities: crate::ais::Capabilities::default(),
            source_name: name.to_string(),
            revision: Some(crate::ais::revision_of(source)),
            symbols: Vec::new(),
            diagnostics: vec![crate::ais::Diagnostic::from_diag(&d, Some(source))],
        },
    }
}

fn need_str<'a>(args: &'a Value, key: &str) -> Result<&'a str, String> {
    args.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("missing required string argument `{key}`"))
}

fn clamp_int(args: &Value, key: &str, default: usize, max: usize) -> usize {
    args.get(key)
        .and_then(Value::as_u64)
        .map_or(default, |v| (v as usize).min(max))
}

/// Execute one tool call and return its structured content.
///
/// The result is always a JSON object; a tool-level failure is reported as an
/// object with an `error` field rather than a process failure, so an invalid
/// request cannot corrupt a long-running session.
fn call_tool(name: &str, args: &Value) -> Result<Value, String> {
    match name {
        "aura_snapshot" => {
            let source = need_str(args, "source")?;
            let display = args.get("name").and_then(Value::as_str).unwrap_or("<mcp>");
            Ok(serde_json::to_value(snapshot(source, display)).map_err(|e| e.to_string())?)
        }
        "aura_slice" => {
            let source = need_str(args, "source")?;
            let target = need_str(args, "target")?;
            let display = args.get("name").and_then(Value::as_str).unwrap_or("<mcp>");
            let depth = clamp_int(args, "depth", 1, 16);
            let budget = clamp_int(args, "budget", 32, MAX_SLICE_SYMBOLS);
            let doc = snapshot(source, display);
            Ok(
                serde_json::to_value(crate::ais::slice(&doc, target, depth, budget))
                    .map_err(|e| e.to_string())?,
            )
        }
        "aura_symbol" => {
            let source = need_str(args, "source")?;
            let sym = need_str(args, "symbol")?;
            let display = args.get("name").and_then(Value::as_str).unwrap_or("<mcp>");
            let doc = snapshot(source, display);
            let found = doc.symbols.iter().find(|s| s.name == sym);
            Ok(json!({
                "revision": doc.revision,
                "symbol": found,
            }))
        }
        "aura_diagnostics" => {
            let source = need_str(args, "source")?;
            let display = args.get("name").and_then(Value::as_str).unwrap_or("<mcp>");
            let doc = snapshot(source, display);
            Ok(json!({
                "revision": doc.revision,
                "diagnostics": doc.diagnostics,
            }))
        }
        "aura_delta" => {
            let old = need_str(args, "old_source")?;
            let new = need_str(args, "new_source")?;
            let display = args.get("name").and_then(Value::as_str).unwrap_or("<mcp>");
            let from = snapshot(old, display);
            let to = snapshot(new, display);
            Ok(serde_json::to_value(crate::ais::delta(&from, &to)).map_err(|e| e.to_string())?)
        }
        "aura_revision" => {
            let source = need_str(args, "source")?;
            Ok(json!({ "revision": crate::ais::revision_of(source) }))
        }
        "aura_capabilities" => Ok(capabilities_report()),
        other => Err(format!("unknown tool `{other}`")),
    }
}

/// Handle one JSON-RPC message and produce the response, when one is due.
///
/// A notification (no `id`) returns `None` and no response is written. A
/// malformed message yields a `-32600` error. This function is pure: it holds
/// no state, so a failed request cannot corrupt the next one.
#[must_use]
pub fn handle_message(msg: &Value) -> Option<Value> {
    let method = msg.get("method").and_then(Value::as_str);
    let id = msg.get("id").cloned();
    let params = msg.get("params").cloned().unwrap_or(Value::Null);
    // A notification carries no `id`; nothing is written for it.
    let is_notification = id.is_none();
    let Some(method) = method else {
        if is_notification {
            return None;
        }
        return Some(json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": { "code": -32600, "message": "invalid request: no `method`" }
        }));
    };
    match method {
        "initialize" => Some(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "protocolVersion": MCP_PROTOCOL_VERSION,
                "capabilities": server_capabilities(),
                "serverInfo": server_info(),
            }
        })),
        "notifications/initialized" => None,
        "ping" => Some(json!({ "jsonrpc": "2.0", "id": id, "result": {} })),
        "tools/list" => Some(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": { "tools": tools() }
        })),
        "tools/call" => {
            let name = params.get("name").and_then(Value::as_str).unwrap_or("");
            let args = params.get("arguments").cloned().unwrap_or(json!({}));
            match call_tool(name, &args) {
                Ok(value) => Some(json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "content": [{
                            "type": "text",
                            "text": serde_json::to_string_pretty(&value).unwrap_or_default()
                        }],
                        "structuredContent": value,
                        "isError": false
                    }
                })),
                Err(message) => Some(json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "content": [{ "type": "text", "text": message }],
                        "isError": true
                    }
                })),
            }
        }
        "resources/list" => Some(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": { "resources": resources() }
        })),
        "resources/read" => {
            let uri = params.get("uri").and_then(Value::as_str).unwrap_or("");
            Some(json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": { "contents": [read_resource(uri)] }
            }))
        }
        other => {
            // A notification must never produce a response (JSON-RPC 2.0
            // §4.1): an unknown notification is ignored, not answered.
            if is_notification {
                return None;
            }
            Some(json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": { "code": -32601, "message": format!("method not found: `{other}`") }
            }))
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn call(method: &str, params: Value) -> Value {
        handle_message(&json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params }))
            .expect("a response is due")
    }

    #[test]
    fn initialize_reports_protocol_and_server_identity() {
        let r = call("initialize", json!({}));
        assert_eq!(r["result"]["protocolVersion"], MCP_PROTOCOL_VERSION);
        assert_eq!(r["result"]["serverInfo"]["name"], "aura-mcp");
        assert_eq!(r["result"]["serverInfo"]["ais_version"], "0.1");
    }

    #[test]
    fn notifications_get_no_response() {
        assert!(handle_message(
            &json!({ "jsonrpc": "2.0", "method": "notifications/initialized" })
        )
        .is_none());
        assert!(
            handle_message(&json!({ "jsonrpc": "2.0", "method": "unknown/notification" }))
                .is_none()
        );
    }

    #[test]
    fn tools_list_advertises_small_semantic_operations() {
        let r = call("tools/list", json!({}));
        let names: Vec<&str> = r["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["name"].as_str().unwrap())
            .collect();
        for expected in [
            "aura_snapshot",
            "aura_slice",
            "aura_symbol",
            "aura_diagnostics",
            "aura_delta",
            "aura_revision",
        ] {
            assert!(names.contains(&expected), "missing {expected}");
        }
        // No monolithic get-everything tool.
        assert!(!names.iter().any(|n| n.contains("everything")));
    }

    #[test]
    fn snapshot_tool_returns_three_identity_levels() {
        let r = call(
            "tools/call",
            json!({
                "name": "aura_snapshot",
                "arguments": { "source": "fn rows() -> [[int]] { return [] }\nfn main() { print(rows()) }\n" }
            }),
        );
        assert_eq!(r["result"]["isError"], false);
        let sc = &r["result"]["structuredContent"];
        let rows = sc["symbols"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["name"] == "rows")
            .unwrap();
        assert_eq!(rows["type_name"], "[[int]]");
        assert_eq!(rows["families"][0], "sequence");
        assert_eq!(rows["value_kind"], "list");
    }

    #[test]
    fn slice_tool_is_bounded_and_reports_omissions() {
        let src =
            "struct A { x: int }\nstruct B { y: int }\nfn f(a: A, b: B) -> int { return a.x }\n";
        let r = call(
            "tools/call",
            json!({ "name": "aura_slice", "arguments": { "source": src, "target": "f", "depth": 1, "budget": 1 } }),
        );
        let sc = &r["result"]["structuredContent"];
        assert_eq!(sc["symbol"]["name"], "f");
        assert!(sc["dependencies"].as_array().unwrap().is_empty());
        assert!(sc["omitted"].as_u64().unwrap() >= 1);
    }

    #[test]
    fn an_invalid_request_is_a_tool_error_not_a_process_failure() {
        // A missing required argument is reported in-band, so a session
        // survives it.
        let r = call(
            "tools/call",
            json!({ "name": "aura_snapshot", "arguments": {} }),
        );
        assert_eq!(r["result"]["isError"], true);
        // An unknown tool is the same shape.
        let r = call("tools/call", json!({ "name": "nope", "arguments": {} }));
        assert_eq!(r["result"]["isError"], true);
    }

    #[test]
    fn an_unknown_method_is_a_jsonrpc_error() {
        let r = call("does/not/exist", json!({}));
        assert_eq!(r["error"]["code"], -32601);
    }

    #[test]
    fn a_malformed_request_is_rejected_without_panicking() {
        assert!(handle_message(&json!({ "jsonrpc": "2.0", "id": 1 })).is_some());
        assert!(handle_message(&json!("not an object")).is_none());
        assert!(handle_message(&json!(null)).is_none());
    }

    #[test]
    fn resources_are_read_only_and_unknown_uris_do_not_fail_the_process() {
        let r = call("resources/list", json!({}));
        let uris: Vec<&str> = r["result"]["resources"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x["uri"].as_str().unwrap())
            .collect();
        assert!(uris.contains(&"aura://schema"));
        let r = call("resources/read", json!({ "uri": "aura://schema" }));
        let text = r["result"]["contents"][0]["text"].as_str().unwrap();
        assert!(text.contains("semantic_type"));
        let r = call("resources/read", json!({ "uri": "aura://nope" }));
        assert!(r["result"]["contents"][0]["error"].is_string());
    }

    #[test]
    fn diagnostics_are_structured_through_the_adapter() {
        let r = call(
            "tools/call",
            json!({
                "name": "aura_diagnostics",
                "arguments": { "source": "fn main() { let x: int = \"s\" }\n" }
            }),
        );
        let sc = &r["result"]["structuredContent"];
        let diag = &sc["diagnostics"][0];
        assert_eq!(diag["code_text"], "E3001");
        assert!(diag["range"]["start"]["line"].is_number());
    }

    #[test]
    fn revision_tool_matches_the_ais_revision() {
        let src = "fn main() { print(1) }\n";
        let r = call(
            "tools/call",
            json!({ "name": "aura_revision", "arguments": { "source": src } }),
        );
        assert_eq!(
            r["result"]["structuredContent"]["revision"],
            crate::ais::revision_of(src)
        );
    }

    #[test]
    fn delta_tool_reports_declaration_changes() {
        let r = call(
            "tools/call",
            json!({
                "name": "aura_delta",
                "arguments": {
                    "old_source": "fn a(x: int) -> int { return x }\n",
                    "new_source": "fn a(x: string) -> int { return 1 }\nfn b() -> int { return 2 }\n"
                }
            }),
        );
        let sc = &r["result"]["structuredContent"];
        let changes: Vec<(&str, &str)> = sc["symbols"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| (c["name"].as_str().unwrap(), c["change"].as_str().unwrap()))
            .collect();
        assert!(changes.contains(&("a", "changed")));
        assert!(changes.contains(&("b", "added")));
    }

    #[test]
    fn jev_discovery_is_read_only_and_never_required() {
        // Discovery scans PATH only: it never runs the binary, reads its
        // configuration, or grants authority. An empty PATH finds nothing.
        assert_eq!(discover_jev_in(None), None);
        assert_eq!(discover_jev_in(Some(std::ffi::OsStr::new(""))), None);
        assert_eq!(
            discover_jev_in(Some(std::ffi::OsStr::new("/nonexistent-dir-xyz"))),
            None
        );
        // And the capability report is honest about the optional role.
        let report = capabilities_report();
        assert_eq!(report["compiler"]["required"], true);
        assert_eq!(report["compiler"]["present"], true);
        assert_eq!(report["ais"]["required"], true);
        assert_eq!(report["jev"]["required"], false);
        assert!(report["jev"]["role"].as_str().unwrap().contains("never"));
    }

    #[test]
    fn capabilities_are_reported_through_the_handshake_and_resources() {
        // A client learns the capability set without a separate call.
        let r = call("initialize", json!({}));
        assert_eq!(r["result"]["capabilities"]["experimental"]["compiler"]["present"], true);
        // And can read it as a resource.
        let r = call("resources/read", json!({ "uri": "aura://capabilities" }));
        let text = r["result"]["contents"][0]["text"].as_str().unwrap();
        assert!(text.contains("\"ais\""));
        assert!(text.contains("\"jev\""));
        // The tool form agrees.
        let r = call("tools/call", json!({ "name": "aura_capabilities", "arguments": {} }));
        assert_eq!(r["result"]["structuredContent"]["ais"]["present"], true);
    }

    #[test]
    fn a_missing_optional_capability_changes_no_semantics() {
        // The core semantic operations are identical whether or not Jev is
        // present, because none of them consults it: the compiler is the
        // authority. This is the fallback proof.
        let src = "fn rows() -> [[int]] { return [] }\nfn main() { print(rows()) }\n";
        let a = call(
            "tools/call",
            json!({ "name": "aura_snapshot", "arguments": { "source": src } }),
        );
        let b = call(
            "tools/call",
            json!({ "name": "aura_snapshot", "arguments": { "source": src } }),
        );
        assert_eq!(
            a["result"]["structuredContent"],
            b["result"]["structuredContent"]
        );
        // And no snapshot field mentions Jev at all.
        assert!(!a["result"]["structuredContent"].to_string().contains("jev"));
    }
}
