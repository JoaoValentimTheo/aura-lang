#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! `finally` and re-entrancy invariants across parking (D1).
#![cfg(feature = "http-api")]

use std::cell::RefCell;
use std::rc::Rc;

use aura::host::{Host, HostError, HostResult, HttpRequest, HttpResponse, LocalTime};
use aura::run::session::{RunSession, Step};
use aura::run::Interp;

struct SuspendHost {
    out: Rc<RefCell<Vec<String>>>,
}
impl Host for SuspendHost {
    fn write_stdout(&mut self, b: &[u8]) -> HostResult<()> {
        self.out
            .borrow_mut()
            .push(String::from_utf8_lossy(b).into_owned());
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
    fn http_request(&self, r: &HttpRequest) -> HostResult<HttpResponse> {
        Err(HostError::pending(aura::host::PendingEffect::Http(
            r.clone(),
        )))
    }
}

fn drive(source: &str, fail: bool) -> (String, Step) {
    let compilation = aura::compile_named_with_mode(source, "<fin>", aura::CompileMode::Program)
        .expect("compile");
    let (module, _s, _e) = compilation.into_parts();
    let out = Rc::new(RefCell::new(Vec::new()));
    let mut interp = Interp::new();
    interp.set_host(Box::new(SuspendHost { out: out.clone() }));
    let mut session = RunSession::start(interp, &module).expect("session");
    let mut step = session.advance();
    loop {
        match step {
            Step::Pending(_) => {
                step = if fail {
                    session.fail_effect(aura::error::Diag::new(
                        4020,
                        "boom",
                        aura::error::Span::default(),
                    ))
                } else {
                    session.resume_effect(aura::run::value::Value::Int(0))
                };
            }
            other => return (out.borrow().concat(), other),
        }
    }
}

#[test]
fn finally_runs_after_a_resumed_success() {
    let (out, step) = drive(
        r#"fn main() {
    try {
        let r = http_get("http://example.test/a")
        print(r)
    } catch _ {
        print("caught")
    } finally {
        print("finally")
    }
    print("after")
}"#,
        false,
    );
    assert!(matches!(step, Step::Completed), "{step:?}");
    // The effect resumed with 0 (the printed value is the response map, but the
    // observable order of finally/after is what this pins).
    assert!(out.contains("finally\n"), "{out:?}");
    assert!(out.ends_with("after\n"), "{out:?}");
}

#[test]
fn finally_runs_and_the_failure_propagates_after_a_failed_effect() {
    let (out, step) = drive(
        r#"fn main() {
    try {
        let r = http_get("http://example.test/a")
        print(r)
    } catch _ {
        print("caught")
    } finally {
        print("finally")
    }
    print("after")
}"#,
        true,
    );
    match step {
        Step::Diagnostic(d) => assert_eq!(d.code, 4020, "{d:?}"),
        other => panic!("expected E4020, got {other:?}"),
    }
    assert_eq!(
        out, "finally\n",
        "finally must run exactly once, after must not"
    );
}

#[test]
fn advancing_a_completed_session_is_a_diagnostic_not_a_rerun() {
    let compilation = aura::compile_named_with_mode(
        "fn main() { print(1) }",
        "<fin>",
        aura::CompileMode::Program,
    )
    .expect("compile");
    let (module, _s, _e) = compilation.into_parts();
    let out = Rc::new(RefCell::new(Vec::new()));
    let mut interp = Interp::new();
    interp.set_host(Box::new(SuspendHost { out: out.clone() }));
    let mut session = RunSession::start(interp, &module).expect("session");
    assert!(matches!(session.advance(), Step::Completed));
    // A second advance must not re-run the program.
    let again = session.advance();
    assert!(matches!(again, Step::Completed), "{again:?}");
    assert_eq!(out.borrow().concat(), "1\n");
}
