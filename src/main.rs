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

use std::io::Read;
use std::process::ExitCode;

use aura::error::render_with_source;

/// The command inventory, used for both dispatch hints and help so the two can
/// never drift (LANGUAGE_SPEC §47/§48).
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
