#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Integer boundary tests.
//!
//! Invariant: no integer operation may panic or silently wrap. Every result
//! is either a correct value or a stable diagnostic.

use aura::error::codes;
use aura::run_source;

/// Build `i64::MIN` at runtime without relying on a negative literal (which
/// the lexer rejects as out of range).
const MIN: &str = "-9223372036854775807 - 1";
const MAX: &str = "9223372036854775807";

fn run(src: &str) -> Result<String, u16> {
    run_source(src, "<boundary>").map_err(|d| d.code)
}

#[test]
fn min_div_neg_one_overflows() {
    let src = format!("fn main() {{ let m = {MIN}\n print(m / -1) }}");
    assert_eq!(run(&src), Err(codes::OVERFLOW));
}

#[test]
fn min_rem_neg_one_overflows() {
    let src = format!("fn main() {{ let m = {MIN}\n print(m % -1) }}");
    assert_eq!(run(&src), Err(codes::OVERFLOW));
}

#[test]
fn max_plus_one_overflows() {
    let src = format!("fn main() {{ print({MAX} + 1) }}");
    assert_eq!(run(&src), Err(codes::OVERFLOW));
}

#[test]
fn min_minus_one_overflows() {
    let src = format!("fn main() {{ let m = {MIN}\n print(m - 1) }}");
    assert_eq!(run(&src), Err(codes::OVERFLOW));
}

#[test]
fn min_negation_overflows() {
    let src = format!("fn main() {{ let m = {MIN}\n print(-m) }}");
    assert_eq!(run(&src), Err(codes::OVERFLOW));
}

#[test]
fn extreme_range_length_saturates() {
    // len() of an enormous range must not overflow; it saturates.
    let src = format!("fn main() {{ print(len(range({MIN}, {MAX}))) }}");
    assert_eq!(run(&src), Ok(format!("{MAX}\n")));
}

#[test]
fn extreme_negative_index_does_not_panic() {
    let src = format!("fn main() {{ let xs = [1]\n print(xs[{MIN}]) }}");
    assert_eq!(run(&src), Err(codes::INDEX));
}

#[test]
fn index_out_of_range_uses_index_code() {
    assert_eq!(run("fn main() { print([1, 2][5]) }"), Err(codes::INDEX));
    assert_eq!(run("fn main() { print(\"ab\"[9]) }"), Err(codes::INDEX));
}

#[test]
fn division_by_zero_is_div_zero() {
    assert_eq!(run("fn main() { print(1 / 0) }"), Err(codes::DIV_ZERO));
    assert_eq!(run("fn main() { print(1 % 0) }"), Err(codes::DIV_ZERO));
}

#[test]
fn float_boundaries() {
    // Division by zero on floats is also a language-level error (not IEEE).
    assert_eq!(run("fn main() { print(1.0 / 0.0) }"), Err(codes::DIV_ZERO));
    assert_eq!(run("fn main() { print(0.0 / 0.0) }"), Err(codes::DIV_ZERO));
}

#[test]
fn assert_uses_its_own_code() {
    assert_eq!(
        run("fn main() { assert(1 == 2, \"math broke\") }"),
        Err(codes::ASSERT)
    );
}

#[test]
fn deeply_nested_expression_is_rejected_not_a_crash() {
    // A flat but deeply nested expression must never overflow the host stack
    // in the parser, the checker, or the evaluator.
    let src = format!("fn main() {{ print(1{}) }}", "+1".repeat(100_000));
    assert_eq!(run(&src), Err(codes::NESTING));
}

#[test]
fn deeply_nested_checker_is_rejected_not_a_crash() {
    // The parser rejects a tree deeper than the language limit, so the
    // checker is never asked to walk one on an arbitrary stack.
    let src = format!("fn main() {{ print(1{}) }}", "+1".repeat(10_000));
    let err = aura::parse::parse(&src).expect_err("must be bounded");
    assert_eq!(err.code, codes::NESTING);
}

#[test]
fn deep_call_recursion_uses_the_call_limit_not_the_nesting_limit() {
    // Expression nesting resets per call, so deep recursion is governed by
    // E4011, never by E1015.
    let src = "fn f(n) { return f(n + 1) }\nfn main() { f(0) }";
    assert_eq!(run(src), Err(codes::RECURSION));
}

