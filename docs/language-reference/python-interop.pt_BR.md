---
layout: default
title: "Interop com Python"
nav_exclude: true
---

[English](python-interop.md) | [Português](python-interop.pt_BR.md)

# Interop com Python

**Status:** Stable · **Evidência:** `aura/parser/to_ast.py` (`parse_import_stmt`,
`parse_from_import_stmt`, `_split_python_prefix`),
`aura/transpiler/transformers/statements.py` (`_transform_python_import`),
`aura/stdlib/python.py` (a ponte dinâmica).

Aura transpila para Python e roda no CPython, então todo módulo Python é
alcançável. O interop é **explícito**: um módulo hospedeiro é importado através do
prefixo `py.`, que separa módulos Python de módulos Aura no local da chamada.

---

## 1. Importar um módulo Python

```aura
import py.re                 // binds `re`
import py.os.path            // binds `path` (the last segment)
import py.re as regex        // binds `regex`
import py.re, py.json        // binds `re` and `json`
```

Regras:

* O prefixo `py.` marca um módulo **Python hospedeiro**. Um `import re` simples
  (sem prefixo) é um módulo **Aura** — os dois nunca se misturam por acidente.
* O nome ligado é o **último segmento do caminho**: `import py.os.path` liga
  `path`, não `os` (isso difere do `import os.path` do Python, que liga `os`).
  Um alias `as` explícito sempre vence.
* O prefixo `py.` é removido antes da geração de código: `import py.re` emite
  `import re as re`; `import py.os.path` emite `import os.path as path`.

```aura
import py.re as re
import py.json

def main() {
  print(re.match("a+", "aaa") != none)   // True
  print(json.loads('{"n": 1}')["n"])     // 1
}
```

---

## 2. Importar nomes de um módulo Python

```aura
from py.math import sqrt, pi as PI
from py.re import match as re_match, sub as re_sub
```

Um nome que colide com uma keyword de Aura **não pode** ser ligado sob esse nome;
use um alias:

```aura
// from py.re import type         // error: 'type' is a reserved keyword
from py.re import type as re_type  // ok
```

Keywords que exigem um alias incluem `type`, `from`, `in`, `is`, `class`,
`match`, e toda outra palavra reservada. Quando um nome Python não tem alias
usável — ou o nome só é conhecido em runtime — alcance-o através da ponte
dinâmica (§4).

---

## 3. Módulos Aura locais vs módulos Python

| Forma | Resolve para |
|---|---|
| `import util` / `import pkg.util` | um módulo Aura (um arquivo `.aura` irmão ou pacote) |
| `from util import x` | um nome em um módulo Aura |
| `import stdlib.math` | a biblioteca padrão Aura (`aura/stdlib`) |
| `import py.re` / `from py.math import sqrt` | um módulo Python hospedeiro |

Um import `py.` é sempre tratado como externo — nunca é confundido com um irmão
Aura, mesmo que exista um módulo Aura com o mesmo nome.

---

## 4. A ponte dinâmica (`python`)

Para acesso em runtime — um nome de módulo construído em tempo de execução, um
atributo que colide com keyword, ou introspecção — importe a ponte `python`:

```aura
import python

let re = python.import_module("re")   // dynamic import
let math = python.load("math")        // alias of import_module
print(math.sqrt(2))                   // 1.4142135623730951

print(python.eval("1 + 2"))           // 3
print(python.type_name(42))           // "builtins.int"
print(python.is_available("requests"))// True/False, never raises
print(python.getattr(re, "type"))     // attribute with a keyword name
```

A ponte é um wrapper fino e total; falhas levantam exceções Python comuns, então
o `try`/`catch` de Aura funciona como esperado.

### 4.1 Superfície da ponte

| Função | Propósito |
|---|---|
| `import_module(name)` / `load(name)` | importa um módulo por nome, retorna um proxy de módulo |
| `reload(module)` | recarrega um módulo |
| `is_available(name)` | `True`/`False`, nunca levanta |
| `eval(expr)` | avalia uma string de expressão |
| `exec_code(src)` / `compile_source(src)` | executa / compila código-fonte |
| `call(func, *args)` | chama qualquer chamável Python |
| `getattr(obj, name, default=None)` | acesso a atributo por nome (escapa colisões de keyword) |
| `setattr(obj, name, value)` | atribuição de atributo por nome |
| `hasattr(obj, name)` / `dir(obj)` | introspecção |
| `type_name(obj)` | nome de tipo totalmente qualificado |
| `is_module` / `is_callable` / `is_class` / `is_instance` | predicados |
| `to_aura(obj)` / `to_python(obj)` | converte entre os dois mundos |
| `add_path(path)` / `site_packages()` / `modules()` | ambiente e descoberta |
| `interpreter_version()` | a string de versão do CPython |

---

## 5. Escolher entre `py.` e `python.*`

* Use `import py.re` / `from py.math import sqrt` quando o módulo e os nomes são
  conhecidos em tempo de compilação — é direto, rápido e legível.
* Use `import python` (a ponte) para nomes dinâmicos, `eval`, introspecção, ou um
  atributo cujo nome é uma keyword de Aura (`python.getattr(mod, "type")`).

Ambas as formas rebaixam para imports/chamadas Python comuns; Aura não adiciona
nenhuma camada de runtime além da remoção do prefixo `py.` e dos helpers da
ponte.

---

## 6. O que NÃO faz parte do interop

| Grafia | Por quê | Use em vez disso |
|---|---|---|
| `import python` alcançando internals arbitrários do CPython diretamente | a ponte é a superfície suportada | `import python` então `python.*` |
| `import re` esperando um módulo Python | imports simples são módulos Aura | `import py.re` |
| `from re import type` | `type` é uma keyword reservada | `from py.re import type as re_type` |
| `python.import_module` no topo do módulo sem `import python` | a ponte deve ser importada | `import python` primeiro |
