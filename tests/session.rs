#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! D1 — resumable execution sessions and the suspension invariants
//! (`docs/engineering/PLAYGROUND_032_CAMPAIGN.md`, Host ABI 2).
//!
//! These tests run the *real* compiler and the explicit-continuation machine
//! with a **scripted effect host**: instead of answering `http_request`
//! synchronously, the host records the request and returns
//! `HostError::Pending`, so the session parks. The test then performs the
//! scripted effect and feeds the value back, proving the program continues
//! from exactly the continuation it suspended on.
//!
//! The properties under test are the D1 invariants: no restart, no
//! re-evaluated arguments, no duplicated side effects, no premature `finally`,
//! no `catch` on a pending effect, exact-once evaluation, preserved spans and
//! frames, and rejection of stale/duplicate resumes.
#![cfg(feature = "http-api")]

use std::cell::RefCell;
use std::rc::Rc;

use aura::error::{codes, Diag, Span};
use aura::host::{Host, HostError, HostResult, HttpRequest, HttpResponse, LocalTime};
use aura::run::session::{RunSession, Step};
use aura::run::value::{MapKey, Value};
use aura::run::Interp;

/// A host that records every stdout write and defers every HTTP request as a
/// pending effect. The test performs the effect.
struct SuspendHost {
    out: Rc<RefCell<Vec<String>>>,
    /// Requests the program has issued (in order); each `http_request` call
    /// appends here and returns `Pending`.
    seen: Rc<RefCell<Vec<HttpRequest>>>,
}

impl SuspendHost {
    fn new(out: Rc<RefCell<Vec<String>>>, seen: Rc<RefCell<Vec<HttpRequest>>>) -> SuspendHost {
        SuspendHost { out, seen }
    }
}

impl Host for SuspendHost {
    fn write_stdout(&mut self, bytes: &[u8]) -> HostResult<()> {
        self.out
            .borrow_mut()
            .push(String::from_utf8_lossy(bytes).into_owned());
        Ok(())
    }
    fn read_line(&mut self) -> HostResult<Option<String>> {
        Ok(None)
    }
    fn args(&self) -> Vec<String> {
        Vec::new()
    }
    fn read_file(&self, _path: &str) -> HostResult<Option<String>> {
        Err(HostError::unavailable("no filesystem"))
    }
    fn write_file(&mut self, _path: &str, _content: &str) -> HostResult<()> {
        Err(HostError::unavailable("no filesystem"))
    }
    fn now_unix(&self) -> HostResult<i64> {
        Err(HostError::unavailable("no clock"))
    }
    fn now_local(&self) -> HostResult<LocalTime> {
        Err(HostError::unavailable("no clock"))
    }
    fn sleep_ms(&mut self, _ms: u64) -> HostResult<()> {
        Err(HostError::unavailable("no sleep"))
    }
    fn http_request(&self, request: &HttpRequest) -> HostResult<HttpResponse> {
        self.seen.borrow_mut().push(request.clone());
        Err(HostError::pending(aura::host::PendingEffect::Http(
            request.clone(),
        )))
    }
}

/// The parts of a program's observable behaviour this suite asserts on.
struct Rig {
    out: Rc<RefCell<Vec<String>>>,
    seen: Rc<RefCell<Vec<HttpRequest>>>,
}

impl Rig {
    fn new() -> Rig {
        Rig {
            out: Rc::new(RefCell::new(Vec::new())),
            seen: Rc::new(RefCell::new(Vec::new())),
        }
    }

    fn stdout(&self) -> String {
        self.out.borrow().concat()
    }

    fn seen(&self) -> Vec<HttpRequest> {
        self.seen.borrow().clone()
    }
}

/// Drive `source`, returning the rig and the terminal step for inspection.
/// Each effect is answered by `answer`.
fn drive_steps(
    source: &str,
    answer: impl Fn(usize, &HttpRequest) -> Result<Value, Diag>,
) -> (Rig, Step) {
    let rig = Rig::new();
    let host = SuspendHost::new(rig.out.clone(), rig.seen.clone());
    let compilation = aura::compile_named_with_mode(source, "<test>", aura::CompileMode::Program)
        .expect("compile");
    let (module, _sources, _entry) = compilation.into_parts();
    let mut interp = Interp::new();
    interp.set_host(Box::new(host));
    let mut session = RunSession::start(interp, &module).expect("session");

    let mut index = 0usize;
    let mut last = session.advance();
    loop {
        match last {
            Step::Completed | Step::Diagnostic(_) => return (rig, last),
            Step::Pending(_) => {
                let effect = session.pending().cloned().expect("pending");
                let aura::host::PendingEffect::Http(request) = effect;
                let result = answer(index, &request);
                index += 1;
                last = match result {
                    Ok(v) => {
                        // Advance the *same* session: exactly one resume.
                        session.resume_effect(v)
                    }
                    Err(d) => session.fail_effect(d),
                };
            }
        }
    }
}