#[test]
fn moderately_long_expression_is_accepted() {
    // Ordinary chains must still work.
    let src = format!("fn main() {{ print({}) }}", "1+".repeat(100) + "1");
    assert_eq!(run(&src), Ok("101\n".to_string()));
}

/// B2: the semantic nesting limit is measured on the AST and reported as
/// `E1015` at a deterministic boundary. A program at the limit runs; a
/// program beyond it is rejected, never a host crash.
#[test]
fn regression_nesting_limit_boundary() {
    // A flat chain is left-nested AST depth. 128 terms is well within the
    // limit; 256 is past it.
    let ok = format!("fn main() {{ print({}) }}", vec!["1"; 128].join("+"));
    assert!(run(&ok).is_ok(), "128-term chain must be accepted");
    let over = format!("fn main() {{ print({}) }}", vec!["1"; 256].join("+"));
    assert_eq!(run(&over), Err(codes::NESTING));

    // Nested collections count AST depth too: just under the limit succeeds,
    // well past it is rejected with E1015 (not E1006, not a crash).
    let under = format!(
        "fn main() {{ print({}1{}) }}",
        "[".repeat(200),
        "]".repeat(200)
    );
    assert!(run(&under).is_ok(), "200 nested lists must be accepted");
    let over = format!(
        "fn main() {{ print({}1{}) }}",
        "[".repeat(300),
        "]".repeat(300)
    );
    assert_eq!(run(&over), Err(codes::NESTING));
}

/// B2: grouping parentheses add no AST depth, so a long parenthesized chain is
/// accepted; only an extreme one hits the parser's host-safety backstop, and
/// it too reports `E1015` rather than overflowing the stack.
#[test]
fn regression_parenthesis_nesting_is_bounded_not_a_crash() {
    let ok = format!(
        "fn main() {{ print({}1{}) }}",
        "(".repeat(1000),
        ")".repeat(1000)
    );
    assert!(
        run(&ok).is_ok(),
        "1000 grouped parentheses must be accepted"
    );
    let extreme = format!(
        "fn main() {{ print({}1{}) }}",
        "(".repeat(4000),
        ")".repeat(4000)
    );
    assert_eq!(run(&extreme), Err(codes::NESTING));
}

/// FEATURE_002 stack hardening: the checker runs with enough native stack that
/// a valid program at the semantic nesting limit is accepted, and one past it
/// is rejected with `E1015` — a language diagnostic, never a native overflow.
/// This guards the invariant that a larger native stack does not change the
/// language limit.
#[test]
fn checker_survives_the_semantic_nesting_limit() {
    // Just below the limit: accepted (the CLI default stack must not overflow).
    let under = format!(
        "fn main() {{ print({}1{}) }}",
        "[".repeat(250),
        "]".repeat(250)
    );
    assert!(run(&under).is_ok(), "250 nested lists must be accepted");
    // At/over the limit: a deterministic diagnostic, not a crash.
    let over = format!(
        "fn main() {{ print({}1{}) }}",
        "[".repeat(256),
        "]".repeat(256)
    );
    assert_eq!(run(&over), Err(codes::NESTING));
    // A deeply nested call chain (Feature 002 argument path) is likewise
    // bounded by the same diagnostic.
    let calls = format!(
        "fn id(x) {{ return x }}\nfn main() {{ print({}1{}) }}",
        "id(".repeat(300),
        ")".repeat(300)
    );
    assert_eq!(run(&calls), Err(codes::NESTING));
}

/// FEATURE_004: a deeply nested `let` pattern is bounded by the parser
/// recursion backstop and reports `E1015`, never a host overflow.
#[test]
fn deep_destructuring_pattern_is_bounded() {
    // A pattern within the backstop parses and checks (the undefined `v` is a
    // checker diagnostic, not a nesting one).
    let under = format!(
        "fn main() {{ let v = 1\n let {}x{} = v }}",
        "[".repeat(200),
        "]".repeat(200)
    );
    assert!(run(&under).is_err());
    // Far past the backstop, the pattern is rejected with the nesting code.
    let over = format!(
        "fn main() {{ let v = 1\n let {}x{} = v }}",
        "[".repeat(3000),
        "]".repeat(3000)
    );
    assert_eq!(run(&over), Err(codes::NESTING));
}

