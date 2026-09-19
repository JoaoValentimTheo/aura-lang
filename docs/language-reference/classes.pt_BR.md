---
layout: default
title: "Classes, Traits e Herança"
nav_exclude: true
---

[English](classes.md) | [Português](classes.pt_BR.md)

# Classes, Traits e Herança

**Status:** Stable (o congelamento de sintaxe OOP, `0.2.0a2`) · **Evidência:**
`aura/parser/to_ast.py` (`parse_class_decl`, `parse_trait_decl`),
`aura/transpiler/transformers/statements.py` (`transform_ClassDecl`,
`transform_TraitDecl`), `aura/transpiler/rules.py` (E307/E308/E309/E314/E315/
E316/E317/E321).

Aura tem quatro tipos de declaração de tipo: `class`, `trait`, `abstract class`
(uma classe com `abstract`) e `enum`. Não há `record` nem `interface`.
Instâncias são criadas chamando o tipo: `Point(1, 2)`, nunca `new Point(1, 2)`
(`new` é rejeitado).

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

### 1.1 Header fields (o construtor)

Campos podem ser declarados no **header** da classe. O header gera o construtor e
os acessores:

```aura
class User(private name: str, mut age: int = 0, public id: int = 0) {
  public def greet() -> str { return "hi " + self.get_name() }
}
```

| Header field | Gerado |
|---|---|
| `name: str` | parâmetro de construtor `name`; `get_name()`; armazenado como `self.name` |
| `private name: str` | igual, mas o campo é privado (mangled pelo dono) |
| `mut age: int = 0` | `get_age()` e `set_age(v)`; default `0` |
| `public id: int = 0` | `get_id()`/`set_id()` **e** acesso direto `u.id` |

Um header field obrigatório (`name: str` sem default) não pode seguir um
opcional (`age: int = 0`) — o construtor não teria uma posição inequívoca para
ele.

### 1.2 Um estilo de construtor por classe

Uma classe tem **exatamente um** estilo de construtor:

* header fields (`class U(name: str) { ... }`), ou
* campos de corpo com um `def new(...) { ... }` manual.

Misturar um header com um `new` manual é **erro de sintaxe**: o header já gera um
construtor e um segundo deixaria silenciosamente os header fields sem atribuição.

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

`def new(...)` compila para `__init__`. Instancie com `C(args)`, nunca
`C.new(args)`.

### 1.3 Protocolos embutidos (métodos dunder)

Aura mapeia um nome de método legível e sem prefixo para seu dunder Python quando
o membro é declarado. O mapeamento é fixo (`SPECIAL_METHOD_NAMES` em
`aura/transpiler/ast.py`); um subconjunto representativo:

| Método Aura | Dunder Python |
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

Nomes fora do mapa (`next`, `aenter`, `aexit`, `await`, …) são escritos como o
**dunder literal** (`def __next__(self) { ... }`), que sempre funciona. Um alias
legível é usado apenas onde o mapa define um. Escrever o dunder literal para um
nome mapeado também é aceito, mas a forma legível é canônica.

`self` (e `cls`) podem ser escritos como o parâmetro receptor explícito —
`def str(self) { ... }` — embora `self` seja uma palavra reservada em outros
lugares.

---

## 2. Visibilidade

Todo membro de classe **deve** carregar um modificador de visibilidade explícito;
omitir é `E307` (`MISSING_VISIBILITY`).

| Modificador | Significado |
|---|---|
| `public` | alcançável de qualquer lugar; um campo `public` também é lido/escrito diretamente |
| `private` | alcançável apenas dentro da classe declarante (mangled pelo dono em runtime) |
| `protected` | alcançável dentro da classe declarante e de suas subclasses |

Acessar um membro não público de fora é `E308` (`INACCESSIBLE_MEMBER`). A
checagem é ciente do dono: uma subclasse pode alcançar um membro `protected` do
pai, mas não o `private`.

```aura
class Account {
  private let balance: int = 0
  public def deposit(amount: int) { self.balance = self.balance + amount }
  public def get_balance() -> int { return self.balance }
}
```

`self.x = ...` (atribuição de membro) é sempre permitida dentro da classe, mesmo
para um campo `let` imutável — a imutabilidade do campo governa o *binding*, não
a mutação in-place do objeto.

---

## 3. Herança

A herança usa apenas `extends` (`implements` é rejeitado). Uma classe pode listar
vários traits/classes: `class C extends A, B`.

