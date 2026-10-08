# FEATURE_002 — Named Function Arguments — Design

**Status:** Design finalized (pre-implementation). All blocking questions Q1–Q5
are resolved; see *Resolved Design Decisions*. No code has been changed.
**Semantic authority:** `docs/LANGUAGE_SPEC.md` (frozen)
**Baseline:** `954dcdc` (Feature 001 implemented and reviewed)
**This document is not an implementation and does not change the language.**

---

## Motivation

Aura's call sites are strictly positional. For a function with more than one
or two parameters, the caller must remember the declaration order, and a
same-typed pair of arguments is easy to transpose without any diagnostic. The
language already allows named arguments in **struct construction**
(`S { a: 1 }`) and already parses named constructor arguments
(`ctor_arg = [ IDENT ":" ] expr`), but rejects named arguments for enum
payloads and never accepted them for function calls.

Feature 002 asks whether named **function** arguments earn their place as the
second post-freeze feature, and if so, defines the complete semantic contract
before any implementation.

## Current Function-Call Model

The pipeline is: parse → check → execute, one authority per layer.

* **Parser.** `Expr::Call(callee, Vec<Expr>, span)` and
  `Expr::Method(recv, name, Vec<Expr>, span)` carry **positional `Expr`s
  only** (`src/parse/mod.rs`, `postfix` → `arg_expr`). `Expr::Construct`
  carries `Vec<Arg>`, where `Arg { name: Option<String>, value: Expr }`.
  `cons_arg` already parses `name: expr`.
* **Grammar.** `call_args = expr { "," expr }` with the comment "function
  calls are positional"; `ctor_arg = [ IDENT ":" ] expr` (constructors only).
* **Checker.** `FnSig { ret: Option<Ty>, params: Vec<Option<Ty>> }`
  (Feature 001) holds a function's return type and per-parameter annotation –
  **no parameter names**. `check_user_call` compares `args.len()` with
  `sig.params.len()` and checks each annotated position with
  `Ty::compatible_with`. Builtins use `Signature { name, params: Vec<Param>,
  min_args, max_args, returns }` where `Param { accepts }` – **no names**.
* **Runtime.** `eval_call` evaluates the argument expressions left-to-right
  into `Vec<Value>` and binds them to parameters by position. Arity is checked
  at runtime (`Interp::call`).
* **REPL.** `GlobalDecl::Function { name, ret, params: Vec<Option<TypeExpr>> }`
  carries types but not names across submissions.
* **Pipeline.** `x |> f(a)` desugars at parse time to `f(x, a)`.

**Conclusion:** named arguments for functions are **not** partially supported.
The AST `Arg` shape exists and is reused by constructors; function calls use a
different representation. Supporting named function arguments requires a
grammar, AST, checker, runtime, registry, and REPL change – a real feature, not
a flag.

## User Problem

```aura
fn create_user(name: string, age: int, email: string) -> User { ... }

create_user("João", 30, "j@x")   # which argument is which?
create_user("João", "j@x", 30)   # transposition is caught (types differ) …
```

When adjacent parameters share a type, transposition is invisible:

```aura
fn rect(width: int, height: int) -> int { return width * height }
rect(30, 20)   # valid, and silently wrong if you meant height=30
```

Named arguments make intent explicit and make transposition impossible at the
call site, without changing the declaration.

---

## Proposed Feature

Allow a call to a **directly resolved user-defined function** to supply some
or all of its arguments **by parameter name**, using the existing
`name: value` form already present in the grammar for constructors.
Positional arguments continue to work unchanged.

```aura
create_user(name: "João", age: 30, email: "j@x")
create_user("João", age: 30, email: "j@x")     # mixed
```

Named arguments are matched to **parameter names**, not to types or positions.

---

## Syntax Investigation

The parser already implements `cons_arg`:

```rust
fn cons_arg(&mut self) -> Result<Arg> {
    if let Tok::Ident(name) = self.at().clone() {
        let save = self.pos;
        self.bump();
        if self.eat(&Tok::Colon) {
            let value = self.expr()?;
            return Ok(Arg { name: Some(name), value });
        }
        self.pos = save;                 // backtrack: not a named argument
    }
    let value = self.expr()?;
    Ok(Arg { name: None, value })
}
```

This is a **lookahead-and-backtrack** rule: `IDENT ":"` is named, anything
else is positional. It is used by `ctor_args`, which the grammar gives to both
struct and variant construction (the latter rejects names at check time).

Candidate syntaxes:

| Candidate | Assessment |
|---|---|
| `f(x: 1)` | Matches the existing `ctor_arg` form exactly. `:` inside call parentheses is currently a parse error, so no ambiguity. Reuses `cons_arg`. **Recommended.** |
| `f(x = 1)` | Uses the assignment token inside an expression; conflicts conceptually with statement assignment and with `let`; no precedent. |
| `f(.x = 1)` / other sigils | New punctuation with no precedent. |

**Ambiguity analysis for `f(x: 1)`:**

* `try_lambda_params` is probed first when the parser sees `(`. For `x: 1` it
  reads `x`, then sees `:` (not `,` or `)`), backtracks, and returns `None`.
  The expression parser then reads `x`; the `:` is currently unconsumed and
  yields `E1006`. Making `arg_expr` (for calls) use the `cons_arg` rule
  resolves this with no lambda ambiguity.
* `:` is used elsewhere only for type annotations (`let x: T`, `fn f(a: T)`),
  struct fields (`struct S { a: T }`), map entries (`{k: v}`), `catch IDENT ->`,
  and named constructor args. Inside call parentheses none of these contexts
  apply, so `IDENT ":"` unambiguously begins a named argument.
* Struct literals (`S { a: 1 }`) are distinguished by `{` vs `(` and by the
  uppercase-first-character rule, so `f(x: 1)` cannot be confused with a
  struct literal.

**No grammar ambiguity was found.**

## Syntax Proposal

Adopt `name: value` for call arguments, reusing the existing `Arg` shape:

```
call_args = arg { "," arg } ;
arg       = [ IDENT ":" ] expr ;
```

