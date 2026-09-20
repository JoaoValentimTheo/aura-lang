---
layout: default
title: "Tipos — Catálogo e Sintaxe"
nav_exclude: true
---

[English](types.md) · [Português](types.pt_BR.md)

# Tipos — Catálogo e Sintaxe

**Status:** Stable (`0.2.0a7`, exceto onde rotulado) · **Evidência:**
`aura/parser/to_ast.py` (`parse_type` §1752, `parse_type_decl` §1653,
`_parse_type_params` §1129), `aura/transpiler/types.py`
(`TypeInference.BUILTIN_NAMES`, `_parse_type_annotation` §1456),
`aura/transpiler/transformers/statements.py` (`transform_TypeDecl`,
`_aura_type_to_python` §770), `types.md`, *probes* de execução.

Este documento lista **quais tipos existem** e **como são escritos**. As regras de
*validade* — o que pode ser atribuído a quê, quando há erro e quais anotações são
apagadas — estão em [type-system.md](type-system.md).

Aura é **gradualmente tipada**: anotações são opcionais e, exceto pelas checagens
em [type-system.md](type-system.md) §5, são **apagadas** antes do emit Python.
O parser armazena um tipo de declaração como uma **string** (`parse_type` retorna
`str`); o checker consome essa string com `_parse_type_annotation`.

---

## 1. Tipos primitivos (6)

| Tipo Aura | Instância `Type` | Emit Python | Notas |
|---|---|---|---|
| `int` | `IntType` | `int` | precisão arbitrária (Python) |
| `float` | `FloatType` | `float` | 64 bits (Python) |
| `str` | `StrType` | `str` | Unicode |
| `bool` | `BoolType` | `bool` | `true` / `false` |
| `bytes` | (`AnyType` — veja abaixo) | `bytes` | sem classe `Type` dedicada |
| `none` | `NoneType` | `None` | valor de ausência **e** tipo |

`TypeInference.BUILTIN_NAMES` (`types.py:293-302`) mapeia exatamente estas grafias:
`int`, `float`, `str`, `bool`, `none`, mais os aliases `string` e `null`
(→ `StrType` / `NoneType`) e `any` (→ `AnyType`).

- **`none` é o literal nulo e o tipo nulo.** O literal `null` **não faz parte de
  Aura**: `let n = null` é rejeitado pelo parser —
  `'null' is not part of Aura; use 'none' instead` (*probe*). Use `none`.
- **`bytes` não tem tipo no checker.** `let b: bytes = b"x"` parseia e transpila
  (emit `b'x'`), mas `bytes` está ausente de `BUILTIN_NAMES`, então
  `_parse_type_annotation("bytes")` cai em `AnyType` (*probe*). A anotação é
  aceita e apagada; ela **não** é aplicada.
- **`any`** é uma grafia real que mapeia para `AnyType` — a válvula de escape da
  tipagem gradual. `object` / `Never` também **parseiam** como nomes de tipo, mas
  resolvem para `AnyType` no checker (`Never` só tem tratamento especial no
  `_SIMPLE_TYPES` do emitter, não no checker).
- **Não existe** `uint`, `char`, `void`, `unit`, `Never` (aplicado), nem
  construtor de tipo `Optional`/`Result`.

### 1.1 `bool` não tem alargamento numérico

`BoolType().is_compatible(IntType())` é `False` e vice-versa (*probe*);
`true + 1` é `E108`. Não há coerção `bool → int` no checker, ao contrário do
modelo Kof.

---

## 2. Tipos de coleção

| Grafia | `Type` do checker | Emit Python | Status |
|---|---|---|---|
| `[T]` | `ListType(T)` | `list` | Stable |
| `{K: V}` | `DictType(K, V)` | `dict` | Stable |
| `{F1: T1, F2: T2}` | `DictType()` (opaco) | `dict` | Stable (estrutural) |
| `{T}` | **não parseável** | `set` | **Erro de sintaxe** (veja §2.3) |
| `(A, B)` | **não parseável** | `tuple` | **Erro de sintaxe** (veja §2.3) |
| `List[T]` | `ListType(T)` | `list` | grafia alias |
| `Set[T]` | `SetType(T)` | `set` | grafia alias |
| `Dict[K, V]` | `DictType(K, V)` | `dict` | grafia alias |

### 2.1 Listas

