# Sistema de Tipos do Aura

Referencia do sistema de tipos do Aura. Aura usa tipagem gradual com inferencia de tipos.

---

## Sumario

1. [Visao Geral](#1-visao-geral)
2. [Tipos Primitivos](#2-tipos-primitivos)
3. [Tipos de Colecao](#3-tipos-de-colecao)
4. [Tipos de Funcao](#4-tipos-de-funcao)
5. [Tipos Union](#5-tipos-union)
6. [Tipos Genericos](#6-tipos-genericos)
7. [Tipos Opcionais](#7-tipos-opcionais)
8. [Tipos Estruturais](#8-tipos-estruturais)
9. [Tipos de Classe](#9-tipos-de-classe)
10. [Inferencia de Tipos](#10-inferencia-de-tipos)
11. [Anotacoes de Tipo](#11-anotacoes-de-tipo)
12. [Compatibilidade de Tipos](#12-compatibilidade-de-tipos)
13. [Mapeamento para Python](#13-mapeamento-para-python)

---

## 1. Visao Geral

Aura fornece **tipagem gradual** com **inferencia de tipos**. Tipos sao opcionais, mas recomendados para assinaturas de funcoes e definicoes de classes. O sistema de tipos fornece:

- Deteccao automatica de tipos a partir de literais e operacoes
- Tipos union para combinar multiplos tipos
- Tipos genericos com sintaxe de colchetes `[T]`
- Reducao de tipos atraves do fluxo de controle
- Validacao de tipos em tempo de compilacao

---

## 2. Tipos Primitivos

| Tipo Aura | Equivalente Python | Descricao |
|-----------|-------------------|-----------|
| `int` | `int` | Inteiro de precisao arbitraria |
| `float` | `float` | Ponto flutuante de 64 bits |
| `str` | `str` | String Unicode |
| `bool` | `bool` | `true` ou `false` |
| `bytes` | `bytes` | Sequencia de bytes |
| `none` | `None` | Valor nulo |

```aura
let x: int = 42
let y: float = 3.14
let s: str = "hello"
let b: bool = true
let n: none = null
```

---

## 3. Tipos de Colecao

### Listas

```aura
// Sintaxe de tipo
[int]                    // Lista de inteiros
[str]                    // Lista de strings
[list[int]]              // Lista aninhada

// Uso
let numeros: [int] = [1, 2, 3]
let mista = [1, "dois", 3.0, true]
let vazia = []
```

### Dicionarios

```aura
// Sintaxe de tipo
{str: int}               // Dict com chaves str, valores int
{str: any}               // Dict com valores any

// Uso
let usuario: {nome: str, idade: int} = {nome: "Alice", idade: 30}
let config = {"tamanho-max": 100, "timeout": 30}
```

### Conjuntos

```aura
// Sintaxe de tipo
{int}                    // Conjunto de inteiros

// Uso
let unico: {int} = {1, 2, 3}
```

### Tuples

```aura
// Tuples sao representadas como listas de tamanho fixo no sistema de tipos
// Em runtime: (1, "hello") e uma tuple do Python
```

---

## 4. Tipos de Funcao

```aura
// Sintaxe de tipo: (TiposParametro) -> TipoRetorno

let manipulador: (int, int) -> int = somar
let predicado: (str) -> bool = e_valido
let callback: () -> none = ao_pronto
```

### Em assinaturas de funcoes

```aura
def aplicar(f: (int) -> int, x: int) -> int {
  return f(x)
}

def compor(f: (int) -> int, g: (int) -> int) -> (int) -> int {
  return (x) => g(f(x))
}
```

---

## 5. Tipos Union

Tipos union permitem que um valor seja um de varios tipos:

```aura
let valor: int | str = 42
valor = "hello"  // tambem valido

// Em assinaturas de funcoes
def processar(valor: int | str) {
  // ...
}

// Multiplos tipos
let resultado: int | float | none = null
```

---

## 6. Tipos Genericos

Generics usam **colchetes** `[T]`:

### Classes genericas

```aura
class Caixa[T] {
  let valor: T

  def new(valor: T) {
    self.valor = valor
  }

  def obter() -> T {
    return self.valor
  }
}

let caixa_int: Caixa[int] = Caixa(42)
let caixa_str: Caixa[str] = Caixa("hello")
```

### Multiplos parametros de tipo

```aura
class Par[A, B] {
  let primeiro: A
  let segundo: B

  def new(primeiro: A, segundo: B) {
    self.primeiro = primeiro
    self.segundo = segundo
  }
}

let p: Par[str, int] = Par("idade", 30)
```

### Funcoes genericas

```aura
def primeiro[T](itens: [T]) -> T | none {
  if len(itens) > 0 { return itens[0] }
  return none
}

def mapear[T, R](itens: [T], fn: (T) -> R) -> [R] {
  return [fn(item) for item in itens]
}
```

---

## 7. Tipos Opcionais

Qualquer tipo pode ser tornado nullable com `?`:

```aura
let nome: str? = obter_nome()   // pode ser str ou null

// Acesso seguro contra nulos
let maiusculo: str? = nome?.upper()

// Coalescencia nula
let display: str = nome ?? "Anonimo"
```

### Em assinaturas de funcoes

```aura
def buscar_usuario(id: int) -> User? {
  let usuario = db.buscar(id)
  return usuario
}

// Uso
let usuario = buscar_usuario(123)
guard usuario != null else {
  return
}
// usuario e garantido nao-nulo aqui
```

---

## 8. Tipos Estruturais

Tipos estruturais descrevem a forma de um valor:

```aura
let ponto: {x: float, y: float} = {x: 1.0, y: 2.0}

let usuario: {
  nome: str,
  idade: int,
  email?: str            // campo opcional
} = {nome: "Alice", idade: 30}
```

---

## 9. Tipos de Classe

Classes criam tipos nomeados:

```aura
class User {
  let nome: str = ""
  let idade: int = 0

  def new(nome: str, idade: int) {
    self.nome = nome
    self.idade = idade
  }
}

// User e agora um tipo
let alice: User = User("Alice", 30)
```

### Heranca e subtipagem

```aura
class Animal {
  let nome: str = ""
}

class Cachorro extends Animal {
  let raca: str = ""
}

// Cachorro e um subtipo de Animal
def processar_animal(a: Animal) {
  // ...
}

let d = Cachorro()
processar_animal(d)  // OK: Cachorro e subtipo de Animal
```

---

## 10. Inferencia de Tipos

O transpiler infere tipos automaticamente a partir de:

| Fonte | Inferencia |
|-------|-----------|
| `42` | `int` |
| `3.14` | `float` |
| `"hello"` | `str` |
| `true` | `bool` |
| `null` / `none` | `none` |
| `[1, 2, 3]` | `[int]` |
| `{a: 1}` | `{str: int}` |
| `1..10` | `[int]` |
| `x + y` (ambos int) | `int` |
| `"a" + "b"` | `str` |

### Exemplos

```aura
let x = 10              // inferido: int
let y = x + 5           // inferido: int
let nome = "Alice"      // inferido: str
let itens = [1, 2, 3]   // inferido: [int]
let ponto = {x: 10, y: 20}  // inferido: {x: int, y: int}

// Tipo de retorno inferido
def dobrar(x) = x * 2   // tipo de retorno inferido como int
```

---

## 11. Anotacoes de Tipo

### Anotacoes de variavel

```aura
let nome: str = "Alice"
let idade: int = 30
let notas: [float] = [9.5, 8.7, 9.2]
```

### Anotacoes de funcao

```aura
def somar(a: int, b: int) -> int {
  return a + b
}

def saudar(nome: str, saudacao: str = "Ola") -> str {
  return f"{saudacao}, {nome}!"
}
```

### Anotacoes de classe

```aura
class User {
  let nome: str = ""
  let idade: int = 0
  let email: str = ""

  def new(nome: str, idade: int) {
    self.nome = nome
    self.idade = idade
  }
}
```

---

## 12. Compatibilidade de Tipos

### Relacoes de subtipagem

- Um tipo e compativel com ele mesmo
- Uma subclasse e compativel com sua classe pai
- `any` e compativel com todos os tipos

```aura
class Animal { }
class Dog(Animal) { }

def processar(a: Animal) { }

let d = Dog()
processar(d)  // OK: Dog e subtipo de Animal
```

### Compatibilidade de tipos union

```aura
let x: int | str = 10      // OK: int e compativel com int | str
```

---

## 13. Mapeamento para Python

| Tipo Aura | Tipo Python | Notas |
|-----------|-------------|-------|
| `int` | `int` | Precisao arbitraria |
| `float` | `float` | 64 bits |
| `str` | `str` | Unicode |
| `bool` | `bool` | `true`/`false` |
| `none` | `NoneType` | Nulo |
| `[T]` | `list` | Array dinamico |
| `{K: V}` | `dict` | Hash map |
| `{T}` | `set` | Hash set |
| `(A, B)` | `tuple` | Tamanho fixo |
| `(A) -> B` | `Callable` | Tipo de funcao |
| `any` | `Any` | Op-out de verificacao |

Tipos do Aura sao uma **feature em tempo de compilacao**. Tipos sao apagados antes da geracao de Python, mas informacoes de tipo sao usadas para deteccao de erros, suporte a IDE e documentacao.
