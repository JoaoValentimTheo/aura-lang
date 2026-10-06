#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Backward-compatibility fixtures for the released language surface.
//!
//! `docs/engineering/RELEASE_ENGINEERING.md` requires that a released language
//! surface stay stable until a documented, versioned break. This file pins the
//! **0.2.0 Core language** by behavior: each case is a small program with an
//! exact expected result. If a future change alters any of these, this suite
//! fails, forcing an explicit compatibility decision rather than a silent
//! break.
//!
//! Diagnostics are pinned by **code**, not prose, so message wording may
//! improve without breaking the fixture, but the *rule* (accept/reject and
//! which code) may not drift.
//!
//! These fixtures are cumulative by language version. When a new language
//! version ships, add a section rather than editing the released one.
//!
//! Scope caveat: the `c020_*` fixtures describe the 0.2.0 Core language
//! surface as it stands on the current development line. Where a rule changed
//! *after* tag `v0.2.0` (notably builtin-name reservation), the fixture notes
//! it explicitly rather than claiming the released tag already had it. The
//! `c021_*` fixtures pin the **0.2.1** language additions; the version-identity
//! question is resolved by ADR-0001 (`docs/adr/0001-release-vs-language-version.md`).
//!
//! Upgrade expectation: a program valid under the `0.2.0` spelling stays valid
//! under `0.2.1` unless it relied on a binding that collided with a builtin
//! (never legal to rely on — it broke assignment lookup) or on an alias chain
//! that expanded past the semantic depth limit (never a real program). Both
//! are pinned below as intentional, documented breaks.

use aura::error::codes;
use aura::run_source;
use std::fmt::Write as _;

fn ok(src: &str) -> String {
    run_source(src, "<compat>").expect("program runs")
}

fn err(src: &str) -> u16 {
    run_source(src, "<compat>").unwrap_err().code
}

// ── 0.2.0 Core ─────────────────────────────────────────────────────────────

#[test]
fn c020_functions_and_recursion() {
    assert_eq!(
        ok("fn fib(n) -> int { if n < 2 { return n }\n return fib(n-1) + fib(n-2) }\nfn main() { print(fib(10)) }"),
        "55\n"
    );
    // Mutual recursion via hoisting.
    assert_eq!(
        ok("fn even(n) -> bool { if n == 0 { return true }\n return odd(n-1) }\nfn odd(n) -> bool { if n == 0 { return false }\n return even(n-1) }\nfn main() { print(even(10))\n print(odd(10)) }"),
        "true\nfalse\n"
    );
}

#[test]
fn c020_immutability_and_mutation() {
    assert_eq!(ok("fn main() { let x = 1\n print(x) }"), "1\n");
    assert_eq!(
        ok("fn main() { let mut x = 1\n x = x + 1\n print(x) }"),
        "2\n"
    );
    // Assigning to an immutable binding is E2001.
    assert_eq!(
        err("fn main() { let x = 1\n x = 2 }"),
        codes::ASSIGN_IMMUTABLE
    );
}

#[test]
fn c020_generic_map_keys() {
    // string, int, bool keys; distinct `1` and `"1"`.
    assert_eq!(
        ok("fn main() { let m = {1: \"a\", \"1\": \"b\"}\n print(m[1])\n print(m[\"1\"]) }"),
        "a\nb\n"
    );
    assert_eq!(
        ok("fn main() { let m = {true: 1, false: 0}\n print(m[true]) }"),
        "1\n"
    );
    // Non-key-capable key type is rejected.
    assert_eq!(
        err("fn main() { let m: {float: int} = {1.5: 1} }"),
        codes::TYPE_MISMATCH
    );
}

#[test]
fn c020_map_items_and_ordering() {
    assert_eq!(
        ok("fn main() { print(keys({3:\"c\",1:\"a\",2:\"b\"})) }"),
        "[1, 2, 3]\n"
    );
    assert_eq!(
        ok("fn main() { print({2:\"b\",1:\"a\"}.items()) }"),
        "[[1, \"a\"], [2, \"b\"]]\n"
    );
}

