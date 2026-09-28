#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Phase-1 syntax matrix. Each row names its grammar family; tests stop at parse
//! unless explicitly checking a byte boundary or pattern length semantics.
use aura::ast::{BinOp, Expr, FPart, Item, Lit};
use aura::error::{codes, Span};
use aura::lex::{lex, lex_at, token::Tok, KEYWORDS};
use aura::parse::{parse, parse_expr, parse_stmt};

#[test]
fn source_text_and_identifiers() {
    for src in ["", " \t\r", "\n", "\r\n", "# eof", "<!-- λ\0 --!>"] {
        assert!(parse(src).unwrap().items.is_empty(), "{src:?}");
    }
    for src in ["1", "1\n", "1\r\n", "1\r"] {
        assert!(matches!(
            parse_expr(src).unwrap(),
            Expr::Lit(Lit::Int(1), _)
        ));
    }
    for src in [
        "aura", "Aura", "_x", "x1", "module", "impl", "trait", "where", "const", "self",
    ] {
        assert_eq!(lex(src).unwrap()[0].tok, Tok::Ident(src.into()));
    }
    for src in [
        "\u{feff}",
        "\u{a0}",
        "\u{200b}",
        "\u{2028}",
        "\0",
        "\x0b",
        "\x0c",
        "\x7f",
        "café",
        "cafe\u{301}",
        "λ",
        "变量",
        "🐍",
    ] {
        let d = lex(src).unwrap_err();
        assert_eq!(d.code, codes::INVALID_CHAR, "{src:?}");
        assert!(src.is_char_boundary(d.span.start));
    }
    for word in KEYWORDS {
        assert!(
            !matches!(lex(word).unwrap()[0].tok, Tok::Ident(_)),
            "{word}"
        );
        assert!(parse(&format!("fn {word}() {{}}")).is_err(), "{word}");
    }
    assert_eq!(lex("1x").unwrap_err().code, codes::INVALID_NUMBER);
    assert_ne!(lex("aura").unwrap()[0].tok, lex("Aura").unwrap()[0].tok);
    assert_eq!(
        lex("'café cafe\u{301} λ 变量 🐍\0\r'").unwrap()[0].tok,
        Tok::Str("café cafe\u{301} λ 变量 🐍\0\r".into())
    );
}

#[test]
fn numeric_edges() {
    for (src, value) in [
        ("0", 0),
        ("0001", 1),
        ("1__0_", 10),
        ("0x_f_f_", 255),
        ("0b10", 2),
        ("0o10", 8),
        ("9223372036854775807", i64::MAX),
    ] {
        assert_eq!(lex(src).unwrap()[0].tok, Tok::Int(value), "{src}");
    }
    for src in [
        "0x",
        "0b2",
        "0o8",
        "0Xff",
        "1e",
        "1e+",
        "1e1_0",
        "9223372036854775809",
        "0x8000000000000000",
    ] {
        assert_eq!(lex(src).unwrap_err().code, codes::INVALID_NUMBER, "{src}");
    }
    for src in ["1.0", "1_0.5_0", "1e2", "2E-2", "1e309"] {
        assert!(matches!(lex(src).unwrap()[0].tok, Tok::Float(_)), "{src}");
    }
    assert_eq!(
        lex("9223372036854775808").unwrap()[0].tok,
        Tok::IntMinMagnitude
    );
    assert_eq!(
        parse_expr("9223372036854775808").unwrap_err().code,
        codes::INVALID_NUMBER
    );
    assert!(matches!(
        parse_expr("-9223372036854775808").unwrap(),
        Expr::Lit(Lit::Int(i64::MIN), _)
    ));
    for src in [".5", "1.", "1...2", "-(9223372036854775808)"] {
        assert!(parse_expr(src).is_err(), "{src}");
    }
}

