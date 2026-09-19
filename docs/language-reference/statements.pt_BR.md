---
layout: default
title: "Statements"
nav_exclude: true
---

[English](statements.md) | [Português](statements.pt_BR.md)

# Statements

**Status:** Stable (exceto onde rotulado) · **Evidência:** `parser/to_ast.py`
(`parse_if_stmt`, `parse_for_stmt`, `parse_try_stmt`, `parse_match_stmt`),
`transpiler/transformers/statements.py` (`transform_IfStmt`,
`_labeled_loop`, `transform_TryStmt`), `transpiler/rules.py` (`RuleChecker`).

Um statement executa um efeito e, com as exceções de um `if`/`match`/`try` com
corpo de expressão (veja [expressions.md](expressions.md)), **não produz um
valor**. Ponto e vírgula são **opcionais** ao final de todo statement; quebras de
linha **não** são significativas. O parser delimita statements pela gramática
(`grammar.md` §1.1, §5).

---

## 1. Bloco

```
block          = "{" , { statement } , "}" ;
```

Introduz um novo escopo. Declarações dentro de um bloco **não** vazam para fora
(`rules.py:1249` `_visit_body` empilha/desempilha um escopo).

- Em posição de statement, `{ … }` é um **bloco** quando abre um corpo de
  controle de fluxo (`if`/`for`/`while`/`def`/…) ou quando contém statements.
- **Pegadinha:** um `{ }` **isolado e vazio** em posição de expressão parseia
  como um literal **dict/struct** vazio, não um bloco vazio (*probe*:
  `def main() { { } }` emite `AuraDict({})`). Um bloco com pelo menos um
  statement, ou um anexado a um cabeçalho de controle de fluxo, é um bloco real.

```aura
def main() {
  {
    let inner = 1
    print(inner)
  }
  // inner is out of scope here
}
```

---

## 2. `if` / `else if` / `else`

```
if_stmt        = "if" , expression , block , [ "else" , ( if_stmt | block ) ] ;
```

- A condição é parseada por `parse_condition`; dentro de uma condição um `{`
  sempre abre o corpo, nunca um literal dict/struct (`to_ast.py:2103-2115`).
- `else if` **encadeia**: o ramo `else` é ele mesmo um `if_stmt`, então qualquer
  profundidade é aceita (`to_ast.py:2122-2123`).
- **`elif` / `elsif` são rejeitados** com um erro apontado dizendo para escrever
  `else if` (`to_ast.py:819-822`, *probe*).
- Cada ramo é um **bloco**; as chaves são obrigatórias (diferente de Kof).
  `if c print(1)` é erro de parse.

```aura
let n = 2
if n == 1 {
  print("one")
} else if n == 2 {
  print("two")
} else {
  print("other")
}
```

*Evidência:* `test_syntax_complete.py::test_if_else_if_else` imprime `two`.

`if` em posição de **expressão** (`let x = if c { 1 } else { 2 }`) é coberto em
[expressions.md](expressions.md).

---

## 3. `unless` (`if` invertido)

```
unless_stmt    = "unless" , expression , block , [ "else" , ( if_stmt | block ) ] ;
```

`unless cond { A }` executa `A` quando `cond` é falsy. Ele transpila para
`if not (cond): ...` (`statements.py:1113-1124`). `else` encadeia como `if`.

```aura
unless authenticated {
  redirect("/login")
}
```

*Evidência:* `test_syntax_complete.py::test_unless`.

---

## 4. `guard cond else { ... }`

```
guard_stmt     = "guard" , expression , "else" , block ;
```

Executa o bloco `else` quando `cond` é **falsy**, e então continua após o guard.
Ele transpila para `if not (cond): <else>` (`statements.py:1126-1135`). Não há
keyword `then` nem forma de valor.

```aura
def process(data) {
  guard data != none else {
    print("No data")
    return
  }
  print(data)          // data is non-none here
}
```

Em **escopo de módulo**, um `return` nu no corpo do guard é reescrito para
`raise SystemExit(...)` para que o Python gerado seja válido; este é o idioma de
Aura para "sair do programa" (`statements.py:1137-1160`).

