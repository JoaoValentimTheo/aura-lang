---
layout: default
title: "Semântica — Modelo de Execução"
nav_exclude: true
---

[English](semantics.md) · [Português](semantics.pt_BR.md)

# Semântica — Modelo de Execução

**Status:** Stable (exceto onde rotulado) · **Evidência:**
`aura/parser/to_ast.py` (`parse_module_decl`, parse de import),
`aura/transpiler/transformers/statements.py` (`transform_Module`,
`transform_ImportStmt`, `transform_IfStmt`, `transform_TryStmt`),
`aura/transpiler/rules.py` (`RuleChecker`), `aura/transpiler/semantics.py`
(`MutabilityChecker`), `aura/cli.py` (`cmd_run`, `_prepare_entrypoint`).

Este documento define **o significado de um programa Aura** — o que acontece
quando ele roda. Aura é **transpilada para Python e executada no CPython**, então
parte do comportamento observável vem do host e não da linguagem; esses pontos
são rotulados **Target-specific** ou **UNSPECIFIED** em vez de escondidos.

---

## 1. Início e fim da execução

1. Um programa é **um arquivo de entrada**, executado com `aura run`. O runtime
   parseia, checa, transpila para Python e faz `exec` do resultado no CPython
   (`cmd_run`, `cli.py:480-552`).
2. O arquivo de entrada deve declarar um **`main`** de nível superior; o runtime o
   chama. O programador nunca escreve uma chamada `main()` à direita
   (`_prepare_entrypoint`, `cli.py:388-468`).
3. `main` pode ser `sync` ou `async`; um `async def main` é aguardado dentro de um
   event loop (`cmd_run`, `cli.py:526-543`).
4. Os statements de `main` executam **sequencialmente, na ordem da fonte**.
5. Se `main` retorna um `int`, ele vira o código de saída do processo; caso
   contrário o programa sai com 0 (`_prepare_entrypoint`, `cli.py:463-467`).
6. Um `guard cond else { return }` de nível superior reescreve o `return` nu para
   `raise SystemExit(...)`, que `cmd_run` trata como uma saída **bem-sucedida**
   (`statements.py:1137-1160`, `cli.py:554-562`).

```aura
def main() -> int {
  return 3          // process exit code 3
}
```

*Evidência:* `test_main_entrypoint.py::test_main_return_value_is_exit_code`,
`::test_runtime_invokes_main`, `::test_async_main_is_awaited`.

### 1.1 Regras de assinatura

| Ponto de entrada | Resultado |
| --- | --- |
| `def main() { }` | OK |
| `async def main() { }` | OK |
| `def main(args: [string]) { }` | OK; `args` recebe os argumentos da CLI |
| `main` ausente no arquivo de entrada | **E310** |
| `main` com qualquer outra assinatura | **E311** |
| `def main` dentro de um corpo de `module` | **E312** |

*Evidência:* `rules.py:_check_main` §162-205;
`test_main_entrypoint.py::test_missing_main_is_rejected`,
`::test_main_with_two_params_is_rejected`,
`::test_main_with_variadic_is_rejected`;
`test_module_facade.py::test_main_inside_a_module_reports_e312`.

> `main` pode receber **nenhum parâmetro**, ou um único parâmetro `args`; um
> parâmetro variádico ou que não seja `args` é E311 (`rules.py:191-205`).
> `aura test` roda arquivos de teste com `require_main=False`, já que um arquivo
> de teste se autogoverna.

---

## 2. Ordem de avaliação

- **Operandos binários** são avaliados **da esquerda para a direita**.
- **Argumentos de chamada** são avaliados em ordem, esquerda→direita.
- **`and` / `or`** têm **curto-circuito**: o lado direito não é avaliado quando o
  esquerdo decide o resultado.
- **Atribuição**: o lado direito é avaliado antes da escrita no lado esquerdo.
- **Encadeamento postfix** (`a.b().c()[d]`) é avaliado da esquerda para a
  direita, receptor antes do membro.

