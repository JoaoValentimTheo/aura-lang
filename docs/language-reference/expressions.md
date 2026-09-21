---
layout: default
title: "Expressions"
parent: Aura Language Reference
nav_order: 6
---

[English](expressions.md) · [Português](expressions.pt_BR.md)

# Expressions

**Status:** Stable (except where labeled) · **Evidence:** `parser/to_ast.py`
(`_PRECEDENCE`, `parse_expression`, `parse_postfix`, `parse_subscript`),
`transpiler/transformers/expressions.py` (`transform_BinaryOp`,
`transform_UnaryOp`, `transform_RangeExpr`, `transform_ComprehensionExpr`).

An expression produces a value. The grammar and associativity are in
[grammar.md](grammar.md) §6; this document is the **semantics** of each form:
what it means and what it compiles to. Every "compiles to" below is the exact
Python emitted by `transpiler/transformers/expressions.py`, verified by
transpiling the shown source (*probe*).

---

## 1. Precedence (lowest to highest)

This table is the parser's `_PRECEDENCE` table verbatim (`to_ast.py:465-488`).
Higher number binds tighter. It matches [grammar.md](grammar.md) §6.1.

| Prec | Operators | Assoc | Compiles to |
|------|-----------|-------|-------------|
| 1 | `=`, `+=`, `-=`, `*=`, `/=`, `%=`, `**=`, `&=`, `&#124;=`, `^=`, `<<=`, `>>=`, `??=`, `&#124;>` | right (pipe left) | assignment / `f(x)` |
| 2 | `? :` | right | `(t if c else f)` |
| 3 | `or` | left | `(a or b)` |
| 4 | `and` | left | `(a and b)` |
| 5 | `==`, `!=`, `<`, `>`, `<=`, `>=`, `in`, `not in`, `is`, `is not` | left | `(a op b)` |
| 6 | `&#124;` | left | `(a &#124; b)` |
| 7 | `^` | left | `(a ^ b)` |
| 8 | `&` | left | `(a & b)` |
| 9 | `<<`, `>>` | left | `(a << b)` |
| 10 | `..`, `..<` | none | `range(...)` |
| 11 | `??`, `?:` | left / right* | helper call |
| 12 | `+`, `-` | left | `(a + b)` |
| 13 | `*`, `/`, `%`, `as` | left | `(a * b)` / `T(x)` |
| 14 | `**` | right | `(a ** b)` |
| 15 | unary `-`, `+`, `~`, `not`, `await`, `...` | prefix | `(- a)` etc. |
| 16 | call, index, slice, member, safe-nav, struct-init | postfix | `f()`, `a.b`, `a[i]` |

\* `??` is right-associative and `?:` is left-associative — the parser's
`is_left_assoc` returns `False` for `??` and `True` for `?:` (`to_ast.py:2654-2655`).
`a ?? b ?? c` → `_aura_null_coalesce(a, _aura_null_coalesce(b, c))`;
`a ?: b ?: c` → `_aura_elvis(_aura_elvis(a, b), c)` (*probe*).

- **Comparison is looser than the bitwise group and the shifts**, exactly as in
  Python: `1 & 2 == 2` → `((1 & 2) == 2)` and `1 < 2 | 3` → `(1 < (2 | 3))`
  (`to_ast.py:472-476`, *probe*).
- **Comparison does not chain**: `a < b < c` → `((a < b) < c)`. Write
  `a < b and b < c` (`to_ast.py:2519-2628`, *probe*).
- **Pipe is the loosest expression operator**, grouped with assignment at level
  1 and parsed specially so it chains left to right (`to_ast.py:2558-2563`).
  It is looser than `??`: `a |> f ?? b` → `_aura_null_coalesce(f, b)(a)`
  (*probe*).
- **Unary `-` binds looser than `**`**: `-2 ** 2` → `(- (2 ** 2))` = `-4`,
  while `- 2 * 3` → `((- 2) * 3)` (`to_ast.py:2495-2504`, *probe*).
- **`not` binds looser than comparison and bitwise but tighter than `and`**:
  `not a in b` → `(not (a in b))`, `not a and b` → `((not a) and b)`
  (`to_ast.py:2487-2494`, *probe*).

---

## 2. Literals

