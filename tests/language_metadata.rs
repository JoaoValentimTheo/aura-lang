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
    // The front-end metadata describes the *browser* language surface: the
    // WebAssembly runtime's build. A native-only capability (the `http`
    // feature, which the WASM runtime deliberately does not enable) is not
    // listed, because the browser playground has no network capability and
    // must not advertise one. Every other builtin is shared and must match.
    let mut expected: Vec<String> = aura::stdlib::builtin_names()
        .into_iter()
        .filter(|name| !aura::stdlib::signatures::is_native_only(name))
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

#[test]
fn primitive_types_are_exactly_the_type_system_primitives() {
    // The four primitive type names the front end colours. These are not
    // lexer keywords (they are contextual), so they are guarded here.
    let mut expected: Vec<String> = ["bool", "float", "int", "string"]
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    expected.sort_unstable();
    assert_eq!(entries(&metadata(), "TYPES"), expected);
}

#[test]
fn contextual_words_match_the_grammar() {
    // Words that are meaningful in the grammar but not hard keywords. Keep this
    // in step with `docs/grammar.md` (module_decl, impl_decl, trait_decl,
    // const_decl, receiver).
    let mut expected: Vec<String> = ["const", "impl", "module", "self", "trait"]
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    expected.sort_unstable();
    assert_eq!(entries(&metadata(), "CONTEXTUAL"), expected);
    // A contextual word is not a hard keyword (it may still be an identifier).
    for word in &expected {
        assert!(
            !aura::lex::KEYWORDS.contains(&word.as_str()),
            "`{word}` is a hard keyword, so it must not be in CONTEXTUAL"
        );
    }
}
