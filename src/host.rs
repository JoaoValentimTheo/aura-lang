//! The host capability boundary.
//!
//! Every language-visible interaction with the outside world goes through
//! [`Host`], which the interpreter owns. The standard library never calls the
//! operating system directly. Native execution installs [`StdHost`] (real
//! filesystem, clock, and sleep); a WebAssembly runtime installs a
//! capability-limited host whose unavailable capabilities are reported as
//! `E5002`. The browser Playground installs [`BrowserHost`], a deterministic,
//! self-contained host behind this same contract.
//!
//! The boundary carries only plain data (strings, integers, bytes). It never
//! exposes JavaScript, the DOM, the network, process execution, environment
//! variables, or native handles.

use crate::error::{codes, Diag, Span};

/// A host operation failure.
#[derive(Debug, Clone)]
pub enum HostError {
    /// A genuine I/O failure (`E4020`).
    Io(String),
    /// The capability is not available in this host (`E5002`).
    Unavailable(String),
    /// The operation cannot complete synchronously and must be resumed by the
    /// embedder (0.3.2 development). The payload names the pending effect so a
    /// resumable substrate (the browser Worker) can perform it and continue the
    /// program without restarting it. A host that cannot suspend treats this as
    /// unavailable. It is **not** a language error and never surfaces to Aura.
    Pending(PendingEffect),
}

/// A host effect that a non-blocking substrate must perform out of band.
///
/// This is transport data, not a diagnostic: it carries exactly what the
/// embedder needs to perform the effect and resume the program (for HTTP, the
/// fully validated request). The language observable is the eventual value or
/// `E4020`/`E5002`, never the pending state itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PendingEffect {
    /// An HTTP request the embedder must perform and feed back.
    Http(HttpRequest),
}

impl HostError {
    /// Build an I/O failure.
    #[must_use]
    pub fn io(msg: impl Into<String>) -> HostError {
        HostError::Io(msg.into())
    }

    /// Build an unavailable-capability failure.
    #[must_use]
    pub fn unavailable(msg: impl Into<String>) -> HostError {
        HostError::Unavailable(msg.into())
    }

    /// Build a pending-effect signal.
    #[must_use]
    pub fn pending(effect: PendingEffect) -> HostError {
        HostError::Pending(effect)
    }

    /// Convert into a language diagnostic with the given span.
    ///
    /// A [`HostError::Pending`] is a programming error at this layer: every
    /// caller that can receive it must first resolve it by suspending. It maps
    /// to `E5002` (the capability is not available *synchronously* here) so a
    /// non-resumable caller that somehow reaches it denies access rather than
    /// hanging or corrupting execution.
    #[must_use]
    pub fn into_diag(self, span: Span) -> Diag {
        match self {
            HostError::Io(m) => Diag::new(codes::IO, m, span),
            HostError::Unavailable(m) => Diag::new(codes::CAPABILITY_UNAVAILABLE, m, span),
            HostError::Pending(_) => Diag::new(
                codes::CAPABILITY_UNAVAILABLE,
                "the requested capability requires a resumable host",
                span,
            ),
        }
    }
}

/// The result of a host operation.
pub type HostResult<T> = std::result::Result<T, HostError>;

/// A local wall-clock reading, as plain data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalTime {
    /// Calendar year.
    pub year: i32,
    /// Month, 1-12.
    pub month: u32,
    /// Day of month, 1-31.
    pub day: u32,
    /// Hour, 0-23.
    pub hour: u32,
    /// Minute, 0-59.
    pub minute: u32,
    /// Second, 0-59.
    pub second: u32,
    /// Seconds since the Unix epoch.
    pub unix: i64,
}

/// A typed HTTP request issued through the `Host` capability (Keystone §21).
///
/// The evaluator builds this from the Aura call; the host decides whether it
/// has network authority at all. No ambient authority is attached: headers
/// are exactly what the program passed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpRequest {
    /// The method, already validated to the documented set (`GET`, `POST`,
    /// `PUT`, `DELETE`, `PATCH`, `HEAD`).
    pub method: String,
    /// The absolute URL.
    pub url: String,
    /// Request headers, in the order supplied.
    pub headers: Vec<(String, String)>,
    /// Request body, when the method carries one.
    pub body: Option<String>,
    /// Timeout in milliseconds, clamped to [`MAX_HTTP_TIMEOUT_MS`].
    pub timeout_ms: u64,
}

/// A typed HTTP response returned through the `Host` capability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpResponse {
    /// The numeric status code.
    pub status: u16,
    /// Response headers, in the order received, preserving every occurrence
    /// (so repeated headers such as `Set-Cookie` are never silently merged).
    pub headers: Vec<(String, String)>,
    /// The exact response body bytes. Binary-safe: the bytes are never
    /// lossily replaced.
    pub body_bytes: Vec<u8>,
    /// The response body decoded as UTF-8 where it is valid, else a lossy
    /// view. This is a *convenience view* for text programs; `body_bytes`
    /// remains authoritative.
    pub body: String,
}

