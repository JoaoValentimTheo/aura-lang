---
layout: default
title: "00 — Introdução"
nav_exclude: true
---

[English](00-introduction.md) · [Português](00-introduction.pt_BR.md)

# 00 — Introdução

> **Aura 0.2.0a8.** Aura transpila para Python e roda no CPython. Este capítulo
> explica o modelo mental; os capítulos 01–16 ensinam a linguagem.

## O que é Aura

Aura é uma **linguagem de programação** com sintaxe própria, transpilada para
código-fonte Python e executada pelo CPython. Quando você escreve:

```aura
def main() {
  print("Hello, Aura!")
}
```

a toolchain o parseia, o checa, emite Python e roda esse Python. A linguagem é
pequena de propósito; o poder vem do target, já que **todo módulo Python é
alcançável** (capítulo 14).

Três níveis são mantidos separados, e a distinção importa por todo este
tutorial:

| Nível | O que é |
|---|---|
| **Aura** | a linguagem — as regras contra as quais você escreve |
| **A toolchain Aura** | o parser, checker, transpiler e CLI (`aura ...`) |
| **CPython** | o target que de fato roda o Python emitido |

## Por que transpilar para Python?

Porque você obtém o ecossistema Python e o runtime do CPython sem a sintaxe de
superfície do Python. Uma biblioteca que você já tem — `json`, `re`, `requests`,
um framework web — é chamável de Aura através do prefixo `py.`:

```aura
import py.math as math

def main() {
  print(math.sqrt(16))   // 4.0
}
```

## O que Aura muda, e o que mantém

Aura **não** é um dialeto de Python com alguns renames. O modelo mental é uma
linguagem que deliberadamente:

* exige `let`/`let mut`/`const` em vez de atribuição nua, então *a mutabilidade é
  visível na declaração* (`x = 1` sozinho não é uma declaração);
* usa `def` — não há `fn`, `fun`, `lambda` nem `function`;
* escreve tipos com colchetes: `Box[T]`, `[int]`, `{str: int}`; `<T>` não existe;
* escreve o valor nulo como `none`; `null`, `None` e `True`/`False` são
  rejeitados;
* usa `and`, `or`, `not`; `&&`, `||`, `!` são rejeitados;
* usa `extends` para herança; `implements` e `class C(A)` são rejeitados;
* faz todo membro de classe declarar uma visibilidade
  (`public`/`private`/`protected`);
* tem recursos reais de linguagem que o Python não tem como sintaxe: `match` com
  guards e desestruturação, `unless`, `until`, `guard`, `loop`, `?.`, `??`, `?:`,
  `|>`.

Como o target é CPython, parte do comportamento é herdada em vez de inventada:

| Comportamento | Regra |
|---|---|
| Inteiros | precisão arbitrária (o `int` do Python) |
| `7 / 2` | `3.5` — divisão verdadeira; **não** há operador floor `//` |
| `-7 % 3` | `2` — o sinal segue o divisor |
| Ordenação de dict | ordem de inserção |
| Tempo de vida de objeto | contagem de referências + GC do CPython |
| `str` | Unicode |

Estas são regras **target-specific**, documentadas na referência em vez de
escondidas. Veja
[`../language-reference/semantics.md`](../language-reference/semantics.md) §6.

## Um primeiro programa real

```aura
def classify(score) -> str {
  match score {
    case s if s >= 90 { return "A" }
    case s if s >= 80 { return "B" }
    case _ { return "F" }
  }
  return "?"
}

def main() {
  for score in [95, 83, 40] {
    print(f"{score}: {classify(score)}")
  }
}
```

```text
95: A
83: B
40: F
```

Isso já mostra várias ideias de Aura de uma vez: `def`, `match` com guards,
f-strings, `for ... in` e um ponto de entrada `main`.

## O ponto de entrada

Um arquivo executado com `aura run` **deve** declarar um `def main()` de nível
superior. O runtime chama `main` para você — nunca escreva uma chamada `main()`
à direita. Um arquivo usado apenas como import (um módulo) não precisa de `main`.

```aura
def main() {
  // your program
}
```

`main` pode não receber parâmetros ou receber um único parâmetro `args`, pode ser
`async` e pode retornar um `int` para definir o código de saída do processo.
Referência:
[`../language-reference/functions.md`](../language-reference/functions.md) §9.

## Como esta trilha é organizada

* Os capítulos **00–06** ensinam o núcleo: variáveis, tipos, controle de fluxo,
  funções.
* Os capítulos **07–08** cobrem orientação a objetos.
* Os capítulos **09–10** cobrem coleções e programação funcional.
* Os capítulos **11–12** cobrem pattern matching e erros.
* Os capítulos **13–16** cobrem módulos, interop com Python,
  decorators/macros e a CLI.

Todo capítulo termina com um link **Próximo passo** e, onde relevante, um
ponteiro para a regra exata na referência da linguagem. O tutorial ensina; a
referência define. Quando você precisar de uma resposta precisa sobre uma
construção, vá à referência.

## Próximo passo

[Instalação →](01-installation.md)
