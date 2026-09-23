#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Tests for the lexer.

use aura::lex::lex;
use aura::lex::token::Tok;

fn toks(src: &str) -> Vec<Tok> {
    lex(src)
        .expect("lex ok")
        .into_iter()
        .map(|t| t.tok)
        .collect()
}

#[test]
fn integers_and_floats() {
    assert_eq!(
        toks("1 2 3"),
        vec![Tok::Int(1), Tok::Int(2), Tok::Int(3), Tok::Eof]
    );
    assert_eq!(toks("1.5"), vec![Tok::Float(1.5), Tok::Eof]);
    assert_eq!(toks("0xff"), vec![Tok::Int(255), Tok::Eof]);
    assert_eq!(toks("0b1011"), vec![Tok::Int(11), Tok::Eof]);
    assert_eq!(toks("0o17"), vec![Tok::Int(15), Tok::Eof]);
    assert_eq!(toks("1_000"), vec![Tok::Int(1000), Tok::Eof]);
    assert_eq!(toks("1e3"), vec![Tok::Float(1000.0), Tok::Eof]);
}

#[test]
fn number_followed_by_name_is_error() {
    assert!(lex("1abc").is_err());
}

#[test]
fn strings_and_escapes() {
    assert_eq!(toks("\"hi\""), vec![Tok::Str("hi".into()), Tok::Eof]);
    assert_eq!(toks("'hi'"), vec![Tok::Str("hi".into()), Tok::Eof]);
    assert_eq!(toks(r#""a\nb""#), vec![Tok::Str("a\nb".into()), Tok::Eof]);
    assert!(lex("\"abc").is_err());
    assert!(lex(r#""\q""#).is_err());
}

#[test]
fn fstrings() {
    assert_eq!(
        toks("f\"x = {x}\""),
        vec![Tok::FStr("x = {x}".into()), Tok::Eof]
    );
}

#[test]
fn keywords() {
    assert_eq!(
        toks("let mut fn"),
        vec![Tok::Let, Tok::Mut, Tok::Fn, Tok::Eof]
    );
    assert_eq!(
        toks("none true false"),
        vec![Tok::None, Tok::True, Tok::False, Tok::Eof]
    );
}

#[test]
fn comments_are_skipped() {
    assert_eq!(
        toks("1 # comment\n2"),
        vec![Tok::Int(1), Tok::Newline, Tok::Int(2), Tok::Eof]
    );
}

#[test]
fn and_or_not_only() {
    assert!(lex("a && b").is_err());
    assert!(lex("!a").is_err());
    // `|` is a valid token (type unions); `||` is therefore two tokens and
    // is rejected later, by the parser.
    assert!(lex("a || b").is_ok());
}

#[test]
fn operators() {
    assert_eq!(
        toks("+ - * / % ^ == != < <= > >= |> += -= *= /="),
        vec![
            Tok::Plus,
            Tok::Minus,
            Tok::Star,
            Tok::Slash,
            Tok::Percent,
            Tok::Caret,
            Tok::EqEq,
            Tok::Ne,
            Tok::Lt,
            Tok::Le,
            Tok::Gt,
            Tok::Ge,
            Tok::Pipe,
            Tok::PlusEq,
            Tok::MinusEq,
            Tok::StarEq,
            Tok::SlashEq,
            Tok::Eof
        ]
    );
}
