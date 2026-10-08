#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Aura 0.3 Keystone HTTP capability tests (feature `http`).
//!
//! Authority: `docs/engineering/HTTP_ARCHITECTURE.md`. The tests use a real
//! loopback server built from `std::net` (no extra dependency) so the
//! capability, the transport, and the typed decode are exercised end to end.
#![cfg(feature = "http")]

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use aura::error::codes;
use aura::host::{Host, HostError, HostResult, HttpRequest};
use aura::run::Interp;

/// A tiny single-purpose HTTP/1.1 server that answers every request with a
/// canned response. It is sufficient to exercise the transport without any
/// test dependency.
struct TestServer {
    port: u16,
    stop: Arc<AtomicBool>,
}

impl TestServer {
    fn start(response: &'static str) -> TestServer {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
        let port = listener.local_addr().expect("addr").port();
        let stop = Arc::new(AtomicBool::new(false));
        let stop_clone = Arc::clone(&stop);
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                if stop_clone.load(Ordering::SeqCst) {
                    break;
                }
                let Ok(mut stream) = stream else { break };
                handle(&mut stream, response);
            }
        });
        TestServer { port, stop }
    }

    fn url(&self, path: &str) -> String {
        format!("http://127.0.0.1:{}{path}", self.port)
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        // Wake the accept loop.
        let _ = TcpStream::connect(("127.0.0.1", self.port));
    }
}

fn handle(stream: &mut TcpStream, response: &'static str) {
    let mut buf = [0u8; 4096];
    let _ = stream.read(&mut buf);
    let _ = stream.write_all(response.as_bytes());
}

/// A shared buffer so the test can read what a program printed.
#[derive(Clone)]
struct Sink(Arc<std::sync::Mutex<Vec<u8>>>);

