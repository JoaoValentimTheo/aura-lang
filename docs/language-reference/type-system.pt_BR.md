---
layout: default
title: "Sistema de Tipos e Checagem de Tipos"
nav_exclude: true
---

[English](type-system.md) · [Português](type-system.pt_BR.md)

# Sistema de Tipos e Checagem de Tipos

**Status:** Stable (regras) · **Evidência:** `aura/transpiler/types.py`
(`TypeChecker` §518, `TypeInference` §285, `_parse_type_annotation` §1456),
`aura/transpiler/errors.py` (`ErrorCode`), `aura/transpiler/rules.py`,
`aura/cli.py` (`cmd_check` §200, `cmd_run` §480),
`aura/transpiler/transformers/statements.py` (`_aura_type_to_python` §770),
*probes* de execução.

> Este documento evita o termo vago "tipagem forte". Ele descreve
> **comportamento concreto**: o que é aceito, o que é rejeitado, quando a
> inferência roda, o que é garantido e — crucialmente — **o que é apagado**
> porque Aura transpila para Python.

---

## 1. Classificação do sistema

Aura é **gradualmente tipada**:

- Anotações são **opcionais**. Qualquer coisa que o checker não consiga provar é
  tratada como `AnyType` e **aceita** (docstring de `TypeChecker`,
  `types.py:518-527`).
- **Inferência local** existe para literais e um conjunto fixo de operadores e
  builtins (`TypeInference`, §3); uma variável nua só tem tipo depois de anotada
  ou atribuída (`_expr_type`, `types.py:1177-1186`).
- A subtipagem é **nominal**: um `ClassType` compara apenas por **nome**
  (`types.py:219-226`); bases declaradas são ligadas, mas a compatibilidade de
  métodos/argumentos não é percorrida estruturalmente.
- Genéricos são **apagados**: argumentos de tipo não são retidos como tipos
  parametrizados (`_parse_type_annotation` §7).

A mensagem deste documento é a **fronteira de garantia**. Onde o checker *não*
impede uma operação, isso é dito explicitamente.

---

## 2. Onde a checagem de tipos acontece (pipeline real)

```
Source ─▶ Tokenizer ─▶ Parser ─▶ AST (types stored as strings)
       ─▶ TypeChecker.check_program   (aura check, LSP, REPL)
       ─▶ RuleChecker.check_program   (aura run / transpile)
       ─▶ Transformer / emit          (annotations mostly erased)
```

**A checagem de tipos não faz parte do caminho run/transpile.** `cmd_run`
(`cli.py:480-553`) e `cmd_transpile` rodam apenas os checkers de *mutabilidade* e
de *regras*; `cmd_check` (`cli.py:200-238`) é o ponto de entrada que instancia
`TypeChecker` e reporta `E1xx`/`W103`/`E109`. O LSP (`aura/lsp/server.py`) e o
REPL (`aura/repl/engine.py`) também o executam. Assim, um programa pode
`aura run` com um erro de tipo que `aura check` reporta.

### 2.1 O type checker não tem "AST tipado"

Anotações são `str` no AST (`parse_type` retorna uma string). O checker as
converte sob demanda com `_parse_type_annotation` (`types.py:1456-1510`) e mantém
bindings resolvidos em um `context: dict[str, Type]` simples. Nada é anexado de
volta ao AST.

### 2.2 Quando os diagnósticos são produzidos

Diagnósticos são anexados conforme o checker percorre (`_add`, `types.py:566-571`);
`check_program` retorna `True` quando `self.errors` está vazio. Existem duas
severidades: `ERROR` (falha `aura check`) e `WARNING` (não falha). Os códigos de
warning `E109` e `W103` usam nomes `E`/`W`, mas ambos carregam
`ErrorSeverity.WARNING` (*probe*).

---

## 3. Inferência de tipos (`TypeInference`)

`TypeInference` (`types.py:285-511`) é **livre de contexto** e deliberadamente
conservadora. Ela retorna `AnyType` para qualquer nó que não modela —
`Identifier`, `MemberExpr`, `IndexExpr`, `MatchExpr`, `TryExpr`, `BlockExpr`,
`PipeExpr` (`types.py:366-368`). `AnyType.is_compatible` é sempre `True`
(`types.py:97-99`), então um `Any` em **qualquer** dos lados silencia uma
checagem.

