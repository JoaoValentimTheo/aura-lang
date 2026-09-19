---
layout: default
title: "Closures e Lambdas"
nav_exclude: true
---

[English](closures.md) · [Português](closures.pt_BR.md)

# Closures e Lambdas

**Status:** Stable (exceto onde rotulado) · **Evidência:** `parser/to_ast.py`
(parse de lambda §2657-2700, `_is_lambda_params_ahead` §2871, `_parse_lambda_params`
§2893, `parse_lambda_body` §2946), `transpiler/transformers/expressions.py`
(`transform_LambdaExpr` §662, `_hoist_block_lambda` §689,
`_nonlocal_declaration` §711, `transform_PipeExpr` §578).

Uma lambda é um **valor** de função anônima. Uma closure é uma lambda que captura
variáveis do escopo envolvente.

---

## 1. Formas de lambda

```
lambda         = ( identifier | "(" , [ param_list ] , ")" ) , "=>" , ( expression | block ) ;
```

| Forma | Exemplo |
| --- | --- |
| Um parâmetro, sem parênteses | `x => x * 2` |
| Parênteses, um parâmetro | `(x) => x * x` |
| Múltiplos parâmetros | `(a, b) => a + b` |
| Sem parâmetros | `() => 42` |
| Corpo de expressão | `(x) => x + 1` |
| Corpo de bloco | `(x) => { let y = x + 1; return y }` |
| Currying | `(n) => (x) => x + n` |

```aura
let double = x => x * 2
let square = (x) => x * x
let add = (a, b) => a + b
let get_answer = () => 42
```

- O marcador `=>` é obrigatório; **não há keyword `lambda`** (`to_ast.py:2650-2653`,
  *probe*).
- Um **corpo de expressão** é emitido como uma expressão `lambda` do Python,
  ex.: `(lambda x: (x * 2))` (`transform_LambdaExpr`, `expressions.py:662-669`).
- Um **corpo de bloco** não pode viver dentro de uma `lambda` Python; ele é
  **hoisted** para uma função nomeada real `_aura_lambda_N` e o valor da lambda
  torna-se esse nome (`_hoist_block_lambda`, `expressions.py:689-708`).

```aura
let compute = (x) => {
  let doubled = x * 2
  return doubled + 1
}
```

*Evidência:* `test_syntax_complete.py::test_lambda_forms`,
`::test_lambda_block_body`, `test_expressions_deep.py::test_transform_simple_lambda`.

### 1.1 Parâmetros

Parâmetros de lambda usam a mesma gramática dos parâmetros de `def`
(`_parse_lambda_params`, `to_ast.py:2893-2944`): anotados, com default, `*args`,
`**kwargs` e um `*` nu.

```aura
let f = (a, *rest) => a
let g = (a, **kw) => a
let h = (a, b = 1) => a + b
let typed = (x: int) => x + 1      // annotation accepted, then erased
```

- Tipos em parâmetros de lambda são **aceitos** na forma `=>`
  (`parse_lambda_params` chama `parse_type`, `to_ast.py:2934-2935`).
- **UNSPECIFIED / apagado:** a `lambda` Python emitida não carrega anotações,
  então a anotação é apenas documentação (*probe*: `(x: int) => x + 1` emite
  `(lambda x: (x + 1))`).
- Um **tipo de retorno `->` não** faz parte da sintaxe de lambda:
  `(x: int) -> int { … }` é erro de parse (*probe*). Use a forma `=>`.

---

## 2. Closures

### 2.1 Captura de leitura (snapshot / variável livre)

Uma lambda que lê um local envolvente o captura por referência na closure Python
gerada.

```aura
def main() {
  let n = 10
  let f = () => n + 1
  print(f())          // 11
}
```

*Evidência:* *probe* (11).

### 2.2 Captura mutável (`nonlocal`)

Quando uma **lambda de bloco** atribui a um local envolvente, a função hoisted
declara `nonlocal <name>`, então a escrita é visível fora da lambda
(`_nonlocal_declaration`, `expressions.py:711`).

```aura
def main() {
  let mut n = 0
  let inc = () => { n = n + 1 }
  inc()
  inc()
  print(n)            // 2
}
```

Forma emitida (abreviada):

```python
def _aura_lambda_1():
    nonlocal n
    n = (n + 1)
inc = _aura_lambda_1
```

