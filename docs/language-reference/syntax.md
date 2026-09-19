---
layout: default
title: "Concrete Syntax (forms and examples)"
parent: Aura Language Reference
nav_order: 11
---

[English](syntax.md) · [Português](syntax.pt_BR.md)

# Concrete Syntax (forms and examples)

**Status:** Stable · **Evidence:** `aura/parser/to_ast.py`, `docs/language-reference/grammar.md`,
`tests/test_parser_statements.py`, `tests/test_parser_expressions.py`

This document shows the **concrete form** of each construct with minimal
examples. The **formal rules** are in [grammar.md](grammar.md); the **tokens**
are in [lexical-structure.md](lexical-structure.md); the **meaning** is in the
domain documents. It does not repeat — it references.

> Every example below is the shape the parser accepts (verified by probe). A
> program executed with `aura run` must declare a top-level `def main()`; a
> module file needs none. Examples that *look* valid but do **not** compile are
> listed in [Anti-forms](#anti-forms-does-not-compile).

---

## Minimal program

An entry file declares `main`. The runtime invokes it; never write a trailing
`main()` call (`grammar.md` §2; `syntax.md`).

```aura
def main() {
  print("Hello, world")
}
```

`main` takes no parameters or a single `args` parameter; any other signature is
`E311`, and a missing `main` in an entry file is `E310`.

---

## Variables and constants

```aura
let name = "Alice"          // immutable
let mut counter = 0         // mutable
counter += 1
const PI = 3.14159          // must be initialised; never reassigned
const MAX: int = 100

let age: int = 30
let items: [int] = [1, 2, 3]
```

Destructuring and multiple assignment use tuple/list patterns, with an optional
rest element `...name`:

```aura
let (a, b) = (1, 2)
let [first, ...rest] = [1, 2, 3, 4]
let x, y = 1, 2
```

Modifiers (`public`, `private`, `protected`, `static`, `volatile`, `abstract`)
appear **before** the declaration keyword: `private let x = 1`, never
`let private x` (`grammar.md` §3).

---

## Types

```aura
int  float  str  bool  bytes  none       // primitives
[int]                                     // list
{str: int}                                // dict
{int}                                     // set
int | str                                 // union
str?                                      // optional
(int, int) -> int                         // function
{x: float, y: float}                      // structural
Box[int]                                  // type argument (brackets only)
```

Types are parsed for checking/documentation and erased at runtime. The `[T]`
bracket form is canonical; `<T>` is **removed** (`grammar.md` §4).

Type parameters and constraints:

```aura
class Box[T] {
  private let value: T

  public def get() -> T {
    return self.value
  }
}

def smallest[T: Comparable](items: [T]) -> T {
  return items[0]
}
```

---

## Functions

```aura
def greet(name) -> str {
  return "Hello, " + name + "!"
}

def add(a: int, b: int) -> int {
  return a + b
}

def square(x: int) -> int = x * x     // expression body
def noop() { }                        // no return type

def configure(level, prefix = "[log]") { }   // default parameter
def sum_all(*numbers) -> int { return 0 }    // variadic positional
def log(level, **context) { }                // variadic keyword
```

The body is always a brace block or a single `= expr`. `def` is the **only**
function keyword; `fn`, `fun`, `function` do not exist. `async def` declares an
async function (`grammar.md` §3.2).

---

## Lambdas

```aura
let double = x => x * 2
let square = (x) => x * x
let add = (a, b) => a + b
let none_ = () => 42
let block = (x) => {
  let doubled = x * 2
  return doubled + 1
}
```

A lambda parameter may be annotated: `(x: int) => x * 2` (`grammar.md` §6.4).

---

## Control flow

```aura
if temperature > 100 {
  print("Boiling!")
} else if temperature > 50 {
  print("Warm")
} else {
  print("Cold")
}

unless authenticated { redirect("/login") }

guard data != none else {
  print("No data")
  return
}

let label = condition ? "yes" : "no"     // ternary; `if` is also an expression
```

---

## Loops

```aura
for i in range(10) { print(i) }
for item in items { print(item) }
for i in 0..<10 step 2 { print(i) }      // range with step

while count < 5 { count += 1 }
until ready { wait(100) }
loop { if quit() { break } }

outer: for i in range(10) {
  inner: for j in range(10) {
    if i * j > 20 { break outer }
  }
}
```

`for` uses the `in` keyword and a pattern. Labels work with `for`, `while`,
`until` and `loop`; `break label` / `continue label` where no such loop is in
scope is `E318` (`grammar.md` §5.2; `syntax.md`1).

---

## Pattern matching

```aura
match status {
  case 0 { print("inactive") }
  case 1 { print("active") }
  case n if n > 100 { print("overflow") }
  case _ { print("unknown") }
}

let text = match x {                 // match as an expression
  case 1 -> "one"
  case _ -> "other"
}
```

Patterns include literals, bindings, `_` wildcard, dotted enum members
(`Color.RED`), tuple/list destructuring with `*rest`, constructor patterns
(`Some(x)`) and or-patterns (`1 | 2`). A `match` with no `case _` over a finite
domain warns `E109` (`grammar.md` §5.4; `syntax.md`2).

---

## Enums

Members are **comma-separated** (a trailing comma is allowed). Values are
optional and auto-number from the previous integer value.

```aura
enum Color { RED, GREEN, BLUE }

enum Status { Pending = "pending", Active = "active" }

let c = Color.RED            // access via dotted member
if status == Status.Active { process() }
```

> Members on separate lines **without commas** do not parse:
> `enum E { A B }` → `Expected '}' but got 'B'` (*probe*). Use commas.

---

## Classes

Header fields generate the constructor, a getter, and (for `mut`) a setter.
Default visibility is `private`; each header field needs a type or a default.

```aura
class User(name: str, mut age: int = 0, public id: int = 0) {
  public def greet() -> str {
    return "hi " + self.get_name()
  }
}

let u = User("ana", 30)       // Aura constructs with Type(args), no `new`
print(u.get_name())
u.set_age(31)
```

Body fields and a manual constructor (do not mix with a header):

```aura
class Point {
  public let x: int = 0
  public let y: int = 0

  public def new(x: int, y: int) {
    self.x = x
    self.y = y
  }

  public def distance() -> float {
    return (self.x ** 2 + self.y ** 2) ** 0.5
  }
}
```

Rules: every member needs an explicit visibility (`E307`); members must be
unique (`E301`); `def new` maps to Python's `__init__`; a class has exactly one
constructor style. Inheritance uses `extends` only:

```aura
class Animal {
  protected let name: str = ""

  public def speak() -> str { return "..." }
}

class Dog extends Animal {
  public def speak() -> str { return self.name + " says woof" }
}

class Model extends django.db.models.Model { }   // dotted base allowed
class C extends A, B { }                          // multiple bases
```

`class Dog(Animal)` and `implements` are not Aura. Overriding is implicit — there
is no `override` (`grammar.md` §3.3; `classes.md`).

### Abstract classes

```aura
abstract class Shape {
  public let name: str = "shape"

  public def describe() -> str { return "a " + self.name }
  public abstract def area() -> float
}

class Square extends Shape {
  public let side: float = 2.0
  public def area() -> float { return self.side * self.side }
}
```

`abstract def` has **no** body; `abstract` before `def` inside a `trait` is a
syntax error (`to_ast.py:1923-1928`; `classes.md`).

---

## Traits

```aura
trait Drawable {
  public def draw() -> void
  public def get_bounds() -> float
}

class Circle extends Drawable {
  private let radius: float = 0.0

  public def draw() -> void { print("circle") }
  public def get_bounds() -> float { return self.radius * 2 }
}
```

A body-less trait method is already abstract — no `abstract` keyword. Traits
extend with `extends` (`grammar.md` §3.4; `classes.md` (traits)).

---

## Decorators

Decorators attach to a `def` or a class; a decorator on a field is `E320`.

```aura
@staticmethod
public def max(a, b) -> int { return a }

class Rect {
  public @property
  def area() -> int { return 4 }

  public @classmethod
  def create(cls) { return cls() }
}
```

Member modifiers and decorators may appear in any order
(`@staticmethod public def f` ≡ `public @staticmethod def f`).

---

## Collections

```aura
let list = [1, 2, 3]
let set = {1, 2, 3}
let dict = {name: "Alice", age: 30}
let pair = (1, "hello")
let single = (1,)
let empty = ()

print(list[0])          // index
print(list[-1])         // negative index
print(list[1:3])        // slice
print(list[::2])        // slice with step

let squares = [x * x for x in range(10)]
let evens = [x for x in range(20) if x % 2 == 0]
let lengths = {w: len(w) for w in words}
let gen = (x for x in xs)
```

List spread and dict spread:

```aura
let combined = [*list1, *list2, extra]
let merged = {**defaults, **overrides}
```

---

## Calls and arguments

```aura
f(a, b)                       // positional
f(name: "Alice", age: 30)     // keyword
f(*args)                      // positional spread
f(**kwargs)                   // keyword spread
f(a, *rest, **kw)             // mixed
```

Struct init is sugar for a constructor call: a `{` after an uppercase identifier
is a struct literal.

```aura
let p = Point{x: 1, y: 2}
```

---

## Operators

Precedence (lowest to highest — `grammar.md` §6.1, matching the parser table
`to_ast.py:465-488`):

| Level | Operators |
|---|---|
| 1 | `=`, `+=`, `-=`, `*=`, `/=`, `%=`, `**=`, `&=`, `\|=`, `^=`, `<<=`, `>>=`, `??=`, `\|>` |
| 2 | `? :` (ternary) |
| 3 | `or` |
| 4 | `and` |
| 5 | `==` `!=` `<` `>` `<=` `>=` `in` `not in` `is` `is not` |
| 6 | `\|` |
| 7 | `^` |
| 8 | `&` |
| 9 | `<<` `>>` |
| 10 | `..` `..<` |
| 11 | `??` `?:` |
| 12 | `+` `-` |
| 13 | `*` `/` `%` `as` |
| 14 | `**` |
| 15 | unary `-` `+` `~` `not` `await` `...` |
| 16 | call, index, slice, member, safe-nav, struct-init |

```aura
let q = total / count                 // true division (never truncates)
let n = int(total / count)            // integer result via cast
let r = value as int                  // cast
let b = a ?? fallback                 // coalesce on none
let e = a ?: fallback                 // fallback on falsy
let c = user?.address?.city           // safe navigation
let i = list?[index]                  // safe index
let x = xs |> filter(f) |> map(g)     // pipe, left to right
let rng = 0..10                       // inclusive range
let rng2 = 0..<10                     // exclusive range
let open = 1..                        // infinite range
```

`is` / `is not` test identity; `x is none` is the null check. Comparing a
literal with `is` is rejected (`to_ast.py:2567-2577`).

---

## Statements

```aura
return value
throw ValueError("bad")
break
continue
break outer
continue outer
yield
yield x + 1
spawn work()
assert x == 1, "message"
```

`yield` is statement-level, not part of `expression`; `yield x + 1` yields the
whole sum (`grammar.md` §6.1).

---

## Blocks, with, errors

```aura
with open("f") as f {
  print(f)
}

async with client() as c { }      // __aenter__/__aexit__; only in async def

try {
  risky()
} catch TypeError {
  print("type")
} catch IOError as e {
  print(e)
} finally {
  cleanup()
}

let result = try { parse_int(s) } catch Error as e { 0 }   // try as expression
```

A `catch` clause has exactly one meaning per spelling (`syntax.md`5):

| Form | Meaning |
|---|---|
| `catch { }` | catch every exception (no binding) |
| `catch Type { }` | catch only `Type` |
| `catch Type as e { }` | catch only `Type`, bind to `e` |
| `catch as e { }` | catch every exception, bind to `e` |

A `try` requires at least one `catch` or a `finally`.

---

## Imports

```aura
import stdlib.math                    // module; reach members by path
import stdlib.math as m               // alias
from stdlib.math import sqrt, PI      // names
from stdlib.math import sqrt as root  // name alias
import stdlib.math { sqrt, PI }       // brace form ≡ from ... import ...
from stdlib.math import *             // wildcard
```

The brace form cannot be combined with `as` (`syntax.md`6).

### Python interop

A `py.` prefix marks a **host Python module** (`_split_python_prefix`,
`to_ast.py:494-505`). The prefix is stripped before code generation; the name
bound is the **last path segment**.

```aura
import py.re                          // binds `re`
import py.os.path                     // binds `path`
from py.math import sqrt, pi as PI    // binds `sqrt` and `PI`
```

A Python name that collides with an Aura keyword **must be aliased**; a bare
collision is rejected (`_reject_python_keyword_binding`, `to_ast.py:1832-1844`):

```aura
from py.re import type as re_type     // ok: aliased
// from py.re import type             // error: 'type' is a reserved Aura keyword
```

---

## Type aliases and modules

```aura
type UserId = int
type Point = {x: float, y: float}
type Pair[T] = [T]

module App.Services {
  export def public_function() -> int { return 42 }
  export const VERSION = "1.0.0"
  let mut cache = 0                   // private to the file
}

module Facade {
  export Components, Utils            // re-export from sibling files
}
```

Module members are private to the declaring file unless marked `export`;
`export` precedes a named declaration or introduces a re-export
(`grammar.md` §3.7; `modules.md`).

---

## Anti-forms (does NOT compile)

Each rejected spelling, with the pointed parser error and its canonical
replacement:

```aura
fn f() { }                  // ❌ 'fn' is not part of Aura; use 'def' instead
fun f() { }                 // ❌ 'fun' is not part of Aura; use 'def'
function f() { }            // ❌ 'function' is not part of Aura; use 'def'
let f = lambda x: x         // ❌ 'lambda' is not part of Aura; use '(x) => expr'
let c = new C()             // ❌ 'new' is not part of Aura; construct with 'Type(args)'
var x = 1                   // ❌ 'var' is not part of Aura; use 'let mut' or 'let'
foreach x in xs { }         // ❌ 'foreach' is not part of Aura; use 'for x in xs'
repeat { }                  // ❌ 'repeat' is not part of Aura; use 'loop { }' or 'until'
do { }                      // ❌ 'do' is not part of Aura; use 'loop { }' with 'break'
switch x { }                // ❌ 'switch' is not part of Aura; use 'match x { case ... }'
if a { } elif b { }         // ❌ 'elif' is not part of Aura; write 'else if'
if a { } elsif b { }        // ❌ 'elsif' is not part of Aura; write 'else if'
let x = null                // ❌ 'null' is not part of Aura; use 'none'
let x = !y                  // ❌ '!' is not part of Aura; use 'not'
let x = a && b              // ❌ '&&' is not part of Aura; use 'and'
let x = a || b              // ❌ '||' is not part of Aura; use 'or'
match x { case 1: f() }     // ❌ ':' is not Aura case syntax; write 'case 1 -> ...'
match x { case 1 => 1 }     // ❌ '=>' is not Aura case syntax; write 'case 1 -> ...'
class C(A) { }              // ❌ parenthesised base; write 'class C extends A'
class C implements D { }    // ❌ 'implements' is not Aura; use 'extends'
class Box<T> { }            // ❌ type parameters use brackets: 'Box[T]'
if x is "a" { }             // ❌ 'is' compares identity; use '=='
enum E { A B }              // ❌ members are comma-separated: 'enum E { A, B }'
```

### Anti-forms from other languages (does NOT exist)

| Do NOT write | Write instead | Evidence |
|---|---|---|
| `fn`, `fun`, `function` | `def` | `to_ast.py:816-818, 2647-2649` |
| `elif`, `elsif` | `else if` | `to_ast.py:819-822` |
| `var` (statement) | `let mut` / `let` | `to_ast.py:791, 801-804` |
| `foreach` | `for x in xs` | `to_ast.py:792` |
| `lambda` | `(x) => expr` | `to_ast.py:795, 2650-2653` |
| `new` | `Type(args)` | `to_ast.py:796, 2640-2646` |
| `repeat` | `loop { }` / `until` | `to_ast.py:797` |
| `switch` | `match x { case ... -> ... }` | `to_ast.py:798` |
| `do` | `loop { }` with `break` | `to_ast.py:799` |
| `null` | `none` | `to_ast.py:2637-2639` |
| `True` / `False` / `None` | `true` / `false` / `none` | `to_ast.py:533-537` |
| `!x` | `not x` | `to_ast.py:2435-2437` |
| `&&` / `\|\|` | `and` / `or` | `to_ast.py:2497-2502` |
| `case x:` / `case x =>` | `case x ->` | `to_ast.py:2310-2317` |
| `class C(A)` | `class C extends A` | `to_ast.py:1270` |
| `implements` | `extends` | `to_ast.py:1887-1891` |
| `Box<T>` | `Box[T]` | `to_ast.py:1112` |
| `volatily` | `volatile` | `to_ast.py:281-284` |
| `a ? b : c` is valid; `x is <literal>` | `==` / `!=` | `to_ast.py:2567-2577` |
