---
layout: default
title: "Módulos, Pacotes e Imports"
nav_exclude: true
---

[English](modules.md) · [Português](modules.pt_BR.md)

# Módulos, Pacotes e Imports

**Status:** Stable (exceto onde rotulado) · **Evidência:**
`aura/parser/to_ast.py` (`parse_module_decl` §1567, `parse_import_stmt` §1825,
`parse_from_import_stmt` §1885, `_split_python_prefix` §494),
`aura/transpiler/transformers/statements.py` (`transform_Module` §1465,
`transform_ImportStmt` §1335, `transform_FromImport` §1390,
`_module_reexport_lines` §1569), `aura/transpiler/rules.py`
(`_check_no_main_in_modules` §246, `_check_reexports` §439,
`_check_const_member_assignment` §1288), `aura/transpiler/modules.py`
(`resolve_reexport` §93).

Um **módulo** Aura é um namespace nomeado declarado com `module Name { ... }`.
Um **arquivo** também é um módulo quando importado. Este documento define como um
módulo é declarado, como nomes cruzam uma fronteira de módulo e como imports são
escritos. O target é CPython, então um import acaba se tornando um `import`
Python.

---

## 1. Declaração de módulo

```
module-decl    = "module" , identifier , { "." , identifier } , "{" , { member } , "}" ;
member         = [ "export" ] , ( declaration | bare-export ) ;
bare-export    = identifier , { "," , identifier } , [ "from" , string ] , [ ";" ] ;
```

`module Name { ... }` cria uma classe com namespace cujas funções são estáticas
(`transform_Module`, `statements.py:1465-1501`). Membros podem ser `def`, `class`,
`trait`, `enum`, `type`, dados `let`/`const`, ou um `module` aninhado
(`LANGUAGE.md` §9).

```aura
module MyLib {
  export def public_function() -> int {
    return 42
  }

  export const VERSION = "1.0.0"

  // Not exported: visible only inside MyLib.
  let mut cache = 0

  def private_helper() -> int {
    return cache
  }

  export def refresh() -> int {
    cache = private_helper() + 1
    return cache
  }
}

def main() {
  print(MyLib.public_function())  // 42
  print(MyLib.VERSION)            // 1.0.0
  print(MyLib.refresh())          // 1
}
```

*Evidência:* `test_modules.py::TestModuleParsing::test_module_records_exports`,
`::test_exports_of_every_member_kind`; *probe* (emite `class MyLib:` com membros
`@staticmethod`, membro privado mangled para `_MyLib__cache`).

### 1.1 Nomes pontuados se aninham

`module App.Services { ... }` é um namespace aninhado alcançado como
`App.Services.member`. Ele transpila para classes Python aninhadas
(`transform_Module`, `statements.py:1494-1501`).

```aura
module Outer.Inner {
  export def value() -> int { return 3 }
}

def main() { print(Outer.Inner.value()) }   // 3
```

*Evidência:* `test_modules.py::test_dotted_module_name`,
`::test_dotted_module_emits_nested_classes`.

### 1.2 Membros privados são mangled

Um membro **não** marcado `export` é emitido sob um nome mangled (`_Lib__cache`),
então a privacidade é aplicada em runtime assim como em tempo de check
(`_module_class`, `statements.py:1536-1543`; `_rename_module_member` §1648).

*Evidência:* `test_modules.py::test_private_member_is_mangled`,
`::test_private_data_member_is_mangled`,
`::test_internal_call_to_private_member_uses_mangled_name`.

### 1.3 Um `main` dentro de um módulo é rejeitado

`main` pertence ao **arquivo de entrada**, nunca a um corpo de módulo. `def main`
dentro de `module Name { ... }` é **E312** (`_check_no_main_in_modules`,
`rules.py:246-265`).

```aura
module M {
  def main() { }   // E312: 'main' is declared inside module 'M'
}
```

*Evidência:* `test_module_facade.py::test_main_inside_a_module_reports_e312`;
*probe* (`E312`).

---

## 2. `export` e privacidade (E308)

Um membro de módulo é **privado ao arquivo declarante por padrão**. Adicione
`export` a um `def`, `class`, `trait`, `enum`, `type`, `let`, `const` ou
`module` aninhado para torná-lo público (`parse_module_decl`,
`to_ast.py:1582-1619`).

| Acesso | Resultado |
| --- | --- |
| Membro exportado, de qualquer lugar | OK |
| Membro privado, de dentro de seu próprio arquivo/módulo | OK |
| Membro privado, de fora do módulo | **E308** |
| `export` fora de um corpo de `module` | erro de parse |
| `export` sem declaração ou nome seguinte | erro de parse |