*Evidência:* *probe* (2), `test_expressions_deep.py::test_hoist_block_lambda_includes_nonlocal`.

### 2.3 Currying / retornar uma lambda

```aura
let make_adder = (n) => (x) => x + n
let add10 = make_adder(10)
print(add10(5))       // 15
```

*Evidência:* *probe* (15).

> **UNSPECIFIED:** a captura é por referência através do protocolo de closure do
> Python, não um snapshot explícito. Uma lambda criada em um loop compartilha a
> célula da variável do loop, então ela observa o valor **final**: *probe* com
> `for i in range(3) { fs.add((x) => x + i) }` dá `fs[0](0) == 2`, não `0`. Vincule
> o valor como um parâmetro default (`(x, i = i) => x + i`) para captura por
> iteração.

---

## 3. Lambdas com `map` / `filter` / `reduce`

Estas funções são auto-importadas de `stdlib.collections` quando usadas
(o transformer injeta `from stdlib.collections import map, filter, reduce, …`).

```aura
let numbers = [1, 2, 3, 4, 5]
let doubled = map(numbers, (x) => x * 2)
let evens = filter(numbers, (x) => x % 2 == 0)
let total = reduce(numbers, (a, b) => a + b, 0)
```

*Evidência:* *probe* — transpila para
`map(numbers, (lambda x: (x * 2)))` etc.

---

## 4. Operador pipe

O pipe `|>` aplica o chamável à direita ao valor à esquerda como seu
**primeiro argumento** (`transform_PipeExpr`, `expressions.py:578-591`).

```aura
let result = [1, 2, 3, 4, 5]
  |> filter((x) => x > 2)
  |> map((x) => x * 10)
  |> reduce((a, b) => a + b, 0)
```

O pipeline é da esquerda para a direita. *Probe*: remover o estágio final dá
`[30, 40, 50]`, e a cadeia completa com `reduce` dá `120`. Se o lado direito é um
identificador nu em vez de uma chamada, ele se torna `f(left)`.

*Evidência:* *probe*, `test_syntax_complete.py::test_pipe_operator`,
`test_expressions_deep.py::test_transform_pipe_into_call_inserts_argument`,
`::test_transform_pipe_into_identifier`.

---

## 5. Lambda à direita / argumentos de bloco

**Aura não tem açúcar de chamada trailing-lambda.** Um `{ ... }` após uma chamada
não é anexado como argumento final; *probe* mostra que `apply(1) { x => x + 1 }`
parseia como a chamada `apply(1)` seguida de um statement de expressão lambda
separado, e `transaction { print("x") }` parseia como dois statements. Argumentos
de chamada devem ficar **dentro** dos parênteses.

```aura
apply((x) => x + 1)          // the lambda is a normal argument
```

Um `{` após um identificador **capitalizado** é um struct init
(`TypeName { field: value }`, `grammar.md` §6.6), não um argumento de bloco.

*Evidência:* *probe*; `to_ast.py:2986-3052` (parse de argumentos de chamada),
`to_ast.py:2678-2679` (ramo de struct-init).

---

## 6. Limites

- **Sem keyword `lambda`** — `lambda x: x` é um erro de parse apontado
  (`to_ast.py:2650-2653`).
- **Sem parâmetro implícito** (`it` / `$0`); nomeie todo parâmetro.
- Uma lambda aceita `(x: T) => …` (**anotado, apagado**), mas **não** tipo de
  retorno `->`: `(x: int) -> int { … }` é erro de parse (*probe*). Veja §1.1.
- **Recursão através de uma lambda ligada a `let` funciona**, para corpos de
  expressão e de bloco: a resolução de nome acontece no momento da chamada,
  depois que o `let` liga o nome. *Probe*: tanto
  `let fact = (n) => n <= 1 ? 1 : n * fact(n - 1)` quanto o equivalente com corpo
  de bloco dão `fact(5) == 120`.
- **UNSPECIFIED:** não há variável de autorreferência implícita; uma lambda deve
  se referir a si mesma através do nome ao qual está ligada.

---

## 7. Tipos de função

O tipo de uma lambda é um **tipo de função** escrito `(T, …) -> R`
(`grammar.md` §4, `LANGUAGE.md` §4).

```aura
let handler: (int, int) -> int = add
```

> **UNSPECIFIED:** tipos de função são apagados em tempo de transpile; o Python
> emitido é não tipado. Veja [types.md](types.md) para as regras de checagem de
> tipos.