```aura
guard ready else { return }     // exits the program when not ready
```

*Evidência:* `test_syntax_complete.py::test_guard_early_return`.

---

## 5. `match` (statement)

```
match_stmt     = "match" , expression , "{" , { match_case } , "}" ;
match_case     = "case" , pattern , [ "if" , expression ] , ( block | "->" , expression ) ;
```

- Um corpo de case é ou um **bloco** `{ ... }` ou uma **seta** `-> expression`
  (`to_ast.py:2301-2308`). `->` aceita um único statement/expressão.
- Um **guard** `case p if cond` refina o padrão. Um case com guard **não**
  conta como catch-all para exaustividade (warning **E109**).
- Padrões: wildcard `_`, literal, identificador nu (vincula), membro pontuado
  (`Color.RED`), desestruturação de lista/tupla com `*rest`, padrões de construtor
  (`Shape.Circle(r)`), or-patterns (`1 | 2`) e `_ as name`
  (`to_ast.py:2344-2376`).
- **Sem fallthrough**: as sintaxes de case `:` e `=>` são rejeitadas com uma
  mensagem apontando para `->` / `{ }` (`to_ast.py:2310-2322`).
- Omitir um `case _` (ou um binding nu `case name`) sobre um domínio escalar,
  `bool` ou enum emite o warning **E109** (nunca falha um build;
  `TypeChecker`, `transpiler/errors.py:66`). Um `case _ if cond` com guard não o
  suprime.

```aura
match status {
  case 0 { print("inactive") }
  case n if n > 100 { print("overflow") }
  case _ { print("unknown") }
}
```

```aura
match command {
  case "quit" -> print("Goodbye!")
  case _      -> print("Unknown")
}
```

`match` em posição de **expressão** é coberto em [expressions.md](expressions.md).

*Evidência:* `test_syntax_complete.py::test_match_literal`,
`::test_match_guard`.

---

## 6. Loops

```
while_stmt     = [ label ] , "while" , expression , block ;
until_stmt     = [ label ] , "until" , expression , block ;
for_stmt       = [ label ] , "for" , pattern , "in" , expression , [ "step" , expression ] , block ;
loop_stmt      = [ label ] , "loop" , block ;
label          = identifier , ":" ;
```

O `label` opcional é um `IDENT` seguido de `:` antes da keyword do loop
(`grammar.md` §5.2).

### 6.1 `while`

Condição avaliada **antes** de cada iteração.

```aura
let mut i = 0
while i < 3 {
  print(i)
  i += 1
}
```

*Evidência:* `test_syntax_complete.py::test_while_loop`.

### 6.2 `until` (`while` invertido)

`until cond { ... }` roda enquanto `cond` é falsy; transpila para
`while not (cond):` (`statements.py:1169-1175`).

```aura
let mut i = 0
until i >= 3 { print(i); i += 1 }
```

*Evidência:* `test_syntax_complete.py::test_until_loop`.

### 6.3 `loop` (infinito)

Transpila para `while True:` (`statements.py:1271-1275`). A terminação é por
`break` (ou `return`/`throw`).

```aura
loop {
  let input = read_input()
  if input == "quit" { break }
}
```

*Evidência:* `test_syntax_complete.py::test_loop_infinite_with_break`.

### 6.4 `for`-in

```
for_stmt       = … , "for" , pattern , "in" , expression , [ "step" , expression ] , block ;
```

Itera qualquer alvo iterável (lista, range, comprehension, gerador). O alvo é um
**padrão**; vários nomes são um padrão de lista, então `for (k, v) in pairs`
desestrutura cada elemento (`to_ast.py:2174-2184`).

```aura
for item in items { print(item) }
for (k, v) in pairs { print(k) }
```

*Evidência:* `test_syntax_complete.py::test_for_loop`.

### 6.5 `for` sobre ranges

`for i in 0..10` é inclusivo (`1..10` produz 1…10); `0..<10` é exclusivo
(`transform_RangeExpr`, `expressions.py:633-646`). Ranges e chamadas `range(...)`
são iteráveis equivalentes.

