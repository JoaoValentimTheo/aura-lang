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

fn cmd_run(args: &[String]) -> ExitCode {
    let Some(path) = args.get(1).map(String::as_str) else {
        eprintln!("usage: aura run <file|-> [program args...]");
        return ExitCode::from(2);
    };
    let (src, file) = match read_source(Some(path)) {
        Ok(v) => v,
        Err(c) => return c,
    };
    // Program arguments are everything after the script path. When the script
    // was read from stdin (`-`), the process stdin has been consumed as
    // source, so no input source is wired.
    let program_args: Vec<String> = args.iter().skip(2).cloned().collect();
    let input = if path == "-" {
        None
    } else {
        Some(Box::new(std::io::BufReader::new(std::io::stdin())) as Box<dyn std::io::BufRead + Send>)
    };
    match aura::run_program_with(&src, &file, program_args, input) {
        Ok(()) => ExitCode::SUCCESS,
        Err(d) => {
            eprintln!("{}", render_with_source(&file, &src, &d));
            ExitCode::FAILURE
        }
    }
}

fn cmd_check(args: &[String]) -> ExitCode {
    let Some(path) = args.get(1).map(String::as_str) else {
        eprintln!("usage: aura check <file|->");
        return ExitCode::from(2);
    };
    if args.len() > 2 {
        eprintln!("aura check takes exactly one source argument");
        return ExitCode::from(2);
    }
    let (src, file) = match read_source(Some(path)) {
        Ok(v) => v,
        Err(c) => return c,
    };
    match aura::compile(&src, "module") {
        Ok(_) => ExitCode::SUCCESS,
        Err(d) => {
            eprintln!("{}", render_with_source(&file, &src, &d));
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
    match aura::run_toplevel_with(code, "<eval>", Vec::new(), input) {
        Ok(()) => ExitCode::SUCCESS,
        Err(d) => {
            eprintln!("{}", render_with_source("<eval>", code, &d));
            ExitCode::FAILURE
        }
    }
}
