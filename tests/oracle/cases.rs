#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// The corpus is intentionally a long sequence of `out.push(Case { ... })`
// grouped by survey, mirroring `tests/corpus.rs`; a single `vec![..]` literal
// would be less readable than the group comments.
#![allow(clippy::vec_init_then_push)]
//! The deterministic differential-oracle corpus.
//!
//! Reuse-first: the existing `tests/corpus/**` boundary fixtures are pulled in
//! with `include_str!` rather than duplicated. New cases exist only where the
//! design's suspension matrix, evaluation-order matrix, callback-frame rule,
//! `finally` matrix, or module-attribution needs them.
//!
//! No randomized generation here; the deterministic corpus is the contract.

use crate::harness::{Case, Kind, MultiSource};

// ---------------------------------------------------------------------------
// Multi-source static graphs (module attribution)
// ---------------------------------------------------------------------------

/// Entry calls a module function that divides by zero at runtime; the
/// diagnostic must attribute to `child.aura`, not `main.aura`.
pub static MULTI_RUNTIME_ERROR: MultiSource = MultiSource {
    entry: ("entry", "main.aura", "fn main() { print(child::boom()) }"),
    extra: &[(
        "child-key",
        "child.aura",
        "pub fn boom() -> int { return 1 / 0 }",
    )],
    edges: &[("entry", "child", "child-key")],
};

/// A module function returns normally to its caller.
pub static MULTI_RETURN: MultiSource = MultiSource {
    entry: ("entry", "main.aura", "fn main() { print(child::value()) }"),
    extra: &[(
        "child-key",
        "child.aura",
        "pub fn value() -> int { return 42 }",
    )],
    edges: &[("entry", "child", "child-key")],
};

/// Module initialization order: top-level `Const`/`Expr` items in the entry and
/// a nested module both print, in deterministic order.
pub static MULTI_INIT_ORDER: MultiSource = MultiSource {
    entry: (
        "entry",
        "main.aura",
        "print(\"entry-init\")\nfn main() { print(\"main\") }",
    ),
    extra: &[(
        "child-key",
        "child.aura",
        "print(\"child-init\")\npub fn value() -> int { return 1 }",
    )],
    edges: &[("entry", "child", "child-key")],
};

/// Closure-source lifetime probe graph: the entry churns many closures, then a
/// lambda defined in `child.aura` faults, checking attribution stability.
pub static MULTI_CHURN: MultiSource = MultiSource {
    entry: (
        "entry",
        "main.aura",
        "fn make(n: int) { return () -> n }\nfn churn() { let mut last = make(0)\n for i in range(2000) { last = make(i) }\n return last }\nfn main() { churn()\n print(child::boom()) }",
    ),
    extra: &[(
        "child-key",
        "child.aura",
        "pub fn boom() -> int { let f = () -> (1 / 0)\n return f() }",
    )],
    edges: &[("entry", "child", "child-key")],
};

/// Uncaught `throw` from a module function: the recursive engine attributes
/// the user-facing `E4026` to the caller's frame source, not the entry source.
pub static MULTI_UNCAUGHT_THROW: MultiSource = MultiSource {
    entry: ("entry", "main.aura", "fn main() { child::raises() }"),
    extra: &[(
        "child-key",
        "child.aura",
        "pub fn raises() -> int { throw 7 }",
    )],
    edges: &[("entry", "child", "child-key")],
};

/// Uncaught `throw` two frames deep: attribution lands on the intermediate
/// frame (`mid.aura`), matching the recursive engine's per-frame recovery.
pub static MULTI_UNCAUGHT_THROW_NESTED: MultiSource = MultiSource {
    entry: ("entry", "main.aura", "fn main() { mid::call() }"),
    extra: &[
        (
            "mid-key",
            "mid.aura",
            "pub fn call() -> int { leaf::boom() }",
        ),
        ("leaf-key", "leaf.aura", "pub fn boom() -> int { throw 9 }"),
    ],
    edges: &[("entry", "mid", "mid-key"), ("mid-key", "leaf", "leaf-key")],
};

/// A `throw` raised in a child module but caught by a `try` in the entry.
pub static MULTI_THROW_CAUGHT: MultiSource = MultiSource {
    entry: (
        "entry",
        "main.aura",
        "fn main() { let x = { try { child::raises() } catch e { e } }\n print(x) }",
    ),
    extra: &[(
        "child-key",
        "child.aura",
        "pub fn raises() -> int { throw 5 }",
    )],
    edges: &[("entry", "child", "child-key")],
};

