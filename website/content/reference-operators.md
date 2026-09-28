# Operators

## Precedence

From loosest to tightest binding:

```text
|>            pipeline (lowest)
or
and
==  !=
<  <=  >  >=
|             bitwise or (int)
&             bitwise and (int)
<<  >>        shift (int)
..            range (right associative)
+  -
*  /  %
^             (right associative)
-  not  ~     (prefix unary)
()  []  .     (postfix: calls, indexing, methods)
```

`|>` is left-associative and lower than every binary operator, so `a + b |> f`
is `(a + b) |> f`.

## Arithmetic

| Operator | Meaning |
|---|---|
| `+` | numeric addition, string concatenation, list concatenation |
| `-` `*` `/` `%` | numeric |
| `^` | exponentiation (right-associative) |

A mixed `int`/`float` operand promotes to `float`. Integer arithmetic is
**checked**: overflow is `E4013`, not wraparound.

```aura
print(2 ^ 10)        # 1024
print(7 % 3)         # 1
```

Division or remainder by zero is `E4007` for both integers and floats.

## Bitwise and shift

`&` (and), `|` (or), `~` (not), `<<`, and `>>` operate on `int`. `|` is
contextual: a type-union separator where a type is expected, bitwise OR in
expression position.

```aura
print(6 & 3)         # 2
print(6 | 1)         # 7
print(1 << 4)        # 16
```

A shift count that is negative or at least 64 is `E4013`. There is **no XOR**
operator: `^` is exponentiation. `&&`/`||` are not operators (they lex as two
`&`/`|` tokens and fail to parse).

## Comparison and equality

`==` and `!=` compare any two values and yield a `bool`. Ordering (`<`, `<=`,
`>`, `>=`) is defined for numbers, strings, and booleans. Comparing
incomparable types is `E3001`. A `NaN` operand makes ordering comparisons
`false`.

```aura
print([1, 2] == [1, 2])    # true   (structural)
print("a" < "b")           # true
```

## Logic

`and`, `or`, and `not` are the logic operators; `!` is a lexical error and
`&&`/`||` are not operators. `and` and `or` short-circuit and always yield a
`bool`.

```aura
print(true and false)      # false
print(false or true)       # true
print(not 0)               # true  (0 is falsy)
```

## Assignment

| Operator | Meaning |
|---|---|
| `=` | assign |
| `+=` `-=` `*=` `/=` `%=` `^=` `&=` `\|=` `<<=` `>>=` | read, apply the binary operator, assign |

`^=` is exponentiation-assign, not XOR-assign. Every assignment operator
checks the target for mutation capability: assigning to an immutable binding
is `E2001`, and so is assigning through one (`xs[0] = v`, `s.f = v`).

## Operators Aura does not have

These are intentionally absent, not merely unimplemented:

* **No XOR.** `^` is exponentiation; there is no bitwise XOR operator.
* **No `++` / `--`.** There is no pre- or post-increment or decrement. Write
  the assignment explicitly: `x = x + 1` or `x += 1`.
* **No self-documenting or conversion f-string fields.** `{x=}`, `{x!r}`, and
  grouping flags like `{x:,}` are not part of Aura. An f-string does support a
  small format mini-language (`{x:.2f}`, `{n:>6}`, `{n:06d}`, `{n:x}`).

## Indexing and fields

* `base[i]` — a list at an integer index (negative allowed), a string at an
  integer character index, a map at a key of its key type (`string`, `int`, or
  `bool`), a struct at a string field.
* `base[i] = v` — mutate a list element, map entry, or struct field.
* `receiver.field` — a struct field, or a zero-argument method call on any other
  value.

Out-of-range indexing is `E4019`; a missing map key on `m[k]` is `E2003`.

## The pipeline

`|>` passes the left operand as the first argument of the right-hand call:

```aura
x |> f          # f(x)
x |> f(a)       # f(x, a)
x |> r.m(a)     # r.m(x, a)
```

A pipeline must stay on one line: a newline ends the statement.
