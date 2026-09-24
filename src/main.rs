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
        Some("run") => cmd_run(&args),
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

fn cmd_run(args: &[String]) -> ExitCode {
    let path = args.get(1).map(String::as_str);
    let (src, file) = match read_source(path) {
        Ok(v) => v,
        Err(c) => return c,
    };
    // Program arguments are everything after the script path. When the script
    // was read from stdin (`-`), the process stdin has been consumed as
    // source, so no input source is wired.
    let program_args: Vec<String> = args.iter().skip(2).cloned().collect();
    let input = if path == Some("-") {
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

fn cmd_check(path: Option<&str>) -> ExitCode {
    let (src, file) = match read_source(path) {
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

fn cmd_eval(code: Option<&str>) -> ExitCode {
    let Some(code) = code else {
        eprintln!("usage: aura eval <code>");
        return ExitCode::from(2);
    };
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
