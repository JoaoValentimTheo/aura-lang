---
layout: default
title: "14 — Interop com Python"
nav_exclude: true
---

[English](14-python-interop.md) · [Português](14-python-interop.pt_BR.md)

# 14 — Interop com Python

> **Meta do capítulo:** chamar o ecossistema Python a partir de Aura. Referência:
> [`../language-reference/python-interop.md`](../language-reference/python-interop.md).
> Exemplo: `../../examples/python_interop.aura`.

## Por que o interop é explícito

Aura roda no CPython, então todo módulo Python instalado é alcançável. Mas um
`import re` simples é um módulo **Aura**, não um Python. Para importar um módulo
**Python hospedeiro**, prefixe o caminho com `py.`:

```aura
import py.math as math
import py.os as os
from py.json import dumps, loads

def main() {
  print(math.sqrt(16))          // 4.0
  print(os.name)                // posix
  let encoded = dumps({"n": 1})
  print(loads(encoded)["n"])    // 1
}
```

O prefixo `py.` é removido antes da geração de código, então
`import py.math as math` emite `import math as math`.

## Regras de binding

| Forma | Nome ligado |
|---|---|
| `import py.re` | `re` (o **último** segmento do caminho) |
| `import py.os.path` | `path` — não `os` |
| `import py.re as regex` | `regex` (um alias explícito vence) |
| `from py.math import sqrt, pi as PI` | `sqrt`, `PI` |

**Colisões de keyword devem ser aliasadas.** Um nome Python que é uma keyword de
Aura não pode ser ligado sob esse nome. Aqui `type` é uma keyword de Aura, então
deve ser aliasado (este roda):

```aura
from py.builtins import type as py_type

def main() {
  print(py_type(5))            // <class 'int'>
}
```

Sem o alias o parser rejeita o binding:

```aura
// from py.re import type          // error: 'type' is a reserved keyword
// from py.re import type as re_type  // ok — aliased
```

Nomes que comumente precisam de um alias: `type`, `from`, `in`, `is`, `class`,
`match`, `new`, `self`, e toda outra palavra reservada. Quando nenhum alias é
possível — ou o nome só é conhecido em runtime — use a ponte dinâmica.

## A ponte `python`

Para acesso em runtime, importe a ponte `python`:

```aura
import python

def main() {
  let re = python.import_module("re")     // dynamic import
  print(python.hasattr(re, "match"))      // True
  print(python.eval("1 + 2"))             // 3
  print(python.type_name(42))             // builtins.int
  print(python.is_available("requests"))  // True/False, never raises
}
```

| Função | Propósito |
|---|---|
| `import_module(name)` / `load(name)` | importa um módulo por nome |
| `reload(module)` | recarrega um módulo |
| `is_available(name)` | `True`/`False`, nunca levanta |
| `eval(expr)` | avalia uma string de expressão |
| `exec_code(src)` / `compile_source(src)` | executa / compila código-fonte |
| `call(func, *args)` | chama qualquer chamável Python |
| `getattr(obj, name, default)` | atributo por nome (escapa colisões) |
| `setattr(obj, name, value)` | atribuição de atributo por nome |
| `hasattr(obj, name)` / `dir(obj)` | introspecção |
| `type_name(obj)` | nome de tipo totalmente qualificado |
| `to_aura(obj)` / `to_python(obj)` | converte entre os dois mundos |
| `add_path(path)` / `site_packages()` / `modules()` | ambiente e descoberta |

A ponte é um wrapper fino e total: falhas levantam exceções Python comuns, então
o `try`/`catch` de Aura funciona como esperado.

## Escolhendo

* Use `import py.x` / `from py.x import y` quando o módulo e os nomes são
  conhecidos em tempo de compilação — direto, rápido, legível.
* Use `import python` para nomes dinâmicos, `eval`, introspecção, ou um atributo
  que colide com keyword (`python.getattr(mod, "type")`).

## Um exemplo de framework

Como um import `py.` é um import Python real, bibliotecas padrão e frameworks
funcionam diretamente. Aqui está um valor construído com `py.json` e lido de
volta através de indexação Aura comum:

```aura
import py.json as json

def main() {
  let payload = {"name": "aura", "ok": true}
  let encoded = json.dumps(payload)
  print(encoded)                       // {"name": "aura", "ok": true}
  let decoded = json.loads(encoded)
  print(decoded["name"], decoded["ok"]) // aura True
}
```

## Coleções Python são coleções Aura

Valores cruzam a fronteira inalterados: um `dict` Python é um dict Aura, um
`list` Python é uma lista Aura. Então as conveniências de Aura se aplicam a
objetos retornados:

```aura
import py.json as json

def main() {
  let data = json.loads('{"items": [1, 2, 3]}')
  print(data["items"].length())    // 3
}
```

## Erros através da fronteira

Uma exceção Python levantada por uma chamada ao host é uma exceção comum para
Aura, então `try`/`catch` funciona. Capture `Error` para qualquer exceção, ou um
tipo específico:

```aura
import py.json as json

def main() {
  let data = try { json.loads("not json") } catch Error as e { none }
  print(data is none)              // true
}
```

A própria ponte é um wrapper fino: ela não engole falhas, deixa a exceção Python
subjacente propagar.

## O que NÃO é interop

| Grafia | Por quê | Use em vez disso |
|---|---|---|
| `import re` esperando Python | imports simples são módulos Aura | `import py.re` |
| `from re import type` | `type` é reservado | `from py.re import type as re_type` |
| `python.import_module(...)` sem `import python` | a ponte deve ser importada | `import python` primeiro |
| internals brutos do CPython através da ponte | a ponte é a superfície suportada | os helpers listados |

## O que você aprendeu

* `py.` marca um módulo Python hospedeiro; o nome ligado é o último segmento.
* Colisões de keyword exigem um alias.
* A ponte `python` para acesso dinâmico e introspecção.
* Valores cruzam a fronteira inalterados.

## Próximo passo

[Macros e Decorators →](15-macros-and-decorators.md)
