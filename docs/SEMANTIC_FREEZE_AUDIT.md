# Aura Language — Semantic Freeze, Contract & Conformance Audit

**Audit HEAD:** `d940fbed54d8afee020e6e7949fe6b2266847422`
**Branch:** `rewrite/v3-rust` (tracks `origin/master`)
**Working tree:** clean at the time of audit
**Method:** full source inspection of `src/`, every test file, every doc, plus
adversarial execution against the built binary. No source, test, framework, or
configuration file was modified to produce this report. Temporary probe
programs were written under `/tmp/opencode/probes` and deleted afterward.

This report formalizes *what Aura is today*. It does not propose features and
does not implement fixes. Where the repository does not decide a behavior, the
behavior is marked `AMBIGUOUS`, `UNSPECIFIED`, or `DESIGN DECISION REQUIRED`.

---

## 1. Executive Summary

**Current HEAD:** `d940fbed54d8afee020e6e7949fe6b2266847422` — confirmed present,
clean tree, in sync with `origin/master`. The claims in `docs/CORRECTIONS.md`
(commit `d940fbe`) were spot-verified and hold: F-01 `1.5 + 2.5` = `4.0`; F-04
return-type propagation works; F-10 rejects `2**63` (E4013) and non-string dict
keys (E5002).

**Repository state:** 152 `#[test]` functions across 13 integration files
(151 execute in any single feature configuration). `cargo test --all-features`
passes. No uncommitted changes.

**Semantic maturity:** The language is **small, coherent, and almost entirely
total** at the expression level. An exhaustive probe of all 384
`(value, operator, value)` arithmetic combinations produced zero `E4999`
internal errors, and a broad sweep of operators, indexing, methods, and builtins
found **no case where the checker accepted a program that failed with a
front-end (`E1xxx`) or internal (`E4999`) diagnostic**. Not one checker
false-positive (checker rejects, runtime would accept) was found.

**Internally coherent:** Yes at the expression/statement level. The runtime
model is a clean `Ctl` (Val/Return/Break/Continue/Throw) tree-walker; control
flow propagates uniformly; `finally` precedence is well-defined and consistent.

**Documentation coherent:** Mostly, but with concrete defects (§27):
* the effective parenthesized-nesting limit is ~61, not the documented 256, and
  reports `E1006` instead of `E1015`;
* the contract says float `%` by zero is `E4007`, but it yields `nan`;
* the contract does not describe struct-literal field validation, no-arg method
  auto-call, or the absence of an empty-map literal.

**Checker/runtime coherent:** Broadly yes for *valid* programs, but the checker
does **not** enforce several annotation promises the contract makes:
function-parameter types at call sites, struct-field types at construction,
enum-payload types at construction, or function-call arity. These are
"checker accepts, runtime rejects" (or silently accepts) — §30 lists them.

**Further implementation work necessary?** Not to keep the language coherent.
What is necessary before expansion is a **decision pass** on the items in §29
(struct construction rules, empty-map literal, no-paren method auto-call,
parameter/payload annotation enforcement) and correction of the three
documentation/limit defects in §30. None is a redesign.

---

## 2. Current Language Pipeline

```
source (UTF-8 &str)
  │  lex::lex(src) -> Result<Vec<Token>>                         [src/lex/mod.rs]
  ▼
Vec<Token { tok: Tok, span: Span }>
  │  parse::parse(src) -> Result<Module>                         [src/parse/mod.rs:9]
  │      • Parser::module -> Parser::item -> … (recursive descent + Pratt)
  │      • parser-internal depth guard MAX_DEPTH = 128 (E1006 "nesting too deep")
  │      • enforce_depth(&Module): iterative, MAX_AST_DEPTH = 256 (E1015)
  ▼
ast::Module { items: Vec<Item> }                                 [src/ast/mod.rs]
  │  compile(src, mode) -> Result<Module>                        [src/lib.rs]
  │      = parse::parse + compile_module
  │  compile_module(&Module, mode) -> Result<()>                 [src/lib.rs]
  │      = check::Checker::module_in_mode
  │  check::Checker { hoist(); item()/expr()/stmt() }            [src/check/mod.rs]
  │      • resolves names, mutability, annotations, patterns
  │      • consults stdlib::signatures for builtins/methods
  ▼
checked ast::Module
  │  execute(module, stdout) -> Result<()>                       [src/lib.rs]
  │      = on_interp_thread(64 MB stack) { Interp::new().run(&module) }
  ▼
run::Interp::run(&Module)                                        [src/run/mod.rs:218]
  │      pass 1: declare_item (fn/struct/enum -> symbol tables)
  │      pass 2: source-order eval of Const and Expr items
  │      then: call `main` if present (E4027 if absent via checker)
  ▼
Ctl { Val | Return | Break | Continue | Throw } -> value::Value
```

**Entry points and their authority:**

| Surface | Function | Compile mode |
|---|---|---|
| Library (collect stdout) | `lib::run_source` → `execute(compile(src,"module"), sink)` | Module |
| Library (stdout) | `lib::run_toplevel_stdout` → `execute(compile(src,"module"), None)` | Module |
| CLI `run` | `main::cmd_run` → `lib::run_program` → `compile(src,"program")` | Program |
| CLI `check` | `main::cmd_check` → `lib::compile(src,"module")` | Module |
| CLI `eval` | `main::cmd_eval` → `lib::run_toplevel_stdout` | Module |
| REPL | `repl::run_with` → `Checker::with_globals` + `check_mode(Module)` + `interp.run_item` | Module (per submission) |
| Tests | `run_source` / `Checker::module` / `Checker::module_with_main` / `parse` directly | both |
| Python | `py_eval` / `py_import` / `py_call` (inside the runtime) | n/a |

**Duplicate pipelines:** None remain. `aura check` uses `compile`, and
`requires-main` is decided once in `Checker::check_mode`. The REPL is the only
surface with its own state (persistent interpreter + `globals`), and its checker
path (`Checker::with_globals`) is a documented, distinct adapter.

**Test helpers that bypass normal execution:** `tests/checker.rs`,
`tests/adversarial.rs`, and `tests/regressions.rs` call `parse` +
`Checker::module[_with_main]` directly (checker-only). `tests/parser.rs` and
`tests/lexer.rs` call `parse` / `lex` directly. `tests/repl.rs` drives
`repl::run_with` with in-memory buffers. These are intentional layer tests.

---

## 3. Sources of Truth

| Source | What it defines | Authority strength | Problems |
|---|---|---|---|
| `docs/contract.md` | Normative language contract; 10 sections | **Strong** — self-declared normative, tested by `tests/contract.rs` | Struct construction, no-paren method call, empty map, parameter/payload annotation enforcement unspecified; float `%` by zero and nesting limit contradicted (§27) |
| `docs/grammar.md` | Normative syntax (EBNF) | **Strong** — self-declared source of truth for syntax | Size-1 map requirement means `{}` is never a map (unstated consequence); `use ... as` implied by `As` token but not in grammar |
| `docs/errors.md` | Diagnostic codes and triggers | **Strong** | Accurate; `tests/grammar.rs` asserts every code reachable |
| `src/parse/mod.rs` | Executable syntax + parser limits | **Executable truth** | `MAX_DEPTH = 128` is stricter than the documented 256 and emits E1006 |
| `src/check/mod.rs` | Executable static rules | **Executable truth** | Conservative; omits several of §2's promises |
| `src/run/mod.rs`, `src/run/value.rs` | Executable runtime semantics | **Executable truth** | Authoritative for equality/ordering/numeric/control flow |
| `src/stdlib/signatures.rs` | Shared builtin/method registry | **Strong** (post-F-02) | Runtime method aliases `up`/`down` are absent from it (dead code) |
| `src/stdlib/mod.rs` | Native implementations | **Executable truth** | `json_encode`/`json_decode` skip the shared `arity()` helper |
| `README.md` | User-facing overview | **Medium** | Consistent with contract on spot-checks |
| `docs/ARCHITECTURE_REVIEW.md` | Point-in-time review (baseline `657c5c2`) | **Historical, not normative** | Superseded by `docs/CORRECTIONS.md` |
| `docs/CORRECTIONS.md` | Resolution report for the review | **Medium** | Verified accurate on spot-checks |
| `examples/*.aura` | Executable documentation | **Medium** | `tests/examples.rs` runs them all |
| `tests/*.rs` | Executable expectations | **Medium** | Coverage gaps enumerated in §25 |
| `src/error.rs` | Code constants | **Strong** | All codes present |