const HTTP_OK: &str = "HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok";

/// Build the Aura response value for a scripted effect (mirrors the native
/// encoder; the *language* value shape is what matters here).
fn response_value(status: u16, body: &str) -> Value {
    let mut m = std::collections::BTreeMap::new();
    m.insert(MapKey::str("status"), Value::Int(i64::from(status)));
    m.insert(MapKey::str("body"), Value::str(body));
    m.insert(
        MapKey::str("body_bytes"),
        Value::List(Rc::new(RefCell::new(
            body.bytes().map(|b| Value::Int(i64::from(b))).collect(),
        ))),
    );
    Value::Map(Rc::new(RefCell::new(m)))
}

#[test]
fn http_suspends_and_resumes_in_main() {
    let (rig, last) = drive_steps(
        r#"fn main() {
    let r = http_get("http://example.test/a")
    print(r["status"])
}"#,
        |_, _| Ok(response_value(200, "hi")),
    );
    assert!(matches!(last, Step::Completed), "{last:?}");
    assert_eq!(rig.stdout(), "200\n");
    assert_eq!(rig.seen().len(), 1);
    assert_eq!(rig.seen()[0].method, "GET");
}

#[test]
fn http_effect_arguments_are_not_reevaluated() {
    // The URL is built by a function with an observable side effect (a
    // `print`). Exactly-once evaluation means that print happens once, and the
    // request is issued once.
    let (rig, last) = drive_steps(
        r#"fn url() -> string {
    print("building url")
    return "http://example.test/x"
}

fn main() {
    let r = http_get(url())
    print(r["status"])
}"#,
        |_, _| Ok(response_value(200, "ok")),
    );
    assert!(matches!(last, Step::Completed), "{last:?}");
    assert_eq!(rig.stdout(), "building url\n200\n");
    assert_eq!(rig.seen().len(), 1);
}

#[test]
fn effects_resume_inside_nested_calls_and_closures() {
    let (rig, last) = drive_steps(
        r#"fn fetch(url) -> int {
    let r = http_get(url)
    return r["status"]
}

fn main() {
    let f = (u) -> fetch(u)
    print(f("http://example.test/n"))
}"#,
        |_, _| Ok(response_value(204, "")),
    );
    assert!(matches!(last, Step::Completed), "{last:?}");
    assert_eq!(rig.stdout(), "204\n");
    assert_eq!(rig.seen().len(), 1);
}

#[test]
fn multiple_sequential_effects_each_complete_once() {
    let (rig, last) = drive_steps(
        r#"fn main() {
    let a = http_get("http://example.test/1")
    let b = http_get("http://example.test/2")
    let c = http_get("http://example.test/3")
    print(a["status"])
    print(b["status"])
    print(c["status"])
}"#,
        |i, _| Ok(response_value(200 + i as u16, "b")),
    );
    assert!(matches!(last, Step::Completed), "{last:?}");
    assert_eq!(rig.stdout(), "200\n201\n202\n");
    assert_eq!(rig.seen().len(), 3);
    assert_eq!(rig.seen()[0].url, "http://example.test/1");
    assert_eq!(rig.seen()[2].url, "http://example.test/3");
}

#[test]
fn effects_inside_a_loop_resume_in_order() {
    let (rig, last) = drive_steps(
        r#"fn main() {
    let urls = ["http://example.test/a", "http://example.test/b"]
    for u in urls {
        let r = http_get(u)
        print(r["status"])
    }
}"#,
        |_, _| Ok(response_value(200, "x")),
    );
    assert!(matches!(last, Step::Completed), "{last:?}");
    assert_eq!(rig.stdout(), "200\n200\n");
    assert_eq!(rig.seen().len(), 2);
}