```aura
let r = mark(1) + mark(2)   // prints 1 then 2
let s = false and side()    // side() never runs
```

*Evidência:* *probe* (`1\n2\n`, e nenhuma saída de `side()`);
[expressions.md](expressions.md).

---

## 3. Escopo e tempo de vida

- Um **bloco** `{ … }` abre um escopo; declarações vivem até o fim do bloco
  (`_visit_body`, `rules.py:1249-1268`; `MutabilityChecker._with_scope`,
  `semantics.py:492-497`).
- Um corpo de **função** é seu próprio escopo; parâmetros são bindings locais e
  são tratados como **mutáveis** (`_visit_function`, `semantics.py:287-317`).
- Um corpo de **módulo** é seu próprio escopo (`rules.py:623-631`; §1.2 de
  [modules.md](modules.md)).

### 3.1 `let` / `const` / `let mut`

| Declaração | Reatribuível |
| --- | --- |
| `let x = 1` | não |
| `let mut x = 1` | sim |
| `const K = 1` | não (nunca) |
| parâmetro de função | sim (um binding local) |
| `self.x = …` / `obj.field = …` | não afetado (muta um objeto) |
| variável de `for`, binding de `with` | tratados como mutáveis |

*Evidência:* docstring do módulo `semantics.py` §1-17;
`_visit_var_decl`/`_visit_const_decl` §275-285;
`test_semantics_deep.py`, `test_language_rules.py`.

```aura skip
let x = 1
x = 2              // E303: reassign an immutable binding

let mut total = 0
total += 1         // ok

const PI = 3.14
PI = 3            // E303
```

### 3.2 Uso antes da declaração

Ler um nome que a **mesma função declara depois em seu próprio corpo** é
**E319** — o caso que de outra forma seria um `UnboundLocalError` do Python
(`_check_use_before_declaration`, `semantics.py:237-262`). Nomes de um escopo
envolvente, do escopo de módulo ou de um import não são afetados
(`_locals_declared_in`, §319-335).

```aura skip
def f() {
  print(x)      // E319: 'x' is used before it is declared
  let x = 1
}
```

*Evidência:* `test_rule_gaps.py` (E319), `language-reference/` §3.

### 3.3 Tempo de vida e memória

Bindings locais `let`/`const` têm **tempo de vida de bloco**. O tempo de vida de
um objeto é gerenciado pelo **host** (contagem de referências + GC cíclico do
CPython). A linguagem **não especifica** quando um objeto é coletado —
**UNSPECIFIED** (intencional; é o GC do target).

### 3.4 Closures

Uma lambda que lê um local envolvente o captura; uma **lambda de bloco** que
atribui a um local envolvente emite `nonlocal <name>` para que a escrita seja
visível fora (`_nonlocal_declaration`, `expressions.py:711`). Um `def` aninhado
que muta um local envolvente também emite `nonlocal`.

```aura
def main() {
  let mut n = 0
  let inc = () => { n = n + 1 }
  inc(); inc()
  print(n)          // 2
}
```

*Evidência:* *probe* (2);
`test_main_entrypoint.py::test_nested_function_mutates_captured_local`;
[closures.md](closures.md) §2.

> **UNSPECIFIED:** a captura é **por referência** através do protocolo de closure
> do CPython, não um snapshot por iteração. Uma lambda criada em um loop observa
> o valor **final** da variável do loop. *Probe*:
> `for i in range(3) { fs.add((x) => x + i) }` dá `fs[0](0) == 2`. Vincule o valor
> como um parâmetro default para captura por iteração
> ([closures.md](closures.md) §2.3).

---

## 4. Semântica de valor vs referência

