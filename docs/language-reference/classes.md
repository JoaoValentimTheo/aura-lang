---
layout: default
title: "Classes, Traits and Inheritance"
parent: Aura Language Reference
nav_order: 1
---

[English](classes.md) · [Português](classes.pt_BR.md)

# Classes, Traits and Inheritance

**Status:** Stable (the OOP syntax freeze, `0.2.0a2`) · **Evidence:**
`aura/parser/to_ast.py` (`parse_class_decl`, `parse_trait_decl`),
`aura/transpiler/transformers/statements.py` (`transform_ClassDecl`,
`transform_TraitDecl`), `aura/transpiler/rules.py` (E307/E308/E309/E314/E315/
E316/E317/E321).

Aura has four kinds of type declaration: `class`, `trait`, `abstract class`
(a class with `abstract`), and `enum`. There is no `record` and no `interface`.
Instances are created by calling the type: `Point(1, 2)`, never `new Point(1, 2)`
(`new` is rejected).

---

## 1. Classes

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

let p = Point(1, 2)
print(p.distance())
```

### 1.1 Header fields (the constructor)

Fields may be declared in the class **header**. The header generates the
constructor and the accessors:

```aura
class User(private name: str, mut age: int = 0, public id: int = 0) {
  public def greet() -> str { return "hi " + self.get_name() }
}
```

| Header field | Generated |
|---|---|
| `name: str` | constructor parameter `name`; `get_name()`; stored as `self.name` |
| `private name: str` | same, but the field is private (owner-mangled) |
| `mut age: int = 0` | `get_age()` and `set_age(v)`; defaults to `0` |
| `public id: int = 0` | `get_id()`/`set_id()` **and** direct access `u.id` |

A required header field (`name: str` with no default) may not follow an
optional one (`age: int = 0`) — the constructor would have no unambiguous
position for it.

### 1.2 One constructor style per class

A class has **exactly one** constructor style:

* header fields (`class U(name: str) { ... }`), or
* body fields with a manual `def new(...) { ... }`.

Mixing a header with a manual `new` is a **syntax error**: the header already
generates a constructor and a second one would silently leave the header fields
unassigned.

```aura
// OK — header style
class A(name: str) { }

// OK — body style
class B {
  public let name: str = ""
  public def new(name: str) { self.name = name }
}

// ERROR — header and manual `new` mix
class C(name: str) {
  public def new(name: str) { self.name = name }
}
```

`def new(...)` compiles to `__init__`. Instantiate with `C(args)`, never
`C.new(args)`.

### 1.3 Built-in protocols (dunder methods)

Aura maps a readable, unprefixed method name to its Python dunder when the
member is declared. The mapping is fixed (`SPECIAL_METHOD_NAMES` in
`aura/transpiler/ast.py`); a representative subset:

| Aura method | Python dunder |
|---|---|
| `new` | `__init__` |
| `destroy` | `__del__` |
| `str` / `repr` / `format` | `__str__` / `__repr__` / `__format__` |
| `bool` / `int` / `float` / `hash` / `len` | `__bool__` / `__int__` / `__float__` / `__hash__` / `__len__` |
| `eq` / `ne` / `lt` / `le` / `gt` / `ge` | `__eq__` / `__ne__` / `__lt__` / `__le__` / `__gt__` / `__ge__` |
| `getitem` / `setitem` / `delitem` / `contains` | `__getitem__` / `__setitem__` / `__delitem__` / `__contains__` |
| `iter` / `reversed` | `__iter__` / `__reversed__` |
| `call` | `__call__` |
| `add` / `sub` / `mul` / `truediv` / `mod` / `pow` / `neg` / `invert` | `__add__` / `__sub__` / `__mul__` / `__truediv__` / `__mod__` / `__pow__` / `__neg__` / `__invert__` |
| `enter` / `exit` | `__enter__` / `__exit__` |

Names not in the map (`next`, `aenter`, `aexit`, `await`, …) are written as the
**literal dunder** (`def __next__(self) { ... }`), which always works. A
readable alias is used only where the map defines one. Writing the literal
dunder for a mapped name is also accepted, but the readable form is canonical.

`self` (and `cls`) may be written as the explicit receiver parameter —
`def str(self) { ... }` — even though `self` is a reserved word elsewhere.

---

## 2. Visibility

Every class member **must** carry an explicit visibility modifier; omitting it
is `E307` (`MISSING_VISIBILITY`).

| Modifier | Meaning |
|---|---|
| `public` | reachable from anywhere; a `public` field is also read/written directly |
| `private` | reachable only inside the declaring class (owner-mangled at runtime) |
| `protected` | reachable inside the declaring class and its subclasses |

Accessing a non-public member from outside is `E308` (`INACCESSIBLE_MEMBER`).
The check is owner-aware: a subclass may reach a parent's `protected` member
but not its `private` one.

```aura
class Account {
  private let balance: int = 0
  public def deposit(amount: int) { self.balance = self.balance + amount }
  public def get_balance() -> int { return self.balance }
}
```

`self.x = ...` (member assignment) is always allowed inside the class, even for
an immutable `let` field — the field's immutability governs the *binding*, not
in-place mutation of the object.

---

## 3. Inheritance

Inheritance uses `extends` only (`implements` is rejected). A class may list
several traits/classes: `class C extends A, B`.

```aura
class Animal { public def speak() -> str { return "..." } }