`[T]` parseia para a string `List[T]` (`to_ast.py:1773-1779`); o checker
re-lê tanto `[T]` quanto `List[T]` em `ListType` (`types.py:1472-1476`). Uma
forma de múltiplos argumentos `[T, U]` também é aceita pelo parser e vira
`List[T, U]`, mas o checker lê apenas o primeiro componente — **UNSPECIFIED**
além disso.

```aura
let xs: [int] = [1, 2, 3]
let nested: [[int]] = [[1], [2]]
let mixed = [1, "two", 3.0, true]   // inferred List[Int] (first element only)
```

A inferência do tipo do elemento usa **apenas o primeiro elemento**
(`types.py:330-333`): `[1, "two"]` infere `List[Int]`. O literal vazio `[]`
infere `List[Any]`.

### 2.2 Dicts e tipos estruturais

`{K: V}` e `{name: str, age: int}` usam o **mesmo ramo do parser**
(`to_ast.py:1781-1797`): um tipo com chaves é uma lista de `key: type` separada
por vírgulas e é armazenado verbatim como `{K: V}` / `{name: str, age: int}`.

O checker trata **qualquer** anotação com chaves como `DictType` e **não**
retém os nomes dos campos (`types.py:1486-1488`, e `StructuralType → DictType()`
em `types.py:1508-1509`). Consequências:

- `{str: int}` → `DictType(StrType, IntType)`.
- `{name: str, age: int}` → `DictType()` (opaco) — a **forma não é**
  checada; qualquer dict é compatível com ela.
- Chaves de entrada única sem `:` (ex.: `{int}`) são um **tipo set**
  (`Set[int]`) — veja §2.3.

Campos opcionais **não** são suportados: `{name: str, email?: str}` é erro de
sintaxe — `Expected ':' but got '?'` (*probe*). Um campo opcional é escrito por
união: `{name: str, email: str | none}`.

Uma palavra de visibilidade à esquerda é tolerada dentro de um tipo com chaves
(`{public name: str}` → `{name: str}`, `to_ast.py:1785-1787`).

### 2.3 Sintaxe de tipo set e tupla

Ambos parseiam e mapeiam para o genérico Python correspondente:

- `{T}` é um **tipo set**: `let s: {int} = {1, 2, 3}` → `Set[int]` (*probe*).
  Um tipo com chaves é um tipo set quando contém um único tipo sem `:`, e um
  tipo estrutural caso contrário (`{name: str}`).
- `(A, B)` é um **tipo tupla**: `let t: (int, str) = (1, "a")` → `Tuple[int, str]`
  (*probe*). Uma lista de tipos entre parênteses seguida de `->` é um tipo de
  função (§4).

Tipos set e tupla também são alcançáveis por suas grafias genéricas `Set[T]` /
`Tuple[T, U]`, e por inferência de um literal (`SetLiteral` → `SetType`,
`TupleLiteral` → `TupleType`, `types.py`). Os valores em runtime `{1,2,3}` e
`(1, "x")` transpilam para `set` e `tuple` do Python.

---

## 3. Tipos opcionais e de união

### 3.1 Opcional `T?`

```ebnf
optional-type = type-ref , "?" ;
```

`T?` parseia como um sufixo (`to_ast.py:1814-1816`) e o checker o rebaixa para
`UnionType({T, NoneType()})` (`types.py:1468-1469`).

```aura
let name: str? = get_name()
```

`T?` e `T | none` são o **mesmo tipo**. Não há classe `Optional` dedicada. Um `?`
pode se repetir (`T??`) — o parser anexa outro `?` e o checker aninha uniões; a
camada extra é **UNSPECIFIED**.

### 3.2 União `T | U`

```ebnf
union-type = type-ref , "|" , type-ref , { "|" , type-ref } ;
```

`|` é associativo à direita no parser (`parse_type` recorre no lado direito após
casar `|`, `to_ast.py:1817-1820`) e o checker constrói um **conjunto**
`UnionType` plano a partir das partes separadas (`types.py:1489-1491`).

```aura
let value: int | str = 42
let result: int | float | none = none
```

A pertinência da união é um `set` (`types.py:251`), então ordem e duplicatas não
são observáveis. Uma união é compatível com cada um de seus membros
(`UnionType.is_compatible` é `any(...)`, `types.py:257-258`); o reverso
(`int` declarado, união esperada) também vale, já que o lado inferido é o membro.
Veja [type-system.md](type-system.md) §5 para a direção da atribuição e a
ressalva de desempacotamento de opcional.

