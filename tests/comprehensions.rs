#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Simple list and map comprehensions (LANGUAGE_SPEC §24-§28).
//!
//! One generator clause and zero or one filter; eager; the iterable is
//! evaluated exactly once; ordinary `for` pattern and truthiness semantics;
//! result order follows iteration; pattern bindings do not leak.

use aura::error::codes;
use aura::run_source;

fn ok(src: &str) -> String {
    run_source(src, "<comprehensions>").expect("expected the program to run")
}

fn code(src: &str) -> u16 {
    run_source(src, "<comprehensions>")
        .expect_err("expected the program to be rejected")
        .code
}

// --------------------------------------------------------- list comprehensions

#[test]
fn list_comprehension_transforms_every_element() {
    assert_eq!(
        ok("fn main() { let xs = [1, 2, 3]\n print([x * 2 for x in xs]) }"),
        "[2, 4, 6]\n"
    );
}

#[test]
fn list_comprehension_filter_uses_ordinary_truthiness() {
    assert_eq!(
        ok("fn main() { print([x for x in [1, 2, 3, 4] if x > 2]) }"),
        "[3, 4]\n"
    );
}

#[test]
fn list_comprehension_destructures_a_pattern() {
    assert_eq!(
        ok("fn main() { let ps = [[1, \"a\"], [2, \"b\"]]\n print([name for [n, name] in ps if n > 1]) }"),
        "[\"b\"]\n"
    );
}

#[test]
fn list_comprehension_preserves_iteration_order() {
    assert_eq!(
        ok("fn main() { print([x for x in {3: 0, 1: 0, 2: 0}]) }"),
        "[1, 2, 3]\n"
    );
}

#[test]
fn list_comprehension_over_an_empty_iterable_is_empty() {
    assert_eq!(ok("fn main() { print([x for x in []]) }"), "[]\n");
}

// ---------------------------------------------------------- map comprehensions

#[test]
fn map_comprehension_builds_a_map() {
    assert_eq!(
        ok("fn main() { print({x: x * x for x in range(1, 4)}) }"),
        "{1: 1, 2: 4, 3: 9}\n"
    );
}

#[test]
fn map_comprehension_over_items_uses_the_pair_pattern() {
    assert_eq!(
        ok("fn main() { let m = {1: 10, 2: 20}\n print({k: v * 2 for [k, v] in m.items()}) }"),
        "{1: 20, 2: 40}\n"
    );
}

#[test]
fn map_comprehension_filter_applies() {
    assert_eq!(
        ok("fn main() { print({x: x for x in [1, 2, 3, 4] if x % 2 == 0}) }"),
        "{2: 2, 4: 4}\n"
    );
}

#[test]
fn map_comprehension_duplicate_keys_follow_ordinary_insertion() {
    // Constant key, varying value: ordinary last-wins insertion.
    assert_eq!(
        ok("fn main() { print({1: x for x in [2, 3]}) }"),
        "{1: 3}\n"
    );
}

#[test]
fn map_comprehension_rejects_a_non_key_capable_generated_key() {
    assert_eq!(
        code("fn main() { print({1.0: x for x in [1]}) }"),
        codes::TYPE_MISMATCH
    );
    assert_eq!(
        code("fn main() { print({[x]: x for x in [1]}) }"),
        codes::TYPE_MISMATCH
    );
}

// --------------------------------------------------------------- scope / semantics

#[test]
fn comprehension_bindings_do_not_leak() {
    assert_eq!(
        code("fn main() { let ys = [x for x in [1, 2]]\n print(x) }"),
        codes::UNDEFINED
    );
}

#[test]
fn comprehension_pattern_is_assertive_like_ordinary_for() {
    // A list pattern of arity 1 cannot match a 1-element list of arity 2.
    assert_ne!(
        run_source("fn main() { let _ = [a for [a, _] in [[1]]] }", "<c>").err_code(),
        None
    );
}

#[test]
fn comprehension_over_a_non_iterable_is_rejected() {
    assert_eq!(
        code("fn main() { print([x for x in 1]) }"),
        codes::NOT_ITERABLE
    );
}

#[test]
fn comprehension_throw_propagates() {
    assert_eq!(
        code("fn f(_) { throw \"boom\" }\nfn main() { print([f(x) for x in [1]]) }"),
        codes::FOREIGN
    );
}

#[test]
fn comprehension_iterable_is_evaluated_once() {
    // A side-effecting iterable must run once, not once per element.
    assert_eq!(
        ok("fn counted() -> [int] { print(\"iterate\")\n return [1, 2, 3] }\nfn main() { print([x for x in counted()]) }"),
        "iterate\n[1, 2, 3]\n"
    );
}

/// Helper extension so a test can assert an error code without unwrapping.
trait ErrCode {
    fn err_code(&self) -> Option<u16>;
}
impl ErrCode for Result<String, aura::error::Diag> {
    fn err_code(&self) -> Option<u16> {
        self.as_ref().err().map(|d| d.code)
    }
}