Primary literals are `IntLiteral`, `FloatLiteral`, `StrLiteral`,
`FStringLiteral`, `BoolLiteral`, `NoneLiteral`, and the collection literals of
§10. Their concrete syntax is in [lexical-structure.md](lexical-structure.md)
§4 and [grammar.md](grammar.md) §1.4.

| Source | Compiles to | Evidence |
|--------|-------------|----------|
| `42` | `42` | `transform_IntLiteral` |
| `3.5` | `3.5` | `transform_FloatLiteral` |
| `"hi"` | `'hi'` | `transform_StrLiteral` |
| `true` / `false` | `True` / `False` | `transform_BoolLiteral` |
| `none` | `None` | `transform_NoneLiteral` |
| `f"x{1+2}"` | `f'x{1 + 2}'` | `transform_FStringLiteral` |

- **`none` is the null literal.** `null` → pointed parse error "use `none`"
  (`to_ast.py:2686-2688`); `True`/`False`/`None` are reserved spellings
  (`to_ast.py:536-540`).
- **F-string fields** are parsed recursively (`to_ast.py:2855-2867`); a `!r`/`!s`/`!a`
  conversion and a `:format` spec are preserved (`to_ast.py:2884-2918`). An
  invalid field expression is a parse error.

---

## 3. Identifiers, member access, indexing, slicing

- `x` — local, parameter, module member, or field. A bare reference to a
  non-local module member resolves to `Module.name` (`transform_Identifier`,
  *probe*: `return f"{owner}.{py_safe_name(node.name)}"`). A name that is a
  Python keyword (`raise`, `class`, …) is emitted with a trailing underscore
  (`py_safe_name`).
- `a.b` — member access → `a.b` (`transform_MemberExpr`). Protocol method names
  (`str`, `eq`, …) map to their dunder when the class declares them
  (`transform_MemberExpr:552-554`).
- `super` — zero-arg `super` renders as `super()` (`_render_object`) so
  `super.new(...)` / `super.m()` work; `super(args)` renders as
  `super().__init__(args)` (`transform_CallExpr:452-454`).
- `a[i]` — index → `a[i]` (`transform_IndexExpr`).

```
postfix_op     = "(" , [ arg_list ] , ")"
               | "[" , slice , "]"
               | "?[" , slice , "]"
               | "." , identifier
               | "?." , identifier ;
```

**Slices** are Python-style, with any component omitted
(`to_ast.py:3014-3033`):

| Source | Compiles to | Evidence |
|--------|-------------|----------|
| `a[1:4]` | `a[1:4]` | `transform_SliceExpr` |
| `a[::2]` | `a[::2]` | `transform_SliceExpr` |
| `a[i:j:k]` | `a[i:j:k]` | `transform_SliceExpr` |

**A range used as an index is a slice, not a key** (`transform_IndexExpr:504-508`,
`_range_as_slice`):

| Source | Compiles to | Note |
|--------|-------------|------|
| `a[0..<3]` | `a[0:3]` | exclusive upper bound |
| `a[0..3]` | `a[0:(3) + 1]` | inclusive → `stop + 1` |
| `a[1..]` | `a[1:]` | open end |
| `a[1..4 step 2]` | `a[1:(4) + 1:2]` | step preserved |

`a[0..]` → `a[0:]` (*probe*). A leading open bound (`a[..3]`) is **not**
accepted: it is a parse error (*probe*). A multi-dimensional subscript
(`a[0:2, 3]`) is a parse error — slices are one-dimensional (*probe*).

---

## 4. Calls

```
arg_list       = argument , { "," , argument } ;
argument       = expression
               | identifier , "=" , expression        // keyword
               | "*" , expression                     // positional spread
               | "**" , expression                    // keyword spread
               | "..." , expression ;                 // adaptive spread
```

- `f(args)` → `f(args)`; `obj.m(args)` → `obj.m(args)` (`transform_CallExpr`).
  `Klass.m(args)` and a Python module/object method emit the name verbatim.
- **Keyword arguments** use `:=` or `:` in the source and render Python `=`
  (`to_ast.py:3070-3084`): `f(1, b: 2)` → `f(1, b=2)` (*probe*). A positional
  argument after a keyword is a parse error.
