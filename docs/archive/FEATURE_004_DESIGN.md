# Feature 004 — Destructuring `let`

## Status

Design (pre-implementation), **normative and implementation-ready**.

**Baseline:** `d9aed732f0d1f47cd83a1853dd918b986d123e4c`
(`feat: add static field-type propagation`, Feature 003).
**Branch:** `rewrite/v3-rust`.
**Semantic authority:** `docs/LANGUAGE_SPEC.md` (frozen).
**Depends on:** the existing `Pattern` system (`src/ast`), the pattern parser
and `for`/`match` machinery (`src/parse`, `src/run`), and the conservative
checker (`src/check`).

No open questions remain in this document. Every semantic decision below is
resolved from the existing implementation or from a deliberate, justified
extension consistent with Aura's frozen philosophy ("one spelling per
construct", `Ty::Unknown` conservatism, deterministic evaluation, resource
safety).

---

## Motivation

`match` and `for` already accept a full pattern (`Pattern`), and the runtime
already binds patterns from values via `Runtime::bind_pattern`. The only place
that cannot use this machinery is `let`: it accepts exactly one identifier
(`IDENT [": " type] "=" expr`).

As a result, extracting a value from a list or a variant requires either an
indexing dance or a throwaway `for` loop:

```aura
# today
let pair = [1, 2]
let a = pair[0]
let b = pair[1]
```

or

```aura
# today
for [a, b] in [pair] { ... }   # awkward one-iteration loop
```

Feature 004 lets `let` bind a pattern directly:

```aura
let [a, b] = pair
let Ok(x) = result
```

It reuses existing infrastructure, adds no new pattern syntax, adds no
inference, and does not change the type checker's conservative boundary.

---

## Current Language Behavior (verified against source)

* `Pattern` (`src/ast/mod.rs`) has variants: `Int`, `Str`, `Bool`, `None`,
  `Bind`, `List`, `Variant`.
* `Parser::pattern` (`src/parse/mod.rs`) parses all of them. An identifier is
  parsed as `Variant(name, [])` when it starts uppercase, otherwise
  `Pattern::Bind(name)`.
* `Pattern::bindings()` returns every `Bind` name except `_`, in
  left-to-right, depth-first order.
* `Stmt::Let` (`src/ast/mod.rs`) is `{ mutable, name: String, ann, value,
  span }` — a single identifier only.
* The parser's `let` path (`stmt_inner`, `Tok::Let`) parses `IDENT`, optional
  `: type`, then requires `= expr`; a missing initializer is `E2005`
  (`LET_NO_INIT`).
* The checker's `Stmt::Let` (`src/check/mod.rs`) evaluates the initializer
  expression once, enforces the annotation against `infer(value)` (`E3001`),
  records the annotation or a non-`Unknown` inferred type in
  `value_types.last_mut()`, then calls `declare(name, mutable, span)`
  (`E2007` on redeclaration in the same scope).
* `Stmt::For` (`src/check/mod.rs`) checks the pattern with `check_pattern`
  (variant existence `E3002`, duplicate bindings `E2014`), pushes a scope,
  declares each `pat.bindings()` name immutably, checks the body, and pops.
* The runtime `Stmt::Let` (`src/run/mod.rs`) evaluates `value`, then
  `env.define(name, v, mutable)` in the **current** environment.
* The runtime `Stmt::For` evaluates the iterable, then for each item creates a
  fresh `env.child()`, calls `bind_pattern(pat, item, scope)`, and executes the
  body in that child scope.
* `Runtime::bind_pattern` (`src/run/mod.rs`) binds `Bind` names into the given
  env; for `List` it requires a `List` of equal length (else
  `E3001 "list pattern arity mismatch"`), for `Variant` a matching
  `Value::Variant` with equal payload length (else `E3001 "variant pattern
  mismatch"`), and for `Int`/`Str`/`Bool`/`None` it does **nothing** (literal
  patterns are not validated by `bind_pattern`; only `match_pattern` validates
  them, and `for` does not call `match_pattern`).
* `Env` (`src/run/mod.rs`) has `root`, `child`, `define`, `get`, and a private
  `assign`. There is **no** merge/iteration API.
