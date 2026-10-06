#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Structural checks for duplicated reference grammar and token inventory.
use std::collections::BTreeSet;

/// Normalize a checked-out document to LF so the checks are byte-identical on
/// every platform. `include_str!` reads the working-tree file, and a Windows
/// checkout with `core.autocrlf` may present CRLF, which would otherwise hide
/// every `` ```ebnf `` fence. Mirrors the CRLF normalization in `tests/examples.rs`.
fn lf(s: &str) -> String {
    s.replace("\r\n", "\n")
}

#[test]
fn website_and_spec_grammar_stay_synchronized() {
    let grammar = lf(include_str!("../docs/grammar.md"));
    let site = lf(include_str!("../website/content/reference-grammar.md"));
    assert_eq!(
        grammar.split_once('\n').unwrap().1,
        site.split_once('\n').unwrap().1
    );
    let ebnf = grammar
        .split("```ebnf\n")
        .nth(1)
        .unwrap()
        .split("```")
        .next()
        .unwrap();
    let spec = lf(include_str!("../docs/LANGUAGE_SPEC.md"));
    let mut count = 0;
    for section in spec.split("```ebnf\n").skip(1) {
        let block = section.split("```").next().unwrap().trim();
        assert!(ebnf.contains(block), "normative grammar drift: {block}");
        count += 1;
    }
    assert_eq!(count, 6);
    let mut productions = BTreeSet::new();
    for line in ebnf.lines() {
        if let Some((lhs, _)) = line.split_once('=') {
            let name = lhs.trim();
            if name.chars().all(|c| c.is_ascii_lowercase() || c == '_') {
                assert!(productions.insert(name), "duplicate production {name}");
            }
        }
    }
    for required in [
        "module_decl",
        "use_decl",
        "type_params",
        "bound",
        "trait_decl",
        "method_decl",
        "receiver",
        "expr_stmt",
        "path",
        "format_type",
    ] {
        assert!(productions.contains(required), "missing {required}");
    }
}

/// The diagnostic reference on the website must be the same text as
/// `docs/errors.md` (the test-pinned authority), differing only in its first
/// heading line, exactly like the grammar pair above. A divergent website
/// copy is an authority defect: two documents answering the same question.
#[test]
fn website_and_spec_error_reference_stay_synchronized() {
    let errors = lf(include_str!("../docs/errors.md"));
    let site = lf(include_str!("../website/content/reference-errors.md"));
    assert_eq!(
        errors.split_once('\n').unwrap().1,
        site.split_once('\n').unwrap().1,
        "website reference-errors.md drifted from docs/errors.md"
    );
}

#[test]
fn every_token_variant_has_a_lexical_case() {
    let mut seen = BTreeSet::new();
    for line in include_str!("syntax_tokens.tsv")
        .lines()
        .filter(|l| !l.starts_with('#'))
    {
        let (variant, spelling) = line.split_once('\t').unwrap();
        let spelling = spelling.replace("\\n", "\n");
        let tokens = aura::lex::lex(&spelling).unwrap();
        assert_eq!(tokens.len(), if variant == "Eof" { 1 } else { 2 });
        let actual = format!("{:?}", tokens[0].tok);
        assert_eq!(actual.split('(').next().unwrap(), variant, "{spelling}");
        assert!(seen.insert(variant));
    }
    let source = include_str!("../src/lex/token.rs");
    let body = source
        .split("pub enum Tok {")
        .nth(1)
        .unwrap()
        .split("\n}")
        .next()
        .unwrap();
    let variants: BTreeSet<_> = body
        .lines()
        .map(str::trim)
        .filter(|s| !s.is_empty() && !s.starts_with('/'))
        .map(|s| s.split(['(', ',']).next().unwrap())
        .collect();
    assert_eq!(
        seen, variants,
        "update the token inventory when adding a variant"
    );
}
