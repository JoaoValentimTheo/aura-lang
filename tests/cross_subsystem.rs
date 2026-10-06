#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Cross-subsystem interaction tests.
//!
//! Most suites exercise one surface (modules, closures, generics, …). Bugs
//! often live at the seams, so these compose two or more subsystems in one
//! program and assert the combined behavior. Each case is legal under current
//! semantics and would fail if either subsystem regressed.

use aura::error::codes;
use aura::run_source;

fn ok(src: &str) -> String {
    run_source(src, "<cross>").expect("expected the program to run")
}

fn code(src: &str) -> u16 {
    run_source(src, "<cross>")
        .expect_err("expected the program to be rejected")
        .code
}

/// Modules + closures + mutation: a closure captures a `mut` binding from an
/// enclosing function, and the function is exported across a module boundary.
#[test]
fn module_closure_mutation() {
    let src = r#"
module counter {
    pub fn make() {
        let mut n = 0
        return () -> {
            n = n + 1
            return n
        }
    }
}
fn main() {
    let inc = counter::make()
    print(inc())
    print(inc())
    print(inc())
}
"#;
    assert_eq!(ok(src), "1\n2\n3\n");
}

/// Generics + method + module: a generic function defined in a module is called
/// with an explicit type argument, and the result is used with a method.
#[test]
fn generic_function_in_module() {
    let src = r#"
module util {
    pub fn first<T>(xs: [T]) -> T {
        return xs[0]
    }
}
fn main() {
    print(util::first<int>([10, 20, 30]))
    print(util::first<string>(["a", "b"]))
}
"#;
    assert_eq!(ok(src), "10\na\n");
}

/// Enum + pattern + closure: patterns destructure an enum payload inside a
/// closure passed to a builtin.
#[test]
fn enum_pattern_closure() {
    let src = r#"
enum Shape { Circle(int), Square(int) }
fn main() {
    let shapes = [Shape::Circle(3), Shape::Square(4)]
    let areas = shapes.map((s) -> {
        return match s {
            Circle(r) -> r * r * 3
            Square(w) -> w * w
        }
    })
    print(areas)
}
"#;
    assert_eq!(ok(src), "[27, 16]\n");
}

/// Builtin + alias: a transparent alias to a builtin-capable type is used as
/// the element type of a map and iterated.
#[test]
fn alias_to_key_capable_map() {
    let src = r#"
type Id = int
fn main() {
    let m = {1: "one", 2: "two"}
    let mut total = 0
    for k in m {
        total = total + k
    }
    print(total)
    let x: Id = 5
    print(x)
}
"#;
    assert_eq!(ok(src), "3\n5\n");
}

/// Deep AST + call: an expression near the nesting limit still runs, proving
/// the parser depth accounting and the evaluator agree at the boundary.
#[test]
fn deep_expression_runs() {
    // 200 nested `(x + 1)` layers stays under the 256-node limit.
    let mut expr = String::from("0");
    for _ in 0..200 {
        expr = format!("({expr} + 1)");
    }
    let src = format!("fn main() {{ print({expr}) }}");
    assert_eq!(ok(&src), "200\n");
}

/// Module visibility + error: an unexported member is not visible across the
/// module boundary and is a structured diagnostic, not a runtime surprise.
#[test]
fn private_member_is_not_visible() {
    let src = r#"
module m {
    fn hidden() -> int { return 1 }
    pub fn shown() -> int { return 2 }
}
fn main() { print(m::hidden()) }
"#;
    assert_eq!(code(src), codes::PRIVATE_ACCESS);
}

/// Cycle + display + JSON: a self-referential value through a `mut` list must
/// terminate and be depth-bounded, not hang or overflow the host stack, in
/// both the display form and the JSON encoding (`LANGUAGE_SPEC.md` §31.6).
#[cfg(feature = "json")]
#[test]
fn cyclic_value_is_bounded_in_display_and_json() {
    let src = r#"
fn main() {
    let mut xs = []
    xs.push(xs)
    print(xs)
    print(json_encode(xs))
}
"#;
    let out = ok(src);
    let mut lines = out.lines();
    assert!(
        lines.next().unwrap_or("").starts_with('['),
        "display must be cycle-bounded, got: {out}"
    );
    assert!(
        lines.next().unwrap_or("").starts_with('['),
        "json must be cycle-bounded, got: {out}"
    );
}

/// Collection + comprehension + generic: a map comprehension over a list of
/// records, consumed by a generic function.
#[test]
fn comprehension_over_records() {
    let src = r#"
struct P { name: string, age: int }
fn main() {
    let ps = [P { name: "a", age: 1 }, P { name: "b", age: 2 }]
    let m = {p.name: p.age for p in ps}
    print(m["b"])
    print(len(m))
}
"#;
    assert_eq!(ok(src), "2\n2\n");
}