```aura
for i in 0..<3 { print(i) }          // 0, 1, 2
for i in range(0, 6, 2) { print(i) } // 0, 2, 4
```

### 6.6 `step`

Um `step N` à direita em um `for` ou se dobra em uma chamada `range(...)`, ou se
anexa a um range `..`/`..<`, ou fatia qualquer outro iterável com `[::N]`
(`statements.py:1189-1216`).

```aura
for i in range(0, 10) step 2 { print(i) }  // 0 2 4 6 8
for i in 0..10 step 2 { print(i) }         // 0 2 4 6 8 10
for x in items step 2 { print(x) }         // items[::2]
```

*Evidência:* `test_syntax_complete.py::test_for_with_step`.

### 6.7 `break` / `continue`

- Encerram / pulam o loop **mais interno**.
- **Um `break` / `continue` fora de um loop é rejeitado** pelo rule checker
  (`rules.py:715-723`, *probe*).
- `break label` / `continue label` têm como alvo um loop envolvente **rotulado**.

**UNSPECIFIED / pegadinha** — O parser toma gananciosamente um identificador
seguinte como label (`to_ast.py:863-876`). Como quebras de linha são
insignificantes, um `break` como último statement de um case de match em seta
seguido de `case` na linha seguinte é parseado como `break case`, produzindo um
`NameError` em runtime. Escreva um `;` ou use um corpo de bloco para
desambiguar. Este é comportamento do parser, não semântica pretendida.

### 6.8 Loops rotulados

Um label nomeia um loop envolvente. `break label` sai desse loop;
`continue label` pula para sua próxima iteração (`_labeled_loop`,
`statements.py:1218-1269`). Labels funcionam em `for`, `while`, `until` e `loop`.

```aura
outer: for i in range(3) {
  inner: for j in range(3) {
    if j == 1 { continue outer }
    if i == 2 { break outer }
    print(i, j)
  }
}
```

- `break`/`continue` nomeando um loop que **não** é envolvente é o erro **E318**
  (`rules.py:724-734`).
- Um label interno **não é visível** a partir do corpo de um loop externo.
- Um `break`/`continue` simples sempre aponta para o loop mais interno.

*Evidência:* `test_syntax_complete.py::test_labeled_break_and_continue`,
`test_rule_gaps.py::TestUnknownLabel`.

---

## 7. `try` / `catch` / `finally`

```
try_stmt       = "try" , block , { catch_clause } , [ "finally" , block ] ;
catch_clause   = "catch" , [ type , [ "as" , identifier ] ] , block
               | "catch" , "as" , identifier , block ;
```

- Um `try` exige **pelo menos uma** cláusula `catch`, ou um `finally`
  (`to_ast.py:2258-2260`).
- Um significado por grafia (`to_ast.py:2233-2252`):

  | Forma | Significado | Emite |
  | --- | --- | --- |
  | `catch { }` | captura toda exceção, sem binding | `except Exception:` |
  | `catch Type { }` | captura apenas `Type` | `except Type:` |
  | `catch Type as e { }` | captura apenas `Type`, vincula a `e` | `except Type as e:` |
  | `catch as e { }` | captura toda exceção, vincula a `e` | `except Exception as e:` |

- Um identificador isolado antes de `{` é sempre um **tipo**, nunca um binding.
  Um segundo identificador nu (`catch Value Error`) é rejeitado: use
  `catch Type as name` (`to_ast.py:2243-2247`).
- `finally` sempre roda (`transform_TryStmt`, `statements.py:1305-1319`).
- **UNSPECIFIED:** cláusulas `catch` são emitidas na ordem da fonte; o filtro de
  tipo é o que o runtime CPython resolver para o nome. Aura não as reordena para
  você, então liste subclasses antes de seus pais.

```aura
try {
  let file = open("data.txt")
  process(file)
} catch IOError as e {
  print("File not found")
} finally {
  print("Cleanup complete")
}
```