| Tipo | Modelo | `==` |
| --- | --- | --- |
| Números, `bool` | valor | igualdade numérica |
| `string` | valor por conteúdo (imutável) | conteúdo |
| instância de classe | referência (identidade) | identidade por padrão |
| list / dict / set | referência (objeto mutável) | identidade por padrão |
| valor struct / em forma de dict | conteúdo | por elemento |

A passagem para uma função é **por valor**; para uma referência esse valor é a
referência, então mutar o objeto é visível ao chamador enquanto reatribuir o
parâmetro não é.

```aura
def mutate(xs) { xs.add(9) }
def reassign(x) { x = 9 }
def main() {
  let a = [1]
  mutate(a)
  print(a)     // [1, 9]
  let n = 1
  reassign(n)
  print(n)     // 1
}
```

*Evidência:* *probe* (`[1, 9]`, `1`); [expressions.md](expressions.md),
[types.md](types.md).

---

## 5. Exceções

Aura `throw`a um **valor**; no CPython o valor é levantado
(`transform_ThrowStmt`, `statements.py:1293-1303`).

| Forma | Emite |
| --- | --- |
| `throw "msg"` | `raise Exception("msg")` (uma string nua é embrulhada) |
| `throw ValueError("bad")` | `raise ValueError('bad')` |
| `catch { }` | `except Exception:` |
| `catch Type { }` | `except Type:` |
| `catch Type as e { }` | `except Type as e:` |
| `catch as e { }` | `except Exception as e:` |

`finally` sempre roda. Uma exceção não capturada aborta o programa com uma
mensagem e um código de saída não zero (`cmd_run`, `cli.py:563-573`).

*Evidência:* `test_syntax_complete.py::test_throw_string_wrapped`,
`::test_try_catch_finally`, `test_custom_errors.py`.

> **TARGET-SPECIFIC:** uma exceção é um objeto CPython. Cláusulas `catch` são
> emitidas na ordem da fonte e o filtro de tipo é o que o CPython resolver para o
> nome; Aura não reordena subclasses antes dos pais.
> **UNSPECIFIED:** `throw` de um valor que não é string nem exceção (`throw 5`)
> é emitido como `raise 5` e falha em runtime; não é rejeitado em tempo de check
> ([statements.md](statements.md) §8).

---

## 6. Semântica específica do target (CPython)

Como o target é CPython, estes comportamentos vêm do host:

| Comportamento | Regra |
| --- | --- |
| Tipo inteiro | `int` do Aura mapeia para o `int` de precisão arbitrária do Python. |
| Float | IEEE-754 binary64, como o CPython. |
| `%` com operando negativo | floor-mod do Python: *probe* `-7 % 3 == 2`. |
| `/` | Divisão verdadeira (float), como o CPython. |
| Iteração | Protocolo de iterador do CPython; um `for` sobre range/list/dict o segue. |
| Ordenação de dict | Ordem de inserção (CPython 3.7+). |
| GC / tempo de vida de objeto | Contagem de referências + GC cíclico do CPython. **Unspecified** quando coletado. |
| Erros de runtime | Aparecem como a exceção `Python` subjacente (`AttributeError`, `TypeError`, …) reportada por `cmd_run`. |

*Evidência:* *probe* (`-7 % 3` → `2`); `cmd_run` reporta o tipo e a mensagem da
exceção (`cli.py:566-573`).

---

## 7. Regras de linguagem aplicadas por `aura check` / `aura run`

`aura check`, `aura run` e o REPL rodam os mesmos checkers antes da geração de
código (`_mutability_diagnostics`, `cli.py:18-40`;
`_rule_diagnostics`, `cli.py:43-62`). As regras são:

