# Operators

## Precedence

From loosest to tightest binding:

```text
|>            pipeline (lowest)
or
and
==  !=
<  <=  >  >=
+  -
*  /  %
^             (right associative)
-  not        (prefix unary)
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

`and`, `or`, and `not` are the logic operators; `&&`, `||`, and `!` are lexical
errors. `and` and `or` short-circuit and always yield a `bool`.

```aura
print(true and false)      # false
print(false or true)       # true
print(not 0)               # true  (0 is falsy)
```

## Assignment

| Operator | Meaning |
|---|---|
| `=` | assign |
| `+=` `-=` `*=` `/=` | read, apply the binary operator, assign |

There is no `%=` or `^=`.

## Operators Aura does not have

These are intentionally absent, not merely unimplemented:

* **No bitwise operators.** `&`, `|`, `~`, `<<`, and `>>` do not exist. `&`
  and `~` are lexical errors, `|` is reserved for type unions, and a shift
  fails to parse. `^` is exponentiation, not XOR.
* **No `++` / `--`.** There is no pre- or post-increment or decrement. Write
  the assignment explicitly: `x = x + 1` or `x += 1`.
* **No f-string format mini-language.** An f-string interpolates `{expr}` and
  escapes braces with `{{`/`}}`; Python's `{x:spec}`, `{x=}`, and `{x!r}` forms
  are not part of Aura.

## Indexing and fields

* `base[i]` — a list at an integer index (negative allowed), a string at an
  integer character index, a map at a string key, a struct at a string field.
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