* Blocks executed as part of a statement body (`while`, `loop`, function
  bodies) are entered with a child scope by their caller
  (`exec_block(..., true)` or an explicit `env.child()`), so a `let` inside
  them persists for the rest of that block and is discarded when the block
  ends. A plain `{ ... }` block statement is not a construct in Aura
  (`Expr::Block` is an expression; `Stmt` has no bare block).

---

## Exact Syntax

```
let_stmt      = "let" [ "mut" ] let_pattern [ ":" type ] "=" expr terminator ;
let_pattern   = IDENT | let_list_pattern | let_variant_pattern ;
let_list_pattern    = "[" [ let_pattern { "," let_pattern } [ "," ] ] "]" ;
let_variant_pattern = IDENT [ "(" [ let_pattern { "," let_pattern } ] ")" ] ;
```

**Normative rule.** The grammar production for `let` changes from an
identifier to a `let_pattern` (the subset of `pattern` that contains no
literal/`none` forms). The optional `: type` annotation is retained
syntactically but is **only legal when the pattern is a single `IDENT`/`Bind`**
(see "Type Annotations"). In all other cases an annotation is a parse error
reusing `E1006` (`EXPECTED`).

**Normative rule.** `let x = expr` continues to parse as
`Pattern::Bind("x")` and is semantically identical to today's `let x = expr`.
`let mut x = expr` is unchanged.

**Normative rule.** `let [a, b] = expr`, `let Foo(x) = expr`, and nested
combinations thereof are the new destructuring forms.

There is **no** new keyword, token, or Pattern variant.

---

## Grammar Impact

* `docs/LANGUAGE_SPEC.md` §4.4: `let_stmt` production changes `IDENT` to
  `let_pattern`, with the dedicated `let_pattern` / `let_list_pattern` /
  `let_variant_pattern` productions (no literal forms).
* `docs/LANGUAGE_SPEC.md` §4.2 (`const_decl`): unchanged. Top-level `let`
  remains an identifier-only module constant; destructuring at the top level is
  **not** introduced.
* `docs/grammar.md`: the same `let_stmt` line is updated to match.
* No other production changes.

**Parser disambiguation.** `Parser::pattern` is reused verbatim. It is
unambiguous for `let` because:
* a lowercase/`_` identifier parses as `Bind` (ordinary `let`);
* an uppercase identifier parses as either a zero-payload `Variant` or, with
  `(`, a payload `Variant`;
* `[` starts a list pattern.

An annotation `:` after a pattern whose single binding is a `Bind` is parsed as
today. An annotation after any other pattern is rejected.

---

## Supported Pattern Forms

**In scope for `let`:**

* `Bind` — `let x = e`, `let _ = e`.
* `List` — `let [a, b] = e`, including nested lists `let [a, [b, c]] = e`.
* `Variant` — `let Ok(x) = e`, `let None = e` form as `Variant("None", [])`
  only if `None` is an enum tag; a bare `none` literal is a `Pattern::None`
  and is excluded (below). `let Circle(r) = e`, nested
  `let Some([a, b]) = e`.
* Nested combinations of `Bind`, `List`, and `Variant`.

**Excluded from `let`:**

* `Pattern::Int`, `Pattern::Str`, `Pattern::Bool`, `Pattern::None` — literal
  and `none` patterns are refutable and are **not** accepted in a `let`
  pattern. A literal or `none` anywhere in a `let` pattern is rejected with
  `E1006` (`EXPECTED`).
* Annotations on any non-single-`Bind` pattern (`let [a, b]: [int] = e` and
  similarly for variants) — rejected with `E1006`.
* `mut` on a destructuring pattern (`let mut [a, b] = e`) — rejected with
  `E1006`. `mut` remains legal only for the single-`Bind` form.
* Destructuring in function parameters, `catch` bindings, top-level module
  constants, and any other binding position.
* New pattern forms (map/struct/rest/wildcard-extended patterns).

This exclusion set is deliberate: `bind_pattern` does not validate literal
patterns, so admitting them would silently bind nothing for a pattern that
looks like it asserts a value. Excluding them keeps `let` irrefutable in
spirit and avoids a new matcher path or a new error code.

---

## Semantic Rules