/// The capability boundary between Aura and its execution environment.
///
/// A host provides standard output, standard input, and program arguments
/// (the capabilities every substrate can offer), plus optional filesystem,
/// clock, and sleep capabilities. An optional capability that a host does not
/// implement returns [`HostError::Unavailable`], which surfaces to Aura as
/// `E5002`; a genuine failure returns [`HostError::Io`] (`E4020`).
pub trait Host {
    /// Write bytes to standard output.
    ///
    /// # Errors
    /// Returns a host failure if the output cannot be written.
    fn write_stdout(&mut self, bytes: &[u8]) -> HostResult<()>;

    /// Read one line from standard input, excluding the line terminator.
    /// `None` means end of input.
    ///
    /// # Errors
    /// Returns a host failure if input cannot be read.
    fn read_line(&mut self) -> HostResult<Option<String>>;

    /// The program arguments exposed to `args()`.
    fn args(&self) -> Vec<String>;

    /// Read a whole file as UTF-8 text. `None` means the path does not exist.
    ///
    /// # Errors
    /// Returns a host failure for any I/O failure other than absence.
    fn read_file(&self, path: &str) -> HostResult<Option<String>>;

    /// Write text to a file, creating or truncating it.
    ///
    /// # Errors
    /// Returns a host failure if the write fails.
    fn write_file(&mut self, path: &str, content: &str) -> HostResult<()>;

    /// The current time as seconds since the Unix epoch.
    ///
    /// # Errors
    /// Returns [`HostError::Unavailable`] if the host has no clock.
    fn now_unix(&self) -> HostResult<i64>;

    /// The current local time.
    ///
    /// # Errors
    /// Returns [`HostError::Unavailable`] if the host has no clock.
    fn now_local(&self) -> HostResult<LocalTime>;

    /// Sleep for `ms` milliseconds.
    ///
    /// # Errors
    /// Returns [`HostError::Unavailable`] if the host cannot sleep.
    fn sleep_ms(&mut self, ms: u64) -> HostResult<()>;

    /// Whether the host is requesting cancellation. Advisory; the primary
    /// hard-cancel mechanism is external termination of the execution
    /// instance. Implementations default to `false`.
    fn should_cancel(&self) -> bool {
        false
    }

    /// Perform one HTTP request (Keystone §21).
    ///
    /// The default implementation denies the capability with
    /// [`HostError::Unavailable`] (`E5002`), so a host has network authority
    /// only when it deliberately provides it. A genuine transport failure is
    /// [`HostError::Io`] (`E4020`); a response body over
    /// [`MAX_HTTP_BODY_BYTES`] is also an I/O failure, never unbounded
    /// memory. Network authority never flows through any other API.
    ///
    /// # Errors
    /// Returns `Unavailable` when the host has no network capability, and
    /// `Io` for a transport, timeout, or resource failure.
    fn http_request(&self, _request: &HttpRequest) -> HostResult<HttpResponse> {
        Err(HostError::unavailable(
            "this host has no network capability (HTTP is unavailable)",
        ))
    }
}

/// Maximum response body materialized by the built-in HTTP capability.
///
/// Resource protection: a peer cannot make a program allocate without bound.
/// A body over this limit is a deterministic error, not a partial success.
pub const MAX_HTTP_BODY_BYTES: usize = 8 * 1024 * 1024;

/// Maximum total time one HTTP request may take.
pub const MAX_HTTP_TIMEOUT_MS: u64 = 30_000;

/// The HTTP methods reachable from Aura, exactly.
pub const HTTP_METHODS: [&str; 6] = ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD"];

/// The default host for the current substrate.
///
/// Native: process stdout, no input, no arguments, with real filesystem,
/// clock, and sleep. WebAssembly: a capability-limited host that discards
/// output and has no input, arguments, filesystem, clock, or sleep.
#[must_use]
pub fn default_host() -> Box<dyn Host> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        Box::new(StdHost::std())
    }
    #[cfg(target_arch = "wasm32")]
    {
        Box::new(LimitedHost::silent())
    }
}

/// The default host for the current substrate, configured with arguments and
/// an optional standard-input source.
#[must_use]
pub fn default_host_from_parts(
    args: Vec<String>,
    input: Option<Box<dyn std::io::BufRead + Send>>,
) -> Box<dyn Host> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let input: Option<Box<dyn std::io::BufRead + Send>> = input;
        Box::new(StdHost::from_parts(
            Box::new(std::io::stdout()),
            args,
            input,
        ))
    }
    #[cfg(target_arch = "wasm32")]
    {
        let input: Option<Box<dyn std::io::BufRead + Send>> = input;
        Box::new(LimitedHost::from_parts(
            Box::new(std::io::sink()),
            args,
            input,
        ))
    }
}

