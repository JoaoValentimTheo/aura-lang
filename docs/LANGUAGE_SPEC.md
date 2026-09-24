# The Aura Language Specification

**Version:** Aura v3 (`3.0.0-alpha.1`)
**Status:** Frozen specification
**Baseline commit:** `aba88668856173337b68cd4fb8e046f0467bf561`

This document is the **normative semantic specification** of Aura. It defines
what Aura programs mean. When this document and any other document or the
implementation disagree, this document is authoritative; where it disagrees
with `docs/contract.md`, this document describes the language and
`docs/contract.md` describes the compatibility contract, and any difference is
a defect to be reconciled.

Conventions used throughout:

* **The key words MUST, MUST NOT, SHALL, SHALL NOT, SHOULD, and MAY are to be
  interpreted as described in RFC 2119.**
* A **normative rule** states a guarantee of the language.
* An *Implementation note* states how the current implementation realizes a
  rule; it is **not** normative and may change.
* A *Non-normative example* illustrates usage.
* *Evidence* cites the repository location that establishes the rule.

Nothing in this document licenses a future implementation to skip a rule; a
rule changes only through the RFC process in `CONTRIBUTING.md`.

---

## 1. Overview

**Aura is a small, dynamically-typed, expression-oriented scripting language
with an optional conservative static checker and a native Rust tree-walking
interpreter.**

The language model:

* **Execution model.** A program is a sequence of top-level *items*
  (functions, structs, enums, type aliases, constants, and expressions). Aura
  is interpreted: source is tokenized, parsed into an AST, checked, and then
  walked by an interpreter. There is no compiler backend, bytecode, or
  optimizer.
* **Static checking role.** A checker runs before execution and rejects a
  program for which it can *prove* a rule violation (undefined names,
  immutable-assignment, duplicate declarations, provable type mismatches, and
  so on). The checker is conservative and sound: it never rejects a program
  merely because it cannot determine a type. Types it cannot determine are
  `Unknown` (§2.3).
* **Runtime role.** The interpreter is the authority on value semantics:
  arithmetic, equality, ordering, indexing, iteration, control flow, and all
  dynamic errors.
* **Annotations.** Type annotations are optional. When present they are
  enforced where the checker can prove a mismatch (§6).
* **Primitive types.** `int` (64-bit signed), `float` (IEEE-754 binary64),
  `bool`, `string` (UTF-8, indexed by Unicode scalar value), and `none`.
* **Compound values.** Lists, maps (string-keyed), struct instances, enum
  variants, first-class functions, and ranges.
* **Functions and closures.** Functions are first-class values. Lambdas close
  over their defining environment.
* **Control flow.** `if`/`else`, `match`, `while`, `loop`, `for`,
  `break`/`continue`, `return`, `throw`, and `try`/`catch`/`finally`.
* **Structural declarations.** `struct` and `enum` declare nominal types;
  `type` declares a transparent alias.
* **Collections.** `[T]` lists and `{string: V}` maps.
* **REPL.** An interactive session with persistence across submissions.
* **CLI.** `aura run`, `aura check`, `aura eval`, `aura repl`, `aura version`.
* **Host boundary.** An optional Python bridge (`py_eval`, `py_import`,
  `py_call`, `py_version`) behind the `py` feature.

This document is normative for semantics, not for presentation. Diagnostic
message wording is not normative; diagnostic *codes* and *phases* are.

---

## 2. Source-to-Execution Model

### 2.1 Pipeline

```
source (UTF-8 text)
  │
  ▼  lex              → tokens
  │
  ▼  parse            → AST (syntax errors, nesting-limit errors)
  │
  ▼  check            → checked AST (name/rule/type errors)
  │
  ▼  execute          → values, output, runtime errors
```

**Normative rule.** A user-visible program is accepted only if it passes
lexing, parsing, and checking. The three front-end phases MUST each produce
stable `E####` diagnostics on failure (see §30).

*Implementation note.* The entry points are `lex::lex` (`src/lex/mod.rs`),
`parse::parse` (`src/parse/mod.rs`), `check::Checker` (`src/check/mod.rs`),
and `run::Interp::run` (`src/run/mod.rs`). They are composed by `compile` and
`execute` in `src/lib.rs`. Parsing and execution each run on a dedicated
large-stack thread; this is an implementation detail with no semantic effect.

### 2.2 Phase responsibilities

| Phase | Accepts | Rejects (code family) |
|---|---|---|
| Lex | tokenizable UTF-8 | invalid character, malformed number, bad escape, unterminated string, number-followed-by-name (`E1xxx`) |
| Parse | grammar-conforming token stream within the nesting limit | expected-token, `else if`, reserved-name, nesting (`E1xxx`) |
| Check | names/declarations/rules/types that can be verified | undefined, redeclaration, immutability, duplicate type/variant/binding, provable type mismatch (`E2xxx`/`E3xxx`) |
| Execute | any checked program | overflow, division by zero, index out of range, no match, uncaught throw, recursion, not iterable, internal (`E4xxx`), Python (`E5xxx`) |

### 2.3 The `Ty::Unknown` boundary

**Normative rule.** The checker MUST NOT reject a program on the basis of a
type it cannot determine. Where the checker's inferred type is `Unknown`, the
type rule is simply not applied; any resulting failure is a runtime error.

**Normative rule.** The checker MAY reject a construct when the inferred type
is definitely incompatible with a required type. "Definitely incompatible"
means both types are known and not compatible under §8.

This boundary is exact: the checker statically knows a type only for
literal values, annotated names, names bound to expressions of statically
known type, builtin/method return types that are declared concrete, and
declared return types of top-level functions. Everything else is `Unknown`.

*Evidence:* `Ty` and `compatible_with` (`src/types.rs`); `Checker::infer`
(`src/check/mod.rs`); `tests/regressions.rs::f04_*`, `b3_*`.

---

## 3. Lexical Specification

### 3.1 Source encoding

**Normative rule.** Source MUST be UTF-8 text. A byte sequence that is not
valid UTF-8 in a position where a character is required is a lexical error.

*Implementation note.* The lexer walks bytes and decodes multi-byte characters
inside strings and identifiers it encounters; invalid sequences fall back to
`U+FFFD` in string bodies, but character-class decisions use ASCII.

### 3.2 Identifiers

**Normative rule.** An identifier begins with an ASCII letter (`A`–`Z`,
`a`–`z`) or `_`, and continues with ASCII letters, digits (`0`–`9`), or `_`.
Identifiers are case-sensitive.

**Normative rule.** Non-ASCII letters are not identifier characters in this
version.

*Evidence:* `Lexer::at_ident_start`, `Lexer::ident` (`src/lex/mod.rs`).

### 3.3 Reserved words

**Normative rule.** The following words are reserved and MUST NOT be used as
binding, parameter, function, field, variant, or type names:

```
let mut fn if else match while loop for in return break continue
use as struct enum type and or not try catch finally throw pub
true false none
```

Using a reserved word where a name is required is a lexical/parse error
(`E1009`, or `E1006` in declaration positions).

*Evidence:* `KEYWORDS` (`src/lex/mod.rs`), `Parser::ident` (`src/parse/mod.rs`).

### 3.4 Whitespace and line structure

**Normative rule.** Space, tab, and carriage return are insignificant
whitespace. A newline is significant: it terminates a statement (see §3.7). A
semicolon MAY be used instead of a newline to terminate a statement. Blank
lines and lines containing only whitespace or a comment are ignored.

*Evidence:* `Lexer::next_token` (`src/lex/mod.rs`), `Parser::end_stmt`.

### 3.5 Comments

**Normative rule.** A comment begins with `#` and extends to the end of the
line. There are no block comments. A `#` inside a string literal is not a
comment.

*Evidence:* `Lexer::next_token` (`src/lex/mod.rs`).

### 3.6 Literals

#### 3.6.1 Integer literals

**Normative rule.** An integer literal MAY be written in:

* decimal: `123`, with `_` as a digit separator (`1_000`);
* hexadecimal: `0xff` (prefix `0x`);
* binary: `0b1011` (prefix `0b`);
* octal: `0o755` (prefix `0o`).

**Normative rule.** An integer literal MUST be representable as a signed
64-bit integer (`i64`). A literal whose value exceeds `i64::MAX` is a lexical
error (`E1002`), with one exception: the decimal magnitude
`9223372036854775808` is tokenized specially and is valid **only** as the
operand of a unary minus, where it denotes `i64::MIN` (§10.2).

**Normative rule.** A number MUST NOT be immediately followed by an
identifier character; `1abc` is an error (`E1002`), never two tokens.

*Evidence:* `Lexer::number`, `Lexer::boundary` (`src/lex/mod.rs`);
`Parser::unary` (`src/parse/mod.rs`).

#### 3.6.2 Float literals

**Normative rule.** A float literal is a decimal magnitude with a fractional
part (`1.5`) and/or an exponent (`1e9`, `1.5e-3`, `2E10`). A `.` is a decimal
point only when followed by a digit; otherwise it is the field/method operator.

**Normative rule.** Float literals are IEEE-754 binary64. A literal that
overflows the format (for example `1e309`) denotes positive infinity; it is
not rejected.

**Normative rule.** There is no literal spelling for `NaN` and none for
infinity (`inf` is produced only by arithmetic; see §10.7).

*Evidence:* `Lexer::number` (`src/lex/mod.rs`); `format_float`
(`src/run/value.rs`).

#### 3.6.3 String literals

**Normative rule.** A string literal is delimited by `"` or `'`; the two
delimiters are interchangeable and have identical meaning. A string literal
MUST NOT contain an unescaped newline; a newline before the closing delimiter
is an unterminated-string error (`E1004`).

**Normative rule.** The following escape sequences are recognized inside a
string literal:

| Escape | Meaning |
|---|---|
| `\n` | line feed |
| `\t` | tab |
| `\r` | carriage return |
| `\0` | NUL |
| `\\` | backslash |
| `\"` | double quote |
| `\'` | single quote |
| `\{` | literal `{` |
| `\}` | literal `}` |

Any other escape is an invalid-escape error (`E1003`). A trailing lone
backslash is an unterminated-string error.

*Evidence:* `Lexer::unescape` (`src/lex/mod.rs`).

#### 3.6.4 F-strings (interpolation)

**Normative rule.** A string literal MAY be prefixed with `f` (or `F`) to make
it an *f-string*. Inside an f-string:

* `{ expr }` interpolates the value of `expr` using its display form;
* `{{` produces a literal `{`;
* `}}` produces a literal `}`.

The expression between braces MUST be a well-formed expression. An empty `{}`
is an error (`E1006`); an unterminated `{` is an error (`E1004`).

*Evidence:* `Lexer::ident` (f-prefix), `Parser::fstring`
(`src/parse/mod.rs`).

#### 3.6.5 Boolean and `none` literals