**Normative rule (binding).** `let P = e` evaluates `e` exactly once, then
binds every name in `P.bindings()` to the corresponding sub-values of the
result. Every bound name is **immutable**, exactly like `let x = e`.

**Normative rule (names).** The set of names introduced is exactly
`P.bindings()`: every `Pattern::Bind(n)` with `n != "_"`, in left-to-right,
depth-first order. `_` binds nothing. Literal patterns bind nothing (and are
excluded from `let` anyway).

**Normative rule (duplicate names).** If `P.bindings()` contains a repeated
name, the checker rejects the statement with `E2014` (`DUPLICATE_BINDING`),
reusing `check_pattern`. This is the same rule `match` and `for` use.

**Normative rule (shadowing).** Destructured names shadow outer bindings
exactly as ordinary `let` does. Redeclaring a name already declared **in the
same scope** is `E2007` (`REDECLARED`), unchanged from ordinary `let`.

**Normative rule (nested patterns).** A nested `List` or `Variant` pattern is
matched recursively. A mismatch at any depth fails the whole statement (see
"Failure / Atomicity"). Names bound by outer components are visible only after
the entire statement succeeds.

---

## Binding / Scope Rules

**Normative rule.** Destructured bindings have the same lifetime and scope as
an ordinary `let` in the same position: they are defined in the current
environment/scope and are visible to every subsequent statement in that block,
including nested statements. They are not visible before the statement, and
they cease to exist when the enclosing block's scope ends.

Concretely, matching the ordinary `let` model exactly:

* In a function body, the names are defined in the function's environment and
  remain visible until the function returns.
* In a `while`/`loop` body, the names are defined in that iteration's child
  scope and are discarded at the end of the iteration, like any `let` there.
* In a `for` body, the names are defined in the per-iteration child scope,
  like any `let` there.
