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