---

## 4. Complete Language Inventory

Legend: **Static** = checker behavior, **Run** = evaluator behavior.

### 4.1 Lexical layer

| Construct | Syntax | Notes / status |
|---|---|---|
| Identifier | `[A-Za-z_][A-Za-z0-9_]*` | ASCII only (`at_ident_start`); non-ASCII letters are not identifier chars |
| Keywords | 29 words (`lex::KEYWORDS`) incl. `true/false/none` | Using one as a name is `E1009` (E1006 in some decl positions) |
| Int literal | decimal; `0x`/`0b`/`0o`; `_` separators | `9223372036854775808` lexes to `IntMinMagnitude`, valid only under unary `-` |
| Float literal | `1.5`, `1e9`, `1e-3` | `1e309` parses to `inf` (not rejected); no NaN literal |
| String | `"…"` or `'…'`, escapes `\n \t \r \0 \\ \" \' \{ \}` | Unknown escape `E1003`; lone trailing `\` `E1004`; terminated by newline |
| f-string | `f"…{expr}…"`, `{{`/`}}` escapes | Inner text re-parsed via `parse_expr`; empty `{}` is `E1006` |
| Comment | `#` to end of line | No block comments |
| Statement separator | newline or `;` | Blank lines skipped |
| Invalid char | `&`, `!` (except `!=`), other | `E1001` with a targeted message for `&`/`!` |

### 4.2 Expressions

| Syntax | AST node | Static | Run | Control-flow? |
|---|---|---|---|---|
| literal | `Expr::Lit` | `Int/Float/String/Bool/Unknown` | value | no |
| name | `Expr::Name` | must resolve (`E2003`) | env → fn → native | no |
| `f"{…}"` | `Expr::FStr` | checks inner exprs | string | inner `Ctl` propagates |
| `-x`, `not x` | `Expr::Unary` | `Not`→Bool; `Neg`→infer(inner) | checked neg / truthiness | operand `Ctl` propagates |
| `a op b` | `Expr::Binary` | orderability for comparisons; no operand-type check for arithmetic | total dispatch | operands propagate; `and`/`or` short-circuit |
| `f(args)` | `Expr::Call` | resolves name; builtins arity+types; **user-fn arity/types not checked** | call | arguments propagate |
| `r.m(args)` | `Expr::Method` | name known + arity/types for known receiver | method dispatch | receiver/args propagate |
| `r.field` | `Expr::Field` | receiver checked only | struct field, else **zero-arg method call** | receiver propagates |
| `b[i]` | `Expr::Index` | operands checked only | list/string/map/struct-field | operands propagate |
| `[a,b]` | `Expr::List` | elements checked | `Value::List` | elements propagate |
| `{k:v}` | `Expr::Map` | keys/values checked (keys not type-checked) | string keys enforced at run (`E3001`) | entries propagate |
| `Name {…}` / `Name(…)` | `Expr::Construct` | name must be a type/variant | struct or variant | args propagate |
| `(a, b)` | `Expr::Tuple` | elements checked | **lowered to `Value::List`** | elements propagate |
| `(x) -> e`, `x -> e`, `fn x -> e` | `Expr::Lambda` | params in child scope, loop_depth reset | `Value::Closure` | captured env by reference |
| `x \|> f` | `Expr::Call`/`Method` (desugared) or `Expr::Pipe` | both sides checked | first-arg insertion, or `call_value` | operand propagates |
| `if c {…} else {…}` | `Expr::If` | cond + branches | branch block value | branches propagate |
| `match v {…}` | `Expr::Match` | patterns validated, no exhaustiveness | first match; `E4029` if none | arms propagate |
| `{ stmts }` | `Expr::Block` | child scope | last value | block value |

### 4.3 Statements

| Statement | AST | Static | Run |
|---|---|---|---|
| `let [mut] n [: T] = e` | `Stmt::Let` | redeclare `E2007`; annotation checked | define in current env |
| `t = e`, `t op= e` | `Stmt::Assign` | immutability; target form; annotated binding type | read-modify-write |
| expression | `Stmt::Expr` | checked | eval, value is block value |
| `return [e]` | `Stmt::Return` | return-type check (in fn) | `Ctl::Return` |
| `throw e` | `Stmt::Throw` | expr checked | `Ctl::Throw` |
| `break` / `continue` | `Stmt::Break`/`Continue` | `loop_depth == 0` → `E2015` | `Ctl::Break`/`Continue` |
| `while c {…}` | `Stmt::While` | cond; body in loop | loop; `break` ends, `continue` reiterates |
| `loop {…}` | `Stmt::Loop` | body in loop | unconditional loop |
| `for p in e {…}` | `Stmt::For` | non-iterable scalars `E4018`; binds pattern | list/string/map/range |
| `try/catch/finally` | `Stmt::Try` | all three blocks in scopes | catches explicit throw only; `finally` precedence |

### 4.4 Items

`fn` (with optional `pub`), `struct`, `enum`, `type` alias, `use`, top-level
`let` (const), and top-level expression. `pub` and `use` are inert (§28).

### 4.5 Values

`int`, `float`, `string`, `bool`, `none`, `list`, `map`, `Instance` (struct),
`Variant` (enum), `Closure`, `Native`, `Range` — see §5.

---

## 5. Type System

| Property | `int` | `float` | `bool` | `string` | `none` | `list` | `map` | struct | enum | fn | range |
|---|---|---|---|---|---|---|---|---|---|---|---|
| Runtime repr | `i64` | `f64` | `bool` | `Rc<str>` | unit | `Rc<RefCell<Vec>>` | `Rc<RefCell<BTreeMap<String,_>>>` | `Rc<Instance>` | `Rc<Variant>` | `Rc<Closure>`/name | `Rc<RangeVal>` |
| Static `Ty` | `Int` | `Float` | `Bool` | `String` | `Unknown` (no `None` variant) | `List(Box<Ty>)` | `Map(Box<Ty>)` | `Named`/`Enum` | `Enum`/`Unknown` | `Unknown` | `Named("range")` |
| Literal | yes | yes | yes | yes | `none` | `[…]` | `{k:v}` (≥1 entry) | `S{…}`/`S(…)` | `V(…)` | lambda | `range()` |
| Equality | numeric cross-eq | semantic | structural | element-wise | with `none` | element-wise | key-wise | ty+fields | tag+payload | **identity** | start/end |
| Ordering | cmp | partial (NaN) | cmp | cmp | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ |
| Truthiness | `≠0` | `≠0.0` | value | `≠""` | false | `≠[]` | `≠{}` | true | true | true | true |
| Iteration | ✗ | ✗ | ✗ | chars | ✗ | values | keys | ✗ | ✗ | ✗ | ints |
| Indexing | ✗ | ✗ | ✗ | `[i]` char | ✗ | `[i]` | `[k]` str | `[k]`/`.k` | ✗ | ✗ | ✗ |
| Methods | ✗ | ✗ | ✗ | 10 | ✗ | 13 | 6 | ✗ | ✗ | ✗ | `len` |
| Python convert | both | both | both | both | both | both | both (str keys) | →repr | →repr | ✗ (E5002 on out) | →repr |
| Assignment | value | value | value | value | value | **shared, mutable** | **shared, mutable** | shared via field | value | value | value |

