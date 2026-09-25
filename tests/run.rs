#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! End-to-end execution tests.

use aura::error::codes;
use aura::error::Diag;
use aura::run_source;

fn out(src: &str) -> String {
    run_source(src, "<test>").expect("program runs")
}

fn fails(src: &str) -> Diag {
    run_source(src, "<test>").expect_err("program is rejected")
}

#[test]
fn hello_world() {
    assert_eq!(
        out("fn main() { print(\"Hello, Aura!\") }"),
        "Hello, Aura!\n"
    );
}

#[test]
fn arithmetic_and_precedence() {
    assert_eq!(out("fn main() { print(1 + 2 * 3) }"), "7\n");
    assert_eq!(out("fn main() { print(2 ^ 3 ^ 2) }"), "512\n");
    assert_eq!(out("fn main() { print(7 / 2) }"), "3\n");
    assert_eq!(out("fn main() { print(7 % 3) }"), "1\n");
}

#[test]
fn integer_overflow_errors() {
    let d = fails("fn main() { let mut x = 9223372036854775807\n print(x + 1) }");
    assert_eq!(d.code, aura::error::codes::OVERFLOW);
}

#[test]
fn division_by_zero_errors() {
    let d = fails("fn main() { print(1 / 0) }");
    assert_eq!(d.code, aura::error::codes::DIV_ZERO);
}

#[test]
fn immutability_is_enforced() {
    let d = fails("fn main() { let x = 1\n x = 2 }");
    assert_eq!(d.code, aura::error::codes::ASSIGN_IMMUTABLE);
    assert_eq!(out("fn main() { let mut x = 1\n x = 2\n print(x) }"), "2\n");
}

#[test]
fn undefined_name_is_rejected_before_running() {
    let d = fails("fn main() { print(nope) }");
    assert_eq!(d.code, aura::error::codes::UNDEFINED);
}

#[test]
fn functions_closures_and_pipes() {
    let src = r#"
fn inc(x) -> int { return x + 1 }
fn main() {
    let add = (a, b) -> a + b
    print(add(2, 3))
    print([1, 2, 3] |> len)
    print([1, 2, 3].map((x) -> x * 2))
    print([1, 2, 3, 4].filter((x) -> x % 2 == 0))
    print([1, 2, 3, 4].reduce((acc, x) -> acc + x, 0))
    print(inc(41))
}
"#;
    assert_eq!(out(src), "5\n3\n[2, 4, 6]\n[2, 4]\n10\n42\n");
}

#[test]
fn closures_capture_environment() {
    let src = r#"
fn make_adder(n) { return (x) -> x + n }
fn main() {
    let add10 = make_adder(10)
    print(add10(5))
}
"#;
    assert_eq!(out(src), "15\n");
}

#[test]
fn recursion_fibonacci() {
    let src = r#"
fn fib(n) -> int {
    if n < 2 { return n }
    return fib(n - 1) + fib(n - 2)
}
fn main() { print(fib(10)) }
"#;
    assert_eq!(out(src), "55\n");
}

#[test]
fn for_loops_and_ranges() {
    let src = r#"
fn main() {
    let mut total = 0
    for i in range(1, 5) { total = total + i }
    print(total)
    for c in "abc" { print(c) }
    for k in {"a": 1, "b": 2} { print(k) }
}
"#;
    assert_eq!(out(src), "10\na\nb\nc\na\nb\n");
}

#[test]
fn while_and_break() {
    let src = r#"
fn main() {
    let mut i = 0
    while true {
        i = i + 1
        if i == 3 { break }
    }
    print(i)
}
"#;
    assert_eq!(out(src), "3\n");
}

#[test]
fn pattern_matching() {
    let src = r#"
fn classify(n) -> string {
    return match n {
        0 -> "zero"
        1 -> "one"
        x if x > 100 -> "big"
        _ -> "other"
    }
}
fn main() {
    print(classify(0))
    print(classify(1))
    print(classify(200))
    print(classify(5))
}
"#;
    assert_eq!(out(src), "zero\none\nbig\nother\n");
}

#[test]
fn structs_and_enums() {
    let src = r#"
struct Point { x: int, y: int }
enum Shape { Circle(int), Square(int) }
fn area(s) -> int {
    return match s {
        Circle(r) -> 3 * r * r
        Square(a) -> a * a
    }
}
fn main() {
    let p = Point { x: 3, y: 4 }
    print(p.x + p.y)
    print(area(Circle(2)))
    print(area(Square(3)))
}
"#;
    assert_eq!(out(src), "7\n12\n9\n");
}