/// A host that discards output.
///
/// Native: a [`StdHost`] writing to a sink, still with the real filesystem,
/// clock, and sleep (it only silences output). WebAssembly: a
/// capability-limited host.
#[must_use]
pub fn silent_host() -> Box<dyn Host> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        Box::new(StdHost::silent())
    }
    #[cfg(target_arch = "wasm32")]
    {
        Box::new(LimitedHost::silent())
    }
}

/// Build a native or capability-limited host from an optional output sink,
/// arguments, and an optional standard-input source.
///
/// When no output sink is supplied the substrate default is used (process
/// stdout on native, a sink on WebAssembly).
#[must_use]
pub fn host_from_parts(
    output: Option<Box<dyn std::io::Write + Send>>,
    args: Vec<String>,
    input: Option<Box<dyn std::io::BufRead + Send>>,
) -> Box<dyn Host> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let out: Box<dyn std::io::Write> = match output {
            Some(w) => w,
            None => Box::new(std::io::stdout()),
        };
        let input: Option<Box<dyn std::io::BufRead + Send>> = input;
        Box::new(StdHost::from_parts(out, args, input))
    }
    #[cfg(target_arch = "wasm32")]
    {
        let out: Box<dyn std::io::Write> = match output {
            Some(w) => w,
            None => Box::new(std::io::sink()),
        };
        let input: Option<Box<dyn std::io::BufRead + Send>> = input;
        Box::new(LimitedHost::from_parts(out, args, input))
    }
}

/// Read one normalized line from a buffered reader: a trailing `\n` is
/// removed and a `\r` immediately before it is removed too. `None` is EOF.
fn read_normalized_line(reader: &mut (dyn std::io::BufRead + Send)) -> HostResult<Option<String>> {
    let mut line = String::new();
    let n = reader
        .read_line(&mut line)
        .map_err(|e| HostError::io(format!("standard input read failed: {e}")))?;
    if n == 0 {
        return Ok(None);
    }
    if line.ends_with('\n') {
        line.pop();
        if line.ends_with('\r') {
            line.pop();
        }
    }
    Ok(Some(line))
}

/// The native host: the real filesystem, clock, and sleep.
///
/// This type is compiled only for non-WebAssembly targets, so a browser
/// runtime cannot reach the operating system through it.
#[cfg(not(target_arch = "wasm32"))]
pub struct StdHost {
    out: Box<dyn std::io::Write>,
    input: Option<Box<dyn std::io::BufRead + Send>>,
    args: Vec<String>,
}

#[cfg(not(target_arch = "wasm32"))]
impl StdHost {
    /// A host writing to the process standard output, with no input source
    /// and no arguments.
    #[must_use]
    pub fn std() -> StdHost {
        StdHost {
            out: Box::new(std::io::stdout()),
            input: None,
            args: Vec::new(),
        }
    }

    /// A host that discards standard output.
    #[must_use]
    pub fn silent() -> StdHost {
        StdHost {
            out: Box::new(std::io::sink()),
            input: None,
            args: Vec::new(),
        }
    }