This is the same production as `ctor_arg`. It replaces
`call_args = expr { "," expr }` for **function and method calls**.

---

## Parameter Name Semantics

**Parameter names are the declared names of a function's parameters**, matched
exactly and case-sensitively, as plain identifiers.

```aura
fn greet(name: string, punctuation: string) { ... }

greet(name: "João", punctuation: "!")   # both named
greet("João", punctuation: "!")         # mixed
greet(punctuation: "!", name: "João")   # reordered by name
```

* Matching is by `String` equality against the declared parameter name.
* Names are not types: `name` identifies a parameter, it is not a value.
* An unknown name is rejected (below).
* A parameter may be supplied at most once across the whole call.

---

## Positional Arguments

Unchanged. A positional argument fills the next **unfilled** parameter in
declaration order. This preserves all existing behavior: a call with only
positional arguments behaves exactly as today.

## Named Arguments

A named argument fills the parameter with that name, regardless of where it
appears in the argument list. Named arguments do not consume a position.

## Mixed Arguments

Two models were considered:

* **(A) Positional-then-named only** — all positional arguments must precede
  all named arguments (`{ positionals } { named }`).
* **(B) Free interleaving** — `f(a: 1, 2, b: 3)` allowed.

**Decision: (A) — positional arguments MUST precede named arguments** (Q4).
Interleaving is rejected as a parse rule:

```
f(1, y: 2)      # valid
f(x: 1, 2)      # invalid: positional argument after a named one (E1006)
```

## Duplicate Arguments

A parameter supplied twice — positionally and by name, or twice by name — is a
**static `E3001`** for a directly resolved call:

```aura
fn f(a, b) { ... }
f(1, a: 2)      # E3001: parameter `a` is given more than once
f(a: 1, a: 2)   # E3001: parameter `a` is given more than once
```

Rationale: Aura's design principle is "one spelling per construct" and its
diagnostic philosophy is "diagnose everything". Silent first-wins or last-wins
would contradict both and would hide a likely bug. Static rejection for
directly resolved calls matches Feature 001's model (arity is already static).

## Missing Arguments

A declared parameter with no positional or named argument is a **missing
argument**. For a directly resolved call it is a static `E3001`, reported as a
missing parameter by name:

```aura
fn f(a, b, c) { ... }
f(a: 1, c: 3)   # E3001: missing argument for parameter `b`
```

This is the model that replaces naive arity.

## Unknown Parameter Names

For a directly resolved call, the checker knows the parameter names. An
argument named with an unknown parameter is a **static `E3001`**:

```aura
fn f(a: int) { ... }
f(b: 1)          # E3001: `f` has no parameter named `b`
```

This parallels the struct-construction rule (unknown field is `E2003`); the
choice between `E3001` and `E2003` is discussed in *Error Model*.

## Arity Semantics

Arity becomes **parameter satisfaction**, not raw argument count. Define:

* `provided(k)` — parameter `k` is filled by a positional or named argument.
* `missing(k)` — a declared parameter with no argument.
* `duplicate(k)` — supplied more than once.
* `unknown` — a name matching no declared parameter.

For a directly resolved call: every parameter must be provided exactly once,
and every provided name must correspond to a parameter. The number of
arguments equals the number of parameters **only when all arguments are
positional**. A named call may have the same count as the parameter list but a
different *mapping*, and a reordered named call has the same count with a
permuted mapping.

Feature 001's raw count check is therefore **generalized** to a
parameter-satisfaction check for calls that contain named arguments; calls
that are entirely positional keep the existing count equality exactly.

---

## Type Checking

After mapping arguments to parameters, Feature 001's existing check runs
unchanged: for each parameter with an annotation, the corresponding
argument's inferred type must be `compatible_with` the annotation; `Unknown`
is permissive.

```aura
fn f(a: int, b: string) { ... }
f(b: "x", a: 1)     # valid: mapped a→1 (int), b→"x" (string)
f(a: "x", b: "y")   # E3001: parameter `a` expects `int`, found `string`
```

No new type logic is introduced; only the argument→parameter mapping is new.

## Unknown Boundary

Type `Unknown` is unchanged: an argument whose inferred type is `Unknown` is
never rejected on type grounds. **Parameter-name resolution is independent of
type resolution**: an unknown *name* is always a static error for a directly
resolved call, even if no type could be inferred. This is the one place where
Feature 002 differs from Feature 001's "only reject what is provable": the
parameter list is *known*, so a bad name is provable without type information.

## Shadowing

Feature 001's resolution rule is preserved. Named-argument checking begins
**only after** the callee resolves to a specific top-level function and is not
shadowed. A local binding that shadows the name means the call is dynamic, and
neither parameter names nor types of the global declaration are applied:

```aura
fn f(x: int) { ... }
fn g() { let f = dynamic_callable
         f(x: 1) }        # dynamic: global `f`'s parameter names not applied
```

## Hoisting

Unchanged. Because declarations are hoisted (and Feature 001 records their
signatures before bodies are checked), named calls to forward-declared and
mutually recursive functions are checked normally.

## Mutual Recursion

Unchanged; named calls inside mutually recursive functions are checked against
the resolved callee's parameter list.

## Built-ins

Built-ins **do not** have parameter names: `Param { accepts }` carries no
name. Extending the registry with names is a larger change that would force
naming decisions for every builtin (`len(value)`? `len(collection)`?) and
would be irreversible metadata.

**Decision: built-ins are OUT OF SCOPE for Feature 002** (Q3). A named
argument passed to a builtin is rejected statically with `E3001`. Positional
builtin calls are unchanged. This keeps one calling convention per callable
kind, avoids inventing names for a frozen registry, and leaves the door open
to add names later without breaking anything.

## Methods

Methods have no parameter names in `MethodSig` either, and user-defined
methods do not exist. **Methods are OUT OF SCOPE for Feature 002** (Q3). A
named argument in a method call is rejected statically with `E3001`.
Positional method calls are unchanged. See *Built-in / Method Boundary*.

## Pipeline