/// FEATURE_005: the empty-map literal `{:}` adds no AST nesting, so a program
/// using it obeys the same depth limits as any other.
#[test]
fn empty_map_literal_adds_no_nesting() {
    // A long chain of `{:}`-valued operators stays within the limit.
    let expr = vec!["{:} == {:}"; 50].join(" == ");
    let within = format!("fn main() {{ print({expr}) }}");
    assert!(run(&within).is_ok());
    // And a deeply nested grouping still reports the existing nesting limit.
    let over = format!(
        "fn main() {{ print({}1{}) }}",
        "(".repeat(5000),
        ")".repeat(5000)
    );
    assert_eq!(run(&over), Err(codes::NESTING));
}

/// H1b: a deeply nested `for`/`match` pattern is bounded by the parser
/// recursion backstop and reports `E1015`, never a host stack overflow.
#[test]
fn deep_match_pattern_is_bounded() {
    let over = format!(
        "fn main() {{ let v = 1\n match v {{ {}x{} -> 1\n _ -> 0 }} }}",
        "[".repeat(5000),
        "]".repeat(5000)
    );
    assert_eq!(run(&over), Err(codes::NESTING));
    let over_for = format!(
        "fn main() {{ let xs = []\n for {}x{} in xs {{ }} }}",
        "[".repeat(5000),
        "]".repeat(5000)
    );
    assert_eq!(run(&over_for), Err(codes::NESTING));
}

/// H1b: a pattern within the parser backstop parses, checks, and executes
/// without a host overflow (runtime pattern recursion is bounded by the
/// parser-accepted depth and runs on the large interpreter stack).
#[test]
fn bounded_deep_pattern_executes_without_overflow() {
    let n = 1000usize;
    let src = format!(
        "fn id(v) {{ return v }}\nfn main() {{ let mut x = id(1)\n \
         for _i in range(0, {n}) {{ x = id([x]) }}\n \
         let r = match x {{ {pat} _ {close} -> \"m\"\n _ -> \"f\" }}\n print(r) }}",
        pat = "[".repeat(n),
        close = "]".repeat(n),
    );
    assert_eq!(run(&src), Ok("m\n".to_string()));
}

/// FEATURE_006: a deep `else if` chain nests existing `Expr::If` nodes and is
/// bounded by the existing AST nesting limit (`E1015`), never a host
/// overflow. The exact off-by-one is measured against real behavior: a modest
/// chain runs, and a clearly-over chain is rejected with `E1015`.
#[test]
fn deep_else_if_chain_is_bounded() {
    let chain = |n: usize| {
        let mut parts = vec!["if false { print(0) }".to_string()];
        for i in 1..n {
            parts.push(format!("else if false {{ print({i}) }}"));
        }
        parts.push("else { print(99) }".to_string());
        format!("fn main() {{ {} }}", parts.join(" "))
    };
    // A modest chain is accepted and evaluates to the final `else`.
    assert_eq!(run(&chain(100)), Ok("99\n".to_string()));
    // A clearly-over chain is rejected with the existing nesting diagnostic.
    assert_eq!(run(&chain(2000)), Err(codes::NESTING));
}

/// Host gate: the parser recursion backstop is an implementation-safety limit,
/// distinct from the semantic AST limit (256). On the native substrate the
/// backstop is 2048, so a chain of grouping parentheses well past the AST
/// limit is accepted when it stays under the backstop, and one past it is
/// `E1015` — never a host overflow.
#[test]
fn parser_recursion_backstop_is_distinct_from_ast_limit() {
    // 1000 grouping parentheses add no AST depth and stay under the native
    // backstop of 2048.
    let ok = format!(
        "fn main() {{ print({}1{}) }}",
        "(".repeat(1000),
        ")".repeat(1000)
    );
    assert_eq!(run(&ok), Ok("1\n".to_string()));

    // Past the backstop, the same construct is the stable nesting diagnostic.
    let over = format!(
        "fn main() {{ print({}1{}) }}",
        "(".repeat(3000),
        ")".repeat(3000)
    );
    assert_eq!(run(&over), Err(codes::NESTING));
}

/// The published budget is stable per substrate: native is 2048.
#[test]
fn native_parse_recursion_budget_is_2048() {
    assert_eq!(aura::parse::parse_recursion_budget(), 2048);
}

