---
layout: default
title: "09 — Collections"
parent: Learn Aura
nav_order: 19
---

[English](09-collections.md) | [Português](09-collections.pt_BR.md)

# 09 — Collections

> **Chapter goal:** store and process data with lists, dicts, sets and tuples.
> Reference:
> [`../language-reference/expressions.md`](../language-reference/expressions.md)
> §14, §16. Example: `../../examples/tour.aura`.

## Literals

```aura
let list = [1, 2, 3]
let set = {1, 2, 3}
let dict = {name: "Alice", age: 30}
let pair = (1, "hello")
let single = (42,)
let empty = ()
```

* Lists use `[ ... ]`.
* Sets use `{ ... }` with no colon.
* Dicts use `{ key: value }`. Access by index `d["k"]` or by member `d.k` both
  work.
* Tuples need the comma: `(42,)` is a one-element tuple; `(42)` is just `42`.

A trailing comma is allowed everywhere. Dicts preserve insertion order (CPython
3.7+).

## Indexing and slicing

```aura
let numbers = [10, 20, 30, 40]

print(numbers[0])       // 10
print(numbers[-1])      // 40 — negative index
print(numbers[1:3])     // [20, 30]
print(numbers[::2])     // [10, 30]
```

A **range used as an index is a slice**:

```aura
print(numbers[0..<2])   // [10, 20] — exclusive upper bound
print(numbers[0..2])    // [10, 20, 30] — inclusive
print(numbers[1..])     // [20, 30, 40] — open end
```

## Length, membership and mutation

```aura
let numbers = [1, 2, 3]
numbers.add(4)               // append
print(numbers.length())      // 4
print(numbers.size())        // 4
print(numbers.contains(2))   // true
print(numbers.is_empty())    // false
print(2 in numbers)          // true
```

`.length()` / `.size()` → `len(x)`; `.contains(x)` → `x in x`; `.add(x)` →
`append`. A **user-declared** method with one of those names wins over the
convenience.

Unknown member calls pass through to Python verbatim: `.append`, `.pop`,
`.sort`, `.reverse`, `.insert`, `.remove`, `.index`, `.count`, `.extend`,
`.clear`.

## Dicts and sets

```aura
let user = {name: "Alice", age: 30}
print(user["name"])          // Alice
print(user.age)              // 30

let unique = {1, 2, 2, 3, 3, 3}
print(unique)                // {1, 2, 3}
```

Dict keys can be written unquoted when they are identifiers: `{name: "Alice"}`.

## Spread

```aura
let list1 = [1, 2]
let list2 = [3, 4]
let combined = [*list1, *list2, 5]     // [1, 2, 3, 4, 5]

let defaults = {a: 1, b: 2}
let overrides = {b: 9}
let merged = {**defaults, **overrides}  // {a: 1, b: 9}
```

A bare spread in a value position is a parse error; spread is allowed only in a
literal or a call argument.

## Comprehensions

Comprehensions build a collection from an iterable, with optional filters:

```aura
let squares = [x * x for x in range(6)]
let evens = [x for x in range(20) if x % 2 == 0]
let lengths = {w: w.length() for w in ["ab", "xyz"]}
let unique_mods = {x % 3 for x in range(10)}
```

```text
[0, 1, 4, 9, 16, 25]
[0, 2, 4, 6, 8, 10, 12, 14, 16, 18]
{'ab': 2, 'xyz': 3}
{0, 1, 2}
```

Multiple `for` and `if` clauses are supported and preserve source order.
Comprehensions are eager; a generator expression `(e for x in xs)` is lazy.

## Strings

Strings are Unicode and immutable, with Aura-named convenience methods that map
to Python:

```aura
let text = "Hello, World"

print(text.to_upper())          // HELLO, WORLD
print(text.to_lower())          // hello, world
print(text.starts_with("He"))   // true
print(text.ends_with("ld"))     // true
print(text.trim())              // trims whitespace
print(text.slice(1, 4))         // ell
print(text.char_at(0))          // H
print(text.length())            // 12
```

Aura aliases include `starts_with`/`ends_with`, `to_upper`/`to_lower`,
`trim`/`trim_left`/`trim_right`, `index_of`, `is_alpha`, `is_digit` and more.

## Passing collections to functions

Collections are **reference** values: mutating one inside a function is visible
to the caller, while reassigning the parameter is not.

```aura
def mutate(xs) { xs.add(9) }
def reassign(x) { x = 9 }

def main() {
  let a = [1]
  mutate(a)
  print(a)          // [1, 9]

  let n = 1
  reassign(n)
  print(n)          // 1
}
```

## What you learned

* Lists, dicts, sets, tuples; indexing, slices, and range-as-slice.
* Length/membership/mutation conveniences and Python passthrough.
* Spreads and comprehensions.
* Collections are references; numbers and strings are values.

## Next step

[Lambdas and Functional Programming →](10-lambdas-and-functional.md)