#[test]
fn c020_comprehensions() {
    assert_eq!(
        ok("fn main() { print([x * 2 for x in range(0, 3)]) }"),
        "[0, 2, 4]\n"
    );
    assert_eq!(
        ok("fn main() { print([x for x in range(0, 6) if x % 2 == 0]) }"),
        "[0, 2, 4]\n"
    );
    assert_eq!(
        ok("fn main() { print({k: k for k in range(0, 3)}) }"),
        "{0: 0, 1: 1, 2: 2}\n"
    );
    // Multiple generators are not part of the language.
    assert_eq!(
        err("fn main() { print([x for x in [1] for y in [2]]) }"),
        codes::EXPECTED
    );
}

#[test]
fn c020_generics_and_aliases() {
    assert_eq!(
        ok("fn id<T>(x: T) -> T { return x }\nfn main() { print(id(5))\n print(id(\"s\")) }"),
        "5\ns\n"
    );
    assert_eq!(
        ok("type Map<K, V> = {K: V}\nfn main() { let m: Map<int, string> = {1: \"a\"}\n print(m[1]) }"),
        "a\n"
    );
}

#[test]
fn c020_structs_enums_traits() {
    assert_eq!(
        ok("struct S { a: int }\nimpl S { fn double(self) -> int { return self.a * 2 } }\nfn main() { print(S { a: 5 }.double()) }"),
        "10\n"
    );
    assert_eq!(
        ok("enum E { A(int), B }\nfn main() { let v = A(7)\n print(match v { A(n) -> n, B -> 0 }) }"),
        "7\n"
    );
    assert_eq!(
        ok("trait T { fn id(self) -> int }\nstruct S { a: int }\nimpl T for S { fn id(self) -> int { return self.a } }\nfn main() { print(S { a: 3 }.id()) }"),
        "3\n"
    );
}

#[test]
fn c020_named_arguments() {
    assert_eq!(
        ok("fn f(a: int, b: int) -> int { return a - b }\nfn main() { print(f(b: 3, a: 10)) }"),
        "7\n"
    );
}

#[test]
fn c020_operators_and_precedence() {
    assert_eq!(ok("fn main() { print(2 + 3 * 4) }"), "14\n");
    assert_eq!(ok("fn main() { print(2 ^ 3 ^ 2) }"), "512\n");
    assert_eq!(ok("fn main() { print(-2 ^ 2) }"), "4\n");
    assert_eq!(ok("fn main() { print(true or false and false) }"), "true\n");
    assert_eq!(ok("fn main() { print(1 | 2 & 3) }"), "3\n");
}

#[test]
fn c020_pipeline_and_ranges() {
    assert_eq!(ok("fn main() { print([1,2,3] |> len) }"), "3\n");
    assert_eq!(ok("fn main() { print(len(range(0, 100))) }"), "100\n");
}

#[test]
fn c020_builtins_unchanged() {
    assert_eq!(ok("fn main() { print(sum([1,2,3])) }"), "6\n");
    assert_eq!(ok("fn main() { print(len([1,2,3])) }"), "3\n");
    assert_eq!(
        ok("fn main() { print(min(3, 7))\n print(max(3, 7)) }"),
        "3\n7\n"
    );
    assert_eq!(
        ok("fn main() { print(\"a,b\".split(\",\")) }"),
        "[\"a\", \"b\"]\n"
    );
}

#[test]
fn c020_builtin_name_reservation() {
    // NOTE: this fixture pins the **post-`v0.2.0` development line**, not the
    // released `v0.2.0` tag. The builtin-name value-namespace reservation
    // (`E1009`, commit `c8ded06`) landed *after* tag `v0.2.0`; `git tag
    // --contains c8ded06` is empty. At the released tag, `let sum = 1` was
    // accepted, and `let mut sum = 0; sum = sum + 1` was a misleading `E2001`.
    // The reservation is a justified fix: it intentionally rejects programs
    // that were never semantically well defined (the binding collided with a
    // builtin and broke assignment lookup). The version-identity question it
    // exposes (HEAD declares `0.2.0` but differs from tag `v0.2.0`) is tracked
    // in HUMAN_DECISIONS_QUEUE.md HD-4.
    assert_eq!(err("fn main() { let sum = 1 }"), codes::RESERVED_NAME);
    assert_eq!(
        err("fn main() { let mut sum = 0\n sum = sum + 1 }"),
        codes::RESERVED_NAME
    );
    // A near-miss identifier is fine.
    assert_eq!(ok("fn main() { let summary = 1\n print(summary) }"), "1\n");
}

