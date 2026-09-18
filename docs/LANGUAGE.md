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

An Aura program is a sequence of declarations, statements, and imports.

An **entry file** — one executed with `aura run` — must declare a top-level
`main`. The runtime invokes `main` for you: never write a trailing `main()`
call.

```aura
import stdlib.math { sqrt, PI }

const GREETING = "Hello"

def greet(name) -> str {
  return GREETING + " " + name
}

def main() {
  print(greet("world"))
}
```

`main` takes no parameters, or a single `args` parameter that receives the
command-line arguments:

```aura
def main(args: [string]) {
  print(f"{args.length()} argument(s)")
}
```

```bash
$ aura run app.aura alpha beta
2 argument(s)
```

`main` may return an `int` to set the process exit code. It may also be
`async`; the runtime awaits it inside an event loop:

```aura
async def main() {
  print(await fetch_data("https://example.com"))
}
```

A file **imported as a module** needs no `main`: any Aura file can expose
functions, classes and constants to another one through `import`. Omitting
`main` in a file run directly is a compile error (`E310`), and a `main` with any
other signature is rejected (`E311`).

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
async      await      break      case       catch      class
const      continue   def        else       enum       export
extends    false      finally    for        from       guard
if         import     in         is         let        loop
match      module     mut        none       private    protected
public     return     self       spawn      static     super
throw      trait      true       try        type       unless
until      volatile   while      with       yield      assert
```

> `null`, `fn`, `init`, `volatily`, `implements`, `!`, `&&`, `||` and `<T>` are
> **not** part of Aura. See [GRAMMAR.md](GRAMMAR.md) for the complete grammar and the list of
> removed spellings.

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
  private let value: T

  public def new(value: T) {
    self.value = value
  }

  public def get() -> T {
    return self.value
  }
}

def map[T, R](fn: (T) -> R, items: [T]) -> [R] {
  return [fn(item) for item in items]
}
```

### Generic constraints

A type parameter may declare a constraint after a colon. The constraint must be
a builtin type or a class/trait declared in the same program (a union of such
names is also accepted); anything else is reported as `E110`:

```aura
class Comparable {
  public def compare(other: Comparable) -> int {
    return 0
  }
}

def smallest[T: Comparable](items: [T]) -> T {
  let mut best = items[0]
  for item in items {
    if item.compare(best) < 0 {
      best = item
    }
  }
  return best
}

// A union constraint accepts any of the listed types.
def display[T: int | str](value: T) -> str {
  return str(value)
}
```

Constraints are compile-time only: the emitted Python is unchanged, and the
checker verifies that every declared constraint resolves.

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

### Class header fields

Declare a class's fields in its header. Each field becomes an instance field,
a constructor parameter, and a getter; a `mut` field also gets a setter:

```aura
class User(name: str, mut age: int = 0, public id: int = 0) {
  public def greet() -> str {
    return "hi " + self.get_name()
  }
}

let u = User("ana", 30)
print(u.get_name())   // ana
print(u.get_age())    // 30
u.set_age(31)
print(u.get_age())    // 31
print(u.id)           // 0 (public field, accessed directly)
```

Rules for the header:

* The default visibility is **`private`**. Write `public` or `protected` per
  field: `class U(private a: int, public b: int)`.
* A field is **immutable by default**, so it gets a getter only. Write `mut`
  (or `let mut`) to make it mutable and get a setter:
  `class U(name: str, mut age: int = 0)`.
* Fields may have defaults (`age: int = 0`). A field without a default cannot
  follow one with a default, matching function parameters.
* Every field needs a type annotation or a default; `class U(a)` is a syntax
  error, so a bare name in parentheses is never a base class.
* Private/protected fields are stored mangled; the getter/setter is the
  supported way to reach them from outside.
* A field declared in the header must not also be declared in the body.

A class may mix a header with body fields, methods, and constants:

```aura
class Counter(start: int = 0) {
  private let mut count: int = 0

  public def new(start: int) {
    self.count = start
  }

  public def increment() -> int {
    self.count = self.count + 1
    return self.count
  }
}
```

> **Note:** `def new(...)` maps to Python's `__init__`. Instantiate with
> `User(args)`, not `User.new(args)`. When a class declares `def new`, that
> constructor wins over the generated one, but its accessors are still
> generated.

### Basic class

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

let p = Point(3, 4)
print(p.distance())  // 5.0
```

### Inheritance

Inheritance uses `extends` — the only spelling. A class may extend several
bases, and base names may be dotted:

```aura
class Animal {
  protected let name: str = ""

