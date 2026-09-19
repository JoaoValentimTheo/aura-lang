---
layout: default
title: "Aura Language Reference"
nav_exclude: true
---

[English](index.md) · [Português](index.pt_BR.md)

# Aura Language Reference

**Versão:** 0.2.0a4 · **Extraída de:** `aura/parser/to_ast.py` e
`aura/transpiler/` · **Modelo:** espelha a referência da linguagem Kof.

Este diretório é a **referência da linguagem Aura**. Ela descreve *o que é um
programa Aura válido* e *qual é o significado desse programa*, independentemente
da toolchain que o implementa.

> **Regra fundadora:** nada aqui é inventado. Cada regra é extraída do parser,
> do transpiler, dos testes ou de comportamento verificado por execução. Onde o
> comportamento não pôde ser determinado com segurança, a regra está marcada
> **UNSPECIFIED** em vez de adivinhada.

---

## Linguagem ≠ Compilador ≠ Target

Três níveis distintos, mantidos separados de propósito:

- **Aura** é a *linguagem* — um conjunto de regras.
- **A toolchain Aura** (`aura/parser`, `aura/transpiler`, a CLI) é *uma
  implementação* da linguagem.
- **CPython** é o *target*: Aura transpila para Python. Quando um comportamento
  observável vem do host (ex.: sinal de `%`, protocolo de iterador), ele é
  documentado aqui como uma regra *target-dependent*, nunca escondida.

---

## O que cada documento responde

| Documento | Pergunta que responde |
|---|---|
| [lexical-structure.md](lexical-structure.md) | Quais são os tokens válidos? (identificadores, literais, operadores, comentários, keywords) |
| [grammar.md](grammar.md) | Qual é a gramática formal? (EBNF, precedência, associatividade) |
| [syntax.md](syntax.md) | Como se escreve cada construção? (forma concreta, exemplos) |
| [types.md](types.md) | Quais tipos existem e como se escrevem? |
| [type-system.md](type-system.md) | Quais operações são válidas? Quando há erro de tipo? |
| [expressions.md](expressions.md) | Semântica de cada expressão e operador. |
| [statements.md](statements.md) | Semântica de cada statement e controle de fluxo. |
| [functions.md](functions.md) | Declaração, parâmetros, retorno, recursão, ponto de entrada. |
| [closures.md](closures.md) | Lambdas, function types, captura de variáveis. |
| [classes.md](classes.md) | Classes, traits, classes abstratas, herança, visibilidade. |
| [modules.md](modules.md) | Módulos, imports, re-exports, resolução de nomes. |
| [semantics.md](semantics.md) | Modelo de execução, ordem de avaliação, escopo, tempo de vida, erros. |
| [python-interop.md](python-interop.md) | Acesso ao Python hospedeiro pelo namespace `py.`. |

A **arquitetura da toolchain** tem documento próprio:
[../DESIGN.md](../DESIGN.md).

---

## Etiquetas de status

| Etiqueta | Significado |
|---|---|
| **Stable** | Definido pela linguagem e congelado (o congelamento de sintaxe, `0.2.0a1`). Não muda sem bump de versão + migração. |
| **Experimental** | Implementado e testável, mas sujeito a mudança. |
| **Target-specific** | Comportamento vem do host CPython, documentado como regra. |
| **Unspecified** | A linguagem ainda não define este ponto. |

---

## Como esta referência é verificável

Toda afirmação normativa aponta para uma **evidência**:

- **Código** — `aura/parser/to_ast.py:linha` ou `aura/transpiler/...:linha`.
- **Teste** — um teste em `tests/` que demonstra a regra.
- **Execução** — comportamento observado rodando um programa (*probe*), usado
  quando não há teste dedicado.

Quando código, teste e documentação divergem, a divergência é um bug a corrigir
— nunca resolvida silenciosamente a favor de uma das fontes.

---

## Trilha de aprendizado

Novo em Aura? Leia primeiro [`../learn/`](../learn/index.md) — um tutorial
numerado que constrói a linguagem — e depois use esta referência para as regras
exatas.
