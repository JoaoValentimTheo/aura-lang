---
layout: default
title: "09 — Coleções"
nav_exclude: true
---

[English](09-collections.md) | [Português](09-collections.pt_BR.md)

# 09 — Coleções

> **Meta do capítulo:** armazenar e processar dados com listas, dicts, sets e
> tuplas. Referência:
> [`../language-reference/expressions.md`](../language-reference/expressions.md)
> §14, §16. Exemplo: `../../examples/tour.aura`.

## Literais

```aura
let list = [1, 2, 3]
let set = {1, 2, 3}
let dict = {name: "Alice", age: 30}
let pair = (1, "hello")
let single = (42,)
let empty = ()
```

* Listas usam `[ ... ]`.
* Sets usam `{ ... }` sem `:`.
* Dicts usam `{ key: value }`. Acesso por índice `d["k"]` ou por membro `d.k`
  funcionam.
* Tuplas precisam da vírgula: `(42,)` é uma tupla de um elemento; `(42)` é apenas
  `42`.

Uma vírgula à direita é permitida em todos os lugares. Dicts preservam a ordem de
inserção (CPython 3.7+).

## Indexação e slices

```aura
let numbers = [10, 20, 30, 40]

print(numbers[0])       // 10
print(numbers[-1])      // 40 — negative index
print(numbers[1:3])     // [20, 30]
print(numbers[::2])     // [10, 30]
```

Um **range usado como índice é um slice**:

```aura
print(numbers[0..<2])   // [10, 20] — exclusive upper bound
print(numbers[0..2])    // [10, 20, 30] — inclusive
print(numbers[1..])     // [20, 30, 40] — open end
```

## Tamanho, pertinência e mutação

```aura
let numbers = [1, 2, 3]
numbers.add(4)               // append
print(numbers.length())      // 4
print(numbers.size())        // 4
print(numbers.contains(2))   // true
print(numbers.is_empty())    // false
print(2 in numbers)          // true
```

`.length()` / `.size()` → `len(x)`; `.contains(x)` → `x in x`; `.add(x)` →
`append`. Um método **declarado pelo usuário** com um desses nomes vence sobre a
conveniência.

Chamadas de membro desconhecidas passam para o Python verbatim: `.append`,
`.pop`, `.sort`, `.reverse`, `.insert`, `.remove`, `.index`, `.count`, `.extend`,
`.clear`.

## Dicts e sets

```aura
let user = {name: "Alice", age: 30}
print(user["name"])          // Alice
print(user.age)              // 30

let unique = {1, 2, 2, 3, 3, 3}
print(unique)                // {1, 2, 3}
```

Chaves de dict podem ser escritas sem aspas quando são identificadores:
`{name: "Alice"}`.

## Spread

```aura
let list1 = [1, 2]
let list2 = [3, 4]
let combined = [*list1, *list2, 5]     // [1, 2, 3, 4, 5]

let defaults = {a: 1, b: 2}
let overrides = {b: 9}
let merged = {**defaults, **overrides}  // {a: 1, b: 9}
```

Um spread nu em posição de valor é erro de parse; o spread é permitido apenas em
um literal ou em um argumento de chamada.

## Comprehensions

Comprehensions constroem uma coleção a partir de um iterável, com filtros
opcionais:

```aura
let squares = [x * x for x in range(6)]
let evens = [x for x in range(20) if x % 2 == 0]
let lengths = {w: w.length() for w in ["ab", "xyz"]}
let unique_mods = {x % 3 for x in range(10)}
```

```text
[0, 1, 4, 9, 16, 25]
[0, 2, 4, 6, 8, 10, 12, 14, 16, 18]
{'ab': 2, 'xyz': 3}
{0, 1, 2}
```

Múltiplas cláusulas `for` e `if` são suportadas e preservam a ordem da fonte.
Comprehensions são eager; uma expressão geradora `(e for x in xs)` é lazy.

## Strings

Strings são Unicode e imutáveis, com métodos de conveniência com nomes Aura que
mapeiam para Python:

```aura
let text = "Hello, World"

print(text.to_upper())          // HELLO, WORLD
print(text.to_lower())          // hello, world
print(text.starts_with("He"))   // true
print(text.ends_with("ld"))     // true
print(text.trim())              // trims whitespace
print(text.slice(1, 4))         // ell
print(text.char_at(0))          // H
print(text.length())            // 12
```

Os aliases de Aura incluem `starts_with`/`ends_with`, `to_upper`/`to_lower`,
`trim`/`trim_left`/`trim_right`, `index_of`, `is_alpha`, `is_digit` e mais.

## Passando coleções para funções

Coleções são valores de **referência**: mutar uma dentro de uma função é visível
ao chamador, enquanto reatribuir o parâmetro não é.

```aura
def mutate(xs) { xs.add(9) }
def reassign(x) { x = 9 }

def main() {
  let a = [1]
  mutate(a)
  print(a)          // [1, 9]

  let n = 1
  reassign(n)
  print(n)          // 1
}
```

## O que você aprendeu

* Listas, dicts, sets, tuplas; indexação, slices e range-como-slice.
* Conveniências de tamanho/pertinência/mutação e passthrough para Python.
* Spreads e comprehensions.
* Coleções são referências; números e strings são valores.

## Próximo passo

[Lambdas e Programação Funcional →](10-lambdas-and-functional.md)
