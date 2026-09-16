# Aura Language Reference

Complete syntax reference for the Aura programming language. Aura transpiles to Python.

---

## Table of Contents

1. [Program Structure](#1-program-structure)
2. [Lexical Elements](#2-lexical-elements)
3. [Variables and Constants](#3-variables-and-constants)
4. [Types](#4-types)
5. [Functions](#5-functions)
6. [Lambdas](#6-lambdas)
7. [Classes](#7-classes)
8. [Traits](#8-traits)
9. [Type Aliases and Modules](#9-type-aliases-and-modules)
10. [Control Flow](#10-control-flow)
11. [Loops](#11-loops)
12. [Pattern Matching](#12-pattern-matching)
13. [Expressions and Operators](#13-expressions-and-operators)
14. [Collections](#14-collections)
15. [Error Handling](#15-error-handling)
16. [Imports](#16-imports)
17. [Macros and Decorators](#17-macros-and-decorators)
18. [Comments](#18-comments)

---

## 1. Program Structure

An Aura program is a sequence of declarations, statements, and imports:

```aura
import stdlib.math { sqrt, PI }

const GREETING = "Hello"

def greet(name) -> str {
  return GREETING + " " + name
}

print(greet("world"))
```

---

## 2. Lexical Elements

### Identifiers

```
identifier ::= [a-zA-Z_] [a-zA-Z0-9_]*
```

- Must start with a letter or underscore
- Case-sensitive
- Convention: `snake_case` for variables/functions, `PascalCase` for classes

### Keywords

```
async      break      case       catch      class      const
continue   def        else       finally    for        from
guard      if         import     in         is         let
loop       match      module     mut        none       null
return     self       static     super      trait      true
false      try        type       unless     until      while
with       assert
```

### Literals

```aura
// Integers
42
1_000_000
0xFF        // hex
0o755       // octal
0b1010      // binary

// Floats
3.14
2.5e10
1.5e-5

// Strings
"hello"
'world'
f"Value: {x}"
"""
Multi-line
string
"""

// Booleans
true
false

// Null
none
null
```

---

## 3. Variables and Constants

### Immutable variables

```aura
let name = "Alice"
let age = 30
```

### Mutable variables

```aura
let mut counter = 0
counter += 1
```

### Constants

```aura
const PI = 3.14159
const MAX_SIZE: int = 100
```

### Type annotations

```aura
let name: str = "Alice"
let age: int = 30
let pi: float = 3.14159
let active: bool = true
let items: [int] = [1, 2, 3]
let user: {name: str, age: int} = {name: "Alice", age: 30}
```

### Multiple assignment

```aura
let mut a, b = 0, 1
let x, y, z = 1, 2, 3
```

### Destructuring

```aura
let [first, second, ...rest] = [1, 2, 3, 4, 5]
// first = 1, second = 2, rest = [3, 4, 5]
```

---

## 4. Types

### Primitive types

| Type    | Description            |
|---------|------------------------|
| `int`   | Integer numbers        |
| `float` | Floating-point numbers |
| `str`   | Unicode strings        |
| `bool`  | `true` or `false`      |
| `bytes` | Byte sequences         |
| `none`  | Null/None value        |

### Collection types

```aura
[int]                          // List of integers
{str: int}                     // Dictionary
{int}                          // Set
```

### Generic types

Generics use square brackets:

```aura
class Box[T] {
  let value: T

  def new(value: T) {
    self.value = value
  }

  def get() -> T {
    return self.value
  }
}

def map[T, R](fn: (T) -> R, items: [T]) -> [R] {
  return [fn(item) for item in items]
}
```

### Union types

```aura
let value: int | str = 42
int | float | none
```

### Function types

```aura
let handler: (int, int) -> int = add
```

### Optional types

```aura
let name: str? = get_name()
```

### Structural types

```aura
let point: {x: float, y: float} = {x: 1.0, y: 2.0}
```

---

## 5. Functions

### Basic function

```aura
def greet(name) -> str {
  return "Hello, " + name + "!"
}
```

### Function with type annotations

```aura
def add(a: int, b: int) -> int {
  return a + b
}
```

### Expression body

```aura
def square(x: int) -> int = x * x
def double(x) = x * 2
```

### Default parameters

```aura
def greet(name, greeting = "Hello") -> str {
  return greeting + ", " + name + "!"
}

greet("Alice")            // "Hello, Alice!"
greet("Bob", "Hi")        // "Hi, Bob!"
```

### Named arguments

```aura
def create_user(name, age, email) {
  return {name: name, age: age, email: email}
}

let user = create_user(name: "Alice", age: 30, email: "alice@example.com")
```

### Variadic parameters

```aura
def sum_all(*numbers) -> int {
  return sum(numbers)
}

def log(level, **context) {
  print(f"[{level}] {context}")
}
```

### Recursive functions

```aura
def factorial(n) -> int {
  if n <= 1 { return 1 }
  return n * factorial(n - 1)
}
```

### Multiple return values

```aura
def swap(a, b) {
  return b, a
}

let x, y = swap(1, 2)
```

---

## 6. Lambdas

### Arrow functions

```aura
// Single parameter, no parens
let double = x => x * 2
print(double(5))  // 10

// Single parameter, with parens
let square = (x) => x * x
print(square(4))  // 16

// Multiple parameters
let add = (a, b) => a + b
print(add(3, 4))  // 7

// No parameters
let get_answer = () => 42
print(get_answer())  // 42
```

### Block body

```aura
let compute = (x) => {
  let doubled = x * 2
  return doubled + 1
}
print(compute(10))  // 21
```

### Closures

```aura
let make_adder = (n) => (x) => x + n
let add10 = make_adder(10)
print(add10(5))  // 15
```

### Lambdas with map/filter

```aura
let numbers = [1, 2, 3, 4, 5]
let doubled = map(numbers, (x) => x * 2)
let evens = filter(numbers, (x) => x % 2 == 0)
```

---

## 7. Classes

### Basic class

```aura
class Point {
  let x: int = 0
  let y: int = 0

  def new(x: int, y: int) {
    self.x = x
    self.y = y
  }

  def distance() -> float {
    return (self.x ** 2 + self.y ** 2) ** 0.5
  }
}

let p = Point(3, 4)
print(p.distance())  // 5.0
```

> **Note:** `def new(...)` maps to Python's `__init__`. Instantiate with `Point(args)`, not `Point.new(args)`.

### Inheritance

```aura
class Animal {
  let name: str = ""

  def new(name: str) {
    self.name = name
  }

  def speak() -> str {
    return "..."
  }
}

class Dog(Animal) {
  def speak() -> str {
    return self.name + " says woof"
  }
}

let d = Dog("Rex")
print(d.speak())  // Rex says woof
```

The transformer automatically inserts `super().__init__()` in the constructor when a base class exists.

### @property

```aura
class Rect {
  let w: int = 0
  let h: int = 0

  def new(w: int, h: int) {
    self.w = w
    self.h = h
  }

  @property
  def area() -> int {
    return self.w * self.h
  }
}

let r = Rect(4, 5)
print(r.area)  // 20 (accessed as property, no parentheses)
```

### @staticmethod

```aura
class MathUtil {
  @staticmethod
  def max(a, b) -> int {
    if a > b { return a }
    return b
  }
}

print(MathUtil.max(3, 9))  // 9
```

### @classmethod

```aura
class Factory {
  let kind: str = ""

  def new(kind: str) {
    self.kind = kind
  }

  @classmethod
  def create(cls, kind) {
    return cls(kind)
  }
}

let f = Factory.create("custom")
print(f.kind)  // custom
```

---

## 8. Traits

```aura
trait Drawable {
  def draw();
  def get_bounds();
}

class Circle implements Drawable {
  let radius: float = 0.0

  def draw() {
    print(f"Drawing circle with radius {self.radius}")
  }

  def get_bounds() {
    return self.radius * 2
  }
}
```

A trait transpiles to a base class, and `implements` becomes inheritance.
Multiple traits can be listed: `class C implements A, B`.

### Visibility

```aura
class Account {
  let private balance: float = 0.0
  let protected account_id: str = ""
  let public name: str = ""
}
```

Visibility is enforced by name mangling **inside classes** only:

| Modifier | Python name | Meaning |
|----------|-------------|---------|
| `public` (default) | `name` | No mangling |
| `protected` | `_name` | Single-underscore convention |
| `private` | `__name` | Python name mangling (`_Class__name`) |

At module or local scope, `private`/`protected` are compile-time metadata
only; names are **not** mangled, so later references keep working.

---

## 9. Type Aliases and Modules

### Type aliases

```aura
type UserId = int
type Point = {x: float, y: float}
```

Type aliases are erased at compile time but a runtime name is still emitted
(`UserId = int`, structural types map to `dict`), so the name can be
referenced.

### Modules

```aura
module MyLib {
  export def public_function() {
    return 42
  }

  def private_function() {
    // not exported
  }

  export const VERSION = "1.0.0"
}

print(MyLib.public_function())
print(MyLib.VERSION)
```

A module transpiles to a namespaced class whose functions are static.
`export` is accepted but has no Python equivalent, so it is ignored.

---

## 10. Control Flow

### if/else

```aura
if temperature > 100 {
  print("Boiling!")
} else if temperature > 50 {
  print("Warm")
} else {
  print("Cold")
}
```

### unless (inverted if)

```aura
unless authenticated {
  redirect("/login")
}
```

### guard

```aura
def process(data) {
  guard data != null else {
    print("No data")
    return
  }
  // data is guaranteed non-null here
  print(data)
}
```

### switch/match

```aura
match status {
  case 0 { print("inactive") }
  case 1 { print("active") }
  case n if n > 100 { print("overflow") }
  case _ { print("unknown") }
}
```

---

## 11. Loops

### for

```aura
for i in range(10) {
  print(i)
}

// With step inside range
for i in range(0, 20, 2) {
  print(i)
}

// `step` keyword forms
for i in range(0, 10) step 2 { print(i) }
for i in 0..10 step 2 { print(i) }

// Iterate over list
for item in items {
  print(item)
}
```

### while

```aura
let mut count = 0
while count < 5 {
  count += 1
}
```

### until (inverted while)

```aura
until ready {
  wait(100)
}
```

### loop (infinite)

```aura
loop {
  let input = read_input()
  if input == "quit" { break }
}
```

### break and continue

```aura
for i in range(100) {
  if i == 5 { break }
  if i % 2 == 0 { continue }
  print(i)
}
```

### Labeled loops

Use labels to break/continue nested loops:

```aura
outer: for i in range(10) {
  inner: for j in range(10) {
    if i * j > 20 {
      break outer
    }
    print(f"{i}, {j}")
  }
}
```

Labels work with `loop`, `for`, and `while`:

```aura
retry: loop {
  let input = read_input()
  if input == "quit" { break retry }
  process(input)
}
```

---

## 12. Pattern Matching

### Basic matching

```aura
match command {
  case "quit" { print("Goodbye!") }
  case "help" { print("Help!") }
  case _ { print("Unknown command") }
}
```

### Guard conditions

```aura
match value {
  case 0 { print("zero") }
  case 1 { print("one") }
  case n if n > 100 { print("big") }
  case n if n > 0 { print("positive") }
  case _ { print("other") }
}
```

### Destructuring patterns

```aura
// List patterns
match numbers {
  case [] { print("Empty") }
  case [x] { print(f"Single: {x}") }
  case [first, *rest] { print(f"First: {first}") }
}

// Binding variables
match response {
  case "ok" { process() }
  case err if err.starts_with("error") { handle_error(err) }
  case _ { log(response) }
}
```

---

## 13. Expressions and Operators

### Operator precedence (highest to lowest)

| Prec | Operator | Description |
|------|----------|-------------|
| 1 | `**` | Exponentiation |
| 2 | `+x`, `-x`, `~x` | Unary plus, minus, bitwise NOT |
| 3 | `*`, `/`, `%` | Multiplication, division, modulo |
| 4 | `+`, `-` | Addition, subtraction |
| 5 | `<<`, `>>` | Bitwise shifts |
| 6 | `&` | Bitwise AND |
| 7 | `^` | Bitwise XOR |
| 8 | `|` | Bitwise OR |
| 9 | `<`, `>`, `<=`, `>=` | Comparisons |
| 10 | `==`, `!=`, `is`, `in` | Equality, identity, membership |
| 11 | `not` | Logical NOT |
| 12 | `and` | Logical AND |
| 13 | `or` | Logical OR |
| 14 | `?:` | Elvis operator |
| 15 | `??` | Null coalescing |
| 16 | `? :` | Ternary conditional |
| 17 | `\|>` | Pipe |
| 18 | `=`, `+=`, etc. | Assignment |

### Ternary

```aura
let result = condition ? "yes" : "no"
```

### Elvis operator

```aura
let value = potential_null ?: default_value
```

### Null coalescing

```aura
let display_name = user?.name ?? "Anonymous"
name ??= "Default Name"
```

### Pipe operator

```aura
let result = [1, 2, 3, 4, 5]
  |> filter((x) => x > 2)
  |> map((x) => x * 10)
  |> reduce((a, b) => a + b, 0)
```

### Range operators

```aura
1..10          // inclusive: 1, 2, ..., 10
0..<100        // exclusive: 0, 1, ..., 99
1..            // infinite
0..100 step 5  // 0, 5, 10, ..., 100
```

### Null-safe navigation

```aura
let city = user?.address?.city
let item = list?[index]
```

### Spread operator

```aura
let combined = [*list1, *list2, extra]
let merged = {**defaults, **overrides}
```

### List comprehensions

```aura
let squares = [x * x for x in range(10)]
let evens = [x for x in range(20) if x % 2 == 0]
let matrix = [[i * j for j in range(3)] for i in range(3)]
```

### Dict comprehensions

```aura
let lengths = {word: len(word) for word in words}
```

---

## 14. Collections

### Lists

```aura
let numbers = [1, 2, 3, 4, 5]
let mixed = [1, "two", 3.0, true]
let empty = []
print(numbers[0])  // 1
print(len(numbers))  // 5
```

### Dictionaries

```aura
let user = {name: "Alice", age: 30, active: true}
print(user.name)       // Alice
print(user["age"])     // 30

let config = {"max-size": 100, "timeout": 30}
```

### Sets

```aura
let unique = {1, 2, 3, 2, 1}  // {1, 2, 3}
print(len(unique))  // 3
```

### Tuples

```aura
let empty = ()
let single = (42,)
let pair = (1, "hello")
```

---

## 15. Error Handling

### try/catch/finally

```aura
try {
  let result = risky_operation()
} catch e {
  print("Error: " + e)
}

// Typed exceptions
try {
  parse("invalid")
} catch SyntaxError {
  print("Syntax error occurred")
}

// Full form
try {
  let file = open("data.txt")
  process(file)
} catch IOError {
  print("File not found")
} finally {
  print("Cleanup complete")
}
```

### Throwing exceptions

```aura
def validate(age) {
  guard age >= 0 else {
    throw ValueError("age cannot be negative")
  }
  return true
}
```

### Try as expression

`try` can be used as an expression that returns a value:

```aura
let result = try {
  parse_int(input)
} catch e {
  0
}

// With finally
let data = try {
  read_config(path)
} catch e {
  "default"
} finally {
  cleanup()
}
```

---

## 16. Imports

Aura supports three import forms, all backed by the standard library:

```aura
// 1. Import a module (access members through the module name)
import stdlib.math
print(stdlib.math.sqrt(16))  // 4.0

// 2. Import a module under an alias
import stdlib.math as m
print(m.sqrt(25))  // 5.0

// 3. Import specific names (optionally with an alias)
from stdlib.math import sqrt, PI
from stdlib.math import sqrt as root
print(sqrt(36))  // 6.0

// Brace form: equivalent to a `from ... import ...`
import stdlib.math { sqrt, PI }
print(PI)  // 3.141592653589793
```

The brace form (`import module { a, b }`) is a convenience that compiles to
`from module import a, b`. It cannot be combined with `as`; use one of the
forms above. There is no `::` namespace separator and no `*` in the brace
form — use `from stdlib.module import *` if a wildcard is required.

---

## 17. Macros and Decorators

Built-in decorators are available as runtime macros:

```aura
// @debug - prints entry/exit with arguments and return value
@debug
def calculate(x, y) -> int {
  return x + y
}

// @timeit - measures execution time
@timeit
def slow_function() {
  // ... work ...
}

// @memoize - caches results
@memoize
def fibonacci(n) -> int {
  if n < 2 { return n }
  return fibonacci(n - 1) + fibonacci(n - 2)
}

// @cache(maxsize=N) - caches with LRU eviction
@cache(maxsize=256)
def expensive_lookup(key) {
  return compute(key)
}

// @must_return - asserts function returns a value
@must_return
def compute(x) -> int {
  return x * 2
}

// @deprecated(message) - warns on use
@deprecated("Use new_function instead")
def old_function() {
  // ...
}
```

---

## 18. Comments

```aura
// Single-line comment

/*
   Multi-line comment
   spans multiple lines
*/
```

> **Note:** `//` always starts a line comment. Aura deliberately has no floor
> division operator; use integer casting on a float division instead:
> `let q = int(total / count)`.

---

## String Methods

Strings support direct method calls:

```aura
let s = "Hello, World"

s.to_upper()         // "HELLO, WORLD"
s.to_lower()         // "hello, world"
s.trim()             // removes whitespace
s.trim_left()        // removes leading whitespace
s.trim_right()       // removes trailing whitespace
s.starts_with("He")  // true
s.ends_with("ld")    // true
s.contains("ell")    // true
s.index_of("ell")    // 1
s.slice(0, 5)        // "Hello"
s.length()           // 13
s.is_alpha()         // false
s.is_digit()         // false
```

---

## Performance Tips

- Use `@memoize` or `@cache` for expensive pure functions
- Prefer `range()` over creating large lists
- Use list comprehensions instead of map/filter for simple cases
- Use the `|>` pipe operator for readable data transformations
- Avoid deep class hierarchies; prefer composition

---

## 19. Enums

Enums define a set of named constants:

```aura
enum Direction {
  North
  South
  East
  West
}

let facing = Direction.North
print(facing)  // Direction.North
```

Enums with values:

```aura
enum Status {
  Pending = "pending"
  Active = "active"
  Disabled = "disabled"
}

let current = Status.Active
```

Enums support pattern matching:

```aura
enum Shape {
  Circle(radius: float)
  Rectangle(width: float, height: float)
}

def area(shape) -> float {
  match shape {
    case Shape.Circle(r) { return 3.14 * r * r }
    case Shape.Rectangle(w, h) { return w * h }
  }
}
```

Enum comparison:

```aura
if status == Status.Active {
  process()
}
```

---

## 20. Async / Await

Aura supports async functions and await expressions:

```aura
async def fetch_data(url) -> str {
  let response = await http_get(url)
  return response.body
}

async def main() {
  let data = await fetch_data("https://example.com")
  print(data)
}
```

Running async code with `aura run` automatically wraps in an event loop.

Multiple concurrent awaits:

```aura
async def parallel() {
  let a = await fetch("url1")
  let b = await fetch("url2")
  return [a, b]
}
```

---

## 21. Standard Library

Aura ships with a standard library of useful modules.

### stdlib.json

JSON parsing and serialization:

```aura
import stdlib.json

let data = stdlib.json.loads('{"name": "Aura"}')
let text = stdlib.json.dumps(data, indent=2)
let pretty = stdlib.json.pretty(data)
let valid = stdlib.json.is_valid('{"ok": true}')
```

### stdlib.time

Time and date utilities:

```aura
import stdlib.time

let t = stdlib.time.now()        // timestamp (seconds)
let ms = stdlib.time.now_ms()    // timestamp (milliseconds)
let iso = stdlib.time.iso()      // ISO 8601 string
stdlib.time.sleep(0.5)           // sleep 500ms
let elapsed = stdlib.time.elapsed(start)
```

### stdlib.io

File and directory operations:

```aura
import stdlib.io

stdlib.io.write("out.txt", "hello")
let content = stdlib.io.read("out.txt")
let lines = stdlib.io.read_lines("data.csv")

if stdlib.io.exists("config.json") {
  let cfg = stdlib.io.read("config.json")
}

stdlib.io.mkdir("output")
stdlib.io.rm("temp.txt")
let files = stdlib.io.ls(".")
```

### stdlib.math

Mathematical functions (30+):

```aura
import stdlib.math { sqrt, PI, sin, cos, floor, ceil }

let r = sqrt(16.0)     // 4.0
let c = cos(PI)         // -1.0
let n = floor(3.7)      // 3
```

### stdlib.string

String manipulation (40+):

```aura
import stdlib.string

let upper = stdlib.string.upper("hello")   // "HELLO"
let parts = stdlib.string.split("a,b,c", ",")  // ["a","b","c"]
```

### stdlib.collections

List, dict, set utilities:

```aura
import stdlib.collections

let mapped = stdlib.collections.list_map([1,2,3], (x) => x * 2)
```

### stdlib.itertools

Iteration utilities:

```aura
import stdlib.itertools

let pairs = stdlib.itertools.combinations([1,2,3], 2)
```
