---
layout: default
title: "06 — Funções"
nav_exclude: true
---

[English](06-functions.md) | [Português](06-functions.pt_BR.md)

# 06 — Funções

> **Meta do capítulo:** declarar, chamar e compor funções. Referência:
> [`../language-reference/functions.md`](../language-reference/functions.md).
> Exemplo: `fibonacci.aura`, `prime_checker.aura`.

## Declaração

`def` é a única keyword de função.

```aura
def add(a: int, b: int) -> int {
  return a + b
}

def square(x: int) -> int = x * x     // expression body
def double(x) = x * 2                 // no return type
def noop() { }                        // no return type, no return
```

O corpo é ou um bloco com chaves ou um único `= expr`; o corpo de expressão é
dessugarizado para `return <expr>`. Parâmetros podem ser anotados ou nus. O tipo
de retorno após `->` é opcional e é **apagado** no emit — uma incompatibilidade
não é aplicada pelo transpiler.

## Chamando

```aura
def add(a, b) -> int {
  return a + b
}

def main() {
  print(add(2, 3))            // positional
  print(add(a: 2, b: 3))      // keyword
}
```

Argumentos keyword são escritos `name: expr` (ou `name = expr`) no local da
chamada e viram argumentos keyword do Python. Um argumento posicional após um
keyword é erro de parse.

## Parâmetros default

```aura
def greet(name, greeting = "Hello") -> str {
  return greeting + ", " + name
}

def main() {
  print(greet("Ana"))           // Hello, Ana
  print(greet("Bob", "Hi"))     // Hi, Bob
}
```

Qualquer argumento com default à direita pode ser omitido.

## Variádicos e apenas keyword

```aura
def sum_all(*numbers) -> int {
  let mut total = 0
  for n in numbers { total += n }
  return total
}

def log(level, **context) {
  print(level, context)
}

def f(a, *, b) { return a + b }     // b must be passed by keyword
```

* `*args` — variádico posicional.
* `**kwargs` — mapeamento keyword.
* Um `*` nu torna os parâmetros seguintes apenas keyword.

No local da chamada, `*xs` espalha uma sequência, `**m` espalha um mapeamento, e
`...v` espalha de forma adaptativa (dict → keywords, caso contrário posicional):

```aura
def add_all(*nums) -> int {
  let mut total = 0
  for n in nums { total += n }
  return total
}

def main() {
  let nums = [1, 2, 3]
  print(add_all(*nums))       // 6
}
```

**Limitação:** `aura check` subconta argumentos de spread, então um spread para
uma função de aridade fixa pode reportar `E105` mesmo que `aura run` tenha
sucesso (o Python emitido honra o spread). Prefira espalhar para uma função
variádica, ou espere o diagnóstico do check.

`UNSPECIFIED`: nomes de parâmetro duplicados não são rejeitados pela gramática; o
Python alvo os rejeita.

## Retorno

`return` com um valor, ou nu. Múltiplos valores são uma lista com vírgulas e viram
uma tupla:

```aura
def swap(a, b) {
  return b, a
}

def main() {
  let x, y = swap(1, 2)       // x = 2, y = 1
  print(x, y)
}
```

Um `return` nu em posição de valor produz `none`. Em uma função tipo `void`,
`return` sozinho encerra o fluxo antecipadamente.

## Recursão

```aura
def factorial(n) -> int {
  if n <= 1 { return 1 }
  return n * factorial(n - 1)
}

def main() {
  print(factorial(5))         // 120
}
```

A recursão direta é suportada. Não há otimização de chamada de cauda garantida;
a recursão profunda é limitada pela pilha do CPython.

## Funções aninhadas

Uma função declarada dentro de outra é emitida no ponto de declaração:

```aura
def main() {
  def helper(x: int) -> int { return x + 1 }
  print(helper(2))            // 3
}
```

## Genéricos

Parâmetros de tipo usam **colchetes** e precedem a lista de parâmetros; uma
restrição pode seguir `:`. Eles são apagados no emit.

```aura
class Comparable {
  public def compare(other: Comparable) -> int { return 0 }
}

def id[T](x: T) -> T { return x }
def smallest[T: Comparable](items: [T]) -> T { return items[0] }

def main() {
  print(id(7))                // 7
  print(smallest([3, 1, 2]))  // 3
}
```

Um parâmetro declarado mas nunca usado avisa `W103`.

## Sem sobrecarga

Não há sobrecarga em nível de linguagem: um nome repetido no mesmo escopo é
`E301`. Funções de nível superior com o mesmo nome sobrescreveriam silenciosamente
em Python, e é por isso que Aura as rejeita.

## `main` — o ponto de entrada

```aura
def main() {
  print("hello")
}
```

Para receber argumentos de linha de comando, dê a `main` um único parâmetro
`args`:

```aura
def main(args: [string]) {
  print(f"{args.length()} argument(s)")
}
```

```bash
aura run app.aura one two
```

```text
2 argument(s)
```

`main` não recebe parâmetros ou recebe um único `args`; pode retornar um código de
saída `int` e pode ser `async`.

## O que você aprendeu

* `def`, corpos de expressão, tipos de retorno opcionais.
* Parâmetros posicionais, keyword, default e variádicos; chamadas com spread.
* Múltiplos retornos como tuplas; recursão; funções aninhadas; genéricos.
* Regras de assinatura de `main`.

## Próximo passo

[Classes e Objetos →](07-classes-and-objects.md)