| Regra | Código | Aplicada em |
| --- | --- | --- |
| Reatribuir um binding `let`/`const` | **E303** | `semantics.py:403-430` |
| Atribuir estado de módulo de fora (incl. um membro exportado) | **E303** | `rules.py:1308-1316` |
| Atribuir um `const` de nível de classe através de um membro | **E303** | `rules.py:1317-1325` |
| Declaração duplicada no mesmo escopo | **E301** | `_declare`, `rules.py:581-592` |
| Nome de parâmetro duplicado | **E301** | `rules.py:1215-1228` |
| `return` fora de uma função (exceto um guard de nível superior) | **E004** | `rules.py:706-714` |
| `break` / `continue` fora de um loop | **E004** | `rules.py:715-723` |
| `break`/`continue` nomeando um label não envolvente | **E318** | `rules.py:724-734` |
| `await` fora de uma função `async` (ou top level) | **E004** | `rules.py:746-757` |
| `yield` fora de uma função | **E004** | `rules.py:758-771` |
| `self` fora de um método de classe | **E004** | `rules.py:792-799` |
| `self`/`cls` dentro de um método `static` | **E317** | `rules.py:800-809` |
| Código inalcançável após `return`/`throw`/`break`/`continue` | **E302** | `rules.py:1249-1268` |
| Alvo de atribuição inválido | **E004** | `rules.py:1272-1284` |
| Ler um local de função antes de sua declaração | **E319** | `semantics.py:237-262` |
| `main` ausente / inválido / dentro de um módulo | **E310 / E311 / E312** | `rules.py:162-265` |
| Re-export de módulo não resolvido | **E313** | `rules.py:439-467` |

```aura skip
let count = 0
count = 1              // E303: reassign an immutable binding

let mut total = 0
total += 1             // ok

guard total > 0 else { return }   // exits the program if the guard fails
```

*Evidência:* `test_language_rules.py`, `test_extreme_rules.py` (E302),
`test_rule_gaps.py` (E318, E319), `test_main_entrypoint.py`,
`test_module_facade.py` (E313).

### 7.1 Regras de posicionamento (detalhe)

- **`return`** é válido dentro de qualquer função. No nível superior é um erro
  **a menos** que fique diretamente em um bloco `guard … else { return }`, que é
  o idioma para "sair do programa" (`rules.py:706-714`; `_in_guard_else_depth`).
- **`break` / `continue`** são válidos apenas dentro de um loop. Um simples
  aponta para o loop mais interno; um rotulado deve nomear um loop envolvente
  (caso contrário E318).
- **`await`** é válido dentro de um `async def`, ou no **nível superior**, porque
  `cmd_run` executa um programa async dentro de uma corrotina
  (`rules.py:746-757`; `cli.py:526-543`).
- **`self`** é válido apenas dentro de um método de um corpo de classe;
  `self`/`cls` dentro de um método `static` é E317 (`rules.py:792-809`).

### 7.2 Código inalcançável (E302)

Um statement que segue `return`, `throw`, `break` ou `continue` (um
*terminator*, `_TERMINATORS`, `rules.py:74`) na mesma lista de statements é
**E302**. O checker reporta **uma vez por região** e reseta (`_visit_body`,
`rules.py:1249-1268`).

```aura
def f() {
  throw "boom";
  print("never")     // E302
}
```

> **Pegadinha:** statements são separados pela gramática, e um `return`/`break`
> seguido na linha seguinte por uma expressão **sem um `;`** é parseado como o
> valor do terminator, então nenhum código morto é visto. *Probe*: `return\n
> print(1)` parseia como `return print(1)` (sem E302), enquanto `return;\n
> print(1)` reporta E302. Use um ponto e vírgula ou um corpo de bloco para
> desambiguar ([statements.md](statements.md) §6.7, §9).

### 7.3 Alvos de atribuição válidos

Um alvo de atribuição deve ser uma variável, um acesso a membro, um índice ou um
alvo de desestruturação (tupla/lista, possivelmente com um padrão rest)
(`_is_assignable`, `rules.py:1332-1339`). Qualquer outra coisa é **E004**.

```aura skip
def main() {
  1 = 2            // E004: invalid assignment target
}
```

*Evidência:* `rules.py:1272-1284`; *probe* (`E004`).

---

