---
layout: default
title: "Semantics — Execution Model"
parent: Aura Language Reference
nav_order: 9
---

[English](semantics.md) · [Português](semantics.pt_BR.md)

# Semantics — Execution Model

**Status:** Stable (except where labeled) · **Evidence:**
`aura/parser/to_ast.py` (`parse_module_decl`, import parsing),
`aura/transpiler/transformers/statements.py` (`transform_Module`,
`transform_ImportStmt`, `transform_IfStmt`, `transform_TryStmt`),
`aura/transpiler/rules.py` (`RuleChecker`), `aura/transpiler/semantics.py`
(`MutabilityChecker`), `aura/cli.py` (`cmd_run`, `_prepare_entrypoint`).

This document defines **the meaning of an Aura program** — what happens when it
runs. Aura is **transpiled to Python and executed on CPython**, so some
observable behaviour comes from the host rather than from the language; those
points are labeled **Target-specific** or **UNSPECIFIED** rather than hidden.

---

## 1. Start and end of execution

1. A program is **one entry file**, executed with `aura run`. The runtime parses,
   checks, transpiles to Python, and `exec`s the result on CPython
   (`cmd_run`, `cli.py:480-552`).
2. The entry file must declare a top-level **`main`**; the runtime calls it. The
   programmer never writes a trailing `main()` call (`_prepare_entrypoint`,
   `cli.py:388-468`).
3. `main` may be `sync` or `async`; an `async def main` is awaited inside an
   event loop (`cmd_run`, `cli.py:526-543`).
4. `main` statements execute **sequentially, in source order**.
5. If `main` returns an `int`, it becomes the process exit code; otherwise the
   program exits 0 (`_prepare_entrypoint`, `cli.py:463-467`).
6. A top-level `guard cond else { return }` rewrites the bare `return` to
   `raise SystemExit(...)`, which `cmd_run` treats as a **successful** exit
   (`statements.py:1137-1160`, `cli.py:554-562`).

```aura
def main() -> int {
  return 3          // process exit code 3
}
```

*Evidence:* `test_main_entrypoint.py::test_main_return_value_is_exit_code`,
`::test_runtime_invokes_main`, `::test_async_main_is_awaited`.

### 1.1 Signature rules

| Entry point | Result |
| --- | --- |
| `def main() { }` | OK |
| `async def main() { }` | OK |
| `def main(args: [string]) { }` | OK; `args` receives CLI arguments |
| Missing `main` in the entry file | **E310** |
| `main` with any other signature | **E311** |
| `def main` inside a `module` body | **E312** |

*Evidence:* `rules.py:_check_main` §162-205;
`test_main_entrypoint.py::test_missing_main_is_rejected`,
`::test_main_with_two_params_is_rejected`,
`::test_main_with_variadic_is_rejected`;
`test_module_facade.py::test_main_inside_a_module_reports_e312`.

> `main` may take **no parameters**, or a single `args` parameter; a variadic or
> non-`args` parameter is E311 (`rules.py:191-205`). `aura test` runs test files
> with `require_main=False`, since a test file drives itself.

---

## 2. Evaluation order

- **Binary operands** are evaluated **left to right**.
- **Call arguments** are evaluated in order, left→right.
- **`and` / `or`** are **short-circuit**: the right side is not evaluated when
  the left decides the result.
- **Assignment**: the right side is evaluated before the write to the left side.
- **Postfix chaining** (`a.b().c()[d]`) is evaluated left to right, receiver
  before member.

```aura
let r = mark(1) + mark(2)   // prints 1 then 2
let s = false and side()    // side() never runs
```

*Evidence:* *probe* (`1\n2\n`, and no output from `side()`);
[expressions.md](expressions.md).

---

## 3. Scope and lifetime

- A **block** `{ … }` opens a scope; declarations live until the end of the
  block (`_visit_body`, `rules.py:1249-1268`; `MutabilityChecker._with_scope`,
  `semantics.py:492-497`).
- A **function** body is its own scope; parameters are local bindings and are
  treated as **mutable** (`_visit_function`, `semantics.py:287-317`).
- A **module** body is its own scope (`rules.py:623-631`; §1.2 of
  [modules.md](modules.md)).

### 3.1 `let` / `const` / `let mut`

