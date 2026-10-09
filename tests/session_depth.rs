#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Verify the call-depth contract holds across parking: a deep recursive
//! chain that parks on an HTTP effect must still hit E4011, never bypass it.
#![cfg(feature = "http-api")]

use std::cell::RefCell;
use std::rc::Rc;

use aura::error::codes;
use aura::host::{Host, HostError, HostResult, HttpRequest, HttpResponse, LocalTime};
use aura::run::session::{RunSession, Step};
use aura::run::value::Value;
use aura::run::Interp;

struct SuspendHost {
    seen: Rc<RefCell<usize>>,
}
impl Host for SuspendHost {
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
    fn http_request(&self, r: &HttpRequest) -> HostResult<HttpResponse> {
        *self.seen.borrow_mut() += 1;
        Err(HostError::pending(aura::host::PendingEffect::Http(
            r.clone(),
        )))
    }
}

#[test]
fn deep_chain_with_effects_still_hits_e4011() {
    // `recurse` parks on an HTTP effect at every level, then recurses. Parking
    // must preserve depth, so the 512-frame limit still fires (E4011) rather
    // than allowing unbounded parking.
    let source = "fn recurse(n: int) -> int {\n\
        let _r = http_get(\"http://example.test/x\")\n\
        return recurse(n + 1)\n\
    }\n\
    fn main() { recurse(0) }";
    let compilation = aura::compile_named_with_mode(source, "<depth>", aura::CompileMode::Program)
        .expect("compile");
    let (module, _sources, _entry) = compilation.into_parts();
    let seen = Rc::new(RefCell::new(0usize));
    let mut interp = Interp::new();
    interp.set_host(Box::new(SuspendHost { seen: seen.clone() }));
    let mut session = RunSession::start(interp, &module).expect("session");

    let mut parks = 0usize;
    let terminal = loop {
        match session.advance() {
            Step::Pending(_) => {
                parks += 1;
                let v = session.resume_effect(Value::Int(0));
                if !matches!(v, Step::Pending(_)) {
                    break v;
                }
                assert!(
                    parks <= 2000,
                    "parking bypassed the depth limit: {parks} parks"
                );
            }
            other => break other,
        }
    };
    match terminal {
        Step::Diagnostic(d) => assert_eq!(d.code, codes::RECURSION, "expected E4011, got {d:?}"),
        other => panic!("expected E4011, got {other:?}"),
    }
    // 512 frames: the effect is requested once per frame before the limit hits.
    assert!((500..=513).contains(&parks), "parks = {parks}");
}
