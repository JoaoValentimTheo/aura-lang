//! The `aura` command line tool.
//!
//! Subcommands:
//! * `aura run <file|-> [args...]` — parse, check, execute
//! * `aura check <file|->`         — parse and check only
//! * `aura eval <code>`            — run a one-liner
//! * `aura repl`                   — interactive session (with the `repl` feature)
//! * `aura version`                — print the version
//!
//! `aura --help` (and `aura <command> --help`) print concise help. Running
//! `aura` with no arguments prints global help. `-` reads source from stdin.

// The CLI serializes the AIS document with `expect`: serialization of the AIS
// schema cannot fail for a well-formed document (no non-string map keys, no
// non-finite floats), and a panic here would be a compiler bug, not user
// input. This mirrors the tests' convention of allowing the infallible-path
// expect.
#![cfg_attr(feature = "json", allow(clippy::expect_used))]

use std::io::Read;
use std::process::ExitCode;

use aura::error::render_with_source;

/// The command inventory, used for both dispatch hints and help so the two can
/// never drift (LANGUAGE_SPEC §47/§48).
#[cfg(feature = "json")]
const COMMANDS: [(&str, &str); 7] = [
    ("run", "parse, check, and execute a program"),
    ("check", "parse and check without executing"),
    ("eval", "run a one-line program"),
    ("repl", "start an interactive session"),
    ("version", "print the compiler version"),
    ("ais", "print an AIS/0.1 semantic payload as JSON"),
    ("mcp", "serve the AIS semantic model over MCP stdio"),
];

#[cfg(not(feature = "json"))]
const COMMANDS: [(&str, &str); 5] = [
    ("run", "parse, check, and execute a program"),
    ("check", "parse and check without executing"),
    ("eval", "run a one-line program"),
    ("repl", "start an interactive session"),
    ("version", "print the compiler version"),
];

