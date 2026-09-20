---
layout: default
title: "Aprenda Aura"
nav_exclude: true
---

[English](index.md) · [Português](index.pt_BR.md)

# Aprenda Aura

Bem-vindo à trilha de aprendizado do Aura.

Aura é uma linguagem de programação pequena e legível que **transpila para
Python** e roda no CPython. Ela mantém o ecossistema Python — todo módulo Python
instalado é alcançável — ao mesmo tempo em que oferece uma sintaxe de superfície
mais limpa e explícita: `let`/`const` em vez de atribuição nua, `def` para
funções, classes com visibilidade obrigatória, pattern matching, lambdas com
`=>`, um operador pipe e um valor nulo chamado `none`.

Como o target é CPython, um programa Aura é Python comum em runtime: `int` é o
inteiro de precisão arbitrária do Python, `/` é divisão verdadeira, `%` segue o
sinal do divisor, e dicionários mantêm a ordem de inserção. As diferenças estão
na **sintaxe e na checagem**, não em um novo runtime.

## O que Aura é hoje

**Versão:** `0.2.0a4`. A referência da linguagem em
[`../language-reference/`](../language-reference/index.md) é a fonte de verdade
exata; este tutorial constrói intuição e aponta para lá para detalhes.

* Gradualmente tipada: anotações são opcionais e, exceto pelas checagens no type
  checker, **apagadas** antes do emit Python.
* Funções declaradas com `def`, corpos de expressão (`def f(x) = x * 2`),
  defaults, variádicos, recursão, genéricos (`def id[T](x: T) -> T`).
* Classes com header fields, `def new` manual, visibilidade
  (`public`/`private`/`protected`), herança com `extends`, `trait`,
  `abstract class`, `enum`, `@property`/`@staticmethod`/`@classmethod`.
* Coleções: listas `[T]`, dicts `{K: V}`, sets, tuplas, slices e comprehensions.
* Lambdas (`x => x * 2`), closures, tipos de função e o operador pipe `|>`.
* `if`/`else if`, `unless`, `guard ... else`, `while`, `until`, `loop`, `for` com
  ranges e labels, e `match` com guards e desestruturação.
* `try`/`catch`/`finally`, `throw`, `assert`, `with`.
* Módulos (`module Name { export ... }`), imports `.aura` locais e interop com
  Python através do prefixo `py.` e da ponte `python`.
* Decorators: `@debug`, `@timeit`, `@memoize`, `@cache(...)`, mais macros de tempo
  de compilação (`assert_eq`, `static_swap`, `stringify`, ...).

## Para quem é

* **Iniciantes** — comece no capítulo 00 e siga a ordem. Cada capítulo é curto e
  todo trecho roda.
* **Desenvolvedores Python** — comece pela introdução, depois leia o que é
  *diferente* do Python (os capítulos 03–06 são em sua maioria familiares).
* **Contribuidores** — depois do tutorial, leia a referência da linguagem e
  [`../DESIGN.md`](../DESIGN.md).

## Ordem recomendada

```
00 → 01 → 02 → 03 → 04 → 05 → 06 → 07 → 08 → 09
   → 10 → 11 → 12 → 13 → 14 → 15 → 16
```

Os capítulos 00–06 são a linguagem central. 07–08 cobrem orientação a objetos.
09–10 cobrem dados e programação funcional. 11–12 cobrem controle e falha. 13–16
cobrem módulos, interop com Python, decorators/macros e a toolchain.

## Índice

| # | Capítulo | Em uma linha |
|---|---------|----------|
| 00 | [Introdução](00-introduction.md) | O que Aura é, e como se relaciona com Python |
| 01 | [Instalação](01-installation.md) | Instale, crie um projeto, rode-o |
| 02 | [Primeiro Programa](02-first-program.md) | `hello.aura` linha por linha |
| 03 | [Noções Básicas](03-language-basics.md) | `let`, `let mut`, `const`, `def`, `main`, `print` |
| 04 | [Variáveis e Tipos](04-variables-and-types.md) | O catálogo de tipos e anotações |
| 05 | [Controle de Fluxo](05-control-flow.md) | `if`, loops, visão geral de `match` |
| 06 | [Funções](06-functions.md) | Parâmetros, defaults, variádicos, recursão |
| 07 | [Classes e Objetos](07-classes-and-objects.md) | Construtores, campos, métodos, properties |
| 08 | [Traits e Classes Abstratas](08-traits-and-abstract-classes.md) | Contratos e polimorfismo |
| 09 | [Coleções](09-collections.md) | Listas, dicts, sets, tuplas, comprehensions |
| 10 | [Lambdas e Funcional](10-lambdas-and-functional.md) | `=>`, closures, `map`/`filter`/`reduce`, `&#124;>` |
| 11 | [Pattern Matching](11-pattern-matching.md) | `match`, guards, desestruturação, enums |
| 12 | [Tratamento de Erros](12-error-handling.md) | `try`/`catch`, `throw`, `guard`, `assert` |
| 13 | [Módulos e Imports](13-modules-and-imports.md) | `module`, `export`, imports |
| 14 | [Interop com Python](14-python-interop.md) | imports `py.` e a ponte `python` |
| 15 | [Macros e Decorators](15-macros-and-decorators.md) | Decorators de runtime e macros de tempo de compilação |
| 16 | [CLI e Tooling](16-cli-and-tooling.md) | `run`, `check`, `repl`, `init`, `format`, ... |

## Exemplos executáveis

O repositório traz programas executáveis em [`../../examples/`](https://github.com/JoaoValentimTheo/aura-lang/tree/master/examples).
Cada capítulo aponta o exemplo mais próximo de seu tema; o conjunto completo é:

| Exemplo | Capítulo | O que mostra |
|---|---|---|
| `hello.aura` | 02 | o menor programa |
| `fibonacci.aura` | 05, 06 | loops e bindings mutáveis |
| `prime_checker.aura` | 05, 09 | funções e comprehensions |
| `functional.aura` | 10 | lambdas, `&#124;>`, closures |
| `pattern_matching.aura` | 11 | `match` com guards e desestruturação |
| `error_handling.aura` | 12 | `try`/`catch`/`finally`, `guard`, `throw` |
| `classes.aura` | 07, 08 | classes, `extends`, `@property`, visibilidade |
| `macros.aura` | 15 | decorators de runtime embutidos |
| `compile_time_macros.aura` | 15 | expansões de macro de tempo de compilação |
| `crypto.aura` | — | hashing e primitivas pós-quânticas de `stdlib.crypto` |
| `python_interop.aura` | 14 | imports `py.` e a ponte `python` |
| `abstract_classes.aura` | 08 | `abstract class` e polimorfismo |
| `tour.aura` | todos | um tour em arquivo único da linguagem |

Rode qualquer um deles diretamente:

```bash
aura run examples/tour.aura
```

## Etiquetas de status usadas nesta trilha

* **Verified** — o trecho foi executado com `aura run` durante a escrita deste
  tutorial.
* **Reference** — um link para a regra exata vive em
  [`../language-reference/`](../language-reference/index.md); o tutorial não a
  repete em profundidade.
* **Limitation** — comportamento que a referência descreve mas que o build atual
  rejeita ou não aplica; apontado onde importa.
* **Gotcha** — um erro comum ou comportamento surpreendente vale saber.
* **UNSPECIFIED** — a linguagem ainda não define este ponto; o comportamento pode
  mudar.
* **TARGET-SPECIFIC** — comportamento vem do host CPython, documentado como uma
  regra.

Comece aqui: [Introdução →](00-introduction.md)