- A **generator expression as the sole argument** is parsed inline
  (`to_ast.py:3090-3112`), so `sum(x for x in xs)` works without extra parens
  (*probe*).
- **Spreads**: `*items` → `*items`; `**mapping` → `**mapping`. The marker is
  placed in `SpreadExpr.is_dict` (`to_ast.py:3051-3069`).
- **`...value` is adaptive** (`transform_CallExpr:459-464`): when it is the sole
  argument, the call lowers to `_aura_call(func, value)`, which unpacks a
  `dict` as keyword arguments and anything else as positional arguments
  (`transform_CallExpr:386-394` for member calls). The helper is injected on
  demand.
- **Aura method conveniences** apply to member calls when the name is *not* a
  user-declared method (`known_member` wins, `transform_CallExpr:400-412`):

| Source | Compiles to | Evidence |
|--------|-------------|----------|
| `xs.length()` | `len(xs)` | `transform_CallExpr:414-415` |
| `xs.size()` | `len(xs)` | `transform_CallExpr:416-420` |
| `xs.len()` | `len(xs)` | `transform_CallExpr:416-420` |
| `xs.is_empty()` | `(not xs)` | `transform_CallExpr:421-422` |
| `xs.contains(x)` | `(x in xs)` | `transform_CallExpr:423-424` |
| `xs.add(x)` | `xs.append(x)` | `transform_CallExpr:425-428` |
| `s.slice(a)` | `s[a:]` | `transform_CallExpr:429-431` |
| `s.slice(a, b)` | `s[a:b]` | `transform_CallExpr:429-433` |
| `s.char_at(i)` | `s[i]` | `transform_CallExpr:434-435` |

- **String-method aliases** map Aura names to Python names
  (`METHOD_ALIASES:102-121`): `starts_with`→`startswith`, `ends_with`→`endswith`,
  `to_upper`→`upper`, `to_lower`→`lower`, `trim`→`strip`, `trim_left`→`lstrip`,
  `trim_right`→`rstrip`, `capitalize_words`/`to_title`→`title`,
  `index_of`→`find`, `last_index_of`→`rfind`, `is_alpha`→`isalpha`,
  `is_alphanumeric`→`isalnum`, `is_digit`→`isdigit`, `is_numeric`→`isnumeric`,
  `is_space`→`isspace`, `is_lower`→`islower`, `is_upper`→`isupper`
  (*probe*: `"hello".to_upper()` → `"HELLO"`).
  These aliases apply only to method calls on **instances** (strings, lists);
  calls on imported **modules** (e.g. `strings.trim()`) keep the original name.
- An **unknown member name** is emitted verbatim (Python passthrough), e.g.
  `xs.append(4)` → `xs.append(4)`, `xs.pop()`, `xs.sort()`
  (`transform_CallExpr:441-444`).

---

## 5. Arithmetic: `+ - * / % **`

Aura follows **Python** arithmetic, which differs from JVM/Kof:

| Expression | Result | Why | Evidence |
|------------|--------|-----|----------|
| `7 / 2` | `3.5` | `/` is **true division**; never truncates | *probe* |
| `-7 % 3` | `2` | `%` follows the **divisor's sign** (Python); Kof/JVM follows the dividend | *probe* |
| `7 % -3` | `-2` | same rule | *probe* |
| `2 ** 3 ** 2` | `512` | `**` is right-associative | *probe* |
| `-2 ** 2` | `-4` | unary `-` binds looser than `**` | *probe* |

- **There is no integer floor division.** `//` always starts a line comment
  ([grammar.md](grammar.md) §1.1); cast to get an integer: `int(total / count)`.
- `+ - * / %` render `(a op b)` and `**` renders `(a ** b)`
  (`transform_BinaryOp:290-298`).
- `+` on two strings (or any sequence) is concatenation; `*` with an integer
  repeats (`"ab" * 2` → `'ab' * 2`, *probe*). These are Python semantics.
- `a + b .. c` → `range((a + b), c + 1)`: `+` binds tighter than `..`
  (*probe*).

---

## 6. Comparison, equality, membership, identity

All of `< > <= >= == != in not in is is not` sit at precedence 5 and render
`(a op b)` (`transform_BinaryOp:294-295`).

- **`==`/`!=` are value equality** through Python's `==` (Python semantics per
  type). **`is`/`is not` are identity** and render Python `is`/`is not`.