| Fonte | Inferido | Evidência |
|---|---|---|
| `42` / `3.14` / `"x"` | `int` / `float` / `str` | `types.py:318-325` |
| `b"x"` | `Any` | sem ramo bytes |
| `true` | `bool` | `types.py:326-327` |
| `none` | `none` | `types.py:328-329` |
| `[1, "x"]` | `[int]` (apenas primeiro elemento) | `types.py:330-333` |
| `{1,2}` | `{int}` (`SetType`) | `types.py:334-337` |
| `(1,"x")` | `(int, str)` (`TupleType`) | `types.py:338-339` |
| `{a: 1}` | `{str: int}` (apenas primeiro par) | `types.py:340-345` |
| `1..10` | `[int]` | `types.py:346-347` |
| `x ?? y` | `strip-none(x)` | `types.py:474-475` |
| `x?.f` | `Optional(infer x)` | `types.py:476-477` |
| `a and b` | `bool` | `types.py:467-469` |
| `f(x)` para um builtin conhecido | guiado por tabela | `_BUILTIN_RETURNS` §375-442 |

`_infer_binary_op` (`types.py:444-480`): `+` de dois `str` → `str`, de duas
`list` → lista da união dos elementos, de dois numéricos → `int`/`float`; `/`
de dois numéricos → `float`; comparações e operadores booleanos → `bool`;
bit a bit sobre `bool` → `bool`, senão `int`; qualquer outra coisa → `Any`.

**A inferência de tipo de retorno** **não** acontece aqui: um `def` sem anotação
tem `FunctionType.return_type = AnyType` (`types.py:836-838`), e um corpo nunca é
re-inferido para sintetizar um tipo de retorno. `def double(x) = x * 2` é aceito
sem anotação de retorno e sem assinatura inferida. (Compare com `types.md` §10,
que afirma inferência de retorno — o checker não a implementa.)

---

## 4. Atribuição e compatibilidade

A compatibilidade é `declared.is_compatible(actual)` (note a **direção**:
*esperado* à esquerda, *obtido* à direita). A implementação base
(`types.py:97-99`) aceita por igualdade ou quando um dos lados é `AnyType`.

| Caso | Aceito? | Evidência |
|---|---|---|
| mesmo tipo | ✅ | `Type.__eq__` por classe |
| `anything` vs `any` | ✅ | `isinstance(other, AnyType)` / `isinstance(self, AnyType)` |
| `int → float` (alargamento) | ❌ | `IntType().is_compatible(FloatType())` é `False` (*probe*) |
| `bool → int` | ❌ | `BoolType().is_compatible(IntType())` é `False` (*probe*) |
| `int → int?` (tornar opcional) | ✅ | `UnionType.is_compatible` é `any(...)` (*probe*) |
| **`int? → int`** (desempacotar opcional) | ✅ **de forma não sólida** | `UnionType({int,None}).is_compatible(int)` é `True` (*probe*) |
| `int \| str → int` | ✅ | união aceita cada membro |
| `int → int \| str` | ❌ (como *declarado*) | `IntType().is_compatible(union)` é `False` (*probe*) |
| `[int] → [int]` | ✅ | `ListType.is_compatible` recorre no elemento |
| `[int] → [str]` | ❌ | `ListType.is_compatible` recorre no elemento |
| `{str:int} → {str:str}` | ❌ | `DictType.is_compatible` recorre em ambos os slots |
| estrutural `{name: str}` (qualquer dict) | ✅ sempre | checker colapsa chaves em `DictType` opaco |
| qualquer par de classes | ✅ (nome ou inerente) | `ClassType.__eq__` é só por nome |

Não há **coerções numéricas**: um literal `int` atribuído a uma variável `float`
é `E101` (*probe*: `let y: float = 1` registra `expected Float, got Int` quando a
anotação está presente e o valor é um literal int nu). O alargamento não é
modelado.

### 4.1 Onde a atribuição é checada

- **Variável** com anotação: `_check_var_decl` (`types.py:781-794`) → `E101` em
  incompatibilidade; o tipo declarado vence no `context` mesmo após o erro.
- **Constante**: `_check_const_decl` (`types.py:796-807`) → `E101`.
- **Retorno**: `_check_return` (`types.py:1267-1279`) → `E101` quando a função
  envolvente tem tipo de retorno anotado **não-`Any`**.
- **Argumentos**: `_check_call_expr` (`types.py:1431-1448`) → `E106`.
- **Aridade**: `E105` para argumentos demais, ou menos que o mínimo exigido
  (`types.py:1408-1430`).
