---
layout: default
title: "15 — Macros e Decorators"
nav_exclude: true
---

[English](15-macros-and-decorators.md) | [Português](15-macros-and-decorators.pt_BR.md)

# 15 — Macros e Decorators

> **Meta do capítulo:** anexar comportamento reutilizável a funções com
> decorators, e usar as macros de tempo de compilação de Aura. Exemplos:
> `../../examples/macros.aura`, `../../examples/compile_time_macros.aura`.

## Decorators

Um decorator é `@name` ou `@name(args)` acima de um `def` (ou de uma classe).
Decorators em um **campo** são rejeitados (`E320`). Modificadores de membro e
decorators podem aparecer em qualquer ordem.

```aura
@staticmethod
public def max(a, b) -> int { return a }

class Rect {
  public @property
  def area() -> int { return 4 }

  public @classmethod
  def create(cls) { return cls() }
}
```

Os decorators de membro de classe são `@property`, `@staticmethod` e
`@classmethod` (capítulo 07). Um decorator em uma função simples pode ser
qualquer chamável, incluindo um que você escreve:

```aura
def trace(f) {
  def wrapper(x) {
    print(f"calling with {x}")
    return f(x)
  }
  return wrapper
}

@trace
def square(x) -> int { return x * x }

def main() {
  print(square(4))
}
```

## Macros de runtime embutidas

Aura traz um pequeno prelúdio de decorators. O prelúdio é injetado
automaticamente apenas quando uma macro é usada.

```aura
@debug
def multiply(a, b) -> int {
  return a * b
}

@timeit
def sum_to(n) -> int {
  let mut total = 0
  for i in range(n) { total += i }
  return total
}

@memoize
def fib(n) -> int {
  if n < 2 { return n }
  return fib(n - 1) + fib(n - 2)
}

@cache(maxsize=256)
def expensive(n) -> int { return n * n }

def main() {
  print(multiply(6, 7))     // DEBUG: enter/exit lines, then 42
  print(sum_to(100))        // a timing line, then 4950
  print(fib(20))            // 6765
  print(expensive(12))      // 144
}
```

| Decorator | Efeito |
|---|---|
| `@debug` | imprime os argumentos e o resultado da chamada |
| `@timeit` | imprime quanto tempo a chamada levou |
| `@memoize` | cache de resultados ilimitado |
| `@cache(maxsize=n)` | cache LRU limitado |

Estes são decorators de **runtime**: eles envolvem a função Python emitida.

## Macros de tempo de compilação

Uma segunda camada expande **antes** de qualquer Python ser emitido. A macro
recebe seus operandos como AST citado e retorna AST de substituição, então nada
sobrevive ao runtime a menos que a expansão escolha emiti-lo. Importe-as de
`macros`:

```aura
import macros

def main() {
  assert_eq(2 + 2, 4)        // evaluate both once, then assert
  assert_ne("a", "b")
  static_assert(true)        // checked while compiling

  let mut a = 1
  let mut b = 2
  swap(a, b)                 // hygienic temporary; a,b change
  print(a, b)                // 2 1

  print(identity(41) + 1)    // identity disappears; prints 42
  print(stringify(42))       // folds the literal at compile time
  print(debug_value(a))      // prints "a = 2", yields the value
}
```

| Macro | Expansão |
|---|---|
| `assert_eq(a, b)` | avalia ambos uma vez, afirma igualdade |
| `assert_ne(a, b)` | avalia ambos uma vez, afirma desigualdade |
| `static_assert(x)` | checada em tempo de compilação; um não-literal é erro |
| `identity(x)` | expande para `x` |
| `stringify(x)` | dobra um literal para uma string em tempo de compilação |
| `swap(a, b)` | troca dois lvalues via um temporário higiênico |
| `debug_value(x)` | imprime `x = <value>` uma vez, depois produz o valor |

Uma macro que introduz bindings usa nomes **higiênicos**, então ela nunca pode
capturar um local no local da chamada.

## Runtime vs tempo de compilação

| | Decorators de runtime | Macros de tempo de compilação |
|---|---|---|
| Quando rodam | quando a função é chamada | durante a transpilação |
| Forma | `@decorator` em um `def` | expressões tipo chamada no corpo |
| Exemplos | `@debug`, `@timeit`, `@memoize`, `@cache` | `assert_eq`, `swap`, `stringify` |
| Use quando | você quer um wrapper em torno de uma chamada | você quer o código reescrito antes do emit |

## Ordem dos decorators

Quando vários decorators são empilhados eles se aplicam de baixo para cima: o mais
próximo do `def` envolve primeiro, o mais externo roda primeiro. A ordem muda o
comportamento. Usando um wrapper `trace` local:

```aura
def trace(f) {
  def wrapper(x) {
    print(f"calling with {x}")
    return f(x)
  }
  return wrapper
}

@memoize
@trace
def fib(n) -> int {
  if n < 2 { return n }
  return fib(n - 1) + fib(n - 2)
}

def main() {
  print(fib(5))
}
```

```text
calling with 5
calling with 4
calling with 3
calling with 2
calling with 1
calling with 0
5
```

Com `@memoize` mais externo, o cache é checado antes do trace, então cada valor de
`n` é rastreado exatamente uma vez. Troque as duas linhas e o trace roda em toda
chamada, porque a recursão alcança a função rastreada antes do cache.

## O que você aprendeu

* Decorators se anexam a `def`/classe (nunca a um campo); `@property`,
  `@staticmethod`, `@classmethod` e decorators personalizados.
* Decorators de runtime embutidos `@debug`, `@timeit`, `@memoize`, `@cache`.
* Macros de tempo de compilação de `macros`, incluindo `assert_eq`, `swap`,
  `stringify`, `static_assert`.

## Próximo passo

[CLI e Tooling →](16-cli-and-tooling.md)