- **`x is none` / `x is not none`** is the idiomatic null check → `(x is None)`
  (*probe*).
- **Identity against any other literal is a parse error**
  (`to_ast.py:508-518`, `2616-2626`): `x is "a"`, `x is 5`, `x is true`,
  `x is not "a"` all raise "'is' compares identity; use '==' (or '!=') to
  compare with a literal" (*probe*). `none` is deliberately excluded from the
  literal check so `x is none` is accepted.
- **`in` / `not in`** render Python `in` / `not in`. `3 in [1, 2, 3]` → `True`
  (*probe*).
- **No comparison chaining**: `a == b == c` → `((a == b) == c)` (*probe*).

---

## 7. Logical: `and or not`

- `and`/`or` render `(a and b)` / `(a or b)` and are **left-associative**
  (`transform_BinaryOp:293`, `to_ast.py:470-471`).
- They use **Python truthiness and return an operand**, not a coerced `bool`:
  `1 and 2` → `2`, `0 or "fallback"` → `"fallback"` (*probe*).
- **Short-circuit**: `and`/`or` do not evaluate the right operand when the left
  decides — inherited from the emitted Python.
- `not` renders `(not a)` (`transform_UnaryOp:323-327`). It binds looser than
  comparison and bitwise, tighter than `and` (`to_ast.py:2487-2494`):
  `not 0` → `True`, `not not a` → `(not (not a))` (*probe*).
- **`&&`, `||`, `!` are removed**: each is a pointed parse error telling you to
  use `and`/`or`/`not` (`to_ast.py:2546-2551`, `2484-2486`, *probe*).

---

## 8. Bitwise and shifts: `& | ^ ~ << >>`

- `& | ^` render bitwise between integers; `<<`/`>>` render Python shifts
  (`transform_BinaryOp:290-298`, *probe*: `x << 2` → `(x << 2)`).
- **`~` is a unary prefix** in Aura and renders `(~ x)` (`transform_UnaryOp:323-327`,
  *probe*). There is **no missing complement** — `~x` is valid.
- Bitwise precedence is Python's: `|` (6) looser than `^` (7) looser than
  `&` (8) looser than shifts (9), all tighter than comparison:
  `x | y & z ^ w` → `(x | ((y & z) ^ w))` (*probe*).
- There is no `>>>` form; `>>` is the only right shift.

---

## 9. Ternary `? :`, Elvis `?:`, null-coalescing `??`

| Form | Meaning | Compiles to | Evidence |
|------|---------|-------------|----------|
| `c ? t : f` | conditional | `(t if c else f)` | `transform_CondExpr` |
| `a ?: b` | `b` when `a` is **falsy** (`none`/`false`/`0`/`""`/`[]`/`{}`) | `_aura_elvis(a, b)` | `transform_BinaryOp:262-264` |
| `a ?? b` | `b` only when `a` is `none` | `_aura_null_coalesce(a, b)` | `transform_BinaryOp:259-261` |

- The `?:` and `??` helpers are injected on demand (`uses_coalesce`,
  `transform_BinaryOp:259-264`). Their bodies are `value if value is not None
  else default` and `value if value else default` respectively (*probe*).
- `a ?? b ?? c` is right-associative; `a ?: b ?: c` is left-associative (§1).
- `??=` is the compound assignment form and lowers to `x = x if x is not None
  else y` (`x ??= y`, *probe*).
- Ternary binds looser than everything down to `or`; the branches parse at the
  lowest precedence (`to_ast.py:2586-2593`): `a ?: b + c` →
  `_aura_elvis(a, (b + c))` (*probe*).

---

## 10. Pipe `|>`

`a |> f` applies `f` to `a`; `a |> f(b, c)` inserts `a` as the **first**
argument (`transform_PipeExpr:578-591`).

| Source | Compiles to |
|--------|-------------|
| `a &#124;> f` | `f(a)` |
| `a &#124;> f(b)` | `f(a, b)` |
| `a &#124;> f &#124;> g` | `g(f(a))` |

- Pipe is the **loosest expression operator** (level 1) and chains **left to
  right** (`to_ast.py:2558-2563`).