**Key modeling facts:**
* There is no static `none` type; `Lit::None` infers `Unknown`. This is why
  `for x in none {}` is not statically rejected (§30).
* Lists and maps are **reference values** (`Rc<RefCell<…>>`): passing to a
  function or binding aliases the same storage; mutation is visible through all
  aliases. Documented only implicitly.
* Struct/enum are **not** orderable; `min`/`max`/`sort` treat them as
  unordered and fall back to source order (contract §7).
* A user `type Name = T` alias is transparent; it validates the target but
  does not create a nominal type.
* Type annotations are advisory but only partially enforced (§19, §30).

---

## 6. Operator Matrix

Notation: result type / value; `E3xxx` static or dynamic rejection.

| op | int, int | int, float | float, int | float, float | str, str | list, list | other |
|---|---|---|---|---|---|---|---|
| `+` | int | float | float | float | concat | concat | `E3001` ("expected a number") |
| `-` | int | float | float | float | `E3001` | `E3001` | `E3001` |
| `*` | int | float | float | float | `E3001` | `E3001` | `E3001` |
| `/` | int (÷0 `E4007`) | float | float | float (÷0 `E4007`) | `E3001` | `E3001` | `E3001` |
| `%` | int (÷0 `E4007`) | float | float | **float (`0.0` → `nan`)** | `E3001` | `E3001` | `E3001` |
| `^` | int (neg exp → overflow) | float | float | float | `E3001` | `E3001` | `E3001` |
| `==` `!=` | bool, cross-numeric | | | | structural | structural | structural/identity/false |
| `<` `<=` `>` `>=` | bool | bool | bool | bool (NaN→false) | bool | **static `E3001`** | `E3001`/static |
| `and` `or` | short-circuit truthiness → bool | | | | | | |
| unary `-` | int (overflow-guarded) | float | | | `E3001` | `E3001` | `E3001` |
| `not` | truthiness negate → bool | | | | | | |
| `\|>` | first-arg insertion / callable call | | | | | | `E3001` if not callable |
| `[i]` | list idx (neg ok) | ✗ | ✗ | ✗ | char idx | map str | index/field |
| `.name` | method call (0-arg) | | | | | | field (struct) |
| `op=` | add/sub/mul/div only (`+= -= *= /=`) | | | | | | |

Arithmetic dispatch is **total**: 384/384 combinations produced a value or a
stable diagnostic, never `E4999` (§43). `%` on floats by zero is the one cell
that disagrees with the contract text.

---

## 7. Equality Model

`Value::equals` (`src/run/value.rs:132`), reflected by `==`/`!=`:

* `none == none` → true. `none == anything-else` → false.
* `int`/`float` compare numerically **across types**: `1 == 1.0` → true.
* `bool`, `string`: value equality.
* `list`: same length and element-wise `equals`.
* `map`: same length and key→value `equals`.
* struct `Instance`: same type name, same field order/length, fields `equals`.
* enum `Variant`: same tag, same payload length, payload `equals` (**type name
  not compared** — tags are globally unique, so this is currently safe).
* `Range`: start and end equal.
* `Closure`: **identity** (`Rc::ptr_eq`) — `g == g` true, distinct closures
  false. `Native`: name equality.
* Cross-type otherwise: false (no error).
* `NaN == NaN` → false; `0.0 == -0.0` → true.
* **Cycles:** list/map cycle equality recurses without a visited set; a cyclic
  structure would recurse. There is no way to construct a cycle in the language
  today except via aliasing + self-reference through a mutable container, which
  the evaluator cannot currently create (`push` can push a list into itself —
  see §31 for the risk). This is an `UNSPECIFIED` edge.

Equality is reflexive and symmetric for all constructs observed. Transitivity
holds for the numeric cross-type rule.

---

## 8. Ordering Model

Two independent predicates now agree by construction:

* `Value::comparable_with` (`value.rs:181`) — runtime.
* `Ty::orderable_with` (`src/types.rs`) — checker.

Both accept only: int/int, float/float, int/float, float/int, str/str, bool/bool.

* Checker: if both operand types are known and not orderable → static `E3001`.
* Runtime: if `cmp_val` is `None` **and** `comparable_with` is false → `E3001`;
  if `comparable_with` is true but unordered (NaN) → comparison is `false`.
* `min`/`max`/`sort` do **not** error on incomparable values; they fall back to
  `Ordering::Equal` (stable source order), per contract §7.

Ordering is **not** derived from equality: values can be equal but unorderable
(structs, enums, lists) and int/float are both equal and orderable.

---

## 9. Numeric Model

* `int` is `i64`, fully checked: `+ - * / % ^` and unary `-` return `E4013` on
  overflow (`checked_*`). `i64::MIN / -1` and `i64::MIN % -1` are `E4013`.
* Literal `-9223372036854775808` is valid; bare `9223372036854775808` is
  `E1002`. `i64::MAX` is valid.
* `float` is `f64`. Mixed int/float promotes to float.
* Division (`/`) by zero errors for **both** int and float (`E4007`).
  **Modulo (`%`) by zero errors for int (`E4007`) but yields `nan` for float.**
  This is the only numeric/contract contradiction found.
* Integer `^` with a negative exponent is `E4013`; float `^` uses `powf`.
* `inf` is reachable (`1e308 * 10.0`, literal `1e309`); `nan` is reachable
  (`inf - inf`, `0.0 % 0.0`, `to_float("nan")`). There is no literal for either.
* `to_int`: int, bool, trimmed numeric string, finite in-range float; otherwise
  `E4013` (float) or `E3001` (string parse). Never saturates.
* `to_float`: int, float, bool, trimmed numeric string; else `E3001`.
* Formatting (`format_float`): `nan`/`inf`/`-inf`; integral with `|x| < 1e16`
  prints one decimal (`1.0`); larger magnitudes print in full digits; `-0.0`
  prints `-0.0`.
* No hashing; maps are `BTreeMap` ordered by string key.

---

## 10. Evaluation Order

Determined from `eval_inner` and helpers. **Stable and, where observable,
left-to-right.**

* Binary: left then right (`and`/`or` short-circuit appropriately).
* Call: arguments left-to-right **before** the callee is resolved
  (`eval_call` evaluates args first). For a name callee this is unobservable;
  for an expression callee, note the ordering.
* Method: receiver, then arguments left-to-right.
* Index: base, then index.
* List/Map/Tuple/Construct: elements/entries in source order, left-to-right.
* f-string: parts in order.
* Pipeline: left operand, then right, then call.
* `if`/`match`: condition/subject, then the selected branch only.
* `let`: initializer evaluated before the binding is defined.

This ordering is `IMPLEMENTED BUT UNDOCUMENTED`; the contract does not state it.
No probe found an order-dependent surprise within a single expression.

---

## 11. Control Flow

Non-local signals are a first-class `Ctl` value threaded through
`eval`/`exec_stmt`/`exec_block`, so they propagate out of expression position
(e.g. `if c { break }`).

