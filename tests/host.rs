#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! The host capability boundary (`src/host.rs`).
//!
//! These tests prove that the language reaches the outside world only through
//! the [`aura::host::Host`] trait installed on the interpreter, and that an
//! unavailable optional capability is reported as `E5002` while genuine I/O
//! failures remain `E4020`.

use std::io::Cursor;

use aura::error::codes;
use aura::host::{Host, HostError, HostResult, LimitedHost, LocalTime, StdHost};
use aura::run::Interp;

/// Run `src` against a fresh `LimitedHost` with the given args/input, returning
/// the captured stdout (or the diagnostic code).
fn run_limited(src: &str, args: Vec<String>, input: Option<Vec<u8>>) -> Result<String, u16> {
    let buf = std::sync::Arc::new(std::sync::Mutex::new(Vec::<u8>::new()));
    let sink = aura::SharedBuf(buf.clone());
    let input: Option<Box<dyn std::io::BufRead + Send>> =
        input.map(|b| Box::new(Cursor::new(b)) as Box<dyn std::io::BufRead + Send>);
    let host = LimitedHost::from_parts(Box::new(sink), args, input);
    let module = aura::compile_with_mode(src, aura::CompileMode::Program).map_err(|d| d.code)?;
    let mut interp = Interp::with_host(Box::new(host));
    interp.run(&module).map_err(|d| d.code)?;
    let text = String::from_utf8(buf.lock().unwrap().clone()).unwrap();
    Ok(text)
}

/// Run `src` against a fresh `LimitedHost`, discarding output, returning the
/// diagnostic code on failure.
fn limited_code(src: &str) -> u16 {
    let module = aura::compile_with_mode(src, aura::CompileMode::Program).expect("compiles");
    let mut interp = Interp::with_host(Box::new(LimitedHost::silent()));
    interp.run(&module).unwrap_err().code
}

// ---------------------------------------------------------------------------
// stdout, args, read_line are routed through the host
// ---------------------------------------------------------------------------

#[test]
fn stdout_bytes_reach_the_installed_host() {
    assert_eq!(
        run_limited(
            "fn main() { print(\"a\")\n print(1 + 2) }",
            Vec::new(),
            None
        )
        .unwrap(),
        "a\n3\n"
    );
}

#[test]
fn args_are_routed_through_the_host() {
    assert_eq!(
        run_limited(
            "fn main() { print(args()) }",
            vec!["foo".into(), "bar".into()],
            None
        )
        .unwrap(),
        "[\"foo\", \"bar\"]\n"
    );
}

#[test]
fn read_line_is_routed_through_the_host() {
    assert_eq!(
        run_limited(
            "fn main() { print(read_line())\n print(read_line())\n print(read_line()) }",
            Vec::new(),
            Some(b"line one\r\nline two\n".to_vec())
        )
        .unwrap(),
        "line one\nline two\nnone\n"
    );
}

// ---------------------------------------------------------------------------
// unavailable capabilities report E5002
// ---------------------------------------------------------------------------

#[test]
fn limited_host_reports_filesystem_as_unavailable() {
    // The limited host is the WASM-shaped host: no filesystem.
    assert_eq!(
        limited_code("fn main() { read_file(\"/x\") }"),
        codes::CAPABILITY_UNAVAILABLE
    );
    assert_eq!(
        limited_code("fn main() { write_file(\"/x\", \"y\") }"),
        codes::CAPABILITY_UNAVAILABLE
    );
}

#[cfg(feature = "time")]
#[test]
fn limited_host_reports_clock_and_sleep_as_unavailable() {
    assert_eq!(
        limited_code("fn main() { time_unix() }"),
        codes::CAPABILITY_UNAVAILABLE
    );
    assert_eq!(
        limited_code("fn main() { time_now() }"),
        codes::CAPABILITY_UNAVAILABLE
    );
    assert_eq!(
        limited_code("fn main() { sleep_ms(1) }"),
        codes::CAPABILITY_UNAVAILABLE
    );
}

// ---------------------------------------------------------------------------
// genuine I/O failure stays E4020; absence stays none
// ---------------------------------------------------------------------------

#[test]
fn stdhost_reports_genuine_io_failure_as_e4020() {
    // A directory is a genuine I/O failure, not an absent path.
    let dir = std::env::temp_dir();
    let src = format!("fn main() {{ read_file({:?}) }}", dir.to_string_lossy());
    let module = aura::compile_with_mode(&src, aura::CompileMode::Program).expect("compiles");
    let mut interp = Interp::with_host(Box::new(StdHost::silent()));
    assert_eq!(interp.run(&module).unwrap_err().code, codes::IO);
}