The pipeline desugars `x |> f(a)` to `f(x, a)` at parse time. The receiver
`x` becomes the **first positional argument**, and the remaining arguments may
be named:

```aura
fn move(dx: int, dy: int) { ... }
point |> move(dy: 2)         # → move(point, dy: 2): dx = point, dy = 2
```

Semantics:

* The receiver fills the first parameter positionally (unchanged).
* Named arguments in the parenthesized suffix fill their parameters by name.
* Therefore in `x |> f(dy: 2)`, the first parameter is filled by `x` and `dy`
  is filled by name.
* Invalid combinations follow the same rules: a name equal to the first
  parameter is a **duplicate** (`f(dx: 2)` in the example), an unknown name is
  an error, and a named argument may not be followed by a positional one.

No pipeline-specific code is needed: the desugaring produces an ordinary
`Expr::Call` with a positional first argument followed by named arguments,
which already satisfies the positional-then-named rule.

## REPL

`GlobalDecl::Function` must carry parameter **names** in addition to types, so
a function declared in one submission exposes its parameter names (and types)
to later submissions. This extends the Feature 001 plumbing
(`params: Vec<Option<TypeExpr>>` becomes, conceptually,
`params: Vec<(String, Option<TypeExpr>)>`). Session semantics are otherwise
unchanged: a failed named call does not corrupt the declaration.

## Runtime Behavior

Two designs were considered:

* **(A) Static-only.** Named arguments are resolved entirely by the checker;
  the runtime receives the already-ordered positional `Vec<Value>`. Invalid
  named calls never reach the runtime for directly resolved functions.
* **(B) Runtime-resolved.** The runtime also understands names.

**Decision: (A).** For a directly resolved user function, the checker maps
named arguments to positions and rejects errors; the runtime call machinery
receives positional values and is unchanged. For all other callables —
built-ins, methods, closures, values, `Unknown` — named arguments are a static
`E3001` (Q1, Q3), so no runtime parameter-name resolution is ever required.
This is a direct consequence of the resolved decisions; it is no longer an
open question.

## Error Model (draft)

> Superseded by the resolved **Error Model** section below, which freezes
> `E3001` for every call-mismatch case and `E1006` for positional-after-named.
> The draft alternative between `E3001` and `E2003` was resolved in favour of
> `E3001`.

All new cases reuse `E3001` (`TYPE_MISMATCH`) because they are all "the
arguments do not match the declared parameters":

| Case | Code | Phase (directly resolved) |
|---|---|---|
| unknown parameter name | `E3001` | check |
| duplicate parameter assignment | `E3001` | check |
| missing parameter | `E3001` | check |
| named argument to a builtin/method with no names | `E3001` | check |
| positional after named | `E1006` (parser) | parse |
| named argument on a dynamic/unknown callee | `E3001` | check, if provable |

No new error code is proposed.

## Compatibility

**Classification: source-compatible extension with edge cases.**

* `f(x: 1)` is currently a **parse error** (`E1006`), so no previously valid
  program changes meaning. The new syntax only turns a previously-invalid
  program into a valid one.
* All previously valid positional calls remain valid and unchanged.
* The one edge case: a program that *relied* on `f(x: 1)` being a parse error
  is not a real program.
* No previously accepted call becomes statically rejected, because named
  arguments did not exist. A positional call with too few arguments was
  already a runtime `E3001`; if it now fails at check time, that is the
  Feature 001 tightening, not a Feature 002 change.

Unlike Feature 001, Feature 002 does **not** tighten any existing accepted
program: it is purely additive syntax.

## Backward Compatibility Examples

| Program | Today | Under Feature 002 |
|---|---|---|
| `f(1, 2)` | valid, positional | valid, unchanged |
| `f()` | runtime `E3001` | unchanged (no names involved) |
| `f(1)` (2 params) | runtime `E3001` | unchanged |
| `f(x: 1)` | parse error `E1006` | **new:** valid named call |
| `f(x: 1, y: 2)` | parse error | new: valid |
| `f(y: 2, x: 1)` | parse error | new: valid (reordered) |
| `f(x: 1, 2)` | parse error | invalid: positional after named |
| `f(x: 1, x: 2)` | parse error | invalid: duplicate `E3001` |
| `f(z: 1)` | parse error | invalid: unknown name `E3001` |
| `f(a: 1)` (missing `b`) | parse error | invalid: missing `b` `E3001` |
| `S { a: 1 }` | valid struct literal | valid, unchanged |
| `A(1)` variant | valid | valid, unchanged |
| `A(x: 1)` variant | check `E3001` | unchanged (variants remain positional) |

## Grammar Impact

**LOW.** Replace, for calls, `call_args = expr { "," expr }` with the
`ctor_arg`-shaped production `call_args = arg { "," arg }`. Everything else is
unchanged. `docs/grammar.md` and `LANGUAGE_SPEC.md` §4.5 must be updated to
describe the `Arg` form for calls and the positional-then-named rule.

## AST Impact

**LOW–MEDIUM.** `Expr::Call` and `Expr::Method` currently carry `Vec<Expr>`.
They must carry named-capable arguments. Two options:

* **(A)** Change `Expr::Call(Box<Expr>, Vec<Expr>, Span)` to
  `Vec<Arg>` (reusing the existing `Arg`), and likewise `Expr::Method`.
  This requires updating every construction and consumer of `Expr::Call`
  (parser, checker, runtime, `span_of`, depth checks, `desugar_pipe`).
* **(B)** Add a parallel `Vec<(Option<String>, Expr)>` alongside the existing
  positional vector, keeping both in the AST. This duplicates representation
  and is discouraged.

**Decision: (A).** Reuse `Arg` uniformly; there is already precedent
(`Expr::Construct`). `desugar_pipe` must insert the piped receiver as a
positional `Arg { name: None, .. }` at the front, which is exactly correct.

## Checker Impact

**MEDIUM.**

* `FnSig` must carry parameter names: conceptually
  `params: Vec<(String, Option<Ty>)>` (or a small `ParamSig { name, ty }`).
