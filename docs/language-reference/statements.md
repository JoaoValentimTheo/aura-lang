---
layout: default
title: "Statements"
parent: Aura Language Reference
nav_order: 10
---

[English](statements.md) · [Português](statements.pt_BR.md)

# Statements

**Status:** Stable (except where labeled) · **Evidence:** `parser/to_ast.py`
(`parse_if_stmt`, `parse_for_stmt`, `parse_try_stmt`, `parse_match_stmt`),
`transpiler/transformers/statements.py` (`transform_IfStmt`,
`_labeled_loop`, `transform_TryStmt`), `transpiler/rules.py` (`RuleChecker`).

A statement executes an effect and, with the exceptions of an expression-bodied
`if`/`match`/`try` (see [expressions.md](expressions.md)), **does not produce a
value**. Semicolons are **optional** at every statement end; newlines are **not**
significant. The parser delimits statements by grammar
(`grammar.md` §1.1, §5).

---

## 1. Block

```
block          = "{" , { statement } , "}" ;
```

Introduces a new scope. Declarations inside a block do **not** leak out
(`rules.py:1249` `_visit_body` pushes/pops a scope).

- In statement position, `{ … }` is a **block** when it opens a control-flow
  body (`if`/`for`/`while`/`def`/…) or when it contains statements.
- **Gotcha:** a **standalone empty** `{ }` in an expression position parses as an
  empty **dict/struct literal**, not an empty block (*probe*:
  `def main() { { } }` emits `AuraDict({})`). A block with at least one
  statement, or one attached to a control-flow header, is a real block.

```aura
def main() {
  {
    let inner = 1
    print(inner)
  }
  // inner is out of scope here
}
```

---

## 2. `if` / `else if` / `else`

```
if_stmt        = "if" , expression , block , [ "else" , ( if_stmt | block ) ] ;
```

- The condition is parsed by `parse_condition`; inside a condition a `{` always
  opens the body, never a dict/struct literal (`to_ast.py:2103-2115`).
- `else if` **chains**: the `else` branch is itself an `if_stmt`, so any depth is
  accepted (`to_ast.py:2122-2123`).
- **`elif` / `elsif` are rejected** with a pointed error telling you to write
  `else if` (`to_ast.py:819-822`, *probe*).
- Each branch is a **block**; the braces are mandatory (unlike Kof). `if c
  print(1)` is a parse error.

```aura
let n = 2
if n == 1 {
  print("one")
} else if n == 2 {
  print("two")
} else {
  print("other")
}
```

*Evidence:* `test_syntax_complete.py::test_if_else_if_else` prints `two`.

`if` in **expression** position (`let x = if c { 1 } else { 2 }`) is covered in
[expressions.md](expressions.md).

---

## 3. `unless` (inverted `if`)

```
unless_stmt    = "unless" , expression , block , [ "else" , ( if_stmt | block ) ] ;
```

`unless cond { A }` runs `A` when `cond` is falsy. It transpiles to
`if not (cond): ...` (`statements.py:1113-1124`). `else` chains like `if`.

```aura
unless authenticated {
  redirect("/login")
}
```

*Evidence:* `test_syntax_complete.py::test_unless`.

---

## 4. `guard cond else { ... }`

```
guard_stmt     = "guard" , expression , "else" , block ;
```

Runs the `else` block when `cond` is **falsy**, then continues past the guard.
It transpiles to `if not (cond): <else>` (`statements.py:1126-1135`). There is
no `then` keyword and no value form.

```aura
def process(data) {
  guard data != none else {
    print("No data")
    return
  }
  print(data)          // data is non-none here
}
```

At **module scope** a bare `return` in the guard body is rewritten to
`raise SystemExit(...)` so the generated Python is valid; this is Aura's idiom
for "exit the program" (`statements.py:1137-1160`).

```aura
guard ready else { return }     // exits the program when not ready
```

*Evidence:* `test_syntax_complete.py::test_guard_early_return`.

---

## 5. `match` (statement)

```
match_stmt     = "match" , expression , "{" , { match_case } , "}" ;
match_case     = "case" , pattern , [ "if" , expression ] , ( block | "->" , expression ) ;
```

- A case body is either a **block** `{ ... }` or an **arrow** `-> expression`
  (`to_ast.py:2301-2308`). `->` takes a single statement/expression.
- A **guard** `case p if cond` refines the pattern. A guarded case does **not**
  count as a catch-all for exhaustiveness (warning **E109**).
- Patterns: wildcard `_`, literal, bare identifier (binds), dotted member
  (`Color.RED`), list/tuple destructuring with `*rest`, constructor patterns
  (`Shape.Circle(r)`), or-patterns (`1 | 2`), and `_ as name`
  (`to_ast.py:2344-2376`).
- **No fallthrough**: `:` and `=>` case syntax are rejected with a message
  pointing at `->` / `{ }` (`to_ast.py:2310-2322`).