*Evidência:* `test_syntax_complete.py::test_try_catch_finally`,
`::test_try_only_finally`, `::test_try_without_handler_is_error`,
`test_custom_errors.py`.

---

## 8. `throw`

```
"throw" , expression , [ ";" ]
```

- `throw ValueError("bad")` levanta o valor como está (`raise ValueError('bad')`).
- Um **literal de string** `throw "msg"` é embrulhado: `raise Exception("msg")`,
  porque o Python não pode `raise` uma string nua (`statements.py:1293-1303`).
- **UNSPECIFIED:** uma expressão que não é string nem exceção (`throw 5`) é
  emitida como `raise 5`, o que falha em runtime. A linguagem não a rejeita em
  tempo de compilação.

```aura
def validate(age) {
  guard age >= 0 else {
    throw ValueError("age cannot be negative")
  }
  return true
}
```

*Evidência:* `test_syntax_complete.py::test_throw_string_wrapped`,
`test_custom_errors.py`.

---

## 9. `return`

```
"return" , [ expression ] , [ ";" ]
```

- Em posição de expressão após `return`, uma lista com vírgulas vira uma
  **tupla**: `return b, a` retorna `(b, a)` (`parse_trailing_tuple`,
  `to_ast.py:2215-2223`).
- `return` (nu) não tem valor.
- **`return` fora de uma função** é rejeitado, exceto dentro de um
  `guard cond else { return }` de nível superior (`rules.py:706-714`, *probe*).

```aura
def swap(a, b) {
  return b, a
}

def main() {
  let x, y = swap(1, 2)   // x = 2, y = 1
}
```

*Evidência:* `test_syntax_complete.py::test_swap`, `test_rule_gaps.py`.

---

## 10. `assert`

```
"assert" , expression , [ "," , expression ] , [ ";" ]
```

- Quando a condição é falsy, levanta `AssertionError` (com a mensagem dada).
- A mensagem é uma expressão (não restrita a um literal).

```aura
assert 1 + 1 == 2
assert 1 == 2, "nope"     // raises AssertionError("nope")
```

*Evidência:* `test_syntax_complete.py::test_assert`.

---

## 11. `with`

```
with_stmt      = [ "async" ] , "with" , with_item , { "," , with_item } , block ;
with_item      = expression , [ "as" , identifier ] ;
```

Usa `__enter__`/`__exit__`; `async with` usa `__aenter__`/`__aexit__`. A regra
documentada é que `async with` é válido apenas dentro de um `async def`
(`grammar.md` §5.3, `LANGUAGE.md` §16b) — **UNSPECIFIED:** diferente de `await`,
este posicionamento **não** é aplicado pelo rule checker (*probe*: `async with`
em um `def` síncrono não produz diagnóstico; só falha se o Python alvo for
inválido).

```aura
with open("x") as fh {
  print(fh)
}
```

*Evidência:* `test_syntax_complete.py::test_with_statement`.

---

## 12. Statement de expressão / statement vazio

```
expression , [ ";" ] ;
```

Qualquer expressão usada por efeito (chamada, atribuição, incremento). Um `;` à
direita é opcional; statements também são separados pela gramática através de
quebras de linha. Um `;` isolado é tolerado e não produz statement (*probe*:
`def main() { ; }` emite `pass`).

```aura
def main() { let a = 1; let b = 2; print(a + b); }
```

---

## 13. Regras aplicadas pelo checker

| Regra | Código | Evidência |
| --- | --- | --- |
| `return` fora de uma função (exceto guard de nível superior) | E004 | `rules.py:706-714` |
| `break`/`continue` fora de um loop | E004 | `rules.py:715-723` |
| Label desconhecido / não envolvente | E318 | `rules.py:724-734` |
| Código inalcançável após `return`/`throw`/`break`/`continue` | E302 | `rules.py:1249-1268` |
| Local usado antes de seu `let`/`const` | E319 | `semantics.py:237-262` |
| `await` fora de `async` (ou top level) | E004 | `rules.py:747-757` |

*Evidência:* `test_extreme_rules.py` (E302),
`test_rule_gaps.py` (E318, E319), `test_language_rules.py`.
