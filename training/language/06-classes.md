---
title: "06 — Classes"
---

# Classes

Aura has four kinds of type declaration: `class`, `trait`, `abstract class`, and `enum`. Instances are created by calling the type: `Point(1, 2)`, never `new Point(1, 2)`.

See `aura/parser/to_ast.py:parse_class_decl`, `aura/transpiler/transformers/statements.py:transform_ClassDecl`.

## Class Declaration

```aura
class Point {
  public let x: int = 0
  public let y: int = 0
  public def new(x: int, y: int) { self.x = x; self.y = y }
  public def distance() -> float {
    return (self.x ** 2 + self.y ** 2) ** 0.5
  }
}
let p = Point(1, 2)
```

`def new(...)` compiles to `__init__`. Instantiate with `Type(args)`, never `Type.new(args)`.

## Header Fields

Fields declared in the class header generate the constructor and accessors:

```aura
class User(private name: str, mut age: int = 0, public id: int = 0) {
  public def greet() -> str { return "hi " + self.get_name() }
}
```

| Header field | Generated |
|-------------|-----------|
| `name: str` | constructor parameter; `get_name()` |
| `private name: str` | same, field is private (mangled) |
| `mut age: int = 0` | `get_age()` and `set_age(v)` |
| `public id: int = 0` | `get_id()`/`set_id()` **and** direct access |

A required header field must not follow an optional one.

## One Constructor Style Per Class

A class has **exactly one** constructor style: header fields or body fields with a manual `def new`. Mixing is a syntax error.

## Visibility

Every class member **must** carry an explicit visibility modifier (E307):

```aura
class Account {
  private let balance: int = 0
  public def deposit(amount: int) { self.balance += amount }
  public def get_balance() -> int { return self.balance }
}
```

Accessing a non-public member from outside is **E308**.

## Inheritance

```aura
class Animal { public def speak() -> str { return "..." } }
class Dog extends Animal { public def speak() -> str { return "woof" } }
```

- `extends` only (`implements` is rejected). Multiple inheritance: `class C extends A, B`.
- Override is implicit. `override` is not a keyword.
- `super(args)` calls parent constructor; `super.method()` calls parent method.

## Traits

A `trait` is a **pure contract**: methods, no state, no constructor. A body-less method is already abstract — `abstract` is not used inside a trait.

```aura
trait Drawable {
  public def draw() -> void
  public def bounds() -> float
}
```

A concrete class must implement every body-less method (E309).

## Abstract Classes

An `abstract class` is a real class with fields, concrete methods, and a constructor. Cannot be instantiated; may defer methods with `abstract def`:

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

// let s = Shape()   // E316: abstract, cannot be instantiated
```

An `abstract def` is body-less and may appear only in an `abstract class`.

## Built-in Protocols (Dunder Methods)

Aura maps readable names to Python dunders when declared:

| Aura method | Python dunder |
|-------------|---------------|
| `new` | `__init__` |
| `str` / `repr` | `__str__` / `__repr__` |
| `eq` / `ne` / `lt` / `gt` | `__eq__` / `__ne__` / `__lt__` / `__gt__` |
| `getitem` / `setitem` | `__getitem__` / `__setitem__` |
| `iter` / `call` | `__iter__` / `__call__` |
| `enter` / `exit` | `__enter__` / `__exit__` |

Names not in the map are written as the literal dunder.

## Static and Class Methods

```aura
class MathUtil {
  public @staticmethod def max(a: int, b: int) -> int {
    if a > b { return a }; return b
  }
  public @classmethod def create() -> MathUtil { return MathUtil() }
}
```

`@staticmethod` — no receiver. `@classmethod` — first parameter is `cls`.

## Generics and Enums

```aura
class Box[T] {
  public let value: T = none
  public def new(value: T) { self.value = value }
  public def get() -> T { return self.value }
}

enum Color { Red, Green, Blue }
enum Status { Ok = 200, NotFound = 404 }
```

Generic parameters use brackets `[T]`. Enum members are comma-separated; `Color.Red` is a distinct value, never `"Red"`.

## Gotcha

- `new ClassName()` is rejected; use `ClassName()`.
- `class C implements I` is rejected; use `extends`.
- `override def f()` is a parse error; overriding is implicit.
- `abstract def f() { ... }` is a syntax error — abstract methods have no body.
- Header fields and manual `def new` cannot coexist.

## Anti-pattern

Do not use a trait when you need state or a constructor. Use an abstract class instead.