  public def new(name: str) {
    self.name = name
  }

  public def speak() -> str {
    return "..."
  }
}

class Dog extends Animal {
  public def speak() -> str {
    return self.name + " says woof"
  }
}

let d = Dog("Rex")
print(d.speak())  // Rex says woof
```

`class Dog(Animal)` and `implements` are **not** Aura: use `extends`. Traits
are extended the same way.

With header fields, a subclass declares only its **own** new fields; inherited
fields are passed to the parent by name:

```aura
class User(name: str, mut age: int = 0) {
}

class Admin extends User(email: str) {
}

let a = Admin(email: "a@x.com", name: "bob")
print(a.get_name())   // bob
print(a.get_email())  // a@x.com
```

The transformer forwards the remaining arguments to `super().__init__(**kwargs)`
when a base class exists.

### @property

```aura
class Rect {
  private let w: int = 0
  private let h: int = 0

  public def new(w: int, h: int) {
    self.w = w
    self.h = h
  }

  public @property
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
  public @staticmethod
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
  public let kind: str = ""

  public def new(kind: str) {
    self.kind = kind
  }

  public @classmethod
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
  public def draw() -> void
  public def get_bounds() -> float
}

class Circle extends Drawable {
  private let radius: float = 0.0

  public def draw() -> void {
    print(f"Drawing circle with radius {self.radius}")
  }

  public def get_bounds() -> float {
    return self.radius * 2
  }
}
```

A trait transpiles to a base class, and `extends` becomes inheritance.
Multiple traits can be listed: `class C extends A, B`.

A method declared without a body is abstract: the trait compiles it to an
`@abstractmethod`, and a concrete class that does not implement every abstract
method it inherits is rejected at compile time (`E309`):

```aura
trait Shape { public def area() -> float }

class Square extends Shape {
  public let side: float = 2.0
  public def area() -> float { return self.side * self.side }
}

// class Bad extends Shape { }   // E309: must implement 'area'
```

Traits may also extend other traits with `extends`. Abstract methods are
inherited transitively:

```aura
trait Greeter { public def greet() -> str }
trait Loud extends Greeter { public def shout() -> str }

class Person extends Loud {
  public def greet() -> str { return "hi" }
  public def shout() -> str { return "HEY" }
}
```

### Visibility

```aura
class Account {
  private let balance: float = 0.0
  protected let account_id: str = ""
  public let name: str = ""
}
```

Every class and trait member (field, method or nested class) **must** declare
its visibility explicitly: `public`, `private` or `protected`. Omitting it is a
compile error (`E307`).

Modifiers come **before** `let`/`def`/`class`; `let private balance` is a syntax
error.

Enforcement happens at **compile time and at runtime**:

* The rule checker rejects an access to a non-public member from outside the
  class (`E308`). It resolves the class of a `self`/`cls` access and of simple
  `let x = Class(...)` instances.
* The transpiler emits owner-aware mangled names, so the restriction also holds
  at runtime even for code the checker cannot prove.

| Modifier | Runtime name | Accessible from |
|----------|--------------|-----------------|
| `public` | `name` | anywhere |
| `protected` | `_name` | the declaring class and its subclasses |
| `private` | `_<DefiningClass>__name` | only the declaring class |

A private member is **not** visible in a subclass: a subclass may only reach it
through a `public`/`protected` method or through the generated accessor.

```aura
class Counter {
  private let mut count: int = 0
  public def increment() {
    self.count = self.count + 1
  }
}

let c = Counter()
c.increment()
print(c.get_count())   // auto-generated getter
```

For every instance field, `get_<name>()` is generated automatically. A
`set_<name>(value)` is generated only for a **mutable** field (`mut`/`let mut`),
so an immutable field has no setter at all. A method you declare with the same
name wins over the generated one.

A plain `let` field is immutable, exactly like `let` at module and local scope;
use `let mut` (or `mut in a class header) when the value must change.