fn help(global: bool, command: Option<&str>) -> ExitCode {
    match command {
        Some("run") => print!(
            "aura run <file|-> [program args...]\n\n\
             Parse, check, and execute a program. `<file>` is a path, or `-` to\n\
             read source from stdin. Arguments after the source path are passed\n\
             to the program and may begin with `-`. When the source is read from\n\
             stdin, `read_line()` has no input left to consume.\n"
        ),
        Some("check") => print!(
            "aura check <file|->\n\n\
             Parse and check a program without executing it. `-` reads source\n\
             from stdin. Prints nothing on success.\n"
        ),
        #[cfg(feature = "json")]
        Some("ais") => print!(
            "aura ais [snapshot|slice|delta] ...\n\n\
             Print an AIS/0.1 semantic payload as JSON. It grants no capability\n\
             (no filesystem, network, Python, or Host authority).\n\n\
             aura ais <file|->                 the semantic snapshot\n\
             aura ais slice <file|-> <target> [depth] [budget]\n\
                                               a task-focused slice\n\
             aura ais delta <old-file|-> <new-file|->\n\
                                               the declaration delta\n"
        ),
        #[cfg(feature = "json")]
        Some("mcp") => print!(
            "aura mcp\n\n\
             Serve the AIS/0.1 semantic model over the MCP stdio transport\n\
             (newline-delimited JSON-RPC 2.0). The adapter exposes the same\n\
             semantic operations as `aura ais` and grants no capability: it\n\
             reads only the source text the client sends, never the environment,\n\
             the network, or a secret.\n"
        ),
        Some("eval") => print!(
            "aura eval <code>\n\n\
             Run a one-line program. Process stdin stays available to\n\
             `read_line()`; `args()` is empty.\n"
        ),
        Some("repl") => print!(
            "aura repl\n\n\
             Start an interactive session. Declarations persist across\n\
             submissions. `:help` lists commands; `:quit` or Ctrl-D exits.\n"
        ),
        Some("version") => print!("aura version\n\nPrint the compiler version.\n"),
        Some(other) => {
            eprintln!("unknown command `{other}`; try: {}", command_list());
            return ExitCode::from(2);
        }
        None if !global => {
            eprintln!("missing command; try: {}", command_list());
            return ExitCode::from(2);
        }
        None => {
            println!(
                "aura {} — the Aura language compiler\n\nUsage: aura <command> [options]\n\nCommands:\n{}",
                aura::VERSION,
                COMMANDS
                    .iter()
                    .map(|(name, desc)| format!("  {name:<8} {desc}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            );
        }
    }
    ExitCode::SUCCESS
}

/// The `run, check, eval, repl, version` hint, derived from [`COMMANDS`].
fn command_list() -> String {
    COMMANDS
        .iter()
        .map(|(name, _)| *name)
        .collect::<Vec<_>>()
        .join(", ")
}

/// Whether the tail begins with the `--help` flag. `--help` is only a request
/// for help in the first position after the command, so program arguments
/// (`aura run app.aura --help`) are passed through to the program rather than
/// intercepted.
fn help_requested(args: &[String]) -> bool {
    args.first().is_some_and(|a| a == "--help")
}

fn cmd_repl() -> ExitCode {
    #[cfg(feature = "repl")]
    {
        match aura::repl::run() {
            Ok(()) => ExitCode::SUCCESS,
            Err(d) => {
                eprintln!("{d}");
                ExitCode::FAILURE
            }
        }
    }
    #[cfg(not(feature = "repl"))]
    {
        eprintln!("this build has no REPL; rebuild with the `repl` feature");
        ExitCode::from(2)
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("run") => {
            if help_requested(&args[1..]) {
                return help(false, Some("run"));
            }
            cmd_run(&args)
        }
        Some("check") => {
            if help_requested(&args[1..]) {
                return help(false, Some("check"));
            }
            cmd_check(&args)
        }
        Some("eval") => {
            if help_requested(&args[1..]) {
                return help(false, Some("eval"));
            }
            cmd_eval(&args)
        }
        #[cfg(feature = "json")]
        Some("ais") => {
            if help_requested(&args[1..]) {
                return help(false, Some("ais"));
            }
            cmd_ais(&args)
        }
        #[cfg(feature = "json")]
        Some("mcp") => {
            if help_requested(&args[1..]) {
                return help(false, Some("mcp"));
            }
            if args.len() > 1 {
                eprintln!("aura mcp takes no arguments; try: {}", command_list());
                return ExitCode::from(2);
            }
            cmd_mcp()
        }
        Some("repl") => {
            if help_requested(&args[1..]) {
                return help(false, Some("repl"));
            }
            if args.len() > 1 {
                eprintln!("aura repl takes no arguments; try: {}", command_list());
                return ExitCode::from(2);
            }
            cmd_repl()
        }
        Some("version") => {
            if help_requested(&args[1..]) {
                return help(false, Some("version"));
            }
            if args.len() > 1 {
                eprintln!("aura version takes no arguments; try: {}", command_list());
                return ExitCode::from(2);
            }
            println!("aura {}", aura::VERSION);
            ExitCode::SUCCESS
        }
        Some("--help") => help(true, None),
        // No command prints global help rather than behaving like `version`.
        None => help(true, None),
        Some(other) => {
            eprintln!("unknown command `{other}`; try: {}", command_list());
            ExitCode::from(2)
        }
    }
}

fn read_source(path: Option<&str>) -> Result<(String, String), ExitCode> {
    let path = path.unwrap_or("-");
    let file = if path == "-" { "<stdin>" } else { path };
    let bytes = if path == "-" {
        let mut buf = Vec::new();
        std::io::stdin().read_to_end(&mut buf).map(|_| buf)
    } else {
        std::fs::read(path)
    }
    .map_err(|e| {
        eprintln!("aura: cannot read {file}: {e}");
        ExitCode::FAILURE
    })?;
    let source = aura::lex::decode_source(&bytes).map_err(|d| {
        eprintln!(
            "{}",
            render_with_source(file, &String::from_utf8_lossy(&bytes), &d)
        );
        ExitCode::FAILURE
    })?;
    Ok((source.to_string(), file.to_string()))
}

/// The color policy for this invocation (Keystone §28). `--color <mode>`
/// overrides `AURA_COLOR`, which overrides the `NO_COLOR` convention; `auto`
/// colors only a terminal, so a redirected stream stays plain and stable.
fn color_choice(args: &[String]) -> aura::diagnostic::ColorChoice {
    let mut override_choice = None;
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--color" {
            if let Some(v) = args.get(i + 1) {
                override_choice = aura::diagnostic::ColorChoice::parse(v);
            }
        } else if let Some(v) = args[i].strip_prefix("--color=") {
            override_choice = aura::diagnostic::ColorChoice::parse(v);
        }
        i += 1;
    }
    aura::diagnostic::resolve_choice(override_choice, &|k| std::env::var(k).ok())
}

/// Whether the diagnostic destination (stderr) is a terminal.
fn stderr_is_terminal() -> bool {
    use std::io::IsTerminal;
    std::io::stderr().is_terminal()
}

fn render_cli_report(
    report: &aura::error::DiagnosticReport,
    stdin_source: bool,
    args: &[String],
) -> String {
    let d = report.diagnostic();
    let location = report.source_diagnostic().location().and_then(|loc| {
        report.sources().get(loc.source).map(|s| {
            let text = s.text();
            let (line, col) = aura::error::line_col(text, loc.span.start);
            format!("{}:{line}:{col}", s.name())
        })
    });
    let color = aura::diagnostic::use_color(color_choice(args), stderr_is_terminal());
    let mut rendered = aura::diagnostic::render_diagnostic(
        d,
        report.source_diagnostic().presentation(),
        location.as_deref(),
        color,
    );
    if stdin_source && d.code == aura::error::codes::UNKNOWN_MODULE {
        rendered.push_str("; filesystem modules are unavailable for <stdin>");
    }
    rendered
}

/// Split the arguments of a subcommand into presentation flags (only those
/// *before* the source path) and the remaining positionals.
///
/// For `run`, everything after the source path is the program's own argument
/// list and may begin with `-`, so `--color` after the path belongs to the
/// program and is not consumed here. For a subcommand with no program
/// arguments, flags may appear anywhere.
fn split_subcommand_args(args: &[String], source_is_first_positional: bool) -> Vec<&String> {
    let mut positionals = Vec::new();
    let mut i = 1; // skip the subcommand name
    while i < args.len() {
        let a = &args[i];
        if source_is_first_positional && !positionals.is_empty() {
            // Past the source path: keep everything verbatim, because `run`'s
            // trailing arguments belong to the program and may start with `-`.
            positionals.push(a);
            i += 1;
            continue;
        }
        if a == "--color" {
            i += 2;
            continue;
        }
        if a.starts_with("--color=") {
            i += 1;
            continue;
        }
        positionals.push(a);
        i += 1;
    }
    positionals
}

fn cmd_run(args: &[String]) -> ExitCode {
    let positionals = split_subcommand_args(args, true);
    let Some(path) = positionals.first().map(|p| p.as_str()) else {
        eprintln!("usage: aura run <file|-> [program args...]");
        return ExitCode::from(2);
    };
    // Program arguments are everything after the script path (presentation
    // flags before the path are not program arguments). When the script was
    // read from stdin (`-`), the process stdin has been consumed as source,
    // so no input source is wired.
    let program_args: Vec<String> = positionals.iter().skip(1).map(|p| (*p).clone()).collect();
    let input = if path == "-" {
        None
    } else {
        Some(Box::new(std::io::BufReader::new(std::io::stdin())) as Box<dyn std::io::BufRead + Send>)
    };
    let result = if path == "-" {
        let (src, file) = match read_source(Some(path)) {
            Ok(v) => v,
            Err(c) => return c,
        };
        aura::compile_named_with_mode(&src, &file, aura::CompileMode::Program)
            .and_then(|compilation| compilation.execute_with(None, program_args, input))
    } else {
        aura::compile_file_with_mode(path, aura::CompileMode::Program)
            .and_then(|compilation| compilation.execute_with(None, program_args, input))
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(report) => {
            eprintln!("{}", render_cli_report(&report, path == "-", args));
            ExitCode::FAILURE
        }
    }
}

fn cmd_check(args: &[String]) -> ExitCode {
    let positionals = split_subcommand_args(args, false);
    let Some(path) = positionals.first().map(|p| p.as_str()) else {
        eprintln!("usage: aura check <file|->");
        return ExitCode::from(2);
    };
    // `--color [mode]` / `--color=mode` is a presentation flag, not a
    // positional argument (Keystone §28); `check` has no program arguments,
    // so the flag may appear anywhere.
    let positionals = split_subcommand_args(args, false);
    if positionals.len() > 1 {
        eprintln!("aura check takes exactly one source argument");
        return ExitCode::from(2);
    }
    let result = if path == "-" {
        let (src, file) = match read_source(Some(path)) {
            Ok(v) => v,
            Err(c) => return c,
        };
        aura::compile_named_with_mode(&src, &file, aura::CompileMode::Module)
    } else {
        aura::compile_file_with_mode(path, aura::CompileMode::Module)
    };
    match result {
        Ok(_) => ExitCode::SUCCESS,
        Err(report) => {
            eprintln!("{}", render_cli_report(&report, path == "-", args));
            ExitCode::FAILURE
        }
    }
}

/// `aura ais <file|->`: print the AIS/0.1 document for a source.
///
/// The document is a stable semantic protocol for tooling. It is emitted even
/// when checking finds diagnostics: the `diagnostics` array carries them, and
/// the exit code is 1 when the source was rejected so a caller can branch
/// without parsing the document.
#[cfg(feature = "json")]
fn cmd_ais(args: &[String]) -> ExitCode {
    let tail: Vec<&str> = args.iter().skip(1).map(String::as_str).collect();
    match tail.first() {
        Some(&"slice") => cmd_ais_slice(&tail[1..]),
        Some(&"delta") => cmd_ais_delta(&tail[1..]),
        Some(&"snapshot") => cmd_ais_snapshot(&tail[1..]),
        _ => cmd_ais_snapshot(&tail),
    }
}

/// Parse a source path into `(text, display-name)` or report the CLI error.
fn ais_read(path: &str) -> Result<(String, String), ExitCode> {
    read_source(Some(path))
}

/// Build the snapshot document for a source, attaching any checker diagnostic.
fn ais_snapshot_of(src: &str, file: &str) -> (aura::ais::Document, bool) {
    let module = match aura::parse::parse(src) {
        Ok(m) => m,
        Err(d) => {
            let mut doc = aura::ais::Document {
                ais_version: aura::ais::AIS_VERSION.to_string(),
                aura_version: aura::VERSION.to_string(),
                language_version: aura::LANGUAGE_VERSION.to_string(),
                capabilities: aura::ais::Capabilities::default(),
                source_name: file.to_string(),
                revision: Some(aura::ais::revision_of(src)),
                symbols: Vec::new(),
                diagnostics: vec![aura::ais::Diagnostic::from_diag(&d, Some(src))],
            };
            doc.diagnostics.truncate(1);
            return (doc, true);
        }
    };
    let mut doc = aura::ais::document(file, src, &module);
    let rejected = match aura::check::Checker::module(&module) {
        Ok(()) => false,
        Err(d) => {
            doc.diagnostics
                .push(aura::ais::Diagnostic::from_diag(&d, Some(src)));
            true
        }
    };
    (doc, rejected)
}

fn print_ais<T: serde::Serialize>(value: &T) -> ExitCode {
    println!(
        "{}",
        serde_json::to_string_pretty(value).expect("AIS payload serializes")
    );
    ExitCode::SUCCESS
}

fn cmd_ais_snapshot(args: &[&str]) -> ExitCode {
    let Some(path) = args.first() else {
        eprintln!("usage: aura ais <file|->");
        return ExitCode::from(2);
    };
    let (src, file) = match ais_read(path) {
        Ok(v) => v,
        Err(c) => return c,
    };
    let (doc, rejected) = ais_snapshot_of(&src, &file);
    let code = print_ais(&doc);
    if rejected {
        ExitCode::FAILURE
    } else {
        code
    }
}

fn cmd_ais_slice(args: &[&str]) -> ExitCode {
    let (Some(path), Some(target)) = (args.first(), args.get(1)) else {
        eprintln!("usage: aura ais slice <file|-> <target> [depth] [budget]");
        return ExitCode::from(2);
    };
    let depth = args
        .get(2)
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(1);
    let budget = args
        .get(3)
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(32);
    let (src, file) = match ais_read(path) {
        Ok(v) => v,
        Err(c) => return c,
    };
    let (doc, _) = ais_snapshot_of(&src, &file);
    let s = aura::ais::slice(&doc, target, depth, budget);
    println!(
        "{}",
        serde_json::to_string_pretty(&s).expect("AIS slice serializes")
    );
    ExitCode::SUCCESS
}

fn cmd_ais_delta(args: &[&str]) -> ExitCode {
    let (Some(old_path), Some(new_path)) = (args.first(), args.get(1)) else {
        eprintln!("usage: aura ais delta <old-file|-> <new-file|->");
        return ExitCode::from(2);
    };
    // `-` can name only one side: stdin is consumed once.
    if *old_path == "-" && *new_path == "-" {
        eprintln!("aura ais delta cannot read both sides from stdin");
        return ExitCode::from(2);
    }
    let (old_src, old_file) = match ais_read(old_path) {
        Ok(v) => v,
        Err(c) => return c,
    };
    let (new_src, new_file) = match ais_read(new_path) {
        Ok(v) => v,
        Err(c) => return c,
    };
    let (from, _) = ais_snapshot_of(&old_src, &old_file);
    let (to, _) = ais_snapshot_of(&new_src, &new_file);
    let d = aura::ais::delta(&from, &to);
    println!(
        "{}",
        serde_json::to_string_pretty(&d).expect("AIS delta serializes")
    );
    ExitCode::SUCCESS
}

/// Serve the AIS model over the MCP stdio transport.
///
/// Newline-delimited JSON-RPC 2.0: one request per line, one response per
/// line. The loop is stateless, so a malformed line produces an error response
/// (or is ignored when it is a notification) and the session continues. A
/// frame larger than `MAX_REQUEST_BYTES` is refused without buffering further.
fn cmd_mcp() -> ExitCode {
    use std::io::{BufRead, Write};
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let mut reader = stdin.lock();
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => return ExitCode::SUCCESS, // EOF ends the session cleanly
            Ok(_) => {}
            Err(e) => {
                eprintln!("aura mcp: input error: {e}");
                return ExitCode::FAILURE;
            }
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.len() > aura::mcp::MAX_REQUEST_BYTES {
            let err = serde_json::json!({
                "jsonrpc": "2.0",
                "id": serde_json::Value::Null,
                "error": { "code": -32600, "message": "request exceeds the adapter's size limit" }
            });
            let _ = writeln!(out, "{err}");
            let _ = out.flush();
            continue;
        }
        // A malformed frame is an error response, never a panic: a client bug
        // must not bring down a long-running session.
        let msg: serde_json::Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(e) => {
                let err = serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": serde_json::Value::Null,
                    "error": { "code": -32700, "message": format!("parse error: {e}") }
                });
                let _ = writeln!(out, "{err}");
                let _ = out.flush();
                continue;
            }
        };
        if let Some(response) = aura::mcp::handle_message(&msg) {
            if writeln!(out, "{response}").is_err() {
                return ExitCode::SUCCESS; // client closed the pipe
            }
            let _ = out.flush();
        }
    }
}

fn cmd_eval(args: &[String]) -> ExitCode {
    let Some(code) = args.get(1).map(String::as_str) else {
        eprintln!("usage: aura eval <code>");
        return ExitCode::from(2);
    };
    if args.len() > 2 {
        eprintln!("aura eval takes exactly one argument; wrap the code in quotes");
        return ExitCode::from(2);
    }
    // `eval` exposes process stdin to `read_line()` but has no program
    // arguments (args() is []).
    let input = Some(
        Box::new(std::io::BufReader::new(std::io::stdin())) as Box<dyn std::io::BufRead + Send>
    );
    match aura::run_named_toplevel_with(code, "<eval>", Vec::new(), input) {
        Ok(()) => ExitCode::SUCCESS,
        Err(report) => {
            eprintln!("{}", report.render());
            ExitCode::FAILURE
        }
    }
}