* At REPL top level, names are defined in the session globals (see "REPL
  Behavior").

**Normative rule.** All names become visible simultaneously, after the RHS has
been evaluated and the pattern has fully matched. No name is visible to the
RHS expression itself (the RHS is evaluated in the enclosing scope, before any
binding).

---

## Evaluation Order

**Normative rule.** The initializer expression is evaluated exactly once,
before any pattern binding occurs, using Aura's strict left-to-right
evaluation. Pattern binding then proceeds in `P.bindings()` order.

**Normative rule.** Binding order is **not** semantically observable: each
bound name is bound to a value already produced by the single RHS evaluation,
and no user code runs during pattern matching. Therefore the language does not
specify an observable binding order beyond "after RHS evaluation"; the
implementation uses `P.bindings()` order for determinism.

**Normative rule.** A `return`/`throw`/`break`/`continue` produced while
evaluating the RHS propagates before any binding, exactly as with ordinary
`let`.

---

## Failure / Atomicity

**Normative rule.** Destructuring `let` is **atomic**: if the runtime value
does not match the pattern, the statement fails with the existing `E3001`
diagnostic and **no** binding is created or modified. A failed destructuring
`let` leaves the environment exactly as it was immediately before the
statement.

**Normative rule.** The diagnostics reused are exactly those produced by
`Runtime::bind_pattern`:

* list arity mismatch → `E3001`, message `list pattern arity mismatch`;
* non-list value for a list pattern → `E3001`, message `pattern expects a list`;
* variant tag or payload-length mismatch → `E3001`, message `variant pattern
  mismatch`;
* non-variant value for a variant pattern → `E3001`, message `pattern expects
  a variant`.

No new error code is introduced.

**Implementation consequence.** `bind_pattern` mutates its target environment
incrementally and is therefore not atomic by itself (verified: a `for` over
`[[1,2],[3]]` binds `a` for the first item and fails on the second). `for`
does not expose this because each iteration uses a throwaway `env.child()`.
To preserve atomicity for `let`, the runtime must bind into a **temporary
child scope** and, only on success, transfer the bound names into the current
environment. Because `Env` exposes no merge/iteration API, the transfer is
performed by iterating `P.bindings()` and reading each name from the temporary
scope with `Env::get`, then `Env::define`-ing it into the current environment.
This requires no change to `Env`'s public API and no change to `bind_pattern`.

Rationale for atomicity (evidence-based, not invented): ordinary `let` never
leaves a partially-initialized binding — if the initializer fails, nothing is
bound. Destructuring must preserve that invariant; otherwise a failed
submission could leak names into scope, violating "a failed submission does
not corrupt the session" (§33 item 4) and making error recovery non-deterministic.
The temporary-scope approach is the only mechanism consistent with the
existing `Env` design and the `for` precedent.

---

## Checker Behavior

**Normative rule.** For a destructuring `let`, the checker:

1. checks the initializer expression via `self.expr(value)` exactly as today;
2. validates the pattern with `check_pattern` (variant existence `E3002`,
   duplicate bindings `E2014`);
3. declares every name in `P.bindings()` immutably via `self.declare` in the
   current scope (`E2007` on same-scope redeclaration);
4. performs **no** type inference, annotation check, or `value_types` update
   for destructured names.

**Normative rule (static types).** Destructured names have **no static
type**: they are treated as `Ty::Unknown` by the checker. No entry is added to
`value_types` for them. This preserves the `Ty::Unknown` conservative
boundary and introduces no speculative inference.

**Normative rule (no change to Feature 003).** The field-type propagation
rule (§17.5) is unchanged. A destructured name is `Unknown`, so a field read
on it remains `Unknown`, exactly as an unannotated ordinary `let` is today.

**Normative rule (ordinary `let`).** The bare-`Bind` form continues through
the existing annotated/unannotated `Stmt::Let` logic unchanged: annotation
check (`E3001`), `value_types` recording, and Feature 003 field-read
propagation for annotated struct bindings are all preserved byte-for-byte.

**Normative rule (annotation check).** No annotation is ever checked against a
destructured pattern, because annotated destructuring is rejected at parse
time. `let x: T = e` remains the only annotated `let`.

---

## Runtime Behavior

**Normative rule.** For the bare-`Bind` form, the runtime is unchanged:
`eval(value)` then `env.define(name, v, mutable)`.

**Normative rule.** For a destructuring form, the runtime:

1. evaluates the RHS once to a value `v`;
2. creates a temporary child scope `tmp = env.child()`;
3. calls `bind_pattern(P, &v, &tmp)`; on error, propagates the diagnostic and
   creates no binding in `env`;
4. on success, for each `n` in `P.bindings()`, reads `tmp.get(n)` and calls
   `env.define(n, value, false)`.

**Normative rule.** All destructured bindings are immutable. `mut` is only
legal on the bare-`Bind` form.

**Normative rule.** No change is made to `Value`, to any other statement, or
to `bind_pattern` itself.

---

## REPL Behavior

**Normative rule.** Each name in a destructured `let` is persisted across
submissions as a session binding, exactly as an ordinary unannotated `let`
is: the REPL adds a `GlobalDecl::Binding { name, mutable: false, ty: None }`
for each name in `P.bindings()` that is not already declared.

**Normative rule.** Because destructured names carry no annotation, they
persist with `ty: None` and are `Unknown` in later submissions. Feature 003
behavior for annotated ordinary bindings (`let user: User = ...`) is
unchanged.

**Normative rule (failure isolation).** When a destructuring `let` fails its
static check or its runtime match, the REPL does not persist any of its
declarations and prior session state is untouched. This follows from the
existing REPL control flow: declarations are recorded only after
`check_stmt`/`exec_stmt` succeed.

**Normative rule.** The REPL continues to record declarations only for the
statement form; no new persistence model is introduced.

---

## Error Model

| Condition | Code | Phase | Source |
|---|---|---|---|
| Duplicate name within one `let` pattern | `E2014` | check | `check_pattern` |
| Unknown variant tag in a `let` pattern | `E3002` | check | `check_pattern` |
| Redeclaring a name already in the same scope | `E2007` | check | `declare` |
| Annotation on a non-`Bind` pattern | `E1006` | parse | new parser guard |
| Literal/`none` pattern in `let` | `E1006` | parse | new parser guard |
| `mut` on a non-`Bind` pattern | `E1006` | parse | new parser guard |
| List arity mismatch at runtime | `E3001` | runtime | `bind_pattern` |
| Non-list for a list pattern | `E3001` | runtime | `bind_pattern` |
| Variant mismatch at runtime | `E3001` | runtime | `bind_pattern` |
| Non-variant for a variant pattern | `E3001` | runtime | `bind_pattern` |

**Normative rule.** No new error code is introduced. The `E1006` parse guards
reuse the existing `EXPECTED` code, which is already the parser's generic
"malformed construct" code.

---

## Compatibility Classification

This is a **syntax extension** (per `docs/FEATURE_ROADMAP.md` versioning
policy): it adds new grammar without changing the meaning of any existing
valid program.

* **Existing `let x = e` / `let mut x = e` / `let x: T = e`:** identical AST
  (`name` is populated from the `Bind`), identical checker behavior, identical
  runtime behavior, identical REPL persistence.
* **Existing `for`:** unchanged. `for` already accepts patterns and binds with
  `bind_pattern`; Feature 004 does not touch it.
* **Existing `match`:** unchanged.
* **No previously valid program changes meaning.** The only newly rejected
  inputs are `let <literal-pattern>`, `let <annotated non-Bind pattern>`, and
  `let mut <non-Bind pattern>`, none of which parse today.
* **No runtime behavior change** for any existing program.

---

## Resource Invariants

No resource constant is changed. The following are preserved exactly:

* AST depth limit 256 → `E1015`;
* parser recursion backstop 2048 → `E1015`;
* call-frame limit 512 → `E4011`;
* range materialization limit 10,000,000 → `E4013`;
* no panic, overflow, or undefined behavior on any well-formed program.

Pattern parsing and binding are bounded by the existing `Parser::enter/leave`
depth guard and the runtime `ast_depth` guard. The temporary child scope for
atomic binding is a single `Rc` allocation and does not recurse beyond the
pattern's own bounded depth.

---

## Interactions with Features 001–003

* **Feature 001 (static argument checking):** unchanged. Destructured names
  are `Unknown`, so an argument that is a destructured name is not statically
  rejected (conservative). No interaction.
* **Feature 002 (named arguments):** unchanged. No interaction.
* **Feature 003 (field-type propagation):** unchanged. A field read on a
  destructured name stays `Unknown` because the name has no recorded type.
  Annotated ordinary `let` retains its Feature 003 behavior.
* **Pipeline:** unchanged. A destructured `let` initializer may contain a
  pipeline; no pipeline-specific handling is added.
* **Functions / closures:** unchanged. Destructuring is not permitted in
  parameters; closures capture destructured names by reference exactly as they
  capture ordinary `let` names.
* **Aliases:** unchanged. A variant pattern's tag resolution uses the existing
  `variants`/`types` tables; aliases are not patterns.
* **Structs / enums / lists:** `List` patterns match `Value::List`;
  `Variant` patterns match `Value::Variant`. Structs are not destructurable
  (no struct pattern exists) and this feature does not add one.

---

## Testing Strategy

**Parser tests**

* `let x = e` and `let mut x = e` and `let x: T = e` produce the same AST as
  before (regression).
* `let [a, b] = e`, `let [a, [b, c]] = e`, `let Ok(x) = e`,
  `let Some([a, b]) = e`, `let [Ok(a), Err(b)] = e` parse.
* `let 5 = e`, `let "s" = e`, `let true = e`, `let none = e` → `E1006`.
* `let [a, b]: [int] = e` → `E1006`.
* `let mut [a, b] = e` → `E1006`.
* `let [a, b` (missing `]`) → `E1006`.
* `let [a, b] =` (missing initializer) → `E2005`.
* Depth boundary: a deeply nested list pattern parses within limits and
  reports `E1015` past the AST limit, never a native overflow.

**Checker tests**

* list destructuring valid;
* nested list pattern valid;
* variant destructuring valid (`Ok(x)`, `Err(x)`);
* nested variant/list pattern valid;
* duplicate binding in one pattern → `E2014`;
* unknown variant tag → `E3002`;
* same-scope redeclaration → `E2007`;
* shadowing an outer binding is accepted;
* destructured names are `Unknown` in the checker (no speculative type);
* `let x: T = e` annotation check still works (`E3001`);
* Feature 001: passing a destructured name to an annotated parameter is not
  statically rejected;
* Feature 003: a field read on a destructured name stays `Unknown`.

**Runtime tests**

* list destructuring binds correct values;
* nested list/variant destructuring binds correct values;
* list arity mismatch → `E3001 "list pattern arity mismatch"`;
* non-list value for list pattern → `E3001 "pattern expects a list"`;
* variant mismatch → `E3001 "variant pattern mismatch"`;
* non-variant value for variant pattern → `E3001 "pattern expects a variant"`;
* `let _ = e` binds nothing and does not error;
* `_` inside a list/variant pattern binds nothing.

**Scope / shadowing tests**

* names visible to later statements in the same block (function body);
* names not visible after a `while`/`loop`/`for` body ends;
* nested block sees outer destructured names;
* destructured name shadows an outer binding;
* redeclaration in the same scope → `E2007`.

**Atomicity tests**

* a failing destructuring `let` leaves a same-named prior binding unchanged;
* a failing destructuring `let` introduces none of its names;
* a nested failure (`let [a, [b, c]] = [1, [2]]`) binds nothing, including
  `a`.

**REPL tests**

* list destructuring across submissions persists each bound name;
* a failed destructuring submission preserves prior bindings and does not
  introduce partial names;
* annotated ordinary `let` (Feature 003) still persists its type.

**Interaction tests**

* `for` and `match` parity: the same pattern yields the same bindings in
  `for`, `match`, and destructuring `let`;
* Functions/closures capture destructured names;
* pipeline initializer: `let [a, b] = xs |> pair_up` works.

**Resource tests**

* deeply nested `let` pattern at the AST limit accepts, one past rejects with
  `E1015`, never a crash.

---

## Property / Differential Strategy

Using the existing `proptest` infrastructure (`tests/property.rs`):

* **Bound-names property.** For every generated valid destructuring `let`
  that runs successfully, the set of newly defined names equals
  `P.bindings()`, and each is immutable.
* **Atomicity property.** For every generated destructuring `let` whose RHS
  does not structurally match the pattern, the environment after the failed
  statement is identical (same name→value mapping) to the environment before
  it, and the diagnostic code is `E3001`.
* **Identifier-equivalence property.** For a single `Bind` pattern, the
  compiled/executed behavior is identical to the pre-Feature-004 `let x = e`
  path (same output, same diagnostics).
* **Differential parity.** For a pattern `P` and a value `v` that matches,
  binding via `for P in [v] { ... }` and via `let P = v` yields the same
  values for `P.bindings()`.

No reference implementation is invented; the differential oracle is the
existing `for`/`match` binding behavior.

---

## Implementation Plan

**Parser (`src/parse/mod.rs`)**

* In `stmt_inner`'s `Tok::Let` branch, parse an optional `mut` (only legal
  before a `Bind`), then call `self.pattern()?` instead of `self.ident(...)`.
