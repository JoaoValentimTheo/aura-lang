# FEATURE_002 — Named Function Arguments — Design

**Status:** Design (pre-implementation)
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
  all named arguments (`{ positionals } { named }`). This is the rule used by
  several languages and is the least ambiguous.
* **(B) Free interleaving** — `f(a: 1, 2, b: 3)` allowed.

**Recommendation: (A).** It matches the ear, is trivial to state and to check,
and avoids the question of whether a positional argument may follow a named
one (which invites "skip a parameter and fill a later one positionally"
confusion). Interleaving is rejected as a syntax/check rule:

```
f(1, y: 2)      # valid
f(x: 1, 2)      # invalid: positional argument after a named one
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

**Recommendation: built-ins are OUT OF SCOPE for Feature 002.** A named
argument passed to a builtin is rejected statically as "`<builtin>` does not
accept named arguments" (or, if the name happens to be unknown, the unknown
name rule). Positional builtin calls are unchanged. This keeps one calling
convention per callable kind, avoids inventing names for a frozen registry,
and leaves the door open to add names later without breaking anything.

## Methods

Methods have no parameter names in `MethodSig` either, and user-defined
methods do not exist. **Methods are OUT OF SCOPE for Feature 002.** A named
argument in a method call is rejected statically, or handled only if the
method's parameters are positional. (See *Open Questions*.)

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

**Recommendation: (A) where possible, with a defined fallback.** For a
directly resolved user function, the checker maps named arguments to positions
and rejects errors; the AST `Arg` list is reordered into positional order
before/at execution, so the runtime call machinery is unchanged. For dynamic
calls (closures, values, unknown callees), the receiver has no declared
parameter list, so **named arguments are a static error** at the point where
the callee is provably not a named-capable function — or, where the callee is
`Unknown`, the runtime cannot resolve names and must reject them. This is an
important open question (below).

## Error Model

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

No new error code is proposed. An alternative for "unknown name" is `E2003`
(undefined name), mirroring the struct-field diagnostic; this is a wording
choice recorded as an open question, not a semantic one.

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

**Recommendation: (A).** Reuse `Arg` uniformly; there is already precedent
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

**LOW.** If the checker reorders named arguments into positional order (design
A under *Runtime Behavior*), the runtime's `Vec<Value>` binding is unchanged.
The only runtime question is what happens for **dynamic** calls that carry
named arguments; see open questions. The parser change means
`Expr::Call` now holds `Vec<Arg>`, so `eval_call` reads `arg.value` for each
argument; behavior is identical for positional calls.

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

## Property / Differential Testing

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

## Open Questions

### OQ1 — Should a named argument be allowed on a dynamic/unknown callee? **REQUIRES HUMAN DESIGN DECISION**

* **Why it matters.** A dynamic callee has no declared parameter list, so a
  name cannot be resolved statically or at runtime.
* **Possible answers.** (a) Always a static error when the callee is provably
  dynamic or `Unknown`. (b) Parse it and reject at runtime. (c) Disallow
  syntactically in a dynamic-call context (impossible to tell syntactically).
* **Recommendation.** (a): reject named arguments at check time whenever the
  callee is not a directly resolved user function. This keeps one convention
  and avoids a runtime name table. Human confirmation is desirable because it
  decides whether `let f = ...; f(x: 1)` is a hard error.

### OQ2 — `E3001` vs `E2003` for an unknown parameter name. **Human preference (wording)**

* The struct-field precedent uses `E2003` for an unknown field. Reusing
  `E3001` keeps all "bad call" errors in one family; using `E2003` matches the
  field analogy. This is a diagnostic-taste decision, not a semantic one.

### OQ3 — Named arguments for built-ins and methods: out of scope now? **Human confirmation**

* The recommendation is out of scope to avoid inventing names for a frozen
  registry. If the project wants `len(value: xs)`, the registry must gain
  names first. Confirm out-of-scope.

### OQ4 — Should `f(x: 1, y: 2)` and `f(y: 2, x: 1)` be considered the "same
call" for any caching/memoization semantics? **Not applicable now**

* Aura has no memoization; noted only so a future optimizer does not assume
  argument order is semantic. No decision needed.

### OQ5 — Free interleaving vs positional-then-named. **Human confirmation**

* The recommendation is positional-then-named (model A). Confirm, since it is
  the feature's most visible ergonomic rule.

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

## Proposed Specification Amendment (for the implementation phase)

**Current (§15.7).**

> Function calls are positional; there are no named arguments at a call site,
> no default parameter values, and no variadic parameters in this version.

**Proposed (§15.7).**

> A call to a directly resolved top-level function MAY supply arguments by
> parameter name, using `name: value`. Positional arguments fill the next
> unfilled parameter in declaration order; a named argument fills the
> parameter with that name. All positional arguments MUST precede all named
> arguments. Every declared parameter MUST be supplied exactly once; a
> duplicate, missing, or unknown parameter is `E3001`. Named arguments are not
> accepted by builtins, methods, or dynamically resolved callables. There are
> still no default or variadic parameters in this version.

**Current (§4.5).**

> `call_args = expr { "," expr }` … "function calls are positional".

**Proposed (§4.5).**

> `call_args = arg { "," arg } ; arg = [ IDENT ":" ] expr ;`

**Current (§23).**

> `x |> f(a)` is `f(x, a)`.

**Proposed (§23).**

> `x |> f(a)` is `f(x, a)`. The piped value is the first positional argument;
> a parenthesized suffix MAY include named arguments, which fill their
> parameters by name. A name equal to the first parameter is a duplicate.

Plus the §6.5 arity generalization, the §29 REPL note, and a §33 frozen
decision. No new error code.

---

## Final Review Checklist (this phase)

```
source code changed = NO
tests changed       = NO
grammar changed     = NO
AST changed         = NO
runtime changed     = NO
LANGUAGE_SPEC changed = NO
only docs/FEATURE_002_DESIGN.md new
```