/// WASM validation gate: the parser backstop must not be so small that it
/// rejects programs the semantic AST limit permits. A program at the AST limit
/// costs ~512 parser frames, and any grouping adds to that, so the WASM
/// budget must exceed 512. This test runs on both substrates and pins the
/// invariant that the budget is greater than the AST-implied frame need.
#[test]
fn parser_backstop_covers_the_ast_limit_on_every_substrate() {
    // The most frame-expensive AST-valid shape: a maximal nested collection
    // wrapped in grouping parentheses. It must parse on every substrate. Its
    // display is the nested brackets themselves.
    let nested_display = format!("{}1{}", "[".repeat(250), "]".repeat(250));

    let ast_max = format!(
        "fn main() {{ print({}1{}) }}",
        "[".repeat(250),
        "]".repeat(250)
    );
    assert_eq!(run(&ast_max), Ok(format!("{nested_display}\n")));

    let grouped = format!(
        "fn main() {{ print({}{}1{}{}) }}",
        "[".repeat(250),
        "(".repeat(50),
        ")".repeat(50),
        "]".repeat(250)
    );
    assert_eq!(
        run(&grouped),
        Ok(format!("{nested_display}\n")),
        "an AST-valid program with grouping must not be rejected by the parser backstop"
    );

    // A budget below the AST-implied peak would be an observable divergence.
    assert!(
        aura::parse::parse_recursion_budget() > 512,
        "the parser backstop must exceed the AST-implied frame need (~512)"
    );
}

/// The semantic AST limit (256) is unchanged by the host work: a program past
/// it is still `E1015`, and the parser backstop is separate.
#[test]
fn semantic_ast_limit_is_unchanged_by_the_host_gate() {
    let over = format!("fn main() {{ print({}) }}", vec!["1"; 256].join("+"));
    assert_eq!(run(&over), Err(codes::NESTING));
}

/// A chain of unary operators recurses through `unary()` directly, so it must
/// consume the parser's host-safety backstop exactly like grouping and types.
/// Before `CONF-RESOURCE-1` a long chain (`-`/`not`/`~`) exhausted the native
/// stack and aborted with no diagnostic; it now reports the stable `E1015` on
/// every substrate (`LANGUAGE_SPEC.md` §31.5).
#[test]
fn unary_chains_are_bounded_not_a_stack_overflow() {
    for op in ["-", "not ", "~"] {
        let chain = |n: usize| format!("fn main() {{ print({}{}1) }}", op.repeat(n), "");
        // A modest chain is a real expression.
        let small = format!("fn main() {{ print({}{}1) }}", op.repeat(50), "");
        assert!(run(&small).is_ok(), "{op}: 50 unary operators must parse");
        // A clearly-over chain is the stable nesting diagnostic, never a trap.
        assert_eq!(run(&chain(20_000)), Err(codes::NESTING), "{op}");
    }
}

/// `LANGUAGE_SPEC.md` §31.2: the parser backstop must accept any AST-valid
/// program *including grouping*. A unary chain is one AST level per operator,
/// so it is bounded by the semantic AST counter, not by the host-stack budget
/// that grouping also consumes. A valid program that mixes deep grouping with
/// a unary chain well within the AST limit must be accepted; the two
/// mechanisms must compose without shrinking each other's allowance
/// (`CONF-RESOURCE-1` refinement).
#[test]
fn unary_and_grouping_budgets_compose() {
    // 1000 grouping parentheses (well within the host backstop) plus a unary
    // chain under `MAX_AST_DEPTH` is AST-valid and must parse.
    let ok = format!(
        "fn main() {{ print({}{}1{}) }}",
        "(".repeat(1000),
        "-".repeat(253),
        ")".repeat(1000)
    );
    assert!(
        run(&ok).is_ok(),
        "grouping + sub-limit unary must be accepted"
    );
    // A unary chain past the AST limit is the stable nesting diagnostic.
    let over = format!("fn main() {{ print({}1) }}", "-".repeat(400));
    assert_eq!(run(&over), Err(codes::NESTING));
    // A very long chain is still bounded, never a host abort.
    let huge = format!("fn main() {{ print({}1) }}", "-".repeat(30_000));
    assert_eq!(run(&huge), Err(codes::NESTING));
}