**Normative rule.** `true` and `false` are the boolean literals. `none` is the
absence value. There is no `null`, `nil`, or `undefined`.

### 3.7 Statement termination

**Normative rule.** A statement is terminated by a newline or a semicolon.
Trailing separators are permitted.

**Normative rule.** A newline is significant and is not skipped in the middle
of a construct. In particular, `else`, `catch`, and `finally` MUST appear on
the same line as the closing `}` of the block they follow; a newline between
them is a parse error (`E1006`).

*Non-normative examples.*

```aura
if c { a } else { b }        # accepted
try { … } catch e -> { … }   # accepted
```

```aura
if c { a }
else { b }        # E1006: a newline before `else` is not allowed
```

*Evidence:* `Parser::atom` (`if`/`match`), `Parser::stmt_inner` (`try`);
`Parser::block` does not skip newlines before a following keyword.

---

## 4. Grammar

This section is normative for **syntax**. It corresponds to the executable
parser in `src/parse/mod.rs`. The authoritative EBNF is also maintained in
`docs/grammar.md`; the two MUST agree.

### 4.1 Program and items

```
file          = { NEWLINE | item } EOF ;
item          = [ "pub" ] ( fn_decl | struct_decl | enum_decl
                          | type_alias | use_decl | const_decl )
              | expr_stmt ;
use_decl      = "use" IDENT { "." IDENT } terminator ;
```

**Normative rule.** `pub` is accepted on any item and is semantically inert
(§27). `use` is parsed and inert (§27).

### 4.2 Declarations

```
fn_decl       = "fn" IDENT "(" [ params ] ")" [ "->" type ] block ;
params        = param { "," param } ;
param         = IDENT [ ":" type ] ;
struct_decl   = "struct" IDENT "{" [ field { "," field } [ "," ] ] "}" ;
field         = IDENT ":" type ;
enum_decl     = "enum" IDENT "{" [ variant { "," variant } [ "," ] ] "}" ;
variant       = IDENT [ "(" [ type { "," type } ] ")" ] ;
type_alias    = "type" IDENT "=" type terminator ;
const_decl    = "let" IDENT [ ":" type ] "=" expr terminator ;
```

**Normative rule.** A top-level `let` declares an immutable module constant.
`let mut` at the top level is rejected (`E1006`).

### 4.3 Types

```
type          = base_type [ "|" "none" ] ;
base_type     = "int" | "float" | "bool" | "string"
              | "[" type "]"
              | "{" type ":" type "}"
              | IDENT ;
```

**Normative rule.** The only union form is `T | none`. Any other `|` union is
a syntax error.

### 4.4 Statements

```
block         = "{" { terminator | stmt } "}" ;
terminator    = NEWLINE | ";" ;
stmt          = let_stmt | assign_or_expr | return_stmt | throw_stmt
              | break_stmt | continue_stmt | while_stmt | loop_stmt
              | for_stmt | try_stmt ;
let_stmt      = "let" [ "mut" ] let_pattern [ ":" type ] "=" expr terminator ;
let_pattern   = IDENT | let_list_pattern | let_variant_pattern ;
let_list_pattern    = "[" [ let_pattern { "," let_pattern } [ "," ] ] "]" ;
let_variant_pattern = IDENT [ "(" [ let_pattern { "," let_pattern } ] ")" ] ;
assign_or_expr= expr [ assign_op expr ] terminator ;
assign_op     = "=" | "+=" | "-=" | "*=" | "/=" ;
return_stmt   = "return" [ expr ] terminator ;
throw_stmt    = "throw" expr terminator ;
break_stmt    = "break" terminator ;
continue_stmt = "continue" terminator ;
while_stmt    = "while" expr block ;
loop_stmt     = "loop" block ;
for_stmt      = "for" pattern "in" expr block ;
try_stmt      = "try" block "catch" IDENT "->" block [ "finally" block ] ;
```

**Normative rule.** `catch` is mandatory after `try`. There is no `try`
without `catch` (§14.5).

**Normative rule.** `if` and `match` are expressions, not statements; they are
reached through `assign_or_expr` (`expr`).

**Normative rule (destructuring `let`).** A `let` statement binds a
`let_pattern`:

* `let IDENT = expr` and `let mut IDENT = expr` are ordinary bindings and are
  exactly as specified in §16.1.
* A `let_list_pattern` or `let_variant_pattern`, including nesting of them, is
  a **destructuring binding**: the initializer is evaluated once and every
  binding name in the pattern is bound from the result (§4.7).
* Only `IDENT`, `let_list_pattern`, and `let_variant_pattern` are legal in
  `let`-pattern position. A `literal_pattern` (`INT`, `STRING`, `"true"`,
  `"false"`, `"none"`) is not a `let_pattern`, so a literal anywhere in a
  `let` pattern — including a nested position — is a syntax error, reported
  as `E1006`. This restriction is specific to `let`; literal patterns remain
  legal in `match` and `for` (§4.6).
* An optional `: type` annotation is legal only when the pattern is a single
  `IDENT`. An annotation on any other pattern is `E1006`.
* `mut` is legal only when the pattern is a single `IDENT`. `mut` on any other
  pattern is `E1006`.

### 4.5 Expressions, precedence, and associativity

Precedence, lowest binding first:

| Level | Operators | Associativity |
|---|---|---|
| 1 | `\|>` pipeline | left |
| 2 | `or` | left |
| 3 | `and` | left |
| 4 | `==` `!=` | left |
| 5 | `<` `<=` `>` `>=` | left |
| 6 | `+` `-` | left |
| 7 | `*` `/` `%` | left |
| 8 | `^` | **right** |
| 9 | unary `-`, unary `not` | prefix (right) |
| 10 | postfix: `f(x)`, `a.b`, `a.b(x)`, `a[i]` | left |
| 11 | atoms: literals, names, groups, lists, maps, lambdas, `if`, `match`, blocks | — |

```
expr            = pipe ;
pipe            = logic_or { "|>" logic_or } ;
logic_or        = logic_and { "or" logic_and } ;
logic_and       = equality { "and" equality } ;
equality        = comparison { ( "==" | "!=" ) comparison } ;
comparison      = additive { ( "<" | "<=" | ">" | ">=" ) additive } ;
additive        = multiplicative { ( "+" | "-" ) multiplicative } ;
multiplicative  = power { ( "*" | "/" | "%" ) power } ;
power           = unary [ "^" power ] ;
unary           = ( "-" | "not" ) unary | postfix ;
postfix         = atom { call_or_member } ;
call_or_member  = "(" [ call_args ] ")"
                | "[" expr "]"
                | "." IDENT [ "(" [ call_args ] ")" ] ;
call_args       = arg { "," arg } ;           (* positional, then named *)
arg             = [ IDENT ":" ] expr ;
atom            = INT | FLOAT | STRING | FSTRING
                | "true" | "false" | "none"
                | IDENT "(" [ ctor_args ] ")"        (* call or variant *)
                | IDENT "{" [ field_init { "," field_init } ] "}"  (* struct *)
                | "(" expr ")"
                | "(" expr "," [ expr { "," expr } ] ")"          (* list sugar *)
                | lambda | list | map | block_expr
                | if_expr | match_expr ;
lambda          = ( IDENT | "(" [ IDENT { "," IDENT } ] ")" ) "->" expr ;
list            = "[" [ expr { "," expr } [ "," ] ] "]" ;
map             = "{" entry { "," entry } [ "," ] "}" ;
entry           = expr ":" expr ;
block_expr      = block ;
ctor_args       = ctor_arg { "," ctor_arg } ;
ctor_arg        = [ IDENT ":" ] expr ;
field_init      = IDENT ":" expr ;
if_expr         = "if" expr block [ "else" expr ] ;
match_expr      = "match" expr "{" { match_arm } "}" ;
match_arm       = pattern [ "if" expr ] "->" ( block | expr terminator ) ;
```

**Normative rule.** `x |> f(a)` desugars, at parse time, to `f(x, a)`;
`x |> r.m(a)` desugars to `r.m(x, a)`; `x |> f` desugars to `f(x)`. See §23.

**Normative rule.** `IDENT "(" ... ")"` is a *call* when the identifier's
first character is lowercase and a *variant construction* when it is uppercase.
`IDENT "{" ... "}"` is a struct literal only when the identifier is uppercase.

**Normative rule.** `(a, b)` with a comma is a *list of the given elements*
(§21); `(expr)` without a comma is a grouping expression and produces no
distinct value.

**Normative rule.** In a call argument list, every positional argument MUST
precede every named argument; a positional argument after a named one is a
syntax error. A named argument is written `name: value` (§15.7).

**Normative rule.** `else if` is not part of the grammar; it is diagnosed as
`E1014` before parsing proceeds.

### 4.6 Patterns

```
pattern         = literal_pattern | bind_pattern | list_pattern
                | variant_pattern ;
literal_pattern = INT | STRING | "true" | "false" | "none" ;
bind_pattern    = IDENT | "_" ;
list_pattern    = "[" [ pattern { "," pattern } [ "," ] ] "]" ;
variant_pattern = IDENT [ "(" [ pattern { "," pattern } ] ")" ] ;
```

**Normative rule.** A capitalized identifier in pattern position is a variant
pattern; a lowercase identifier or `_` is a binding pattern. `_` binds nothing.
Literal patterns (`INT`, `STRING`, `"true"`, `"false"`, `"none"`) are legal in
`match` and `for` pattern position.

### 4.7 Destructuring `let`

**Normative rule (binding).** `let P = e` evaluates `e` exactly once and binds
every name in `P.bindings()` (every `IDENT` in the pattern, except `_`) from
the corresponding component of the resulting value. Every name bound by a
destructuring `let` is immutable. `_` binds nothing.

**Normative rule (scope).** The names bound by a destructuring `let` have the
same scope and lifetime as an ordinary `let` in the same position (§16.3): they
are visible from the statement to the end of the enclosing scope, including
nested scopes, and they shadow outer bindings exactly as an ordinary `let`
does. Declaring a name already declared in the same scope is `E2007`. All
names become visible only after the initializer has been evaluated and the
pattern has fully matched.

**Normative rule (evaluation order).** The initializer expression is evaluated
once, before any binding, under the strict left-to-right evaluation of §13.
Pattern binding then binds the names of `P.bindings()`; because no user code
runs during binding, the order in which the names are bound is not observable.
A `return`, `throw`, `break`, or `continue` raised while evaluating the
initializer propagates before any binding, exactly as for an ordinary `let`.

**Normative rule (failure and atomicity).** Destructuring `let` is atomic. If
the runtime value does not match the pattern, the statement fails and no
binding is created or modified; the environment is left exactly as it was
before the statement. The failure uses the existing runtime diagnostics of
pattern binding: a list arity mismatch, a non-list value for a list pattern, a
variant tag or payload-length mismatch, or a non-variant value for a variant
pattern are each `E3001`.

