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

    /// Convert into a language diagnostic with the given span.
    #[must_use]
    pub fn into_diag(self, span: Span) -> Diag {
        match self {
            HostError::Io(m) => Diag::new(codes::IO, m, span),
            HostError::Unavailable(m) => Diag::new(codes::CAPABILITY_UNAVAILABLE, m, span),
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
}

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
        std::thread::sleep(std::time::Duration::from_millis(ms.min(60_000)));
        Ok(())
    }
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
        }
    }

    /// Bound the total number of bytes this host will accept on standard
    /// output. This is an *application* resource policy, not a language
    /// rule: exceeding it is a genuine host I/O failure (`E4020`), so a
    /// runaway `print` loop cannot exhaust the embedder's memory before the
    /// embedder terminates execution.
    #[must_use]
    pub fn with_stdout_limit(mut self, max_bytes: usize) -> BrowserHost {
        self.max_stdout = Some(max_bytes);
        self
    }

    /// A cloneable handle to this host's standard-output buffer.
    #[must_use]
    pub fn stdout_handle(&self) -> BrowserStdout {
        self.stdout.clone()
    }

    /// The bytes written to standard output so far.
    #[must_use]
    pub fn stdout(&self) -> Vec<u8> {
        self.stdout.lock().map(|v| v.clone()).unwrap_or_default()
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
        if let Ok(mut v) = self.stdout.lock() {
            if let Some(limit) = self.max_stdout {
                if v.len().saturating_add(bytes.len()) > limit {
                    return Err(HostError::io(format!(
                        "standard output exceeded the {limit} byte limit"
                    )));
                }
            }
            v.extend_from_slice(bytes);
        }
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
