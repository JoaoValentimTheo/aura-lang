#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Aura 0.0.1 scripting I/O and arguments (`docs/RELEASE_0_0_X_DESIGN.md`).
//!
//! `read_line`, `read_file`, `write_file`, `args`, and `E4020`. These exercise
//! the context-aware execution entry points directly, with in-memory input and
//! temporary files, so no test relies on repository-global files or the
//! process environment.

use std::io::Cursor;
use std::path::PathBuf;

use aura::error::codes;

/// Run a program with `args` and `stdin`, returning collected stdout.
fn run_with(src: &str, args: &[&str], stdin: &[u8]) -> Result<String, u16> {
    let args: Vec<String> = args.iter().map(|s| (*s).to_string()).collect();
    let input: Box<dyn std::io::BufRead + Send> = Box::new(Cursor::new(stdin.to_vec()));
    // Capture stdout the same way `run_source` does.
    let buf = std::sync::Arc::new(std::sync::Mutex::new(Vec::<u8>::new()));
    let sink = aura::SharedBuf(buf.clone());
    aura::execute_with(
        aura::compile_with_mode(src, aura::CompileMode::Program).map_err(|d| d.code)?,
        Some(Box::new(sink)),
        args,
        Some(input),
    )
    .map_err(|d| d.code)?;
    let text = buf.lock().unwrap().clone();
    Ok(String::from_utf8_lossy(&text).into_owned())
}

fn out(src: &str, args: &[&str], stdin: &[u8]) -> String {
    run_with(src, args, stdin).expect("program runs")
}

fn fails(src: &str, args: &[&str], stdin: &[u8]) -> u16 {
    run_with(src, args, stdin).expect_err("program is rejected")
}

/// A unique temporary directory for one test.
fn temp_dir(tag: &str) -> PathBuf {
    let mut d = std::env::temp_dir();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |t| t.as_nanos());
    d.push(format!("aura_io_{tag}_{nanos}"));
    std::fs::create_dir_all(&d).expect("create temp dir");
    d
}

// ---------------------------------------------------------------------------
// args()
// ---------------------------------------------------------------------------

#[test]
fn args_excludes_command_subcommand_and_script_path() {
    // The context receives exactly the user arguments; the command, the
    // subcommand, and the script path are excluded by the CLI before reaching
    // the interpreter.
    assert_eq!(out("fn main() { print(args()) }", &[], b""), "[]\n");
    assert_eq!(
        out("fn main() { print(args()) }", &["foo"], b""),
        "[\"foo\"]\n"
    );
    assert_eq!(
        out("fn main() { print(args()) }", &["foo", "bar"], b""),
        "[\"foo\", \"bar\"]\n"
    );
}

#[test]
fn args_preserve_order_and_unicode() {
    assert_eq!(
        out(
            "fn main() { for a in args() { print(a) } }",
            &["b", "a", "c"],
            b""
        ),
        "b\na\nc\n"
    );
    assert_eq!(
        out("fn main() { print(args()) }", &["π", "ação"], b""),
        "[\"π\", \"ação\"]\n"
    );
}

#[test]
fn args_repeated_calls_are_stable() {
    assert_eq!(
        out(
            "fn main() { print(args() == args())\n print(len(args())) }",
            &["x", "y"],
            b""
        ),
        "true\n2\n"
    );
}

// ---------------------------------------------------------------------------
// read_line()
// ---------------------------------------------------------------------------

#[test]
fn read_line_eof_and_empty_line() {
    // No input: immediate EOF.
    assert_eq!(out("fn main() { print(read_line()) }", &[], b""), "none\n");
    // A blank line is a real empty string, distinct from EOF.
    assert_eq!(
        out(
            "fn main() { let a = read_line()\n print(a == \"\")\n print(read_line()) }",
            &[],
            b"\n"
        ),
        "true\nnone\n"
    );
}

#[test]
fn read_line_multiple_lines_and_repeat() {
    assert_eq!(
        out(
            "fn main() { let a = read_line()\n let b = read_line()\n let c = read_line()\n print(a)\n print(b)\n print(c) }",
            &[],
            b"one\ntwo\nthree\n"
        ),
        "one\ntwo\nthree\n"
    );
    assert_eq!(
        out(
            "fn main() { print(read_line())\n print(read_line()) }",
            &[],
            b"only\n"
        ),
        "only\nnone\n"
    );
}

#[test]
fn read_line_strips_lf_and_crlf() {
    assert_eq!(
        out(
            "fn main() { let l = read_line()\n print(len(l)) }",
            &[],
            b"hello\n"
        ),
        "5\n"
    );
    assert_eq!(
        out(
            "fn main() { let l = read_line()\n print(len(l))\n print(l == \"hello\") }",
            &[],
            b"hello\r\n"
        ),
        "5\ntrue\n"
    );
    // A lone trailing `\r` without `\n` is data, preserved.
    assert_eq!(
        out("fn main() { print(len(read_line())) }", &[], b"ab\r"),
        "3\n"
    );
}