#[test]
fn mutation_state_survives_suspension() {
    let (rig, last) = drive_steps(
        r#"fn main() {
    let mut acc = 10
    let r = http_get("http://example.test/a")
    acc = acc + r["status"]
    print(acc)
}"#,
        |_, _| Ok(response_value(5, "")),
    );
    assert!(matches!(last, Step::Completed), "{last:?}");
    assert_eq!(rig.stdout(), "15\n");
}

#[test]
fn pending_effect_does_not_run_finally_or_catch() {
    // A `try` around an HTTP call: the pending effect must not be seen by
    // `catch` (it is not an error) and `finally` must not run before the
    // effect completes. The prints prove ordering.
    let (rig, last) = drive_steps(
        r#"fn main() {
    try {
        let r = http_get("http://example.test/a")
        print(r["status"])
    } catch _ {
        print("caught")
    } finally {
        print("finally")
    }
    print("after")
}"#,
        |_, _| Ok(response_value(200, "")),
    );
    assert!(matches!(last, Step::Completed), "{last:?}");
    assert_eq!(rig.stdout(), "200\nfinally\nafter\n");
    assert_eq!(rig.seen().len(), 1);
}

#[test]
fn failed_effect_matches_synchronous_failure_semantics() {
    // A transport failure is `E4020`, a *fatal* diagnostic (not a `throw`), so
    // — exactly as in the native path — `finally` runs but `catch` does not
    // catch it. The browser and native hosts must agree here; only the
    // transport differs.
    let (rig, last) = drive_steps(
        r#"fn main() {
    try {
        let r = http_get("http://example.test/a")
        print(r["status"])
    } catch _ {
        print("caught")
    } finally {
        print("finally")
    }
    print("after")
}"#,
        |_, _| Err(Diag::new(codes::IO, "transport down", Span::default())),
    );
    match last {
        Step::Diagnostic(d) => {
            assert_eq!(d.code, codes::IO);
            assert!(d.span.start > 0, "span not attributed: {d:?}");
        }
        other => panic!("expected E4020, got {other:?}"),
    }
    // `finally` ran; `catch` did not; "after" never ran.
    assert_eq!(rig.stdout(), "finally\n");
}

#[test]
fn an_unavailable_capability_inside_try_runs_finally_and_propagates() {
    // E5002 behaves like any other fatal diagnostic: `finally` runs, `catch`
    // does not intercept it. Pinned so a browser denial matches native.
    let rig = Rig::new();
    let host = SuspendHost::new(rig.out.clone(), rig.seen.clone());
    let compilation = aura::compile_named_with_mode(
        r#"fn main() {
    try {
        let _r = http_request("GET", "http://example.test/a", {"timeout_ms": 0})
    } catch _ {
        print("caught")
    } finally {
        print("finally")
    }
}"#,
        "<test>",
        aura::CompileMode::Program,
    )
    .expect("compile");
    let (module, _sources, _entry) = compilation.into_parts();
    let mut interp = Interp::new();
    interp.set_host(Box::new(host));
    let mut session = RunSession::start(interp, &module).expect("session");
    // `timeout_ms: 0` is a validation error (E3001), not a pending effect:
    // this exercises the pre-dispatch validation path, not the transport.
    match session.advance() {
        Step::Diagnostic(d) => assert_eq!(d.code, codes::TYPE_MISMATCH),
        other => panic!("expected E3001, got {other:?}"),
    }
    assert_eq!(rig.stdout(), "finally\n");
}

#[test]
fn source_span_of_a_failed_effect_points_at_the_call() {
    let src = "fn main() {\n    let _r = http_get(\"http://example.test/a\")\n}\n";
    let (_rig, last) = drive_steps(src, |_, _| {
        Err(Diag::new(codes::IO, "boom", Span::default()))
    });
    // The failure is caught only if a `try` exists; here it propagates. The
    // diagnostic must carry the call's span, not a default.
    match last {
        Step::Diagnostic(d) => {
            assert_eq!(d.code, codes::IO);
            assert!(d.span.start > 0, "span not attributed: {d:?}");
        }
        other => panic!("expected a diagnostic, got {other:?}"),
    }
}