/// Nested in-source `module` blocks recurse through the parser's `item()`, so
/// the descent must consume the host-safety backstop exactly like nested types
/// and expressions. Before `CONF-RESOURCE-2` ~20,000 nested modules aborted
/// the process with a native stack overflow and no diagnostic; now they report
/// the stable `E1015` (`LANGUAGE_SPEC.md` §31.2, §31.5).
#[test]
fn nested_modules_are_bounded_not_a_stack_overflow() {
    // A moderately nested module is accepted and resolves. Items are
    // separated by newlines (CONF-PARSE-8): a real separator is required
    // between items, so the block bodies carry newlines rather than relying
    // on adjacency before `}`.
    let nested = format!(
        "{}pub fn f() -> int {{\n return 1\n}}{}",
        "module A {\n".repeat(50),
        "\n}".repeat(50)
    );
    assert!(run(&nested).is_ok(), "50 nested modules must be accepted");
    // Module nesting counts toward the semantic limit on every substrate
    // (ADR-0004): 256 levels is accepted, 257 is `E1015`.
    let at_limit = format!(
        "{}pub fn f() -> int {{\n return 1\n}}{}\nfn main() {{ print(1) }}\n",
        "module A {\n".repeat(256),
        "\n}".repeat(256)
    );
    assert!(
        run(&at_limit).is_ok(),
        "256 nested modules must be accepted"
    );
    let over_limit = format!(
        "{}pub fn f() -> int {{\n return 1\n}}{}\nfn main() {{ print(1) }}\n",
        "module A {\n".repeat(257),
        "\n}".repeat(257)
    );
    assert_eq!(run(&over_limit), Err(codes::NESTING));
    // A deeply nested module chain is the same stable diagnostic (never a host
    // abort), well past the semantic limit.
    let over = format!("{}{}", "module A {\n".repeat(20_000), "\n}".repeat(20_000));
    assert_eq!(run(&over), Err(codes::NESTING));
    // A flat set of many sibling modules is accepted (no depth).
    let mut flat = String::new();
    for i in 0..2000 {
        use std::fmt::Write as _;
        let _ = write!(
            flat,
            "module m{i} {{\n pub fn f() -> int {{\n return {i}\n }}\n}}\n"
        );
    }
    assert!(run(&format!("{flat}\nfn main() {{ print(m1999::f()) }}")).is_ok());
}

/// `LANGUAGE_SPEC.md` §31.1 + ADR-0004: structural `TypeExpr` nesting counts
/// toward the same `MAX_AST_DEPTH = 256` semantic budget as every other AST
/// node, so a type annotation at the limit is accepted and one past it is the
/// stable `E1015` on *every* substrate. Before ADR-0004 the parser did not
/// descend `TypeExpr` nodes, so type nesting was bounded only by the
/// substrate-calibrated parser backstop (native 2047 accepted / 2048 rejected;
/// WASM 767 / 768), a native/WASM acceptance divergence. This test pins the
/// unified boundary at N-1/N/N+1.
#[test]
fn type_nesting_counts_toward_the_semantic_ast_limit() {
    // A generic application nests structurally: `Box<Box<...<int>>>`.
    let type_src = |n: usize| {
        let mut t = "int".to_string();
        for _ in 0..n {
            t = format!("Box<{t}>");
        }
        format!("struct Box<T> {{ value: T }}\nfn f(_x: {t}) -> int {{ return 1 }}\nfn main() {{ print(1) }}")
    };
    // The `int` atom occupies the deepest level, so `n` generic wraps is
    // `n + 1` AST levels. "At most 256 levels" therefore accepts 255 wraps and
    // rejects 256 (the parity this test pins at N-1/N/N+1 levels).
    assert!(
        run(&type_src(254)).is_ok(),
        "255 AST levels must be accepted"
    );
    assert!(
        run(&type_src(255)).is_ok(),
        "256 AST levels (the limit) must be accepted"
    );
    assert_eq!(
        run(&type_src(256)),
        Err(codes::NESTING),
        "257 AST levels must be `E1015`"
    );

    // A *flat* union is a loop over members, not nesting, so a long union is
    // accepted: ADR-0004 must not penalize union breadth per member.
    let defs = (0..4000)
        .map(|i| format!("type T{i} = int"))
        .collect::<Vec<_>>()
        .join("\n");
    let members = (0..4000)
        .map(|i| format!("T{i}"))
        .collect::<Vec<_>>()
        .join(" | ");
    let flat = format!("{defs}\ntype All = {members}\nfn main() {{ let _: All = 1 }}");
    assert!(
        run(&flat).is_ok(),
        "a flat 4000-member union must stay accepted"
    );
}