* A new mapping step: given `Vec<Arg>` and `FnSig`, produce a positional
  `Vec<usize>` (argument index per parameter), and detect unknown, duplicate,
  and missing parameters.
* The existing `check_user_call` splits into "map arguments to parameters"
  and "check mapped argument types" (the latter unchanged).
* Builtins/methods: named arguments are rejected (`E3001`).
* Dynamic callees: no parameter list; named arguments cannot be resolved.

## Runtime Impact

**LOW.** Because the checker resolves named arguments to positions (design A
under *Runtime Behavior*) and rejects named arguments on every non-direct
callable (Q1, Q3), the runtime never resolves parameter names; its
`Vec<Value>` binding is unchanged. The parser change means `Expr::Call` now
holds `Vec<Arg>`, so `eval_call` reads `arg.value` for each argument; behavior
is identical for positional calls.

## Stdlib Impact

**NONE** if built-ins are out of scope (recommended). Positional calls through
the registry are unchanged. (Adding parameter names to the registry is future
work, not Feature 002.)

## REPL Impact

**LOW.** Extend the `GlobalDecl::Function` payload and `declarations_of` to
carry parameter names alongside types. No change to session semantics.

## Specification Impact

The design proposes the following amendments (to be made at implementation
time, not now). **Current:** §15.7 says "Function calls are positional; there
are no named arguments at a call site…". **Proposed:**

* §4.5 grammar: `call_args = arg { "," arg }`, `arg = [ IDENT ":" ] expr`;
  state the positional-then-named rule.
* §15.7: allow named arguments for directly resolved user functions; define
  matching by declared parameter name, duplicates, missing parameters, unknown
  names; state that builtins and methods and dynamic callees do not accept
  named arguments.
* §6.5: generalize the arity check to parameter satisfaction when named
  arguments are present.
* §23: state that the pipeline receiver is the first positional argument and
  that a name equal to the first parameter is a duplicate.
* §29: state that the REPL carries parameter names across submissions.
* §33: add a frozen decision for named-argument matching semantics.

No new error code; `E3001` is reused.

---

## Test Plan

These tests are **specified, not implemented**. Each is source → expected.

### Basic named calls
| Source | Expected |
|---|---|
| `fn f(x) { return x }` + `f(x: 1)` | runs, value `1` |
| `fn f(x, y) { return x + y }` + `f(y: 2, x: 1)` | runs, value `3` |
| `fn f(a, b, c) { return [a, b, c] }` + `f(c: 3, a: 1, b: 2)` | `[1, 2, 3]` |

### Unknown parameter
| `f(z: 1)` against `fn f(x)` | `E3001` at check |

### Duplicate
| `f(1, a: 2)` against `fn f(a, b)` | `E3001` at check |
| `f(a: 1, a: 2)` | `E3001` at check |

### Missing
| `f(a: 1)` against `fn f(a, b)` | `E3001` (missing `b`) at check |

### Mixed positional/named
| `f(1, y: 2)` against `fn f(x, y)` | runs |
| `f(y: 2, x: 1)` | runs |
| `f(x: 1, y: 2)` | runs |

### Invalid order
| `f(x: 1, 2)` | parse error `E1006` |

### Type checking
| `f(x: "wrong")` against `fn f(x: int)` | `E3001` at check |
| `f(b: 1, a: "x")` against `fn f(a: int, b: int)` | `E3001` at check |

### Unknown type
| `f(x: none)` against `fn f(x: int)` | accepted (Unknown permissive) |

### Shadowing
| global `fn f(x: int)`, local `f` callable, `f(x: 1)` | dynamic; global names not applied |

### Hoisting
| named call to a function declared later | checked |

### Mutual recursion
| named call inside mutually recursive functions | checked |

### REPL
| `fn f(x, y)` then `f(y: 2, x: 1)` in a later submission | runs |
| failed named call between valid ones | session preserved |

### Pipeline
| `fn move(dx: int, dy: int)`, `p \|> move(dy: 2)` | runs as `move(p, dy: 2)` |
| `p \|> move(dx: 2)` | duplicate `dx` → `E3001` |

### Built-ins / methods
| `len(value: [1])` | `E3001` (builtins reject named arguments) |
| `xs.map(fn: (x) -> x)` | `E3001` (methods reject named arguments) |

### Positional regression
| every existing positional call test | unchanged |

## Property / Differential Testing (draft)

> Superseded by the final **Property / Differential Testing** section below,
> which additionally freezes the evaluation-order property.

Design properties to implement later:

* **Permutation equivalence.** For `fn f(a, b, c)`, every permutation of named
  arguments that supplies all three produces the same result as the positional
  call, once the mapped order is applied.
* **Rejection determinism.** Every invalid mapping (unknown name, duplicate,
  missing) is rejected deterministically at the check phase for a directly
  resolved call.
* **Additivity.** For a call with no named arguments, the static and runtime
  verdicts are identical to the pre-feature behavior.
* **Checker/runtime agreement.** If the checker accepts a named call, the
  runtime does not fail with a type error at that call.
* **Pipeline equivalence.** `x |> f(b: v)` equals `f(x, b: v)` equals the
  positional call with the same mapping.

## Reference Implementation Examples

```aura
# Basic named arguments
fn rect(width: int, height: int) -> int { return width * height }

rect(width: 30, height: 20)     # 600
rect(height: 20, width: 30)     # 600, reordered
```

```aura
# Mixed positional + named
fn create_user(name: string, age: int, email: string) -> string {
    return f"{name}/{age}/{email}"
}

create_user("João", age: 30, email: "j@x")
```

```aura
# Type checking after mapping
fn f(a: int, b: string) -> string { return b }
f(b: "x", a: 1)     # valid
f(a: "x", b: "y")   # E3001: `a` expects int, found string
```

```aura
# Errors
fn f(a: int, b: int) -> int { return a + b }
f(a: 1, c: 2)       # E3001: no parameter named `c`
f(a: 1, a: 2)       # E3001: parameter `a` given more than once
f(a: 1)             # E3001: missing argument for parameter `b`
f(a: 1, 2)          # E1006: positional after named
```