    /// A host from explicit parts.
    #[must_use]
    pub fn from_parts(
        out: Box<dyn std::io::Write>,
        args: Vec<String>,
        input: Option<Box<dyn std::io::BufRead + Send>>,
    ) -> StdHost {
        StdHost { out, input, args }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Host for StdHost {
    fn write_stdout(&mut self, bytes: &[u8]) -> HostResult<()> {
        use std::io::Write;
        self.out
            .write_all(bytes)
            .map_err(|e| HostError::io(format!("standard output write failed: {e}")))
    }

    fn read_line(&mut self) -> HostResult<Option<String>> {
        match self.input.as_mut() {
            Some(reader) => read_normalized_line(reader.as_mut()),
            None => Ok(None),
        }
    }

    fn args(&self) -> Vec<String> {
        self.args.clone()
    }

    fn read_file(&self, path: &str) -> HostResult<Option<String>> {
        // A directory is a genuine I/O failure (`E4020`), never absence, on
        // every platform. Windows rejects `read_to_string` on some directory
        // spellings (for example a path with a trailing separator, as
        // `std::env::temp_dir()` returns) with `NotFound`, which would
        // otherwise be misreported as `none`. Detect the directory explicitly
        // so the documented contract holds identically everywhere.
        if std::path::Path::new(path).is_dir() {
            return Err(HostError::io(format!(
                "cannot read `{path}`: it is a directory"
            )));
        }
        match std::fs::read_to_string(path) {
            Ok(text) => Ok(Some(text)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(HostError::io(format!("cannot read `{path}`: {e}"))),
        }
    }

    fn write_file(&mut self, path: &str, content: &str) -> HostResult<()> {
        std::fs::write(path, content.as_bytes())
            .map_err(|e| HostError::io(format!("cannot write `{path}`: {e}")))
    }

    fn now_unix(&self) -> HostResult<i64> {
        #[cfg(feature = "time")]
        {
            Ok(chrono::Utc::now().timestamp())
        }
        #[cfg(not(feature = "time"))]
        {
            Err(HostError::unavailable(
                "the clock capability is not available in this host",
            ))
        }
    }

    fn now_local(&self) -> HostResult<LocalTime> {
        #[cfg(feature = "time")]
        {
            use chrono::{Datelike, Timelike};
            let now = chrono::Local::now();
            Ok(LocalTime {
                year: now.year(),
                month: now.month(),
                day: now.day(),
                hour: now.hour(),
                minute: now.minute(),
                second: now.second(),
                unix: now.timestamp(),
            })
        }
        #[cfg(not(feature = "time"))]
        {
            Err(HostError::unavailable(
                "the clock capability is not available in this host",
            ))
        }
    }

    fn sleep_ms(&mut self, ms: u64) -> HostResult<()> {
        std::thread::sleep(std::time::Duration::from_millis(clamped_sleep_ms(ms)));
        Ok(())
    }

    /// The native host provides the HTTP capability when the `http` feature
    /// is compiled in (Keystone §21). Without the feature the trait default
    /// denies it, so a default build has no network surface at all.
    #[cfg(feature = "http")]
    fn http_request(&self, request: &HttpRequest) -> HostResult<HttpResponse> {
        http::perform(request)
    }
}

/// The native HTTP provider (feature `http`).
///
/// This is the only place a socket is opened, and it is reachable only
/// through [`Host::http_request`]. See
/// `docs/engineering/HTTP_ARCHITECTURE.md` for the security review.
#[cfg(feature = "http")]
pub mod http {
    use super::{HostError, HostResult, HttpRequest, HttpResponse, MAX_HTTP_BODY_BYTES};
    use std::io::Read;

    /// Perform one request with a bounded body and a hard timeout.
    ///
    /// A non-2xx response is an ordinary response (the program inspects
    /// `status`); only a genuine transport/timeout/resource failure is
    /// [`HostError::Io`].
    pub(super) fn perform(request: &HttpRequest) -> HostResult<HttpResponse> {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(std::time::Duration::from_millis(request.timeout_ms)))
            // No redirect following: a 3xx is returned to Aura as an ordinary
            // response, so a redirect cannot expand authority implicitly.
            .max_redirects(0)
            // A 4xx/5xx is a response, not an error: the program decides.
            .http_status_as_error(false)
            .build()
            .into();
        let method = request.method.to_ascii_uppercase();
        // Method/body policy (`HTTP_ARCHITECTURE.md`): a body supplied for a
        // method that by contract carries none is a caller error, reported
        // rather than silently dropped. GET/HEAD are bodyless; POST/PUT/PATCH
        // always send (a bodyless one sends empty); DELETE sends a body only
        // when the program supplied one.
        if matches!(method.as_str(), "GET" | "HEAD") && request.body.is_some() {
            return Err(HostError::io(format!(
                "`{method}` requests do not carry a body; omit the `body` option"
            )));
        }
        let body = request.body.as_deref();

        // Apply the caller's headers to every builder. `ureq`'s typed builders
        // take headers one at a time; a repeated name is appended in order, so
        // the wire request carries exactly what the program passed.
        let apply = |mut req: ureq::RequestBuilder<ureq::typestate::WithoutBody>| {
            for (name, value) in &request.headers {
                req = req.header(name, value);
            }
            req
        };
        let apply_body = |mut req: ureq::RequestBuilder<ureq::typestate::WithBody>| {
            for (name, value) in &request.headers {
                req = req.header(name, value);
            }
            req
        };

        // A body-carrying method sends its body (or an empty one when the
        // program passed none); the others are bodyless builders sent with
        // `call`.
        let response = match (method.as_str(), body) {
            ("GET", _) => apply(agent.get(&request.url)).call(),
            ("HEAD", _) => apply(agent.head(&request.url)).call(),
            // DELETE is method-body-noncompliant per spec; `force_send_body`
            // is ureq's documented escape hatch when the program supplies one.
            ("DELETE", Some(b)) => apply_body(agent.delete(&request.url).force_send_body()).send(b),
            ("DELETE", None) => apply(agent.delete(&request.url)).call(),
            ("POST", Some(b)) => apply_body(agent.post(&request.url)).send(b),
            ("POST", None) => apply_body(agent.post(&request.url)).send_empty(),
            ("PUT", Some(b)) => apply_body(agent.put(&request.url)).send(b),
            ("PUT", None) => apply_body(agent.put(&request.url)).send_empty(),
            ("PATCH", Some(b)) => apply_body(agent.patch(&request.url)).send(b),
            ("PATCH", None) => apply_body(agent.patch(&request.url)).send_empty(),
            (other, _) => {
                return Err(HostError::io(format!("unsupported HTTP method `{other}`")));
            }
        }
        .map_err(|e| HostError::io(format!("HTTP request failed: {e}")))?;
        let status = response.status().as_u16();
        let headers: Vec<(String, String)> = response
            .headers()
            .iter()
            .map(|(name, value)| {
                (
                    name.as_str().to_string(),
                    value.to_str().unwrap_or("").to_string(),
                )
            })
            .collect();
        // A body is decoded as UTF-8 only after the whole (bounded) byte
        // sequence is read: decoding per read chunk would split a multi-byte
        // character that straddles a boundary and corrupt it into U+FFFD.
        let mut bytes = Vec::new();
        let mut reader = response.into_body().into_reader();
        let mut buf = [0u8; 8192];
        loop {
            let n = reader
                .read(&mut buf)
                .map_err(|e| HostError::io(format!("HTTP body read failed: {e}")))?;
            if n == 0 {
                break;
            }
            if bytes.len() + n > MAX_HTTP_BODY_BYTES {
                return Err(HostError::io(format!(
                    "HTTP response body exceeds the {MAX_HTTP_BODY_BYTES} byte limit"
                )));
            }
            bytes.extend_from_slice(&buf[..n]);
        }
        // The bytes are authoritative and never replaced. `body` is a text
        // view: exact when the payload is valid UTF-8, a lossy view otherwise.
        // A binary consumer reads `body_bytes`.
        let body = String::from_utf8_lossy(&bytes).into_owned();
        Ok(HttpResponse {
            status,
            headers,
            body_bytes: bytes,
            body,
        })
    }
}

/// Maximum sleep a single `sleep_ms` call may block for.
///
/// The native host is the only host that provides sleep (the WebAssembly host
/// reports `E5002`), and a script must not be able to pin a process
/// indefinitely with one call. This is a host policy bound, not a language
/// semantic one: it caps the *duration of one call* and never errors.
pub const MAX_SLEEP_MS: u64 = 60_000;

/// Clamp a requested sleep to [`MAX_SLEEP_MS`].
#[must_use]
pub fn clamped_sleep_ms(ms: u64) -> u64 {
    ms.min(MAX_SLEEP_MS)
}

/// A capability-limited host.
///
/// It provides standard output, standard input, and arguments (from plain
/// data supplied by the embedder) but has no filesystem, clock, or sleep
/// capability. Every optional capability returns [`HostError::Unavailable`],
/// which Aura reports as `E5002`. This is the host a WebAssembly runtime
/// installs, and it compiles on every target with no operating-system
/// authority.
pub struct LimitedHost {
    out: Box<dyn std::io::Write>,
    input: Option<Box<dyn std::io::BufRead + Send>>,
    args: Vec<String>,
}

impl LimitedHost {
    /// A host that discards output, with no input and no arguments.
    #[must_use]
    pub fn silent() -> LimitedHost {
        LimitedHost {
            out: Box::new(std::io::sink()),
            input: None,
            args: Vec::new(),
        }
    }

    /// A host from explicit parts.
    #[must_use]
    pub fn from_parts(
        out: Box<dyn std::io::Write>,
        args: Vec<String>,
        input: Option<Box<dyn std::io::BufRead + Send>>,
    ) -> LimitedHost {
        LimitedHost { out, input, args }
    }
}

impl Host for LimitedHost {
    fn write_stdout(&mut self, bytes: &[u8]) -> HostResult<()> {
        use std::io::Write;
        self.out
            .write_all(bytes)
            .map_err(|e| HostError::io(format!("standard output write failed: {e}")))
    }

    fn read_line(&mut self) -> HostResult<Option<String>> {
        match self.input.as_mut() {
            Some(reader) => read_normalized_line(reader.as_mut()),
            None => Ok(None),
        }
    }

    fn args(&self) -> Vec<String> {
        self.args.clone()
    }

    fn read_file(&self, _path: &str) -> HostResult<Option<String>> {
        Err(HostError::unavailable(
            "the filesystem capability is not available in this host",
        ))
    }

    fn write_file(&mut self, _path: &str, _content: &str) -> HostResult<()> {
        Err(HostError::unavailable(
            "the filesystem capability is not available in this host",
        ))
    }

    fn now_unix(&self) -> HostResult<i64> {
        Err(HostError::unavailable(
            "the clock capability is not available in this host",
        ))
    }

    fn now_local(&self) -> HostResult<LocalTime> {
        Err(HostError::unavailable(
            "the clock capability is not available in this host",
        ))
    }

    fn sleep_ms(&mut self, _ms: u64) -> HostResult<()> {
        // Deliberately unavailable: a synchronous WebAssembly instance must
        // not busy-wait or block the browser.
        Err(HostError::unavailable(
            "the sleep capability is not available in this host",
        ))
    }
}

/// A cloneable handle to a [`BrowserHost`]'s standard-output buffer.
///
/// The interpreter takes ownership of the host, so the embedder keeps this
/// handle to read the collected output after execution. It is explicit
/// shared state local to one execution, never global browser state.
pub type BrowserStdout = std::sync::Arc<std::sync::Mutex<Vec<u8>>>;

/// The retained state of a bounded output sink (0.3.2 development).
///
/// A sink is the *only* place output bytes come to rest. It separates the
/// bytes retained for display from the bytes the program actually wrote, so
/// the embedder can report "N of M bytes" honestly instead of silently
/// discarding data or turning a policy bound into a program failure.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct OutputStats {
    /// Total bytes the program wrote (retained + omitted).
    pub written: usize,
    /// Bytes retained in the preview (never more than the preview bound).
    pub retained: usize,
    /// Bytes deliberately not retained because the preview was full.
    pub omitted: usize,
}

impl OutputStats {
    /// Whether the preview dropped bytes the program wrote.
    #[must_use]
    pub fn truncated(&self) -> bool {
        self.omitted > 0
    }
}

/// How a bounded output sink treats bytes beyond its retention bound.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Overflow {
    /// Keep executing: count the excess as omitted and continue. This is the
    /// preview mode — a full preview is not an error.
    Omit,
    /// Stop retaining further bytes and flag the capture as capped. The run
    /// itself continues; only the *capture* is bounded. This is the
    /// complete-output mode's honest capacity limit.
    Cap,
}