```aura
module M {
  def hidden() -> int { return 1 }
}

def main() {
  print(M.hidden())   // E308: 'hidden' is not exported from module 'M'
}
```

O diagnóstico nomeia o membro e o módulo, e sugere adicionar `export`
(`_visit_member_access`, `rules.py:1143-1149`).

*Evidência:* `test_modules.py::TestModuleRules::test_accessing_a_non_exported_member_reports_e308`,
`::test_e308_message_names_the_module_and_export`;
`test_modules.py::test_export_outside_module_is_rejected`,
`::test_export_without_a_name_is_rejected`.

> **Target-specific:** um membro privado de um arquivo **importado** é aplicado
> apenas pelo nome mangled em runtime; o checker não reinspeciona o módulo de
> outro arquivo. *Probe*: `lib.Lib.sec()` onde `sec` é privado em `lib.aura`
> reporta `OK` para `aura check app.aura` e falha em runtime com
> `AttributeError: type object 'Lib' has no attribute 'sec'`.

---

## 3. Estado de módulo não é gravável de fora (E303)

Um membro de módulo não pode ser atribuído de fora do módulo, mesmo um
`export`ado. Mute o estado do módulo através de uma função exportada, que pode
usar livremente membros `let mut` internamente (`_check_const_member_assignment`,
`rules.py:1288-1316`).

```aura
module Counters {
  let mut count = 0
  export def bump() -> int {
    count = count + 1
    return count
  }
}

def main() {
  Counters.bump()
  Counters.bump()
  // Counters.count = 9   // E303: module state is not writable from outside
}
```

Dentro do corpo do módulo (e dentro de suas funções) atribuir ao estado do módulo
é permitido (`_is_module_body`, `rules.py:1327-1330`), então o `bump` acima é
limpo.

*Evidência:* `test_modules.py::TestModuleRules::test_assigning_module_state_from_outside_reports_e303`,
`::test_internal_assignment_to_module_state_is_clean`;
`test_modules.py::TestModuleRuntime::test_module_private_state_is_mutable_internally`;
*probe* (`E303` em `M.count = 9`).

---

## 4. Imports

Aura tem quatro grafias de import, todas respaldadas pelo mesmo parser e todas
transpilando para imports Python (`parse_import_stmt`, `to_ast.py:1825-1869`;
`parse_from_import_stmt` §1885-1909).

```
import-decl    = "import" , module-path , ( [ "{" , import-item , { "," , import-item } , "}" ]
                 | import-list ) , [ "as" , identifier ] , [ ";" ] ;
import-list    = module-path , [ "as" , identifier ] , { "," , module-path , [ "as" , identifier ] } ;
import-item    = identifier , [ "as" , identifier ] ;
from-import    = "from" , module-path , "import" , ( "*" | import-item , { "," , import-item } ) , [ ";" ] ;
module-path    = identifier , { "." , identifier } ;
```

| Forma | Exemplo | Emite |
| --- | --- | --- |
| Módulo | `import stdlib.math` | `import stdlib.math` |
| Alias de módulo | `import stdlib.math as m` | `import stdlib.math as m` |
| Nomes | `from stdlib.math import sqrt, PI` | `from stdlib.math import sqrt, PI` |
| Alias de nome | `from stdlib.math import sqrt as root` | `from stdlib.math import sqrt as root` |
| Forma com chaves | `import stdlib.math { sqrt, PI }` | `from stdlib.math import sqrt, PI` |
| Wildcard | `from stdlib.math import *` | `from stdlib.math import *` |
| Múltiplos | `import a, b as c` | `import a, b as c` |

```aura
import stdlib.math
print(stdlib.math.sqrt(16))  // 4.0

import stdlib.math as m
print(m.sqrt(25))            // 5.0

from stdlib.math import sqrt, PI
print(sqrt(36))              // 6.0

import stdlib.math { sqrt, PI }   // equivalent to from stdlib.math import sqrt, PI
print(PI)                         // 3.141592653589793
```

Regras:

- A **forma com chaves não pode ser combinada com `as`** no mesmo statement
  (`LANGUAGE.md` §16). Um alias no módulo mais nomes selecionados
  (`import a.b as c { x }`) é tratado emitindo tanto um import aliasado quanto um
  `from ... import` (`transform_ImportStmt`, `statements.py:1358-1364`).
- **Não há separador `::`**; um caminho de módulo é pontuado
  (`to_ast.py:1828-1830`).
- Um wildcard é escrito apenas como `from module import *`; não há `*` na forma
  com chaves (`LANGUAGE.md` §16).