class Dog extends Animal {
  public def speak() -> str { return "woof" }
}
```

* A subclass that declares a method with the same name **overrides** it. There
  is no `override` keyword; writing `override def` is a parse error.
* The subclass header (`class Admin extends User(email: str)`) declares only
  its **own** fields; inherited fields arrive from the base constructor.
* A base must resolve to a class/trait or a built-in exception root; otherwise
  `E314` (`UNKNOWN_BASE_CLASS`). A duplicate base or an inheritance cycle is
  `E315` (`INVALID_INHERITANCE`).
* `super(args)` calls the parent constructor; `super.method()` calls the parent
  method. Calling `super.m()` where `m` is an abstract (body-less) method with
  no implementation is `E321` (`ABSTRACT_SUPER_CALL`).

---

## 4. Traits

A `trait` is a **pure contract**: methods, no state, no constructor. A method
with no body is already abstract — `abstract` is **not** used inside a trait
(`trait T { abstract def f() }` is a parse error).

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
```

A trait transpiles to a base class. A concrete class must implement every
body-less method it inherits, or it is `E309` (`UNIMPLEMENTED_ABSTRACT`).

---

## 5. Abstract classes

An `abstract class` is a **real class** — fields, concrete methods and a
constructor — that cannot be instantiated and may defer methods with
`abstract def`.

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

print(Square().area())       // 4.0
print(Square().describe())   // a shape
// let s = Shape()           // E316: abstract, cannot be instantiated
```

Rules:

* `abstract` is a modifier before `class` or before `def`:
  `abstract class C { public abstract def f() -> int }`. An `abstract def` is
  **body-less**; `abstract def f() { ... }` and `abstract def f() = expr` are
  syntax errors. An `abstract def` may appear only in an `abstract class`.
* An `abstract class` may itself defer: `abstract class Mid extends Shape`
  compiles; the obligation passes transitively to the concrete subclass.
* Instantiating an `abstract class` is `E316` at compile time; it also compiles
  to a Python ABC with `@abstractmethod`, so the rule holds at runtime.
* A concrete class that does not implement every inherited abstract method
  (from a trait or an abstract class) is `E309`, naming the class, the method
  and its declarer.
* Overriding is implicit; `override` is not a keyword.

Use a trait when the base needs no state; use an abstract class when it needs
fields or shared concrete behaviour.

---

## 6. Static and class methods, properties

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
  public @property def area() -> int { return self.w * 2 }
}

print(MathUtil.max(3, 9))     // 9
print(Rect(4).area)           // 8 — accessed without parentheses
```

* `@staticmethod` — no receiver; using `self`/`cls` inside is `E317`
  (`SELF_IN_STATIC`).
* `@classmethod` — first parameter is `cls`.
* `@property` — the method is read as a field (`r.area`, not `r.area()`).

---

## 7. Nested classes and generics

Classes may nest (`class Outer { public class Inner { } }`), and may be generic
with bracket type parameters:

```aura
class Box[T] {
  public let value: T = none
  public def new(value: T) { self.value = value }
  public def get() -> T { return self.value }
}

let b = Box(42)
print(b.get())     // 42
```

Generic parameters use brackets only: `Box[T]`, never `Box<T>`.

---

## 8. Enums

```aura
enum Color { Red, Green, Blue }
enum Status { Ok = 200, NotFound = 404 }
```

Members are separated by `,`, `;`, or a newline. `enum` transpiles to a Python
enum: `Color.Red` prints as `Color.Red`, is a distinct member value, and never
equals the bare string `"Red"`.

---

## 9. What is NOT part of class syntax

| Spelling | Why | Use instead |
|---|---|---|
| `new C()` | `new` is rejected | `C()` |
| `class C(...)` as a record | no records in Aura | a class with header fields |
| `class C implements I` | `implements` is not Aura | `class C extends I` |
| `override def f()` | overriding is implicit | `def f()` |
| `def init()` | one constructor name | `def new()` |
| `abstract def` in a trait | trait methods are already abstract | `def f()` |
| `let private x` in a body | modifiers prefix the declaration | `private let x` |
| header fields **and** `def new` | one constructor style | pick one |
| metaclasses | not exposed | — |