/// ADR-0004 also covers nesting inside other annotation positions: a list and
/// a map nest the same way a generic application does and must be bounded.
#[test]
fn list_and_map_type_nesting_are_bounded() {
    let list_src = |n: usize| {
        let mut t = "int".to_string();
        for _ in 0..n {
            t = format!("[{t}]");
        }
        format!("fn f(_x: {t}) -> int {{ return 1 }}\nfn main() {{ print(1) }}")
    };
    assert!(run(&list_src(255)).is_ok());
    assert_eq!(run(&list_src(300)), Err(codes::NESTING));

    let map_src = |n: usize| {
        let mut t = "int".to_string();
        for _ in 0..n {
            t = format!("{{string: {t}}}");
        }
        format!("fn main() {{ let _: {t} = {{}} }}")
    };
    assert!(run(&map_src(100)).is_ok());
    assert_eq!(run(&map_src(300)), Err(codes::NESTING));
}

/// ADR-0004 covers every annotation position, not just `fn` parameters: all
/// type syntax routes through the parser's single `ty()` entry, which owns the
/// `type_depth` counter. This pins the deeply-nested type in a struct field,
/// an enum payload, an alias target, a `let` annotation, and a return type.
#[test]
fn type_nesting_is_bounded_in_every_annotation_position() {
    // 256 wraps is 257 AST levels (reject); 255 wraps is 256 (accept).
    let deep = |n: usize| {
        let mut t = "int".to_string();
        for _ in 0..n {
            t = format!("Box<{t}>");
        }
        t
    };
    let over = deep(256);
    let ok = deep(255);
    let prelude = "struct Box<T> { value: T }\n";

    let cases = [
        format!("{prelude}struct S {{ x: {over} }}\nfn main() {{ print(1) }}"),
        format!("{prelude}enum E {{ V({over}) }}\nfn main() {{ print(1) }}"),
        format!("{prelude}type MyAlias = {over}\nfn main() {{ print(1) }}"),
        format!("{prelude}fn main() {{ let _: {over} = 0 }}"),
        format!("{prelude}fn f() -> {over} {{ return 0 }}\nfn main() {{ print(1) }}"),
    ];
    for (i, src) in cases.iter().enumerate() {
        assert_eq!(
            run(src),
            Err(codes::NESTING),
            "annotation position {i} must reject a 257-level type"
        );
    }
    // The same positions accept one level under the limit.
    let ok_cases = [
        format!("{prelude}struct S {{ x: {ok} }}\nfn main() {{ print(1) }}"),
        format!("{prelude}enum E {{ V({ok}) }}\nfn main() {{ print(1) }}"),
        format!("{prelude}type MyAlias = {ok}\nfn main() {{ print(1) }}"),
    ];
    for (i, src) in ok_cases.iter().enumerate() {
        assert!(
            run(src).is_ok(),
            "annotation position {i} must accept 256 levels"
        );
    }
}

/// TD-15: depth-limit `E1015` diagnostics must carry a source location, so a
/// tool can point at the offending node rather than the whole program. Before,
/// some (`module`, and the post-parse expression/statement walk) used
/// `Span::default()`.
#[test]
fn depth_diagnostics_carry_a_source_location() {
    // Deep expression: the post-parse walk attributes the exact node.
    let expr = (0..300).fold(String::from("1"), |acc, _| format!("({acc} + 1)"));
    let src = format!("fn main() {{ let x = {expr} }}");
    let module = aura::parse::parse(&src).expect_err("must be E1015");
    assert_eq!(module.code, codes::NESTING);
    let loc = module.span;
    assert!(
        loc.start > 0 && loc.end > loc.start,
        "expression depth diagnostic must carry a real span, got {loc:?}"
    );

    // Deep module: located at the nesting site, not 1:1.
    let nested = format!(
        "{}fn f() {{}}\n{}\nfn main() {{ print(1) }}\n",
        "module M {\n".repeat(300),
        "}\n".repeat(300)
    );
    let module = aura::parse::parse(&nested).expect_err("must be E1015");
    assert_eq!(module.code, codes::NESTING);
    let (line, _col) = aura::error::line_col(&nested, module.span.start);
    assert_eq!(
        line, 257,
        "module depth diagnostic must point at the deep level"
    );
}