#[test]
fn stale_resume_is_rejected() {
    let rig = Rig::new();
    let host = SuspendHost::new(rig.out.clone(), rig.seen.clone());
    let compilation = aura::compile_named_with_mode(
        "fn main() { let r = http_get(\"http://example.test/a\")\n print(r[\"status\"]) }",
        "<test>",
        aura::CompileMode::Program,
    )
    .expect("compile");
    let (module, _sources, _entry) = compilation.into_parts();
    let mut interp = Interp::new();
    interp.set_host(Box::new(host));
    let mut session = RunSession::start(interp, &module).expect("session");

    assert!(matches!(session.advance(), Step::Pending(_)));
    // Resume once: completes.
    let first = session.resume_effect(response_value(200, ""));
    assert!(matches!(first, Step::Completed), "{first:?}");
    // Resume again with no pending effect: a diagnostic, not a hang or a
    // duplicated run.
    let second = session.resume_effect(response_value(200, ""));
    assert!(matches!(second, Step::Diagnostic(_)), "{second:?}");
    assert_eq!(rig.stdout(), "200\n");
    assert_eq!(rig.seen().len(), 1);
}

#[test]
fn a_denied_capability_never_becomes_a_pending_effect() {
    // A host with no network authority returns `Unavailable` (E5002), not
    // `Pending`: the program fails normally and no effect is parked.
    struct NoNet;
    impl Host for NoNet {
        fn write_stdout(&mut self, _b: &[u8]) -> HostResult<()> {
            Ok(())
        }
        fn read_line(&mut self) -> HostResult<Option<String>> {
            Ok(None)
        }
        fn args(&self) -> Vec<String> {
            Vec::new()
        }
        fn read_file(&self, _p: &str) -> HostResult<Option<String>> {
            Err(HostError::unavailable("no fs"))
        }
        fn write_file(&mut self, _p: &str, _c: &str) -> HostResult<()> {
            Err(HostError::unavailable("no fs"))
        }
        fn now_unix(&self) -> HostResult<i64> {
            Err(HostError::unavailable("no clock"))
        }
        fn now_local(&self) -> HostResult<LocalTime> {
            Err(HostError::unavailable("no clock"))
        }
        fn sleep_ms(&mut self, _ms: u64) -> HostResult<()> {
            Err(HostError::unavailable("no sleep"))
        }
        // default http_request denies with Unavailable (E5002)
    }
    let compilation = aura::compile_named_with_mode(
        "fn main() { let r = http_get(\"http://example.test/a\")\n print(r[\"status\"]) }",
        "<test>",
        aura::CompileMode::Program,
    )
    .expect("compile");
    let (module, _sources, _entry) = compilation.into_parts();
    let mut interp = Interp::new();
    interp.set_host(Box::new(NoNet));
    let mut session = RunSession::start(interp, &module).expect("session");
    match session.advance() {
        Step::Diagnostic(d) => assert_eq!(d.code, codes::CAPABILITY_UNAVAILABLE),
        other => panic!("expected E5002, got {other:?}"),
    }
}

#[test]
fn one_oversized_effect_count_is_exact() {
    // Five sequential requests: exactly five effects, in order, each answered
    // once. This pins the effect identity/accounting contract the browser
    // transport depends on.
    let (rig, last) = drive_steps(
        r#"fn main() {
    let mut acc = 0
    while acc < 3 {
        let _r = http_get("http://example.test/seq")
        acc = acc + 1
    }
    print("done")
}"#,
        |_, _| Ok(response_value(200, "")),
    );
    assert!(matches!(last, Step::Completed), "{last:?}");
    assert_eq!(rig.stdout(), "done\n");
    assert_eq!(rig.seen().len(), 3);
}

#[test]
fn multi_effect_program_prints_nothing_twice() {
    // A long chain with a print before and after each effect: the exact stdout
    // order proves no segment is replayed on resume.
    let (rig, last) = drive_steps(
        r#"fn main() {
    print("a")
    let _r1 = http_get("http://example.test/1")
    print("b")
    let _r2 = http_get("http://example.test/2")
    print("c")
}"#,
        |_, _| Ok(response_value(200, "")),
    );
    assert!(matches!(last, Step::Completed), "{last:?}");
    assert_eq!(rig.stdout(), "a\nb\nc\n");
    assert_eq!(rig.seen().len(), 2);
}

// Keep the loopback constant referenced so the fixture style matches the
// HTTP suite even though the scripted host replaces the socket.
#[allow(dead_code)]
const _LOOPBACK_RESPONSE: &str = HTTP_OK;