```aura
class Animal { public def speak() -> str { return "..." } }

class Dog extends Animal {
  public def speak() -> str { return "woof" }
}
```

* Uma subclasse que declara um método com o mesmo nome **sobrescreve**-o. Não há
  keyword `override`; escrever `override def` é erro de parse.
* O header da subclasse (`class Admin extends User(email: str)`) declara apenas
  seus **próprios** campos; campos herdados vêm do construtor base.
* Uma base deve resolver para uma classe/trait ou uma raiz de exceção embutida;
  caso contrário, `E314` (`UNKNOWN_BASE_CLASS`). Uma base duplicada ou um ciclo
  de herança é `E315` (`INVALID_INHERITANCE`).
* `super(args)` chama o construtor do pai; `super.method()` chama o método do pai.
  Chamar `super.m()` onde `m` é um método abstrato (sem corpo) sem implementação
  é `E321` (`ABSTRACT_SUPER_CALL`).

---

## 4. Traits

Um `trait` é um **contrato puro**: métodos, sem estado, sem construtor. Um método
sem corpo já é abstrato — `abstract` **não** é usado dentro de um trait
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
```

Um trait transpila para uma classe base. Uma classe concreta deve implementar
todo método sem corpo que herda, ou é `E309` (`UNIMPLEMENTED_ABSTRACT`).

---

## 5. Classes abstratas

Uma `abstract class` é uma **classe real** — campos, métodos concretos e um
construtor — que não pode ser instanciada e que pode adiar métodos com
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

Regras:

* `abstract` é um modificador antes de `class` ou antes de `def`:
  `abstract class C { public abstract def f() -> int }`. Um `abstract def` é
  **sem corpo**; `abstract def f() { ... }` e `abstract def f() = expr` são
  erros de sintaxe. Um `abstract def` pode aparecer apenas em uma
  `abstract class`.
* Uma `abstract class` pode ela mesma adiar: `abstract class Mid extends Shape`
  compila; a obrigação passa transitivamente para a subclasse concreta.
* Instanciar uma `abstract class` é `E316` em tempo de compilação; ela também
  compila para uma ABC Python com `@abstractmethod`, então a regra vale em
  runtime.
* Uma classe concreta que não implementa todo método abstrato herdado (de um
  trait ou de uma abstract class) é `E309`, nomeando a classe, o método e seu
  declarante.
* A sobrescrita é implícita; `override` não é keyword.

Use um trait quando a base não precisa de estado; use uma abstract class quando
ela precisa de campos ou comportamento concreto compartilhado.

---

## 6. Métodos estáticos e de classe, properties

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

* `@staticmethod` — sem receptor; usar `self`/`cls` dentro é `E317`
  (`SELF_IN_STATIC`).
* `@classmethod` — o primeiro parâmetro é `cls`.
* `@property` — o método é lido como um campo (`r.area`, não `r.area()`).

---

## 7. Classes aninhadas e genéricos

Classes podem se aninhar (`class Outer { public class Inner { } }`) e podem ser
genéricas com parâmetros de tipo entre colchetes:

```aura
class Box[T] {
  public let value: T = none
  public def new(value: T) { self.value = value }
  public def get() -> T { return self.value }
}

let b = Box(42)
print(b.get())     // 42
```

Parâmetros genéricos usam apenas colchetes: `Box[T]`, nunca `Box<T>`.

---

## 8. Enums

```aura
enum Color { Red, Green, Blue }
enum Status { Ok = 200, NotFound = 404 }
```

Membros são separados por `,`, `;` ou uma quebra de linha. `enum` transpila para
um enum Python: `Color.Red` imprime como `Color.Red`, é um valor de membro
distinto e nunca é igual à string nua `"Red"`.

---

## 9. O que NÃO faz parte da sintaxe de classe

| Grafia | Por quê | Use em vez disso |
|---|---|---|
| `new C()` | `new` é rejeitado | `C()` |
| `class C(...)` como record | não há records em Aura | uma classe com header fields |
| `class C implements I` | `implements` não é Aura | `class C extends I` |
| `override def f()` | a sobrescrita é implícita | `def f()` |
| `def init()` | um nome de construtor | `def new()` |
| `abstract def` em um trait | métodos de trait já são abstratos | `def f()` |
| `let private x` em um corpo | modificadores prefixam a declaração | `private let x` |
| header fields **e** `def new` | um estilo de construtor | escolha um |
| metaclasses | não expostas | — |
