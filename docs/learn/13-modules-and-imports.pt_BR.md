---
layout: default
title: "13 — Módulos e Imports"
nav_exclude: true
---

[English](13-modules-and-imports.md) · [Português](13-modules-and-imports.pt_BR.md)

# 13 — Módulos e Imports

> **Meta do capítulo:** organizar código entre arquivos e namespaces. Referência:
> [`../language-reference/modules.md`](../language-reference/modules.md).

## Arquivos são módulos

Qualquer arquivo `.aura` pode ser importado. Um arquivo importado como módulo
**não precisa de `main`**.

```aura
// lib.aura
def greet(name: str) -> str {
  return "hi " + name
}

const VERSION = "1.0"
```

```aura
// app.aura
import lib
from lib import greet as g

def main() {
  print(lib.greet("ana"))     // hi ana
  print(g("bob"))             // hi bob
  print(lib.VERSION)          // 1.0
}
```

`import lib` liga o arquivo `lib.aura`; `import pkg.util` liga
`pkg/util.aura`. O runtime instala um import hook que mapeia o caminho Aura
pontuado para o arquivo irmão.

## A declaração `module`

`module Name { ... }` cria um grupo com namespace cujas funções são estáticas.
Membros são **privados ao arquivo declarante a menos que marcados `export`**:

```aura skip
module Greeter {
  export def hello(name: str) -> str {
    return "hi " + name
  }

  let mut count = 0

  export def bump() -> int {
    count = count + 1
    return count
  }
}

def main() {
  print(Greeter.hello("ana"))    // hi ana
  print(Greeter.bump())          // 1
  print(Greeter.bump())          // 2
}
```

Regras:

* `export` precede uma declaração nomeada (`def`, `class`, `trait`, `enum`,
  `type`, `let`, `const`, um `module` aninhado) ou introduz um re-export.
* Um membro não exportado é **mangled** no Python gerado, então a privacidade é
  aplicada em runtime também; alcançá-lo de fora é `E308`.
* O estado do módulo **não é gravável de fora** (`E303`); mute-o através de uma
  função exportada.
* Um `main` dentro de um corpo de módulo é `E312`.

```aura skip
module M {
  def hidden() -> int { return 1 }
}

def main() {
  print(M.hidden())    // E308: 'hidden' is not exported from module 'M'
}
```

## Nomes de módulo pontuados

`module App.Services { ... }` se aninha, alcançado como `App.Services.member`:

```aura skip
module Outer.Inner {
  export def value() -> int { return 3 }
}

def main() { print(Outer.Inner.value()) }    // 3
```

## Grafias de import

| Forma | Exemplo | Vincula |
|---|---|---|
| Módulo | `import stdlib.math` | caminho `stdlib.math` |
| Alias de módulo | `import stdlib.math as m` | `m` |
| Nomes | `from stdlib.math import sqrt, PI` | `sqrt`, `PI` |
| Alias de nome | `from stdlib.math import sqrt as root` | `root` |
| Forma com chaves | `import stdlib.math { sqrt, PI }` | `sqrt`, `PI` (≡ `from`) |
| Wildcard | `from stdlib.math import *` | todos os nomes públicos |
| Múltiplos | `import a, b as c` | `a`, `c` |

```aura
import stdlib.math as m
from stdlib.math import sqrt, PI

def main() {
  print(m.sqrt(16))     // 4.0
  print(sqrt(36))       // 6.0
  print(PI)             // 3.141592653589793
}
```

A forma com chaves não pode ser combinada com `as` no mesmo statement. Não há
separador `::` — caminhos de módulo são pontuados. `*` é escrito apenas como
`from module import *`.

## Facades (re-exports)

Um `module` pode re-exportar símbolos de arquivos irmãos. Coloque a facade em uma
pasta com o nome dela:

```text
App/
  App.aura          module App { export Components, Utils }
  components.aura   class Components { ... }
  utils.aura        def double(...) / const VERSION
main.aura           import App
```

```aura skip
// App/App.aura
module App {
  export Components, Utils
}
```

```aura
// main.aura
import App

def main() {
  print(App.Components("header").describe())
  print(App.Utils.double(21))
  print(App.Utils.VERSION)
}
```

Uma fonte explícita é aceita com `export Name from "module"`. O caminho deve ser
um nome pontuado simples; separadores, `..` e caminhos absolutos são rejeitados.
Um re-export não resolvido é `E313`.

## Resolução de nomes

Fora de um módulo, um nome nu resolve através do escopo local, depois da função
envolvente, depois das declarações de nível superior, depois dos imports, depois
de um arquivo `.aura` irmão, depois de um módulo hospedeiro `py.`. Dentro de
`module M`, os próprios membros de `M` vencem.
`UNSPECIFIED`: Aura não diagnostica um import ambíguo; o binding Python emitido
decide ("o último binding vence").

## O que você aprendeu

* Qualquer arquivo `.aura` é importável; arquivos importados não precisam de
  `main`.
* `module Name { export ... }`; `export` e privacidade (`E308`, `E303`, `E312`).
* Módulos pontuados; todas as grafias de import; facades e re-exports.
* Aproximadamente como nomes resolvem.

## Próximo passo

[Interop com Python →](14-python-interop.md)
