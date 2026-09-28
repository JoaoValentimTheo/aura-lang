#![cfg(feature = "cli")]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! The `aura` command-line contract (LANGUAGE_SPEC §46-§49).
//!
//! Real subprocess tests: exit code, stdout, and stderr for each command,
//! including stdin, program arguments, and usage errors.

use std::io::Write;
use std::process::{Command, Stdio};

fn aura() -> Command {
    Command::new(env!("CARGO_BIN_EXE_aura"))
}

/// Run with the given args and no stdin, returning `(code, stdout, stderr)`.
fn run(args: &[&str]) -> (i32, String, String) {
    let out = aura().args(args).stdin(Stdio::null()).output().unwrap();
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn tmp(name: &str, contents: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!("aura-cli-{}-{name}", std::process::id()));
    std::fs::write(&path, contents).unwrap();
    path
}

#[test]
fn no_arguments_prints_global_help_and_succeeds() {
    let (code, stdout, _) = run(&[]);
    assert_eq!(code, 0);
    assert!(stdout.contains("Usage: aura"));
    for command in ["run", "check", "eval", "repl", "version"] {
        assert!(stdout.contains(command), "help omits `{command}`");
    }
}

#[test]
fn dash_dash_help_prints_global_help() {
    let (code, stdout, _) = run(&["--help"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("Usage: aura"));
}

#[test]
fn per_command_help_succeeds() {
    for command in ["run", "check", "eval", "repl", "version"] {
        let (code, stdout, stderr) = run(&[command, "--help"]);
        assert_eq!(code, 0, "{command} --help");
        assert!(stdout.contains(command), "{command} --help names itself");
        assert!(stderr.is_empty());
    }
}

#[test]
fn unknown_command_is_a_usage_error() {
    let (code, stdout, stderr) = run(&["nope"]);
    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    assert!(stderr.contains("unknown command"));
    for command in ["run", "check", "eval", "repl", "version"] {
        assert!(stderr.contains(command), "hint omits `{command}`");
    }
}

#[test]
fn version_prints_and_rejects_extra_arguments() {
    let (code, stdout, _) = run(&["version"]);
    assert_eq!(code, 0);
    assert!(stdout.starts_with("aura "));
    let (code, _, stderr) = run(&["version", "extra"]);
    assert_eq!(code, 2);
    assert!(stderr.contains("takes no arguments"));
}

#[test]
fn run_requires_a_source_argument() {
    let (code, _, stderr) = run(&["run"]);
    assert_eq!(code, 2);
    assert!(stderr.contains("usage: aura run"));
}

#[test]
fn check_requires_a_source_argument() {
    let (code, _, stderr) = run(&["check"]);
    assert_eq!(code, 2);
    assert!(stderr.contains("usage: aura check"));
}

#[test]
fn eval_requires_exactly_one_argument() {
    let (code, _, stderr) = run(&["eval"]);
    assert_eq!(code, 2);
    assert!(stderr.contains("usage: aura eval"));
    let (code, _, stderr) = run(&["eval", "print(1)", "extra"]);
    assert_eq!(code, 2);
    assert!(stderr.contains("exactly one argument"));
}

#[test]
fn repl_rejects_positional_arguments() {
    let (code, _, stderr) = run(&["repl", "extra"]);
    assert_eq!(code, 2);
    assert!(stderr.contains("takes no arguments"));
}

#[test]
fn run_executes_a_valid_program() {
    let path = tmp("ok.aura", "fn main() { print(1 + 2) }");
    let (code, stdout, stderr) = run(&["run", path.to_str().unwrap()]);
    std::fs::remove_file(&path).ok();
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(stdout, "3\n");
    assert!(stderr.is_empty());
}

#[test]
fn run_reports_a_language_diagnostic_on_stderr_with_code_1() {
    let path = tmp("bad.aura", "fn main() { print(undefined) }");
    let (code, stdout, stderr) = run(&["run", path.to_str().unwrap()]);
    std::fs::remove_file(&path).ok();
    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert!(stderr.contains("E2003"));
}

#[test]
fn check_is_silent_on_success() {
    let path = tmp("check-ok.aura", "fn main() { print(1) }");
    let (code, stdout, stderr) = run(&["check", path.to_str().unwrap()]);
    std::fs::remove_file(&path).ok();
    assert_eq!(code, 0);
    assert!(stdout.is_empty());
    assert!(stderr.is_empty());
}

#[test]
fn program_arguments_after_the_source_may_begin_with_dash() {
    let path = tmp("args.aura", "fn main() { for a in args() { print(a) } }");
    let (code, stdout, _) = run(&["run", path.to_str().unwrap(), "--flag", "x"]);
    std::fs::remove_file(&path).ok();
    assert_eq!(code, 0);
    assert_eq!(stdout, "--flag\nx\n");
}

#[test]
fn run_reads_source_from_stdin_with_dash() {
    let mut child = aura()
        .args(["run", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"fn main() { print(42) }")
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(String::from_utf8_lossy(&out.stdout), "42\n");
}

#[test]
fn check_reads_source_from_stdin_with_dash() {
    let mut child = aura()
        .args(["check", "-"])
        .stdin(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"fn main() { print(1) }")
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn eval_runs_a_one_liner() {
    let (code, stdout, _) = run(&["eval", "print(1 + 2)"]);
    assert_eq!(code, 0);
    assert_eq!(stdout, "3\n");
}

#[test]
fn eval_reports_a_diagnostic_on_stderr() {
    let (code, _, stderr) = run(&["eval", "nope"]);
    assert_eq!(code, 1);
    assert!(stderr.contains("E2003"));
}

#[test]
fn missing_file_is_an_io_error() {
    let (code, _, stderr) = run(&["run", "/nonexistent/aura-file.aura"]);
    assert_eq!(code, 1);
    assert!(stderr.contains("cannot read"));
}

#[test]
fn unicode_paths_are_supported() {
    let path = std::env::temp_dir().join(format!("aura-cli-üñíçødé-{}.aura", std::process::id()));
    std::fs::write(&path, "fn main() { print(\"ok\") }").unwrap();
    let (code, stdout, _) = run(&["run", path.to_str().unwrap()]);
    std::fs::remove_file(&path).ok();
    assert_eq!(code, 0);
    assert_eq!(stdout, "ok\n");
}

#[test]
fn crlf_and_lf_sources_are_equivalent() {
    let lf = tmp("lf.aura", "fn main() {\n print(1)\n print(2)\n}");
    let crlf = tmp("crlf.aura", "fn main() {\r\n print(1)\r\n print(2)\r\n}");
    let (c1, o1, _) = run(&["run", lf.to_str().unwrap()]);
    let (c2, o2, _) = run(&["run", crlf.to_str().unwrap()]);
    std::fs::remove_file(&lf).ok();
    std::fs::remove_file(&crlf).ok();
    assert_eq!(c1, 0);
    assert_eq!(c2, 0);
    assert_eq!(o1, o2);
}