- When the right side is a call to a stdlib collection function (`map`, `filter`,
  `reduce`, `take`, `drop`), the transformer injects the `stdlib.collections`
  import (`transform_PipeExpr`, *probe*). The canonical pipeline is in
  [closures.md](closures.md) §4.

---

## 11. Ranges: `..`, `..<`, `step`, open end

```
range_expr     = coalesce , [ ( ".." | "..<" ) , coalesce , [ "step" , coalesce ] ] ;
```

| Source | Compiles to | Note | Evidence |
|--------|-------------|------|----------|
| `1..10` | `range(1, 10 + 1)` | inclusive upper bound | `transform_RangeExpr` |
| `0..<100` | `range(0, 100)` | exclusive | `transform_RangeExpr` |
| `0..100 step 5` | `range(0, 100 + 1, 5)` | step | `transform_RangeExpr:643-646` |
| `a..<b step 2` | `range(a, b, 2)` | step with exclusive | *probe* |
| `1..` | `itertools.count(1)` | open-ended | `transform_RangeExpr:635-639` |

- A range evaluates to a Python `range` object; `list(1..5)` → `[1, 2, 3, 4, 5]`
  (*probe*).
- **An open-ended range** (`0..`) needs the next token to *not* begin an end
  expression: a closer, `{`, `;`, `,`, `)`, `]`, `:`, `=`, `step`, a statement
  keyword, EOF, or a token on a **later line** ends it (`_range_ends_here`,
  `to_ast.py:2636-2652`). The transformer injects `import itertools`
  (`uses_infinite_range`).
- `a .. b + c` → `range(a, (b + c) + 1)`: `+` binds tighter than `..` (*probe*).
- Using a range **as an index** is a slice, not `range(...)` — see §3.

---

## 12. Safe navigation: `?.` and `?[`

| Source | Compiles to | Evidence |
|--------|-------------|----------|
| `a?.b` | `(a.b if a is not None else None)` | `transform_SafeNavExpr:561-575` |
| `a?[i]` | `(a[i] if a is not None else None)` | `transform_SafeNavExpr:564-566` |
| `a?.b?.c` | nested conditional, one guard per step | *probe* |
| `a?.b()` | `(a.b if a is not None else None)()` | `transform_CallExpr:386-394` |

- Guarding is against **`is not None`**, not falsiness (*probe*).
- `user?.address?.city` yields `None` when `user` is `none` (*probe*).
- **Caveat (Unspecified):** `a?[0..<2]` is *not* converted to a slice — it emits
  `(a[range(0, 2)] if a is not None else None)`, which raises `TypeError` at
  runtime. Only the plain `a[0..<2]` path (`transform_IndexExpr:504-508`) applies
  the range-as-slice rule (*probe*).

---

## 13. Spread: `*`, `**`, `...`

| Context | Form | Compiles to | Evidence |
|---------|------|-------------|----------|
| list | `[*a, *b]` | `[*a, *b]` | `transform_ListLiteral` |
| set | `{*a}` | `{*a}` | `transform_SetLiteral:218-225` |
| dict | `{**a, **b}` | `AuraDict({**a, **b})` | `transform_DictLiteral:199-216` |
| call | `f(*a)`, `f(**m)` | `f(*a)`, `f(**m)` | `_render_call_args:479-495` |
| call | `f(...v)` | `_aura_call(f, v)` | `transform_CallExpr:456-464` |

- `*`/`**`/`...` are **prefix markers**, valid only where a spread is allowed
  (call arguments and list/set/tuple/dict literals). A bare spread in a value
  position is a parse error (`to_ast.py:2505-2515`, *probe*: `[**a]` fails).
- `...` inside a list or tuple literal behaves as positional spread
  (`to_ast.py:3170-3171`, `3206-3207`).
- A dotted name and a spread are the only alternatives: `1...10` is rejected
  with a hint to use `..`/`..<` (`to_ast.py:2538-2543`, *probe*).

---

## 14. Comprehensions

```
comprehension  = list_comp | set_comp | dict_comp | generator_expr ;
```

| Source | Compiles to | Evidence |
|--------|-------------|----------|
| `[e for p in it]` | `[e for p in it]` | `transform_ComprehensionExpr:764-765` |
| `[e for p in it if c]` | `[e for p in it if c]` | same, filters appended |
| `{e for p in it}` | `{e for p in it}` | `transform_ComprehensionExpr:766-767` |
| `{k: v for p in it}` | `{k: v for p in it}` | `transform_ComprehensionExpr:736-739` |
| `(e for p in it)` | `(e for p in it)` | `transform_ComprehensionExpr:768-769` |

