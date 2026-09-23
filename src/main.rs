//! The `aura` command line tool.
//!
//! Subcommands:
//! * `aura run <file>`   — parse, check, execute
//! * `aura check <file>` — parse and check only
//! * `aura eval <code>`  — run a one-liner
//! * `aura version`      — print the version
//!
//! `aura run` reads `<file>`, or stdin when `<file>` is `-`.

use std::io::Read;
use std::process::ExitCode;

use aura::error::render_with_source;

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
        Some("run") => cmd_run(args.get(1).map(String::as_str)),
        Some("check") => cmd_check(args.get(1).map(String::as_str)),
        Some("eval") => cmd_eval(args.get(1).map(String::as_str)),
        Some("repl") => cmd_repl(),
        Some("version") | None => {
            println!("aura {}", aura::VERSION);
            ExitCode::SUCCESS
        }
        Some(other) => {
            eprintln!("unknown command `{other}`; try: run, check, eval, version");
            ExitCode::from(2)
        }
    }
}

fn read_source(path: Option<&str>) -> Result<(String, String), ExitCode> {
    let path = path.unwrap_or("-");
    if path == "-" {
        let mut buf = String::new();
        std::io::stdin()
            .read_to_string(&mut buf)
            .map_err(|_| ExitCode::FAILURE)?;
        Ok((buf, "<stdin>".to_string()))
    } else {
        std::fs::read_to_string(path)
            .map(|s| (s, path.to_string()))
            .map_err(|e| {
                eprintln!("aura: cannot read {path}: {e}");
                ExitCode::FAILURE
            })
    }
}

fn cmd_run(path: Option<&str>) -> ExitCode {
    let (src, file) = match read_source(path) {
        Ok(v) => v,
        Err(c) => return c,
    };
    match aura::run_program(&src, &file) {
        Ok(()) => ExitCode::SUCCESS,
        Err(d) => {
            eprintln!("{}", render_with_source(&file, &src, &d));
            ExitCode::FAILURE
        }
    }
}

fn cmd_check(path: Option<&str>) -> ExitCode {
    let (src, file) = match read_source(path) {
        Ok(v) => v,
        Err(c) => return c,
    };
    match aura::parse::parse(&src).and_then(|m| aura::check::Checker::module(&m)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(d) => {
            eprintln!("{}", render_with_source(&file, &src, &d));
            ExitCode::FAILURE
        }
    }
}

fn cmd_eval(code: Option<&str>) -> ExitCode {
    let Some(code) = code else {
        eprintln!("usage: aura eval <code>");
        return ExitCode::from(2);
    };
    match aura::run_toplevel_stdout(code, "<eval>") {
        Ok(()) => ExitCode::SUCCESS,
        Err(d) => {
            eprintln!("{}", render_with_source("<eval>", code, &d));
            ExitCode::FAILURE
        }
    }
}