- Missing a `case _` (or a bare binding `case name`) over a scalar, `bool` or
  enum domain emits warning **E109** (never fails a build;
  `TypeChecker`, `transpiler/errors.py:66`). A guarded `case _ if cond` does not
  suppress it.

```aura
match status {
  case 0 { print("inactive") }
  case n if n > 100 { print("overflow") }
  case _ { print("unknown") }
}
```

```aura
match command {
  case "quit" -> print("Goodbye!")
  case _      -> print("Unknown")
}
```

`match` in **expression** position is covered in
[expressions.md](expressions.md).

*Evidence:* `test_syntax_complete.py::test_match_literal`,
`::test_match_guard`.

---

## 6. Loops

```
while_stmt     = [ label ] , "while" , expression , block ;
until_stmt     = [ label ] , "until" , expression , block ;
for_stmt       = [ label ] , "for" , pattern , "in" , expression , [ "step" , expression ] , block ;
loop_stmt      = [ label ] , "loop" , block ;
label          = identifier , ":" ;
```

The optional `label` is an `IDENT` followed by `:` before the loop keyword
(`grammar.md` §5.2).

### 6.1 `while`

Condition evaluated **before** each iteration.

```aura
let mut i = 0
while i < 3 {
  print(i)
  i += 1
}
```

*Evidence:* `test_syntax_complete.py::test_while_loop`.

### 6.2 `until` (inverted `while`)

`until cond { ... }` runs while `cond` is falsy; transpiles to
`while not (cond):` (`statements.py:1169-1175`).

```aura
let mut i = 0
until i >= 3 { print(i); i += 1 }
```

*Evidence:* `test_syntax_complete.py::test_until_loop`.

### 6.3 `loop` (infinite)

Transpiles to `while True:` (`statements.py:1271-1275`). Termination is by
`break` (or `return`/`throw`).

```aura
loop {
  let input = read_input()
  if input == "quit" { break }
}
```

*Evidence:* `test_syntax_complete.py::test_loop_infinite_with_break`.

### 6.4 `for`-in

```
for_stmt       = … , "for" , pattern , "in" , expression , [ "step" , expression ] , block ;
```

Iterates any iterable target (list, range, comprehension, generator). The target
is a **pattern**; multiple names are a list pattern, so `for (k, v) in pairs`
destructures each element (`to_ast.py:2174-2184`).

```aura
for item in items { print(item) }
for (k, v) in pairs { print(k) }
```

*Evidence:* `test_syntax_complete.py::test_for_loop`.

### 6.5 `for` over ranges

`for i in 0..10` is inclusive (`1..10` yields 1…10); `0..<10` is exclusive
(`transform_RangeExpr`, `expressions.py:633-646`). Ranges and `range(...)` calls
are equivalent iterables.

```aura
for i in 0..<3 { print(i) }          // 0, 1, 2
for i in range(0, 6, 2) { print(i) } // 0, 2, 4
```

### 6.6 `step`

A trailing `step N` on a `for` either folds into a `range(...)` call, attaches
to a `..`/`..<` range, or slices any other iterable with `[::N]`
(`statements.py:1189-1216`).

```aura
for i in range(0, 10) step 2 { print(i) }  // 0 2 4 6 8
for i in 0..10 step 2 { print(i) }         // 0 2 4 6 8 10
for x in items step 2 { print(x) }         // items[::2]
```

*Evidence:* `test_syntax_complete.py::test_for_with_step`.

### 6.7 `break` / `continue`

- End / skip the **innermost** loop.
- **A `break` / `continue` outside a loop is rejected** by the rule checker
  (`rules.py:715-723`, *probe*).
- `break label` / `continue label` target a **labeled** enclosing loop.

**UNSPECIFIED / gotcha** — The parser greedily takes a following identifier as a
label (`to_ast.py:863-876`). Because newlines are insignificant, a `break` as the
last statement of a `match` arrow case followed by `case` on the next line is
parsed as `break case`, producing a runtime `NameError`. Write a `;` or use a
block body to disambiguate. This is parser behavior, not intended semantics.

### 6.8 Labeled loops

A label names an enclosing loop. `break label` exits that loop; `continue label`
skips to its next iteration (`_labeled_loop`, `statements.py:1218-1269`).
Labels work on `for`, `while`, `until` and `loop`.

```aura
outer: for i in range(3) {
  inner: for j in range(3) {
    if j == 1 { continue outer }
    if i == 2 { break outer }
    print(i, j)
  }
}
```

- `break`/`continue` naming a loop that is **not** enclosing is error **E318**
  (`rules.py:724-734`).
- An inner label is **not visible** from an outer loop body.
- A plain `break`/`continue` always targets the innermost loop.

*Evidence:* `test_syntax_complete.py::test_labeled_break_and_continue`,
`test_rule_gaps.py::TestUnknownLabel`.

---

## 7. `try` / `catch` / `finally`

```
try_stmt       = "try" , block , { catch_clause } , [ "finally" , block ] ;
catch_clause   = "catch" , [ type , [ "as" , identifier ] ] , block
               | "catch" , "as" , identifier , block ;
```