- Imports são **declarações de nível superior**; o parser os lê antes/no fluxo de
  statements do programa. Imports relativos e relativos ao pai não fazem parte da
  gramática.

*Evidência:* `syntax.md`6; *probe* (cada forma transpila como a tabela mostra).

### 4.1 Arquivos `.aura` locais e pacotes

Um arquivo Aura simples é importável com a mesma sintaxe. `import util` liga o
arquivo `util.aura`; `import pkg.util` liga `pkg/util.aura`; um arquivo importado
como módulo **não precisa de `main`** (`LANGUAGE.md` §16.2, §1). O runtime instala
um import hook que mapeia o caminho Aura pontuado para o arquivo `.aura` irmão
(`install_aura_import_hook`, chamado de `_install_aura_imports`,
`cli.py:118-127`).

```aura
// lib.aura — a plain module file needs no export marker at top level
def greet(name: str) -> str {
  return "hi " + name
}

const VERSION = "9"
```

```aura
// app.aura
import lib
from lib import greet as g

def main() {
  print(lib.greet("ana"))   // hi ana
  print(g("bob"))           // hi bob
}
```

*Evidência:* *probe* (imprime `hi ana` / `hi bob`, exit 0);
`test_module_facade.py::TestImportedFileRules::test_imported_library_needs_no_main`;
`test_modules.py::TestModuleImports::test_module_in_another_file`.

Um `export` de nível superior é **significativo apenas dentro de um corpo de
`module`** (`to_ast.py:775-780`). Um arquivo irmão que uma facade re-exporta é
escrito com declarações simples de nível superior, não `export` (veja §6).

*Probe:* `export def f() ...` no nível superior de um arquivo importado levanta
`SyntaxError: 'export' is only meaningful inside a 'module' body`.

### 4.2 Imports de interop Python (`py.`)

Um caminho de módulo prefixado com `py.` marca um import **Python hospedeiro** em
vez de um módulo Aura (`_split_python_prefix`, `to_ast.py:494-505`;
`_transform_python_import`, `statements.py:1371-1388`). Isso é coberto em
[python-interop.md](python-interop.md) §1–2.

```aura
import py.re             // binds `re`  (last path segment)
from py.math import pi   // binds `pi`
```

---

## 5. Resolução de nomes

Dentro de um corpo de módulo um nome de membro nu resolve para o namespace do
próprio módulo; o transformer o reescreve para `Module.name` durante o corpo
(`_module_scopes`, `statements.py:1531-1535`). Um membro privado resolve para seu
nome mangled (`_module_privates`, §1536-1545).

Para um identificador `x` escrito **fora** de um módulo, a ordem de resolução é:

```text
local scope (function → block chain)
  → enclosing function scope / closure cell
  → top-level declarations in the file
  → the module of an importing alias or a `from` binding
  → an Aura sibling file loadable by the import hook
  → a `py.` host module / the `python` bridge
  → otherwise: name error at runtime, or E3xx at check time
```

- Dentro de `module M`, os próprios membros de `M` vencem sobre declarações
  externas com o mesmo nome; o rule checker mantém escopos de módulo aninhados
  em uma pilha (`_module_depth`, `_check_duplicate_members`, `rules.py:623-631`,
  §910).
- Um membro de módulo pode compartilhar um nome de nível superior sem conflito.

*Evidência:* `test_modules.py::TestModuleRules::test_module_member_may_share_a_top_level_name`.

> **UNSPECIFIED:** a linguagem não define uma ordem total de resolução de nomes
> que resolva colisões entre bindings `import`/`from` e declarações de nível
> superior. A resolução segue, em última análise, o binding de nome Python emitido
> (o último binding vence no namespace do módulo). Aura não diagnostica um import
> ambíguo.

---

## 6. Facades de módulo (re-exports)

Um módulo pode atuar como a **facade** de uma pasta-fonte: um `export Name` nu
(sem `def`/`class`) re-exporta um símbolo definido em um arquivo irmão. Coloque a
facade em uma pasta com o nome dela, para que `App/App.aura` seja o ponto de
entrada do pacote `App` (`LANGUAGE.md` §9; `_parse_item_export`,
`to_ast.py:1633-1651`).

```text
App/
  App.aura          module App { export Components, Utils }
  components.aura   class Components { ... }
  utils.aura        def double(...) / const VERSION
main.aura           import App
```

```aura
// App/App.aura
module App {
  export Components, Utils
}
```

```aura
// main.aura
import App

def main() {
  print(App.Components("header").describe())  // a class from components.aura
  print(App.Utils.double(21))                 // the utils module as a namespace
  print(App.Utils.VERSION)
}
```

