//! A native harness for the Playground execution engine.
//!
//! It runs the *same* `execute` engine the WebAssembly artifact exposes and
//! prints the structured JSON result, so the differential test can compare the
//! native and wasm substrates byte for byte.
//!
//! Usage:
//!   aura-playground-native <source-file> [options-file]
//!
//! The options file, when given, uses the same line protocol as the wasm ABI
//! (`arg <text>` lines and an optional `stdin-bytes <n>` block).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::io::Read;

fn main() {
    let mut args = std::env::args().skip(1);
    let source_path = args.next().unwrap_or_else(|| {
        eprintln!("usage: aura-playground-native <source-file> [options-file]");
        std::process::exit(2);
    });
    let source = std::fs::read(&source_path).expect("read source");
    let options = match args.next() {
        Some(path) => std::fs::read(&path).expect("read options"),
        None => Vec::new(),
    };
    let source = String::from_utf8_lossy(&source);
    let (json, _status, _version) = aura_playground_runtime::execute(&source, &options);
    println!("{json}");
    let _ = std::io::stdin().read(&mut [0u8; 0]);
}