| Construct | `return` escapes? | `break`/`continue` escape? | `throw` escapes? | `finally` runs? |
|---|---|---|---|---|
| `{}` block | yes | yes | yes | n/a |
| `if` branch | yes | yes | yes | n/a |
| `match` arm | yes | yes | yes | n/a |
| `while`/`loop`/`for` | yes | **consumed** (`break` ends loop, `continue` reiterates) | yes | n/a |
| function call | no (becomes return value) | `E2015` at runtime | crosses boundary as internal `E4099`, value kept in `pending_throw` | n/a |
| lambda | yes (returns from lambda) | rejected statically (`loop_depth` reset) | yes | n/a |
| `try` | yes | yes | yes (caught if explicit) | yes |
| `catch` | yes | yes | yes | yes |
| `finally` | **replaces pending** | **replaces pending** | **replaces pending** | — |

`return`/`break`/`continue` at top level are parse errors (`E1006`), because
only statements inside blocks/items are allowed there.

### 12. `finally` Precedence (explicit)

Confirmed by execution:

* `try { return 1 } catch e -> {} finally { return 2 }` → `2`.
* `try { try { throw "a" } catch e -> { print("caught") } finally { throw "b" } } catch e -> { print(e) }`
  → prints `caught` then `b`.
* `try { continue } catch e -> {} finally { break }` inside `loop` → breaks.
* `try { break } catch e -> {} finally { print("fin") }` → prints `fin`.

Rule (`run/mod.rs:483`): if the `finally` block returns a non-`Val` `Ctl`, it
**replaces** the pending outcome; a `Ctl::Val` leaves it untouched. A Rust-level
error inside `finally` (e.g. division by zero) propagates via `?` and is not
catchable. Documented in contract §7. **Status: FROZEN.** This is a defensible
rule, now documented.

---

## 12. Functions and Closures

* Declaration `fn name(params) [-> T] { body }`. `pub` inert.
* Parameters: names, optional annotations; duplicates are `E1006`.
* `_`-prefixed parameters must be unused (`E2009`).
* Return type: annotated or inferred as `none`-ish; a call to a function with a
  declared return type infers that type (F-04).
* Missing `return` → block value; `fn f() { 1 }` returns `1`; `fn f() {}`
  returns `none`.
* Recursion and mutual recursion work (functions hoisted).
* **Closures** capture their defining `Env` by reference (`Rc<RefCell>`), so a
  closure mutating a captured `let mut` is visible to the outer scope (verified:
  prints `2` then `2`). This is `IMPLEMENTED BUT UNDOCUMENTED`.
* Function values: `Value::Closure`/`Native`; `Expr::Name` of a function or
  native yields a callable value; display is `<fn>`.
* Equality: identity only (§7).
* **No static arity or argument-type checking for user functions** (§30).

### Return-type inference boundaries

`infer(Call)` returns the declared `Ty` when the callee is a top-level name; it
does not model lambdas, higher-order returns, or branch joins
(`if`/`match`/`block` → `Unknown`). Field access and indexing are always
`Unknown`. This is a deliberate conservative model, but the consequence
(unenforced parameter annotations) is a gap, not just conservatism.

---

## 13. Mutability and Scope

* `let` binds immutably; `let mut` allows reassignment. Enforced statically
  (`E2001`) and at runtime (`Env::assign`).
* Top-level `let` is a constant; `let mut` there is `E1006`.
* Mutation is **binding-level** for scalars; **value-level and shared** for
  lists/maps/structs (through `Rc<RefCell>`). `x[0] = …`, `x.k = …`, `push`,
  `pop`, map `insert`/`remove` mutate in place and are visible through aliases.
* Parameters are immutable bindings (`declare(..., false)`).
* Annotated bindings enforce their type on plain reassignment; compound
  assignment (`+=` etc.) is **not** type-checked (checker skips by design).
* Scope is lexical and block-based. Shadowing in a nested block is allowed;
  redeclaration in the same scope is `E2007`. Loop/catch/match/pattern bindings
  are scoped to their construct and do not leak.
* Constants are initialized in source order; a constant may not read a later
  constant (`E2003`), but functions may read constants declared later because
  functions run after initialization.

---

## 14. Structs and Enums

**Structs.** `struct S { f: T, … }`. Fields in declaration order. Construction
`S { f: v }` or positional `S(v)`. Field access `.f` and `[index-string]`.
Field mutation via assignment. Equality is by type + ordered fields. Not
orderable. Display `S { f: v }`.

* **Checker:** validates the struct name and field *names* for assignment
  targets (`s.f = …`), but **does not validate construction**: field types,
  field count, or unknown/missing fields (except via the name lookup). At runtime
  the named-args path requires exactly the declared fields (missing → `E3001`)
  and **silently drops extra fields**; a wrong-typed field value is accepted.

**Enums.** `enum E { V(T), … }`. Variant tags must be globally unique
(`E2013`). Construction `V(args)`. Matching by tag and payload. Equality by
tag + payload (type name not compared). Not orderable.

* **Checker:** validates variant names; **does not validate payload count or
  payload types**. Named constructor args (`V(x: 1)`) parse but the runtime
  collects them separately and reports an arity error.

**Status:** struct/enum declaration, pattern matching, and equality are
coherent and tested. Construction validation is a substantial static gap.

---

## 15. Match

* `match subject { pattern [if guard] -> (block | expr terminator) … }`.
* Patterns: int, string, bool, `none`, lowercase binding / `_`, `[p, …]`,
  `Variant(p, …)`.
* Guards are optional expressions evaluated in the pattern's scope.
* First matching arm wins; no exhaustiveness requirement; no match → `E4029`.
* Duplicate binding in one pattern → `E2014`; unknown variant → `E3002`.
* Arm bodies are statements; a bare control-flow keyword needs a block
  (`1 -> { break }` works, `1 -> break` is a parse error) — **documented
  restriction (F-12), deliberate**.
* Patterns bind in a child scope; matched value is not re-evaluated.
* Arms may `return`/`break`/`throw`; all propagate.
* No unreachable-arm analysis, no arm-type compatibility check.
* The contract says "every arm must yield when used as a value" but there is no
  static check that all arms produce a value (`infer(Match) = Unknown`).

---

## 16. Loops and Ranges

* `while`, `loop`, `for pattern in expr`.
* `for` iterates lists (values), strings (Unicode chars), maps (**keys**), and
  `range`.
* `range(a, b)` is start-inclusive, end-exclusive, step always +1. `range(n)`
  = `range(0, n)`. Empty/negative-length ranges yield nothing. `len` uses
  saturating subtraction.
* Ranges iterate **lazily** in `for` (a huge range with an immediate `break`
  terminates); materialization elsewhere (e.g. `iterate`) is capped at
  10,000,000 elements (`E4013`).
* Mutation of a list during `for` does not affect the iteration (snapshot).
* `break` ends the loop returning `none`; `continue` reiterates; `return`/
  `throw` propagate; `finally` around loop control behaves per §12.
* No `for … else`, no step, no inclusive range.

---

## 17. Pipeline

`x |> f` → `f(x)`; `x |> f(a, b)` → `f(x, a, b)`; `x |> r.m(a)` → `r.m(x, a)`;
any other RHS `c` → `c(x)`. Applied **at parse time** (`desugar_pipe`); the only
surviving `Expr::Pipe` is a non-call RHS (e.g. `5 |> 3`), which the evaluator
handles by `call_value`.

* Left-associative; lower precedence than every binary operator.
* `1 + 2 |> to_string` = `(1+2) |> to_string` = `"3"`.
* Grammar and contract agree (F-17 fixed); a locking test exists.

---

## 18. Builtins and Methods