```aura
# Pipeline
fn move(dx: int, dy: int) -> int { return dx * 10 + dy }
point |> move(dy: 2)      # move(point, dy: 2)
```

```aura
# REPL, two submissions
# > fn f(x, y) { return x + y }
# > f(y: 2, x: 1)
# 3
```

---

## In Scope

* Named arguments (`name: value`) for **directly resolved user-defined
  top-level functions**.
* Positional arguments unchanged.
* Positional-then-named ordering; a positional argument after a named one is a
  parse error.
* Parameter matching by declared name, case-sensitive.
* Static rejection (directly resolved): unknown name, duplicate assignment,
  missing parameter.
* Static type checking after mapping, reusing `Ty::compatible_with` and the
  `Unknown` boundary.
* Pipeline receiver as the first positional argument.
* REPL parameter-name persistence.
* Reuse of the existing AST `Arg`, the existing parser helper, and the
  Feature 001 checker infrastructure.

## Out of Scope

* Default parameter values.
* Variadic parameters.
* Named arguments for **built-ins** (no names in the registry).
* Named arguments for **methods**.
* Named arguments for **dynamic/unknown callables** (no parameter list).
* User-defined methods.
* Function types, higher-order inference, overloads, generics, modules.
* Any change to enum payload semantics (variants remain positional).
* Any change to struct-construction semantics.

## Future Extensions

* **Default parameters.** The parameter-satisfaction model makes this a clean
  follow-on: a parameter with a default may be `missing` without error, and
  the default is applied at the call. No redesign required.
* **Built-in/method parameter names.** The registry `Param` could gain a
  `name: Option<&'static str>` in a later feature; named arguments would then
  extend to builtins without changing the call syntax.
* **Variadic arguments.** A variadic parameter would absorb extra positional
  arguments; the model must define whether a variadic parameter can be named
  (likely not). Documented as future work.

## Resolved Design Decisions

All four blocking design questions are resolved. They are frozen; the
implementation must follow them exactly.

| ID | Question | Decision |
|---|---|---|
| Q1 | Named arguments on dynamic / unknown callees | **Rejected** with `E3001`; named arguments require a statically known parameter set |
| Q2 | Diagnostic for an unknown parameter name | **`E3001`** (the call-mismatch family), reusing the existing code |
| Q3 | Built-ins and methods | **Out of scope** for Feature 002; remain positional |
| Q4 | Argument ordering | **Positional arguments MUST precede named arguments**; the reverse is a parser error |
| Q5 | Free interleaving vs positional-then-named | **Positional-then-named** (this is the same decision as Q4; the prior draft listed it separately) |

### Q1 — Dynamic / unknown callees

**Decision.** Named arguments require a statically known parameter set. If the
callee cannot be statically resolved to a supported named-argument signature,
named-argument syntax is rejected with `E3001`.

This applies to function values, closures, callable variables, `Unknown`
callees, and every other unresolved or non-direct callable:

```aura
let f = (x) -> x
f(x: 1)              # E3001: named arguments require a resolved function
```

Rationale: the checker cannot validate parameter-name existence or
satisfaction without the callable's parameter list. Feature 002 must not
invent a dynamic parameter-name system or expand into function-type
inference.

### Q2 — Unknown parameter name diagnostic

**Decision.** An unknown parameter name is `E3001`.

Rationale: it is a function-call argument mismatch, not a lexical symbol
resolution failure. It stays in the "bad call" diagnostic family used by
Feature 001 for arity and type mismatches.

```aura
fn f(x: int) { return x }
f(y: 1)              # E3001: `f` has no parameter named `y`
```

No new error code is introduced.

### Q3 — Built-ins and methods

**Decision.** **Out of scope.** Named arguments are supported only for
directly, statically resolved top-level user-defined functions.

Built-ins and methods remain positional. Their named arguments are rejected
under the current Feature 002 scope. The standard-library signature registry
(`src/stdlib/signatures.rs`) is not changed, and no canonical parameter names
are invented. This is a deliberate feature boundary and a future extension
point (see *Future Extensions* and *Standard Library Future Extension*).

### Q4 — Positional-then-named ordering

**Decision.** Positional arguments MUST precede named arguments.

Valid:

```aura
f(1, y: 2, z: 3)
```

Invalid (parser error, per existing grammar conventions):

```aura
f(x: 1, 2)           # E1006: positional argument after a named argument
```

Arbitrary positional/named interleaving is not supported.

### Q5 — Interleaving

Q5 (free interleaving vs positional-then-named) is the same decision as Q4
and is resolved identically: **positional-then-named**. The prior draft listed
it separately; it is not a distinct open question.

---

## Parameter Satisfaction Semantics

A call is a **parameter-satisfaction** process, not a raw argument count.

For:

```aura
fn f(a: int, b: string, c: bool) { ... }
```

a call proceeds conceptually as:

```
1. Parse arguments in source order.
2. Evaluate argument expressions in source order (see Evaluation Order).
3. Map positional arguments to the next unfilled parameter.
4. Map named arguments to the exact declared parameter by name.
5. Reject duplicate parameter assignment.
6. Reject unknown parameter names.
7. Reject missing required parameters.
8. Apply the existing Feature 001 type checks to the final parameter mapping.
9. Execute using the resulting parameter bindings.
```

Define, for a resolved function signature `P = [p_0 … p_{n-1}]`:

* `provided(p)` — `p` is filled by a positional or named argument.
* `missing(p)` — `p` is never provided.
* `duplicate(p)` — `p` is provided more than once.
* `unknown(name)` — a named argument whose name matches no parameter.

**Invariant.** Every parameter in `P` is provided exactly once, and every
named argument's name is in `P`. The argument count equals the parameter count
only when every argument is positional; a named call may have equal counts
with a permuted mapping.

This generalizes Feature 001's raw count equality. A call whose arguments are
**entirely positional** keeps the existing count equality exactly.

---

## Evaluation Order vs Parameter Binding

> Named-argument binding changes **parameter association**, not evaluation
> order.