| Declaration | Reassignable |
| --- | --- |
| `let x = 1` | no |
| `let mut x = 1` | yes |
| `const K = 1` | no (never) |
| function parameter | yes (a local binding) |
| `self.x = …` / `obj.field = …` | not affected (mutates an object) |
| `for` variable, `with` binding | treated as mutable |

*Evidence:* `semantics.py` module docstring §1-17;
`_visit_var_decl`/`_visit_const_decl` §275-285;
`test_semantics_deep.py`, `test_language_rules.py`.

```aura
let x = 1
x = 2              // E303: reassign an immutable binding

let mut total = 0
total += 1         // ok

const PI = 3.14
PI = 3            // E303
```

### 3.2 Use before declaration

Reading a name that the **same function declares later in its own body** is
**E319** — the case that would otherwise be a Python `UnboundLocalError`
(`_check_use_before_declaration`, `semantics.py:237-262`). Names from an
enclosing scope, module scope or an import are unaffected
(`_locals_declared_in`, §319-335).

```aura
def f() {
  print(x)      // E319: 'x' is used before it is declared
  let x = 1
}
```

*Evidence:* `test_rule_gaps.py` (E319), `language-reference/` §3.

### 3.3 Lifetimes and memory

Local `let`/`const` bindings have **block lifetime**. An object's lifetime is
managed by the **host** (CPython's reference counting + cyclic GC). The language
**does not specify** when an object is collected — **UNSPECIFIED**
(intentional; it is the target's GC).

### 3.4 Closures

A lambda that reads an enclosing local captures it; a **block lambda** that
assigns to an enclosing local emits `nonlocal <name>` so the write is visible
outside (`_nonlocal_declaration`, `expressions.py:711`). A nested `def` that
mutates an enclosing local likewise emits `nonlocal`.

```aura
def main() {
  let mut n = 0
  let inc = () => { n = n + 1 }
  inc(); inc()
  print(n)          // 2
}
```

*Evidence:* *probe* (2);
`test_main_entrypoint.py::test_nested_function_mutates_captured_local`;
[closures.md](closures.md) §2.

> **UNSPECIFIED:** capture is by **reference** through CPython's closure
> protocol, not a per-iteration snapshot. A lambda created in a loop observes
> the loop variable's **final** value. *Probe*:
> `for i in range(3) { fs.add((x) => x + i) }` gives `fs[0](0) == 2`. Bind the
> value as a default parameter for per-iteration capture
> ([closures.md](closures.md) §2.3).

---

## 4. Value vs reference semantics

| Kind | Model | `==` |
| --- | --- | --- |
| Numbers, `bool` | value | numeric equality |
| `string` | value by content (immutable) | content |
| class instance | reference (identity) | identity by default |
| list / dict / set | reference (mutable object) | identity by default |
| struct / dict-shaped value | content | per element |

Passing to a function is **by value**; for a reference that value is the
reference, so mutating the object is visible to the caller while reassigning
the parameter is not.

```aura
def mutate(xs) { xs.add(9) }
def reassign(x) { x = 9 }
def main() {
  let a = [1]
  mutate(a)
  print(a)     // [1, 9]
  let n = 1
  reassign(n)
  print(n)     // 1
}
```

*Evidence:* *probe* (`[1, 9]`, `1`); [expressions.md](expressions.md),
[types.md](types.md).

---

## 5. Exceptions

Aura `throw`s a **value**; on CPython the value is raised
(`transform_ThrowStmt`, `statements.py:1293-1303`).

| Form | Emits |
| --- | --- |
| `throw "msg"` | `raise Exception("msg")` (a bare string is wrapped) |
| `throw ValueError("bad")` | `raise ValueError('bad')` |
| `catch { }` | `except Exception:` |
| `catch Type { }` | `except Type:` |
| `catch Type as e { }` | `except Type as e:` |
| `catch as e { }` | `except Exception as e:` |

`finally` always runs. An uncaught exception aborts the program with a message
and a non-zero exit code (`cmd_run`, `cli.py:563-573`).

*Evidence:* `test_syntax_complete.py::test_throw_string_wrapped`,
`::test_try_catch_finally`, `test_custom_errors.py`.

> **TARGET-SPECIFIC:** an exception is a CPython object. `catch` clauses are
> emitted in source order and the type filter is whatever CPython resolves the
> name to; Aura does not reorder subclasses before parents.
> **UNSPECIFIED:** `throw` of a non-string, non-exception value (`throw 5`) is
> emitted as `raise 5` and fails at runtime; it is not rejected at check time
> ([statements.md](statements.md) §8).

---

## 6. Target-specific semantics (CPython)

Because the target is CPython, these behaviours come from the host:

| Behaviour | Rule |
| --- | --- |
| Integer type | Aura `int` maps to Python's arbitrary-precision `int`. |
| Float | IEEE-754 binary64, as CPython. |
| `%` with a negative operand | Python floor-mod: *probe* `-7 % 3 == 2`. |
| `/` | True division (float) as CPython. |
| Iteration | CPython's iterator protocol; a `for` over a range/list/dict follows it. |
| Dict ordering | Insertion order (CPython 3.7+). |
| GC / object lifetime | CPython reference counting + cyclic GC. **Unspecified** when collected. |
| Runtime errors | Surface as the underlying `Python` exception (`AttributeError`, `TypeError`, …) reported by `cmd_run`. |

*Evidence:* *probe* (`-7 % 3` → `2`); `cmd_run` reports the exception type and
message (`cli.py:566-573`).

---

## 7. Language rules enforced by `aura check` / `aura run`

`aura check`, `aura run` and the REPL run the same checkers before code
generation (`_mutability_diagnostics`, `cli.py:18-40`;
`_rule_diagnostics`, `cli.py:43-62`). The rules are:

| Rule | Code | Enforced in |
| --- | --- | --- |
| Reassigning a `let`/`const` binding | **E303** | `semantics.py:403-430` |
| Assigning module state from outside (incl. an exported member) | **E303** | `rules.py:1308-1316` |
| Assigning a class-level `const` through a member | **E303** | `rules.py:1317-1325` |
| Duplicate declaration in the same scope | **E301** | `_declare`, `rules.py:581-592` |
| Duplicate parameter name | **E301** | `rules.py:1215-1228` |
| `return` outside a function (except a top-level guard) | **E004** | `rules.py:706-714` |
| `break` / `continue` outside a loop | **E004** | `rules.py:715-723` |
| `break`/`continue` naming a non-enclosing label | **E318** | `rules.py:724-734` |
| `await` outside an `async` function (or top level) | **E004** | `rules.py:746-757` |
| `yield` outside a function | **E004** | `rules.py:758-771` |
| `self` outside a class method | **E004** | `rules.py:792-799` |
| `self`/`cls` inside a `static` method | **E317** | `rules.py:800-809` |
| Unreachable code after `return`/`throw`/`break`/`continue` | **E302** | `rules.py:1249-1268` |
| Invalid assignment target | **E004** | `rules.py:1272-1284` |
| Reading a function-local before its declaration | **E319** | `semantics.py:237-262` |
| `main` missing / invalid / inside a module | **E310 / E311 / E312** | `rules.py:162-265` |
| Unresolved module re-export | **E313** | `rules.py:439-467` |

```aura
let count = 0
count = 1              // E303: reassign an immutable binding

let mut total = 0
total += 1             // ok

guard total > 0 else { return }   // exits the program if the guard fails
```

*Evidence:* `test_language_rules.py`, `test_extreme_rules.py` (E302),
`test_rule_gaps.py` (E318, E319), `test_main_entrypoint.py`,
`test_module_facade.py` (E313).

### 7.1 Placement rules (detail)

- **`return`** is valid inside any function. At the top level it is an error
  **unless** it sits directly in a `guard … else { return }` block, which is the
  idiom for "exit the program" (`rules.py:706-714`; `_in_guard_else_depth`).
- **`break` / `continue`** are valid only inside a loop. A plain one targets the
  innermost loop; a labeled one must name an enclosing loop (else E318).
- **`await`** is valid inside an `async def`, or at the **top level**, because
  `cmd_run` executes an async program inside a coroutine
  (`rules.py:746-757`; `cli.py:526-543`).
- **`self`** is valid only inside a method of a class body; `self`/`cls` inside a
  `static` method is E317 (`rules.py:792-809`).

### 7.2 Unreachable code (E302)

A statement following `return`, `throw`, `break` or `continue` (a *terminator*,
`_TERMINATORS`, `rules.py:74`) in the same statement list is **E302**. The
checker reports **once per region** and resets (`_visit_body`,
`rules.py:1249-1268`).

```aura
def f() {
  throw "boom";
  print("never")     // E302
}
```

> **Gotcha:** statements are separated by grammar, and a `return`/`break`
> followed on the next line by an expression **without a `;`** is parsed as the
> terminator's value, so no dead code is seen. *Probe*: `return\n print(1)`
> parses as `return print(1)` (no E302), while `return;\n print(1)` reports
> E302. Use a semicolon or a block body to disambiguate
> ([statements.md](statements.md) §6.7, §9).

### 7.3 Valid assignment targets

An assignment target must be a variable, a member access, an index, or a
destructuring target (tuple/list, possibly with a rest pattern)
(`_is_assignable`, `rules.py:1332-1339`). Anything else is **E004**.

```aura
def main() {
  1 = 2            // E004: invalid assignment target
}
```

*Evidence:* `rules.py:1272-1284`; *probe* (`E004`).

---

## 8. Error and diagnostic model

Diagnostics are structured records with a **code**, a **severity**, a message,
an optional source **location**, an optional **hint**, and related locations
(`AuraError`, `errors.py:108-144`). Codes are grouped by namespace
(`ErrorCode`, `errors.py:36-107`):

| Prefix | Meaning |
| --- | --- |
| `E0xx` | lexical / syntax |
| `E1xx` | type |
| `E3xx` | semantic / structural |
| `E9xx` | fatal / internal |
| `W0xx` | style warnings |
| `W1xx` | unused / redundant warnings |

A diagnostic is **not** a structured code when it comes from the parser: the
parser raises a Python `SyntaxError` carrying `line`/`column` instead
(`errors.py:55-59`; `to_ast.py` uses `self.error`).

### 8.1 Collection and severity

- `ErrorCollector` gathers diagnostics and stops after **10 errors**, appending a
  fatal `E999` summary (`errors.py:146-171`).
- A **warning** must use a `W` code; `add_warning` rejects an `E` code
  (`errors.py:173-184`), so a `W` diagnostic can never masquerade as an error.
- `has_errors()` is true only for `ERROR`/`FATAL`; warnings do not fail a build
  (`errors.py:186-188`).

### 8.2 Display and exit status

The formatted diagnostic is (`errors.py:118-141`):

```text
<file>:<line>:<col>: ERROR [E303]
  Cannot reassign immutable binding 'x'; ...
  hint: write 'let mut x' at its declaration
```

- `aura check` prints all diagnostics; a rule/mutability/type failure returns
  exit 1 (`cmd_check`, `cli.py:200-239`).
- `aura run` reports the same diagnostics and returns exit 2 before executing
  (`cmd_run`, `cli.py:503-508`).
- A runtime error is reported as `Runtime error: <Type>: <message>` and returns
  exit 1 (`cli.py:566-573`).
- An `int` returned by `main` is the exit code; a bare top-level `guard` exit is
  0 (`cli.py:554-562`).

*Evidence:* `test_diagnostics.py` (codes stay in sync with `docs/ERRORS.md`),
`test_cli_diagnostics.py`.

> `docs/ERRORS.md` is the **single source of truth** for diagnostic codes; a
> test asserts `ErrorCode` and that document stay in sync
> (`errors.py:39-41`, `tests/test_diagnostics.py`).

---

## 9. What the language does NOT define (summary)

| Point | State |
| --- | --- |
| When an object is collected | **Unspecified** (CPython GC) |
| Per-iteration capture in a loop | **Unspecified** (capture is by reference) |
| `throw` of a non-exception value | **Unspecified** (not rejected at check time) |
| Ordering of `catch` clauses | Source order; host resolves types (**Target-specific**) |
| Ambiguous import resolution | **Unspecified** (last binding wins in the target namespace) |
| `%` sign, division, integer width | **Target-specific** (CPython) |
| Runtime error text | **Target-specific** (the underlying `Python` exception) |

---

## 10. Cross-references

- [statements.md](statements.md) — statement semantics and the checker table.
- [functions.md](functions.md) §9 — the entry point.
- [closures.md](closures.md) — lambdas, capture and `nonlocal`.
- [modules.md](modules.md) — modules, imports, re-exports, name resolution.
- [python-interop.md](python-interop.md) — the CPython target surface.
- [../ERRORS.md](../ERRORS.md) — the canonical diagnostic-code reference.
