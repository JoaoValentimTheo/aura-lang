---
layout: default
title: "08 — Traits and Abstract Classes"
parent: Learn Aura
nav_order: 18
---

[English](08-traits-and-abstract-classes.md) | [Português](08-traits-and-abstract-classes.pt_BR.md)

# 08 — Traits and Abstract Classes

> **Chapter goal:** express contracts and polymorphism. Reference:
> [`../language-reference/classes.md`](../language-reference/classes.md) §4–5.
> Example: `../../examples/abstract_classes.aura`.

## Trait — a pure contract

A `trait` declares methods **without state and without a constructor**. A
body-less method is already abstract; do **not** write `abstract` inside a
trait (`trait T { abstract def f() }` is a parse error).

```aura
trait Drawable {
  public def draw() -> void
  public def bounds() -> float
}

class Circle extends Drawable {
  public let radius: float = 1.0

  public def draw() -> void { print("circle") }
  public def bounds() -> float { return 3.14159 * self.radius * self.radius }
}

def main() {
  let c = Circle(2.0)
  c.draw()                   // circle
  print(c.bounds())          // 12.56636
}
```

A concrete class must implement every body-less method it inherits, or it is
`E309`. Traits extend other traits with `extends`.

Use a trait when the base needs **no state**.

## Abstract class — shared state + deferred methods

An `abstract class` is a real class: fields, concrete methods and a constructor.
It **cannot be instantiated** (`E316`) and may defer methods with `abstract def`.

```aura
abstract class Shape {
  public let name: str = "shape"

  public def describe() -> str {
    return "a " + self.name
  }

  public abstract def area() -> float       // no body
}

class Square extends Shape {
  public let side: float = 2.0

  public def area() -> float { return self.side * self.side }
}

def main() {
  let s = Square(3.0)
  print(s.area())            // 9.0
  print(s.describe())        // a shape
  // let bad = Shape()       // E316: an abstract class cannot be instantiated
}
```

Rules:

* `abstract` is a modifier before `class` or before `def`.
* An `abstract def` is **body-less**; `abstract def f() { ... }` and
  `abstract def f() = expr` are syntax errors.
* An `abstract def` may appear only in an `abstract class`.
* An abstract class may itself defer; the obligation passes to the concrete
  subclass.
* A concrete class missing an inherited abstract method is `E309`.

Use an abstract class when the base needs **fields or shared concrete
behaviour**.

## Polymorphism through the base

A function parameter annotated with the base type accepts any subtype:

```aura
abstract class Shape {
  public let name: str = "shape"
  public def describe() -> str { return "a " + self.name }
  public abstract def area() -> float
}

class Square extends Shape {
  public let side: float = 2.0
  public def new(side: float) {
    self.name = "square"
    self.side = side
  }
  public def area() -> float { return self.side * self.side }
}

class Circle extends Shape {
  public let radius: float = 1.0
  public def new(radius: float) {
    self.name = "circle"
    self.radius = radius
  }
  public def area() -> float { return 3.14159 * self.radius * self.radius }
}

def print_area(shape: Shape) {
  print(f"{shape.describe()}: {shape.area()}")
}

def main() {
  print_area(Square(3.0))
  print_area(Circle(2.0))
}
```

```text
a square: 9.0
a circle: 12.56636
```

The parameter type is the **abstract base**; the call dispatches to the
subclass's `area`. This is ordinary CPython method dispatch after emit.

## `super`

```aura
class Base {
  public def name() -> str { return "base" }
}

class Derived extends Base {
  public def name() -> str { return "derived of " + super.name() }
}
```

`super.method()` calls the parent method; `super(args)` calls the parent
constructor. Calling `super.m()` where `m` is an abstract method with no
implementation is `E321`.

## Choosing

| Need | Use |
|---|---|
| A contract with no state | `trait` |
| Shared fields or concrete methods, no direct instances | `abstract class` |
| Full implementation, instantiable | `class` |

## What you learned

* `trait` is a state-free contract; body-less methods are already abstract.
* `abstract class` has state and may defer methods with `abstract def`.
* Neither a trait nor an abstract class can be instantiated.
* Polymorphism through the base type; `super` for the parent.

## Next step

[Collections →](09-collections.md)