Argument expressions are always evaluated **left-to-right in source order**,
before or while being associated with parameters. For:

```aura
fn f(a, b) { ... }
f(b: side_effect_1(), a: side_effect_2())
```

the observable evaluation order MUST be:

```text
side_effect_1()
side_effect_2()
```

even though the resulting bindings are:

```text
a ← result of side_effect_2()
b ← result of side_effect_1()
```

This distinction is mandatory:

```text
evaluation order  ≠  parameter binding order
```

The implementation MUST NOT reorder evaluation to match declaration order. It
maps arguments to parameters after evaluating them in source order; the
runtime binds the evaluated values to positions produced by that mapping.

---

## Dynamic Callable Boundary

Static named-argument validation applies only when the callee resolves to a
specific supported user-defined top-level function (the Feature 001
resolution rule). If the callee is shadowed, dynamic, `Unknown`, a function
value, a closure, a parameter, or any non-`Name` expression, the user
function's parameter list is not applied.

For named arguments specifically, an unresolvable call is a hard `E3001`,
because parameter names cannot be validated dynamically under this feature's
static contract. Feature 002 does not expand into higher-order static
inference.

```aura
fn f(x: int) { return x }
fn g() { let f = (a) -> a
         return f(x: 1) }     # E3001: `f` here is a local callable, not the
                              # resolved function; named arguments are rejected
```

---

## Built-in / Method Boundary

Frozen boundary:

```text
Feature 002 supports named arguments for:
    directly, statically resolved top-level user-defined functions only.

Built-ins and methods:
    positional only. Named syntax is rejected (E3001).
```

Rationale: the registry's `Signature`/`MethodSig` carry parameter **types**
(`Param { accepts }`) but no names, and inventing canonical names (`value`?
`collection`?) is irreversible public API metadata. This is deferred, not
forgotten (see *Standard Library Future Extension*).

---

## Compatibility Classification

**Source-compatible additive extension.** Every previously valid program keeps
its exact meaning and result.

* `f(x: 1)` is currently a **parse error** (`E1006`); Feature 002 only turns
  a previously invalid program into a valid one. No existing valid syntax is
  reinterpreted and no parser precedence changes.
* Positional calls (`f(1, 2)`) are byte-for-byte unchanged.
* No previously accepted program becomes statically rejected by Feature 002
  itself. (A too-few-argument positional call was already a runtime `E3001`,
  and Feature 001 already made some of those static; that is Feature 001, not
  Feature 002.)

The one deliberate tightening is for **previously invalid** syntax: named
arguments that cannot be resolved (dynamic callee, unknown name, duplicate,
missing) fail at check time. That is a new syntax whose errors are static;
it removes no valid program.

---

## Pipeline Semantics

Existing semantics are preserved and reused; no pipeline-specific binding
algorithm is introduced.

```text
x |> f(a)      is      f(x, a)
```

The piped value is the **first positional argument**. A parenthesized suffix
may include named arguments, which fill their parameters by name:

```aura
fn move(dx: int, dy: int) -> int { return dx * 10 + dy }
point |> move(dy: 2)         # → move(point, dy: 2)
```

Consequences, derived from ordinary call rules:

* `point |> move(dy: 2)` binds `dx = point`, `dy = 2`.
* `point |> move(dx: 2)` is a **duplicate** assignment of `dx` → `E3001`.
* A name equal to the first parameter is always a duplicate.
* Positional-then-named still holds: the receiver is positional and precedes
  any named suffix.

No separate pipeline argument-binding algorithm exists.

---

## REPL Signature Persistence

The session must preserve parameter **names** as well as types.

Feature 001 carries `GlobalDecl::Function { name, ret, params: Vec<Option<TypeExpr>> }`.
Feature 002 requires the parameter name alongside the annotation, conceptually:

```text
Function signature:
    name
    parameter name
    parameter annotation/type
    return type
```

i.e. `params: Vec<(String, Option<TypeExpr>)>`, or the repository's equivalent
unified representation. No parallel metadata table is introduced.

Required conceptual session:

```text
submission 1:
fn greet(name: string, punctuation: string) { ... }

submission 2:
greet(punctuation: "!", name: "João")
```

must resolve against the persistent signature. An invalid named call must not
corrupt the REPL state.

---

## Error Model

All Feature 002 call-mismatch errors use the existing `E3001`
(`TYPE_MISMATCH`). No new code is introduced.

| Situation | Code | Phase (directly resolved) |
|---|---|---|
| unknown parameter name | `E3001` | check |
| duplicate parameter assignment | `E3001` | check |
| missing required parameter | `E3001` | check |
| static argument type mismatch | `E3001` | check (Feature 001) |
| named argument to a built-in | `E3001` | check |
| named argument to a method | `E3001` | check |
| named argument on a dynamic/unresolved callee | `E3001` | check |
| positional argument after a named one | `E1006` | parse |

Diagnostics preserve the existing conventions: stable code, call-site span,
deterministic first error, and the same behavior across CLI, library, and REPL.

---

## Open Questions

No blocking open questions remain. Q1–Q5 are resolved above. The only
remaining items are genuinely future-design questions, recorded under
*Future Extensions* and *Standard Library Future Extension*:

* Whether and how to give built-ins and methods canonical parameter names
  (future feature, requires registry changes).
* Whether a future variadic parameter may be named (future design).
* Whether a future optimizer may treat argument order as non-semantic
  (informational; Aura has no memoization today).

None of these blocks Feature 002.

---

## Implementation Contract

Design contract for the eventual implementation:

```
Given a call expression whose argument list may carry names:

Resolve the callee using the existing lexical rules.

If the callee resolves to a specific, unshadowed top-level user function:

    build the parameter list from the resolved signature (names + types);
    reject any named argument whose name matches no parameter;
    reject any parameter supplied more than once;
    reject any parameter left unfilled;
    map each argument to its parameter (positional fills next unfilled,
    named fills by name);
    apply the existing Feature 001 type check to the mapped arguments;
    preserve Unknown conservatism (an Unknown argument is never rejected);
    emit any of the above mismatches as E3001 during checking.

Otherwise (builtin, method, dynamic callable, unknown callee):

    named arguments are not resolvable and are rejected at check time
    with E3001; positional behavior is unchanged.

Runtime validation remains active for every call.
```

