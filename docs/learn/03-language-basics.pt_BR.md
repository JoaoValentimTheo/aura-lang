---
layout: default
title: "03 — Noções Básicas"
nav_exclude: true
---

[English](03-language-basics.md) · [Português](03-language-basics.pt_BR.md)

# 03 — Noções Básicas

> **Meta do capítulo:** aprender como bindings, funções e o ponto de entrada se
> encaixam. Referência:
> [`../language-reference/statements.md`](../language-reference/statements.md),
> [`../language-reference/functions.md`](../language-reference/functions.md).

## `let`, `let mut`, `const`

Aura torna a mutabilidade explícita na declaração.

```aura
let name = "Alice"          // immutable binding
let mut counter = 0         // mutable binding
const LIMIT = 3             // constant; must be initialised

counter += 1                // ok — counter is mutable
```

Regras:

* `let` liga um nome que não pode ser **reatribuído**.
* `let mut` liga um nome que pode ser reatribuído.
* `const` deve ser inicializado e nunca é reatribuível.
* Reatribuir um `let` ou `const` é **E303**, e é um erro (não um warning) tanto
  em `aura run`, quanto em `aura check` e no REPL.

```aura
let x = 1
x = 2              // E303: reassign an immutable binding

let mut total = 0
total += 1         // ok
```

Uma anotação é opcional e vem após `:`:

```aura
let age: int = 30
let items: [int] = [1, 2, 3]
const MAX: int = 100
```

**Não há `var`**. `var x = 1` é rejeitado com uma mensagem apontando para
`let mut`/`let`.

### Desestruturação

```aura
let (a, b) = (1, 2)
let [first, ...rest] = [1, 2, 3, 4]
print(a, b, first, rest)     // 1 2 1 [2, 3, 4]
```

O elemento `...name` coleta os itens restantes.

## `def` — funções

Funções são declaradas com `def`. Não há `fn`, `fun` nem `function`.

```aura
def greet(name) -> str {
  return "Hello, " + name
}

def square(x: int) -> int = x * x     // expression body
```

O corpo é ou um bloco com chaves ou um único `= expr`. Um tipo de retorno após
`->` é opcional; parâmetros podem ser anotados, não anotados, com default ou
variádicos. Tudo isso é o capítulo 06.

## `main` — o ponto de entrada

Um arquivo rodado com `aura run` deve ter um `main` de nível superior. O runtime
o chama; nunca o chame você mesmo.

```aura
def main() {
  print("entry point")
}
```

`main` pode:

* não receber parâmetros, ou receber um único parâmetro `args`;
* ser `async`;
* retornar um `int`, que vira o código de saída do processo.

```aura
def main() -> int {
  return 3          // process exit code 3
}
```

Um `main` ausente é **E310**; uma assinatura errada é **E311**; um `main` dentro
de um corpo de `module` é **E312**.

## `print`

`print` escreve seus argumentos na saída padrão, separados por espaço, com uma
quebra de linha final:

```aura
def main() {
  print("value:", 42, true)
}
```

```text
value: 42 True
```

Literais booleanos são `true` e `false`; o valor nulo é `none`. Note como `true`
imprime como o `True` do Python: booleanos são booleanos do CPython em runtime.

## Juntando tudo

```aura
const GREETING = "Hello"

def greet(name, punctuation = "!") -> str {
  return GREETING + ", " + name + punctuation
}

def main() {
  let mut count = 0
  print(greet("world"))
  count += 1
  print(f"greeted {count} time(s)")
}
```

```text
Hello, world!
greeted 1 time(s)
```

## Posição dos modificadores

Modificadores de classe e de módulo aparecem **antes** da keyword de declaração:
`private let x = 1`, nunca `let private x`. Isso importa a partir do capítulo 07.

## O que você aprendeu

* `let` é imutável, `let mut` é mutável, `const` é constante.
* Reatribuir um binding imutável é E303.
* `def` declara funções; o corpo é um bloco ou `= expr`.
* `main` é o ponto de entrada e é invocado pelo runtime.
* `print`, `true`/`false`, `none` e f-strings.

## Próximo passo

[Variáveis e Tipos →](04-variables-and-types.md)
