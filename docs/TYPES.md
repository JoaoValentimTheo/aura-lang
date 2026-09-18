# Aura Type System

Reference for the Aura type system. Aura uses gradual typing with type inference.

---

## Table of Contents

1. [Overview](#1-overview)
2. [Primitive Types](#2-primitive-types)
3. [Collection Types](#3-collection-types)
4. [Function Types](#4-function-types)
5. [Union Types](#5-union-types)
6. [Generic Types](#6-generic-types)
7. [Optional Types](#7-optional-types)
8. [Structural Types](#8-structural-types)
9. [Class Types](#9-class-types)
10. [Type Inference](#10-type-inference)
11. [Type Annotations](#11-type-annotations)
12. [Type Compatibility](#12-type-compatibility)
13. [Python Type Mapping](#13-python-type-mapping)

---

## 1. Overview

Aura provides **gradual typing** with **type inference**. Types are optional but recommended for function signatures and class definitions. The type system provides:

- Automatic type detection from literals and operations
- Union types to combine multiple types
- Generic types with square bracket syntax `[T]`
- Type narrowing through control flow
- Compile-time type validation

---

## 2. Primitive Types

| Aura Type | Python Equivalent | Description |
|-----------|-------------------|-------------|
| `int` | `int` | Arbitrary precision integer |
| `float` | `float` | 64-bit floating point |
| `str` | `str` | Unicode string |
| `bool` | `bool` | `true` or `false` |
| `bytes` | `bytes` | Byte sequence |
| `none` | `None` | Null value |

```aura
let x: int = 42
let y: float = 3.14
let s: str = "hello"
let b: bool = true
let n: none = null
```

---

## 3. Collection Types

### Lists

```aura
// Type syntax
[int]                    // List of integers
[str]                    // List of strings
[list[int]]              // Nested list

// Usage
let numbers: [int] = [1, 2, 3]
let mixed = [1, "two", 3.0, true]
let empty = []
```

### Dictionaries

```aura
// Type syntax
{str: int}               // Dict with string keys, int values
{str: any}               // Dict with any values

// Usage
let user: {name: str, age: int} = {name: "Alice", age: 30}
let config = {"max-size": 100, "timeout": 30}
```

### Sets

```aura
// Type syntax
{int}                    // Set of integers

// Usage
let unique: {int} = {1, 2, 3}
```

### Tuples

```aura
// Tuples are represented as fixed-size lists in the type system
// Runtime: (1, "hello") is a Python tuple
```

---

## 4. Function Types

```aura
// Type syntax: (ParamTypes) -> ReturnType

let handler: (int, int) -> int = add
let predicate: (str) -> bool = is_valid
let callback: () -> none = on_ready
```

### In function signatures

```aura
def apply(f: (int) -> int, x: int) -> int {
  return f(x)
}

def compose(f: (int) -> int, g: (int) -> int) -> (int) -> int {
  return (x) => g(f(x))
}
```

---

## 5. Union Types

Union types allow a value to be one of several types:

```aura
let value: int | str = 42
value = "hello"  // also valid

// In function signatures
def process(value: int | str) {
  // ...
}

// Multiple types
let result: int | float | none = null
```

---

## 6. Generic Types

Generics use **square brackets** `[T]`:

### Generic classes

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

let int_box: Box[int] = Box(42)
let str_box: Box[str] = Box("hello")
```

### Multiple type parameters

```aura
class Pair[A, B] {
  let first: A
  let second: B

  def new(first: A, second: B) {
    self.first = first
    self.second = second
  }
}

let p: Pair[str, int] = Pair("age", 30)
```

### Generic functions

```aura
def first[T](items: [T]) -> T | none {
  if len(items) > 0 { return items[0] }
  return none
}

def map[T, R](items: [T], fn: (T) -> R) -> [R] {
  return [fn(item) for item in items]
}
```

---

## 7. Optional Types

Any type can be made nullable with `?`:

```aura
let name: str? = get_name()   // can be str or null

// Null-safe access
let upper: str? = name?.upper()

// Null coalescing
let display: str = name ?? "Anonymous"
```

### In function signatures

```aura
def find_user(id: int) -> User? {
  let user = db.query(id)
  return user
}

// Usage
let user = find_user(123)
guard user != null else {
  return
}
// user is guaranteed non-null here
```

---

## 8. Structural Types

Structural types describe the shape of a value:

```aura
let point: {x: float, y: float} = {x: 1.0, y: 2.0}

let user: {
  name: str,
  age: int,
  email?: str            // optional field
} = {name: "Alice", age: 30}
```

---

## 9. Class Types

Classes create named types:

```aura
class User {
  public let name: str = ""
  public let age: int = 0

  public def new(name: str, age: int) {
    self.name = name
    self.age = age
  }
}

// User is now a type
let alice: User = User("Alice", 30)
```

Class header fields are annotated types too, and give the constructor its
signature:

```aura
class User(name: str, age: int = 0) {
}

let alice: User = User("Alice", 30)
```

### Inheritance and subtyping

```aura
class Animal {
  public let name: str = ""
}

class Dog extends Animal {
  public let breed: str = ""
}

// Dog is a subtype of Animal
def process_animal(a: Animal) {
  // ...
}

let d = Dog()
process_animal(d)  // OK: Dog is subtype of Animal
```

Inheritance uses `extends` only; multiple bases are separated by commas and a
member lookup searches each base in order.

---

## 10. Type Inference

The transpiler infers types automatically from:

| Source | Inference |
|--------|-----------|
| `42` | `int` |
| `3.14` | `float` |
| `"hello"` | `str` |
| `true` | `bool` |
| `null` / `none` | `none` |
| `[1, 2, 3]` | `[int]` |
| `{a: 1}` | `{str: int}` |
| `1..10` | `[int]` |
| `x + y` (both int) | `int` |
| `"a" + "b"` | `str` |

### Examples

```aura
let x = 10              // inferred: int
let y = x + 5           // inferred: int
let name = "Alice"      // inferred: str
let items = [1, 2, 3]   // inferred: [int]
let point = {x: 10, y: 20}  // inferred: {x: int, y: int}

// Return type inferred
def double(x) = x * 2   // return type inferred as int
```

---

## 11. Type Annotations

### Variable annotations

```aura
let name: str = "Alice"
let age: int = 30
let scores: [float] = [9.5, 8.7, 9.2]
```

### Function annotations

```aura
def add(a: int, b: int) -> int {
  return a + b
}

def greet(name: str, greeting: str = "Hello") -> str {
  return f"{greeting}, {name}!"
}
```

### Class annotations

```aura
class User {
  let name: str = ""
  let age: int = 0
  let email: str = ""

  def new(name: str, age: int) {
    self.name = name
    self.age = age
  }
}
```

---

## 12. Type Compatibility

### Subtype relationships

- A type is compatible with itself
- A subclass is compatible with its parent class
- `any` is compatible with all types

```aura
class Animal { }
class Dog(Animal) { }

def process(a: Animal) { }

let d = Dog()
process(d)  // OK: Dog is subtype of Animal
```

### Union type compatibility

```aura
let x: int | str = 10      // OK: int is compatible with int | str
```

---

## 13. Python Type Mapping

| Aura Type | Python Type | Notes |
|-----------|-------------|-------|
| `int` | `int` | Arbitrary precision |
| `float` | `float` | 64-bit |
| `str` | `str` | Unicode |
| `bool` | `bool` | `true`/`false` |
| `none` | `NoneType` | Null |
| `[T]` | `list` | Dynamic array |
| `{K: V}` | `dict` | Hash map |
| `{T}` | `set` | Hash set |
| `(A, B)` | `tuple` | Fixed-size |
| `(A) -> B` | `Callable` | Function type |
| `any` | `Any` | Opt-out of checking |

Aura types are a **compile-time feature**. Types are erased before Python generation, but type information is used for error detection, IDE support, and documentation.