`src/stdlib/signatures.rs` is the shared registry (name, arity, parameter type
classes, return type) consumed by the checker (`check_builtin_call`,
`check_method_call`) and the runtime (`arity`, `method_exists_anywhere`).

**Remaining duplication / registry drift found:**

1. Runtime method aliases `"up"`/`"down"` exist in `string_method`
   (`stdlib/mod.rs:473-474`) but **not** in the registry. The checker rejects
   `s.up()` as `E2003`, so these aliases are **unreachable dead code** through
   the normal pipeline. (Architecture debt + violates "one spelling".)
2. `json_encode`/`json_decode` natives (`ext.rs:78`) do **not** call the shared
   `arity()`; they use `args.first()`. Arity is enforced for them only by the
   checker and by `arity` nowhere at runtime. Through `run_source` this is
   unobservable (checker always runs first), but it is a runtime/registry gap.
3. `arity()` in `stdlib/mod.rs` still carries a hard-coded min/max fallback
   path for names "not yet described" in the registry; no builtin currently
   takes that path, but it is a second source of arity truth.
4. The registry's `Accepts` models parameter *classes*, not the ad-hoc
   per-native `match` checks; a native can accept something the registry's
   class allows but the native rejects (and vice-versa). The checker and native
   are aligned today only because both were authored together.

Every builtin/method was reviewed; signatures match the natives on
name/arity for all reachable paths.

---

## 19. Unknown / Dynamic Typing

`Ty::Unknown` is the checker's "cannot decide" value. It is compatible with
everything and disables checks. Sources of `Unknown`:

* `none` literal (no `Ty::None`),
* `if`/`match`/`block` expressions,
* field access, indexing, tuple,
* lambdas,
* calls whose callee is not a top-level name,
* dynamic returns (`Returns::Dynamic`),
* unannotated bindings whose initializer is `Unknown`.

**Consequences:** for a receiver/operand typed `Unknown`, method existence,
builtin type checks, ordering, and annotation assignment checks are all skipped.
Errors become runtime-only (`E2003`/`E3001`). This is a deliberate conservative
choice (no false positives) but it means the boundary between "statically
guaranteed" and "runtime-checked" is: *only literals, annotated names, builtin
returns, and user-function declared returns are statically known.* This boundary
is documented in `types.rs` prose but not in `contract.md`.

---

## 20. Error Model

| Class | Codes | Phase | Span? | Catchable? | Documented |
|---|---|---|---|---|---|
| Lexical | E1001, E1002, E1003, E1004, E1009 | lex | yes | n/a | yes |
| Parser | E1006, E1014, E1015 | parse | yes | n/a | yes |
| Checker | E2001, E2003, E2005, E2007, E2009, E2010, E2011, E2012, E2013, E2014, E2015, E3001, E3002, E3005 | check | yes | n/a | yes |
| Runtime | E4007, E4011, E4013, E4018, E4019, E4026, E4027, E4028, E4029, E4030 | run | yes (mostly) | **no** | yes |
| Internal signal | E4099 (`THROWN`) | run | — | consumed by `try` | not user-facing |
| Internal | E4999 | run | yes | no | "bug in Aura" |
| Python | E5001, E5002 | run | yes | no | yes |
| Feature | E5003 | check/run | yes | no | yes |

**`E4999` analysis:** only two sites produce it — the `And`/`Or` arm of
`binary()` and the fall-through of `numeric()`. Both are unreachable for
well-formed input because `and`/`or` are intercepted in `eval_inner` and
`numeric` is total over `BinOp`. Exhaustive probing found **zero** reachable
`E4999`. Invariant D holds for the arithmetic path.

**`E4099` leakage:** a `throw` inside a called function becomes an internal
`E4099` diagnostic carrying a `pending_throw` value; `try` consumes it. If such
a diagnostic reaches the top (`main`), `run()` converts it to `E4026` with the
pending value's display. So `E4099` never reaches the user as-is. Confirmed.

---

## 21. Resource / Host Safety

Intentional limits (all preserved; none removed):

| Limit | Value | Enforced where | Code |
|---|---|---|---|
| Parser recursion depth | 128 | `Parser::enter` | E1006 "nesting too deep" |
| AST nesting (post-parse) | 256 | `enforce_depth`, checker/evaluator depth counters | E1015 |
| Flat infix chain length | 256 | `expr_nodes` in `expr_bp`/`postfix` | E1015 |
| Call frames | 512 | `Interp::depth` | E4011 |
| Range materialization | 10,000,000 | `iterate` | E4013 |
| Interpreter stack | 64 MB | `on_interp_thread` | `E4xxx`/E4999 on thread abort |
| Checker statement walk | 10,000,000 guard | `check_stmt_depth` | E1015 |

No `panic!`, `unwrap()`, `expect()`, `unreachable!`, `todo!`, `assert!`, or
`debug_assert!` exists in production code. Direct indexing is either
`scopes[0]` (always exists), `params[0]` (guarded), `args[0..1]` in `range`
(guarded by `match args.len()`), `payload[0]` in `json` (guarded by
`len() == 1`), or token indexing clamped by `.min(len-1)` with an `Eof`
sentinel. All `from_utf8` uses are `unwrap_or_default`. Miri runs unset.

**Observation:** the *documented* nesting limit (256, E1015) is not the
effective limit for parenthesized expressions — the parser's 128 depth guard
fires at ~61 parentheses as E1006 (§30).

---

## 22. Python Boundary

**Aura → Python** (`to_py`): `none`, `bool`, `int`, `float`, `string`, `list`,
`map` (string keys). Struct/enum/closure/native/range → `E5002`
("cannot cross into Python"). No recursion-depth guard on nested containers,
but the same 256 AST nesting limit bounds source-level nesting.

**Python → Aura** (`to_value`):
* `None` → `none`.
* `bool` checked before `int` (Python `bool` subclasses `int`).
* `int`: if it is a Python `int` and fits `i64` → `Int`; otherwise `E4013`.
* `float` → `Float` (including `nan`/`inf`).
* `str` → `Str`.
* `list` → `List` (recursively).
* `dict`: keys must be Python `str`, else `E5002`; values recursively.
* Anything else → its `repr()` **string** (documented as opaque).

Both integrity fixes from the prior phase are present and verified. No remaining
silent-loss conversion found: big ints and non-string keys are rejected; tuples,
sets, and customs become explicit repr strings (documented). `py_import` reports
public names as `"<python>"` placeholders (documented shape, not a value copy).

---

## 23. CLI / REPL / API Consistency

* `aura run` / `aura eval` / `aura check` / library entry points all funnel
  through `compile` + `execute`; the only difference is `CompileMode::Program`
  vs `Module` (the `main` requirement), decided once in `check_mode`.
* `aura check` and `aura run` agree on all checker diagnostics (same checker).
* **REPL divergence — confirmed defect:** structs, enums, and type aliases
  declared in one REPL submission are **not** visible to the checker in later
  submissions. `globals: Vec<(String,bool,bool)>` only records `let` bindings
  and functions (`repl.rs:141-150`), so `Checker::with_globals` never learns a
  struct/enum/alias. The interpreter *does* retain them (`declare_item`), but
  the checker rejects the next line first. Verified:
  `struct P { x: int }` then `P { x: 1 }` → `E3002`;
  `enum E { A }` then `A` → `E2003`. This is a real REPL/CLI inconsistency.
* REPL reprompts/retains state after an error (tested).
* `aura eval` rejects top-level `let mut` exactly like `aura run` (module
  constants), so `eval` is not a more permissive surface.

---

## 24. Grammar ↔ AST ↔ Checker ↔ Runtime Map