#[test]
fn try_catch_finally() {
    let src = r#"
fn main() {
    try {
        throw "boom"
    } catch e -> {
        print(f"caught {e}")
    } finally {
        print("cleanup")
    }
}
"#;
    assert_eq!(out(src), "caught boom\ncleanup\n");
}

#[test]
fn finally_control_flow_overrides_the_pending_outcome() {
    // F-07: a control-flow signal raised in `finally` replaces the pending
    // result. `return` in `finally` wins over the `return` in `try`.
    let src = r#"
fn f() -> int {
    try {
        return 1
    } catch e -> {
        return 0
    } finally {
        return 2
    }
}
fn main() { print(f()) }
"#;
    assert_eq!(out(src), "2\n");

    // A `throw` in `finally` replaces a caught throw.
    let src = r#"
fn main() {
    try {
        try {
            throw "a"
        } catch e -> {
            print("caught a")
        } finally {
            throw "b"
        }
    } catch e -> {
        print(f"outer {e}")
    }
}
"#;
    assert_eq!(out(src), "caught a\nouter b\n");
}

#[test]
fn pub_and_use_are_inert_but_parse() {
    // F-08/F-13: `pub` on any item and `use` anywhere are accepted and have
    // no effect.
    let src = r#"
use stdlib
use a.b.c
pub fn helper() -> int { return 41 }
pub struct S { x: int }
pub enum E { A(int) }
pub type Id = int
fn main() {
    print(helper() + 1)
    print(S { x: 1 }.x)
    print(match A(1) {
        A(n) -> n
        _ -> 0
    })
}
"#;
    assert_eq!(out(src), "42\n1\n1\n");
}

#[test]
fn string_methods_and_interpolation() {
    let src = r#"
fn main() {
    let name = "Aura"
    print(f"hi {name}")
    print("  x  ".trim())
    print("a,b,c".split(","))
    print("abc".upper())
}
"#;
    assert_eq!(out(src), "hi Aura\nx\n[\"a\", \"b\", \"c\"]\nABC\n");
}

#[test]
fn named_arguments_for_structs() {
    let src = r#"
struct User { name: string, age: int }
fn main() {
    let u = User { name: "ana", age: 30 }
    print(u.name)
    print(u.age)
}
"#;
    assert_eq!(out(src), "ana\n30\n");
}

#[test]
fn lists_maps_indexing() {
    let src = r#"
fn main() {
    let xs = [10, 20, 30]
    xs.push(40)
    print(xs[0])
    print(xs[-1])
    print(xs.len())
    let m = {"k": 1}
    m["j"] = 2
    print(m.get("j"))
    print(m.has("k"))
}
"#;
    assert_eq!(out(src), "10\n40\n4\n2\ntrue\n");
}

#[test]
fn fstring_and_unicode() {
    assert_eq!(out("fn main() { print(f\"{1 + 1}\") }"), "2\n");
    assert_eq!(out("fn main() { print(\"ação\") }"), "ação\n");
    assert_eq!(out("fn main() { print(len(\"ação\")) }"), "4\n");
}

// ---------------------------------------------------------------------------
// FEATURE_004: destructuring `let` (`LANGUAGE_SPEC.md` §4.7).
// ---------------------------------------------------------------------------

#[test]
fn destructuring_list_binds_each_name() {
    assert_eq!(
        out("fn main() { let [a, b] = [1, 2]\n print(a + b) }"),
        "3\n"
    );
}

#[test]
fn destructuring_nested_list() {
    assert_eq!(
        out("fn main() { let [a, [b, c]] = [1, [2, 3]]\n print(a + b + c) }"),
        "6\n"
    );
}

#[test]
fn destructuring_variant() {
    assert_eq!(
        out("enum E { A(int), B(int) }\nfn main() { let A(x) = A(5)\n print(x) }"),
        "5\n"
    );
}

#[test]
fn destructuring_nested_variant_and_list() {
    assert_eq!(
        out("enum E { A(int), B(int) }\nfn main() { let [A(x), B(y)] = [A(1), B(2)]\n print(x + y) }"),
        "3\n"
    );
}

#[test]
fn destructuring_underscore_binds_nothing() {
    assert_eq!(out("fn main() { let [_, b] = [1, 2]\n print(b) }"), "2\n");
}

