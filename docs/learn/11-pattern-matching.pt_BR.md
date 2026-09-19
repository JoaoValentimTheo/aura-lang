---
layout: default
title: "11 — Pattern Matching"
nav_exclude: true
---

[English](11-pattern-matching.md) | [Português](11-pattern-matching.pt_BR.md)

# 11 — Pattern Matching

> **Meta do capítulo:** despachar por forma e valor com `match`. Referência:
> [`../language-reference/statements.md`](../language-reference/statements.md)
> §5. Exemplo: `../../examples/pattern_matching.aura`.

## `match` como statement

```aura
def classify(value) -> str {
  match value {
    case 0 { return "zero" }
    case 1 { return "one" }
    case n if n > 100 { return "big" }
    case n if n > 0 { return "positive" }
    case n if n < 0 { return "negative" }
    case _ { return "other" }
  }
  return "unreachable"
}
```

```text
0 -> zero
1 -> one
42 -> positive
200 -> big
-7 -> negative
```

Um corpo de case é um **bloco** `{ ... }` ou uma **seta** `-> expression`. Não há
fallthrough. As sintaxes de case `:` e `=>` são rejeitadas com uma mensagem
apontada.

## Padrões

| Padrão | Exemplo | Vincula |
|---|---|---|
| Wildcard | `case _` | nada |
| Literal | `case 0`, `case "quit"` | nada |
| Binding | `case n` | `n` |
| Guard | `case n if n > 0` | `n`, apenas se o guard valer |
| Or-pattern | `case 1 \| 2` | — |
| Membro de enum | `case Color.RED` | — |
| Desestruturação de lista/tupla | `case [a, b]`, `case [first, *rest]` | `a`, `b`, ... |

Um case com guard **não** conta como catch-all: um `match` sobre um domínio
escalar, `bool` ou enum sem `case _` (ou binding nu) avisa `E109`.

## Casando strings

```aura
def command(name) -> str {
  match name {
    case "quit" { return "exiting" }
    case "help" { return "showing help" }
    case _ { return "unknown command" }
  }
  return ""
}
```

## Desestruturação

```aura
def sum_pair(pair) -> int {
  match pair {
    case [a, b] { return a + b }
    case _ { return 0 }
  }
  return 0
}
```

`*rest` coleta o restante: `case [first, *rest]`.

## `match` como expressão

Com corpos em seta, `match` produz um valor:

```aura
def main() {
  let n = 2
  let text = match n {
    case 1 -> "one"
    case 2 -> "two"
    case _ -> "other"
  }
  print(text)              // two
}
```

A forma de expressão é hoisted para uma função auxiliar; a expressão final de
cada case vira o valor retornado.

## Enums

```aura
enum Color { RED, GREEN, BLUE }

def main() {
  let c = Color.RED
  match c {
    case Color.RED { print("red") }
    case _ { print("other color") }
  }
}
```

Membros são separados por vírgula. `Color.RED` é um valor distinto, nunca a
string `"Red"`.

## Pegadinhas

* **`break` em um case com seta engole o próximo `case` como label.** Como
  quebras de linha são insignificantes, `case x -> break` seguido de `case y`
  parseia como `break case`. Use um corpo de bloco ou um `;` à direita.
* `match` precisa de um catch-all para um domínio finito, ou `E109` avisa (nunca
  falha um build). Um `case _ if cond` com guard **não** suprime o warning.
* Não há keyword `when`; guards usam `if`.

## Escolher `match` em vez de `if`

Use `if`/`else if` para um par de condições booleanas. Use `match` quando você
está despachando por **forma** (desestruturação), por um **enum**, ou por vários
valores literais ao mesmo tempo — ele mantém o valor inspecionado em um lugar só.

## Padrões, precisamente

Um padrão não é uma expressão: ele nunca *computa*, apenas **testa** e
**vincula**. Um nome nu em minúsculas sempre vincula (não compara contra uma
variável de mesmo nome); membros pontuados capitalizados como `Color.RED`
comparam contra um membro de enum. Essa distinção é o motivo de `case n` ser um
binding catch-all enquanto `case Color.RED` é um teste de valor. Para comparar
contra um valor computado, use um guard: `case x if x == threshold`.

Um or-pattern `case 1 | 2` casa quando **qualquer** alternativa casa, e toda
alternativa deve vincular os mesmos nomes. A desestruturação `case [a, b]` casa
apenas uma lista/tupla de exatamente dois elementos; adicione `*rest` para
permitir mais.

## Um exemplo mais completo

```aura
enum Color { RED, GREEN, BLUE }

def describe(value) -> str {
  match value {
    case 0 { return "zero" }
    case Color.RED { return "red" }
    case [a, b] { return f"pair summing to {a + b}" }
    case [first, *rest] { return f"list starting with {first}" }
    case "hello" { return "the greeting" }
    case _ { return "something else" }
  }
  return ""
}
```

```text
zero
pair summing to 3
list starting with 1
the greeting
red
something else
```

A ordem importa: padrões são testados de cima para baixo e o primeiro que casa
vence. Coloque os padrões mais específicos primeiro, e termine com `case _` quando
o domínio não é exaustivo.

**Pegadinha:** um padrão de binding nu (`case s`) casa **qualquer** valor, então
um guard sobre ele como `case s if s.length() > 3` também será alcançado por
valores que não têm `length` — ordene os cases concretos primeiro, ou cheque o
tipo no guard.

## Exercícios

1. Escreva `def sign(n) -> str` usando padrões de literal e de guard.
2. Faça match sobre uma lista e retorne sua cabeça, ou `none` quando vazia.
3. Construa um enum de dois valores e despache sobre ele, depois remova `case _`
   e observe o warning `E109` de `aura check`.

## O que você aprendeu

* `match` como statement e como expressão.
* Padrões wildcard, literal, binding, guard, or, enum e desestruturação.
* Sem fallthrough; corpos `->` e `{ }`; a pegadinha do `break`; `E109`.

## Próximo passo

[Tratamento de Erros →](12-error-handling.md)