| Grammar | AST | Checker | Runtime | Status |
|---|---|---|---|---|
| `file/item` | `Module`/`Item` | hoist + item | declare + run | ✅ |
| `fn_decl` | `Item::Fn` | body/return | closure | ✅ |
| `params` | `Param` | names + annotations recorded | params bound | ✅ (call-site types unchecked) |
| `struct_decl` | `Item::Struct` | dup type; field types parsed | fields map | ⚠ construction unchecked |
| `enum_decl` | `Item::Enum` | tag uniqueness; payload types parsed | variant map | ⚠ construction unchecked |
| `type_alias` | `Item::Alias` | target validated | no-op | ✅ |
| `use_decl` | `Item::Use` | inert | inert | ✅ (frozen) |
| `const_decl` | `Item::Const` | annotation vs value | source-order define | ✅ |
| `expr_stmt` | `Item::Expr` | checked | eval | ✅ |
| `let_stmt` | `Stmt::Let` | redeclare/annotation | define | ✅ |
| `assign_or_expr` | `Stmt::Assign`/`Expr` | target checks | read-modify-write | ✅ |
| `return/throw/break/continue` | `Stmt::*` | position/loop checks | `Ctl` | ✅ |
| `while/loop/for` | `Stmt::*` | loop_depth, iterable | loops | ✅ |
| `try_stmt` | `Stmt::Try` | scopes | catch/finally | ✅ |
| `expr` (all) | `Expr` | see §4.2 | see §4.2 | ✅/⚠ noted |
| `pattern` | `Pattern` | variant/bind validation | match/bind | ✅ |
| `map` literal | `Expr::Map` | keys/values checked | string keys enforced | ✅ (no empty map) |
| `lambda` | `Expr::Lambda` | scope, loop reset | closure | ✅ |
| `f-string` | `Expr::FStr` | inner checked | display concat | ✅ |

**No dead AST variants.** Every `Expr`/`Stmt`/`Item` variant is constructed by
the parser. `Expr::Pipe` is constructed only for a non-call RHS (rare but real:
`5 |> 3`). **Dead token:** `Tok::As` is lexed but no production consumes it;
`use a as b` fails with E1006.

---

## 25. Test Coverage Matrix

152 `#[test]` functions. Coverage by semantic dimension:

| Dimension | Coverage | Notes |
|---|---|---|
| Lexing | `TESTED` | ints/floats/strings/escapes/f-strings/keywords/comments/operators |
| Parsing | `TESTED` | precedence, pow assoc, if/else, else-if rejection, match, lambda, pipe, lists/maps/index, method/field, nesting bound |
| Arithmetic | `TESTED` | an operator/type matrix in `regressions.rs`; boundaries |
| Equality | `TESTED` | structural + function identity |
| Ordering | `PARTIALLY TESTED` | incomparable list static; NaN; no ordering table for all pairs |
| Numeric boundaries | `TESTED` | i64 min/max, div/rem, float div-by-zero, to_int range |
| **Float `% 0.0`** | **NOT TESTED** | the `nan` result is untested |
| Control flow | `TESTED` | return through match/if; throw across calls; finally precedence |
| Functions/closures | `PARTIALLY TESTED` | closure mutation observed in manual probe, not in tests |
| Mutability/scope | `TESTED` | immutability, shadowing, scope leaks |
| Structs | `PARTIALLY TESTED` | construction validation gaps untested; field assign type tested |
| Enums | `PARTIALLY TESTED` | payload construction type/arity untested |
| Match | `PARTIALLY TESTED` | guards, non-exhaustive, dup binding; arm-type compat untested |
| Loops/ranges | `TESTED` | lazy break, non-iterable, index boundaries |
| Pipeline | `TESTED` | first-arg insertion, chaining, method form |
| Builtins | `TESTED` | arity, types, to_int/to_float |
| Methods | `PARTIALLY TESTED` | existence/arity via regressions; `up`/`down` untested (dead) |
| Errors | `TESTED` | every documented code reachable (`grammar.rs`) |
| Python | `TESTED` (feature `py`) | eval/round-trip/errors/big-int/bad keys |
| CLI/API | `PARTIALLY TESTED` | `run_program` no-main; no CLI-level tests |
| REPL | `PARTIALLY TESTED` | persistence of let/fn; **struct/enum persistence untested** |
| Resource limits | `TESTED` | deep nesting, recursion, huge range |
| Differential | `TESTED (bounded)` | see §26 |

Checker-only tests: `checker.rs`, most of `adversarial.rs`/`regressions.rs`.
Runtime-only: `boundaries.rs`, `run.rs`. End-to-end: `contract.rs`,
`grammar.rs`, `examples.rs`, `run.rs`. Differential: `property.rs`. Property:
`property.rs`.

---

## 26. Differential Testing Assessment

The F-15 test (`property.rs:checker_and_evaluator_agree`) generates
**well-formed expressions only**, of the forms: int/float/string/bool/none
leaves, `+`, `-`, `<`, `==`, and `if/else`. For 2000 cases it asserts:

* checker accepts ⇒ runtime does **not** fail with `E1xxx` or `E4999`;
* checker rejects ⇒ the rejection is `TYPE_MISMATCH`.

**What it proves:** the arithmetic/comparison/if expression subset has no
checker-accepts-then-front-end-fails contradiction.

**What it cannot detect:**
* missing static checks (checker accepts, runtime fails with *any* `E2xxx`/
  `E3xxx`/`E4xxx`) — it only excludes `E1xxx`/`E4999`;
* checker false-positives (rejects something runnable) beyond the TY_MISMATCH
  code;
* operators `* / % ^ and or`, unary, method/field/index, calls, structs/enums,
  `for`/`while`/`loop`/`match`/`try`, `return`/`break`/`throw`, annotations;