#[test]
fn destructuring_runtime_mismatches_are_e3001() {
    assert_eq!(
        fails("fn main() { let [a, b] = [1] }").code,
        codes::TYPE_MISMATCH
    );
    assert_eq!(
        fails("fn main() { let [a, b] = 5 }").code,
        codes::TYPE_MISMATCH
    );
    assert_eq!(
        fails("enum E { A(int) }\nfn main() { let A(x) = 5 }").code,
        codes::TYPE_MISMATCH
    );
    assert_eq!(
        fails("enum E { A(int) }\nfn main() { let A(x) = A(1, 2) }").code,
        codes::TYPE_MISMATCH
    );
}

#[test]
fn destructuring_rhs_evaluated_once() {
    // `count()` is called exactly once; if the RHS were evaluated per name the
    // printed value would exceed 1.
    assert_eq!(
        out("fn f() { print(\"x\")\n return [1, 2] }\nfn main() { let [a, b] = f()\n print(a + b) }"),
        "x\n3\n"
    );
}

#[test]
fn destructuring_failure_is_e3001_and_binds_nothing() {
    // A failed destructuring is the existing runtime `E3001`; the REPL tests
    // prove that no partial binding is left behind (the checker rejects a
    // same-scope redeclaration before execution, so atomicity is observed
    // across REPL submissions where fresh names are used).
    let err = fails("fn main() { let [a, [b, c]] = [1, [2]] }");
    assert_eq!(err.code, codes::TYPE_MISMATCH);
}

#[test]
fn destructuring_scope_subsequent_statements() {
    assert_eq!(
        out("fn main() { let [a, b] = [1, 2]\n let c = a + b\n print(c) }"),
        "3\n"
    );
}

#[test]
fn destructuring_nested_block_sees_names() {
    assert_eq!(
        out("fn main() { let [a, b] = [1, 2]\n while false { }\n print(a + b) }"),
        "3\n"
    );
}

#[test]
fn destructuring_shadows_outer_in_nested_scope() {
    assert_eq!(
        out(
            "fn main() { let a = 100\n if true { let [a, b] = [1, 2]\n print(a + b) }\n print(a) }"
        ),
        "3\n100\n"
    );
}

// ---------------------------------------------------------------------------
// FEATURE_005: empty-map literal runtime (`LANGUAGE_SPEC.md` §20.3).
// ---------------------------------------------------------------------------

#[test]
fn empty_map_literal_is_an_empty_map() {
    assert_eq!(out("fn main() { print(len({:})) }"), "0\n");
    assert_eq!(out("fn main() { print({:}.has(\"x\")) }"), "false\n");
    assert_eq!(out("fn main() { print({:}.keys()) }"), "[]\n");
    assert_eq!(out("fn main() { print({:}.values()) }"), "[]\n");
    assert_eq!(out("fn main() { print({:} == {:}) }"), "true\n");
    assert_eq!(out("fn main() { print({:} == {\"a\": 1}) }"), "false\n");
}

#[test]
fn empty_map_literal_whitespace_variants_are_equal() {
    assert_eq!(out("fn main() { print({:} == { : }) }"), "true\n");
    assert_eq!(out("fn main() { print({:} == {\n:\n}) }"), "true\n");
}

#[test]
fn empty_map_missing_key_uses_existing_e2003() {
    assert_eq!(fails("fn main() { {:}[\"x\"] }").code, codes::UNDEFINED);
}

#[test]
fn empty_braces_remain_a_block_value() {
    // `{}` yields `none`, not an empty map.
    assert_eq!(out("fn main() { print({}) }"), "none\n");
    assert_eq!(out("fn main() { print({} == none) }"), "true\n");
    assert_eq!(out("fn main() { print({} == {:}) }"), "false\n");
}

#[test]
fn non_empty_map_literal_is_unchanged() {
    assert_eq!(
        out("fn main() { let m = {\"a\": 1, \"b\": 2}\n print(m.len()) }"),
        "2\n"
    );
}

// ---------------------------------------------------------------------------
// H1a: pattern correctness at runtime is consistent with the checker.
// ---------------------------------------------------------------------------

/// `match` on a real variant still matches (no regression), and the checker
/// no longer lets a non-variant tag reach a silent fall-through.
#[test]
fn variant_pattern_matching_is_unchanged() {
    assert_eq!(
        out("enum E { A(int), B(int) }\nfn main() { print(match A(5) { A(v) -> v\n B(w) -> w }) }"),
        "5\n"
    );
    assert_eq!(
        out("enum E { A, B }\nfn main() { print(match B() { A -> \"a\"\n B -> \"b\" }) }"),
        "b\n"
    );
}

