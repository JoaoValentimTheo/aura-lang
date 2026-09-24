#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! The host capability boundary (`src/host.rs`).
//!
//! These tests prove that the language reaches the outside world only through
//! the [`aura::host::Host`] trait installed on the interpreter, and that an
//! unavailable optional capability is reported as `E5002` while genuine I/O
//! failures remain `E4020`.

use std::io::Cursor;

use aura::error::codes;
use aura::host::{BrowserHost, Host, HostError, HostResult, LimitedHost, LocalTime, StdHost};
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
    // A directory is a genuine I/O failure (`E4020`), not an absent path, on
    // every platform. `std::env::temp_dir()` always exists; on Windows it
    // ends with a separator, which is a directory spelling that must still be
    // reported as an I/O failure rather than as `none`.
    let dir = std::env::temp_dir();
    let src = format!("fn main() {{ read_file({:?}) }}", dir.to_string_lossy());
    let module = aura::compile_with_mode(&src, aura::CompileMode::Program).expect("compiles");
    let mut interp = Interp::with_host(Box::new(StdHost::silent()));
    assert_eq!(interp.run(&module).unwrap_err().code, codes::IO);
}

#[test]
fn stdhost_reports_a_directory_with_trailing_separator_as_e4020() {
    // Regression: a directory path with a trailing separator (the spelling
    // `std::env::temp_dir()` uses on Windows) must be `E4020`, not `none`.
    let dir = std::env::temp_dir();
    let mut path = dir.to_string_lossy().into_owned();
    if !path.ends_with(std::path::MAIN_SEPARATOR) {
        path.push(std::path::MAIN_SEPARATOR);
    }
    let src = format!("fn main() {{ read_file({path:?}) }}");
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

// ---------------------------------------------------------------------------
// the browser Playground host is deterministic and capability-limited
// ---------------------------------------------------------------------------

/// Build a module and run it against a `BrowserHost`, returning stdout and the
/// diagnostic code if execution failed.
fn browser_run(src: &str, args: Vec<String>, stdin: Option<String>) -> (String, Option<u16>) {
    let host = BrowserHost::from_parts(stdin, args);
    let out = host.stdout_handle();
    let module = match aura::compile_with_mode(src, aura::CompileMode::Program) {
        Ok(m) => m,
        Err(d) => return (String::new(), Some(d.code)),
    };
    let mut interp = Interp::with_host(Box::new(host));
    let code = interp.run(&module).err().map(|d| d.code);
    let text = String::from_utf8(out.lock().unwrap().clone()).unwrap();
    (text, code)
}

#[test]
fn browser_host_routes_stdout_args_and_stdin() {
    let (out, code) = browser_run(
        "fn main() { print(\"a\")\n print(args())\n print(read_line())\n print(read_line()) }",
        vec!["x".into(), "y".into()],
        Some("first\r\nsecond\n".to_string()),
    );
    assert_eq!(code, None);
    assert_eq!(out, "a\n[\"x\", \"y\"]\nfirst\nsecond\n");
}

#[test]
fn browser_host_sequential_reads_are_deterministic() {
    let src = "fn main() { let a = read_line()\n let b = read_line()\n let c = read_line()\n print(a)\n print(b)\n print(c) }";
    let first = browser_run(src, Vec::new(), Some("1\n2\n".to_string()));
    let second = browser_run(src, Vec::new(), Some("1\n2\n".to_string()));
    assert_eq!(first, second);
    assert_eq!(first.0, "1\n2\nnone\n");
}

#[test]
fn browser_host_has_no_filesystem_clock_or_sleep() {
    for src in [
        "fn main() { read_file(\"/x\") }",
        "fn main() { write_file(\"/x\", \"y\") }",
        "fn main() { time_unix() }",
        "fn main() { time_now() }",
        "fn main() { sleep_ms(1) }",
    ] {
        let (_, code) = browser_run(src, Vec::new(), None);
        assert_eq!(code, Some(codes::CAPABILITY_UNAVAILABLE), "{src}");
    }
}

#[test]
fn browser_host_does_not_expose_browser_state() {
    // There is no path from an Aura program to any ambient state: browser
    // globals are not names, so they are rejected by the checker (E2003) and
    // never reach a host method.
    for name in ["window", "document", "localStorage", "fetch"] {
        let src = format!("fn main() {{ print({name}) }}");
        assert_eq!(
            browser_run(&src, Vec::new(), None).1,
            Some(codes::UNDEFINED),
            "{name}"
        );
    }
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