* If the pattern is a single `Bind`, preserve the existing path exactly
  (optional `: type`, then require `=`, build `Stmt::Let`).
* If the pattern is not a single `Bind`: reject a following `:` with `E1006`,
  reject `mut` with `E1006`, reject any literal/`None` sub-pattern with
  `E1006`, require `=`, and build the destructuring `Stmt::Let`.
* Keep `Tok::Let` at top level (`const_item`) unchanged.

**AST (`src/ast/mod.rs`)**

* Require a representation for `Stmt::Let` that can hold either a single
  annotated name or a pattern. The minimal, low-risk choice: keep the existing
  `Stmt::Let { mutable, name, ann, value, span }` for the ordinary form and
  add a sibling variant, e.g. `Stmt::LetPattern { pattern, value, span }`, so
  every existing `Stmt::Let` consumer is untouched and the new path is
  additive. (Equivalent alternative: a `pattern` field plus `name`/`ann`
  options; the additive variant is preferred to minimize regression surface.)
* Reuse the existing `Pattern` enum unchanged.

**Checker (`src/check/mod.rs`)**

* Add handling for the destructuring variant: `self.expr(value)?`,
  `self.check_pattern(pattern)?`, declare each `pattern.bindings()` name via
  `self.declare(&b, false, span)?`. No `value_types` entry, no annotation
  check, no inference.
