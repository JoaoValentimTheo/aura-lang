---
layout: default
title: "07 — Classes e Objetos"
nav_exclude: true
---

[English](07-classes-and-objects.md) · [Português](07-classes-and-objects.pt_BR.md)

# 07 — Classes e Objetos

> **Meta do capítulo:** modelar dados e comportamento com classes. Referência:
> [`../language-reference/classes.md`](../language-reference/classes.md).
> Exemplo: `../../examples/classes.aura`.

## Uma classe com campos de corpo e um construtor manual

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

Regras-chave:

* Todo membro **deve** declarar uma visibilidade (`public`, `private`,
  `protected`); omitir é `E307`.
* `def new(...)` é o construtor e compila para `__init__`.
* Instancie chamando o tipo: `Point(3, 4)`. **Não há keyword `new`** e nem
  chamada `C.new(...)`. `new C()` é rejeitado.
* `self` é o receptor. `self.x = ...` é permitido dentro da classe mesmo para um
  campo `let` — a imutabilidade governa o *binding*, não o objeto.

## Header fields

Campos podem ser declarados no **header** da classe. O header gera o construtor e
os acessores:

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

| Header field | Gerado |
|---|---|
| `name: str` | parâmetro de construtor; `get_name()`; armazenado como `self.name` |
| `private name: str` | como acima, mas mangled pelo dono e inalcançável de fora |
| `mut age: int = 0` | `get_age()` **e** `set_age(v)`; default `0` |
| `public id: int = 0` | `get_id()`/`set_id()` e acesso direto `u.id` |

Um header field obrigatório não pode seguir um opcional.

## Um estilo de construtor por classe

Uma classe tem **exatamente um** estilo de construtor: header fields **ou** campos
de corpo com um `def new` manual. Misturá-los é erro de sintaxe.

```aura skip
class A(name: str) { }                 // OK — header style

class B {                              // OK — body style
  public let name: str = ""
  public def new(name: str) { self.name = name }
}

class C(name: str) {                   // ERROR — both styles
  public def new(name: str) { self.name = name }
}
```

## Visibilidade

```aura
class Account {
  private let balance: int = 0

  public def deposit(amount: int) {
    self.balance = self.balance + amount
  }

  public def get_balance() -> int { return self.balance }
}
```

Acessar um membro não público de fora é `E308`. Uma subclasse pode alcançar um
membro `protected` do pai, mas não o `private`.

## Herança com `extends`

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

* `extends` é a única keyword de herança. `implements` e `class C(A)` são
  rejeitados.
* A sobrescrita é **implícita** — não há keyword `override`.
* O header de uma subclasse declara apenas seus **próprios** campos; campos
  herdados vêm do construtor base.
* `super(args)` chama o construtor do pai; `super.method()` chama um método do
  pai.
* Uma base pontuada (`class Model extends django.db.models.Model`) marca uma base
  que o Python possui, então metaclasses de framework funcionam sem alteração.

## Métodos estáticos, de classe e properties

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

* `@staticmethod` não tem receptor; usar `self`/`cls` dentro é `E317`.
* `@classmethod` recebe `cls` como seu primeiro parâmetro.
* `@property` é lida como um campo: `r.area`, não `r.area()`.
* Modificadores de membro e decorators podem aparecer em qualquer ordem.

## Enums

```aura
enum Color { RED, GREEN, BLUE }
enum Status { Ok = 200, NotFound = 404 }
```

Membros são separados por vírgula (uma vírgula à direita é permitida). `Color.RED`
é um valor distinto que nunca é igual à string nua `"Red"`.

## Métodos dunder

Aura mapeia nomes de método legíveis para dunders Python quando declarados:
`new` → `__init__`, `str` → `__str__`, `len` → `__len__`, `eq`/`lt`/... →
`__eq__`/`__lt__`, `getitem` → `__getitem__`, `call` → `__call__`, e assim por
diante. Um nome fora do mapa é escrito como o dunder literal
(`def __next__(self)`). `self` pode ser escrito explicitamente como o parâmetro
receptor.

## O que NÃO é Aura

| Grafia | Use em vez disso |
|---|---|
| `new C()` | `C()` |
| `class C(A)` | `class C extends A` |
| `implements` | `extends` |
| `override def f()` | `def f()` (a sobrescrita é implícita) |
| `def init()` | `def new()` |
| `let private x` | `private let x` |

## O que você aprendeu

* Classes com campos de corpo e header fields; `def new`.
* Visibilidade obrigatória; `extends` e sobrescrita implícita.
* `@staticmethod`, `@classmethod`, `@property`; enums; mapeamento de dunder.

## Próximo passo

[Traits e Classes Abstratas →](08-traits-and-abstract-classes.md)