/// A bounded-memory standard-output sink (0.3.2 development).
///
/// The sink is shared between the host (which writes) and the embedder
/// (which reads after, or during, execution). It never grows past
/// `retention_limit` bytes, so a program that prints unboundedly cannot
/// exhaust memory; the bytes it cannot retain are counted, never silently
/// lost, and never turned into a program failure.
#[derive(Debug, Clone)]
pub struct OutputSink {
    inner: std::sync::Arc<std::sync::Mutex<OutputSinkState>>,
}

#[derive(Debug)]
struct OutputSinkState {
    retained: Vec<u8>,
    written: usize,
    omitted: usize,
    retention_limit: usize,
    overflow: Overflow,
    capped: bool,
}

impl OutputSink {
    /// A preview sink retaining at most `limit` bytes and continuing past it.
    #[must_use]
    pub fn preview(limit: usize) -> OutputSink {
        OutputSink {
            inner: std::sync::Arc::new(std::sync::Mutex::new(OutputSinkState {
                retained: Vec::new(),
                written: 0,
                omitted: 0,
                retention_limit: limit,
                overflow: Overflow::Omit,
                capped: false,
            })),
        }
    }

    /// A sink retaining at most `limit` bytes and flagging capture as capped
    /// past it (the complete-output mode's bounded capacity).
    #[must_use]
    pub fn complete(limit: usize) -> OutputSink {
        OutputSink {
            inner: std::sync::Arc::new(std::sync::Mutex::new(OutputSinkState {
                retained: Vec::new(),
                written: 0,
                omitted: 0,
                retention_limit: limit,
                overflow: Overflow::Cap,
                capped: false,
            })),
        }
    }