The implementation must not: add new error codes; duplicate type
compatibility; duplicate the parameter list in a second table; apply a global
function's parameter names to a shadowed local callable; or change enum
payload or struct-construction semantics.

---

## Architectural Impact Map

| Layer | Impact | Note |
|---|---|---|
| Grammar | LOW | `call_args` adopts the `arg` form |
| AST | MEDIUM | `Expr::Call`/`Method` carry `Arg` instead of `Expr`; `desugar_pipe` inserts a positional `Arg` |
| Parser | LOW | reuse `cons_arg` for call arguments |
| Checker | MEDIUM | parameter names in `FnSig`; mapping + duplicate/missing/unknown |
| Runtime | LOW | checker maps to positions; runtime binding unchanged |
| Stdlib registry | NONE | builtins out of scope |
| REPL | LOW | `GlobalDecl::Function` carries names |
| Pipeline | LOW | desugaring already produces a positional first argument |
| Docs | MEDIUM | §4.5, §6.5, §15.7, §23, §29, §33, grammar, contract |
| Tests | MEDIUM | the matrix above |

---

## Abstract Syntax Impact Detail

```text
Expr::Call(Box<Expr>, Vec<Expr>, Span)   →   Expr::Call(Box<Expr>, Vec<Arg>, Span)
Expr::Method(Box<Expr>, String, Vec<Expr>, Span)
                                         →   Expr::Method(Box<Expr>, String, Vec<Arg>, Span)
```

Every consumer of `Expr::Call`/`Method` arguments must be updated:
`parse` (construct), `check` (arg visiting + call checks), `run` (`eval_call`,
method dispatch), `span_of`, the depth checks (`check_expr_depth`), and
`desugar_pipe`. This is the largest mechanical part of the feature and is why
the AST impact is rated MEDIUM rather than LOW.

---

## Proposed LANGUAGE_SPEC Amendment

This is the exact normative text that will be inserted into
`docs/LANGUAGE_SPEC.md` at implementation time. It is **not** applied in this
phase.

### Syntax (§4.5)

**Proposed.** `call_args = arg { "," arg } ; arg = [ IDENT ":" ] expr ;`
where an argument with a name is a **named argument** and one without is a
**positional argument**. All positional arguments in a call MUST precede all
named arguments; a positional argument after a named one is a syntax error.

### Calls (§15.7)

**Proposed.**

> A call to a directly resolved top-level function MAY supply arguments by
> parameter name, using `name: value`. Positional arguments fill the next
> unfilled parameter in declaration order; a named argument fills the
> parameter with that name. A call MUST supply every declared parameter
> exactly once: a parameter supplied twice is `E3001`, a named argument
> naming no parameter is `E3001`, and a declared parameter left unfilled is
> `E3001`. Named arguments are accepted only when the callee is statically
> resolved to a top-level `fn`; a named argument to a built-in, a method, or
> any dynamically resolved callable is `E3001`. There are still no default or
> variadic parameters in this version.

### Static checking (§6.5)

**Proposed.** Generalize the argument-count rule: for a directly resolved
call, the checker MUST validate **parameter satisfaction** — every parameter
provided exactly once, no unknown names, no duplicates — and then apply the
existing annotated-type checks to the resulting argument→parameter mapping.
A call whose arguments are all positional retains the existing count-equality
behavior. `Unknown` argument types remain permissive (§6.4).

### Evaluation order (§13)

**Proposed.** Named-argument binding changes parameter association, not
evaluation order. Argument expressions are evaluated left-to-right in source
order; mapping to parameters happens independently of that order.

### Pipeline (§23)

**Proposed.** `x |> f(a)` is `f(x, a)`. The piped value is the first positional
argument; a parenthesized suffix MAY include named arguments, which fill their
parameters by name. A name equal to the first parameter is a duplicate.

### REPL (§29)

**Proposed.** The session carries each function's parameter names and
annotations across submissions, so a named call in a later submission resolves
against the persistent signature.

### Frozen decision (§33)

**Proposed addition.** Named-argument matching: positional arguments fill the
next unfilled parameter; named arguments match parameter names exactly and
case-sensitively; positional arguments precede named arguments; duplicate,
missing, and unknown parameters are `E3001`; named arguments are limited to
directly resolved top-level functions; argument evaluation remains source
order.

No new diagnostic code.

---

## Standard Library Future Extension

Future (not Feature 002): extend callable signatures with canonical parameter
names so built-ins and methods can accept named arguments.

```text
Future:
    registry Param { accepts } → Param { name: Option<&'static str>, accepts }
    then named arguments extend to built-ins and methods.
```

Parameter names must be treated as **deliberate public API metadata**, not
inferred from implementation variable names, because:

* a name becomes part of the frozen call contract once exposed;
* renaming an implementation variable must not change the language;
* names must be chosen once and remain stable across releases (the same
  stability guarantee as diagnostic codes).

Feature 002 does not touch `src/stdlib/signatures.rs`.

---

## Implementation Plan

The smallest implementation path reuses existing infrastructure.

```
existing AST Arg             (already carries Option<String>; already parsed by cons_arg)
        +
FnSig parameter metadata     (extend params from Vec<Option<Ty>> to carry the name)
        +
existing resolver            (resolves_to_user_function, unchanged)
        +
existing compatible_with     (unchanged)
        +
existing diagnostics         (E3001, unchanged)
```

Steps, in order:

1. **Grammar / parser (`src/parse/mod.rs`, `docs/grammar.md`).** Make call
   argument parsing use the `cons_arg` rule for `Expr::Call` and
   `Expr::Method`; enforce positional-then-named (a positional argument after
   a named one is `E1006`).