*Evidência:* `test_module_facade.py::TestFacadeRuntime::test_export_class_and_module`
(imprime `component:header` / `42` / `1.0.0`).

### 6.1 Ordem de resolução

Um nome re-exportado é resolvido por convenção, **deterministicamente**, contra a
pasta que contém a facade (`resolve_reexport`, `modules.py:93-130`):

1. uma declaração no mesmo arquivo (a facade re-exporta seu próprio membro);
2. um arquivo irmão cujo stem corresponde, sem distinção de maiúsculas/minúsculas
   (`Components` → `components.aura`);
3. uma subpasta com o nome do símbolo (`Components/Components.aura` ou
   `Components/__init__.aura`).

Quando o arquivo resolvido **declara** o nome, o binding é aquela declaração
(`App.Components` é a classe). Quando não declara, todo o módulo irmão é exposto
como o namespace (`App.Utils` é o módulo `utils`), então suas funções e constantes
são alcançadas como `App.Utils.double(...)` (`declares_name`, `modules.py:125-129`;
bindings construídos em `_module_reexport_lines`, `statements.py:1616-1622`).

A resolução é **estática e confinada à pasta da facade**: um caminho deve
permanecer dentro dela, então um re-export nunca pode ler fora do projeto
(`_safe_child`, `modules.py:41-51`). Nada é executado no momento da resolução;
apenas o texto do irmão é escaneado em busca de uma declaração (`_defines`,
§54-60).

### 6.2 Fonte explícita

Uma fonte explícita é aceita com `export Name from "module"`
(`_parse_item_export`, `to_ast.py:1639-1648`; `_candidate_files`,
`modules.py:63-75`):

```aura
module App {
  export Widgets from "widgets"      // resolves widgets.aura
  export X from "pkg.sub"            // resolves pkg/sub.aura
}
```

O caminho deve ser um **nome pontuado simples**: separadores (`/`, `\`),
travessia (`..`) e caminhos absolutos são rejeitados em tempo de parse
(`_INVALID_MODULE_PATH`, `to_ast.py:11-13`, §1646-1648).

*Evidência:* `test_module_facade.py::test_export_from_with_traversal_is_rejected`,
`::test_export_from_with_separator_is_rejected`,
`::test_reexport_source_cannot_traverse`,
`::test_reexport_cannot_reach_absolute_path`.

### 6.3 Diagnósticos

| Condição | Código | Onde |
| --- | --- | --- |
| Um re-export não resolve para nenhuma fonte irmã ou declaração local | **E313** | `_check_reexports`, `rules.py:439-467` |
| Um `main` dentro de um módulo (incluindo uma facade) | **E312** | `rules.py:246-265` |
| Dois re-exports resolvendo para arquivos diferentes sob um nome | conflito | `find_reexport_conflicts`, `modules.py:133-151` |

```aura
module App { export Missing }   // E313: module 'App' exports 'Missing', ...
```

*Evidência:* `test_module_facade.py::TestFacadeDiagnostics::test_unresolved_reexport_reports_e313`,
`::test_main_inside_a_facade_reports_e312`.

> **TARGET-SPECIFIC:** uma facade de pacote (`App/App.aura`) emite seus membros em
> **nível de módulo**, então o Python importa `App` diretamente; uma facade em um
> arquivo simples (`facade.aura`) mantém uma classe com namespace
> (`_is_package_facade`, `statements.py:1503-1513`). Imports de irmãos são
> hoisted para fora do corpo da classe para evitar sombreamento
> (`statements.py:1519-1527`, §1625-1642).

---

## 7. O que NÃO faz parte de módulos

| Não suportado | Em vez disso |
| --- | --- |
| keyword `namespace` | `module` |
| alias de re-export `export ... as` | binding `from` com um alias |
| `import a.b as c { x, y }` como uma única forma inequívoca | suportado emitindo dois statements Python (*superfície ambígua*) |
| import relativo (`import .sibling`) | caminho local pontuado (`import pkg.sibling`) |
| separador `::` | caminhos pontuados |
| `*` na forma com chaves | `from module import *` |
| `main` dentro de um módulo | `main` de nível superior no arquivo de entrada (E312) |

---

## 8. Referências cruzadas

- [python-interop.md](python-interop.md) — `import py.x`, `from py.x import y`,
  e a ponte `python` em runtime.
- [functions.md](functions.md) §9 — o ponto de entrada `main` (E310/E311/E312).
- [statements.md](statements.md) §13 — a tabela de regras aplicada pelo checker.
- [semantics.md](semantics.md) — modelo de execução e modelo de diagnóstico.