/// A nullary variant whose tag equals a declared struct/enum type name is a
/// real variant tag and still works.
#[test]
fn variant_tag_equal_to_type_name_matches() {
    assert_eq!(
        out("enum E { E(int) }\nfn main() { print(match E(7) { E(v) -> v }) }"),
        "7\n"
    );
}

// ---------------------------------------------------------------------------
// FEATURE_006: `else if` runtime behavior (`LANGUAGE_SPEC.md` §4.5).
// ---------------------------------------------------------------------------

#[test]
fn else_if_single_and_multiple_clauses() {
    let src = "fn grade(n) { if n > 90 { return \"A\" } else if n > 80 { return \"B\" } else if n > 70 { return \"C\" } else { return \"F\" } }\n\
               fn main() { print(grade(95))\n print(grade(85))\n print(grade(75))\n print(grade(10)) }";
    assert_eq!(out(src), "A\nB\nC\nF\n");
}

/// A chain with no truthy condition and no final `else` yields `none`.
#[test]
fn else_if_without_final_else_yields_none() {
    assert_eq!(
        out("fn main() { let x = if false { 1 } else if false { 2 }\n print(x) }"),
        "none\n"
    );
}

/// Conditions are evaluated in source order, at most once, and later
/// conditions only when all earlier ones were falsy.
#[test]
fn else_if_condition_evaluation_order() {
    let src = "fn c(n) { print(f\"c{n}\")\n return n == 2 }\n\
               fn main() { if c(1) { print(\"A\") } else if c(2) { print(\"B\") } else if c(3) { print(\"C\") } else { print(\"D\") } }";
    // c1 falsy, c2 truthy: c3 is never evaluated.
    assert_eq!(out(src), "c1\nc2\nB\n");
}

/// Each clause body is its own scope; an inner `let` shadows an outer binding
/// and does not leak.
#[test]
fn else_if_branch_scope_and_shadowing() {
    let src =
        "fn main() { let x = 1\n if false { } else if true { let x = 2\n print(x) }\n print(x) }";
    assert_eq!(out(src), "2\n1\n");
}

/// `return` through an `else if` returns from the enclosing function.
#[test]
fn else_if_return_propagates() {
    let src = "fn f(n) { if n == 1 { return 10 } else if n == 2 { return 20 } else { return 30 } }\nfn main() { print(f(2)) }";
    assert_eq!(out(src), "20\n");
}

/// `throw` through an `else if` is catchable by an enclosing `try`.
#[test]
fn else_if_throw_propagates() {
    let src =
        "fn main() { try { if false { } else if true { throw \"x\" } } catch e -> { print(e) } }";
    assert_eq!(out(src), "x\n");
}

/// `break`/`continue` through an `else if` affect the enclosing loop.
#[test]
fn else_if_break_and_continue() {
    let src = "fn main() { let mut out = 0\n for i in range(0, 10) { if i == 2 { continue } else if i == 5 { break } else { out = out + i } }\n print(out) }";
    // i = 0,1,3,4 summed; i=2 continued; i=5 breaks.
    assert_eq!(out(src), "8\n");
}

/// `finally` runs on the exit path through an `else if` chain.
#[test]
fn else_if_try_finally_interaction() {
    let src = "fn f() -> int { for i in range(0, 3) { try { if i == 1 { break } else if i == 0 { continue } } catch e -> { } finally { print(f\"f{i}\") } }\n return 7 }\nfn main() { print(f()) }";
    // i=0: continue -> finally prints f0; i=1: break -> finally prints f1; returns 7.
    assert_eq!(out(src), "f0\nf1\n7\n");
}

/// `range(n)` is `range(0, n)`, displayed `start..end` (`LANGUAGE_SPEC.md`
/// §22.1), and a descending range is empty rather than reversed or an error.
#[test]
fn range_single_argument_display_and_direction() {
    assert_eq!(out("fn main() { print(range(3)) }"), "0..3\n");
    assert_eq!(out("fn main() { print(range(2, 5)) }"), "2..5\n");
    assert_eq!(
        out("fn main() { print(len(range(3, 1)))\n for i in range(3, 1) { print(i) }\n print(\"done\") }"),
        "0\ndone\n"
    );
    assert_eq!(
        out("fn main() { for i in range(-2, 1) { print(i) } }"),
        "-2\n-1\n0\n"
    );
}

/// Ranges compare by equality (start and end), are never equal to a list,
/// and are not orderable (§22.1): `<` on two ranges is `E3001`.
#[test]
fn range_equality_and_unorderability() {
    assert_eq!(
        out("fn main() { print(range(1, 3) == range(1, 3))\n print(range(1, 3) == range(1, 4))\n print(range(0, 2) == [0, 1]) }"),
        "true\nfalse\nfalse\n"
    );
    let d = fails("fn main() { print(range(0, 2) < range(0, 3)) }");
    assert_eq!(d.code, codes::TYPE_MISMATCH);
}