/// Three sources, entry → mid → leaf, with a runtime diagnostic in the leaf.
pub static MULTI_CHAIN: MultiSource = MultiSource {
    entry: ("entry", "main.aura", "fn main() { print(mid::call()) }"),
    extra: &[
        (
            "mid-key",
            "mid.aura",
            "pub fn call() -> int { return leaf::boom() }",
        ),
        (
            "leaf-key",
            "leaf.aura",
            "pub fn boom() -> int { return 1 % 0 }",
        ),
    ],
    edges: &[("entry", "mid", "mid-key"), ("mid-key", "leaf", "leaf-key")],
};

// ---------------------------------------------------------------------------
// Corpus
// ---------------------------------------------------------------------------

/// Every oracle case, in a stable order.
#[must_use]
pub fn cases() -> Vec<Case> {
    let mut out: Vec<Case> = Vec::new();

    // ----- core semantics ------------------------------------------------
    out.push(Case {
        group: "core",
        name: "literals",
        file: "literals.aura",
        source: r#"fn main() {
  print(1)
  print(1.5)
  print("hi")
  print(true)
  print(none)
}
"#,
        kind: Kind::ExecuteProgram,
    });
    out.push(Case {
        group: "core",
        name: "arithmetic",
        file: "arithmetic.aura",
        source: r#"fn main() {
  print(1 + 2 * 3)
  print(7 / 2)
  print(7 % 2)
  print(1 < 2)
  print("a" + "b")
  print([1] + [2])
}
"#,
        kind: Kind::ExecuteProgram,
    });
    out.push(Case {
        group: "core",
        name: "unary_bool",
        file: "unary_bool.aura",
        source: r#"fn main() {
  print(not false)
  print(-3)
  print(~0)
  print(1 == 1.0)
  print("a" < "b")
  print(true and false)
  print(true or false)
}
"#,
        kind: Kind::ExecuteProgram,
    });
    out.push(Case {
        group: "core",
        name: "nested_calls",
        file: "nested_calls.aura",
        source: r#"fn inc(x: int) -> int { return x + 1 }
fn add(a: int, b: int) -> int { return a + b }
fn main() { print(inc(add(inc(1), 2))) }
"#,
        kind: Kind::ExecuteProgram,
    });
    out.push(Case {
        group: "core",
        name: "arg_order",
        file: "arg_order.aura",
        source: r#"fn a() -> int { print("a"); return 1 }
fn b() -> int { print("b"); return 2 }
fn add(x: int, y: int) -> int { return x + y }
fn main() { print(add(a(), b())) }
"#,
        kind: Kind::ExecuteProgram,
    });
    out.push(Case {
        group: "core",
        name: "direct_recursion",
        file: "direct_recursion.aura",
        source: r#"fn fact(n: int) -> int {
  if n <= 1 { return 1 }
  return n * fact(n - 1)
}
fn main() { print(fact(10)) }
"#,
        kind: Kind::ExecuteProgram,
    });
    out.push(Case {
        group: "core",
        name: "mutual_recursion",
        file: "mutual_recursion.aura",
        source: r#"fn even(n: int) -> bool {
  if n == 0 { return true }
  return odd(n - 1)
}
fn odd(n: int) -> bool {
  if n == 0 { return false }
  return even(n - 1)
}
fn main() {
  print(even(10))
  print(odd(7))
}
"#,
        kind: Kind::ExecuteProgram,
    });
    out.push(Case {
        group: "core",
        name: "closures_capture",
        file: "closures_capture.aura",
        source: r#"fn main() {
  let mut x = 1
  let f = () -> { x = x + 1 }
  f()
  f()
  print(x)
}
"#,
        kind: Kind::ExecuteProgram,
    });
    out.push(Case {
        group: "core",
        name: "returned_closure",
        file: "returned_closure.aura",
        source: r#"fn make(n: int) { return () -> n }
fn main() {
  let g = make(5)
  print(g())
}
"#,
        kind: Kind::ExecuteProgram,
    });
    out.push(Case {
        group: "core",
        name: "methods_self",
        file: "methods_self.aura",
        source: r#"struct Point { x: int, y: int }
impl Point { fn sum(self) -> int { return self.x + self.y } }
fn main() {
  let p = Point { x: 3, y: 4 }
  print(p.sum())
}
"#,
        kind: Kind::ExecuteProgram,
    });
    out.push(Case {
        group: "core",
        name: "in_source_module",
        file: "in_source_module.aura",
        source: r#"module math { pub fn twice(x: int) -> int { return x * 2 } }
fn main() { print(math::twice(21)) }
"#,
        kind: Kind::ExecuteProgram,
    });
    out.push(Case {
        group: "core",
        name: "loops_break_continue",
        file: "loops_break_continue.aura",
        source: r#"fn main() {
  let mut i = 0
  let mut acc = 0
  while i < 5 {
    i = i + 1
    if i == 3 { continue }
    if i == 5 { break }
    acc = acc + i
  }
  print(acc)
  for x in range(3) { print(x) }
}
"#,
        kind: Kind::ExecuteProgram,
    });
    out.push(Case {
        group: "core",
        name: "match",
        file: "match.aura",
        source: r#"enum Color { Red, Green, Blue }
fn name(c: Color) -> int {
  return match c { Red() -> 0
 Green() -> 1
 Blue() -> 2 }
}
fn main() {
  print(name(Green()))
  print(name(Blue()))
}
"#,
        kind: Kind::ExecuteProgram,
    });
    out.push(Case {
        group: "core",
        name: "collections",
        file: "collections.aura",
        source: r#"fn main() {
  let mut xs = [1, 2, 3]
  xs.push(4)
  print(xs)
  print(len(xs))
  print(xs[1])
  xs[1] = 9
  print(xs)
  let m = {"a": 1, "b": 2}
  print(m["a"])
  print(keys(m))
}
"#,
        kind: Kind::ExecuteProgram,
    });
    out.push(Case {
        group: "core",
        name: "comprehensions",
        file: "comprehensions.aura",
        source: r#"fn main() {
  print([x * 2 for x in [1, 2, 3]])
  print([x for x in [1, 2, 3, 4] if x % 2 == 0])
  print({x: x * x for x in [1, 2, 3]})
}
"#,
        kind: Kind::ExecuteProgram,
    });
    out.push(Case {
        group: "core",
        name: "fstrings",
        file: "fstrings.aura",
        source: r#"fn main() {
  let name = "Aura"
  print(f"hi {name}")
  print(f"{1 + 2}")
  print(f"{3.14159:.2f}")
  print(f"{42:04d}")
}
"#,
        kind: Kind::ExecuteProgram,
    });
    out.push(Case {
        group: "core",
        name: "pipe",
        file: "pipe.aura",
        source: r#"fn inc(x: int) -> int { return x + 1 }
fn main() { print(41 |> inc); print([1, 2, 3] |> len) }
"#,
        kind: Kind::ExecuteProgram,
    });
    // Module-mode acceptance without `main` (declaration set / expression list).
    out.push(Case {
        group: "core",
        name: "module_mode_accepts_no_main",
        file: "module_mode.aura",
        source: "fn helper(x: int) -> int { return x }\n",
        kind: Kind::CompileModule,
    });
    // Module-mode execution of top-level items, no `main` required.
    out.push(Case {
        group: "core",
        name: "module_mode_executes_toplevel",
        file: "module_exec.aura",
        source: "print(\"toplevel\")\nlet n = 2 + 3\nprint(n)\n",
        kind: Kind::ExecuteModule,
    });

    // ----- runtime / compile diagnostics ---------------------------------
    out.push(Case {
        group: "diag",
        name: "div_zero",
        file: "div_zero.aura",
        source: "fn main() { print(1 / 0) }\n",
        kind: Kind::ExecuteProgram,
    });
    out.push(Case {
        group: "diag",
        name: "index_out_of_range",
        file: "index.aura",
        source: "fn main() { let xs = [1]; print(xs[5]) }\n",
        kind: Kind::ExecuteProgram,
    });
    out.push(Case {
        group: "diag",
        name: "overflow",
        file: "overflow.aura",
        source: "fn main() { print(9223372036854775807 + 1) }\n",
        kind: Kind::ExecuteProgram,
    });
    out.push(Case {
        group: "diag",
        name: "type_mismatch_runtime",
        file: "type_mismatch.aura",
        source: "fn main() { print(1 + \"a\") }\n",
        kind: Kind::ExecuteProgram,
    });
    out.push(Case {
        group: "diag",
        name: "no_match",
        file: "no_match.aura",
        source: "enum E { A }\nfn main() { print(match 5 { A() -> 1 }) }\n",
        kind: Kind::ExecuteProgram,
    });
    out.push(Case {
        group: "diag",
        name: "break_outside_loop",
        file: "break_outside.aura",
        source: "fn main() { break }\n",
        kind: Kind::ExecuteProgram,
    });
    out.push(Case {
        group: "diag",
        name: "uncaught_throw",
        file: "uncaught_throw.aura",
        source: "fn main() { throw \"boom\" }\n",
        kind: Kind::ExecuteProgram,
    });
    out.push(Case {
        group: "diag",
        name: "undefined_name_compile",
        file: "undefined.aura",
        source: "fn main() { print(nope) }\n",
        kind: Kind::CompileProgram,
    });
    out.push(Case {
        group: "diag",
        name: "immutable_assign_compile",
        file: "immutable.aura",
        source: "fn main() { let x = 1; x = 2 }\n",
        kind: Kind::CompileProgram,
    });
    out.push(Case {
        group: "diag",
        name: "missing_main_compile",
        file: "no_main.aura",
        source: "fn helper() { print(1) }\n",
        kind: Kind::CompileProgram,
    });
    // Module-mode compile-accept of ordinary arithmetic (not the nesting
    // boundary; that is pinned by `tests/boundaries.rs`).
    out.push(Case {
        group: "diag",
        name: "arithmetic_compile_accept",
        file: "arithmetic_accept.aura",
        source: "fn main() { print(1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1) }\n",
        kind: Kind::CompileModule,
    });

    // Float edge cases: NaN, +inf, and signed zero must be observable. NaN is
    // reached without division (which is E4007 for floats too).
    out.push(Case {
        group: "float",
        name: "nan_inf_signed_zero",
        file: "float_edge.aura",
        source: r#"fn big() -> float { return 1.0e308 * 10.0 }
fn main() {
  print(big())
  print(big() - big())
  print(big() == big() - big())
  print(big() - big() == big() - big())
  print(-0.0)
  print(0.0 == -0.0)
}
"#,
        kind: Kind::ExecuteProgram,
    });
    out.push(Case {
        group: "float",
        name: "integral_float_display",
        file: "float_int.aura",
        source: "fn main() { print(1.0); print(to_string(1.0)); print([1.0, 2.0]) }\n",
        kind: Kind::ExecuteProgram,
    });

    // ----- source provenance: identical bytes, different source names -----
    // Both cases are byte-identical; only the display name differs. The oracle
    // must keep them distinguishable.
    out.push(Case {
        group: "provenance",
        name: "same_bytes_a",
        file: "a.aura",
        source: "fn main() { print(1 / 0) }\n",
        kind: Kind::ExecuteProgram,
    });
    out.push(Case {
        group: "provenance",
        name: "same_bytes_b",
        file: "b.aura",
        source: "fn main() { print(1 / 0) }\n",
        kind: Kind::ExecuteProgram,
    });

    // ----- call-position / evaluation-order matrix ------------------------
    add_call_position_cases(&mut out);

    // ----- callbacks (native vs closure frame asymmetry) ------------------
    out.push(Case {
        group: "callback",
        name: "native_callback_value",
        file: "native_cb.aura",
        source: "fn main() { print([1].map(to_string)) }\n",
        kind: Kind::ExecuteProgram,
    });
    out.push(Case {
        group: "callback",
        name: "closure_callback_value",
        file: "closure_cb.aura",
        source: "fn main() { print([1].map((x) -> x)) }\n",
        kind: Kind::ExecuteProgram,
    });
    out.push(Case {
        group: "callback",
        name: "closure_callback_recursion_small",
        file: "closure_cb_rec.aura",
        source: r#"fn rec(n: int) -> int { if n <= 0 { return 0 }
  return [0].map((x) -> rec(n - 1) + 1)[0] }
fn main() { print(rec(3)) }
"#,
        kind: Kind::ExecuteProgram,
    });
    out.push(Case {
        group: "callback",
        name: "closure_callback_counts_frame_at_boundary",
        file: "closure_cb_boundary.aura",
        source: r#"fn f(n: int) -> int {
  if n <= 0 { return len([1].map((x) -> x)) }
  return f(n - 1) + 1
}
fn main() { print(f(510)) }
"#,
        kind: Kind::ExecuteProgram,
    });
    out.push(Case {
        group: "callback",
        name: "native_callback_adds_no_frame_at_boundary",
        file: "native_cb_boundary.aura",
        source: r#"fn f(n: int) -> int {
  if n <= 0 { return len([1].map(to_string)) }
  return f(n - 1) + 1
}
fn main() { print(f(510)) }
"#,
        kind: Kind::ExecuteProgram,
    });
    out.push(Case {
        group: "callback",
        name: "callback_list_snapshot",
        file: "callback_snapshot.aura",
        source: r#"fn main() {
  let mut xs = [1, 2, 3]
  let ys = xs.map((x) -> { xs.push(99); return x })
  print(ys)
  print(xs)
}
"#,
        kind: Kind::ExecuteProgram,
    });

    // ----- compound assignment (current behavior, pinned) -----------------
    out.push(Case {
        group: "compound",
        name: "index_target_double_eval",
        file: "compound_index.aura",
        source: r#"fn idx() -> int { print("idx"); return 0 }
fn main() {
  let mut a = [1]
  a[idx()] += 10
  print(a)
}
"#,
        kind: Kind::ExecuteProgram,
    });
    out.push(Case {
        group: "compound",
        name: "index_rhs_order",
        file: "compound_rhs.aura",
        source: r#"fn rhs() -> int { print("rhs"); return 5 }
fn idx() -> int { print("idx"); return 0 }
fn main() {
  let mut a = [1]
  let r = rhs()
  a[idx()] += r
  print(a)
}
"#,
        kind: Kind::ExecuteProgram,
    });

    // ----- try / catch / finally matrix ----------------------------------
    add_finally_matrix(&mut out);

    // ----- B-1 contract-boundary corpus (reused fixtures) -----------------
    out.push(boundary(
        "else_510",
        include_str!("../corpus/call-frames/else_510.aura"),
    ));
    out.push(boundary(
        "else_511",
        include_str!("../corpus/call-frames/else_511.aura"),
    ));
    out.push(boundary(
        "simple_510",
        include_str!("../corpus/call-frames/simple_510.aura"),
    ));
    out.push(boundary(
        "simple_511",
        include_str!("../corpus/call-frames/simple_511.aura"),
    ));
    out.push(boundary(
        "match_510",
        include_str!("../corpus/call-frames/match_510.aura"),
    ));
    out.push(boundary(
        "match_511",
        include_str!("../corpus/call-frames/match_511.aura"),
    ));
    out.push(boundary(
        "closure_510",
        include_str!("../corpus/call-frames/closure_510.aura"),
    ));
    out.push(boundary(
        "closure_511",
        include_str!("../corpus/call-frames/closure_511.aura"),
    ));
    out.push(boundary(
        "method_510",
        include_str!("../corpus/call-frames/method_510.aura"),
    ));
    out.push(boundary(
        "method_511",
        include_str!("../corpus/call-frames/method_511.aura"),
    ));
    out.push(boundary(
        "module_510",
        include_str!("../corpus/call-frames/module_510.aura"),
    ));
    out.push(boundary(
        "module_511",
        include_str!("../corpus/call-frames/module_511.aura"),
    ));
    out.push(boundary(
        "mutual_510",
        include_str!("../corpus/call-frames/mutual_510.aura"),
    ));
    out.push(boundary(
        "mutual_511",
        include_str!("../corpus/call-frames/mutual_511.aura"),
    ));
    out.push(boundary(
        "try_510",
        include_str!("../corpus/call-frames/try_510.aura"),
    ));
    out.push(boundary(
        "try_511",
        include_str!("../corpus/call-frames/try_511.aura"),
    ));

    // ----- multi-source / module attribution ------------------------------
    out.push(Case {
        group: "module",
        name: "runtime_error_in_child",
        file: "main.aura",
        source: "",
        kind: Kind::Multi(&MULTI_RUNTIME_ERROR),
    });
    out.push(Case {
        group: "module",
        name: "return_to_caller",
        file: "main.aura",
        source: "",
        kind: Kind::Multi(&MULTI_RETURN),
    });
    out.push(Case {
        group: "module",
        name: "init_order",
        file: "main.aura",
        source: "",
        kind: Kind::Multi(&MULTI_INIT_ORDER),
    });
    out.push(Case {
        group: "module",
        name: "chain_error_in_leaf",
        file: "main.aura",
        source: "",
        kind: Kind::Multi(&MULTI_CHAIN),
    });
    out.push(Case {
        group: "module",
        name: "uncaught_throw_in_child",
        file: "main.aura",
        source: "",
        kind: Kind::Multi(&MULTI_UNCAUGHT_THROW),
    });
    out.push(Case {
        group: "module",
        name: "uncaught_throw_in_leaf",
        file: "main.aura",
        source: "",
        kind: Kind::Multi(&MULTI_UNCAUGHT_THROW_NESTED),
    });
    out.push(Case {
        group: "module",
        name: "throw_in_child_caught",
        file: "main.aura",
        source: "",
        kind: Kind::Multi(&MULTI_THROW_CAUGHT),
    });

    // ----- value observables (REPL-equivalent final value) ----------------
    out.push(Case {
        group: "value",
        name: "int_expression",
        file: "<value>",
        source: "1 + 2 * 3\n",
        kind: Kind::Value,
    });
    out.push(Case {
        group: "value",
        name: "list_expression",
        file: "<value>",
        source: "[1, 2, 3]\n",
        kind: Kind::Value,
    });
    out.push(Case {
        group: "value",
        name: "map_expression",
        file: "<value>",
        source: "{\"b\": 2, \"a\": 1}\n",
        kind: Kind::Value,
    });
    out.push(Case {
        group: "value",
        name: "string_expression",
        file: "<value>",
        source: "\"hello\"\n",
        kind: Kind::Value,
    });
    // Type-preserving value cases: `1`, `1.0`, and `"1"` all display as `1`
    // at top level, so only a type-tagged observable distinguishes them.
    out.push(Case {
        group: "value",
        name: "float_expression",
        file: "<value>",
        source: "1.0\n",
        kind: Kind::Value,
    });
    out.push(Case {
        group: "value",
        name: "string_number_expression",
        file: "<value>",
        source: "\"1\"\n",
        kind: Kind::Value,
    });
    out.push(Case {
        group: "value",
        name: "bool_expression",
        file: "<value>",
        source: "true\n",
        kind: Kind::Value,
    });
    out.push(Case {
        group: "value",
        name: "none_expression",
        file: "<value>",
        source: "none\n",
        kind: Kind::Value,
    });

    out
}

