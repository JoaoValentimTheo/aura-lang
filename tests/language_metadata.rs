#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! The front-end language metadata must not drift from the compiler.
//!
//! `playground/web/language.js` is the single front-end inventory of keywords,
//! literals, types, builtins, and methods. It is presentational only; the
//! lexer and signature registry stay authoritative. This test fails when a word
//! exists in the compiler but not the front end (or vice versa), so highlighting
//! and completion can never silently go stale (`LANGUAGE_SPEC.md` §56).

fn entries(source: &str, name: &str) -> Vec<String> {
    // Find `export const NAME = [ ... ];` and extract the quoted strings.
    let start = source
        .find(&format!("export const {name} = ["))
        .unwrap_or_else(|| panic!("metadata file has no `{name}` array"));
    let rest = &source[start..];
    let end = rest.find("];").expect("array is not terminated");
    let body = &rest[..end];
    let mut out = Vec::new();
    let mut chars = body.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '"' {
            let mut s = String::new();
            for c in chars.by_ref() {
                if c == '"' {
                    break;
                }
                s.push(c);
            }
            out.push(s);
        }
    }
    out.sort_unstable();
    out
}

fn metadata() -> String {
    std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("playground/web/language.js"),
    )
    .expect("playground/web/language.js must exist")
}

#[test]
fn keywords_match_the_lexer() {
    let mut expected: Vec<String> = aura::lex::KEYWORDS
        .iter()
        .filter(|k| !matches!(**k, "true" | "false" | "none"))
        .map(|k| (*k).to_string())
        .collect();
    expected.sort_unstable();
    assert_eq!(entries(&metadata(), "KEYWORDS"), expected);
}

#[test]
fn literals_match_the_lexer() {
    let mut expected: Vec<String> = aura::lex::KEYWORDS
        .iter()
        .filter(|k| matches!(**k, "true" | "false" | "none"))
        .map(|k| (*k).to_string())
        .collect();
    expected.sort_unstable();
    assert_eq!(entries(&metadata(), "LITERALS"), expected);
}

#[test]
fn builtins_match_the_signature_registry() {
    let mut expected: Vec<String> = aura::stdlib::builtin_names()
        .into_iter()
        .map(str::to_string)
        .collect();
    expected.sort_unstable();
    expected.dedup();
    assert_eq!(entries(&metadata(), "BUILTINS"), expected);
}

#[test]
fn methods_match_the_signature_registry() {
    let mut expected: Vec<String> = aura::stdlib::signatures::methods()
        .iter()
        .map(|m| m.name.to_string())
        .collect();
    expected.sort_unstable();
    expected.dedup();
    assert_eq!(entries(&metadata(), "METHODS"), expected);
}