- **Condições** (`if`/`while`/`until`/`assert`/ternário/guard de match):
  `_check_condition` (`types.py:1333-1342`) → `E101` quando o tipo inferido é um
  de `int/float/str/list/dict/set/tuple/none`.
- **Operadores**: `_check_binary_op` (`types.py:1344-1390`) → `E108`.

Uma anotação com **nome desconhecido** (`let x: Unknwn = 1`) **não** é erro:
`_parse_type_annotation` retorna `AnyType` (`types.py:1492-1494`), e a declaração
é aceita (*probe*). Isso é tipagem gradual intencional.

---

## 5. Narrowing (união / opcional)

`_narrowings` (`types.py:1030-1074`) reconhece exatamente duas formas, ambas sobre
um **identificador nu** à esquerda:

| Condição | ramo then | ramo else |
|---|---|---|
| `x != none` / `x is not none` | `strip-none(x)` | `none` |
| `x == none` / `x is none` | `none` | `strip-none(x)` |
| `x is T` (T um nome de tipo) | `T` | inalterado |
| `x is not T` | inalterado | `T` |

O narrowing é aplicado por `_with_narrowing` apenas ao corpo daquele ramo e é
restaurado depois (`types.py:1017-1028`). **Não há** narrowing via `and`/`or`, um
ternário, uma condição de `guard` ou uma reatribuição. `strip-none` remove `None`
de um conjunto `UnionType` (`types.py:499-503`).

Como `int? → int` já passa em §4, o narrowing muda **apenas a precisão da
análise**, não a aceitação: `let y: str = x` para `x: str?` é aceito mesmo sem
narrowing (*probe*). A checagem que *é* real é a direção reversa (um valor
não-opcional usado onde um opcional é esperado — sempre aceito).

---

## 6. Tipos de retorno

- Um tipo de retorno declarado não-`Any` é checado contra o tipo inferido em cada
  `return` (`E101`).
- Uma função **sem anotação** tem tipo de retorno `Any`; todo `return` é aceito.
- Uma função com retorno declarado não-`Any` e **nenhum** `return` em lugar algum
  **não** é reportada pelo type checker (`E304 MISSING_RETURN` é documentado como
  não aplicado; the *Removed codes* table in [ERRORS.md](../ERRORS.md)). O Python emitido simplesmente retorna
  `None`.
- A inferência de tipo de retorno a partir do corpo **não** é implementada (§3).

```aura
def f() -> int { return "x" }   // E101: expected Int, got String
def g() -> int { }              // accepted; returns None at runtime
def h() { return "x" }          // no annotation; no check
```

---

## 7. Genéricos e restrições

- **Apagados.** Um parâmetro de tipo resolve para `TypeVariable` apenas
  **dentro** de seu escopo declarante (`types.py:811-815, 871-874`).
  `TypeVariable.is_compatible` é incondicionalmente `True` (`types.py:277-279`),
  então um parâmetro tipado como `T` aceita qualquer coisa.
- `Box[int]` **não** é um `ClassType` parametrizado; o nome com colchetes cai em
  `AnyType` (`types.py:1492-1494`). Argumentos genéricos não são checados em
  lugar algum.
- **Restrições são apenas resolvidas por nome.** `_check_type_constraints`
  (`types.py:653-691`) reporta `E110` quando uma restrição não é um builtin, uma
  classe/trait declarada, ou uma união destes. Ele **não** checa que um argumento
  de tipo fornecido satisfaz a restrição — a aplicação no local de chamada é
  **UNSPECIFIED/ausente**.
- Um parâmetro declarado mas não usado é o warning `W103`
  (`UNUSED_TYPE_PARAMETER`, `types.py:899-915`).
- O Python emitido para uma classe genérica preserva a maquinaria:
  `class Box(_aura_Generic[_aura_TypeVar('T')])` (*probe*) — mas isso é emit,
  não checagem.

---

## 8. O que o checker aplica, por código

Os códigos ficam em `aura/transpiler/errors.py`; o catálogo `E1xx`/`W1xx` é
espelhado em `docs/ERRORS.md` e guardado por `tests/test_diagnostics.py`.