---

## 4. Tipos de função

```ebnf
function-type = "(" , [ type-ref , { "," , type-ref } ] , ")" , "->" , type-ref ;
```

```aura
let handler: (int, int) -> int = add
let predicate: (str) -> bool = is_valid
let callback: () -> none = on_ready
let compose: ((int) -> int) -> (int) -> int = make
```

O parser reconhece um `(` inicial como um tipo de função e retorna a string
`(A, B) -> C` (`to_ast.py:1760-1771`). O checker mapeia **qualquer** string que
comece com `(` e contenha `->` para um `FunctionType()` nu
(`types.py:1470-1471`) — tipos de parâmetro e retorno **não** são preservados na
anotação. Um tipo sem seta entre parênteses é erro de sintaxe.

Não há marcador `async` ou variádico na gramática de anotação; o `FunctionType`
do checker carrega `is_async`/`variadic`/`required_args`, mas estes são
preenchidos a partir de **declarações** (`_method_to_function_type`,
`_check_function_decl`), não de anotações de tipo de função.

---

## 5. Tipos genéricos

```ebnf
generic-type = name , "[" , type-ref , { "," , type-ref } , "]" ;
```

```aura
class Box[T] { public let value: T = none }
class Pair[A, B] { public let first: A = none }

def first[T](items: [T]) -> T | none { return items[0] }
def map[T, R](items: [T], f: (T) -> R) -> [R] { return [f(i) for i in items] }

let int_box: Box[int] = Box(42)
let p: Pair[str, int] = Pair("age", 30)
```

- **Apenas colchetes.** `Box<T>` é rejeitado com um erro apontado: *"type
  arguments use brackets, not '<...>' (write 'Box[T]')"* (`to_ast.py:1809-1813`).
- Parâmetros de tipo são introduzidos como `Name[T, U]` na declaração
  (`_parse_type_params`, `to_ast.py:1129`) e resolvidos para `TypeVariable`
  dentro do escopo da declaração (`types.py:811-815, 871-874`).
- `Box[int]` parseia para a string `Box[int]`; o checker **não** constrói um
  `ClassType` parametrizado — um nome com colchetes que não é `List`/`Set`/
  `Dict` cai em `AnyType` (`types.py:1492-1494`). Argumentos genéricos são
  efetivamente **apagados** em ambos os lados.
- Nós AST `GenericType` (construção programática) mapeiam para
  `ListType`/`SetType` para essas duas bases e para `AnyType` caso contrário
  (`types.py:1503-1507`).

### 5.1 Restrições genéricas

Um parâmetro pode carregar uma restrição após `:`. A restrição é um tipo embutido
ou uma classe/trait declarada no mesmo programa; uma união de tais nomes é
aceita. Qualquer outra coisa é `E110` (`UNKNOWN_TYPE_CONSTRAINT`,
`types.py:653-691`).

```aura
class Comparable { public def compare(other: Comparable) -> int { return 0 } }

def smallest[T: Comparable](items: [T]) -> T { return items[0] }
def display[T: int | str](value: T) -> str { return str(value) }
```

Restrições são **apenas em tempo de compilação**: o Python emitido não muda,
exceto pela maquinaria de classe genérica (`Generic[TypeVar('T')]`); o checker
verifica apenas que todo nome de restrição **resolve** — ele não aplica que os
locais de chamada a satisfaçam. Um parâmetro declarado mas nunca usado dispara o
warning **`W103`** (`UNUSED_TYPE_PARAMETER`).

Parâmetros de tipo são parseados para `class`, `trait`, `def` e `type`. Uma
restrição sobre um nome que não é um dos parâmetros do dono é `E110`.

---

## 6. Tipos de classe e alias de tipo

### 6.1 Classes, traits, enums

Um nome de `class`/`trait`/`enum` declarado no programa é um tipo, escrito por
nome (`User`, `Animal`). Um nome pontuado (`pkg.Base`) **não** parseia como
anotação de tipo — `foo.bar.Baz` → `Unexpected token '.'` (*probe*); bases
pontuadas são tratadas apenas na cláusula `extends`, não em `parse_type`. Veja
[classes.md](classes.md).

### 6.2 Aliases de tipo

```ebnf
type-decl = "type" , name , [ "[" , type-param , { "," , type-param } , "]" ] , "=" , type-ref ;
```

