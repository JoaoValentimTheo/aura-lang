#![no_main]
//! Fuzz target: the AIS/0.1 model and the MCP adapter.
//!
//! INVARIANT: no input reaches a Rust panic, abort, hang, or unbounded
//! allocation. Arbitrary bytes are treated as (a) source text for the AIS
//! snapshot/slice/delta, and (b) JSON-RPC frames for the MCP handler. Every
//! path terminates with a value or a structured error.
//!
//! AIS is a *reader* surface (it never executes), so the risk class here is a
//! crash or unbounded expansion while describing a hostile document, not
//! authority escalation. The MCP handler must be total: a malformed frame is
//! a JSON-RPC error, never a process failure, because a long-running session
//! must survive a client bug.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // (a) Treat the bytes as source text (lossy, like the CLI's outer decode).
    let text = String::from_utf8_lossy(data);
    let parsed = aura::parse::parse(&text);
    let module = match parsed {
        Ok(m) => m,
        Err(_) => {
            // The snapshot still exists for an unparseable source.
            let doc = aura::ais::Document {
                ais_version: aura::ais::AIS_VERSION.to_string(),
                aura_version: aura::VERSION.to_string(),
                language_version: aura::LANGUAGE_VERSION.to_string(),
                capabilities: aura::ais::Capabilities::default(),
                source_name: "<fuzz>".to_string(),
                revision: Some(aura::ais::revision_of(&text)),
                symbols: Vec::new(),
                narrowings: Vec::new(),
                diagnostics: Vec::new(),
            };
            let _ = aura::ais::slice(&doc, "target", 4, 16);
            return;
        }
    };
    let doc = aura::ais::document("<fuzz>", &text, &module);
    // Bounded slice with an arbitrary target and bounded depth/budget.
    let _ = aura::ais::slice(&doc, "target", 4, 16);
    let _ = aura::ais::slice(&doc, "", 0, 1);
    // Delta against a structurally different document.
    let other = aura::ais::document("<fuzz2>", &text, &module);
    let _ = aura::ais::delta(&doc, &other);

    // (b) Treat the same bytes as an MCP frame.
    if let Ok(value) = serde_json::from_slice::<serde_json::Value>(data) {
        let _ = aura::mcp::handle_message(&value);
    }
    // And as a well-formed request whose `source` is arbitrary text.
    let frame = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": { "name": "aura_snapshot", "arguments": { "source": text } }
    });
    let _ = aura::mcp::handle_message(&frame);
});