| Código | Nome | Detecta | Local |
|---|---|---|---|
| `E101` | `TYPE_MISMATCH` | incompatibilidade declarado-vs-real em variável/constante/retorno/condição | `types.py:788,803,1276,1338` |
| `E105` | `WRONG_ARGUMENT_COUNT` | argumentos demais, ou menos que o exigido | `types.py:1416,1427` |
| `E106` | `WRONG_ARGUMENT_TYPE` | argumento incompatível com um tipo de parâmetro declarado | `types.py:1445` |
| `E108` | `INCOMPATIBLE_OPERANDS` | `+`/`-`/`*`/`/`/`%`/`**`, comparações, bit a bit sobre operandos errados | `types.py:1360,1371,1379,1387` |
| `E109` | `NON_EXHAUSTIVE_MATCH` | `match` não exaustivo sobre `bool`/enum/int/str (**warning**) | `types.py:1134-1161` |
| `E110` | `UNKNOWN_TYPE_CONSTRAINT` | nome de restrição genérica não resolve | `types.py:666,686` |
| `W103` | `UNUSED_TYPE_PARAMETER` | parâmetro de tipo de classe nunca usado (**warning**) | `types.py:911-915` |

Todos os seis códigos `E1xx` são a **totalidade** da superfície de erro de tipo.
`E0xx` são erros de sintaxe do parser; `E3xx` são regras estruturais do
`RuleChecker`.

### 8.1 O que **não** é aplicado (apagado ou não checado)

- compatibilidade de argumento de tipo (`Box[int]` vs `Box[str]`);
- tipos de elemento de literais list/dict/set **em atribuições**:
  `let a: [int] = [1, "x"]` é aceito (*probe*);
- formas de tipo estrutural com chaves: qualquer `dict` casa com
  `{name: str, age: int}`;
- alargamento/redução numérico e coerção `bool`/numérico (sem modelo de coerção);
- legalidade de operador sobre classes do usuário (sem resolução estilo `__add__`);
- nomes indefinidos, chamadas de não-chamáveis, nomes de tipo desconhecidos
  (deixados para o Python);
- `bytes` como tipo checado (resolve para `Any`);
- aliases de tipo (`TypeDecl` é pulado, `types.py:719-720`);
- mutabilidade — esse é o checker de mutabilidade separado (`E303`);
- abstract/visibilidade/herança — `RuleChecker` (`E309`/`E316`/`E308`/…).

---

## 9. Apagamento: anotações e o emit Python

Aura transpila para Python. `_aura_type_to_python`
(`statements.py:770-798`) mapeia uma anotação para um *nome de runtime*; a
anotação **não** chega ao Python como uma checagem de tipo.

| Anotação Aura | Python emitido |
|---|---|
| `int`/`float`/`str`/`bool`/`bytes` | `int`/`float`/`str`/`bool`/`bytes` |
| `none` (ou `null`) | `None` |
| `any` | `object` |
| `Never` | `type(None)` |
| `[T]` | `list` |
| `{K: V}` | `dict` |
| `[T]` chaves uniformes | `list` |
| `(A) -> B` ou outro `->` | `object` |
| uma união `T \| U` | o texto **literal** `T \| U` |
| um nome nu | esse nome |

Consequências, verificadas por *probe*:

- `let x: int = 1` emite exatamente `x = 1` — **sem anotação**.
- `def f(a: int) -> str { ... }` emite `def f(a):` — anotações de parâmetro e
  retorno são **descartadas**.
- `let y: str? = none` emite `y = None`.

Então a *única* informação de tipo que sobrevive ao runtime é (a) a maquinaria
genérica em classes genéricas, (b) o alias/nome de melhor esforço emitido para
`type`, e (c) o texto da união. Se uma anotação é uma **checagem em tempo de
compilação** ou uma **anotação apagada** importa, portanto: as checagens em §8
rodam apenas sob `aura check` / LSP / REPL, e nunca em `aura run`.

---

## 10. Não é uma garantia: lista rápida

- A compatibilidade de atribuição **não** é uma rede de segurança: nomes
  desconhecidos e `Any` passam silenciosamente; a direção de desempacotamento de
  opcional é aceita.
- A inferência é **livre de contexto** e apenas do primeiro elemento para
  coleções.
- Genéricos são apagados em ambos os lados; restrições são cosméticas.
- Tipos estruturais são dicts opacos.
- O checker roda em `aura check`, não em `aura run`/`aura transpile`.

Onde uma regra não está declarada acima, ela é **UNSPECIFIED** em vez de
presumida.