* Leave ordinary `Stmt::Let` handling and Feature 003's `infer` untouched.

**Runtime (`src/run/mod.rs`)**

* Add handling for the destructuring variant: evaluate RHS once; create
  `tmp = env.child()`; `bind_pattern(pattern, &v, &tmp)?`; on success, for
  each name in `pattern.bindings()`, `env.define(name, tmp.get(name)..., false)`.
* Do **not** modify `bind_pattern`, `Env`, or any other statement.

**REPL (`src/repl.rs`)**

* In `eval_line`, after a successful destructuring statement, push a
  `GlobalDecl::Binding { name, mutable: false, ty: None }` for each
  `pattern.bindings()` name not already declared. Ordinary `Stmt::Let`
  persistence is unchanged.

**Must remain untouched**

* `src/stdlib/` (no new builtins/methods);
* `src/bridge/` (Python);
* resource constants (`MAX_AST_DEPTH`, `MAX_PARSE_DEPTH`, `MAX_CALL_FRAMES`,
  range cap);
* `bind_pattern`, `match_pattern`, `Env`, `Value`;
* `for`, `match`, function/closure machinery, pipeline, aliases, structs,
  enums.

---

## Non-Goals

* Destructuring `let` annotations (`let [a, b]: [int] = e`).
* Destructuring function parameters, `catch` bindings, or top-level module
  constants.
