#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Builtin-name reservation: user-defined value-namespace bindings must not
//! use registered builtin names (human law). `E1009` (`RESERVED_NAME`).
//!
//! Value namespace (rejected): `let`/`let mut`, function parameters, lambda
//! parameters, loop bindings, catch bindings, pattern bindings, user
//! functions, top-level `let`, import aliases.
//!
//! Separate namespaces (accepted): structs, enums, type aliases, modules,
//! struct fields, enum variants, methods.

use aura::error::codes;
use aura::run_source;

fn code(src: &str) -> Result<String, u16> {
    run_source(src, "<builtin-reservation>").map_err(|d| d.code)
}

fn builtin_names() -> Vec<&'static str> {
    aura::stdlib::builtin_names()
}

#[test]
fn sum_minimal_is_reserved() {
    let src = "fn main() {\n    let mut sum = 0\n    sum = sum + 1\n    print(sum)\n}";
    assert_eq!(
        code(src),
        Err(codes::RESERVED_NAME),
        "user `let mut sum` must be E1009, not E2001 and not OK"
    );
}

#[test]
fn every_builtin_let_binding_is_reserved() {
    let mut failures = Vec::new();
    for name in builtin_names() {
        let src = format!("fn main() {{ let mut {name} = 0\n print({name}) }}");
        if code(&src) != Err(codes::RESERVED_NAME) {
            failures.push(name.to_string());
        }
    }
    assert!(
        failures.is_empty(),
        "value bindings using builtin names must be E1009: {failures:?}"
    );
}

#[test]
fn every_builtin_immutable_let_is_reserved() {
    let mut failures = Vec::new();
    for name in builtin_names() {
        let src = format!("fn main() {{ let {name} = 1\n print({name}) }}");
        if code(&src) != Err(codes::RESERVED_NAME) {
            failures.push(name.to_string());
        }
    }
    assert!(
        failures.is_empty(),
        "immutable `let` using builtin names must be E1009: {failures:?}"
    );
}

#[test]
fn every_builtin_param_is_reserved() {
    let mut failures = Vec::new();
    for name in builtin_names() {
        let src = format!("fn foo({name}: int) {{ print({name}) }}\nfn main() {{ foo(1) }}");
        if code(&src) != Err(codes::RESERVED_NAME) {
            failures.push(name.to_string());
        }
    }
    assert!(
        failures.is_empty(),
        "parameters using builtin names must be E1009: {failures:?}"
    );
}

#[test]
fn every_builtin_user_function_is_reserved() {
    let mut failures = Vec::new();
    for name in builtin_names() {
        let src = format!("fn {name}() {{ return 1 }}\nfn main() {{ print(1) }}");
        if code(&src) != Err(codes::RESERVED_NAME) {
            failures.push(name.to_string());
        }
    }
    assert!(
        failures.is_empty(),
        "user functions using builtin names must be E1009: {failures:?}"
    );
}

#[test]
fn every_builtin_top_level_let_is_reserved() {
    let mut failures = Vec::new();
    for name in builtin_names() {
        let src = format!("let {name} = 1\nfn main() {{ print(1) }}");
        if code(&src) != Err(codes::RESERVED_NAME) {
            failures.push(name.to_string());
        }
    }
    assert!(
        failures.is_empty(),
        "top-level `let` using builtin names must be E1009: {failures:?}"
    );
}

#[test]
fn every_builtin_loop_binding_is_reserved() {
    let mut failures = Vec::new();
    for name in builtin_names() {
        let src = format!("fn main() {{ for {name} in [1] {{ print({name}) }} }}");
        if code(&src) != Err(codes::RESERVED_NAME) {
            failures.push(name.to_string());
        }
    }
    assert!(
        failures.is_empty(),
        "loop bindings using builtin names must be E1009: {failures:?}"
    );
}

#[test]
fn every_builtin_list_pattern_is_reserved() {
    let mut failures = Vec::new();
    for name in builtin_names() {
        let src = format!("fn main() {{ let [{name}, x] = [1, 2]\n print({name}) }}");
        if code(&src) != Err(codes::RESERVED_NAME) {
            failures.push(name.to_string());
        }
    }
    assert!(
        failures.is_empty(),
        "pattern bindings using builtin names must be E1009: {failures:?}"
    );
}

#[test]
fn import_alias_to_builtin_is_reserved() {
    let src =
        "module M { pub fn foo() { return 1 } }\nuse M::foo as sum\nfn main() { print(sum()) }";
    assert_eq!(code(src), Err(codes::RESERVED_NAME));
}

#[test]
fn type_namespace_does_not_collide_with_builtins() {
    // `struct sum` lives in the type namespace, which is separate from the
    // value namespace that builtins occupy (`LANGUAGE_SPEC.md` §26).
    // Lowercase struct spelling is unconventional but must remain accepted.
    let src = "struct sum { a: int }\nfn main() { print(1) }";
    assert!(
        code(src).is_ok(),
        "type namespace must allow `sum`: {:?}",
        code(src)
    );
}

#[test]
fn member_namespaces_do_not_collide_with_builtins() {
    // Fields, variants, and methods are member namespaces, not the value
    // namespace.
    assert!(
        code("struct S { sum: int }\nfn main() { let s = S { sum: 1 }\n print(s.sum) }").is_ok()
    );
    assert!(code("enum E { sum, other }\nfn main() { print(1) }").is_ok());
    assert!(code("struct S { a: int }\nimpl S { fn sum(self) { return self.a } }\nfn main() { let s = S { a: 1 }\n print(s.sum()) }").is_ok());
}

#[test]
fn modules_do_not_collide_with_builtins() {
    assert!(
        code("module sum { pub fn foo() { return 1 } }\nfn main() { print(sum::foo()) }").is_ok()
    );
}

#[test]
fn normal_names_still_work() {
    for name in ["tot", "x", "foo", "bar", "counter", "running"] {
        let src = format!("fn main() {{ let mut {name} = 0\n {name} = 1\n print({name}) }}");
        assert!(code(&src).is_ok(), "{name} should be OK");
    }
}

#[test]
fn builtin_calls_still_work() {
    // The reservation forbids *bindings*, not *calls*.
    assert!(code("fn main() { print(sum([1, 2])) }").is_ok());
    assert!(code("fn main() { print(len([1])) }").is_ok());
}