- **Multiple `for` and multiple `if` clauses** are supported and preserve source
  order: `[x*y for x in r for y in s]` → `[(x * y) for x in r for y in s]`
  (*probe*); `[x for x in r if a if b]` chains the filters.
- **A two-name tuple pattern over a dict inserts `.items()`** only when the
  iterable is provably dict-shaped (a dict literal, an `AuraDict(...)`/`dict(...)`
  call, or `.items()`/`.keys()`); a list of pairs is left untouched
  (`_needs_items`, `transform_ComprehensionExpr:748-754`).
- A comprehension's optional leading spread (`[ *a for x in xs ]`) makes the
  element a `SpreadExpr` (`to_ast.py:3170-3171`).
- Comprehensions are **eager**; generator expressions are lazy (Python).

---

## 15. Lambdas (expression)

```
lambda         = ( identifier | "(" , [ param_list ] , ")" ) , "=>" , ( expression | block ) ;
```

| Source | Compiles to | Evidence |
|--------|-------------|----------|
| `x => x` | `(lambda x: x)` | `transform_LambdaExpr:662-669` |
| `(a, b) => a + b` | `(lambda a, b: (a + b))` | *probe* |
| `() => 1` | `(lambda : 1)` | *probe* |
| `(x) => { return x }` | a **hoisted `def`**, its name referenced | `_hoist_block_lambda:689-709` |

- A single identifier before `=>` declares a one-parameter lambda
  (`to_ast.py:2719-2722`).
- A **block-bodied lambda cannot be a Python `lambda`** (statements / `return`),
  so it is hoisted to a real function `_aura_lambda_N`; captured mutable locals
  are declared `nonlocal` (`_nonlocal_declaration:711-731`).
- Parameters use the `def` grammar: defaults, `*args`, `**kwargs`, bare `*`
  (`_render_param:649-660`, `_parse_lambda_params:2942-2993`).
- See [closures.md](closures.md) for function types and capture semantics.

---

## 16. Collections expression surface

Collection literals and their operations. Concrete syntax is in
[grammar.md](grammar.md) §6.5.

### 16.1 Literals

| Source | Compiles to | Evidence |
|--------|-------------|----------|
| `[1, 2, 3]` | `[1, 2, 3]` | `transform_ListLiteral:195-197` |
| `[]` | `[]` | same |
| `{"a": 1}` | `AuraDict({'a': 1})` | `transform_DictLiteral:199-216` |
| `{}` | `AuraDict({})` | same |
| `{1, 2, 3}` | `{1, 2, 3}` | `transform_SetLiteral:218-225` |
| `()`, `(42,)`, `(1, "a")` | `()`, `(42,)`, `(1, 'a')` | `transform_TupleLiteral:227-231` |
| `Type { x: 1 }` | `Type(**AuraDict({'x': 1}))` | `to_ast.py:3119-3136` |

- A **trailing comma** is allowed in every literal: `[1,]` → `[1]`, `{1,}` →
  `{1}` (*probe*).
- `(1)` is **not** a tuple — it is the parenthesised expression `1`; a
  one-element tuple needs the comma `(42,)` (`transform_TupleLiteral:229-230`).
- A **dict** lowers through `AuraDict` (imported on demand). Access by `.name`
  or `["key"]` both work: `d.name` → `d.name`, `d["age"]` → `d["age"]`
  (*probe*).
- **Struct init** is sugar for `Type(**AuraDict({...}))` and is recognised only
  after a **capitalised** identifier (`to_ast.py:3119-3136`). Inside a condition
  or a `case` pattern, `{` opens a block/body, so struct init is suppressed
  (`_no_struct_depth`, `_pattern_depth`).
- An **empty `{ }` in expression position** parses as an empty dict literal, not
  an empty block (see [statements.md](statements.md) §1).

### 16.2 Indexing, slicing, membership, operators