**Normative rule (checker).** The checker validates a destructuring `let`
pattern with the same pattern validation used by `match` and `for` (§4.6): a
variant pattern MUST name a declared variant (`E3002`), and a pattern MUST NOT
bind the same name twice (`E2014`). Each bound name is declared in the current
scope, so a same-scope redeclaration is `E2007`. The checker performs no type
inference for destructured names: they have no static type and are treated as
`Unknown` (§2.3). No annotation is checked for a destructuring `let`, because
annotations are only legal on the single-`IDENT` form.

**Normative rule (REPL).** Each name bound by a destructuring `let` is
persisted across REPL submissions as an unannotated session binding, exactly
as an ordinary unannotated `let` is (§16.1, §33 item 4). Because destructured names
have no annotation, they persist with no declared type and are `Unknown` in
later submissions. A failed destructuring submission does not persist any of
its bindings and does not alter prior session state.

**Normative rule (compatibility).** Destructuring `let` is an additive
syntax extension. `let IDENT = e`, `let mut IDENT = e`, and `let IDENT: T = e`
retain their existing meaning, AST, checker behavior, runtime behavior, and
REPL persistence. No previously valid program changes meaning.

*Evidence:* `Parser::pattern`, `Parser::stmt_inner` (`Tok::Let`)
(`src/parse/mod.rs`); `check_pattern`, `Stmt::Let` (`src/check/mod.rs`);
`bind_pattern`, `Env::child`/`get`/`define` (`src/run/mod.rs`);
`eval_line` (`src/repl.rs`).

---

## 5. Values and Type System

### 5.1 Runtime value universe

**Normative rule.** The runtime values of Aura are exactly:

| Value kind | Runtime representation | `type_name` |
|---|---|---|
| integer | signed 64-bit | `int` |
| float | IEEE-754 binary64 | `float` |
| string | immutable UTF-8 text | `string` |
| boolean | `true`/`false` | `bool` |
| absence | `none` | `none` |
| list | ordered, mutable, reference | `list` |
| map | string-keyed, ordered by key, mutable, reference | `map` |
| struct instance | named fields in declaration order, reference | `struct` |
| enum variant | tag + positional payload, reference | `enum` |
| function | closure or native | `fn` |
| range | `start`/`end`, step 1 | `range` |

*Evidence:* `Value` (`src/run/value.rs`).

### 5.2 Checker type universe (`Ty`)

**Normative rule.** The checker's type domain is:

```
int, float, bool, string, [T], {string: V}, Named(name), Enum(name), Unknown
```

**Normative rule.** `[T]` is a list type, `{string: V}` a map type with a
string key, `Named(n)` a user struct or alias-resolved type, `Enum(n)` an enum
type, and `Unknown` the "not determined" type.

**Normative rule.** A `map` annotation with a non-`string` key is rejected
(`E3001`); maps are string-keyed in this version.

**Normative rule.** There is no static `none` type; `none` infers `Unknown`.

*Evidence:* `Ty` (`src/types.rs`); `Ty::from_expr`; `Checker::infer`.

### 5.3 Type properties

| Property | int | float | bool | string | list | map | struct | enum | fn | range |
|---|---|---|---|---|---|---|---|---|---|---|
| Equality | numeric | numeric | value | value | structural | structural | nominal + structural | tag + payload | identity | start/end |
| Ordering | yes | yes (NaN unordered) | yes | yes | no | no | no | no | no | no |
| Indexing | no | no | no | `[i]` by character | `[i]` | `[k]` (string) | `["k"]` / `.k` | no | no | no |
| Iterable | no | no | no | yes (characters) | yes (elements) | yes (keys) | no | no | no | yes (ints) |
| Callable | no | no | no | no | no | no | no | no | yes | no |
| Mutable | value | value | value | value | **shared** | **shared** | **shared fields** | value | value | value |

"Shared" means the value is a reference held through `Rc<RefCell<…>>`; passing
or binding it aliases the same storage (§16.6).

---

## 6. Type Annotations

### 6.1 Where annotations are legal

**Normative rule.** A type annotation MAY appear on:

* a binding: `let x: T = e` (local) and top-level constants;
* a function parameter: `fn f(x: T)`;
* a function return type: `fn f() -> T`;
* a struct field: `struct S { f: T }`;
* an enum variant payload position: `enum E { V(T) }`.

**Normative rule.** Annotations are optional everywhere they are legal. A
bindings' annotation, a function return annotation, a struct field annotation,
and an enum payload annotation are all **validated** to name an existing type
(`E3002` otherwise).

### 6.2 When annotations are enforced

