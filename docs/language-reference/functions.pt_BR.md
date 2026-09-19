---
layout: default
title: "Funções"
nav_exclude: true
---

[English](functions.md) | [Português](functions.pt_BR.md)

# Funções

**Status:** Stable (exceto onde rotulado) · **Evidência:** `parser/to_ast.py`
(`parse_function_decl` §1116, `parse_function_decl_after_keyword` §2028),
`transpiler/transformers/statements.py` (`transform_FunctionDecl` §319),
`transpiler/rules.py` (`RuleChecker`).

---

## 1. Declaração

```
function_decl  = modifiers , [ "async" ] , "def" , identifier , [ type_params ]
                 , "(" , [ param_list ] , ")" , [ "->" , type ] , block ;
type_params    = "[" , identifier , { "," , identifier } , "]" ;
```

`def` é a **única** keyword de função. `fn` é rejeitado com uma mensagem apontada
(`to_ast.py:1123-1126`); `lambda`, `fun` e `func` não são Aura.

Um corpo de função é ou um **bloco com chaves** ou um **corpo de expressão**:

```aura
def greet(name) -> str {
  return "Hello, " + name
}

def square(x: int) -> int = x * x     // expression body
def double(x) = x * 2                 // no return type
```

Um corpo de expressão é dessugarizado para `return <expr>` em tempo de parse
(`to_ast.py:1200-1204`).

*Evidência:* `test_syntax_complete.py::test_def_and_fn_forms`.

### 1.1 Tipo de retorno

- **Opcional.** `def f() { }` não tem anotação; o Python emitido é `def f():`.
- A anotação vem após `->` (`def f() -> int`).
- **UNSPECIFIED:** a anotação de retorno é documentação; o Python emitido não
  carrega anotação de tipo de retorno (`transform_FunctionDecl`,
  `statements.py:351-391`), então uma incompatibilidade entre o tipo de retorno
  declarado e o real não é aplicada pelo transpiler.

### 1.2 `async`

`async def` emite `async def`, e `await` pode aparecer em seu corpo.

```aura
async def fetch_data(url) -> str {
  let response = await http_get(url)
  return response.body
}
```

*Evidência:* `test_syntax_complete.py::test_async_await`.

---

## 2. Parâmetros

```
param_list     = param , { "," , param } ;
param          = "*" | "**" , identifier
               | identifier , [ ":" , type ] , [ "=" , expression ] ;
```

Duas formas por posição: anotado (`name: type`) ou nu, ambas com um default
opcional `= expr`.

```aura
def f(name, greeting = "Hello") { return greeting + ", " + name }
```

### 2.1 Parâmetros default

Defaults são emitidos diretamente na assinatura Python (`py_safe_name(name)=default`,
`statements.py:343-345`). Uma chamada pode omitir qualquer argumento com default
à direita.

```aura
def greet(name, greeting = "Hello") -> str { return greeting + ", " + name }
greet("Alice")            // "Hello, Alice"
greet("Bob", "Hi")        // "Hi, Bob"
```

*Evidência:* `test_syntax_complete.py::test_function_default_and_kwargs`.

> **UNSPECIFIED:** uma chamada que omite um argumento *não final* por keyword
> funciona em Python, mas a gramática de Aura não aplica regras de ordem de
> parâmetros além do que o alvo aceita.

### 2.2 Argumentos nomeados / keyword

Em um local de chamada, um argumento escrito `name: expr` **ou** `name = expr`
torna-se um argumento keyword do Python (`to_ast.py:3021-3035`).

```aura
def create_user(name, age, email) {
  return {name: name, age: age, email: email}
}

let u = create_user(name: "Alice", age: 30, email: "a@x.com")
```

*Evidência:* *probe* — `create_user(age: 3, name: 'a')` imprime `a3`.

### 2.3 Parâmetros variádicos

- `*args` — variádico posicional (tupla).
- `**kwargs` — mapeamento apenas keyword.
- Um **`*` nu** marca os parâmetros seguintes como apenas keyword
  (`to_ast.py:1147-1155`).

```aura
def sum_all(*numbers) -> int {
  return sum(numbers)
}

def log(level, **context) {
  print(f"[{level}] {context}")
}

def f(a, *, b) { return a + b }   // b must be passed as a keyword
```

*Evidência:* `test_syntax_complete.py::test_variadic_and_kwonly`,
`::test_spread_call`, `::test_spread_dict_call`.

### 2.4 Spread no local de chamada

Argumentos de chamada aceitam `*iterable` (spread posicional), `**mapping`
(spread keyword) e `...value` (adaptativo: dict → kwargs, caso contrário
posicional).

```aura
add(*nums)      // add(a, b, c) from a list
add(**kw)       // add(a=…, b=…) from a dict
add(...nums)    // adaptive
```

*Evidência:* `test_syntax_complete.py::test_spread_call`,
`::test_spread_dict_call`, `::test_adaptive_spread`.