    /// Lock the state, recovering from poison (the mutation is a single
    /// infallible step, so recovered bytes are authoritative — a poisoned
    /// lock must never silently drop accepted output).
    fn lock(&self) -> std::sync::MutexGuard<'_, OutputSinkState> {
        match self.inner.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    /// Accept one write. The bytes the sink cannot retain are counted in
    /// `omitted` and (in [`Overflow::Cap`]) mark the capture capped. Always
    /// returns `Ok`: a bounded capture is a policy, not a program error.
    ///
    /// A retained prefix that would split a UTF-8 sequence is trimmed to the
    /// last char boundary, so the preview is always valid UTF-8; the trimmed
    /// bytes are counted as omitted, never lost from accounting.
    pub fn write(&self, bytes: &[u8]) {
        let mut state = self.lock();
        state.written = state.written.saturating_add(bytes.len());
        let room = state.retention_limit.saturating_sub(state.retained.len());
        if bytes.len() <= room {
            state.retained.extend_from_slice(bytes);
            return;
        }
        if room > 0 {
            // Retain as much as fits, trimmed to a char boundary so the
            // preview never ends mid-codepoint.
            let take = utf8_boundary_prefix(&bytes[..room]);
            state.retained.extend_from_slice(&bytes[..take]);
            state.omitted = state.omitted.saturating_add(bytes.len() - take);
        } else {
            state.omitted = state.omitted.saturating_add(bytes.len());
        }
        if state.overflow == Overflow::Cap {
            state.capped = true;
        }
    }