At module or local scope, `private`/`protected` are compile-time metadata only;
names are **not** mangled, so later references keep working.

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
  guard data != none else {
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

### Matching enums and exhaustiveness

Match an enum value with a dotted member pattern (`Color.RED`), which refers to
the constant rather than binding a new name:

```aura
enum Color { RED, GREEN, BLUE }

def name(c: Color) -> str {
  match c {
    case Color.RED { return "red" }
    case Color.GREEN { return "green" }
    case Color.BLUE { return "blue" }
  }
}
```

A `match` over a value with a known finite domain should handle every case or
provide a catch-all. The checker reports `E109` (a warning, so it never fails a
build) when a `match` over `bool`, an enum, or a scalar has no `case _`
fallback:

```aura
match count {
  case 1 { print("one") }
  // warning E109: no case handles unlisted values
}

match count {
  case 1 { print("one") }
  case _ { print("many") }   // exhaustive
}
```

A guarded fallback (`case _ if cond`) does not count as exhaustive, because the
guard may reject the value. `case name` binds the value and does count.

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

### Custom exception types

The exception root is `Error`, which needs no import. Define your own error
types with `extends` and pass a message to the parent constructor with
`super(message)`:

```aura
class AppError extends Error {
  public def new(message: str) {
    super(message)
  }
}

class NotFoundError extends AppError {
  public def new(message: str) {
    super(message)
  }
}

def find(id: int) -> str {
  throw NotFoundError("no item " + str(id))
}

try {
  find(7)
} catch NotFoundError {
  print("not found")      // most specific first
} catch AppError {
  print("application error")
} catch Error {
  print("something else")  // catches any remaining Error
}
```

`catch` clauses are matched in order, so list subclasses before their parents.
Catching `Error` catches every custom and builtin exception.

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

### 16.1 Python Interop

Any Python standard-library module or installed PyPI package can be imported
directly. For dynamic access, use the `python` bridge:

```aura
import python

let re = python.import_module("re")        // dynamic import
let math = python.load("math")             // same, returns a ModuleProxy
print(math.sqrt(2))

print(python.eval("1 + 2"))                // evaluate an expression
print(python.type_name(math))              // fully-qualified type name
print(python.is_available("requests"))     // True/False, never raises
```

Available helpers include `import_module`, `load`, `reload`, `is_available`,
`eval`, `exec_code`, `compile_source`, `call`, `getattr`, `setattr`, `hasattr`,
`dir`, `type_name`, `is_module`, `is_callable`, `is_class`, `is_instance`,
`to_aura`, `to_python`, `add_path`, `site_packages`, `modules` and
`interpreter_version`.

### 16.2 Modules

`module` declarations create a namespaced class. Dotted names nest:

```aura
module Geometry {
  def area(w, h) { return w * h }
}

module Outer.Inner {
  def value() { return 3 }
}

print(Geometry.area(2, 3))   // 6
print(Outer.Inner.value())   // 3
```

Local `.aura` files and packages are importable with the same syntax
(`import util`, `import pkg.util`, `from pkg.util import x`).

---

## 16b. Language Rules

Aura is strict by default. The following rules are enforced by `aura check`,
`aura run` and the REPL:

- **Immutability** — `let` bindings cannot be reassigned; use `let mut`.
  `const` can never be reassigned. Member assignments (`self.x = ...`) are not
  affected.
- **No duplicate declarations** in the same scope, and no duplicate parameter
  names.
- **`return` only inside a function** (or in a top-level
  `guard cond else { return }`, which exits the program).
- **`break` / `continue` only inside a loop.**
- **`await` only inside an `async` function**, or at the top level (Aura runs
  async programs inside a coroutine).
- **`self` only inside a class method.**
- **No unreachable code** after `return` / `throw` / `break` / `continue`.
- **Valid assignment targets only** (variables, members, indexes, destructuring
  patterns).

```aura
let count = 0
count = 1              // rule error: reassign an immutable binding
let mut total = 0
total += 1             // ok

guard total > 0 else { return }   // exits the program if the guard fails
```

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

Running async code with `aura run` automatically wraps the program in an event
loop; an `async def main` is awaited, so no explicit call is needed.

Multiple concurrent awaits:

```aura
async def parallel() {
  let a = await fetch("url1")
  let b = await fetch("url2")
  return [a, b]
}
```

### Async standard library

`stdlib.io` and `stdlib.http` expose `*_async` helpers so an `async def` never
blocks the event loop on file or network I/O. Each pairs with its synchronous
counterpart and behaves identically (same errors, same security checks):

```aura
import stdlib.io as io
import stdlib.http as http

async def main() {
  await io.write_async("out.txt", "hello\n")
  let text = await io.read_async("out.txt")
  print(text.trim())

  let response = await http.aget("https://example.com")
  print(response.status)

  let data = await http.aget_json("https://example.com/data.json")
}
```

Async HTTP keeps the same SSRF guard as the synchronous API: only `http` and
`https` are allowed, and private, loopback, link-local and unresolvable hosts
are refused unless `AURA_HTTP_ALLOW_PRIVATE=1` is set.

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
