---
layout: default
title: "04 — Variáveis e Tipos"
nav_exclude: true
---

[English](04-variables-and-types.md) | [Português](04-variables-and-types.pt_BR.md)

# 04 — Variáveis e Tipos

> **Meta do capítulo:** saber quais tipos existem, como escrevê-los e o que o
> checker de fato aplica. Referência:
> [`../language-reference/types.md`](../language-reference/types.md) e
> [`../language-reference/type-system.md`](../language-reference/type-system.md).

## Aura é gradualmente tipada

Anotações são **opcionais**. Um binding sem anotação assume o tipo inferido de seu
inicializador; um binding que o checker não consegue provar é tratado como `any` e
aceito. Não há regra de "deve anotar".

```aura
let inferred = 42          // int, from the literal
let explicit: int = 42     // written out
```

Crucialmente, as anotações são **apagadas** antes do emit Python. Elas documentam
e (em parte) checam; não mudam o runtime. `aura check` roda o type checker;
`aura run` não — então um programa pode rodar com um erro de tipo que `check`
reportaria.

## Tipos primitivos

| Tipo Aura | Significado |
|---|---|
| `int` | inteiro (precisão arbitrária, o `int` do Python) |
| `float` | ponto flutuante de 64 bits |
| `str` | string Unicode |
| `bool` | `true` / `false` |
| `bytes` | string de bytes (anotação aceita, não checada) |
| `none` | valor de ausência **e** seu tipo |

```aura
let age: int = 30
let price: float = 3.14
let title: str = "Aura"
let active: bool = true
let nothing: none = none
```

`null` **não** é Aura — `let n = null` é um erro de parse apontado dizendo para
usar `none`. Não há tipo `char`, `uint`, `void` nem `unit`.

### `bool` não alarga para `int`

Diferente de Kof/JVM, `true + 1` é rejeitado (`E108`); não há coerção
`bool → int`. Converta explicitamente com `int(b)` se precisar de um número.

## Tipos de coleção

```aura
let items: [int] = [1, 2, 3]              // list
let scores: {str: int} = {math: 95}       // dict
let structured: {x: float, y: float} = {x: 1.0, y: 2.0}   // structural
let boxed: Box[int] = Box(42)             // generic (brackets only)
```

| Grafia | Significado |
|---|---|
| `[T]` | lista de `T` (também `List[T]`) |
| `{K: V}` | dict de `K` para `V` (também `Dict[K, V]`) |
| `{name: T, ...}` | forma estrutural; checada como dict, nomes de campo não retidos |
| `Set[T]` | set de `T` |
| `Box[T]` | aplicação de tipo genérico — **colchetes**, nunca `<...>` |

`Box<T>` é rejeitado: *"type arguments use brackets, not '<...>'"*. As grafias de
**tipo** set e tupla são limitadas: `{T}` e `(A, B)` não parseiam como tipos
(veja [`types.md`](../language-reference/types.md) §2.3), embora os **valores**
set e tupla `{1, 2}` e `(1, "x")` existam e sejam inferidos.

## Tipos opcionais e de união

```aura
let maybe: str? = none            // optional — same as `str | none`
let value: int | str = 42         // union
```

`T?` rebaixa para `T | none`. Uma união é compatível com cada um de seus membros.

## Tipos de função

```aura
let handler: (int, int) -> int = add
let predicate: (str) -> bool = is_valid
```

Um `(` inicial com `->` é um tipo de função. Tipos de parâmetro e retorno não são
retidos pelo checker — qualquer anotação desse tipo vira um tipo de função
simples.

## Aliases de tipo

```aura
type UserId = int
type Point = {x: float, y: float}
```

Aliases são documentação: o checker não os expande, então um nome de alias em uma
anotação resolve para `any` a menos que também seja uma classe declarada.

## Casts: `as`

```aura
let q = 3.0 as int        // int(3.0)
let s = 42 as str         // str(42)
print(7 / 2)              // 3.5
print(int(7 / 2))         // 3 — integer quotient
```

`x as T` emite `T(x)`. Aura **não tem operador de divisão inteira**; `//` é um
comentário de linha. Use `int(a / b)`.

## `none` e checagens de nulo

```aura
let missing = none
print(missing is none)          // true
print(missing ?? "fallback")    // fallback
print((0) ?: "zero-ish")        // zero-ish
```

* `x is none` / `x is not none` — checagem de identidade de nulo.
* `x ?? y` — produz `y` apenas quando `x` é `none`.
* `x ?: y` — produz `y` quando `x` é **falsy** (`none`, `false`, `0`, `""`, `[]`,
  `{}`).

Comparar qualquer outro literal com `is` é erro de parse: escreva `x == 5`, não
`x is 5`.

## O que o checker garante

* Mesmo tipo, ou qualquer lado `any` — aceito.
* O alargamento `int → float` **não** é implícito; anote ou faça cast.
* Funções sem anotação não têm tipo de retorno inferido; a anotação é
  documentação.

A fronteira de garantia está detalhada em
[`type-system.md`](../language-reference/type-system.md) §4.

## O que você aprendeu

* Primitivos: `int`, `float`, `str`, `bool`, `bytes`, `none`.
* Coleções: `[T]`, `{K: V}`, `{shape}`, `Set[T]`, `Box[T]`.
* Opcional `T?`, união `T | U`, tipos de função `(T) -> R`.
* `none`, `is none`, `??`, `?:` e o cast `as`.
* Anotações são opcionais, checadas parcialmente e apagadas em runtime.

## Próximo passo

[Controle de Fluxo →](05-control-flow.md)