    /// The retained bytes for display.
    #[must_use]
    pub fn retained(&self) -> Vec<u8> {
        self.lock().retained.clone()
    }

    /// The retained bytes as (valid) UTF-8 for display.
    #[must_use]
    pub fn retained_text(&self) -> String {
        String::from_utf8_lossy(&self.lock().retained).into_owned()
    }

    /// Retained / written / omitted byte counts.
    #[must_use]
    pub fn stats(&self) -> OutputStats {
        let s = self.lock();
        OutputStats {
            written: s.written,
            retained: s.retained.len(),
            omitted: s.omitted,
        }
    }

    /// Whether complete-output capture hit its capacity bound.
    #[must_use]
    pub fn capped(&self) -> bool {
        self.lock().capped
    }
}

/// The largest prefix of `bytes` that ends on a UTF-8 char boundary.
///
/// Used to keep a truncated preview valid UTF-8 without scanning back into an
/// already-accepted buffer. A leading continuation byte (`10xxxxxx`) means the
/// cut landed inside a sequence, so we shorten to the last lead byte.
fn utf8_boundary_prefix(bytes: &[u8]) -> usize {
    let mut end = bytes.len();
    // Walk back over continuation bytes (at most 3 for a valid sequence).
    let mut back = 0;
    while end > 0 && back < 3 {
        if bytes[end - 1] & 0b1100_0000 == 0b1000_0000 {
            end -= 1;
            back += 1;
        } else {
            break;
        }
    }
    if end == 0 {
        return 0;
    }
    let lead = bytes[end - 1];
    let expected = if lead < 0x80 {
        1
    } else if lead & 0b1110_0000 == 0b1100_0000 {
        2
    } else if lead & 0b1111_0000 == 0b1110_0000 {
        3
    } else if lead & 0b1111_1000 == 0b1111_0000 {
        4
    } else {
        1
    };
    if bytes.len() - (end - 1) >= expected {
        bytes.len()
    } else {
        end - 1
    }
}

/// The browser-hosted Playground host.
///
/// It is a deterministic, self-contained [`Host`]: standard output is
/// collected in memory, standard input is a caller-supplied string consumed
/// line by line, and arguments are plain data. It holds no browser state, no
/// global state, and no operating-system authority. Its optional capabilities
/// (filesystem, clock, sleep) are unavailable and report `E5002`; it never
/// fakes them and never reaches the network, the DOM, or storage.
///
/// Embedders construct it from an optional stdin string and arguments, keep a
/// [`BrowserStdout`] handle, run a module through [`Interp`](crate::run::Interp)
/// with it, then read the collected stdout back through the handle.
pub struct BrowserHost {
    stdout: BrowserStdout,
    input: Option<std::io::Cursor<Vec<u8>>>,
    args: Vec<String>,
    max_stdout: Option<usize>,
    /// Optional bounded-memory sink (0.3.2 development). When present, writes
    /// are mirrored into the sink and `max_stdout` is not applied: the sink's
    /// own retention policy (preview or complete) governs, so a full preview
    /// never fails the program.
    sink: Option<OutputSink>,
}

impl BrowserHost {
    /// A host with no input and no arguments and no output limit.
    #[must_use]
    pub fn new() -> BrowserHost {
        BrowserHost {
            stdout: BrowserStdout::default(),
            input: None,
            args: Vec::new(),
            max_stdout: None,
            sink: None,
        }
    }

    /// A host with an optional standard-input string and program arguments.
    #[must_use]
    pub fn from_parts(stdin: Option<String>, args: Vec<String>) -> BrowserHost {
        BrowserHost {
            stdout: BrowserStdout::default(),
            input: stdin.map(|s| std::io::Cursor::new(s.into_bytes())),
            args,
            max_stdout: None,
            sink: None,
        }
    }

    /// A host that writes standard output into a caller-supplied shared
    /// buffer. The interpreter takes ownership of the host, so this is how an
    /// embedder that cannot hold the host (for example a WebAssembly export
    /// that builds it inside a closure) reads the output back afterwards.
    #[must_use]
    pub fn with_stdout(
        stdout: BrowserStdout,
        stdin: Option<String>,
        args: Vec<String>,
    ) -> BrowserHost {
        BrowserHost {
            stdout,
            input: stdin.map(|s| std::io::Cursor::new(s.into_bytes())),
            args,
            max_stdout: None,
            sink: None,
        }
    }

