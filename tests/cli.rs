#![cfg(feature = "cli")]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! The `aura` command-line contract (LANGUAGE_SPEC §46-§49).
//!
//! Real subprocess tests: exit code, stdout, and stderr for each command,
//! including stdin, program arguments, and usage errors.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_PROJECT: AtomicUsize = AtomicUsize::new(0);

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

fn run_in(cwd: &Path, args: &[&str]) -> (i32, String, String) {
    let out = aura()
        .current_dir(cwd)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn run_stdin_in(cwd: &Path, args: &[&str], source: &str) -> (i32, String, String) {
    let mut child = aura()
        .current_dir(cwd)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(source.as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

struct TempProject(PathBuf);

impl TempProject {
    fn new(label: &str) -> Self {
        let id = NEXT_PROJECT.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "aura-cli-project-{}-{id}-{label}",
            std::process::id()
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }

    fn write(&self, relative: &str, source: &str) -> PathBuf {
        let path = self.0.join(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&path, source).unwrap();
        path
    }

    fn run(&self, args: &[&str]) -> (i32, String, String) {
        run_in(&self.0, args)
    }
}

impl Drop for TempProject {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn tmp(name: &str, contents: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("aura-cli-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("main.aura");
    std::fs::write(&path, contents).unwrap();
    path
}

fn rm_tmp(path: &std::path::Path) {
    if let Some(parent) = path.parent() {
        std::fs::remove_dir_all(parent).ok();
    }
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
fn check_rejects_extra_arguments() {
    let path = tmp("check-extra.aura", "fn main() { print(1) }");
    let (code, _, stderr) = run(&["check", path.to_str().unwrap(), "bogus"]);
    rm_tmp(&path);
    assert_eq!(code, 2);
    assert!(stderr.contains("takes exactly one"));
}

#[test]
fn run_passes_a_help_like_program_argument_through() {
    // `--help` after the source path belongs to the program, not the CLI.
    let path = tmp(
        "args-help.aura",
        "fn main() { for a in args() { print(a) } }",
    );
    let (code, stdout, stderr) = run(&["run", path.to_str().unwrap(), "--help"]);
    rm_tmp(&path);
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(stdout, "--help\n");
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
    rm_tmp(&path);
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(stdout, "3\n");
    assert!(stderr.is_empty());
}

#[test]
fn run_reports_a_language_diagnostic_on_stderr_with_code_1() {
    let path = tmp("bad.aura", "fn main() { print(undefined) }");
    let (code, stdout, stderr) = run(&["run", path.to_str().unwrap()]);
    rm_tmp(&path);
    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert!(stderr.contains("E2003"));
}

#[test]
fn run_holds_the_frame_limit_on_the_production_path() {
    // End-to-end contract canary: through the real subprocess, the cut-over
    // `Compilation` path, and the CLI's error mapping, a legal depth-510
    // program (main is frame 1) must run to completion and frame 513 must be
    // the structured E4011 the language promises — never a host failure.
    // Note: native entry points wrap execution in the 64 MiB execution
    // substrate, so this pins the *user-visible contract*, not engine
    // selection; engine discrimination lives in `tests/b1_production_path.rs`
    // (REPL, inline) and `playground/tests/node/b1_boundary.test.mjs` (wasm).
    let deep = "fn f(n) { if n <= 0 { return 0 }\n return 1 + f(n - 1) }\n";
    let ok_path = tmp(
        "frames-510.aura",
        &format!("{deep}fn main() {{ print(f(510)) }}"),
    );
    let (code, stdout, stderr) = run(&["run", ok_path.to_str().unwrap()]);
    rm_tmp(&ok_path);
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(stdout, "510\n");

    let over_path = tmp(
        "frames-511.aura",
        &format!("{deep}fn main() {{ print(f(511)) }}"),
    );
    let (code, stdout, stderr) = run(&["run", over_path.to_str().unwrap()]);
    rm_tmp(&over_path);
    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert!(stderr.contains("E4011"), "{stderr}");
}

#[test]
fn check_is_silent_on_success() {
    let path = tmp("check-ok.aura", "fn main() { print(1) }");
    let (code, stdout, stderr) = run(&["check", path.to_str().unwrap()]);
    rm_tmp(&path);
    assert_eq!(code, 0);
    assert!(stdout.is_empty());
    assert!(stderr.is_empty());
}

#[test]
fn program_arguments_after_the_source_may_begin_with_dash() {
    let path = tmp("args.aura", "fn main() { for a in args() { print(a) } }");
    let (code, stdout, _) = run(&["run", path.to_str().unwrap(), "--flag", "x"]);
    rm_tmp(&path);
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
    let dir = std::env::temp_dir().join(format!("aura-cli-üñíçødé-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("main.aura");
    std::fs::write(&path, "fn main() { print(\"ok\") }").unwrap();
    let (code, stdout, _) = run(&["run", path.to_str().unwrap()]);
    rm_tmp(&path);
    assert_eq!(code, 0);
    assert_eq!(stdout, "ok\n");
}

#[test]
fn crlf_and_lf_sources_are_equivalent() {
    let lf = tmp("lf.aura", "fn main() {\n print(1)\n print(2)\n}");
    let crlf = tmp("crlf.aura", "fn main() {\r\n print(1)\r\n print(2)\r\n}");
    let (c1, o1, _) = run(&["run", lf.to_str().unwrap()]);
    let (c2, o2, _) = run(&["run", crlf.to_str().unwrap()]);
    rm_tmp(&lf);
    rm_tmp(&crlf);
    assert_eq!(c1, 0);
    assert_eq!(c2, 0);
    assert_eq!(o1, o2);
}

#[test]
fn filesystem_child_module_runs_and_checks_from_relative_entry() {
    let project = TempProject::new("child-success");
    project.write("main.aura", "fn main() { print(child::value()) }\n");
    project.write("child.aura", "pub fn value() -> int { return 9 }\n");

    let (code, stdout, stderr) = project.run(&["run", "main.aura"]);
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(stdout, "9\n");
    let (code, stdout, stderr) = project.run(&["check", "main.aura"]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stdout.is_empty());
}

#[test]
fn filesystem_nested_child_module_runs() {
    let project = TempProject::new("nested-success");
    project.write("main.aura", "fn main() { print(foo::bar::value()) }\n");
    project.write("foo/mod.aura", "\n");
    project.write("foo/bar.aura", "pub fn value() -> int { return 11 }\n");

    let (code, stdout, stderr) = project.run(&["run", "main.aura"]);
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(stdout, "11\n");
}

#[test]
fn filesystem_child_parser_resolver_and_checker_errors_keep_file_names() {
    for (label, child_source, expected) in [
        ("parser", "@\n", "E1001"),
        ("resolver", "use missing::value\n", "E2019"),
        (
            "checker",
            "pub fn bad() -> int { return \"wrong\" }\n",
            "E3005",
        ),
    ] {
        let project = TempProject::new(label);
        project.write("main.aura", "fn main() { }\n");
        project.write("child.aura", child_source);
        let (code, stdout, stderr) = project.run(&["check", "main.aura"]);
        assert_eq!(code, 1, "{label}: {stderr}");
        assert!(stdout.is_empty(), "{label}");
        assert!(stderr.contains(expected), "{label}: {stderr}");
        assert!(stderr.contains("child.aura:"), "{label}: {stderr}");
        assert!(
            !stderr.contains(project.path().to_string_lossy().as_ref()),
            "{label}: {stderr}"
        );
    }
}

#[test]
fn filesystem_child_runtime_error_keeps_file_name() {
    let project = TempProject::new("runtime-provenance");
    project.write("main.aura", "fn main() { child::boom() }\n");
    project.write("child.aura", "pub fn boom() { assert(false) }\n");
    let (code, stdout, stderr) = project.run(&["run", "main.aura"]);
    assert_eq!(code, 1, "{stderr}");
    assert!(stdout.is_empty());
    assert!(stderr.contains("E4028"), "{stderr}");
    assert!(stderr.contains("child.aura:"), "{stderr}");
}

#[test]
fn filesystem_visibility_remains_resolver_owned() {
    let project = TempProject::new("visibility");
    project.write("main.aura", "fn main() { print(child::value()) }\n");
    project.write("child.aura", "fn value() -> int { return 3 }\n");
    let (code, _, stderr) = project.run(&["run", "main.aura"]);
    assert_eq!(code, 1, "{stderr}");
    assert!(stderr.contains("E2018"), "{stderr}");
    assert!(stderr.contains("main.aura:") || stderr.contains("child.aura:"));

    project.write("child.aura", "pub fn value() -> int { return 3 }\n");
    let (code, stdout, stderr) = project.run(&["run", "main.aura"]);
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(stdout, "3\n");
}

#[test]
fn filesystem_use_and_module_alias_resolve_through_existing_resolver() {
    let project = TempProject::new("use-alias");
    project.write("child.aura", "pub fn value() -> int { return 5 }\n");
    project.write(
        "main.aura",
        "use child::value\nfn main() { print(value()) }\n",
    );
    let (code, stdout, stderr) = project.run(&["run", "main.aura"]);
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(stdout, "5\n");

    project.write(
        "main.aura",
        "use child as c\nfn main() { print(c::value()) }\n",
    );
    let (code, stdout, stderr) = project.run(&["run", "main.aura"]);
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(stdout, "5\n");
}

#[test]
fn filesystem_pub_use_and_transitive_pub_use_resolve() {
    let project = TempProject::new("pub-use");
    project.write("a.aura", "pub fn value() -> int { return 4 }\n");
    project.write("b.aura", "pub use a::value\n");
    project.write("c.aura", "pub use b::value\n");
    project.write("main.aura", "fn main() { print(c::value()) }\n");

    let (code, stdout, stderr) = project.run(&["run", "main.aura"]);
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(stdout, "4\n");
}

#[test]
fn filesystem_in_source_and_external_owner_collision_is_e2020() {
    let project = TempProject::new("owner-collision");
    project.write("main.aura", "module bar { }\nfn main() { }\n");
    project.write("bar.aura", "\n");
    let (code, stdout, stderr) = project.run(&["check", "main.aura"]);
    assert_eq!(code, 1, "{stderr}");
    assert!(stdout.is_empty());
    assert!(stderr.contains("E2020"), "{stderr}");
}

#[test]
fn filesystem_direct_and_indirect_semantic_cycles_terminate() {
    let direct = TempProject::new("direct-cycle");
    direct.write("main.aura", "fn main() { print(a::even(6)) }\n");
    direct.write(
        "a.aura",
        "pub fn even(n: int) -> bool { if n == 0 { return true }\n return b::odd(n - 1) }\n",
    );
    direct.write(
        "b.aura",
        "pub fn odd(n: int) -> bool { if n == 0 { return false }\n return a::even(n - 1) }\n",
    );
    let (code, stdout, stderr) = direct.run(&["run", "main.aura"]);
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(stdout, "true\n");

    let indirect = TempProject::new("indirect-cycle");
    indirect.write("main.aura", "fn main() { print(a::step(3)) }\n");
    indirect.write(
        "a.aura",
        "pub fn step(n: int) -> int { if n == 0 { return 7 }\n return b::step(n - 1) }\n",
    );
    indirect.write(
        "b.aura",
        "pub fn step(n: int) -> int { if n == 0 { return 7 }\n return c::step(n - 1) }\n",
    );
    indirect.write(
        "c.aura",
        "pub fn step(n: int) -> int { if n == 0 { return 7 }\n return a::step(n - 1) }\n",
    );
    let (code, stdout, stderr) = indirect.run(&["run", "main.aura"]);
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(stdout, "7\n");
}

#[test]
fn filesystem_semantic_diamond_loads_shared_owner_once() {
    let project = TempProject::new("diamond");
    project.write("shared.aura", "pub fn value() -> int { return 4 }\n");
    project.write("a.aura", "pub use shared::value\n");
    project.write("b.aura", "pub use shared::value\n");
    project.write(
        "main.aura",
        "fn main() { print(a::value() + b::value()) }\n",
    );

    let (code, stdout, stderr) = project.run(&["run", "main.aura"]);
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(stdout, "8\n");
}

#[test]
fn stdin_unknown_module_is_contextualized_and_never_uses_cwd_files() {
    let project = TempProject::new("stdin-isolation");
    project.write("child.aura", "pub fn value() -> int { return 99 }\n");
    let source = "use child::value\nfn main() { print(value()) }\n";

    for command in ["check", "run"] {
        let (code, stdout, stderr) = run_stdin_in(project.path(), &[command, "-"], source);
        assert_eq!(code, 1, "{command}: {stderr}");
        assert!(stdout.is_empty(), "{command}: {stdout}");
        assert!(stderr.contains("E2019"), "{command}: {stderr}");
        assert!(
            stderr.contains("filesystem modules are unavailable for <stdin>"),
            "{command}: {stderr}"
        );
        assert!(stderr.contains("<stdin>:"), "{command}: {stderr}");
        assert!(
            !stderr.contains(project.path().to_string_lossy().as_ref()),
            "{command}: {stderr}"
        );
    }
}

// ---------------------------------------------------------------------------
// Diagnostic presentation policy (Keystone §28)
// ---------------------------------------------------------------------------

/// Run with an explicit environment for the two color variables, in `cwd`.
fn run_env_in(cwd: &Path, args: &[&str], envs: &[(&str, &str)]) -> (i32, String, String) {
    let mut cmd = aura();
    cmd.current_dir(cwd).args(args).stdin(Stdio::null());
    // Start from a clean slate for the two policy variables so a developer's
    // own shell settings cannot change the test.
    cmd.env_remove("AURA_COLOR").env_remove("NO_COLOR");
    for (k, v) in envs {
        cmd.env(k, v);
    }
    let out = cmd.output().unwrap();
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn write_type_error(cwd: &Path) {
    std::fs::write(cwd.join("color.aura"), "fn main() { let x: int = \"s\" }\n").unwrap();
}

#[test]
fn redirected_diagnostics_are_plain_by_default() {
    let p = TempProject::new("color-plain");
    write_type_error(&p.0);
    let (code, _, stderr) = run_env_in(&p.0, &["check", "color.aura"], &[]);
    assert_eq!(code, 1);
    assert!(stderr.contains("E3001"), "{stderr}");
    assert!(
        !stderr.contains('\x1b'),
        "a redirected stream is plain: {stderr:?}"
    );
    // The stable snapshot form is `file:line:col: severity[E####]: message`.
    assert!(stderr.contains("color.aura:1:13: error[E3001]"), "{stderr}");
}

#[test]
fn aura_color_always_forces_escapes() {
    let p = TempProject::new("color-always");
    write_type_error(&p.0);
    let (code, _, stderr) = run_env_in(&p.0, &["check", "color.aura"], &[("AURA_COLOR", "always")]);
    assert_eq!(code, 1);
    assert!(stderr.contains("\x1b["), "forced color: {stderr:?}");
}

#[test]
fn aura_color_never_and_no_color_disable_escapes() {
    let p = TempProject::new("color-never");
    write_type_error(&p.0);
    let (_, _, stderr) = run_env_in(&p.0, &["check", "color.aura"], &[("AURA_COLOR", "never")]);
    assert!(!stderr.contains('\x1b'));
    let (_, _, stderr) = run_env_in(&p.0, &["check", "color.aura"], &[("NO_COLOR", "1")]);
    assert!(!stderr.contains('\x1b'));
    // An explicit --color overrides NO_COLOR.
    let (_, _, stderr) = run_env_in(
        &p.0,
        &["check", "--color=always", "color.aura"],
        &[("NO_COLOR", "1")],
    );
    assert!(stderr.contains("\x1b["));
}

#[test]
fn color_is_a_presentation_flag_not_a_source_argument() {
    let p = TempProject::new("color-flag");
    write_type_error(&p.0);
    // `--color` before the path is consumed; `check` still sees one source.
    let (code, _, stderr) = run_env_in(&p.0, &["check", "--color", "never", "color.aura"], &[]);
    assert_eq!(code, 1);
    assert!(stderr.contains("E3001"));
    // `--color=never` after the path is also consumed for `check`.
    let (code, _, stderr) = run_env_in(&p.0, &["check", "color.aura", "--color=never"], &[]);
    assert_eq!(code, 1);
    assert!(stderr.contains("E3001"));
}

#[test]
fn run_program_arguments_after_the_path_are_not_consumed() {
    let p = TempProject::new("color-run-args");
    std::fs::write(p.0.join("args.aura"), "fn main() { print(args()) }\n").unwrap();
    // `--color` after the source path belongs to the program's argument list.
    let (code, stdout, _) = run_env_in(&p.0, &["run", "args.aura", "--color=always"], &[]);
    assert_eq!(code, 0);
    assert_eq!(stdout, "[\"--color=always\"]\n");
}

#[test]
fn diagnostic_identity_is_stable_across_color_policies() {
    let p = TempProject::new("color-identity");
    write_type_error(&p.0);
    let (_, _, plain) = run_env_in(&p.0, &["check", "color.aura"], &[]);
    let (_, _, colored) = run_env_in(&p.0, &["check", "color.aura"], &[("AURA_COLOR", "always")]);
    // The code and message are identical; only styling differs.
    for text in ["E3001", "is annotated as `int`"] {
        assert!(plain.contains(text), "{plain}");
        assert!(colored.contains(text), "{colored}");
    }
}

// ---------------------------------------------------------------------------
// AIS/0.1 semantic document (Keystone §36)
// ---------------------------------------------------------------------------

#[test]
fn ais_prints_a_structured_semantic_document() {
    let p = TempProject::new("ais-doc");
    p.write(
        "doc.aura",
        "struct User { name: string, email: string | none }\n\
         fn find(_ok: bool) -> User | none { return User { name: \"a\", email: none } }\n\
         fn main() { let u = find(true)\n if u != none { print(u.name) } }\n",
    );
    let (code, stdout, _) = p.run(&["ais", "doc.aura"]);
    assert_eq!(code, 0, "{stdout}");
    let doc: serde_json::Value = serde_json::from_str(&stdout).expect("AIS document is JSON");
    assert_eq!(doc["ais_version"], "0.1");
    assert_eq!(doc["capabilities"]["symbols"], true);
    // The optional field keeps its full spelling: the union is described, not
    // erased, which is the point of AIS's Keystone type coverage.
    let user = doc["symbols"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["name"] == "User")
        .expect("User symbol");
    let email = user["fields"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["name"] == "email")
        .expect("email field");
    assert!(email["type_name"].as_str().unwrap().contains("none"));
    // A clean document omits the empty diagnostics array (the schema skips
    // empty optional collections, so absence means "none"); either an omitted
    // field or an empty array is a clean document.
    if let Some(diags) = doc.get("diagnostics") {
        assert_eq!(diags.as_array().map(Vec::len), Some(0), "{stdout}");
    }
}

#[test]
fn ais_reports_structured_diagnostics_and_exit_code() {
    let p = TempProject::new("ais-diag");
    p.write("bad.aura", "fn main() { let x: int = \"s\" }\n");
    let (code, stdout, _) = p.run(&["ais", "bad.aura"]);
    assert_eq!(code, 1);
    let doc: serde_json::Value = serde_json::from_str(&stdout).expect("AIS document is JSON");
    let diag = &doc["diagnostics"][0];
    assert_eq!(diag["code_text"], "E3001");
    assert_eq!(diag["severity"], "error");
    assert!(diag["range"]["start"]["line"].is_number());
    // No ANSI escapes: presentation never crosses the protocol boundary.
    assert!(!stdout.contains('\u{1b}'));
}

#[test]
fn ais_output_grants_no_capability() {
    // The document describes the program; it must not contain host authority
    // material (paths to secrets, environment, or network state) beyond the
    // caller-supplied source name.
    let p = TempProject::new("ais-cap");
    p.write("c.aura", "fn main() { print(1) }\n");
    let (code, stdout, _) = p.run(&["ais", "c.aura"]);
    assert_eq!(code, 0);
    for forbidden in ["py_call", "read_file(\"", "API_KEY", "token", "password"] {
        assert!(!stdout.contains(forbidden), "unexpected {forbidden}");
    }
    // The only filesystem name present is the caller's own source path.
    assert!(stdout.contains("c.aura"));
}