#[test]
fn read_line_unicode() {
    assert_eq!(
        out("fn main() { print(read_line()) }", &[], "ação\n".as_bytes()),
        "ação\n"
    );
}

#[test]
fn read_line_invalid_utf8_is_e4020() {
    assert_eq!(
        fails("fn main() { print(read_line()) }", &[], b"\xff\xfe\n"),
        codes::IO
    );
}

// ---------------------------------------------------------------------------
// read_file()
// ---------------------------------------------------------------------------

#[test]
fn read_file_existing_empty_and_unicode() {
    let dir = temp_dir("read");
    let path = dir.join("data.txt");
    std::fs::write(&path, "hello\nworld\n").unwrap();
    let empty = dir.join("empty.txt");
    std::fs::write(&empty, "").unwrap();
    let uni = dir.join("uni.txt");
    std::fs::write(&uni, "ação\n").unwrap();

    let prog = "fn main() { print(read_file(args()[0])) }";
    assert_eq!(
        out(prog, &[path.to_str().unwrap()], b""),
        "hello\nworld\n\n"
    );
    assert_eq!(out(prog, &[empty.to_str().unwrap()], b""), "\n");
    assert_eq!(out(prog, &[uni.to_str().unwrap()], b""), "ação\n\n");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn read_file_missing_is_none() {
    let dir = temp_dir("missing");
    let path = dir.join("nope.txt");
    assert_eq!(
        out(
            "fn main() { print(read_file(args()[0]) == none) }",
            &[path.to_str().unwrap()],
            b""
        ),
        "true\n"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn read_file_directory_is_e4020() {
    let dir = temp_dir("dir");
    assert_eq!(
        fails(
            "fn main() { print(read_file(args()[0])) }",
            &[dir.to_str().unwrap()],
            b""
        ),
        codes::IO
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn read_file_invalid_utf8_is_e4020() {
    let dir = temp_dir("badutf");
    let path = dir.join("bad.bin");
    std::fs::write(&path, [0xff, 0xfe, 0x00]).unwrap();
    assert_eq!(
        fails(
            "fn main() { print(read_file(args()[0])) }",
            &[path.to_str().unwrap()],
            b""
        ),
        codes::IO
    );
    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// write_file()
// ---------------------------------------------------------------------------

#[test]
fn write_file_creates_and_overwrites() {
    let dir = temp_dir("write");
    let path = dir.join("out.txt");
    let p = path.to_str().unwrap();
    let prog = "fn main() { write_file(args()[0], args()[1]) }";

    assert_eq!(out(prog, &[p, "first"], b""), "");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "first");
    // Overwrite truncates.
    assert_eq!(out(prog, &[p, "second"], b""), "");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "second");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn write_file_missing_parent_is_e4020() {
    let dir = temp_dir("wparent");
    let path = dir.join("no_such_dir").join("out.txt");
    assert_eq!(
        fails(
            "fn main() { write_file(args()[0], \"x\") }",
            &[path.to_str().unwrap()],
            b""
        ),
        codes::IO
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn write_file_directory_target_is_e4020() {
    let dir = temp_dir("wdir");
    assert_eq!(
        fails(
            "fn main() { write_file(args()[0], \"x\") }",
            &[dir.to_str().unwrap()],
            b""
        ),
        codes::IO
    );
    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// Integration
// ---------------------------------------------------------------------------

#[test]
fn stdin_filter_loop() {
    let src = "fn main() { let mut line = read_line()\n while line != none { print(line.upper())\n line = read_line() } }";
    assert_eq!(out(src, &[], b"alpha\nbeta\n"), "ALPHA\nBETA\n");
}

#[test]
fn file_transformer_round_trip() {
    let dir = temp_dir("transform");
    let input = dir.join("in.txt");
    let output = dir.join("out.txt");
    std::fs::write(&input, "a\nb\nc\n").unwrap();
    let src = "fn main() { let t = read_file(args()[0])\n let up = t.upper()\n write_file(args()[1], up) }";
    out(
        src,
        &[input.to_str().unwrap(), output.to_str().unwrap()],
        b"",
    );
    assert_eq!(std::fs::read_to_string(&output).unwrap(), "A\nB\nC\n");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn e4020_is_not_catchable_but_finally_runs() {
    let dir = temp_dir("catch");
    let bad = dir.join("no_dir").join("out.txt");
    let src = "fn main() { try { write_file(args()[0], \"x\") } catch e -> { print(\"caught\") } finally { print(\"finally\") } }";
    // The catch must NOT intercept E4020; finally still runs.
    assert_eq!(fails(src, &[bad.to_str().unwrap()], b""), codes::IO);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn catch_still_catches_explicit_throw() {
    let src =
        "fn main() { try { throw \"boom\" } catch e -> { print(e) } finally { print(\"f\") } }";
    assert_eq!(out(src, &[], b""), "boom\nf\n");
}

#[test]
fn args_are_empty_when_none_supplied() {
    assert_eq!(out("fn main() { print(args() == []) }", &[], b""), "true\n");
}