#[test]
fn c020_syntax_rules() {
    // Statement separators required.
    assert_eq!(err("fn main() { let x = 1 let y = 2 }"), codes::EXPECTED);
    // Numeric underscore placement.
    assert_eq!(ok("fn main() { print(1_000) }"), "1000\n");
    assert_eq!(err("fn main() { print(1_) }"), codes::INVALID_NUMBER);
    // f-string brace escaping.
    assert_eq!(ok("fn main() { print(f\"{{literal}}\") }"), "{literal}\n");
    assert_eq!(err("fn main() { print(f\"}\") }"), codes::EXPECTED);
}

#[test]
fn c020_error_codes_are_stable() {
    // A representative set of released diagnostics, pinned by code.
    assert_eq!(err("fn main() { let x = y }"), codes::UNDEFINED);
    assert_eq!(err("fn main() { print(1 / 0) }"), codes::DIV_ZERO);
    assert_eq!(
        err("fn main() { print(9223372036854775807 + 1) }"),
        codes::OVERFLOW
    );
    // `run_program` (the `aura run` path) requires `main`; `E4027`.
    assert_eq!(
        aura::run_program("fn foo() { print(1) }", "<program>")
            .unwrap_err()
            .code,
        codes::NO_MAIN
    );
}

// ── 0.2.1 language additions ───────────────────────────────────────────────

/// Builtin-name reservation (`E1009`): the 0.2.1 language rejects a user value
/// binding that reuses a builtin name. This is a documented, intentional break
/// from the pre-`0.2.1` spelling (ADR-0002); a program that relied on such a
/// binding was never assignment-safe.
#[test]
fn c021_builtin_binding_is_reserved() {
    assert_eq!(err("fn main() { let len = 1 }"), codes::RESERVED_NAME);
    // Non-value namespaces are unaffected (ADR-0002): a module member may
    // reuse a builtin spelling.
    assert_eq!(
        ok("module M { pub fn len() -> int { return 1 } }\nfn main() { print(M::len()) }"),
        "1\n"
    );
    // A parameter is a value binding too.
    assert_eq!(
        err("fn f(print: int) -> int { return print }\nfn main() { print(1) }"),
        codes::RESERVED_NAME
    );
}

/// Unified structural nesting (ADR-0004): a type annotation nested past the
/// semantic AST limit is `E1015` on every substrate, and a flat union stays
/// accepted.
#[test]
fn c021_type_nesting_is_unified() {
    let deep = |n: usize| {
        let mut t = String::from("int");
        for _ in 0..n {
            t = format!("Box<{t}>");
        }
        format!("struct Box<T> {{ value: T }}\nfn f(_x: {t}) -> int {{ return 1 }}\nfn main() {{ print(1) }}")
    };
    assert!(run_source(&deep(255), "<compat>").is_ok());
    assert_eq!(
        run_source(&deep(256), "<compat>").unwrap_err().code,
        codes::NESTING
    );
}

/// Bound alias-chain resolution (train-1 hardening): a chain of wrapping
/// aliases past the semantic depth limit is `E1015`, never a host abort.
#[test]
fn c021_alias_chain_depth_is_bounded() {
    let chain = |n: usize| {
        let mut s = String::from("type A0 = int\n");
        for i in 1..=n {
            let _ = writeln!(s, "type A{i} = [A{}]", i - 1);
        }
        let _ = write!(
            s,
            "fn f(_x: A{n}) -> int {{ return 1 }}\nfn main() {{ print(1) }}\n"
        );
        s
    };
    assert!(run_source(&chain(255), "<compat>").is_ok());
    assert_eq!(
        run_source(&chain(20_000), "<compat>").unwrap_err().code,
        codes::NESTING
    );
}