* type-*value* agreement (checker's predicted type vs runtime type).

The highest-value missing dimensions are: (a) statement/declaration contexts
with annotations; (b) call arity/type checking; (c) method/field/index; (d)
control-flow propagation. The new `regressions.rs` covers some by example but is
not generative. The generator is intentionally bounded and should **not** simply
be increased in case count; it should be widened in grammar coverage.

---

## 27. Documentation Drift

| Rule | Doc location | Implementation | Classification |
|---|---|---|---|
| "Expressions nest at most 256 levels; deeper is E1015" | contract §7; grammar note | parenthesized nesting fails at **61 levels** with **E1006**; flat infix chains do reach E1015 at 257 | **CONTRADICTED** |
| "Division by zero is E4007 (`/`, `%`, and float division)" | contract §7 | `1.0 % 0.0` → `nan`; `1 / 0.0`/`1 % 0` → E4007 | **CONTRADICTED** |
| "Types … when present they are checked before execution" | contract §2 | parameter/struct-field/enum-payload annotations are not checked at construction/call | **PARTIALLY DOCUMENTED** |
| Struct construction rules | (absent) | named args require all fields (missing `E3001`); extra fields silently dropped; wrong type accepted | **UNSPECIFIED IN IMPLEMENTATION/DOC** |
| Empty map literal | grammar `map` requires ≥1 entry | `{}` is always a block; no empty map is expressible | **UNSPECIFIED** |
| `a.b` without parens | contract §4 lists `a.b` as postfix | auto-calls the zero-arg method (not a value) | **PARTIALLY DOCUMENTED** |
| Closure capture/mutation by reference | (absent) | captured `let mut` mutations escape the closure | **UNSPECIFIED** |
| Evaluation order | (absent) | stable left-to-right | **UNSPECIFIED** (impl-but-undocumented) |
| Float formatting ≥1e16 | contract §7 "very large magnitudes print in full digits" | matches | CONSISTENT |
| Pipeline, finally, pub/use, function equality | contract §4/§7/§10 | matches | CONSISTENT |
| `E1015` vs parser `MAX_DEPTH=128` note | grammar note says 256 | not the binding limit | **OUTDATED** |

---

## 28. Frozen Decisions

| Decision | Evidence | Implementation | Documentation | Tests | Status |
|---|---|---|---|---|---|
| `try` requires `catch` | `parse/mod.rs:657`; contract §5 grammar | yes | §5 | grammar samples | **FROZEN** |
| `(a, b)` is a list, no tuple value | `run/mod.rs:940`; grammar note | yes | noted | run.rs | **FROZEN** |
| Function equality by identity | `value.rs:172`; contract §6/§7 | yes | §6/§7 | regressions | **FROZEN** |
| `pub`/`use` inert | `parse`/`check`/`run`; contract §10 | yes | §10 | run.rs | **FROZEN** |
| Globally unique enum tags | `check/mod.rs:189`; contract §6 (E2013) | yes | errors.md | adversarial | **FROZEN** |
| Nesting 256 / frames 512 | `MAX_AST_DEPTH`/`MAX_CALL_FRAMES`; contract §7 | yes | §7 | boundaries | **FROZEN** (but effective paren limit differs — see §30) |
| Pipeline first-argument insertion | `desugar_pipe`; contract §4; grammar note | yes | §4 | grammar/adversarial | **FROZEN** |
| `finally` overrides pending control flow | `run/mod.rs:483`; contract §7 | yes | §7 | run.rs | **FROZEN** |
| Conservative `Unknown` inference | `types.rs`; `check/mod.rs` | yes | prose only | property | **FROZEN** |
| Python representability (int must fit i64; str keys) | `bridge/mod.rs`; contract §8 | yes | §8 | python.rs/regressions | **FROZEN** |
| `match` arm needs a block for bare control flow | `parse/mod.rs:1104` | yes | grammar `match_arm` | — | **FROZEN** (implicit) |
| `%` float-by-zero = NaN | `numeric()` | yes | contradicted by §7 | none | **DESIGN DECISION REQUIRED** |

---

## 29. Ambiguities Requiring Human Language Decisions

These cannot be resolved from repository evidence alone. No decision is made
here.

### A. Are parameter/field/payload annotations enforced at construction and call?
* **Current behavior:** `fn f(a: int){}; f("x")` runs; `struct S{a:int}; S{a:"x"}`
  runs and `S{a:1,b:2}` silently drops `b`; `enum E{A(int)}; A("x")` runs.
* **Possible interpretations:** (1) annotations are *contractual* and must be
  enforced at call/construction (contract §2 says "checked"); (2) annotations
  are *documentation only* at those positions and only binding-level.
* **Evidence for (1):** contract §2; principle #6 "Diagnose everything"; the
  checker already *has* `struct_fields`, param count/types, and variant payload
  counts. **Evidence for (2):** the checker records param types and uses them in
  the body, but never wired call-site enforcement; no test asserts it; the
  review called the checker "conservative but sound", not complete.
* **Consequences:** silent wrong-typed struct fields and dropped extra fields
  are data-integrity hazards.
* **Recommended candidates:** (1) enforce at least *count/name* for
  construction and *arity* for user calls now, and types where statically known;
  or (2) document these positions as unenforced.

### B. Is `{}` an empty map or an empty block?
* **Current behavior:** always a block (`none`); no empty map literal exists.
* **Possible interpretations:** (1) keep as-is and document "no empty map
  literal" (use `{"": …}`? there is no way); (2) make `{}` a map in map
  position; (3) add explicit empty-map syntax.
* **Evidence:** grammar `map` requires ≥1 entry; `map_ahead` returns false for
  empty braces. No test covers empty maps.
* **Consequences:** maps cannot be built empty then filled; a type-annotated
  `let m: {string:int} = {}` silently binds `none`.
* **Recommended candidates:** document the limitation (least change) or decide
  an empty-map spelling.

### C. Should `r.method` (no parens) be a value or an immediate call?
* **Current behavior:** immediate zero-arg call (`Expr::Field` → method).
* **Possible interpretations:** (1) keep and document; (2) make it a bound
  method value; (3) require parens and error on bare `r.method`.
* **Evidence:** contract §4 lists `a.b` as postfix without stating auto-call;
  the runtime comment treats non-Instance fields as method calls.
* **Consequences:** surprising for a language with first-class functions;
  `r.method` where a method expects args errors at runtime.
* **Recommended candidates:** document as "zero-argument method call sugar".

### D. Should `%` float-by-zero follow the contract (`E4007`) or IEEE (`nan`)?
* **Current behavior:** `nan`.
* **Possible interpretations:** (1) keep NaN and fix the contract; (2) raise
  `E4007` to match `/`.
* **Evidence:** `/` raises E4007 for floats; `%` does not; contract says "`/`,
  `%`, and float division". `to_float("nan")` and `inf - inf` already yield NaN,
  so NaN is a first-class reachable value.
* **Recommended candidates:** keep NaN (IEEE-consistent, NaN already reachable)
  and correct §7.

### E. Are struct/enum/type declarations persistent across REPL submissions?
* **Current behavior:** the interpreter retains them, the checker does not.
* **Possible interpretations:** (1) it is intended that each line is independent
  (unlikely, since `let`/`fn` persist); (2) it is a bug (most likely).
* **Evidence:** `globals` tracks only let/fn; REPL tests cover only let/fn.
* **Recommended candidates:** track declared types in REPL globals so the
  checker agrees with the interpreter.

---

## 30. Bugs

Only defects that violate an established/self-declared contract.

1. **B1 — Float `%` by zero returns `nan` instead of `E4007`.**
   * Repro: `fn main() { print(1.0 % 0.0) }` → `nan`.
   * Contract: §7 "Division by zero is E4007 (`/`, `%`, and float division)".
   * Site: `run/mod.rs` `numeric()` float arm `Rem => a % b`.
   * Classification: BUG (contract violation).

2. **B2 — Parenthesized nesting limit is ~61, reported as `E1006`, not
   256/`E1015`.**
   * Repro: 61 nested `(` works; 62 fails with
     `E1006: nesting too deep`.
   * Contract §7 / grammar note: 256 levels, `E1015`.
   * Site: `parse/mod.rs` `MAX_DEPTH = 128` consumed twice per paren level.
   * Classification: BUG (documented limit and code are wrong).

3. **B3 — Struct fields are not type-checked at construction; wrong-typed
   values are accepted.**
   * Repro: `struct S { a: int } fn main() { print(S { a: "x" }) }` → prints
     `S { a: "x" }`.
   * Contract §2: annotations are checked; §6: E3001 is a check-time rule.
   * Site: `check/mod.rs` `Expr::Construct` checks only the type name.
   * Classification: BUG (contract violation), assuming (A) interpretation (1).

4. **B4 — Extra struct-literal fields are silently dropped.**
   * Repro: `struct S { a: int } … S { a: 1, b: 2 }` → prints `S { a: 1 }`.
   * Contract: principle #6 "Diagnose everything"; no silent loss.
   * Site: `run/mod.rs` `construct` named-args path ignores `b`; checker accepts.
   * Classification: BUG (silent data loss; contract violation).

5. **B5 — REPL loses struct/enum/type declarations across submissions.**
   * Repro: `struct P { x: int }` then `P { x: 1 }` → `E3002`;
     `enum E { A }` then `A` → `E2003`.
   * Contract/§23: the REPL must behave like the file front end while keeping
     state (`repl.rs` doc comment); it does not.
   * Site: `repl.rs:141-150` `globals` omits types.
   * Classification: BUG.

6. **B6 — Extra/unknown/named-argument variant construction is accepted by the
   checker and fails confusingly at runtime.**
   * Repro: `enum E { A(int) } fn main() { print(match A(x: 1) { A(n) -> n }) }`
     → runtime `E3001: variant A expects 1 value(s)`.
   * Contract: named constructor args are grammatically allowed (`ctor_arg`)
     but meaningless for variants; no diagnostic says so.
   * Site: parser `cons_arg`; `run::construct` variant path uses positional only.
   * Classification: BUG/SPECIFICATION GAP (grammar accepts a construct the
     runtime cannot satisfy).

### Borderline (static gaps, not clear contract violations)

These are "checker accepts, runtime rejects" but the runtime error is a
legitimate dynamic error and the contract does not promise a static rejection.
Listed in §29(A) and as SPECIFICATION GAPS, not BUGs:
* user-function call arity (`f()`, `f(1,2)`),
* user-function call argument types,
* enum payload arity/type,
* calling a non-callable binding,
* `for x in none`,
* non-exhaustive match (documented as runtime `E4029`),
* `Expr::Field` method existence (`"abc".nope`).

---

## 31. Test Gaps

Behavior is defined; tests are missing:

1. Float `% 0.0` → `nan` (B1) — no test.
2. Parenthesized nesting boundary (B2) — `boundaries.rs` tests flat chains, not
   parens.
3. Struct construction: wrong field type, missing field, extra field (B3/B4).
4. Enum payload construction: wrong type, wrong arity, named args (B6).
5. Function call arity/type enforcement (currently unchecked; a test would
   pin whichever decision §29(A) makes).
6. REPL struct/enum/type persistence (B5).
7. Closure capture-by-reference mutation visible outside.
8. Method existence on an `Unknown` receiver (`"abc".nope`, `x.nope()`).
9. Ordering table across all type pairs (only list + NaN are tested).
10. Empty-map expression behavior (`{}`).
11. `a.b` no-paren zero-arg method auto-call.
12. `range` materialization cap (10M) boundary.
13. Runtime/registry arity gap for `json_encode`/`json_decode`.

---

## 32. Architecture Debt

1. **Dead runtime aliases** `"up"`/`"down"` in `string_method`, absent from the
   registry, unreachable through the pipeline. Violates "one spelling".
2. **`Tok::As`** is lexed but unused; `use ... as ...` is unsupported.
3. **`ast::Arg.name`** is meaningful for structs but ignored for variants,
   producing the B6 confusion.
4. **`json_*` natives bypass the shared `arity()` helper**, leaving a
   runtime-only arity gap.
5. **`stdlib::arity` retains a second min/max fallback** beside the registry.
6. **`Expr::Tuple` lowers to a list**, so the AST node carries no runtime
   meaning (documented intentional; deferred in the review as F-09).
7. **`TypeClass::Other`** conflates range/struct/enum/closure for method
   lookup; the checker maps all `Named`/`Enum` to "no class", so range methods
   are checked only when the receiver type is statically `Named("range")`.
8. **REPL state tracking** is a parallel declaration store that can drift from
   the interpreter (B5).
9. **Error `E4099`** is a user-invisible internal signal multiplexed through the
   public code namespace.
10. The **differential generator** is narrow (§26) and its one-directional
    assertion cannot detect most checker/runtime gaps.

---

## 33. Future Work

Not implemented here; recorded only.

* **F-09** — lower `Expr::Tuple` to `Expr::List` at parse time.
* **F-11** — allow `try { } finally { }` without `catch` (design decision).
* **F-12** — allow bare control-flow statements as match arm bodies.
* **F-16** — specify `Range` ordering (currently incomparable).
* Static rigor for annotations at call/construction sites (§29A).
* An empty-map literal or explicit documentation of its absence (§29B).
* A REPL declaration store for types (§29E).
* Widen the differential generator to statements, calls, methods, and control
  flow (§26).
* Module/import system, generics, traits, async, VM/IR — all remain explicitly
  out of scope (NO-GO per the architecture review).

---

## 34. Semantic Readiness Assessment

**SEMANTICALLY CONSISTENT BUT REQUIRES DESIGN DECISIONS**

Rationale: execution semantics are coherent and total for well-formed programs
(no internal errors, no false-positive checker, stable left-to-right evaluation,
consistent scoping and control flow). The documentation has two concrete
contradictions (`%` by zero; nesting limit) and several unspecified areas
(struct/enum construction, no-paren method call, empty map, closure capture,
evaluation order). Three of the remaining issues are genuine bugs by the
repository's own contract (B1–B5), and one is a grammar/runtime misalignment
(B6). None requires a redesign; all require either a small fix or an explicit
language decision, which is exactly the scope of a semantic-freeze phase.

---

## Invariant Evaluation

| Invariant | Verdict | Evidence |
|---|---|---|
| **A — Parser completeness** | **PASS** | Every grammar production maps to an AST path; no dead AST variants; only `Tok::As` is unused (no production) |
| **B — Checker/runtime compatibility** | **PASS (qualified)** | No checker-accepted program fails with `E1xxx`/`E4999`; failures are legitimate `E2xxx`/`E3xxx`/`E4xxx`. Static-gap cases exist (§29A) but are dynamic errors, not contradictions |
| **C — Type consistency** | **PASS (limited)** | Where the checker predicts a type (literals, builtin/declared returns, annotated names) it matches runtime; field/index/lambda are `Unknown` by design |
| **D — Error consistency** | **PASS** | Every user error is a stable `E####`; no panic/unwrap in production; zero reachable `E4999` |
| **E — Single semantic authority** | **PASS (mostly)** | One signature registry, one `compile/execute`, one orderability predicate; residual duplication: `up`/`down` aliases, `json_*` arity, `stdlib::arity` fallback |
| **F — Documentation consistency** | **FAIL (partial)** | Two contradictions (`%`/zero, nesting limit) and several unspecified areas (§27) |
| **G — Data integrity** | **PASS (Python) / FAIL (struct)** | Python boundary rejects lossy conversions; struct literals silently drop extra fields (B4) |
| **H — Deterministic evaluation** | **PASS** | Order stable and left-to-right; deterministic across runs (property test) |
| **I — Resource safety** | **PASS** | All limits intact; adversarial nesting/recursion/huge-range terminate with diagnostics; no host crash observed |

---

## Verification Commands Actually Run

```
git rev-parse HEAD                          -> d940fbed54d8afee020e6e7949fe6b2266847422
git status --short                          -> (clean)
cargo build                                 -> Finished
cargo test --all-features                   -> all files ok; 151 executed, 0 failed
cargo build (default features incl. py)     -> Finished
```

Adversarial probes (temporary files under `/tmp/opencode/probes`, deleted):

```
arithmetic sweep   384 combos (8 values x 8 values x 6 ops) -> 0 x E4999
expression sweep   ~45 forms (ops/index/methods/builtins)   -> 0 check-ok/run-fail at E1xxx/E4999
statement sweep    ~19 programs                             -> 10 check-ok/run-fail (static gaps, all legitimate E2xxx/E3xxx/E4xxx)
false-positive sweep ~19 valid programs                     -> 0 check-fail/run-ok
E4999 targeted sweep ~25 programs                           -> 0 x E4999
nesting boundary   61 parens ok / 62 -> E1006
float mod zero     1.0 % 0.0 -> nan (contract says E4007)
struct construct   S { a: "x" } runs; S { a: 1, b: 2 } drops b
REPL                struct/enum across submissions -> E3002/E2003
```

No test, source, doc, or config file was modified; nothing was committed.