#[test]
fn strings_require_real_closing_quotes() {
    for src in [
        "\"",
        "'",
        "f\"",
        "F'",
        "\"abc\\\"",
        "f\"abc\\\"",
        "\"abc\\",
        "f\"abc\\",
        "'abc\n'",
        "f'abc\n'",
    ] {
        assert_eq!(
            lex(src).unwrap_err().code,
            codes::UNTERMINATED_STRING,
            "{src:?}"
        );
    }
    assert_eq!(
        lex(r#""\n\t\r\0\\\"\'\{\}""#).unwrap()[0].tok,
        Tok::Str("\n\t\r\0\\\"'{}".into())
    );
}

#[test]
fn escape_and_token_spans_are_source_local() {
    let src = "  '\\q'";
    let d = lex(src).unwrap_err();
    assert_eq!(d.code, codes::INVALID_ESCAPE);
    assert_eq!(&src[d.span.start..d.span.end], "\\q");
    let d = lex_at("'\\q'", 40).unwrap_err();
    assert_eq!(d.span, Span::new(41, 43));
    let ts = lex("  # comment\n  name < int").unwrap();
    assert_eq!(ts[1].span, Span::new(14, 18));
    for prefix in ["", " ", "<!-- λ --!> "] {
        let src = format!("{prefix}f\"{{nope}}\"");
        let Expr::FStr(parts, _) = parse_expr(&src).unwrap() else {
            panic!()
        };
        let FPart::Expr(Expr::Name(_, span), _) = &parts[0] else {
            panic!()
        };
        assert_eq!(&src[span.start..span.end], "nope");
    }
    assert_eq!(lex("'\\λ'").unwrap_err().code, codes::INVALID_ESCAPE);
    assert!(matches!(lex("f'\\λ'").unwrap()[0].tok, Tok::FStr(ref s) if s == "\\λ"));
}

#[test]
fn interpolation_respects_expression_tokens() {
    for src in [
        r#"f"{'{'}""#,
        r#"f"{'}'}""#,
        r#"f"{m::value}""#,
        r#"f"{m::value:>6}""#,
        r#"f"{{{1}}}""#,
        r#"f"{{'x': 1}}""#,
        r#"f"{f(x: 1)}""#,
        r#"f"{1 <!-- } : --!> + 2}""#,
    ] {
        assert!(parse_expr(src).is_ok(), "{src}: {:?}", parse_expr(src));
    }
    for (src, code) in [
        (r#"f"{}""#, 1006),
        (r#"f"{x""#, 1004),
        (r#"f"{x=}""#, 1006),
        (r#"f"{x!r}""#, 1001),
        (r#"f"{1:.f}""#, 1006),
        (r#"f"{1:z}""#, 1006),
    ] {
        assert_eq!(parse_expr(src).unwrap_err().code, code, "{src}");
    }
}

#[test]
fn pipeline_requires_rhs_in_all_entry_points() {
    for src in ["1 |>", "1 |>\n", "1 |>\r\n", "1 |> # eof", "1 |>\nnext"] {
        assert_eq!(
            parse_expr(src).unwrap_err().code,
            codes::EXPECTED,
            "{src:?}"
        );
        assert_eq!(parse(src).unwrap_err().code, codes::EXPECTED, "{src:?}");
        assert_eq!(
            parse_stmt(src).unwrap_err().code,
            codes::EXPECTED,
            "{src:?}"
        );
    }
    assert!(parse_expr("1 |> <!--\n--!> next").is_ok());
    assert!(parse_expr("1 |> next").is_ok());
    assert!(parse_expr("1\n|> next").is_err());
}

#[test]
fn delimiter_comma_matrix() {
    for src in [
        "fn f(a,) {}",
        "fn f(\nmut a: int,\n) {}",
        "fn f<T,>() {}",
        "struct S<T,> { pub x: T, }",
        "enum E<T,> { A(T,), B, }",
        "type X = Box<int,>",
        "type X = Box<Box<int,>>",
        "trait T { fn m(self,) }",
        "impl S { fn m(mut self,) {} }",
    ] {
        assert!(parse(src).is_ok(), "{src}: {:?}", parse(src));
    }
    for src in [
        "f(1,)",
        "[1,\n]",
        "(1,\n)",
        "{'x': 1,\n}",
        "S { x: 1,\n}",
        "S(1,)",
        "(x,) -> x",
        "fn(\nx,\n) -> x",
        "match x { A(a,) -> a, }",
        "match x { [a,\n] -> a; }",
    ] {
        assert!(parse_expr(src).is_ok(), "{src}: {:?}", parse_expr(src));
    }
    for src in ["let A(a,) = x", "let [a,\n] = x"] {
        assert!(parse_stmt(src).is_ok(), "{src}");
    }
    for src in ["f(,)", "[1,,]", "(1,,)", "{:,}", "S {,}", "match x {,}"] {
        assert!(parse_expr(src).is_err(), "{src}");
    }
}

#[test]
fn generic_adjacency_is_real_source_adjacency() {
    assert!(matches!(parse_expr("f<int>(x)").unwrap(), Expr::Call(_,_,args,_) if args.len()==1));
    for src in ["f <int>(x)", "f<!-- comment --!><int>(x)"] {
        assert!(
            matches!(parse_expr(src).unwrap(), Expr::Binary(BinOp::Gt, _, _, _)),
            "{src}"
        );
    }
}

#[test]
fn grammar_family_matrix() {
    // Positive/negative pairs exercise syntax only; name resolution is Phase 2.
    for (family, good, bad) in [
        ("module", "pub module m { module n {} }", "module m {"),
        ("use", "pub use a.b::c as d", "use a:: as d"),
        (
            "fn",
            "pub fn f<T: A + B>(mut x: T,) -> T { x }",
            "fn f(mut mut x) {}",
        ),
        (
            "struct",
            "pub struct S<T> { pub x: T, }",
            "struct S { pub pub x: int }",
        ),
        ("enum", "pub enum E<T> { A(T,), B, }", "enum E { pub A }"),
        (
            "alias",
            "pub type X<T> = {string: [T | none]}",
            "type X = int |",
        ),
        (
            "trait",
            "pub trait T<A> { pub fn m<B>(self: A, x: B) -> B }",
            "trait T { fn m() }",
        ),
        (
            "impl",
            "impl<A> T<A> for m::S<A> { pub fn m(self) {} }",
            "pub impl S {}",
        ),
        ("const", "pub const Name: int = 1", "const name = 1"),
        ("top-let", "pub let x: int = 1", "let mut x = 1"),
        ("visibility", "pub fn f() {}", "pub pub fn f() {}"),
        (
            "nested-item",
            "module m { fn f() {} }",
            "fn f() { fn g() {} }",
        ),
        (
            "type-args",
            "type X = m::S<[int], {string: int | none}>",
            "type X = int<bool>",
        ),
        (
            "bound",
            "fn f<T: A<int> + B>(x: T) {}",
            "fn f<T: A +>(x: T) {}",
        ),
    ] {
        for source in [good.to_string(), good.replace('\n', "\r\n")] {
            assert!(
                parse(&source).is_ok(),
                "{family}: {source}: {:?}",
                parse(&source)
            );
        }
        assert!(parse(bad).is_err(), "{family}: {bad}");
    }
}

#[test]
fn statements_patterns_and_types_matrix() {
    for (good, bad) in [
        ("let mut x: int = 1", "let mut [x] = xs"),
        ("let [A(x), _] = xs", "let [1] = xs"),
        ("x[0] += 1", "x ="),
        ("return 1", "throw"),
        ("while x { break }", "while x"),
        ("loop { continue }", "loop"),
        ("for [x] in xs {}", "for x xs {}"),
        ("try {} catch e {} finally {}", "try {} finally {}"),
        (
            "match x { m::A(a) if a -> a, _ -> none }",
            "match x { [a, ..b] -> a }",
        ),
        ("let f = (mut x: int,) -> x", "let f = (x=1) -> x"),
        ("let x: Box<[int | none]> = v", "let x: fn(int) -> int = v"),
    ] {
        assert!(parse_stmt(good).is_ok(), "{good}");
        assert!(parse_stmt(bad).is_err(), "{bad}");
    }
    for src in [
        "match x { -1 -> 0 }",
        "match x { 1.5 -> 0 }",
        "match x { S { x: a } -> a }",
        "let m::A(x) = v",
        "let x: int? = 1",
        "use a::{b,c}",
    ] {
        assert!(parse_stmt(src).is_err(), "{src}");
    }
}

#[test]
fn comments_and_newlines() {
    assert_eq!(
        lex("1<!--\n--!>+2 #eof")
            .unwrap()
            .iter()
            .filter(|t| t.tok == Tok::Newline)
            .count(),
        0
    );
    assert_eq!(lex("# a\rb\n1").unwrap()[0].tok, Tok::Newline);
    assert_eq!(lex("<!-- a <!-- b --!>").unwrap()[0].tok, Tok::Eof);
    assert_eq!(
        lex("<!-- -->").unwrap_err().code,
        codes::UNTERMINATED_COMMENT
    );
    for src in [
        "if x {}\nelse {}",
        "try {}\ncatch e {}",
        "try {} catch e {}\nfinally {}",
    ] {
        assert!(parse_stmt(src).is_err());
    }
    // CONF-PARSE-8: preserve observations while separator policy is unresolved.
    assert!(parse("1 2").is_ok());
    assert!(parse_stmt("{ let x = 1 let y = 2 }").is_ok());
    assert!(parse_stmt("{;;}").is_ok());
}

fn shape(e: &Expr) -> String {
    match e {
        Expr::Name(n, _) => n.clone(),
        Expr::Binary(op, l, r, _) => format!("({op:?} {} {})", shape(l), shape(r)),
        Expr::Range(l, r, _) => format!("(range {} {})", shape(l), shape(r)),
        Expr::Unary(op, x, _) => format!("({op:?} {})", shape(x)),
        Expr::Call(f, a, _, _) => format!(
            "(call {} {})",
            shape(f),
            a.iter()
                .map(|a| shape(&a.value))
                .collect::<Vec<_>>()
                .join(" ")
        ),
        Expr::Pipe(l, r, _) => format!("(pipe {} {})", shape(l), shape(r)),
        other => panic!("unexpected {other:?}"),
    }
}

#[test]
fn every_precedence_boundary_and_associativity() {
    for (src, expected) in [
        ("a |> b or c", "(pipe a (Or b c))"),
        ("a or b and c", "(Or a (And b c))"),
        ("a and b == c", "(And a (Eq b c))"),
        ("a == b < c", "(Eq a (Lt b c))"),
        ("a < b | c", "(Lt a (BitOr b c))"),
        ("a | b & c", "(BitOr a (BitAnd b c))"),
        ("a & b << c", "(BitAnd a (Shl b c))"),
        ("a << b..c", "(Shl a (range b c))"),
        ("a..b + c", "(range a (Add b c))"),
        ("a + b * c", "(Add a (Mul b c))"),
        ("a * b ^ c", "(Mul a (Pow b c))"),
        ("-a ^ b", "(Pow (Neg a) b)"),
        ("-f(a)", "(Neg (call f a))"),
        ("a..b..c", "(range a (range b c))"),
        ("a ^ b ^ c", "(Pow a (Pow b c))"),
        ("a - b - c", "(Sub (Sub a b) c)"),
        ("a < b < c", "(Lt (Lt a b) c)"),
        ("a |> f |> g", "(call g (call f a))"),
        ("a |> f() + b", "(pipe a (Add (call f ) b))"),
    ] {
        assert_eq!(shape(&parse_expr(src).unwrap()), expected, "{src}");
    }
    assert!(parse_expr("a = b").is_err());
    assert!(matches!(
        parse_expr("a.b(c)[d].e").unwrap(),
        Expr::Field(_, _, _)
    ));
}

#[test]
fn diagnostics_are_deterministic_and_bounded() {
    for src in [
        "fn",
        "module x {",
        "struct S { x }",
        "enum E { A( }",
        "type A =",
        "trait T { fn m() }",
        "impl S { let x = 1 }",
        "use",
        "const X",
        "1 +",
        "f(x: 1, 2)",
        "f\"{ }\"",
        "'\\q'",
        "match x { [ -> 1 }",
        "fn f() { try {} }",
    ] {
        let a = parse(src).unwrap_err();
        let b = parse(src).unwrap_err();
        assert_eq!((a.code, &a.message, a.span), (b.code, &b.message, b.span));
        assert!(
            a.span.start <= a.span.end && a.span.end <= src.len(),
            "{src}: {a:?}"
        );
    }
}

#[test]
fn list_patterns_match_exact_length() {
    for (xs, expected) in [("[]", "0\n"), ("[1]", "1\n"), ("[1,2]", "0\n")] {
        assert_eq!(
            aura::run_source(
                &format!("fn main() {{ print(match {xs} {{ [x] -> 1, _ -> 0 }}) }}"),
                "matrix"
            )
            .unwrap(),
            expected
        );
    }
}

#[test]
fn entry_points_have_distinct_documented_roots() {
    assert!(parse("let mut x = 1").is_err());
    assert!(parse_stmt("let mut x = 1").is_ok());
    assert!(parse_expr("let mut x = 1").is_err());
    assert!(matches!(
        parse("fn f() {}").unwrap().items[0],
        Item::Fn { .. }
    ));
    assert!(parse_stmt("fn f() {}").is_err());
    assert!(parse_expr("fn x -> x").is_ok());
    assert!(parse_expr("1; 2").is_err());
}

#[test]
fn return_accepts_every_unary_expression() {
    let aura::ast::Stmt::Return(Some(Expr::Unary(aura::ast::UnOp::BitNot, _, _)), _) =
        parse_stmt("return ~0").unwrap()
    else {
        panic!("return lost its operand")
    };
}

#[test]
fn annotated_block_is_not_misclassified_as_a_map() {
    for src in [
        "{ let x: int = 1; x }",
        "{ print(1); let x: int = 2; x }",
        "{ let f = (x: int) -> x; f(1) }",
    ] {
        assert!(
            matches!(parse_expr(src).unwrap(), Expr::Block(_, _)),
            "{src}"
        );
    }
}

#[test]
fn byte_decoder_rejects_invalid_utf8_everywhere() {
    for src in [
        b"\xff".as_slice(),
        b"'\xff'",
        b"#\xff",
        b"<!--\xff--!>",
        b"'\xe2\x82",
    ] {
        let d = aura::lex::decode_source(src).unwrap_err();
        assert_eq!(d.code, codes::INVALID_CHAR);
        assert!(d.span.end <= src.len());
    }
}

#[test]
fn interpolation_format_fill_and_unicode_whitespace() {
    for src in [r#"f"{1:'>6}""#, r#"f"{1:#>6}""#, r#"f"{ {'x': 1} }""#] {
        assert!(parse_expr(src).is_ok(), "{src}");
    }
    assert_eq!(
        parse_expr("f\"{\u{a0}x}\"").unwrap_err().code,
        codes::INVALID_CHAR
    );
}

#[cfg(feature = "cli")]
#[test]
fn cli_rejects_invalid_utf8_in_file_and_stdin() {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let path = std::env::temp_dir().join(format!("aura-invalid-utf8-{}.aura", std::process::id()));
    std::fs::write(&path, b"# invalid byte: \xff").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_aura"))
        .arg("check")
        .arg(&path)
        .output()
        .unwrap();
    std::fs::remove_file(path).unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8(out.stderr).unwrap().contains("E1001"));
    let mut child = Command::new(env!("CARGO_BIN_EXE_aura"))
        .args(["check", "-"])
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(b"'\xff'").unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8(out.stderr).unwrap().contains("E1001"));
}