/// The documented `range(a, b)` edge cases (`LANGUAGE_SPEC.md` §22.1): an
/// empty range, a single-element range, and a ten-element range all obey
/// start-inclusive/end-exclusive, half-open semantics.
#[test]
fn range_edge_bounds_are_half_open() {
    // range(0, 0) is empty; len is 0 and iteration yields nothing.
    assert_eq!(
        out("fn main() { print(len(range(0, 0)))\n for i in range(0, 0) { print(i) }\n print(\"done\") }"),
        "0\ndone\n"
    );
    // range(0, 1) is the single element 0.
    assert_eq!(
        out("fn main() { print(len(range(0, 1)))\n for i in range(0, 1) { print(i) } }"),
        "1\n0\n"
    );
    // range(0, 10) is 0..=9.
    assert_eq!(
        out("fn main() { print(len(range(0, 10)))\n let mut s = 0\n for i in range(0, 10) { s = s + i }\n print(s) }"),
        "10\n45\n"
    );
    // The end bound is never yielded.
    assert_eq!(
        out("fn main() { for i in range(0, 2) { print(i) }\n print(\"end\") }"),
        "0\n1\nend\n"
    );
}

/// A large range is iterated lazily (§22.2): an immediate `break` returns
/// before materializing the range, so a range far past the materialization cap
/// still terminates.
#[test]
fn large_range_iterates_lazily_with_early_break() {
    assert_eq!(
        out("fn main() { let mut n = 0\n for i in range(0, 100000000) { n = n + 1\n if i == 2 { break } }\n print(n) }"),
        "3\n"
    );
}

/// A closure captures its defining environment by reference (§15.5): it
/// observes later mutations of a `let mut` binding it closes over.
#[test]
fn closure_observes_later_mutation_of_captured_variable() {
    let src = "fn main() { let mut x = 1\n let f = () -> x\n x = 2\n print(f()) }";
    assert_eq!(out(src), "2\n");
}

/// Each `for` iteration binds a fresh environment (§16.3): closures created
/// per iteration keep their own copy of the loop variable.
#[test]
fn for_loop_bindings_are_per_iteration() {
    let src = "fn main() { let mut fs = []\n for i in range(3) { fs.push(() -> i) }\n for f in fs { print(f()) } }";
    assert_eq!(out(src), "0\n1\n2\n");
}

/// Capture is by reference (§15.5), so a `while` loop counter declared
/// outside the loop is one shared binding: every closure sees the final
/// value. This is the deliberate contrast with the per-iteration `for`
/// binding above.
#[test]
fn while_loop_counter_is_a_shared_captured_binding() {
    let src = "fn main() { let mut fs = []\n let mut i = 0\n while i < 3 { fs.push(() -> i)\n i = i + 1 }\n for f in fs { print(f()) } }";
    assert_eq!(out(src), "3\n3\n3\n");
}

/// A closure may assign to a captured `let mut` binding (capture by
/// reference), and the write is visible outside; assigning to a captured
/// immutable binding is a static `E2001`.
#[test]
fn closure_can_write_captured_mutable_binding() {
    let src = "fn main() { let mut x = 1\n let f = () -> { x = 5 }\n f()\n print(x) }";
    assert_eq!(out(src), "5\n");
    let d = fails("fn main() { let x = 1\n let f = () -> { x = 5 }\n f() }");
    assert_eq!(d.code, codes::ASSIGN_IMMUTABLE);
}

/// A `match` arm's pattern bindings live only in the arm scope; using the
/// bound name after the match is a static `E2003`.
#[test]
fn match_arm_bindings_do_not_leak() {
    let d = fails("fn main() { match [1, 2] { [a, b] -> { print(a + b) } }\n print(a) }");
    assert_eq!(d.code, codes::UNDEFINED);
    assert_eq!(
        out("fn main() { let a = 9\n match [1, 2] { [a, b] -> { print(a) } }\n print(a) }"),
        "1\n9\n"
    );
}

/// A `catch` binding lives only in the catch scope; using it afterwards is a
/// static `E2003`.
#[test]
fn catch_binding_does_not_leak() {
    let d = fails("fn main() { try { throw \"e\" } catch err -> { print(err) }\n print(err) }");
    assert_eq!(d.code, codes::UNDEFINED);
}
