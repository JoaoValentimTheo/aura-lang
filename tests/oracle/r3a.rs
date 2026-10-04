#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! B-1R3A — the clearly identified iterative-engine oracle subset.
//!
//! These cases exercise **only** the constructs the explicit-continuation
//! machine supports (literals, name lookup, expression statements, sequential
//! blocks, `let` shadowing, block expressions, and `if`/`else`). Every case is
//! run through the real `Engine::iterative` path by the differential test in
//! `tests/evaluator_oracle.rs`; none may fall back to recursion.
//!
//! The unsupported set is asserted independently: each source is valid for the
//! recursive engine but must fail with the deterministic `E4999`
//! "not supported by the iterative engine" diagnostic when run iteratively.
//! That is the anti-fallback guard.

use crate::harness::{Case, Kind};

fn value(name: &'static str, source: &'static str) -> Case {
    Case {
        group: "r3a-value",
        name,
        file: "<r3a>",
        source,
        kind: Kind::Value,
    }
}

fn program(name: &'static str, source: &'static str) -> Case {
    Case {
        group: "r3a-program",
        name,
        file: "r3a.aura",
        source,
        kind: Kind::ExecuteProgram,
    }
}

/// Constructs the iterative engine supports. Recursive and iterative must agree.
#[must_use]
pub fn supported_cases() -> Vec<Case> {
    vec![
        // ----- literals --------------------------------------------------
        value("lit_int", "42\n"),
        value("lit_float", "1.5\n"),
        value("lit_str", "\"hi\"\n"),
        value("lit_bool_true", "true\n"),
        value("lit_bool_false", "false\n"),
        value("lit_none", "none\n"),
        // ----- blocks and sequencing ------------------------------------
        value("block_empty", "{ }\n"),
        value("block_single", "{ 1 }\n"),
        value("block_multi", "{ 1\n 2\n 3 }\n"),
        value("block_trailing_none", "{ 1\n none }\n"),
        // ----- let / shadowing ------------------------------------------
        value("let_simple", "{ let x = 7\n x }\n"),
        value("let_shadow_same_block", "{ let x = 1\n let x = 2\n x }\n"),
        value(
            "let_shadow_inner_block",
            "{ let x = 1\n { let x = 2\n x } }\n",
        ),
        value(
            "let_shadow_does_not_leak_out",
            "{ let x = 1\n { let x = 2\n 0 }\n x }\n",
        ),
        // ----- name lookup ----------------------------------------------
        // A single-overload top-level function referenced as a value.
        value("name_function_value", "fn f() {}\nf\n"),
        // A native referenced as a value.
        value("name_native_value", "len\n"),
        // ----- if / else -------------------------------------------------
        value("if_true", "if true { 1 } else { 2 }\n"),
        value("if_false", "if false { 1 } else { 2 }\n"),
        value("if_no_else_false", "if false { 1 }\n"),
        value("if_no_else_true", "if true { 1 }\n"),
        value("if_block_body", "if true { let x = 5\n x } else { 0 }\n"),
        value(
            "if_nested",
            "if true { if false { 1 } else { 2 } } else { 3 }\n",
        ),
        // Truthiness in the condition (mirrors `Value::truthy`).
        value("if_truthiness_zero", "if 0 { 1 } else { 2 }\n"),
        value("if_truthiness_empty_str", "if \"\" { 1 } else { 2 }\n"),
        value("if_truthiness_none", "if none { 1 } else { 2 }\n"),
        // ----- block-valued if ------------------------------------------
        value(
            "if_yields_block_value",
            "{ if true { let x = 3\n x } else { 0 } }\n",
        ),
        // ----- diagnostics reachable in the subset ----------------------
        // Undefined name -> E2003, with the same message and span.
        value("undefined_name", "missing_name\n"),
        // The machine's runtime `E1015` AST-depth guard is exercised directly
        // by `src/run/iterative.rs::tests::ast_depth_limit_is_e1015`, because
        // the checker rejects an over-deep *source* before execution; a
        // differential case here would compare two compile rejections, not the
        // engines.
        // ----- top-level constants (value-path `Const` routing) ---------
        value("const_literal", "const X = 5\nX\n"),
        value("const_from_const", "const X = 5\nconst Y = X\nY\n"),
        // ----- program (real frame boundary) ----------------------------
        program("main_empty", "fn main() { }\n"),
        program("main_if_false", "fn main() { if false { } }\n"),
        program("main_return", "fn main() { return }\n"),
        program("main_if_return", "fn main() { if true { return } }\n"),
        program(
            "main_nested_block",
            "fn main() { if true { let x = 1\n if x { } } }\n",
        ),
    ]
}

/// Sources that are valid (recursive engine accepts them) but unsupported by
/// the iterative machine. Each must produce `E4999` when run iteratively — and
/// must **not** silently recurse.
#[must_use]
pub fn unsupported_cases() -> Vec<Case> {
    vec![
        // `fstring` was removed in B-1R3B.7: f-string evaluation is supported
        // now and asserted by `r3b::fstring_supported_cases`.
        // `list_literal`/`tuple_literal` were removed in B-1R3B.4.1: list and
        // tuple construction are supported now and asserted by
        // `r3b::list_supported_cases`. `map_literal` was removed in
        // B-1R3B.4.2: map construction is supported now and asserted by
        // `r3b::map_supported_cases`. `range_literal` was removed in
        // B-1R3B.5: range construction is supported now and asserted by
        // `r3b::range_supported_cases`.
        value("call", "len([1, 2])\n"),
        value("method_call", "[1, 2].len()\n"),
        value(
            "field_access",
            "struct P { x: int }\n{ let p = P { x: 1 }\n p.x }\n",
        ),
        // `index` was removed in B-1R3B.6: index reads are supported now and
        // asserted by `r3b::index_supported_cases`. `field_access` stays: it
        // builds a struct with `Expr::Construct`, which is still unsupported,
        // so the E4999 sentinel occurs before the field read; the reachable
        // field surface (builtin zero-argument methods) is asserted by
        // `r3b::field_supported_cases`.
        value("lambda", "() -> 1\n"),
        value("pipe", "[1, 2] |> len\n"),
        value("match", "match 1 { 1 -> { 2 } }\n"),
        value("list_comp", "[x for x in [1, 2]]\n"),
        value("while_stmt", "{ while false { 1 } }\n"),
        value("loop_stmt", "{ loop { break } }\n"),
        value("let_pattern", "{ let [a] = [1]\n a }\n"),
        value("assign", "{ let mut x = 1\n x = 2\n x }\n"),
    ]
}