| Operation | Compiles to | Evidence |
|-----------|-------------|----------|
| `xs[i]` (negative ok) | `xs[i]` | `transform_IndexExpr` |
| `xs[a:b:c]` | `xs[a:b:c]` | `transform_SliceExpr` |
| `x in xs` | `(x in xs)` | `transform_BinaryOp:295` |
| `xs + [6]` | `(xs + [6])` | `transform_BinaryOp` |
| `[0] * 3` | `([0] * 3)` | `transform_BinaryOp` |

Runtime results (*probe*): `xs[-1]` → last element, `xs[1:4]` → `[2, 3, 4]`,
`xs[::2]` → every other element, `3 in xs` → `True`, `xs + [6]` →
concatenation, `[0] * 3` → `[0, 0, 0]`.

### 16.3 Length, membership and mutation conveniences

| Source | Compiles to | Evidence |
|--------|-------------|----------|
| `xs.size()` / `xs.length()` / `xs.len()` | `len(xs)` | §4 table |
| `xs.contains(x)` | `(x in xs)` | §4 table |
| `xs.is_empty()` | `(not xs)` | §4 table |
| `xs.add(x)` | `xs.append(x)` | §4 table |
| `xs.append(x)`, `xs.pop()`, `xs.sort()`, `xs.reverse()`, `xs.insert(...)`, `xs.remove(...)`, `xs.index(...)`, `xs.count(...)`, `xs.extend(...)`, `xs.clear()` | emitted verbatim | Python passthrough (`transform_CallExpr:441-444`) |

- A **user-declared method** named `size`, `length`, `len`, `contains`, `add`,
  … is a `known_member` and **wins over the convenience** (`transform_CallExpr:400-412`).
- `.size()` etc. apply to any object with a Python `len`/member protocol, not
  just lists (e.g. `dict`, `set`, `str`).

---

## 17. Casts: `as`

`x as T` renders `T(x)` (`transform_BinaryOp:284-288`). The right side is
parsed as a **type** (`to_ast.py:2602-2608`).

| Source | Compiles to |
|--------|-------------|
| `x as int` | `int(x)` |
| `v as str` | `str(v)` |
| `x as float` | `float(x)` |

`as` sits at precedence 13, with `*`/`/`/`%`: `1 + x as int` → `(1 + int(x))`
and `x as int + 1` → `(int(x) + 1)` (*probe*).

---

## 18. Expression-position `if`, `match`, `try`, blocks

These constructs may appear where a value is expected
(`to_ast.py:2708-2715`, `transform_IfStmt:607-630`, `transform_MatchExpr`,
`transform_TryExpr`, `transform_BlockExpr`).

- `if c { a } else { b }` in expression position → `(a if c else b)`, using the
  last expression / `return` of each branch.
- `match` / `try` in expression position are **hoisted** into helper functions
  (`_aura_match_N`, `_aura_try_N`); the case body's tail expression becomes the
  returned value.
- A bare `{ … }` used as a value is hoisted into `_aura_block_N()`, whose last
  expression is returned (`transform_BlockExpr:890-931`).
- Assignment is **not** a value expression: `x = y` produces a statement, not a
  value (§1; the parser emits an assignment node, not a `BinaryOp`).

---

## 19. Operations that are NOT expressions

- `throw`, `return`, `break`, `continue` are **statements**
  ([statements.md](statements.md)).
- `yield` is a statement-level prefix form, not part of `expression`; its
  operand parses at the lowest precedence, so `yield x + 1` yields `x + 1` and
  `yield a, b` yields the tuple `(a, b)` ([grammar.md](grammar.md) §6.1,
  `to_ast.py:2470-2483`).
- There is no `??=`, `&&`, `||`, `!` as *operators* (they are removed spellings),
  no `//` floor division, and no `>>>` shift.

---

## 20. Summary of verified divergences from Kof/JVM

| Point | Aura | Kof/JVM | Evidence |
|-------|------|---------|----------|
| `7 / 2` | `3.5` (true division) | `3` (truncating) | *probe* |
| `-7 % 3` | `2` (divisor sign) | `-1` (dividend sign) | *probe* |
| `~x` | valid complement | `PARSE041` (no `~`) | *probe* |
| Ternary `? :` | present | absent | *probe* |
| `??`, `?:`, `..`, `in` | present | absent | `_PRECEDENCE` |
| `is` literal | parse error | n/a | *probe* |
