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
    // ADR-0001: release, language, and runtime identities are separate. The
    // runtime artifact records its own pre-release identity (a development
    // runtime is not a release); the language version is the observable
    // language contract; the release version is at least the language version.
    assert_eq!(rt::RUNTIME_VERSION, env!("CARGO_PKG_VERSION"));
    assert_eq!(rt::ABI_VERSION, 1);
    assert!(aura::LANGUAGE_VERSION <= aura::VERSION);

    // The runtime reports the language contract it actually implements, read
    // from the constant, not a hardcoded duplicate. This build tracks the
    // current language line, so its reported language version equals the
    // crate's `LANGUAGE_VERSION`.
    let (_json, _status, language_version) = rt::execute("fn main() {}", &[]);
    assert_eq!(language_version, aura::LANGUAGE_VERSION);
}

#[test]
fn ok_result_is_structured() {
    let (json, status, version) = rt::execute("fn main() { print(1 + 2) }", &[]);
    assert_eq!(status, rt::status::OK);
    assert_eq!(version, aura::LANGUAGE_VERSION);
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

/// Too many arguments is a host transport rejection (`E4020`), reported as a
/// structured diagnostic rather than a partial run.
#[test]
fn too_many_args_are_rejected() {
    let args: Vec<String> = (0..=rt::limits::MAX_ARGS).map(|i| i.to_string()).collect();
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let (json, status, _) = rt::execute("fn main() {}", &options(&refs, None));
    assert_eq!(status, rt::status::DIAGNOSTIC);
    assert_eq!(parse(&json)["diagnostics"][0]["code"], 4020);
    // Exactly the limit is accepted.
    let ok_refs: Vec<&str> = args[..rt::limits::MAX_ARGS]
        .iter()
        .map(String::as_str)
        .collect();
    let (_json, status, _) = rt::execute("fn main() {}", &options(&ok_refs, None));
    assert_eq!(status, rt::status::OK);
}

/// A single argument over the byte limit is rejected; exactly the limit is
/// accepted.
#[test]
fn oversized_arg_is_rejected() {
    let big = "a".repeat(rt::limits::MAX_ARG_BYTES + 1);
    let (json, status, _) = rt::execute("fn main() {}", &options(&[&big], None));
    assert_eq!(status, rt::status::DIAGNOSTIC);
    assert_eq!(parse(&json)["diagnostics"][0]["code"], 4020);

    let at_limit = "a".repeat(rt::limits::MAX_ARG_BYTES);
    let (_json, status, _) = rt::execute("fn main() {}", &options(&[&at_limit], None));
    assert_eq!(status, rt::status::OK);
}

/// An argument is delivered to `args()` unchanged when within the bounds.
#[test]
fn args_round_trip_through_the_host() {
    let (json, status, _) = rt::execute(
        "fn main() { print(args()) }",
        &options(&["one", "two"], None),
    );
    assert_eq!(status, rt::status::OK);
    let v = parse(&json);
    assert_eq!(v["stdout"], "[\"one\", \"two\"]\n");
}

#[test]
fn deterministic_across_runs() {
    let a = rt::execute("fn main() { print(\"x\") }", &[]).0;
    let b = rt::execute("fn main() { print(\"x\") }", &[]).0;
    assert_eq!(a, b);
}

// ---------------------------------------------------------------------------
// Standard-output capture bound (Host ABI application resource policy)
// ---------------------------------------------------------------------------
//
// `rt::limits::MAX_STDOUT_BYTES` is the Playground's bounded-capture policy:
// the interpreter emits to a host-controlled sink, and the host refuses any
// write that would cross the bound. These tests pin the exact contract the
// observed `E4020` relies on: capture stops at the bound, a crossing write is
// refused whole with earlier bytes retained, the refusal is fatal to the run
// (not catchable; `finally` regions still run), and the bound is per
// execution (every `execute*` builds a fresh `BrowserHost`).

/// `print(s)` writes `s.len()` bytes plus one newline, so a stdin string of
/// `LIMIT - 1` bytes lands on exactly the bound.
const LIMIT: usize = rt::limits::MAX_STDOUT_BYTES;

fn stdout_of(v: &serde_json::Value) -> &str {
    v["stdout"].as_str().expect("stdout is a string")
}

fn run_with_stdin(src: &str, stdin: &str) -> serde_json::Value {
    let (json, _status, _version) = rt::execute(src, &options(&[], Some(stdin)));
    parse(&json)
}

#[test]
fn stdout_capture_accepts_exactly_the_limit() {
    let v = run_with_stdin(
        "fn main() {\n let s = read_line()\n print(s)\n}",
        &"a".repeat(LIMIT - 1),
    );
    assert_eq!(v["status"], "ok");
    assert_eq!(stdout_of(&v).len(), LIMIT);
}

#[test]
fn stdout_capture_over_limit_is_e4020_with_prior_output_retained() {
    let v = run_with_stdin(
        "fn main() {\n print(\"before\")\n let s = read_line()\n print(s)\n}",
        &"a".repeat(LIMIT),
    );
    assert_eq!(v["status"], "diagnostic");
    let d = &v["diagnostics"][0];
    assert_eq!(d["code"], 4020);
    assert!(
        d["message"]
            .as_str()
            .unwrap()
            .contains("1048576 byte limit"),
        "{d}"
    );
    assert_eq!(d["line"], 4);
    // The bytes accepted before the crossing write are retained; the refused
    // write contributed nothing (atomicity).
    assert_eq!(stdout_of(&v), "before\n");
}

#[test]
fn stdout_refusal_is_atomic_for_one_oversized_write() {
    let v = run_with_stdin(
        "fn main() {\n let s = read_line()\n print(s)\n}",
        &"a".repeat(LIMIT),
    );
    assert_eq!(v["status"], "diagnostic");
    assert_eq!(v["diagnostics"][0]["code"], 4020);
    assert_eq!(stdout_of(&v), "");
}

#[test]
fn many_small_writes_cross_the_bound_atomically() {
    // 61680 prints of "0123456789abcdef" x 17 bytes (newline included) fill
    // 1_048_560 bytes; the next 18-byte print exceeds the bound and is
    // refused whole, so the captured output is exactly the accumulated bytes.
    let v = run_with_stdin(
        "fn main() {\n let mut i = 0\n while i < 61680 { print(\"0123456789abcdef\")\n i = i + 1 }\n print(\"0123456789abcdefX\")\n}",
        "",
    );
    assert_eq!(v["status"], "diagnostic");
    assert_eq!(v["diagnostics"][0]["code"], 4020);
    assert_eq!(stdout_of(&v).len(), 61680 * 17);
    assert!(stdout_of(&v).ends_with("0123456789abcdef\n"));
}

#[test]
fn stdout_utf8_write_landing_exactly_on_the_bound_is_valid() {
    // Fill LIMIT - 4 bytes, then one `print("€")` writes 4 UTF-8 bytes.
    let vm = run_with_stdin(
        "fn main() {\n let s = read_line()\n print(s)\n print(\"€\")\n}",
        &"a".repeat(LIMIT - 5),
    );
    assert_eq!(vm["status"], "ok");
    // `String::len` is the UTF-8 byte length, which is what the bound counts.
    assert_eq!(stdout_of(&vm).len(), LIMIT);
    assert!(stdout_of(&vm).ends_with("€\n"));
}

#[test]
fn stdout_utf8_write_one_byte_over_is_refused_whole() {
    // LIMIT - 3 accepted bytes, then 4 more would cross: refused whole, the
    // retained prefix is byte-identical to what was accepted.
    let vm = run_with_stdin(
        "fn main() {\n let s = read_line()\n print(s)\n print(\"€\")\n}",
        &"a".repeat(LIMIT - 4),
    );
    assert_eq!(vm["status"], "diagnostic");
    assert_eq!(vm["diagnostics"][0]["code"], 4020);
    assert_eq!(stdout_of(&vm).len(), LIMIT - 3);
}

#[test]
fn stdout_e4020_is_not_catchable_and_finally_still_runs() {
    // The `try` body's print crosses the bound (LIMIT + 1 bytes): the failure
    // is a fatal host I/O failure, so `catch` must not intercept it, while the
    // `finally` block still runs — and its short write succeeds against the
    // empty capture.
    let v = run_with_stdin(
        "fn main() {\n let s = read_line()\n try {\n print(s)\n } catch e {\n print(\"caught\")\n } finally {\n print(\"finally\")\n }\n}",
        &"a".repeat(LIMIT),
    );
    assert_eq!(v["status"], "diagnostic");
    assert_eq!(v["diagnostics"][0]["code"], 4020);
    assert_eq!(stdout_of(&v), "finally\n");
}

#[test]
fn stdout_bound_is_per_execution() {
    // Two separate executions may each land on exactly the bound: no state
    // (stdout or budget) survives across `execute*` calls.
    let src = "fn main() {\n let s = read_line()\n print(s)\n}";
    let stdin = "a".repeat(LIMIT - 1);
    for _ in 0..2 {
        let v = run_with_stdin(src, &stdin);
        assert_eq!(v["status"], "ok");
        assert_eq!(stdout_of(&v).len(), LIMIT);
    }
}

#[test]
fn runaway_print_reports_structured_e4020_not_a_trap() {
    // The reported Playground incident shape: a loop that prints far more
    // than the bound must terminate with a structured E4020, never a guest
    // trap or unbounded capture.
    let (json, status, _version) =
        rt::execute("fn main() {\n for i in 0..600000 { print(i) }\n}", &[]);
    assert_eq!(status, rt::status::DIAGNOSTIC);
    let v = parse(&json);
    assert_eq!(v["diagnostics"][0]["code"], 4020);
    assert!(v["stdout"].as_str().unwrap().len() <= LIMIT);
}