```aura
type UserId = int
type Point = {x: float, y: float}
type Pair[A, B] = (A, B)
```

`TypeDecl` armazena o alias como a **string** `type_expr` e **não** é resolvido
pelo checker (`types.py:719-720` pula `TypeDecl`). Usar um nome de alias em uma
anotação, portanto, resolve via `self.classes`/`BUILTIN_NAMES` e, caso contrário,
para `AnyType` (`types.py:1492-1494`). O alias emite um nome Python de melhor
esforço (`UserId = int  # type alias: int`,
`statements.py:751-798`).

Aliases de tipo de união parseiam e emitem corretamente:
`type Maybe = int | none` registra `type_expr = 'int | none'` e emite
`Maybe = int | none  # type alias: int | none` (um componente de união após `|`
continua o tipo em vez de iniciar um novo statement).

---

## 7. Literais e seus tipos inferidos

Inferido por `TypeInference.infer` (`types.py:313-371`):

| Literal | Inferido | Evidência |
|---|---|---|
| `42` | `int` | `IntLiteral` → `IntType` |
| `3.14` | `float` | `FloatLiteral` → `FloatType` |
| `"…"` / `f"…"` | `str` | `StrLiteral`/`FStringLiteral` → `StrType` |
| `b"…"` | **`Any`** | nenhum ramo de literal `bytes` |
| `true` / `false` | `bool` | `BoolLiteral` → `BoolType` |
| `none` | `none` | `NoneLiteral` → `NoneType` |
| `[1, 2]` | `[int]` (primeiro elemento) | `types.py:330-333` |
| `{1, 2}` | set `{int}` | `SetLiteral` → `SetType` |
| `(1, "x")` | `(int, str)` | `TupleLiteral` → `TupleType` |
| `{a: 1}` | `{str: int}` (primeiro par) | `types.py:340-345` |
| `1..10` | `[int]` | `RangeExpr` → `ListType(IntType)` |
| `1 + 2` | `int` | `_infer_binary_op` |
| `3 / 2` | `float` | `/` sempre produz `float` |
| `"a" + "b"` | `str` | concatenação de string |
| `x ?? y` | `strip-none(x)` | `CoalesceExpr` |
| `x?.f` | `Optional(infer x)` | `SafeNavExpr` |

A inferência binária (`types.py:444-480`) é um modelo **local ao checker**. Note
que `3 / 2` é `float` aqui mesmo que o `/` do Python sobre ints também seja
float — consistente; não há distinção de operador de divisão inteira nesta
tabela.

---

## 8. O que NÃO é um tipo

| Grafia | Resultado | Evidência |
|---|---|---|
| `null` (literal ou tipo) | Erro de sintaxe — use `none` | *probe*: `'null' is not part of Aura` |
| `{T}` (anotação set) | Erro de sintaxe (`Expected ':'`) | ramo `{` de `parse_type` |
| `(A, B)` (anotação tupla) | Erro de sintaxe (`Expected '->'`) | ramo `(` de `parse_type` |
| `email?: str` (campo opcional) | Erro de sintaxe | *probe* |
| `Box<T>` (genéricos com ângulo) | Erro de sintaxe, mensagem apontada | `to_ast.py:1809-1813` |
| `pkg.Type` em uma anotação | Erro de sintaxe (`Unexpected '.'`) | *probe* |
| `void` / `unit` | não é nome de tipo; `none` é o tipo de ausência | `BUILTIN_NAMES` |
| `uint` / `char` | ausente | `BUILTIN_NAMES` |
| `Optional[T]` / `Result[T, E]` | tratado como nome genérico → `AnyType`, sem checagem | `types.py:1492-1494` |
| tipos de interseção, tipos literais/singleton | ausente | — |
| tipo de `bytes` **checado** | parseia, mas `AnyType` (apagado) | `BUILTIN_NAMES` |

---

## 9. Recursão de tipo

Um tipo pode se referir a si mesmo **por nome** em um campo de classe, uma
assinatura de método ou um tipo estrutural com chaves:
`class Node { public let next: Node? = none }`. O checker resolve nomes
preguiçosamente e apaga nomes desconhecidos para `AnyType`, então a recursão é
permitida e finita — a resolução é por nome, não por expansão.
`List[List[int]]` aninha livremente. **Tipos recursivos são permitidos.**
