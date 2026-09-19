---
layout: default
title: "07 — Classes and Objects"
parent: Learn Aura
nav_order: 17
---

[English](07-classes-and-objects.md) | [Português](07-classes-and-objects.pt_BR.md)

# 07 — Classes and Objects

> **Chapter goal:** model data and behaviour with classes. Reference:
> [`../language-reference/classes.md`](../language-reference/classes.md).
> Example: `../../examples/classes.aura`.

## A class with body fields and a manual constructor

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

def main() {
  let p = Point(3, 4)
  print(p.distance())        // 5.0
}
```

Key rules:

* Every member **must** declare a visibility (`public`, `private`,
  `protected`); omitting it is `E307`.
* `def new(...)` is the constructor and compiles to `__init__`.
* Instantiate by calling the type: `Point(3, 4)`. There is **no `new` keyword**
  and no `C.new(...)` call. `new C()` is rejected.
* `self` is the receiver. `self.x = ...` is allowed inside the class even for a
  `let` field — immutability governs the *binding*, not the object.

## Header fields

Fields may be declared in the class **header** instead. The header generates the
constructor and accessors:

```aura
class User(private name: str, mut age: int = 0, public id: int = 0) {
  public def greet() -> str {
    return "hi " + self.get_name()
  }

  public def birthday() {
    self.set_age(self.get_age() + 1)
  }
}

def main() {
  let u = User("ana", 30)
  print(u.greet())           // hi ana
  u.birthday()
  print(u.get_age())         // 31
}
```

| Header field | Generated |
|---|---|
| `name: str` | constructor parameter; `get_name()`; stored as `self.name` |
| `private name: str` | as above, but owner-mangled and not reachable outside |
| `mut age: int = 0` | `get_age()` **and** `set_age(v)`; default `0` |
| `public id: int = 0` | `get_id()`/`set_id()` and direct access `u.id` |

A required header field may not follow an optional one.

## One constructor style per class

A class has **exactly one** constructor style: header fields **or** body fields
with a manual `def new`. Mixing them is a syntax error.

```aura
class A(name: str) { }                 // OK — header style

class B {                              // OK — body style
  public let name: str = ""
  public def new(name: str) { self.name = name }
}

class C(name: str) {                   // ERROR — both styles
  public def new(name: str) { self.name = name }
}
```

## Visibility

```aura
class Account {
  private let balance: int = 0

  public def deposit(amount: int) {
    self.balance = self.balance + amount
  }

  public def get_balance() -> int { return self.balance }
}
```

Accessing a non-public member from outside is `E308`. A subclass may reach a
parent's `protected` member but not its `private` one.

## Inheritance with `extends`

```aura
class Animal {
  public def speak() -> str { return "..." }
}

class Dog extends Animal {
  public def speak() -> str { return "woof" }
}

def main() {
  print(Dog().speak())       // woof
}
```

* `extends` is the only inheritance keyword. `implements` and `class C(A)` are
  rejected.
* Overriding is **implicit** — there is no `override` keyword.
* A subclass header declares only its **own** fields; inherited fields come from
  the base constructor.
* `super(args)` calls the parent constructor; `super.method()` calls a parent
  method.
* A dotted base (`class Model extends django.db.models.Model`) marks a base
  Python owns, so framework metaclasses work unchanged.

## Static, class methods and properties

```aura
class MathUtil {
  public @staticmethod def max(a: int, b: int) -> int {
    if a > b { return a }
    return b
  }

  public @classmethod def create() -> MathUtil {
    return MathUtil()
  }
}

class Rect {
  public let w: int = 0
  public def new(w: int) { self.w = w }

  public @property
  def area() -> int { return self.w * 2 }
}

def main() {
  print(MathUtil.max(3, 9))  // 9
  print(Rect(4).area)        // 8 — no parentheses
}
```

* `@staticmethod` has no receiver; using `self`/`cls` inside is `E317`.
* `@classmethod` receives `cls` as its first parameter.
* `@property` is read like a field: `r.area`, not `r.area()`.
* Member modifiers and decorators may appear in any order.

## Enums

```aura
enum Color { RED, GREEN, BLUE }
enum Status { Ok = 200, NotFound = 404 }
```

Members are comma-separated (a trailing comma is allowed). `Color.RED` is a
distinct value that never equals the bare string `"Red"`.

## Dunder methods

Aura maps readable method names to Python dunders when declared: `new` →
`__init__`, `str` → `__str__`, `len` → `__len__`, `eq`/`lt`/... →
`__eq__`/`__lt__`, `getitem` → `__getitem__`, `call` → `__call__`, and so on. A
name outside the map is written as the literal dunder (`def __next__(self)`).
`self` may be written explicitly as the receiver parameter.

## What is NOT Aura

| Spelling | Use instead |
|---|---|
| `new C()` | `C()` |
| `class C(A)` | `class C extends A` |
| `implements` | `extends` |
| `override def f()` | `def f()` (overriding is implicit) |
| `def init()` | `def new()` |
| `let private x` | `private let x` |

## What you learned

* Body-field and header-field classes; `def new`.
* Mandatory visibility; `extends` and implicit overriding.
* `@staticmethod`, `@classmethod`, `@property`; enums; dunder mapping.

## Next step

[Traits and Abstract Classes →](08-traits-and-abstract-classes.md)