2. **AST (`src/ast/mod.rs`).** Change `Expr::Call` and `Expr::Method` to carry
   `Vec<Arg>` instead of `Vec<Expr>`. Update every consumer: `parse`
   (construct + `span_of` + depth checks), `check`, `run` (`eval_call`, method
   dispatch), and `desugar_pipe` (insert the piped receiver as
   `Arg { name: None, .. }`).
3. **Checker (`src/check/mod.rs`).** Extend `FnSig` so each parameter carries
   `(name, Option<Ty>)`. Replace the raw count check in `check_user_call` with
   a mapping step that produces an argument-index-per-parameter mapping and
   reports unknown / duplicate / missing as `E3001`; then run the existing
   per-parameter type check over the mapped arguments. Reject named arguments
   for built-ins, methods, and unresolved callees (`E3001`).
4. **Runtime (`src/run/mod.rs`).** Read `arg.value` for each argument and
   evaluate in source order. Where the checker has produced a positional
   mapping, invoke the existing `Vec<Value>` binding unchanged; runtime
   validation remains.
5. **REPL (`src/repl.rs`, `GlobalDecl`).** Carry parameter names alongside
   types in `GlobalDecl::Function`.
6. **Docs.** Apply the proposed `LANGUAGE_SPEC` amendment, update
   `docs/grammar.md`, `docs/contract.md`, README, and the implementation
   report.

Avoid: a new type system, a new callable abstraction, a new runtime calling
protocol, a parallel signature registry, or any change to enum-payload or
struct-construction semantics.

Likely files: `src/parse/mod.rs`, `src/ast/mod.rs`, `src/check/mod.rs`,
`src/run/mod.rs`, `src/repl.rs`, `docs/*`, `tests/*`. None are modified in
this phase.

---

## Property / Differential Testing

Design properties for the future implementation.

* **Permutation equivalence (binding).** For `fn f(a, b, c)`, the positional
  call `f(v1, v2, v3)` and the named permutation
  `f(c: v3, a: v1, b: v2)` produce the same parameter bindings and result.
* **Evaluation-order invariance.** The observable order of side effects in
  named arguments equals their source order, independent of parameter
  declaration order.
* **Rejection determinism.** Duplicate assignment, unknown parameter, and
  missing parameter each produce a deterministic `E3001` at check time for a
  directly resolved call.
* **Additivity.** A call with no named arguments yields the same static and
  runtime verdict as before Feature 002.
* **Checker/runtime agreement.** If the checker accepts a named call, the
  runtime does not fail with a type error at that call.
* **Pipeline equivalence.** `x |> f(b: v)` equals `f(x, b: v)` equals the
  positional call with the same binding.

---

## Test Matrix (future implementation)

This matrix is specified, not implemented. Each row is source → expected.

### Basic
| Source | Expected |
|---|---|
| `fn f(x) { return x }` + `f(x: 1)` | runs, `1` |
| `fn f(x, y) { return x + y }` + `f(x: 1, y: 2)` | runs, `3` |

### Reordering
| `fn f(x, y) { return x + y }` + `f(y: 2, x: 1)` | runs, `3` |
| `fn f(a, b, c) { return [a,b,c] }` + `f(c: 3, a: 1, b: 2)` | `[1, 2, 3]` |

### Mixed
| `fn f(x, y) { return x + y }` + `f(1, y: 2)` | runs, `3` |

### Invalid ordering
| `fn f(x, y) { }` + `f(x: 1, 2)` | parse error `E1006` |

### Unknown name
| `fn f(x) { }` + `f(z: 1)` | `E3001` at check |

### Duplicate
| `fn f(a, b) { }` + `f(1, a: 2)` | `E3001` at check |
| `fn f(a) { }` + `f(a: 1, a: 2)` | `E3001` at check |

### Missing
| `fn f(a, b, c) { }` + `f(a: 1)` | `E3001` (missing `b`, `c`) at check |

### Type checking
| `fn f(x: int) { }` + `f(x: "w")` | `E3001` at check |
| `fn f(a: int, b: int) { }` + `f(b: 1, a: "x")` | `E3001` at check |

### Unknown argument type
| `fn f(x: int) { }` + `f(x: none)` | accepted (permissive) |

### Shadowing
| global `fn f(x: int)`, local callable `f`, `f(x: 1)` | `E3001` — global signature not applied; named call unresolved |

### Hoisting
| named call to a function declared later | checked |

### Mutual recursion
| named calls in mutually recursive functions | checked both directions |

### Dynamic call
| `let f = (a) -> a` + `f(x: 1)` | `E3001` |

### Built-in
| `len(value: [1])` | `E3001` (out of scope) |

### Method
| `xs.map(fn: (x) -> x)` | `E3001` (out of scope) |

### Pipeline
| `fn move(dx: int, dy: int)` + `p \|> move(dy: 2)` | runs as `move(p, dy: 2)` |
| `p \|> move(dx: 2)` | `E3001` (duplicate `dx`) |

### Evaluation order
| `fn f(a, b)` + `f(b: side1(), a: side2())` | prints `side1` then `side2` |

### REPL
| `fn greet(name, punctuation)` then `greet(punctuation: "!", name: "João")` | runs |
| a failed named call between valid ones | session preserved |

### Alias parameter types
| `type Id = int` + `fn f(x: Id)` + `f(x: 1)` | runs |
| `f(x: "s")` | `E3001` at check |

---

## Final Review Checklist (this phase)

```
Q1 resolved (dynamic callee named args → E3001)                 YES
Q2 resolved (unknown parameter name → E3001)                    YES
Q3 resolved (built-ins and methods out of scope)                YES
Q4 resolved (positional arguments precede named arguments)      YES
evaluation-order rule frozen                                    YES
parameter-satisfaction model defined                            YES
pipeline behavior defined                                       YES
REPL signature persistence defined                              YES
compatibility classified                                        YES
LANGUAGE_SPEC amendment drafted                                 YES
implementation plan defined                                     YES
test matrix defined                                             YES

source code changed = NO
tests changed       = NO
grammar changed     = NO
AST changed         = NO
runtime changed     = NO
LANGUAGE_SPEC changed = NO
only docs/FEATURE_002_DESIGN.md changed
```
