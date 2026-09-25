#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Native tests for the Playground execution engine.
//!
//! The wrapper's [`execute`] is the exact engine the WebAssembly ABI calls, so
//! running it natively exercises the same code path the browser uses, without
//! a browser. It proves the structured-result contract independent of the
//! browser integration suite.

use aura_playground_runtime as rt;

/// Encode options as the runtime's line protocol.
fn options(args: &[&str], stdin: Option<&str>) -> Vec<u8> {
    let mut out = Vec::new();
    for a in args {
        out.extend_from_slice(format!("arg {a}\n").as_bytes());
    }
    if let Some(s) = stdin {
        out.extend_from_slice(format!("stdin-bytes {}\n", s.len()).as_bytes());
        out.extend_from_slice(s.as_bytes());
    }
    out
}

fn parse(json: &str) -> serde_json::Value {
    serde_json::from_str(json).expect("valid JSON result")
}

#[test]
fn version_model_is_coherent() {
    // The language semantics stay at 0.0.1; the runtime artifact is a
    // development pre-release on the 0.0.2 line, never equal to the release.
    assert_eq!(aura::LANGUAGE_VERSION, "0.0.1");
    assert_eq!(aura::VERSION, "0.0.2");
    assert_eq!(rt::RUNTIME_VERSION, env!("CARGO_PKG_VERSION"));
    assert!(
        rt::RUNTIME_VERSION.starts_with("0.0.2-"),
        "development runtime must be a pre-release of its release line: {}",
        rt::RUNTIME_VERSION
    );
    assert_ne!(rt::RUNTIME_VERSION, aura::VERSION);
    assert_eq!(rt::ABI_VERSION, 1);
    let (_json, _status, language_version) = rt::execute("fn main() {}", &[]);
    assert_eq!(language_version, "0.0.1");
}

#[test]
fn ok_result_is_structured() {
    let (json, status, version) = rt::execute("fn main() { print(1 + 2) }", &[]);
    assert_eq!(status, rt::status::OK);
    assert_eq!(version, "0.0.1");
    let v = parse(&json);
    assert_eq!(v["status"], "ok");
    assert_eq!(v["stdout"], "3\n");
    assert_eq!(v["result"], serde_json::Value::Null);
    assert_eq!(v["diagnostics"].as_array().unwrap().len(), 0);
}

#[test]
fn diagnostic_carries_code_and_position() {
    let (json, status, _) = rt::execute("fn main() {\n throw \"boom\"\n}", &[]);
    assert_eq!(status, rt::status::DIAGNOSTIC);
    let v = parse(&json);
    assert_eq!(v["status"], "diagnostic");
    let d = &v["diagnostics"][0];
    assert_eq!(d["code"], 4026);
    assert_eq!(d["code_text"], "E4026");
    assert!(d["line"].as_u64().unwrap() >= 1);
    assert!(d["column"].as_u64().unwrap() >= 1);
}

#[test]
fn args_and_stdin_route_through_the_host() {
    let (json, status, _) = rt::execute(
        "fn main() {\n print(args())\n print(read_line())\n}",
        &options(&["a", "b"], Some("hi\n")),
    );
    assert_eq!(status, rt::status::OK);
    assert_eq!(parse(&json)["stdout"], "[\"a\", \"b\"]\nhi\n");
}

#[test]
fn unavailable_capabilities_are_e5002() {
    for src in [
        "fn main() { read_file(\"/x\") }",
        "fn main() { write_file(\"/x\", \"y\") }",
        "fn main() { time_unix() }",
        "fn main() { sleep_ms(1) }",
    ] {
        let (json, status, _) = rt::execute(src, &[]);
        assert_eq!(status, rt::status::DIAGNOSTIC, "{src}");
        assert_eq!(parse(&json)["diagnostics"][0]["code"], 5002, "{src}");
    }
}

#[test]
fn no_main_falls_back_to_module_semantics() {
    let (json, status, _) = rt::execute("1 + 2", &[]);
    assert_eq!(status, rt::status::OK);
    let v = parse(&json);
    assert_eq!(v["result"], "3");
}

#[test]
fn recursion_limit_is_e4011_not_internal() {
    let (json, status, _) = rt::execute("fn f() { f() }\nfn main() { f() }", &[]);
    assert_eq!(status, rt::status::DIAGNOSTIC);
    assert_eq!(parse(&json)["diagnostics"][0]["code"], 4011);
}

#[test]
fn oversized_source_is_rejected_without_running() {
    let src = "a".repeat(rt::limits::MAX_SOURCE_BYTES + 1);
    let (json, status, _) = rt::execute(&src, &[]);
    assert_eq!(status, rt::status::DIAGNOSTIC);
    assert_eq!(parse(&json)["diagnostics"][0]["code"], 4020);
}

#[test]
fn oversized_stdin_is_rejected() {
    let big = "x".repeat(rt::limits::MAX_STDIN_BYTES + 1);
    let (json, status, _) = rt::execute("fn main() {}", &options(&[], Some(&big)));
    assert_eq!(status, rt::status::DIAGNOSTIC);
    assert_eq!(parse(&json)["diagnostics"][0]["code"], 4020);
}

#[test]
fn deterministic_across_runs() {
    let a = rt::execute("fn main() { print(\"x\") }", &[]).0;
    let b = rt::execute("fn main() { print(\"x\") }", &[]).0;
    assert_eq!(a, b);
}
