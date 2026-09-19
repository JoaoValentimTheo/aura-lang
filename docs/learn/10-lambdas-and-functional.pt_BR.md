---
layout: default
title: "10 — Lambdas e Programação Funcional"
nav_exclude: true
---

[English](10-lambdas-and-functional.md) | [Português](10-lambdas-and-functional.pt_BR.md)

# 10 — Lambdas e Programação Funcional

> **Meta do capítulo:** escrever funções anônimas, capturar variáveis e construir
> pipelines de dados. Referência:
> [`../language-reference/closures.md`](../language-reference/closures.md).
> Exemplo: `../../examples/functional.aura`.

## Formas de lambda

Uma lambda é um valor de função anônima, escrita com `=>`. **Não há keyword
`lambda`**.

```aura
let double = x => x * 2
let square = (x) => x * x
let add = (a, b) => a + b
let get_answer = () => 42
```

| Forma | Exemplo |
|---|---|
| Um parâmetro, sem parênteses | `x => x * 2` |
| Um parâmetro, parênteses | `(x) => x * x` |
| Múltiplos parâmetros | `(a, b) => a + b` |
| Nenhum | `() => 42` |
| Corpo de expressão | `(x) => x + 1` |
| Corpo de bloco | `(x) => { let y = x + 1; return y }` |
| Currying | `(n) => (x) => x + n` |

Um corpo de expressão vira uma `lambda` Python; um corpo de bloco é hoisted para
uma função nomeada real (`_aura_lambda_N`), porque a `lambda` do Python não pode
conter statements.

Parâmetros usam a gramática de `def`: anotados, com default, `*args`, `**kwargs`.
Uma anotação é aceita e apagada; um tipo de retorno `->` **não** é sintaxe de
lambda.

## Chamando lambdas

```aura
def main() {
  let add = (a, b) => a + b
  let blocky = (x) => {
    let y = x + 1
    return y * 2
  }
  print(add(2, 3))        // 5
  print(blocky(4))        // 10
}
```

## Closures

Uma lambda que lê um local envolvente o captura por referência:

```aura
def main() {
  let base = 10
  let add_base = (x) => x + base
  print(add_base(5))      // 15
}
```

Currying — retornar uma lambda que captura um argumento:

```aura
def main() {
  let make_adder = (n) => (x) => x + n
  let add10 = make_adder(10)
  print(add10(5))         // 15
}
```

**Limitação:** uma **lambda de bloco que atribui** a um local envolvente
atualmente dispara `E319` (`'n' is used before it is declared`) no checker, mesmo
que a referência descreva emitir `nonlocal`. Captura somente leitura e currying
estão verificados; para um contador, use um objeto mutável (uma lista de um
elemento ou uma classe) em vez de mutar um local capturado.

**Pegadinha:** a captura é **por referência**, não um snapshot por iteração. Uma
lambda criada em um loop vê o valor **final** da variável do loop. Vincule-a como
um parâmetro default (`(x, i = i) => x + i`) para captura por iteração.

## Funções de ordem superior

Lambdas são valores comuns, então funções as recebem e retornam. É assim que os
helpers de coleção funcionam.

```aura
def apply_twice(f, x) {
  return f(f(x))
}

def main() {
  print(apply_twice((x) => x + 3, 1))    // 7
}
```

## `map` / `filter` / `reduce`

Estes vêm de `stdlib.collections` e são auto-importados quando usados:

```aura
def main() {
  let numbers = [1, 2, 3, 4, 5]
  let doubled = map(numbers, (x) => x * 2)          // [2, 4, 6, 8, 10]
  let evens = filter(numbers, (x) => x % 2 == 0)    // [2, 4]
  let total = reduce(numbers, (a, b) => a + b, 0)   // 15
  print(doubled, evens, total)
}
```

## O operador pipe `|>`

`a |> f` aplica `f` a `a`; `a |> f(b)` insere `a` como o **primeiro** argumento.
Pipelines se leem da esquerda para a direita:

```aura
def main() {
  let numbers = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]

  let result = numbers
    |> filter((x) => x % 2 == 0)
    |> map((x) => x * x)
    |> reduce((acc, x) => acc + x, 0)

  print(f"Sum of even squares: {result}")    // 220
}
```

Quando o lado direito é uma função de coleção da stdlib (`map`, `filter`,
`reduce`, `take`, `drop`), o import é injetado automaticamente. Pipe é o operador
de expressão **mais frouxo** e encadeia da esquerda para a direita. Se o lado
direito é um identificador nu, ele se torna `f(left)`.

## Composição

```aura
def main() {
  let compose = (f, g) => (x) => g(f(x))
  let inc = x => x + 1
  let double = x => x * 2
  print(compose(inc, double)(5))    // 12 — double(inc(5))
}
```

## Sem açúcar de trailing-lambda

Aura **não** tem sintaxe de chamada trailing-lambda `f { ... }`. Um `{ ... }`
após uma chamada é ou um bloco separado ou um inicializador de struct, não um
argumento final. Coloque a lambda dentro dos parênteses:

```aura
apply((x) => x + 1)      // correct
// apply(1) { x => x + 1 }   // NOT a trailing lambda
```

## O que você aprendeu

* Lambdas `x => ...`, `(a, b) => ...`, `() => ...`, com corpos de expressão ou de
  bloco.
* Captura de leitura e currying; a limitação de captura mutável.
* Funções de ordem superior; `map`/`filter`/`reduce`; o operador pipe.
* Sem açúcar de trailing-lambda.

## Próximo passo

[Pattern Matching →](11-pattern-matching.md)