impl Write for Sink {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Run an Aura program with the native host (which provides HTTP when the
/// feature is compiled in), capturing what it printed.
fn run(src: &str) -> Result<String, u16> {
    let buf = Arc::new(std::sync::Mutex::new(Vec::new()));
    let host = aura::host::StdHost::from_parts(Box::new(Sink(Arc::clone(&buf))), Vec::new(), None);
    let mut it = Interp::with_host(Box::new(host));
    aura::stdlib::install(&mut it);
    let module = aura::parse::parse(src).expect("parses");
    aura::check::Checker::module(&module).map_err(|d| d.code)?;
    it.run(&module).map_err(|d| d.code)?;
    let bytes = buf.lock().unwrap().clone();
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

/// A host that deliberately has no network capability, to prove denial.
struct NoNetworkHost;

impl Host for NoNetworkHost {
    fn write_stdout(&mut self, _bytes: &[u8]) -> HostResult<()> {
        Ok(())
    }
    fn read_line(&mut self) -> HostResult<Option<String>> {
        Ok(None)
    }
    fn args(&self) -> Vec<String> {
        Vec::new()
    }
    fn read_file(&self, _path: &str) -> HostResult<Option<String>> {
        Ok(None)
    }
    fn write_file(&mut self, _path: &str, _content: &str) -> HostResult<()> {
        Ok(())
    }
    fn now_unix(&self) -> HostResult<i64> {
        Err(HostError::unavailable("no clock"))
    }
    fn now_local(&self) -> HostResult<aura::host::LocalTime> {
        Err(HostError::unavailable("no clock"))
    }
    fn sleep_ms(&mut self, _ms: u64) -> HostResult<()> {
        Err(HostError::unavailable("no sleep"))
    }
    // `http_request` is not overridden: the trait default denies it.
}

/// A 302 that points at another path on the same server: the layer must
/// return it as an ordinary response rather than following it, so a redirect
/// cannot expand authority implicitly.
const REDIRECT_RESPONSE: &str = "HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:1/elsewhere\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";

const OK_RESPONSE: &str = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nX-Test: yes\r\nContent-Length: 26\r\nConnection: close\r\n\r\n{\"name\": \"bob\", \"age\": 30}";

#[test]
fn request_returns_status_headers_and_body() {
    let server = TestServer::start(OK_RESPONSE);
    let src = format!(
        "fn main() {{\n let r = http_get(\"{}\")\n print(r[\"status\"])\n print(r[\"headers\"][\"x-test\"])\n print(r[\"body\"]) }}",
        server.url("/user")
    );
    let out = run(&src).expect("request succeeds");
    assert_eq!(out, "200\nyes\n{\"name\": \"bob\", \"age\": 30}\n");
}

#[test]
fn response_composes_with_typed_json_decode() {
    let server = TestServer::start(OK_RESPONSE);
    let src = format!(
        "struct User {{ name: string, age: int }}\nfn main() {{\n let r = http_get(\"{}\")\n let u = json_decode_as(r[\"body\"], User)\n print(u.name)\n print(u.age) }}",
        server.url("/user")
    );
    let out = run(&src).expect("decode succeeds");
    assert_eq!(out, "bob\n30\n");
}

#[test]
fn cap_denied_host_reports_capability_unavailable() {
    let mut it = Interp::with_host(Box::new(NoNetworkHost));
    aura::stdlib::install(&mut it);
    let module =
        aura::parse::parse("fn main() { let _r = http_get(\"http://127.0.0.1:1/x\")\n print(_r) }")
            .expect("parses");
    aura::check::Checker::module(&module).expect("checks");
    let err = it.run(&module).expect_err("network is denied");
    assert_eq!(err.code, codes::CAPABILITY_UNAVAILABLE);
}

#[test]
fn connection_failure_is_io_not_a_user_exception() {
    // Port 1 is not listening; the failure is E4020 (I/O), never a catchable
    // throw and never E5002.
    let err = run("fn main() { let _r = http_get(\"http://127.0.0.1:1/x\") }")
        .expect_err("connection refused");
    assert_eq!(err, codes::IO);
}

#[test]
fn unsupported_method_is_rejected_before_the_capability() {
    let err = run("fn main() { let _r = http_request(\"TRACE\", \"http://127.0.0.1:1/\") }")
        .expect_err("method rejected");
    assert_eq!(err, codes::TYPE_MISMATCH);
}

#[test]
fn oversized_body_is_a_resource_error() {
    // A body larger than the documented limit is E4020, not partial success.
    let big = 9 * 1024 * 1024;
    let response = Box::leak(
        format!("HTTP/1.1 200 OK\r\nContent-Length: {big}\r\nConnection: close\r\n\r\n")
            .into_boxed_str(),
    );
    let server = TestServer::start(response);
    let src = format!(
        "fn main() {{ let r = http_get(\"{}\")\n print(len(r[\"body\"])) }}",
        server.url("/huge")
    );
    let err = run(&src).expect_err("body over the limit");
    assert_eq!(err, codes::IO);
}

#[test]
fn default_http_request_denies_network() {
    // The trait default is the denial: any host that does not implement HTTP
    // has no network authority, with no language-level flag involved.
    let host = NoNetworkHost;
    let err = host
        .http_request(&HttpRequest {
            method: "GET".into(),
            url: "http://127.0.0.1:1/".into(),
            headers: Vec::new(),
            body: None,
            timeout_ms: 1000,
        })
        .expect_err("denied");
    assert!(matches!(err, HostError::Unavailable(_)));
}

#[test]
fn multibyte_character_across_the_read_boundary_is_intact() {
    // The body reader fills an 8 KiB buffer; a multi-byte character must not
    // be corrupted by decoding each chunk independently (review finding 7).
    let prefix = "a".repeat(8191);
    let body = format!("{prefix}é");
    let response = Box::leak(
        format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        )
        .into_boxed_str(),
    );
    let server = TestServer::start(response);
    let src = format!(
        "fn main() {{ let r = http_get(\"{}\")\n print(len(r[\"body\"]))\n print(r[\"body\"][8191]) }}",
        server.url("/utf8")
    );
    let out = run(&src).expect("request");
    // 8192 characters, and the character at index 8191 is `é`, not U+FFFD.
    assert_eq!(
        out,
        "8192
é
"
    );
}

#[test]
fn program_exception_still_flies_through() {
    // HTTP is ordinary capability work: an Aura `throw` is unaffected.
    let server = TestServer::start(OK_RESPONSE);
    let src = format!(
        "fn main() {{ try {{ let _r = http_get(\"{}\")\n throw \"my error\" }} catch e {{ print(e) }} }}",
        server.url("/x")
    );
    let out = run(&src).expect("catch handles the throw");
    assert_eq!(out, "my error\n");
}

#[test]
fn a_redirect_is_returned_not_followed() {
    // Following a redirect would send a request the program never asked for,
    // to a destination it did not choose. The 3xx is the response.
    let server = TestServer::start(REDIRECT_RESPONSE);
    let src = format!(
        "fn main() {{ let r = http_get(\"{}\")\n print(r[\"status\"]) }}",
        server.url("/a")
    );
    let out = run(&src).expect("3xx is a response");
    assert_eq!(out, "302\n");
}

#[test]
fn response_shape_is_a_map_with_documented_keys() {
    let server = TestServer::start(OK_RESPONSE);
    let src = format!(
        "fn main() {{ let r = http_get(\"{}\")\n print(keys(r)) }}",
        server.url("/x")
    );
    let out = run(&src).expect("keys");
    assert_eq!(out, "[\"body\", \"headers\", \"status\"]\n");
}