> **UNSPECIFIED:** nomes de parâmetro duplicados não são checados pela gramática;
> o Python alvo os rejeita.

---

## 3. Retorno

- `return` com um valor, ou `return` nu.
- Um `return` nu em posição que produz valor gera `None`
  (`transform_ReturnStmt`, `statements.py:1287-1291`).
- Múltiplos valores são escritos como uma lista com vírgulas e viram uma **tupla**
  (`to_ast.py:2215-2223`).

```aura
def swap(a, b) {
  return b, a
}

let x, y = swap(1, 2)     // x = 2, y = 1
```

Retornos são cobertos em profundidade em [statements.md](statements.md) §9.

*Evidência:* `test_syntax_complete.py::test_swap`.

---

## 4. Genéricos

```
function_decl  = … , "def" , identifier , type_params , "(" , … ;
type_params    = "[" , identifier , { "," , identifier } , "]" ;
```

Parâmetros de tipo usam **colchetes** e precedem a lista de parâmetros. Uma
restrição pode seguir `:` em um parâmetro (`grammar.md` §4).

```aura
class Comparable {
  public def compare(other: Comparable) -> int { return 0 }
}

def id[T](x: T) -> T { return x }
def smallest[T: Comparable](items: [T]) -> T { return items[0] }
def map[T, R](f: (T) -> R, items: [T]) -> [R] { return [f(i) for i in items] }
```

- **UNSPECIFIED:** parâmetros de tipo são apagados em tempo de transpile — a
  função Python emitida é não tipada (`transform_FunctionDecl`,
  `statements.py:319-391`), então `id(7)` não precisa de `[int]` explícito.
- Restrições são checadas quanto à resolução em outro lugar (veja `TYPES.md`),
  não aqui.

*Evidência:* *probe* — `def id[T](x: T) -> T { return x }` transpila para
`def id(x): return x`.

---

## 5. Recursão

A recursão direta funciona; a chamada resolve contra a função envolvente.

```aura
def factorial(n) -> int {
  if n <= 1 { return 1 }
  return n * factorial(n - 1)
}

factorial(5)     // 120
```

*Evidência:* *probe* (120). **UNSPECIFIED:** nenhuma otimização de chamada de
cauda é garantida; a recursão profunda é limitada pela pilha do CPython.

---

## 6. Aninhamento

Uma função declarada dentro de outro corpo de função é **hoisted**: o `def`
interno é emitido no ponto de declaração dentro da função Python envolvente.

```aura
def main() {
  def helper(x: int) -> int { return x + 1 }
  print(helper(2))     // 3
}
```

*Evidência:* *probe* (3), `test_comprehensive.py::test_nested_function`.

---

## 7. Onde as funções vivem / visibilidade

- Funções de **nível superior** emitem `def`s Python em nível de módulo.
- Dentro de uma **classe**, `transform_FunctionDecl` transforma um método
  `private` em `__name` e um `protected` em `_name` (`statements.py:331-333`).
  Regras de visibilidade e unicidade de membros estão documentadas em
  [classes.md](classes.md).
- `@staticmethod` é emitido para `is_static` (`statements.py:322-323`).

Não há **sobrecarga** em nível de linguagem: um nome repetido no mesmo corpo de
classe é **E301** (veja `LANGUAGE.md` §8 "Unique members"). Funções de nível
superior com o mesmo nome sobrescreveriam silenciosamente em Python.

---

## 8. Decorators

```
decorator      = "@" , dotted_name , [ "(" , [ arg_list ] , ")" ] ;
```

Um decorator em um `def` é permitido; um decorator em um **campo** é rejeitado
(`E320`, `LANGUAGE.md` §17). Macros de runtime embutidas incluem `@debug`,
`@timeit`, `@memoize`, `@cache(maxsize=…)`, `@must_return` e `@deprecated(…)`;
`@property`, `@staticmethod` e `@classmethod` são decorators de membro de classe.

```aura
@memoize
def fib(n) -> int {
  if n < 2 { return n }
  return fib(n - 1) + fib(n - 2)
}

@timeit
def slow() { }
```

*Evidência:* `test_syntax_complete.py::test_decorator`,
`::test_builtin_macros_present`.

---

## 9. Ponto de entrada (`main`)

Um **arquivo de entrada** (executado com `aura run`) deve declarar um `main` de
nível superior; o runtime o invoca. Um arquivo importado como módulo não precisa
de `main`.

- `main` ausente em um arquivo executado → **E310**.
- Um `main` com qualquer outra assinatura → **E311** (`main` não recebe
  parâmetros, ou recebe um único parâmetro `args`).
- Um `main` dentro de um corpo de `module` → **E312**.

```aura
def main() {
  print("hello")
}
```

Um `main` que recebe os argumentos de linha de comando:

```aura
def main(args: [string]) {
  print(f"{args.length()} argument(s)")
}
```

`main` pode retornar um `int` para definir o código de saída, e pode ser `async`.

*Evidência:* `rules.py:_check_main`, `LANGUAGE.md` §1.