    /// Bound the total number of bytes this host will accept on standard
    /// output. This is an *application* resource policy, not a language
    /// rule: exceeding it is a genuine host I/O failure (`E4020`), so a
    /// runaway `print` loop cannot exhaust the embedder's memory before the
    /// embedder terminates execution.
    ///
    /// A host also configured with [`BrowserHost::with_output_sink`] ignores
    /// this bound: the sink's own retention policy governs and a full sink is
    /// never a program failure. This method is retained for the historical
    /// 1 MiB capture contract (the released runtimes and their tests).
    #[must_use]
    pub fn with_stdout_limit(mut self, max_bytes: usize) -> BrowserHost {
        self.max_stdout = Some(max_bytes);
        self
    }

    /// Attach a bounded-memory [`OutputSink`]. When present, output is
    /// mirrored into the sink and `with_stdout_limit` is not applied, so a
    /// program that prints more than the retention bound keeps running and the
    /// embedder reads exact retained/written/omitted counts instead of a fatal
    /// `E4020`.
    #[must_use]
    pub fn with_output_sink(mut self, sink: OutputSink) -> BrowserHost {
        self.sink = Some(sink);
        self
    }

    /// The attached output sink, if any.
    #[must_use]
    pub fn output_sink(&self) -> Option<OutputSink> {
        self.sink.clone()
    }

    /// A cloneable handle to this host's standard-output buffer.
    #[must_use]
    pub fn stdout_handle(&self) -> BrowserStdout {
        self.stdout.clone()
    }

    /// The bytes written to standard output so far.
    ///
    /// A poisoned lock cannot occur while the bytes are being mutated (the
    /// mutation is a single `extend_from_slice` with no panic-capable step
    /// that could tear the buffer), so poison is recovered rather than
    /// treated as an empty buffer: accepted bytes are never silently lost.
    #[must_use]
    pub fn stdout(&self) -> Vec<u8> {
        match self.stdout.lock() {
            Ok(v) => v.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        }
    }

    /// The bytes written to standard output, as UTF-8 (lossily).
    #[must_use]
    pub fn stdout_text(&self) -> String {
        String::from_utf8_lossy(&self.stdout()).into_owned()
    }
}

impl Default for BrowserHost {
    fn default() -> BrowserHost {
        BrowserHost::new()
    }
}

impl Host for BrowserHost {
    fn write_stdout(&mut self, bytes: &[u8]) -> HostResult<()> {
        // Recover from a poisoned lock instead of silently dropping the write.
        // `Vec<u8>` bytes are never left torn by a panic elsewhere (the only
        // mutation here is a single infallible `extend_from_slice`), so the
        // recovered bytes are authoritative. Returning `Ok` after dropping a
        // write would corrupt the observable output contract.
        //
        // When a bounded sink is attached, it is the sole destination: bytes
        // go only to the sink (never to the unbounded raw buffer), so memory
        // is genuinely bounded by the retention policy and a full preview
        // never fails the program. The embedder reads counts and the retained
        // prefix from the sink.
        if let Some(sink) = &self.sink {
            sink.write(bytes);
            return Ok(());
        }
        let mut v = match self.stdout.lock() {
            Ok(v) => v,
            Err(poisoned) => poisoned.into_inner(),
        };
        if let Some(limit) = self.max_stdout {
            if v.len().saturating_add(bytes.len()) > limit {
                return Err(HostError::io(format!(
                    "standard output exceeded the {limit} byte limit"
                )));
            }
        }
        v.extend_from_slice(bytes);
        Ok(())
    }

    fn read_line(&mut self) -> HostResult<Option<String>> {
        match self.input.as_mut() {
            Some(reader) => read_normalized_line(reader),
            None => Ok(None),
        }
    }

    fn args(&self) -> Vec<String> {
        self.args.clone()
    }

    fn read_file(&self, _path: &str) -> HostResult<Option<String>> {
        Err(HostError::unavailable(
            "the filesystem capability is not available in this host",
        ))
    }

    fn write_file(&mut self, _path: &str, _content: &str) -> HostResult<()> {
        Err(HostError::unavailable(
            "the filesystem capability is not available in this host",
        ))
    }

    fn now_unix(&self) -> HostResult<i64> {
        Err(HostError::unavailable(
            "the clock capability is not available in this host",
        ))
    }

    fn now_local(&self) -> HostResult<LocalTime> {
        Err(HostError::unavailable(
            "the clock capability is not available in this host",
        ))
    }

    fn sleep_ms(&mut self, _ms: u64) -> HostResult<()> {
        // Deliberately unavailable: the browser must not busy-wait or block.
        Err(HostError::unavailable(
            "the sleep capability is not available in this host",
        ))
    }
}