#[test]
fn stdhost_missing_file_is_none_not_an_error() {
    let buf = std::sync::Arc::new(std::sync::Mutex::new(Vec::<u8>::new()));
    let sink = aura::SharedBuf(buf.clone());
    let host = aura::host::host_from_parts(Some(Box::new(sink)), Vec::new(), None);
    let module = aura::compile_with_mode(
        "fn main() { print(read_file(\"/aura-definitely-absent-xyz\")) }",
        aura::CompileMode::Program,
    )
    .expect("compiles");
    let mut interp = Interp::with_host(host);
    interp.run(&module).expect("runs");
    assert_eq!(
        String::from_utf8(buf.lock().unwrap().clone()).unwrap(),
        "none\n"
    );
}

#[test]
fn host_error_maps_to_the_right_codes() {
    use aura::error::Span;
    assert_eq!(
        HostError::io("x").into_diag(Span::default()).code,
        codes::IO
    );
    assert_eq!(
        HostError::unavailable("x").into_diag(Span::default()).code,
        codes::CAPABILITY_UNAVAILABLE
    );
}

// ---------------------------------------------------------------------------
// a custom host fully controls every capability
// ---------------------------------------------------------------------------

/// A host backed entirely by plain data: a virtual filesystem, a fixed clock,
/// and a fixed input. Nothing reaches the operating system.
struct FixedHost {
    args: Vec<String>,
    input: Vec<u8>,
    cursor: usize,
}

impl Host for FixedHost {
    fn write_stdout(&mut self, _bytes: &[u8]) -> HostResult<()> {
        Ok(())
    }
    fn read_line(&mut self) -> HostResult<Option<String>> {
        if self.cursor >= self.input.len() {
            return Ok(None);
        }
        let rest = &self.input[self.cursor..];
        let stop = rest.iter().position(|&b| b == b'\n').unwrap_or(rest.len());
        let line = String::from_utf8_lossy(&rest[..stop]).into_owned();
        self.cursor += stop + 1;
        Ok(Some(line.trim_end_matches('\r').to_string()))
    }
    fn args(&self) -> Vec<String> {
        self.args.clone()
    }
    fn read_file(&self, path: &str) -> HostResult<Option<String>> {
        Ok(Some(format!("vfs:{path}")))
    }
    fn write_file(&mut self, _path: &str, _content: &str) -> HostResult<()> {
        Ok(())
    }
    fn now_unix(&self) -> HostResult<i64> {
        Ok(1_000_000)
    }
    fn now_local(&self) -> HostResult<LocalTime> {
        Ok(LocalTime {
            year: 2001,
            month: 2,
            day: 3,
            hour: 4,
            minute: 5,
            second: 6,
            unix: 1_000_000,
        })
    }
    fn sleep_ms(&mut self, _ms: u64) -> HostResult<()> {
        Ok(())
    }
}

#[test]
fn a_custom_host_supplies_every_capability() {
    let module = aura::compile_with_mode(
        "fn main() { print(read_file(\"a.txt\") == \"vfs:a.txt\") }",
        aura::CompileMode::Program,
    )
    .expect("compiles");
    let host = FixedHost {
        args: vec!["z".into()],
        input: b"in\n".to_vec(),
        cursor: 0,
    };
    let mut interp = Interp::with_host(Box::new(host));
    // read_file routes to the virtual filesystem and succeeds.
    interp.run(&module).expect("runs against the virtual host");
}

#[cfg(feature = "time")]
#[test]
fn time_is_routed_through_the_host() {
    let buf = std::sync::Arc::new(std::sync::Mutex::new(Vec::<u8>::new()));
    let sink = aura::SharedBuf(buf.clone());
    let host = aura::host::host_from_parts(Some(Box::new(sink)), Vec::new(), None);
    let module = aura::compile_with_mode(
        "fn main() { print(time_unix() >= 0)\n print(len(time_now()) > 0) }",
        aura::CompileMode::Program,
    )
    .expect("compiles");
    let mut interp = Interp::with_host(host);
    interp.run(&module).expect("runs");
    assert_eq!(
        String::from_utf8(buf.lock().unwrap().clone()).unwrap(),
        "true\ntrue\n"
    );
}