* `mut` on non-`Bind` patterns.
* Literal, `none`, or range patterns in `let`.
* New pattern variants (map/struct/rest/wildcard-extended).
* Static refutability/irrefutability analysis.
* Any type inference for destructured names.
* Changes to `for`, `match`, `bind_pattern`, or `match_pattern`.
* `if let` / `while let`.

---

## Future Extensions

* Annotated destructuring patterns (`let [a, b]: [int] = e`) once element
  typing exists.
* Struct patterns (`let Point { x, y } = p`) when a struct-pattern form is
  designed.
* Destructuring parameters and `catch` bindings (would reuse this machinery).
* `if let` / `while let` (requires refutable pattern matching control flow).
* Rest patterns (`let [first, ..rest] = xs`).

Each of these is a separate feature requiring its own design; none is implied
by Feature 004.

---

## Resolved Design Decisions

1. **Supported forms.** `let` accepts `Bind`, `List`, `Variant`, and their
   nesting. Literals/`None` are rejected with `E1006`.
2. **Ordinary `let` equivalence.** A lowercase/`_` identifier is
   `Pattern::Bind`; `let x = e` is byte-for-byte equivalent to today. No
   ambiguity is introduced.
3. **Bindings.** Exactly `Pattern::bindings()`; all immutable; `_` binds
   nothing; nested patterns bind recursively.
4. **Duplicates.** `E2014` via `check_pattern`, matching `match`/`for`.
5. **Shadowing.** Outer shadowing allowed; same-scope redeclaration `E2007`.
6. **Scope.** Identical to ordinary `let` in the same position; visible to
   later statements in the block, discarded with the block.
7. **Evaluation order.** RHS once, before binding; binding after RHS; binding
   order not observable.
8. **Failure diagnostics.** Reuse `bind_pattern`'s `E3001` messages exactly.
9. **Atomicity.** Guaranteed via a temporary child scope + transfer on
   success; no partial or leaked bindings.
10. **Checker.** `check_pattern` + `declare` each name; no inference; no
    `value_types` entry; destructured names are `Unknown`.
11. **Annotations.** Only `let x: T = e`; annotated destructuring is `E1006`.
12. **REPL.** Each name persisted as an unannotated session binding;
    Feature 003 annotated ordinary bindings unchanged; failure isolation
    preserved.
13. **Error codes.** No new codes. `E1006`, `E2007`, `E2014`, `E3001`,
    `E3002` reused.
14. **Grammar.** Only `let_stmt` changes; `const_decl` and all other
    productions unchanged.
15. **Resource invariants.** All preserved; no constant changes; no new
    recursion.
16. **Compatibility.** Syntax extension; no existing valid program changes
    meaning.
17. **AST representation.** Additive sibling statement variant preferred;
    all existing `Stmt::Let` consumers untouched.
