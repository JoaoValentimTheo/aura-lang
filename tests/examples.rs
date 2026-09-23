#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Every `.aura` file under `examples/` must compile and run.
//!
//! If a sibling `.out` file exists, its contents are compared with stdout.
//! This keeps the documentation honest: examples are executable tests.

use std::fs;
use std::path::{Path, PathBuf};

fn example_files() -> Vec<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples");
    let mut out = Vec::new();
    collect(&dir, &mut out);
    out.sort();
    out
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, out);
        } else if path.extension().is_some_and(|e| e == "aura") {
            // Skip examples that need the optional Python feature; they are
            // covered by the `py`-feature integration test instead.
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name.contains("interop") {
                continue;
            }
            out.push(path);
        }
    }
}

#[test]
fn all_examples_compile_and_run() {
    let files = example_files();
    assert!(!files.is_empty(), "no examples found");
    let mut failures = Vec::new();
    for path in &files {
        let src = fs::read_to_string(path).expect("read example");
        let file = path.display().to_string();
        match aura::run_source(&src, &file) {
            Ok(actual) => {
                // Normalize line endings: on Windows, stdout uses CRLF while
                // the checked-in `.out` files use LF.
                let actual = actual.replace("\r\n", "\n");
                let expected_path = path.with_extension("out");
                if let Ok(expected) = fs::read_to_string(&expected_path) {
                    let expected = expected.replace("\r\n", "\n");
                    if actual != expected {
                        failures.push(format!(
                            "{file}: output mismatch\n--- expected ---\n{expected}\n--- actual ---\n{actual}"
                        ));
                    }
                }
            }
            Err(d) => failures.push(format!("{file}: {d}")),
        }
    }
    assert!(
        failures.is_empty(),
        "example failures:\n{}",
        failures.join("\n")
    );
}