fn boundary(name: &'static str, source: &'static str) -> Case {
    Case {
        group: "b1-boundary",
        name,
        file: "boundary.aura",
        source,
        kind: Kind::ExecuteProgram,
    }
}

fn add_call_position_cases(out: &mut Vec<Case>) {
    out.push(Case {
        group: "call-position",
        name: "binary_left",
        file: "cp_binary_left.aura",
        source: r#"fn a() -> int { print("a"); return 1 }
fn b() -> int { print("b"); return 2 }
fn main() { print(a() + b()) }
"#,
        kind: Kind::ExecuteProgram,
    });
    out.push(Case {
        group: "call-position",
        name: "binary_right_vs_left",
        file: "cp_binary_right.aura",
        source: r#"fn a() -> int { print("a"); return 1 }
fn b() -> int { print("b"); return 2 }
fn main() { print((a() * 10) + b()) }
"#,
        kind: Kind::ExecuteProgram,
    });
    out.push(Case {
        group: "call-position",
        name: "function_argument",
        file: "cp_arg.aura",
        source: r#"fn a() -> int { print("a"); return 1 }
fn b() -> int { print("b"); return 2 }
fn add(x: int, y: int) -> int { return x + y }
fn main() { print(add(a(), b())) }
"#,
        kind: Kind::ExecuteProgram,
    });
    out.push(Case {
        group: "call-position",
        name: "short_circuit",
        file: "cp_short_circuit.aura",
        source: r#"fn t() -> bool { print("t"); return true }
fn f() -> bool { print("f"); return false }
fn main() {
  print(false and t())
  print(true or f())
  print(true and t())
  print(false or f())
}
"#,
        kind: Kind::ExecuteProgram,
    });
    out.push(Case {
        group: "call-position",
        name: "index_base_and_key",
        file: "cp_index.aura",
        source: r#"fn base() -> [int] { print("base"); return [10, 20, 30] }
fn idx() -> int { print("idx"); return 1 }
fn main() { print(base()[idx()]) }
"#,
        kind: Kind::ExecuteProgram,
    });
    out.push(Case {
        group: "call-position",
        name: "method_receiver_and_arg",
        file: "cp_method.aura",
        source: r#"struct S { n: int }
impl S { fn add(self, x: int) -> int { print("add"); return self.n + x } }
fn recv() -> S { print("recv"); return S { n: 1 } }
fn arg() -> int { print("arg"); return 5 }
fn main() { print(recv().add(arg())) }
"#,
        kind: Kind::ExecuteProgram,
    });
    out.push(Case {
        group: "call-position",
        name: "list_items",
        file: "cp_list.aura",
        source: r#"fn a() -> int { print("a"); return 1 }
fn b() -> int { print("b"); return 2 }
fn main() { print([a(), b()]) }
"#,
        kind: Kind::ExecuteProgram,
    });
    out.push(Case {
        group: "call-position",
        name: "map_key_and_value",
        file: "cp_map.aura",
        source: r#"fn ka() -> string { print("ka"); return "x" }
fn kb() -> string { print("kb"); return "y" }
fn a() -> int { print("a"); return 1 }
fn b() -> int { print("b"); return 2 }
fn main() { print({ka(): a(), kb(): b()}) }
"#,
        kind: Kind::ExecuteProgram,
    });
    out.push(Case {
        group: "call-position",
        name: "fstring_interpolation",
        file: "cp_fstr.aura",
        source: r#"fn a() -> int { print("a"); return 1 }
fn b() -> int { print("b"); return 2 }
fn main() { print(f"{a()} {b()}") }
"#,
        kind: Kind::ExecuteProgram,
    });
    out.push(Case {
        group: "call-position",
        name: "comprehension_value",
        file: "cp_comp_value.aura",
        source: r#"fn a() -> int { print("a"); return 1 }
fn main() { print([a() for x in [1, 2]]) }
"#,
        kind: Kind::ExecuteProgram,
    });
    out.push(Case {
        group: "call-position",
        name: "comprehension_filter",
        file: "cp_comp_filter.aura",
        source: r#"fn keep(x: int) -> bool { print(x); return x > 1 }
fn main() { print([x for x in [1, 2, 3] if keep(x)]) }
"#,
        kind: Kind::ExecuteProgram,
    });
    out.push(Case {
        group: "call-position",
        name: "match_guard",
        file: "cp_match_guard.aura",
        source: r#"fn g() -> bool { print("g"); return true }
fn main() { print(match 1 { 1 if g() -> 10
 _ -> 20 }) }
"#,
        kind: Kind::ExecuteProgram,
    });
    out.push(Case {
        group: "call-position",
        name: "return_expression",
        file: "cp_return.aura",
        source: r#"fn a() -> int { print("a"); return 1 }
fn b() -> int { print("b"); return 2 }
fn r() -> int { return a() + b() }
fn main() { print(r()) }
"#,
        kind: Kind::ExecuteProgram,
    });
    out.push(Case {
        group: "call-position",
        name: "assign_rhs",
        file: "cp_assign.aura",
        source: r#"fn a() -> int { print("a"); return 1 }
fn b() -> int { print("b"); return 2 }
fn main() {
  let mut x = 0
  x = a() + b()
  print(x)
}
"#,
        kind: Kind::ExecuteProgram,
    });
    out.push(Case {
        group: "call-position",
        name: "range_bounds",
        file: "cp_range.aura",
        source: r#"fn a() -> int { print("a"); return 1 }
fn b() -> int { print("b"); return 4 }
fn main() { for x in (a()..b()) { print(x) } }
"#,
        kind: Kind::ExecuteProgram,
    });
    out.push(Case {
        group: "call-position",
        name: "construct_args",
        file: "cp_construct.aura",
        source: r#"struct P { x: int, y: int }
fn a() -> int { print("a"); return 1 }
fn b() -> int { print("b"); return 2 }
fn main() { print(P { x: a(), y: b() }) }
"#,
        kind: Kind::ExecuteProgram,
    });
}