- A `try` requires **at least one** `catch` clause, or a `finally`
  (`to_ast.py:2258-2260`).
- One meaning per spelling (`to_ast.py:2233-2252`):

  | Form | Meaning | Emits |
  | --- | --- | --- |
  | `catch { }` | catch every exception, no binding | `except Exception:` |
  | `catch Type { }` | catch only `Type` | `except Type:` |
  | `catch Type as e { }` | catch only `Type`, bind to `e` | `except Type as e:` |
  | `catch as e { }` | catch every exception, bind to `e` | `except Exception as e:` |

- A lone identifier before `{` is always a **type**, never a binding. A second
  bare identifier (`catch Value Error`) is rejected: use `catch Type as name`
  (`to_ast.py:2243-2247`).
- `finally` always runs (`transform_TryStmt`, `statements.py:1305-1319`).
- **UNSPECIFIED:** `catch` clauses are emitted in source order; the type filter
  is whatever the CPython runtime resolves the name to. Aura does not reorder
  them for you, so list subclasses before their parents.

```aura
try {
  let file = open("data.txt")
  process(file)
} catch IOError as e {
  print("File not found")
} finally {
  print("Cleanup complete")
}
```

*Evidence:* `test_syntax_complete.py::test_try_catch_finally`,
`::test_try_only_finally`, `::test_try_without_handler_is_error`,
`test_custom_errors.py`.

---

## 8. `throw`

```
"throw" , expression , [ ";" ]
```

- `throw ValueError("bad")` raises the value as-is (`raise ValueError('bad')`).
- A **string literal** `throw "msg"` is wrapped: `raise Exception("msg")`, because
  Python cannot `raise` a bare string (`statements.py:1293-1303`).
- **UNSPECIFIED:** a non-string, non-exception expression (`throw 5`) is emitted
  as `raise 5`, which fails at runtime. The language does not reject it at
  compile time.

```aura
def validate(age) {
  guard age >= 0 else {
    throw ValueError("age cannot be negative")
  }
  return true
}
```

*Evidence:* `test_syntax_complete.py::test_throw_string_wrapped`,
`test_custom_errors.py`.

---

## 9. `return`

```
"return" , [ expression ] , [ ";" ]
```

- In expression position after `return`, a comma list becomes a **tuple**:
  `return b, a` returns `(b, a)` (`parse_trailing_tuple`,
  `to_ast.py:2215-2223`).
- `return` (bare) has no value.
- **`return` outside a function** is rejected, except inside a top-level
  `guard cond else { return }` (`rules.py:706-714`, *probe*).

```aura
def swap(a, b) {
  return b, a
}

def main() {
  let x, y = swap(1, 2)   // x = 2, y = 1
}
```

*Evidence:* `test_syntax_complete.py::test_swap`, `test_rule_gaps.py`.

---

## 10. `assert`

```
"assert" , expression , [ "," , expression ] , [ ";" ]
```

- When the condition is falsy, raises `AssertionError` (with the given message).
- The message is an expression (not restricted to a literal).

```aura
assert 1 + 1 == 2
assert 1 == 2, "nope"     // raises AssertionError("nope")
```

*Evidence:* `test_syntax_complete.py::test_assert`.

---

## 11. `with`

```
with_stmt      = [ "async" ] , "with" , with_item , { "," , with_item } , block ;
with_item      = expression , [ "as" , identifier ] ;
```

Uses `__enter__`/`__exit__`; `async with` uses `__aenter__`/`__aexit__`. The
documented rule is that `async with` is only valid inside an `async def`
(`grammar.md` §5.3, `modules.md` §16b) — **UNSPECIFIED:** unlike `await`, this
placement is **not** enforced by the rule checker (*probe*: `async with` in a
sync `def` produces no diagnostic; it fails only if the target Python is
invalid).

```aura
with open("x") as fh {
  print(fh)
}
```

*Evidence:* `test_syntax_complete.py::test_with_statement`.

---

## 12. Expression statement / empty statement

```
expression , [ ";" ] ;
```

Any expression used for effect (call, assignment, increment). A trailing `;` is
optional; statements are also separated by grammar across newlines. An isolated
`;` is tolerated and produces no statement (*probe*: `def main() { ; }` emits
`pass`).

```aura
def main() { let a = 1; let b = 2; print(a + b); }
```

---

## 13. Rules enforced by the checker

| Rule | Code | Evidence |
| --- | --- | --- |
| `return` outside a function (except top-level guard) | E004 | `rules.py:706-714` |
| `break`/`continue` outside a loop | E004 | `rules.py:715-723` |
| Unknown / non-enclosing label | E318 | `rules.py:724-734` |
| Unreachable code after `return`/`throw`/`break`/`continue` | E302 | `rules.py:1249-1268` |
| Local used before its `let`/`const` | E319 | `semantics.py:237-262` |
| `await` outside `async` (or top level) | E004 | `rules.py:747-757` |

*Evidence:* `test_extreme_rules.py` (E302),
`test_rule_gaps.py` (E318, E319), `test_language_rules.py`.