## 8. Modelo de erro e diagnóstico

Diagnósticos são registros estruturados com um **código**, uma **severidade**,
uma mensagem, uma **localização** de fonte opcional, uma **dica** opcional e
localizações relacionadas (`AuraError`, `errors.py:108-144`). Os códigos são
agrupados por namespace (`ErrorCode`, `errors.py:36-107`):

| Prefixo | Significado |
| --- | --- |
| `E0xx` | léxico / sintaxe |
| `E1xx` | tipo |
| `E3xx` | semântico / estrutural |
| `E9xx` | fatal / interno |
| `W0xx` | warnings de estilo |
| `W1xx` | warnings de não usado / redundante |

Um diagnóstico **não** é um código estruturado quando vem do parser: o parser
levanta um `SyntaxError` do Python carregando `line`/`column` em vez disso
(`errors.py:55-59`; `to_ast.py` usa `self.error`).

### 8.1 Coleta e severidade

- `ErrorCollector` reúne diagnósticos e para após **10 erros**, anexando um
  resumo fatal `E999` (`errors.py:146-171`).
- Um **warning** deve usar um código `W`; `add_warning` rejeita um código `E`
  (`errors.py:173-184`), então um diagnóstico `W` nunca pode se disfarçar de erro.
- `has_errors()` é verdadeiro apenas para `ERROR`/`FATAL`; warnings não falham um
  build (`errors.py:186-188`).

### 8.2 Exibição e status de saída

O diagnóstico formatado é (`errors.py:118-141`):

```text
<file>:<line>:<col>: ERROR [E303]
  Cannot reassign immutable binding 'x'; ...
  hint: write 'let mut x' at its declaration
```

- `aura check` imprime todos os diagnósticos; uma falha de regra/mutabilidade/tipo
  retorna exit 1 (`cmd_check`, `cli.py:200-239`).
- `aura run` reporta os mesmos diagnósticos e retorna exit 2 antes de executar
  (`cmd_run`, `cli.py:503-508`).
- Um erro de runtime é reportado como `Runtime error: <Type>: <message>` e
  retorna exit 1 (`cli.py:566-573`).
- Um `int` retornado por `main` é o código de saída; uma saída de `guard` de nível
  superior nua é 0 (`cli.py:554-562`).

*Evidência:* `test_diagnostics.py` (códigos permanecem em sincronia com
`docs/ERRORS.md`), `test_cli_diagnostics.py`.

> `docs/ERRORS.md` é a **única fonte de verdade** para códigos de diagnóstico; um
> teste afirma que `ErrorCode` e esse documento permanecem em sincronia
> (`errors.py:39-41`, `tests/test_diagnostics.py`).

---

## 9. O que a linguagem NÃO define (resumo)

| Ponto | Estado |
| --- | --- |
| Quando um objeto é coletado | **Unspecified** (GC do CPython) |
| Captura por iteração em um loop | **Unspecified** (a captura é por referência) |
| `throw` de um valor que não é exceção | **Unspecified** (não rejeitado em tempo de check) |
| Ordenação de cláusulas `catch` | Ordem da fonte; o host resolve os tipos (**Target-specific**) |
| Resolução de import ambígua | **Unspecified** (o último binding vence no namespace do target) |
| Sinal de `%`, divisão, largura de inteiro | **Target-specific** (CPython) |
| Texto de erro de runtime | **Target-specific** (a exceção `Python` subjacente) |

---

## 10. Referências cruzadas

- [statements.md](statements.md) — semântica de statements e a tabela do checker.
- [functions.md](functions.md) §9 — o ponto de entrada.
- [closures.md](closures.md) — lambdas, captura e `nonlocal`.
- [modules.md](modules.md) — módulos, imports, re-exports, resolução de nomes.
- [python-interop.md](python-interop.md) — a superfície do target CPython.
- [../ERRORS.md](../ERRORS.md) — a referência canônica de códigos de diagnóstico.