fn add_finally_matrix(out: &mut Vec<Case>) {
    // Pending `{Val, Return, Throw, Break, Continue}` × finally
    // `{Val, Return, Throw}`. Every combination that is structurally legal is
    // pinned to its current behavior; the corpus is the migration contract.
    let mut push = |name: &'static str, src: &'static str| {
        out.push(Case {
            group: "finally",
            name,
            file: "finally.aura",
            source: src,
            kind: Kind::ExecuteProgram,
        });
    };

    push(
        "pending_val__finally_val",
        "fn f() -> int { try { 1 } catch e { 2 } finally { print(\"F\") } }\nfn main() { print(f()) }\n",
    );
    push(
        "pending_val__finally_return",
        "fn f() -> int { try { 1 } catch e { 2 } finally { return 7 } }\nfn main() { print(f()) }\n",
    );
    push(
        "pending_val__finally_throw",
        "fn f() -> int { try { 1 } catch e { 2 } finally { throw \"T\" } }\nfn main() { print(f()) }\n",
    );
    push(
        "pending_return__finally_val",
        "fn f() -> int { try { return 1 } catch e { return 2 } finally { print(\"F\") } }\nfn main() { print(f()) }\n",
    );
    push(
        "pending_return__finally_return",
        "fn f() -> int { try { return 1 } catch e { return 2 } finally { return 7 } }\nfn main() { print(f()) }\n",
    );
    push(
        "pending_return__finally_throw",
        "fn f() -> int { try { return 1 } catch e { return 2 } finally { throw \"T\" } }\nfn main() { print(f()) }\n",
    );
    push(
        "pending_throw__finally_val",
        "fn f() -> int { try { throw \"x\" } catch e { return 2 } finally { print(\"F\") } }\nfn main() { print(f()) }\n",
    );
    push(
        "pending_throw__finally_return",
        "fn f() -> int { try { throw \"x\" } catch e { return 2 } finally { return 7 } }\nfn main() { print(f()) }\n",
    );
    push(
        "pending_throw__finally_throw",
        "fn f() -> int { try { throw \"x\" } catch e { return 2 } finally { throw \"T\" } }\nfn main() { print(f()) }\n",
    );
    push(
        "pending_break__finally_val",
        "fn f() -> int { let mut i = 0\n while i < 2 { i = i + 1\n try { break } catch e { print(\"caught\") } finally { print(\"F\") } }\n return i }\nfn main() { print(f()) }\n",
    );
    push(
        "pending_break__finally_return",
        "fn f() { let mut i = 0\n while i < 2 { i = i + 1\n try { break } catch e { print(\"caught\") } finally { return } } }\nfn main() { f(); print(\"after\") }\n",
    );
    push(
        "pending_break__finally_throw",
        "fn f() { let mut i = 0\n while i < 2 { i = i + 1\n try { break } catch e { print(\"caught\") } finally { throw \"T\" } } }\nfn main() { f(); print(\"unreached\") }\n",
    );
    push(
        "pending_continue__finally_val",
        "fn main() { let mut i = 0\n let mut s = 0\n while i < 3 { i = i + 1\n try { continue } catch e { print(\"caught\") } finally { s = s + i } }\n print(s) }\n",
    );
    push(
        "pending_continue__finally_return",
        "fn f() { let mut i = 0\n while i < 3 { i = i + 1\n try { continue } catch e { print(\"caught\") } finally { return } } }\nfn main() { f(); print(\"after\") }\n",
    );
    push(
        "pending_continue__finally_throw",
        "fn f() { let mut i = 0\n while i < 3 { i = i + 1\n try { continue } catch e { print(\"caught\") } finally { throw \"T\" } } }\nfn main() { f(); print(\"unreached\") }\n",
    );
    // Fatal error from finally propagates over a pending value.
    push(
        "pending_val__finally_fatal_error",
        "fn f() -> int { try { 1 } catch e { 2 } finally { let z = 1 / 0 } }\nfn main() { print(f()) }\n",
    );
    // Nested try inside finally.
    push(
        "nested_try_in_finally",
        "fn f() -> int { try { return 1 } catch e { return 0 } finally { try { throw \"x\" } catch f { print(f) } } }\nfn main() { print(f()) }\n",
    );
    // Call inside finally.
    push(
        "call_in_finally",
        "fn g() -> int { print(\"g\"); return 0 }\nfn f() -> int { try { return 1 } catch e { return 0 } finally { g() } }\nfn main() { print(f()) }\n",
    );
}