**Normative rule.** An annotation is enforced (its stated type is compared
with the value's inferred type) in exactly these positions:

* the initializer of an annotated local `let`;
* the initializer of an annotated top-level constant;
* a `return` expression, against the function's declared return type;
* a struct field value at construction, against the field's declared type;
* an enum payload value at construction, against the payload's declared type;
* a plain reassignment `x = e` to an annotated binding (`+=` etc. are not
  re-checked).

**Normative rule.** A function's parameter annotations are recorded and used
when typing the body, and they are also enforced at call sites that the
checker can resolve to a specific top-level function declaration (§6.5).

### 6.3 Compatibility

**Normative rule.** "`A` is compatible with `B`" (writing the expected type
first) holds when:

* either side is `Unknown`; or
* both are the same primitive; or
* both are `[T]`/`[T']` and `T` is compatible with `T'`; or
* both are maps with compatible value types; or
* both are the same `Named`/`Enum` type.

**Normative rule.** `int` and `float` are **not** compatible in annotations;
there is no implicit numeric coercion in annotations.

*Evidence:* `Ty::compatible_with` (`src/types.rs`).

### 6.4 The `Unknown` boundary in annotations

**Normative rule.** If the value's inferred type is `Unknown`, the annotation
check passes. This preserves the conservative soundness rule of §2.3.

*Non-normative example.* `let x = none; let y: int = x` passes the checker;
`x` is `Unknown`. A later runtime use of `y` may still fail.

### 6.5 Argument checking at directly resolved calls

**Normative rule.** When a call's callee is statically resolved to a specific
top-level `fn` declaration — that is, the callee is a name that resolves to a
declared function and the name is not shadowed by a local binding — the
checker MUST validate, before execution:

* the **argument count**, which MUST equal the number of declared parameters;
* for each parameter that has a type annotation, the corresponding argument's
  inferred type, which MUST be compatible with the annotation under §6.3.

A provable mismatch in either case is `E3001`, diagnosed during checking.

**Normative rule.** A parameter **without** a type annotation imposes no
static type constraint; only the argument count applies to it.

**Normative rule.** An argument whose inferred type is `Unknown` imposes no
static type constraint; the annotation check for that argument is skipped,
preserving §6.4 and §2.3.

**Normative rule.** If the callee cannot be statically resolved to a specific
top-level function declaration — for example a function value, a closure, a
variable holding a callable, or any expression whose type is `Unknown` — the
static argument check does **not** apply. Such calls retain their existing
dynamic behavior, and the runtime remains authoritative for them.

**Normative rule.** The static check applies only after successful lexical
resolution to the actual declaration. A local binding that **shadows** a
declared function's name means the name is a callable value, not a directly
resolved function, so the declared function's signature MUST NOT be applied to
it.

**Normative rule.** Because declarations are hoisted (§26), the check applies
to calls that textually precede the declaration, including mutually recursive
calls.

**Normative rule.** Runtime argument validation MUST remain in place even
though the check now rejects some mismatches earlier.

See §6.5.1 for the compatibility classification of this rule.

#### 6.5.1 Compatibility note

**Compatibility note.** Static argument checking at directly resolved calls is
a **semantic tightening with source-compatibility impact**, not a
language-version redesign.

* Every **previously valid** Aura program — one whose arguments were already
  compatible with the callee's declared parameters, or whose callee was not
  directly resolved — remains valid and behaves identically.
* A **previously checker-accepted but dynamically invalid** program — one that
  passed an argument the annotation did not allow, or called a resolved
  function with the wrong count — may now be rejected during checking with the
  same `E3001` code it would previously have produced (or that it evaded) at
  runtime. Its acceptance changes; its meaning does not.
* Runtime argument validation is retained, so any call the checker does not
  resolve is still checked dynamically.

*Evidence:* `Checker::check` and the call handling in `src/check/mod.rs`;
`tests/regressions.rs` (FEATURE_001 tests added at implementation time).

---

## 7. Type Aliases

**Normative rule.** `type Name = T` declares a **transparent alias**. `Name`
denotes `T` in every type position: binding annotations, parameter
annotations, return annotations, struct fields, enum payloads, and nested
positions inside `[T]`, `{string: V}`, and `T | none`.

**Normative rule.** An alias does not create a distinct nominal type and has
no runtime representation; `type Id = int` and `int` are the same type to the
checker and the interpreter.

**Normative rule.** An alias target MUST name an existing type (`E3002`
otherwise).

**Normative rule.** Aliases MAY be chained (`type A = B; type B = int`); the
checker resolves them transitively.

**Normative rule.** Aliases are visible throughout a module and, in the REPL,
across later submissions (§29).

**Normative rule.** Alias resolution recurses through compound positions
(`[T]`, `{string: V}`, `T | none`). A **recursive alias** — a `type` whose
target refers, directly or through other aliases, to itself — has no concrete
target and MUST be rejected with `E3002`; it MUST NOT recurse without bound or
crash the host.

*Implementation note.* The checker stores alias targets and substitutes them
with `resolve_type_expr`; the interpreter has no alias table.

*Evidence:* `TypeExpr::Named` handling and `resolve_type_expr`
(`src/check/mod.rs`); `tests/regressions.rs` alias cases;
`tests/repl.rs::regression_repl_alias_persistence`.

---

## 8. Type Compatibility Matrix

`E` = expected type, `A` = inferred/actual type. The cell states whether the
checker accepts (`A` = accept, `R` = reject with `E3001`).

| E \ A | int | float | bool | string | list | map | Named | Enum | Unknown |
|---|---|---|---|---|---|---|---|---|---|
| **int** | A | R | R | R | R | R | R | R | A |
| **float** | R | A | R | R | R | R | R | R | A |
| **bool** | R | R | A | R | R | R | R | R | A |
| **string** | R | R | R | A | R | R | R | R | A |
| **[T]** | R | R | R | R | A* | R | R | R | A |
| **{string:V}** | R | R | R | R | R | A* | R | R | A |
| **Named(n)** | R | R | R | R | R | R | A** | R | A |
| **Enum(n)** | R | R | R | R | R | R | R | A** | A |
| **Unknown** | A | A | A | A | A | A | A | A | A |

\* Element/value types must themselves be compatible.
\** Same nominal name only.

**Normative rule.** A `Named` value is compatible only with the same `Named`
name; struct types are nominal.

This matrix is the normative statement behind §6.3 and §13.

---

## 9. Operators

### 9.1 Operator behavior matrix

The table states runtime behavior for each operator and operand kind. `OK`
means the operator produces a value; a `E####` is a runtime diagnostic; `static`
means the checker rejects before execution; `—` means the parser rejects the
construct or it is not applicable.

| Op | int,int | int,float | float,int | float,float | str,str | list,list | other |
|---|---|---|---|---|---|---|---|
| `+` | int (checked) | float | float | float | concat | concat | `E3001` unless numeric |
| `-` | int (checked) | float | float | float | `E3001` | `E3001` | `E3001` |
| `*` | int (checked) | float | float | float | `E3001` | `E3001` | `E3001` |
| `/` | int (÷0 `E4007`) | float | float | float (÷0 `E4007`) | `E3001` | `E3001` | `E3001` |
| `%` | int (÷0 `E4007`) | float | float | float (÷0 `E4007`) | `E3001` | `E3001` | `E3001` |
| `^` | int (neg exp → `E4013`) | float | float | float | `E3001` | `E3001` | `E3001` |
| `==` | bool | bool | bool | bool | bool | structural | structural/identity/false |
| `!=` | bool | bool | bool | bool | bool | `!` of `==` | `!` of `==` |
| `<` `<=` `>` `>=` | bool | bool | bool | bool (NaN→false) | bool | static `E3001` | `E3001` |
| `and` | short-circuit truthiness → bool | | | | | | |
| `or` | short-circuit truthiness → bool | | | | | | |

**Normative rule.** A mixed `int`/`float` arithmetic operand promotes to
`float`.

**Normative rule.** `+` is defined for `int`, `float`, `string`, and `list`.
All other arithmetic operators are defined only for numbers.

**Normative rule.** Comparison operators (`<`, `<=`, `>`, `>=`) are defined
for `int`/`float` (in any numeric mix), `string`, and `bool` pairs; for any
other pair they are a type error (`E3001` at runtime, or statically when the
checker can prove it).

### 9.2 Unary operators

**Normative rule.** Unary `-` is defined for `int` (checked; negation of
`i64::MIN` is `E4013`) and `float`. Unary `not` accepts any value and yields
the boolean negation of its truthiness.

### 9.3 Logical operators

**Normative rule.** `a and b` evaluates `a`; if `a` is falsy the result is
`false` and `b` is **not** evaluated. Otherwise the result is the truthiness of
`b`. `a or b` evaluates `a`; if `a` is truthy the result is `true` and `b` is
**not** evaluated. Otherwise the result is the truthiness of `b`. Both always
yield a `bool`.

**Normative rule.** There is no `&&`, `||`, or `!` operator; those are lexical
errors.

### 9.4 Assignment operators

**Normative rule.** `=`, `+=`, `-=`, `*=`, `/=`. A compound assignment reads
the current target, applies the corresponding binary operator, and writes the
result. There is no `%=` or `^=`.

### 9.5 Indexing

**Normative rule.** `base[index]` reads:

* a list at an integer index;
* a string at an integer index, yielding the character (Unicode scalar);
* a map at a string key;
* a struct at a string field name.

Negative indices count from the end. An out-of-range index is `E4019`; a map
lookup with a missing key is `E2003`; a type that cannot be indexed is
`E3001`.

**Normative rule.** The assignment forms `base[index] = v` mutate a list
element, a map entry, or an existing struct field. A missing map key is
inserted; a missing struct field is `E2003`. Indexing is not defined for other
kinds.

### 9.6 Field access and method calls

**Normative rule.** `receiver.field` on a struct instance reads the field. On
any other value, `receiver.name` is a **zero-argument method call** (§24).

---

## 10. Numeric Model

### 10.1 Integers

**Normative rule.** `int` is a signed 64-bit two's-complement integer.

**Normative rule.** All integer arithmetic is checked: `+`, `-`, `*`, unary
`-`, `/`, `%`, and `^` produce `E4013` on overflow instead of wrapping or
saturating.

### 10.2 Integer boundaries

**Normative rule.** The minimum integer literal is `-9223372036854775808`
(`i64::MIN`), written as a unary minus applied to the special magnitude. The
maximum literal is `9223372036854775807`. The bare magnitude
`9223372036854775808` is a lexical error (`E1002`).

**Normative rule.** `i64::MIN / -1` and `i64::MIN % -1` are `E4013`.

*Evidence:* `Lexer::number`, `Parser::unary`; `numeric` (`src/run/mod.rs`);
`tests/boundaries.rs`.

### 10.3 Floats

**Normative rule.** `float` is IEEE-754 binary64.

**Normative rule.** Integer/float arithmetic promotes to `float`. Converting
an `int` to `float` for a mixed operation is exact for values within the
float's exact-integer range and rounds otherwise; this is the only implicit
numeric conversion.

### 10.4 Promotion

**Normative rule.** In any arithmetic operation with one `int` operand and one
`float` operand, the `int` operand is converted to `float` and the result is
`float`.

### 10.5 Division, remainder, and exponentiation

**Normative rule.** Integer `/` truncates toward zero. `7 / 2` is `3`;
`-7 / 2` is `-3`.

**Normative rule.** `%` follows the sign of the dividend. `7 % 3` is `1`;
`-7 % 3` is `-1`.

**Normative rule.** Integer `^` raises to a non-negative integer power; a
negative exponent is `E4013`; an overflowing power is `E4013`.

**Normative rule.** Float `^` is `powf`; float `%` is IEEE remainder; float
`/` is IEEE division.

### 10.6 Division and remainder by zero

**Normative rule.** Division by zero is `E4007`. This applies to:

* integer `/` and `%` by zero;
* float `/` by `0.0` or `-0.0`;
* float `%` by `0.0` or `-0.0`.

**Normative rule.** `0.0` and `-0.0` are both zero for this rule.

*Evidence:* `numeric` (`src/run/mod.rs`); `tests/regressions.rs::b1_*`;
`tests/boundaries.rs::float_boundaries`.

### 10.7 NaN and infinity

**Normative rule.** `NaN` and infinities are ordinary `float` values. They are
reachable through arithmetic (for example `1e309` denotes `+inf`, and
`1e309 - 1e309` is `NaN`), not through literals.

**Normative rule.** Any comparison with `NaN` (`<`, `<=`, `>`, `>=`) yields
`false`; `NaN == NaN` yields `false`.

**Normative rule.** `format_float` renders `NaN` as `nan`, positive infinity as
`inf`, and negative infinity as `-inf`.

### 10.8 Negative zero

**Normative rule.** `-0.0` is a distinct `float` value. It compares equal to
`0.0` and formats as `-0.0`.

### 10.9 Conversions

**Normative rule.** `to_int(x)` accepts:

* an `int` (identity);
* a `bool` (`true`→1, `false`→0);
* a `string` that trims and parses as a decimal `i64`;
* a finite `float` with `i64::MIN ≤ x < 2^63`, truncated toward zero.

Anything else is `E4013` (for a non-finite or out-of-range float) or `E3001`
(for a non-numeric string or unsupported kind). `to_int` never saturates.

**Normative rule.** `to_float(x)` accepts an `int`, `float`, `bool`, or a
string that trims and parses as a float; otherwise `E3001`.

---

## 11. Equality

**Normative rule.** Equality is defined by `==` and `!=` and is the same
relation, negated. It is:

* `none == none` is `true`; `none` equals no other value.
* `bool` and `string` compare by value.
* `int` and `float` compare numerically and **across types**: `1 == 1.0` is
  `true`.
* `list` compares by length and element-wise equality.
* `map` compares by key set and key-wise value equality.
* struct compares by nominal type name, field order, field count, and
  field-wise equality.
* enum compares by tag, payload length, and payload-wise equality. (Tags are
  globally unique, so the enum type name need not be compared.)
* range compares by `start` and `end`.
* function compares by **identity**: a closure equals only itself; two native
  functions are equal when they name the same builtin. Distinct functions are
  never equal, even with identical bodies.
* any other cross-kind pair is `false` (no error).

**Normative rule.** `NaN == NaN` is `false`; `0.0 == -0.0` is `true`.

**Normative rule.** Equality is reflexive and symmetric on all values, and
transitive where the value model permits (numeric equality is transitive;
function identity is transitive).

*Evidence:* `Value::equals` (`src/run/value.rs`); `tests/run.rs`;
`tests/contract.rs::r7_*`; `tests/regressions.rs::f06_*`.

---

## 12. Ordering

**Normative rule.** The ordering operators `<`, `<=`, `>`, `>=` are defined
only for these operand pairs:

* `int` with `int`, `float`, `int`/`float` mixed;
* `string` with `string`;
* `bool` with `bool`.

**Normative rule.** For any other operand pair the operation is a type error:
`E3001` at runtime, and statically `E3001` when the checker can prove both
operand types are non-orderable.

**Normative rule.** `NaN` is unordered: every ordering comparison involving a
`NaN` operand yields `false`, not an error.

**Normative rule.** `bool` ordering is `false < true`.

**Normative rule.** Ordering is **not** defined for lists, maps, structs,
enums, or ranges; there is no lexicographic order.

**Normative rule.** `sort`, `min`, and `max` do not raise an error for
unordered values; they treat an unordered pair as equal and preserve source
order (a stable fallback).

*Evidence:* `Value::comparable_with`, `Value::cmp_val` (`src/run/value.rs`);
`Ty::orderable_with` (`src/types.rs`); `tests/regressions.rs::f05_*`.

---

## 13. Evaluation Order

**Normative rule.** Evaluation is strictly left-to-right and deterministic for
all constructs. In particular:

| Construct | Order |
|---|---|
| binary `a op b` | `a`, then `b` |
| `and` / `or` | left operand, then right **only if** needed (§9.3) |
| call `f(a, b)` | arguments left-to-right, then the callee is invoked |
| method `r.m(a, b)` | receiver, then arguments left-to-right |
| field / index `b[i]` | base, then index |
| list `[a, b]` | elements left-to-right |
| map `{k: v}` | entries left-to-right |
| constructor `C(a, b)` | arguments left-to-right |
| f-string | parts left-to-right |
| pipeline `x \|> f` | left operand, then right, then the call |
| `if c {…} else {…}` | condition, then the selected branch only |
| `match v {…}` | subject, then arms in source order until one matches |
| `let x = e` | initializer, then the binding is defined |
| `a op= e` | right-hand side, then read the target, then write |

**Normative rule.** Evaluating an expression never evaluates an operand that
the semantics do not require (short-circuiting).

**Normative rule.** Parameter binding is independent of evaluation order. In a
call with named arguments, the argument expressions are still evaluated
left-to-right in source order, exactly once; mapping them to parameters
afterwards MUST NOT reorder or repeat evaluation. For
`f(b: e1(), a: e2())`, `e1()` is evaluated before `e2()`, and the resulting
values are then bound to `b` and `a` respectively (§15.7).

**Normative rule.** A compound assignment `target op= e` evaluates `e` once,
then evaluates the target's subexpressions to read its current value, then
evaluates them again to write the result. The target subexpressions are
therefore evaluated twice. (This mirrors the implementation; do not rely on a
side-effecting target subexpression being evaluated only once.)

*Evidence:* `eval_inner` and `exec_stmt` (`src/run/mod.rs`); verified in
`tests/property.rs` determinism checks.

---

## 14. Control Flow

### 14.1 Values and truthiness

**Normative rule.** The following values are **falsy**: `false`, `none`,
integer `0`, float `0.0`, the empty string, the empty list, and the empty map.
Every other value is **truthy**. This is used by `if`, `while`, `and`, `or`,
`not`, and the `assert` builtin.

### 14.2 `if`

**Normative rule.** `if c { A } else { B }` evaluates `c`; if truthy it
evaluates `A` and yields the value of its last statement; otherwise it
evaluates `B` and yields its value.

**Normative rule.** Without an `else`, a false condition yields `none`.

### 14.3 Loops

* `while c { body }` evaluates `c` before each iteration; a falsy condition
  ends the loop and yields `none`.
* `loop { body }` repeats until exited by `break`, `return`, or `throw`.
* `for pattern in iterable { body }` binds the pattern to each element and
  runs the body. The loop yields `none`.

**Normative rule.** `break` ends the innermost loop and yields `none`.
`continue` skips to the next iteration.

**Normative rule.** `break` and `continue` MUST appear inside a loop; the
checker rejects them otherwise (`E2015`). A lambda resets this context: a
`break` inside a lambda is not licensed by a loop outside the lambda.

### 14.4 `return`

**Normative rule.** `return [e]` terminates the innermost enclosing function (a
`fn` body or a lambda body), yielding `e` (or `none`). A `return` reaching
expression position where no function body is active is a parse error at the
statement level (`E1006`); a `return` used as a value inside an expression
(for example `if true { return 1 } else { 2 }` in a `let`) propagates as a
control-flow signal and, if it escapes the function, is `E4030`.

### 14.5 `throw` and `try`/`catch`/`finally`

**Normative rule.** `throw e` raises the value of `e` as a *throwable*. A
`try { … } catch x -> { … }` catches a throwable raised in its body (including
one raised in a called function) and binds `x` to the thrown value.

**Normative rule.** Only explicit `throw` is catchable. Runtime diagnostics
(overflow, division by zero, index errors, and so on) are **not** values and
are **not** catchable; they propagate to the top and terminate the program.

**Normative rule.** An uncaught throwable terminates the program with `E4026`.

**Normative rule.** `catch` is mandatory: there is no `try` without `catch`.

### 14.6 `finally`

**Normative rule.** If present, a `finally` block runs exactly once on every
exit path from the `try`/`catch`: normal completion, `return`, `break`,
`continue`, `throw`, and a fatal runtime error.

**Normative rule.** If the `finally` block itself raises a control-flow signal
(`return`, `break`, `continue`, or `throw`), that signal **replaces** the
pending outcome. Otherwise the pending outcome is preserved.

*Non-normative examples.* (Note that `catch`/`finally` are written on the same
line as the preceding `}`, per §3.7.)

```aura
fn f() -> int {
    try {
        return 1
    } catch e -> {
        return 0
    } finally {
        return 2
    }
}
# f() == 2
```

```aura
try {
    throw "a"
} catch e -> {
    print("caught")
} finally {
    throw "b"
}
# prints `caught`, then throws "b"
```

**Normative rule.** A fatal runtime error inside `finally` propagates (it is
not catchable) and supersedes any pending outcome.

*Evidence:* `Stmt::Try` in `exec_stmt` (`src/run/mod.rs`); `tests/run.rs::
finally_control_flow_overrides_the_pending_outcome`.

---

## 15. Functions and Closures

### 15.1 Declaration

**Normative rule.** `fn name(params) [-> T] { body }` declares a named
function at the top level. Parameters are names with optional type
annotations. Annotations are recorded and used to type the body (§6.2).

**Normative rule.** A function body is a block. Its value is the value of its
last statement; a body with no value yields `none`.

### 15.2 Return values

**Normative rule.** `return e` yields `e`. A function with no `return` and no
final value yields `none`.

**Normative rule.** If a function has a declared return type, every `return`
expression in its body MUST be compatible with that type (`E3005` otherwise).

### 15.3 First-class functions

**Normative rule.** A named function used as a value denotes a function value.
A builtin name used as a value denotes a native function value. A lambda
denotes a closure value.

**Normative rule.** Function values are displayed as `<fn>` and compare by
identity (§11).

### 15.4 Lambdas

**Normative rule.** A lambda is written `x -> e`, `(x, y) -> e`, or
`(x, y) -> { … }`. A block-bodied lambda uses the block as its body, so
`return` works and the last expression is the value. An expression-bodied
lambda returns that expression.

**Normative rule.** Lambda parameters, like function parameters, are immutable
bindings.

### 15.5 Closures and capture

**Normative rule.** A lambda captures its **defining environment by
reference**. A captured `let mut` binding can be mutated by the closure, and
the mutation is visible to the environment outside the closure.

*Non-normative example.*

```aura
let mut x = 1
let f = () -> { x = x + 1; return x }
f()      # 2
x        # 2
```

**Normative rule.** Capture is not by value and there is no explicit capture
syntax.

> **SPECIFICATION STATUS — closure lifetime.** The language has no way to
> observe capture after the defining scope ends as an error; environments are
> reference-counted and remain alive while any closure holds them. Aura does
> not expose a lifetime or ownership concept. This is frozen only to the
> extent above; no other capture guarantees are made.

### 15.6 Recursion

**Normative rule.** Named top-level functions are hoisted and may be
mutually recursive. Recursion is bounded by the call-frame limit (§31.3):
exceeding 512 active calls is `E4011`.

### 15.7 Arguments

**Normative rule.** A call to a **directly resolved top-level function** MAY
supply arguments by position or by parameter name, using `name: value`. A
positional argument fills the next unfilled parameter in declaration order; a
named argument fills the parameter with that exact, case-sensitive name. All
positional arguments MUST precede all named arguments.

**Normative rule.** A call MUST supply every declared parameter exactly once.
For a directly resolved call this is checked before execution: a parameter
supplied more than once, a named argument naming no parameter, and a declared
parameter left unfilled are each `E3001`.

**Normative rule.** After arguments are mapped to parameters, the annotated
type of each parameter is checked against the mapped argument's inferred type
(§6.5). An argument whose inferred type is `Unknown` imposes no constraint
(§6.4).

**Normative rule.** Named arguments require a statically known parameter set.
A named argument is accepted only for a directly resolved top-level function.
A named argument supplied to a built-in, to a method, or to any dynamically
resolved callable (a function value, a closure, a variable holding a callable,
or an `Unknown` callee) is `E3001`, diagnosed during checking when the
category is known.

**Normative rule.** Evaluation order is source order and is independent of
parameter binding: argument expressions are evaluated left-to-right exactly
once, before or while being associated with parameters. Mapping arguments to
parameters MUST NOT reorder or repeat evaluation (§13).

**Normative rule.** Calling a function with the wrong number of arguments, or
with an argument whose inferred type is incompatible with an annotated
parameter, is `E3001`. For a call the checker resolves to a specific top-level
`fn` declaration, this is diagnosed during checking (§6.5); for any other
callable it remains a runtime error. Calling a non-function value is a runtime
`E3001`.

**Normative rule.** There are no default parameter values and no variadic
parameters in this version. A parameter that is not supplied is an error, not
an omitted default.

### 15.8 Nested functions

**Normative rule.** Functions are declared at the top level only. A named
function cannot be declared inside another function's body. Nested functions
are expressed as lambdas bound to names.

---

## 16. Mutability and Scope

### 16.1 Bindings

**Normative rule.** `let name = e` binds immutably; `let mut name = e` binds
mutably. A plain reassignment to an immutable binding is `E2001` (checked when
statically visible, and enforced by the runtime in all cases).

**Normative rule.** Every binding requires an initializer (`E2005`). A
destructuring `let` (§4.7) binds each name in its pattern immutably and also
requires an initializer.

### 16.2 Reassignment

**Normative rule.** `x = e` writes a binding in the nearest scope that
declares it, which MUST be mutable. `x op= e` reads, applies, and writes.

### 16.3 Scope

**Normative rule.** Scopes are lexical and block-based. A block, `if` branch,
loop body, `for` body, `match` arm, `catch` body, function body, and lambda
body each introduce a scope. A binding is visible from its declaration to the
end of its scope, including nested scopes.

**Normative rule.** A loop variable, match binding, or catch binding is scoped
to its construct and is not visible afterwards.

**Normative rule.** Shadowing in a nested scope is permitted. Declaring the
same name twice in the *same* scope is `E2007`.

### 16.4 Parameters

**Normative rule.** Parameters are immutable bindings in the function's scope.
Reassigning a parameter is `E2001`.

**Normative rule.** A parameter whose name begins with `_` MUST NOT be used in
the body; using it is `E2009`.

### 16.5 Structure and collection mutation

**Normative rule.** A struct field is assignable through `s.field = v` or
`s["field"] = v`; a list element through `xs[i] = v`; a map entry through
`m[k] = v` (inserting the key if absent). These mutate in place and are
visible through every alias of the value.

### 16.6 Shared reference semantics

**Normative rule.** Lists, maps, and struct instances have **reference
semantics**. Binding or passing such a value aliases the same storage; a
mutation through one alias is observable through all aliases.

**Normative rule.** Mutability is a property of the *binding*, not of the
value. `let` makes the binding immutable (it cannot be reassigned), but a
list, map, or struct reached through an immutable binding can still be
mutated in place through an index, field, or entry assignment (`a[0] = x`,
`s.x = x`, `m[k] = v`) or a mutating method (`push`). `let mut` is required
only to reassign the *binding* itself.

*Evidence:* `Value::List`/`Map`/`Instance` use `Rc<RefCell<…>>`
(`src/run/value.rs`); `index_set`, `field_set`, `push` (`src/run/mod.rs`,
`src/stdlib/mod.rs`).

---

## 17. Structs

### 17.1 Declaration

**Normative rule.** `struct Name { f1: T1, f2: T2, ... }` declares a nominal
struct type with fields in declaration order. Every field MUST have a type
annotation. Field names MUST be unique within the struct. The struct name MUST
be unique among user types (`E2012`).

### 17.2 Construction

**Normative rule.** A struct is constructed by name:

* **named form:** `S { f: v, ... }` — every declared field MUST be supplied
  exactly once. An unknown field name is `E2003`; a missing, duplicate, or
  wrong-typed field is `E3001`.
* **positional form:** `S(v1, v2, ...)` — exactly one value per declared
  field, in declaration order. A wrong count is `E3001`.

**Normative rule.** No supplied field is ever silently dropped, and a partial
named construction is an error.

**Normative rule.** A field value whose inferred type is `Unknown` is
accepted; a field value whose inferred type is definitely incompatible with
the declared type is `E3001`.

**Normative rule.** Mixing named and positional fields is not expressible in
the named form (the grammar requires `name: value`) and is rejected.

### 17.3 Access and assignment

**Normative rule.** `s.field` reads a field; `s.field = v` and `s["field"] = v`
assign a field. Reading or writing an undeclared field is `E2003`.

### 17.4 Equality and ordering

**Normative rule.** Two struct instances are equal when they have the same
type name, the same fields in the same order, and equal field values. Structs
are not orderable.

### 17.5 Display and mutation

**Normative rule.** A struct displays as `Name { f: v, ... }` in declaration
order, quoting nested strings.

**Normative rule.** Struct field types are validated at construction. A field
read `s.field` on a receiver whose inferred type is a declared struct `S`
that has a field `field` infers the declared type of that field. A field read
on any other receiver (an `Unknown` type, a primitive, a list, a map, an
enum, or a struct without that field) infers `Unknown`. This inferred type
participates in every existing check that uses compatibility or ordering
(§6.3, §6.5, §9, §15.7). The runtime is unchanged and remains authoritative
for field access.

*Evidence:* `check_struct_construction`, `check_field_value`
(`src/check/mod.rs`); `construct` (`src/run/mod.rs`);
`tests/regressions.rs::b3_*`, `b4_*`.

---

## 18. Enums

### 18.1 Declaration

**Normative rule.** `enum Name { V1, V2(T), V3(T1, T2), ... }` declares an
enum type. Each variant is a tag with zero or more positional payload types.
A variant tag MUST be unique **across the whole program** (`E2013`); tags are
global.

### 18.2 Construction

**Normative rule.** A variant is constructed by its tag with **positional**
arguments, including the empty form for a zero-payload variant:

```
A            # NOT valid as an expression: `A` is a name lookup
A()          # valid: zero-payload variant
A(1)         # one-payload variant
```

**Normative rule.** A zero-payload variant MUST be constructed with `A()`
(empty parentheses) in expression position. A bare `A` in expression position
is an undefined-name error (`E2003`).

**Normative rule.** Named payload arguments (`A(x: 1)`) are **rejected**
(`E3001` by the checker and by the runtime). Enum payloads are positional.

**Normative rule.** The number of positional payload values MUST equal the
number of declared payload types (`E3001`). Each payload value whose inferred
type is known MUST be compatible with the declared payload type (`E3001`).

### 18.3 Equality and ordering

**Normative rule.** Two variants are equal when their tags are equal and their
payloads are element-wise equal. Enums are not orderable.

### 18.4 Display

**Normative rule.** A variant displays as its tag alone when it has no
payload, and as `Tag(p1, p2, ...)` otherwise, quoting nested strings.

*Evidence:* `construct` (`src/run/mod.rs`); `check_variant_construction`
(`src/check/mod.rs`); `tests/regressions.rs::b6_*`.

---

## 19. Match

### 19.1 Syntax

```
match subject {
    pattern1 -> expr
    pattern2 if guard -> { stmt; ... }
    tag(v)   -> { ... }
    _        -> expr
}
```

**Normative rule.** `match` is an expression. Each arm is `pattern [if guard]
-> body` where `body` is either a block or a single expression terminated by a
newline or semicolon.

### 19.2 Matching

**Normative rule.** Arms are tried in source order. An arm matches when its
pattern matches the subject and, if present, its guard evaluates truthy in the
pattern's binding scope. The value of `match` is the value of the matching
arm's body.

**Normative rule.** There is **no exhaustiveness requirement**. If no arm
matches, the result is `E4029` at runtime.

### 19.3 Patterns

* A literal pattern matches a value equal to the literal.
* A binding pattern (`name` or `_`) always matches; `name` binds.
* A list pattern matches a list of exactly the same length whose elements all
  match.
* A variant pattern matches a variant with the same tag and the same payload
  count whose payload elements all match.

**Normative rule.** A pattern MUST NOT bind the same name twice (`E2014`). A
variant pattern MUST name a declared variant (`E3002` otherwise).

### 19.4 Control flow in arms

**Normative rule.** A `return`, `break`, `throw`, or `continue` executed while
evaluating an arm propagates out of the `match`. A bare control-flow keyword as
an arm body requires a block (`tag -> { break }`); `tag -> break` is a parse
error.

*Evidence:* `Expr::Match` in `eval_inner`, `match_pattern`, `bind_pattern`
(`src/run/mod.rs`); `tests/run.rs`; `tests/adversarial.rs`.

---

## 20. Collections

### 20.1 Lists

**Normative rule.** `[a, b, c]` constructs a list. A list has reference
semantics and is mutable in place.

* Indexing: `xs[i]`, integer, negative allowed; out of range is `E4019`.
* Iteration: `for x in xs` yields elements; the iteration operates on a
  snapshot, so mutating the list during iteration does not change the sequence
  being iterated.
* Mutation: `xs[i] = v`, `xs.push(v)`, `xs.pop()`.
* Equality: length and element-wise (§11).
* Ordering: not defined.
* Display: `[a, b, c]` with nested strings quoted.

### 20.2 Maps

**Normative rule.** `{k: v, ...}` constructs a map. Keys MUST be `string`;
a non-string key in a literal is `E3001` at runtime (and in an annotation).

* Lookup: `m[k]` (string key) is `E2003` if absent; `m.get(k)` yields `none`
  if absent.
* Insertion/update: `m[k] = v` inserts or replaces.
* Equality: key set and key-wise value equality.
* Ordering: not defined.
* Iteration: `for k in m` yields keys in ascending key order (maps are ordered
  by key).
* Display: `{"k": v, ...}` in ascending key order.
* Removal: `m.remove(k)` yields the removed value or `none`.

### 20.3 Empty-map limitation

**Normative rule.** `{}` is an empty *block*, not an empty map; an empty block
yields `none`. Aura has **no empty-map literal** in this version.

> **KNOWN LANGUAGE LIMITATION.** Because the map grammar requires at least one
> entry and `{}` parses as a block, an empty map cannot be written directly.
> This is a real limitation of the current language, not a designed feature.

*Evidence:* `Parser::atom` (`{` → `map_ahead` decides map vs block);
`tests/boundaries.rs`; `docs/SEMANTIC_FREEZE_AUDIT.md`.

---

## 21. Tuples

**Normative rule.** Aura has **no distinct tuple type**. `(a, b)` with a comma
is **list sugar**: it constructs a list of its elements. Consequently a tuple
value is indistinguishable from a list: it supports indexing, `len`, equality
with a list, mutation, and list display.

*Non-normative example.* `(1, 2)` displays as `[1, 2]`, is equal to `[1, 2]`,
and has `.len()` = 2.

*Evidence:* `Expr::Tuple` lowers to `Value::list` in `eval_inner`
(`src/run/mod.rs`).

> **SPECIFICATION STATUS — intentionality.** The elimination of a distinct
> tuple type is consistent with the grammar note ("Aura has no distinct tuple
> value") and the list-sugar grammar production. It is frozen as: a
> parenthesized comma-list denotes a list. The AST retains an `Expr::Tuple`
> node as an implementation detail; it carries no additional semantics.

---

## 22. Loops and Ranges

### 22.1 `range`

**Normative rule.** `range(n)` is `range(0, n)`. `range(a, b)` denotes the
integers `a, a+1, ..., b-1`. The range is start-inclusive and end-exclusive,
with step always `+1`. Bounds MUST be integers (`E3001` otherwise).

**Normative rule.** An empty or descending range (`b <= a`) is empty. Negative
bounds are permitted.

**Normative rule.** `len(range(a, b))` is `max(0, b - a)` computed with
saturating arithmetic.

**Normative rule.** Ranges are not orderable. Ranges compare by equality
(`start` and `end`). Iterating a range yields its integers in increasing
order.

### 22.2 Iteration laziness

**Normative rule.** `for x in range(a, b)` iterates lazily: an immediate
`break` does not materialize the range. Materializing a range elsewhere is
bounded by a limit of 10,000,000 elements (`E4013` beyond it).

### 22.3 `for` over other iterables

**Normative rule.** `for` iterates lists (elements), strings (Unicode scalar
characters), maps (keys, ascending), and ranges (integers). Iterating any
other value is `E4018` (checked statically for known scalars, otherwise at
runtime).

### 22.4 Mutation during iteration

**Normative rule.** Iteration over a list or map uses a snapshot of the
iteration sequence; mutating the collection during iteration does not change
the sequence.

*Evidence:* `iterate`, `Stmt::For` (`src/run/mod.rs`); `tests/adversarial.rs`
(`huge_range_loop_can_break_immediately`).

---

## 23. Pipeline Operator

**Normative rule.** The pipeline operator `|>` passes the left operand as the
**first argument** of the right-hand call:

* `x |> f`    is `f(x)`;
* `x |> f(a)` is `f(x, a)`;
* `x |> r.m(a)` is `r.m(x, a)`;
* `x |> c` where `c` is a non-call callable value is `c(x)`.

**Normative rule.** The piped value becomes the **first positional argument**,
so a parenthesized suffix MAY include named arguments after it:
`x |> f(y: 1)` is `f(x, y: 1)`. A named argument naming the first parameter is
a duplicate assignment (§15.7). No pipeline-specific argument binding exists;
the desugared call uses ordinary call semantics.

**Normative rule.** The desugaring is applied at parse time. Beyond a call,
method call, or bare callable value, `x |> y` is a runtime type error
(`E3001`) if `y` is not callable.

**Normative rule.** `|>` is left-associative and has lower precedence than
every binary operator, so `a + b |> f` is `(a + b) |> f`.

**Normative rule.** Evaluation order is: left operand, then the right-hand
operand, then the call.

*Evidence:* `desugar_pipe` (`src/parse/mod.rs`); `Expr::Pipe` in `eval_inner`
(`src/run/mod.rs`); `tests/grammar.rs::
pipeline_passes_the_left_operand_as_the_first_argument`.

---

## 24. Method Calls

**Normative rule.** `receiver.name(args)` calls a method on `receiver`.
Methods are defined only on built-in receiver kinds (string, list, map, range);
structs and enums have no methods.

**Normative rule.** A method call on a receiver whose type the checker knows
MUST name a method that exists on that type (`E2003` otherwise), and its
argument count and argument types are checked (`E3001`). This includes a
receiver whose type is a user struct or enum: since those types have no
methods, **any** method call on a known struct or enum receiver is `E2003` at
check time. A `range` receiver exposes only `len`; any other method is `E2003`
at check time. A receiver of type `Unknown` remains permissive (§2.3).

**Normative rule.** `receiver.name` **without parentheses** on a non-struct
receiver is a **zero-argument method call** and is validated against the same
method table as the parenthesized form (`E2003` when the method does not exist
on a known receiver type). On a struct receiver it is a field read.

> **SPECIFICATION STATUS — no-paren method calls.** This behavior is
> implemented (`Expr::Field` dispatches to a zero-argument method for
> non-struct receivers) and is frozen as part of the language.

**Normative rule.** If the named method does not exist, the call is `E2003`.
It is rejected at check time when the receiver type is known, and at runtime
otherwise.

### 24.1 Method inventory

Methods exist for these receivers. Signatures are normative for arity; the
return types below are the checker's declared returns.

**string** (`TypeClass::Str`):

| Method | Args | Returns |
|---|---|---|
| `len` | 0 | `int` |
| `upper` | 0 | `string` |
| `lower` | 0 | `string` |
| `trim` | 0 | `string` |
| `contains` | 1 string | `bool` |
| `starts_with` | 1 string | `bool` |
| `ends_with` | 1 string | `bool` |
| `split` | 1 string | `[string]` |
| `replace` | 2 string | `string` |
| `chars` | 0 | `[string]` |

**list** (`TypeClass::List`):

| Method | Args | Returns |
|---|---|---|
| `len` | 0 | `int` |
| `push` | 1 any | `none` |
| `pop` | 0 | dynamic |
| `first` | 0 | dynamic |
| `last` | 0 | dynamic |
| `join` | 1 string | `string` |
| `contains` | 1 any | `bool` |
| `sort` | 0 | dynamic |
| `reverse` | 0 | dynamic |
| `map` | 1 fn | `[T]` |
| `filter` | 1 fn | `[T]` |
| `reduce` | 1 fn, 1 any | dynamic |

**map** (`TypeClass::Map`):

| Method | Args | Returns |
|---|---|---|
| `len` | 0 | `int` |
| `get` | 1 string | dynamic |
| `has` | 1 string | `bool` |
| `keys` | 0 | `[string]` |
| `values` | 0 | `[T]` |
| `remove` | 1 string | dynamic |

**range**: `len` (0 args) → `int`.

*Evidence:* `methods()` (`src/stdlib/signatures.rs`); `method`,
`string_method`, `list_method`, `map_method` (`src/stdlib/mod.rs`).

---

## 25. Built-in Functions

These functions are part of the language surface. Names, arity bounds, and
argument classes are normative. Return types marked `dynamic` are
`Ty::Unknown` to the checker but are described below.

| Name | Arity | Arguments | Returns | Notes |
|---|---|---|---|---|
| `print` | 0+ | any | `none` | prints each argument's display separated by a space, then a newline |
| `len` | 1 | string/list/map/range | `int` | length; error otherwise |
| `to_string` | 1 | any | `string` | display form |
| `to_int` | 1 | any | `int` | see §10.9; `E4013`/`E3001` on failure |
| `to_float` | 1 | any | `float` | see §10.9 |
| `range` | 1–2 | int(s) | `range` | `range(n)` / `range(a, b)` |
| `abs` | 1 | int/float | same | `E4013` for `i64::MIN` |
| `min` | 2 | any, any | one of them | unordered ⇒ source order |
| `max` | 2 | any, any | one of them | unordered ⇒ source order |
| `push` | 2 | list, any | `none` | mutates the list |
| `keys` | 1 | map | `[string]` | ascending order |
| `values` | 1 | map | `[T]` | ascending key order |
| `sort` | 1 | list | `[T]` | stable; unordered ⇒ source order |
| `reverse` | 1 | string/list | same | reversed copy |
| `map` | 2 | list, fn | `[T]` | applies fn to each element |
| `filter` | 2 | list, fn | `[T]` | keeps elements whose predicate is truthy |
| `reduce` | 3 | list, fn, any | dynamic | left fold `f(acc, item)` |
| `sum` | 1 | list | int/float | int overflow is `E4013` |
| `assert` | 1–2 | any, string? | `none` | `E4028` if falsy |
| `enumerate` | 1 | list | `[[int, T]]` | index/value pairs |
| `zip` | 2 | list, list | `[[T, U]]` | pairs, shortest length |

**Python bridge** (see §32):

| Name | Arity | Arguments | Notes |
|---|---|---|---|
| `py_eval` | 1 | string | evaluates a Python expression |
| `py_import` | 1 | string | imports a module; returns a map of public names |
| `py_call` | 2+ | string, string, any... | calls `module.attr(args...)` |
| `py_version` | 0 | — | Python version string |

**Feature-gated** (present only when compiled with the feature):

* `json_encode(value) -> string`, `json_decode(string) -> value` (feature `json`);
* `regex_match`, `regex_find`, `regex_find_all`, `regex_replace` (feature `regex`);
* `time_now`, `time_unix`, `sleep_ms` (feature `time`).

**Normative rule.** A call to a builtin with the wrong argument count or a
provably wrong argument type is `E3001` before execution, and is also enforced
at runtime. An undefined builtin is `E2003`.

*Evidence:* `builtins()` (`src/stdlib/signatures.rs`); natives in
`src/stdlib/mod.rs`, `src/stdlib/ext.rs`, `src/bridge/mod.rs`.

---

## 26. Declarations and Symbol Visibility

**Normative rule.** A module's top-level declarations (functions, structs,
enums, aliases, constants) are visible throughout the module, including to
declarations that textually precede them ("forward reference"). This is
achieved by a hoisting pre-pass.

**Normative rule.** Hoisting also makes each function's parameter count and
parameter annotations available before any body is checked, so calls to
top-level functions are checked against the callee's signature regardless of
declaration order, including mutually recursive calls (§6.5).

**Normative rule.** Redeclaring a name in the same scope is `E2007`; declaring
two user types with the same name is `E2012`; declaring two variants with the
same tag anywhere is `E2013`.

**Normative rule.** Constants and top-level expressions are evaluated in
source order after all declarations are registered. A constant's initializer
MUST NOT read a constant declared later (`E2003`), but functions may read
constants freely because functions run after initialization.

**Normative rule.** Newlines separate statements, so the keyword `as` is
reserved but no construct consumes it; `use a as b` is a parse error.

*Evidence:* `Checker::hoist`, `active_const`/`const_order` (`src/check/mod.rs`);
`Interp::run` passes 1 and 2 (`src/run/mod.rs`); `tests/adversarial.rs::
const_forward_reference_is_rejected_by_checker_and_runtime`.

---

## 27. `pub` and `use`

**Normative rule.** `pub` and `use` are **reserved and semantically inert**.

* `pub` MAY prefix any item (`fn`, `struct`, `enum`, `type`) and has no
  effect; it carries no visibility meaning and is never consulted by the
  checker or the runtime.
* `use path.to.module` is accepted anywhere an item may appear and is a no-op.
  `use stdlib` and `use a.b.c` are equivalent.

**Normative rule.** There is no module system, no imports, and no visibility
in this version. `pub`/`use` exist so that future syntax can be added without
a breaking change.

*Evidence:* `Parser::item` (`pub` read and ignored except forwarding),
`Item::Use` inert in checker and runtime; `docs/contract.md` §10.

---

## 28. Entry Points, CLI, Library, REPL

### 28.1 Compile modes

**Normative rule.** A module is compiled in one of two modes:

* **module mode** — no entry point required; used by `aura check` and
  `aura eval`, and by the REPL per submission.
* **program mode** — `fn main` required (`E4027` otherwise); used by
  `aura run`.

### 28.2 Execution

**Normative rule.** Executing a module: all declarations are registered, then
top-level constants and expressions are evaluated in source order, then `main`
(if present) is called with no arguments. The value of `main` is discarded.

**Normative rule.** `main`, if present, MUST take no parameters (`E2011`
otherwise). `main` MAY have a return annotation; the value is discarded.

### 28.3 CLI

| Command | Behavior |
|---|---|
| `aura run <file>` | program mode: parse, check, execute; requires `main` |
| `aura check <file>` | module mode: parse and check only |
| `aura eval <code>` | module mode: parse, check, execute a snippet |
| `aura repl` | interactive session |
| `aura version` | print the version |

`<file>` may be `-` for standard input.

### 28.4 Library

**Normative rule.** The library entry points `run_source`, `run_toplevel_stdout`,
and `run_program` execute the same pipeline as the CLI; they differ only in
compile mode and output capture. A caller MAY also use `compile` +
`execute` directly.

### 28.5 Consistency rule

**Normative rule.** The CLI, library, and REPL share one parser, one checker,
and one interpreter. A given source string MUST produce the same checking
verdict, the same result, and the same diagnostic code across these surfaces,
except for the intended differences of §28.1 (the `main` requirement), output
presentation, and REPL session persistence (§29).

*Evidence:* `src/lib.rs`, `src/main.rs`, `src/repl.rs`.

---

## 29. REPL

**Normative rule.** The REPL evaluates submissions one at a time against a
persistent session. A submission may be a single statement (including
top-level `let`/`let mut`), a set of items, or a bare expression.

**Normative rule.** Session state persists across submissions:

* `let` bindings (and their mutability), including values;
* functions;
* structs (with their fields);
* enums (with their variants and payloads);
* type aliases;
* mutation performed in earlier submissions.

**Normative rule.** A submission is checked against all declarations from prior
submissions. A later submission MAY reference an earlier binding, function,
struct, enum, or alias. A function's parameter **names** and annotations are
part of its persisted session signature, so a named call in a later submission
resolves against them (§15.7).

**Normative rule.** A failed submission reports its diagnostic and leaves the
session unchanged: it neither adds new declarations nor removes existing ones.

**Normative rule.** A bare expression submission prints the value's display
form. A declaration submission (`let`, `fn`, `struct`, `enum`, `type`) is
silent.

**Normative rule.** `:quit`, `:q`, `:exit` end the session; `:help` prints
help; EOF ends the session cleanly. Blank lines are ignored.

**Normative rule.** There is no `main` requirement in the REPL; the module
mode applies to each submission.

**Normative rule.** Redefinition of an existing session name follows module
semantics (`E2007`).

*Evidence:* `src/repl.rs`; `check::GlobalDecl`;
`tests/repl.rs::regression_repl_struct_persistence`,
`regression_repl_enum_persistence`, `regression_repl_alias_persistence`.

---

## 30. Error Model

### 30.1 Categories

**Normative rule.** Diagnostics are grouped by phase and carry a stable
`E####` code:

* `E1xxx` — lexical and syntactic (lexer/parser);
* `E2xxx` — name and rule checks (checker);
* `E3xxx` — type-level checks (checker);
* `E4xxx` — runtime;
* `E5xxx` — optional features / Python bridge.

### 30.2 Error codes

| Code | Meaning |
|---|---|
| E1001 | invalid character |
| E1002 | malformed number |
| E1003 | invalid escape |
| E1004 | unterminated string |
| E1006 | expected token |
| E1009 | reserved word used as a name |
| E1014 | `else if` is not part of the language |
| E1015 | nesting limit exceeded |
| E2001 | assignment to an immutable binding |
| E2003 | undefined name/function/field/key |
| E2005 | `let` without initializer |
| E2007 | redeclaration in the same scope |
| E2009 | `_`-prefixed parameter used |
| E2010 | invalid assignment target |
| E2011 | invalid `main` |
| E2012 | duplicate user type |
| E2013 | duplicate enum variant tag |
| E2014 | duplicate pattern binding |
| E2015 | `break`/`continue` outside a loop |
| E3001 | type mismatch |
| E3002 | unknown type or constructor |
| E3005 | return type mismatch |
| E4007 | division or remainder by zero |
| E4011 | call-depth limit exceeded |
| E4013 | integer overflow / unrepresentable conversion |
| E4018 | value is not iterable |
| E4019 | index out of range |
| E4026 | uncaught thrown value |
| E4027 | missing `main` |
| E4028 | assertion failed |
| E4029 | no `match` arm matched |
| E4030 | `return` used a value where none is allowed |
| E4099 | (internal) throw crossing a call boundary |
| E4999 | internal error (never a user mistake) |
| E5001 | Python error |
| E5002 | Python bridge unavailable / value cannot cross |

*E5003 is defined in the implementation but is not currently produced; it is
not part of the normative surface (see §34.3).*

### 30.3 Error properties

**Normative rule.** Every user-visible rejection carries a code, a location
(span), and a message. The location is a byte span into the source.

**Normative rule.** Diagnostics are deterministic: the same source produces the
same first diagnostic.

**Normative rule.** Runtime diagnostics are terminal and are not catchable
values (§14.5). Only an explicit `throw` is a value.

**Normative rule.** `E4999` (internal error) MUST NOT be reachable from a
syntactically valid, well-formed program. The two occurrences in the operator
dispatch are unreachable for any input.

### 30.4 Error-code reachability

**Normative rule.** Every code listed in §30.2 is reachable from a
syntactically valid program and is covered by
`tests/grammar.rs::every_documented_error_code_is_reachable`. `E4030` is
produced when a `return` (or `break`/`continue`/`throw`) raised while computing
an expression escapes to a position where a value is required, for example
`let x = if true { return 1 } else { 2 }`.

*Evidence:* `codes` (`src/error.rs`); `docs/errors.md`;
`tests/grammar.rs::every_documented_error_code_is_reachable`.

---

## 31. Resource and Host Safety

### 31.1 Semantic limits

**Normative rule.** An expression or statement MUST NOT nest more than **256
AST levels**. Deeper nesting is `E1015`, never a crash. The limit counts AST
nodes (calls, operators, collections, blocks, and so on), starting at 1 for
each top-level expression. Grouping parentheses add no AST depth.

### 31.2 Parser recursion backstop

**Normative rule.** Independent of the semantic limit, the parser MUST NOT
recurse without bound. Grouping tokens such as parentheses, which add no AST
depth, are bounded by a separate internal recursion budget. Exceeding it is
also `E1015`, so users see one consistent nesting diagnostic. This is a
host-safety guard, not a second language nesting rule.

> **RESOLVED CONCEPTUAL DISTINCTION.** These are two distinct mechanisms:
>
> 1. the **semantic AST-node limit** (256; §31.1), which is the language
>    rule and is enforced iteratively after parsing; and
> 2. the **parser recursion backstop**, which protects the host stack against
>    non-AST recursion (grouping) and reports the same code.
>
> "256" describes **AST nodes**, not raw parentheses. A chain of grouping
> parentheses is limited by mechanism 2, not by the number 256.

### 31.3 Call-frame limit

**Normative rule.** At most **512** simultaneous user call frames MAY be
active, including the entry call to `main`. Exceeding this is `E4011`. This is
a language rule, not a host limitation.

### 31.4 Range materialization

**Normative rule.** Materializing a range (outside lazy `for` iteration) is
bounded at 10,000,000 elements; a larger range is `E4013`.

### 31.5 No host failures

**Normative rule.** No syntactically valid, well-formed program may cause a
host panic, stack overflow, or undefined behavior. Exceeding a limit produces
a stable `E####` diagnostic.

*Evidence:* `MAX_AST_DEPTH`, `MAX_PARSE_DEPTH`, `PARSE_STACK`
(`src/parse/mod.rs`); `MAX_CALL_FRAMES` (`src/run/mod.rs`);
`tests/boundaries.rs`, `tests/adversarial.rs`.

---

## 32. Python Boundary (feature `py`)

When compiled with the `py` feature, four functions cross the boundary. Without
the feature, the same names exist and every call is `E5002`.

### 32.1 Aura → Python

**Normative rule.** `int`, `float`, `bool`, `string`, `none`, `list`, and
`map` convert to their Python equivalents. A struct, enum, function, or range
does not convert; passing one is `E5002`.

### 32.2 Python → Aura

**Normative rule.** `None` → `none`; `bool` → `bool` (checked before `int`);
`int` → `int` if it fits `i64`, otherwise `E4013` (never a lossy float);
`float` → `float` (including `nan`/`inf`); `str` → `string`; `list` → `list`
(recursively); `dict` → `map` (recursively) with the restriction below.

**Normative rule.** A Python `dict` key that is not a `str` is `E5002`; keys
are never stringified. This prevents distinct keys such as `1` and `"1"` from
colliding.

**Normative rule.** Any other Python object converts to its `repr` string
(documented as opaque).

### 32.3 Data integrity

**Normative rule.** The boundary MUST NOT silently lose representable data:
out-of-range integers are rejected (`E4013`), and non-string dict keys are
rejected (`E5002`).

*Evidence:* `to_value`/`to_py` (`src/bridge/mod.rs`); `tests/python.rs`;
`tests/regressions.rs::f10_*`.

---

## 33. Frozen Design Decisions

The following decisions are established by the implementation and its tests
and are hereby frozen. Future changes require the RFC process.

1. **Function equality is identity.** A closure equals only itself; natives
   compare by name. `g == g` is `true`; distinct functions are never equal.
2. **Enum payloads are positional.** `A(1, 2)`; named payload arguments are
   rejected.
3. **Type aliases are transparent.** `type A = T` denotes `T` everywhere and
   has no runtime representation.
4. **REPL declarations persist.** Bindings, functions, structs, enums, and
   aliases carry across submissions; a failed submission does not corrupt the
   session.
5. **`finally` precedence.** A control-flow signal raised in `finally`
   replaces the pending outcome.
6. **Pipeline receiver insertion.** `x |> f(a)` means `f(x, a)`.
7. **Struct construction is validated.** Unknown fields (`E2003`), and
   missing/duplicate/extra/wrong-typed fields (`E3001`), are rejected before
   execution. No field is silently dropped.
8. **Remainder by zero is `E4007`** for both `int` and `float`, treating
   `0.0` and `-0.0` alike.
9. **Nesting limits.** 256 AST levels (`E1015`); 512 call frames (`E4011`);
   a separate parser recursion backstop for grouping, also `E1015`.
10. **Parenthesized comma-lists are lists; there is no tuple type.**
11. **`{}` is a block, not an empty map.** (See Known Limitations.)
12. **`pub` and `use` are inert.**
13. **`try` requires `catch`.**
14. **Enum tags are globally unique.**
15. **No-paren member access on a non-struct receiver is a zero-argument
    method call.**
16. **Maps are string-keyed and ordered by key.**
17. **Lists, maps, and structs have reference semantics.**
18. **Named arguments** are supported for directly resolved top-level
    functions only. Positional arguments precede named arguments; a parameter
    is supplied exactly once; a duplicate, missing, or unknown parameter is
    `E3001`; built-ins, methods, and dynamic callables reject named arguments.
    Evaluation stays in source order and is independent of parameter binding.
19. **Struct field reads propagate declared types.** When the receiver's
    inferred type is a known struct, `s.field` infers the field's declared
    type; otherwise it infers `Unknown`. This never evaluates anything and
    never changes evaluation order.

---

## 34. Known Limitations

These are explicit limitations of the current language. They are **not**
designed features waiting to be enabled; they are recorded so that users and
implementers do not assume guarantees the language does not make.

### 34.1 Language limitations

* **No empty-map literal.** `{}` is a block; a map requires at least one
  entry (§20.3).
* **No tuple type.** `(a, b)` is list sugar (§21).
* **No `try` without `catch`** (§14.5).
* **`match` arms cannot be bare control-flow keywords**; a block is required
  (§19.4).
* **No default or variadic function arguments** (§15.7).
* **Named arguments are limited to directly resolved top-level functions**
  (§15.7); built-ins, methods, and dynamic callables are positional.
* **No nested named function declarations**; use lambdas (§15.8).
* **`else if` is not part of the language** (§4.5).
* **No `for ... else`, no step on ranges** (§22.1).
* **No lexicographic ordering for lists/maps/structs/enums/ranges** (§12).

### 34.2 Static-checking limitations

* **Calls through a function value, closure, variable, or any callee that is
  not a directly resolved top-level function are not checked statically**
  against arity or parameter annotations; a mismatch is a runtime `E3001`
  (§6.5). Directly resolved top-level calls **are** checked.
* **Field reads infer a declared type only when the receiver's type is a
  known struct.** A field read on a value whose type the checker cannot
  determine (for example a parameter without an annotation, an `if`
  expression, a call whose return type is not declared, or an element of a
  list) still infers `Unknown` (§17.5), as do indexed reads and map lookups.
* **`if`/`match`/block expressions and lambdas infer `Unknown`** (§2.3), so
  their results are not statically checked.

### 34.3 Implementation limitations

* **`E5003`** is defined but unused and is not part of the normative surface.
* **`E4099`** is an internal signal, not a user-facing code.
* **`Tok::As`** is lexed but no construct consumes it.
* Runtime method aliases `up`/`down` exist in the interpreter's dispatch table
  but not in the signature registry. Because the checker validates both
  `receiver.name(...)` and no-parentheses `receiver.name` against the
  registry, they are unreachable through the normal pipeline and are not part
  of the language.

---

## 35. Specification Completeness Notes

Everything a user can write is described in this document. The following are
explicitly **out of scope** for this specification and are not language
features:

* module or package systems;
* generics, traits, or interfaces;
* async or concurrency;
* macros;
* a bytecode VM, optimizer, or native code generation;
* operator overloading;
* user-defined methods on structs/enums;
* reflection or runtime type inspection beyond the built-ins in §25.

These do not exist in Aura. This document defines the language as it is, not
as it might become.
