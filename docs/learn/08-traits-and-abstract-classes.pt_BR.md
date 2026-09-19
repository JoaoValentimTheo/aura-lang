---
layout: default
title: "08 — Traits e Classes Abstratas"
nav_exclude: true
---

[English](08-traits-and-abstract-classes.md) | [Português](08-traits-and-abstract-classes.pt_BR.md)

# 08 — Traits e Classes Abstratas

> **Meta do capítulo:** expressar contratos e polimorfismo. Referência:
> [`../language-reference/classes.md`](../language-reference/classes.md) §4–5.
> Exemplo: `../../examples/abstract_classes.aura`.

## Trait — um contrato puro

Um `trait` declara métodos **sem estado e sem construtor**. Um método sem corpo já
é abstrato; **não** escreva `abstract` dentro de um trait
(`trait T { abstract def f() }` é erro de parse).

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

Uma classe concreta deve implementar todo método sem corpo que herda, ou é
`E309`. Traits estendem outros traits com `extends`.

Use um trait quando a base **não precisa de estado**.

## Classe abstrata — estado compartilhado + métodos adiados

Uma `abstract class` é uma classe real: campos, métodos concretos e um
construtor. Ela **não pode ser instanciada** (`E316`) e pode adiar métodos com
`abstract def`.

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

Regras:

* `abstract` é um modificador antes de `class` ou antes de `def`.
* Um `abstract def` é **sem corpo**; `abstract def f() { ... }` e
  `abstract def f() = expr` são erros de sintaxe.
* Um `abstract def` pode aparecer apenas em uma `abstract class`.
* Uma classe abstrata pode ela mesma adiar; a obrigação passa para a subclasse
  concreta.
* Uma classe concreta que não implementa um método abstrato herdado é `E309`.

Use uma classe abstrata quando a base precisa de **campos ou comportamento
concreto compartilhado**.

## Polimorfismo através da base

Um parâmetro de função anotado com o tipo base aceita qualquer subtipo:

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

O tipo do parâmetro é a **base abstrata**; a chamada despacha para o `area` da
subclasse. Este é o despacho de método comum do CPython após o emit.

## `super`

```aura
class Base {
  public def name() -> str { return "base" }
}

class Derived extends Base {
  public def name() -> str { return "derived of " + super.name() }
}
```

`super.method()` chama o método do pai; `super(args)` chama o construtor do pai.
Chamar `super.m()` onde `m` é um método abstrato sem implementação é `E321`.

## Escolhendo

| Necessidade | Use |
|---|---|
| Um contrato sem estado | `trait` |
| Campos compartilhados ou métodos concretos, sem instâncias diretas | `abstract class` |
| Implementação completa, instanciável | `class` |

## O que você aprendeu

* `trait` é um contrato sem estado; métodos sem corpo já são abstratos.
* `abstract class` tem estado e pode adiar métodos com `abstract def`.
* Nem um trait nem uma classe abstrata podem ser instanciados.
* Polimorfismo através do tipo base; `super` para o pai.

## Próximo passo

[Coleções →](09-collections.md)
