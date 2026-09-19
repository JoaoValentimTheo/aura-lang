---
layout: default
title: "05 — Controle de Fluxo"
nav_exclude: true
---

[English](05-control-flow.md) | [Português](05-control-flow.pt_BR.md)

# 05 — Controle de Fluxo

> **Meta do capítulo:** ramificar e repetir. Referência:
> [`../language-reference/statements.md`](../language-reference/statements.md)
> (controle de fluxo) e [`../language-reference/expressions.md`](../language-reference/expressions.md)
> (o operador ternário). Exemplos: `fibonacci.aura`, `prime_checker.aura`.

## `if` / `else if` / `else`

```aura
def main() {
  let n = 7
  if n % 2 == 0 {
    print("even")
  } else if n % 3 == 0 {
    print("divisible by 3")
  } else {
    print("odd")
  }
}
```

```text
odd
```

As chaves são **obrigatórias**. `if c print(1)` é erro de parse. `else if`
encadeia a qualquer profundidade, mas `elif` e `elsif` são rejeitados com uma
mensagem apontada dizendo para escrever `else if`.

## `unless` — `if` invertido

`unless cond { ... }` executa o bloco quando `cond` é falsy:

```aura
def main() {
  let authenticated = false
  unless authenticated {
    print("please log in")
  }
}
```

```text
please log in
```

`unless` também aceita um `else`.

## `guard ... else`

`guard cond else { ... }` executa o bloco `else` quando `cond` é **falsy**, e
então continua. É o idioma para validação antecipada — o caminho feliz permanece
plano:

```aura
def process(value) {
  guard value > 0 else {
    print(f"invalid: {value}")
    return
  }
  print(f"processing {value}")
}
```

No nível superior, um `return` nu no corpo do guard sai do programa. Este é o
idioma de Aura para "pare aqui":

```aura
guard ready else { return }
```

## Loops

Aura tem quatro formas de loop mais o `for ... in`.

```aura
let mut i = 0
while i < 3 {
  print(i)
  i += 1
}

until i >= 5 {
  i += 1
}

loop {
  if i >= 6 { break }
  i += 1
}
```

| Forma | Roda |
|---|---|
| `while cond { }` | enquanto `cond` é truthy |
| `until cond { }` | enquanto `cond` é **falsy** (`while` invertido) |
| `loop { }` | para sempre, até `break`/`return`/`throw` |

Não há `repeat` nem `do`; use `loop { }` ou `until`.

### `for ... in` e ranges

```aura
for item in [10, 20, 30] { print(item) }

for i in 0..<3 { print(i) }          // 0 1 2   (exclusive)
for i in 0..5 step 2 { print(i) }    // 0 2 4
for i in range(0, 6, 2) { print(i) } // 0 2 4
```

`1..10` é **inclusivo** (1 até 10); `0..<10` é **exclusivo**.
`for x in items step 2` fatia qualquer iterável com `[::2]`. O alvo de um `for` é
um padrão, então `for (k, v) in pairs` desestrutura cada elemento.

### Labels

Um label antes de um loop o nomeia; `break label` / `continue label` têm como
alvo esse loop:

```aura
outer: for i in range(3) {
  inner: for j in range(3) {
    if j == 1 { continue outer }
    if i == 2 { break outer }
    print(i, j)
  }
}
```

Um `break`/`continue` fora de um loop é rejeitado (`E004`); um label que não
nomeia nenhum loop envolvente é `E318`.

**Pegadinha:** um `break` no fim de um case de `match` com corpo em seta engole o
`case` seguinte como label (quebras de linha são insignificantes). Use um `;` ou
um corpo de bloco para desambiguar.

## `match` — uma primeira olhada

`match` compara um valor contra padrões. É statement e expressão, e é coberto em
profundidade no capítulo 11:

```aura
def classify(value) -> str {
  match value {
    case 0 { return "zero" }
    case n if n > 100 { return "big" }
    case n if n > 0 { return "positive" }
    case _ { return "other" }
  }
  return "unreachable"
}
```

Corpos de case são blocos `{ ... }` ou setas `-> expression`. Guards usam `if`.
`:` e `=>` são rejeitados.

## Ternário (expressão condicional)

```aura
let label = condition ? "yes" : "no"
```

`c ? t : f` emite `t if c else f`. Ele liga mais frouxo que tudo até `or`.

## Statement vs expressão

A maioria das construções de controle de fluxo são statements. `if`, `match` e
`try` também têm uma forma de **expressão** que produz um valor:

```aura
let x = if ready { 1 } else { 0 }
let kind = match n { case 1 -> "one"; case _ -> "other" }
```

A atribuição em si **não** é uma expressão.

## O que você aprendeu

* `if`/`else if`/`else`, `unless`, `guard ... else`.
* `while`, `until`, `loop`, `for ... in`, ranges e `step`, labels.
* Uma primeira olhada em `match`, no ternário `? :` e em `if`/`match` como
  expressões.

## Próximo passo

[Funções →](06-functions.md)
