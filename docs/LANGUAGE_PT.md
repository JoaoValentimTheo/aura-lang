# Referencia da Linguagem Aura

Referencia completa de sintaxe da linguagem de programacao Aura. Aura transpila para Python.

---

## Sumario

1. [Estrutura do Programa](#1-estrutura-do-programa)
2. [Elementos Lexicos](#2-elementos-lexicos)
3. [Variaveis e Constantes](#3-variaveis-e-constantes)
4. [Tipos](#4-tipos)
5. [Funcoes](#5-funcoes)
6. [Lambdas](#6-lambdas)
7. [Classes](#7-classes)
8. [Traits](#8-traits)
9. [Aliases de Tipo e Modulos](#9-aliases-de-tipo-e-modulos)
10. [Controle de Fluxo](#10-controle-de-fluxo)
11. [Loops](#11-loops)
12. [Pattern Matching](#12-pattern-matching)
13. [Expressoes e Operadores](#13-expressoes-e-operadores)
14. [Colecoes](#14-colecoes)
15. [Tratamento de Erros](#15-tratamento-de-erros)
16. [Imports](#16-imports)
17. [Macros e Decorators](#17-macros-e-decorators)
18. [Comentarios](#18-comentarios)

---

## 1. Estrutura do Programa

Um programa Aura e uma sequencia de declaracoes, instrucoes e imports.

Um **arquivo executavel** — aquele rodado com `aura run` — precisa declarar uma
`main` de topo. O runtime invoca a `main` por voce: nunca escreva uma chamada
`main()` no final.

```aura
import stdlib.math { sqrt, PI }

const SAUDACAO = "Ola"

def saudar(nome) -> str {
  return SAUDACAO + " " + nome
}

def main() {
  print(saudar("mundo"))
}
```

A `main` nao recebe parametros, ou recebe um unico parametro `args` com os
argumentos de linha de comando:

```aura
def main(args: [string]) {
  print(f"{args.length()} argumento(s)")
}
```

```bash
$ aura run app.aura alfa beta
2 argumento(s)
```

A `main` pode retornar um `int` para definir o codigo de saida do processo, e
pode ser `async`; o runtime a aguarda dentro de um event loop:

```aura
async def main() {
  print(await buscar_dados("https://example.com"))
}
```

Um arquivo **importado como modulo** nao precisa de `main`: qualquer arquivo
Aura pode expor funcoes, classes e constantes a outro via `import`. Omitir a
`main` num arquivo executado diretamente e erro de compilacao (`E310`), e uma
`main` com outra assinatura e rejeitada (`E311`).

---

## 2. Elementos Lexicos

### Identificadores

```
identificador ::= [a-zA-Z_] [a-zA-Z0-9_]*
```

- Deve comecar com uma letra ou sublinhado
- Diferencia maiusculas de minusculas
- Convencoes: `snake_case` para variaveis/funcoes, `PascalCase` para classes

### Palavras-chave

```
async      break      case       catch      class      const
continue   def        else       finally    for        from
guard      if         import     in         is         let
loop       match      module     mut        none       null
return     self       static     super      trait      true
false      try        type       unless     until      while
with       assert
```

### Literais

```aura
// Inteiros
42
1_000_000
0xFF        // hexadecimal
0o755       // octal
0b1010      // binario

// Floats
3.14
2.5e10
1.5e-5

// Strings
"hello"
'mundo'
f"Valor: {x}"
"""
String
multilinha
"""

// Booleans
true
false

// Nulo
none
null
```

---

## 3. Variaveis e Constantes

### Variaveis imutaveis

```aura
let nome = "Alice"
let idade = 30
```

### Variaveis mutaveis

```aura
let mut contador = 0
contador += 1
```

### Constantes

```aura
const PI = 3.14159
const TAMANHO_MAX: int = 100
```

### Anotacoes de tipo

```aura
let nome: str = "Alice"
let idade: int = 30
let pi: float = 3.14159
let ativo: bool = true
let itens: [int] = [1, 2, 3]
let usuario: {nome: str, idade: int} = {nome: "Alice", idade: 30}
```

### Atribuicao multipla

```aura
let mut a, b = 0, 1
let x, y, z = 1, 2, 3
```

### Destructuring

```aura
let [primeiro, segundo, ...resto] = [1, 2, 3, 4, 5]
// primeiro = 1, segundo = 2, resto = [3, 4, 5]
```

---

## 4. Tipos

### Tipos primitivos

| Tipo    | Descricao                  |
|---------|----------------------------|
| `int`   | Numeros inteiros           |
| `float` | Numeros de ponto flutuante |
| `str`   | Strings Unicode            |
| `bool`  | `true` ou `false`          |
| `bytes` | Sequencias de bytes        |
| `none`  | Valor nulo                 |

### Tipos de colecao

```aura
[int]                          // Lista de inteiros
{str: int}                     // Dicionario
{int}                          // Conjunto
```

### Tipos genericos

Generics usam colchetes:

```aura
class Caixa[T] {
  private let valor: T

  public def new(valor: T) {
    self.valor = valor
  }

  public def obter() -> T {
    return self.valor
  }
}

def mapear[T, R](fn: (T) -> R, itens: [T]) -> [R] {
  return [fn(item) for item in itens]
}
```

### Tipos union

```aura
let valor: int | str = 42
int | float | none
```

### Tipos de funcao

```aura
let manipulador: (int, int) -> int = somar
```

### Tipos opcionais

```aura
let nome: str? = obter_nome()
```

### Tipos estruturais

```aura
let ponto: {x: float, y: float} = {x: 1.0, y: 2.0}
```

---

## 5. Funcoes

### Funcao basica

```aura
def saudar(nome) -> str {
  return "Ola, " + nome + "!"
}
```

### Funcao com anotacoes de tipo

```aura
def somar(a: int, b: int) -> int {
  return a + b
}
```

### Corpo de expressao

```aura
def quadrado(x: int) -> int = x * x
def dobrar(x) = x * 2
```

### Parametros com valor padrao

```aura
def saudar(nome, saudacao = "Ola") -> str {
  return saudacao + ", " + nome + "!"
}

saudar("Alice")            // "Ola, Alice!"
saudar("Bob", "Oi")        // "Oi, Bob!"
```

### Argumentos nomeados

```aura
def criar_usuario(nome, idade, email) {
  return {nome: nome, idade: idade, email: email}
}

let usuario = criar_usuario(nome: "Alice", idade: 30, email: "alice@exemplo.com")
```

### Parametros variaveis

```aura
def somar_todos(*numeros) -> int {
  return sum(numeros)
}

def registrar(nivel, **contexto) {
  print(f"[{nivel}] {contexto}")
}
```

### Funcoes recursivas

```aura
def fatorial(n) -> int {
  if n <= 1 { return 1 }
  return n * fatorial(n - 1)
}
```

### Multiplos valores de retorno

```aura
def trocar(a, b) {
  return b, a
}

let x, y = trocar(1, 2)
```

---

## 6. Lambdas

### Funcoes seta

```aura
// Um parametro, sem parenteses
let dobrar = x => x * 2
print(dobrar(5))  // 10

// Um parametro, com parenteses
let quadrado = (x) => x * x
print(quadrado(4))  // 16

// Multiplos parametros
let somar = (a, b) => a + b
print(somar(3, 4))  // 7

// Sem parametros
let obter_resposta = () => 42
print(obter_resposta())  // 42
```

### Corpo de bloco

```aura
let calcular = (x) => {
  let dobrado = x * 2
  return dobrado + 1
}
print(calcular(10))  // 21
```

### Closures

```aura
let fazer_somador = (n) => (x) => x + n
let somar10 = fazer_somador(10)
print(somar10(5))  // 15
```

### Lambdas com map/filter

```aura
let numeros = [1, 2, 3, 4, 5]
let dobrados = map(numeros, (x) => x * 2)
let pares = filter(numeros, (x) => x % 2 == 0)
```

---

## 7. Classes

### Classe basica

```aura
class Ponto {
  public let x: int = 0
  public let y: int = 0

  public def new(x: int, y: int) {
    self.x = x
    self.y = y
  }

  public def distancia() -> float {
    return (self.x ** 2 + self.y ** 2) ** 0.5
  }
}

let p = Ponto(3, 4)
print(p.distancia())  // 5.0
```

> **Nota:** `def new(...)` corresponde ao `__init__` do Python. Instancie com `Ponto(args)`, nao `Ponto.new(args)`.

### Heranca

```aura
class Animal {
  protected let nome: str = ""

  public def new(nome: str) {
    self.nome = nome
  }

  public def falar() -> str {
    return "..."
  }
}

class Cachorro(Animal) {
  public def falar() -> str {
    return self.nome + " diz au au"
  }
}

let d = Cachorro("Rex")
print(d.falar())  // Rex diz au au
```

O transpiler insere automaticamente `super().__init__()` no construtor quando existe uma classe base.

### @property

```aura
class Retangulo {
  private let w: int = 0
  private let h: int = 0

  public def new(w: int, h: int) {
    self.w = w
    self.h = h
  }

  public @property
  def area() -> int {
    return self.w * self.h
  }
}

let r = Retangulo(4, 5)
print(r.area)  // 20 (acessado como propriedade, sem parenteses)
```

### @staticmethod

```aura
class UtilMat {
  public @staticmethod
  def max(a, b) -> int {
    if a > b { return a }
    return b
  }
}

print(UtilMat.max(3, 9))  // 9
```

### @classmethod

```aura
class Fabrica {
  public let tipo: str = ""

  public def new(tipo: str) {
    self.tipo = tipo
  }

  public @classmethod
  def criar(cls, tipo) {
    return cls(tipo)
  }
}

let f = Fabrica.criar("custom")
print(f.tipo)  // custom
```

---

## 8. Traits

```aura
trait Desenhavel {
  public def desenhar() -> void
  public def obter_limites() -> float
}

class Circulo implements Desenhavel {
  private let raio: float = 0.0

  public def desenhar() -> void {
    print(f"Desenhando circulo com raio {self.raio}")
  }

  public def obter_limites() -> float {
    return self.raio * 2
  }
}
```

Um trait transpila para uma classe base, e `implements` vira heranca.
Multiplos traits podem ser listados: `class C implements A, B`.

Um metodo declarado sem corpo e abstrato: o trait o compila para um
`@abstractmethod`, e uma classe concreta que nao implementa todos os metodos
abstratos herdados e rejeitada em tempo de compilacao (`E309`):

```aura
trait Forma { public def area() -> float }

class Quadrado implements Forma {
  public let lado: float = 2.0
  public def area() -> float { return self.lado * self.lado }
}

// class Ruim implements Forma { }   // E309: precisa implementar 'area'
```

Traits tambem podem estender outros traits, com `trait Alto(Falante)` ou
`trait Alto implements Falante`. Metodos abstratos sao herdados
transitivamente:

```aura
trait Falante { public def falar() -> str }
trait Alto implements Falante { public def gritar() -> str }

class Pessoa implements Alto {
  public def falar() -> str { return "oi" }
  public def gritar() -> str { return "OI" }
}
```

### Visibilidade

```aura
class Conta {
  private let saldo: float = 0.0
  protected let id: str = ""
  public let nome: str = ""
}
```

Todo membro de classe/trait (campo, metodo ou classe aninhada) **precisa**
declarar a visibilidade explicitamente: `public`, `private` ou `protected`.
Omitir e erro de compilacao (`E307`).

Os modificadores vem **antes** de `let`/`def`/`class`.

A aplicacao acontece em **tempo de compilacao e em tempo de execucao**:

* O rule checker rejeita o acesso a um membro nao-publico de fora da classe
  (`E308`), resolvendo `self`/`cls` e instancias simples `let x = Classe(...)`.
* O transpilador emite nomes com mangling ciente do dono, entao a restricao
  tambem vale em runtime mesmo quando o checker nao consegue provar.

| Modificador | Nome em runtime | Acessivel de |
|-------------|-----------------|--------------|
| `public` | `nome` | qualquer lugar |
| `protected` | `_nome` | a classe que declara e suas subclasses |
| `private` | `_<ClasseDona>__nome` | apenas a classe que declara |

Um membro `private` **nao** e visivel em uma subclasse; a subclasse so o
alcanca por um metodo `public`/`protected` ou pelo acessor gerado.

Para todo campo nao-publico sao gerados automaticamente `get_<nome>()` e
`set_<nome>(valor)`, a menos que a classe ja defina um metodo com esse nome.

No escopo de modulo ou local, `private`/`protected` sao apenas metadados de
compilacao; os nomes **nao** sao modificados.

---

## 9. Aliases de Tipo e Modulos

### Aliases de tipo

```aura
type UserId = int
type Ponto = {x: float, y: float}
```

Aliases de tipo sao apagados em tempo de compilacao, mas um nome em runtime
ainda e emitido (`UserId = int`, tipos estruturais viram `dict`).

### Modulos

```aura
module MinhaLib {
  export def funcao_publica() {
    return 42
  }

  def funcao_privada() {
    // nao exportada
  }

  export const VERSAO = "1.0.0"
}

print(MinhaLib.funcao_publica())
print(MinhaLib.VERSAO)
```

Um modulo transpila para uma classe com namespace cujas funcoes sao
estaticas. `export` e aceito mas nao tem equivalente em Python, entao e
ignorado.

---

## 10. Controle de Fluxo

### if/else

```aura
if temperatura > 100 {
  print("Fervendo!")
} else if temperatura > 50 {
  print("Quente")
} else {
  print("Frio")
}
```

### unless (if invertido)

```aura
unless autenticado {
  redirecionar("/login")
}
```

### guard

```aura
def processar(dados) {
  guard dados != null else {
    print("Sem dados")
    return
  }
  // dados e garantido nao-nulo aqui
  print(dados)
}
```

### switch/match

```aura
match status {
  case 0 { print("inativo") }
  case 1 { print("ativo") }
  case n if n > 100 { print("overflow") }
  case _ { print("desconhecido") }
}
```

---

## 11. Loops

### for

```aura
for i in range(10) {
  print(i)
}

// Com step dentro do range
for i in range(0, 20, 2) {
  print(i)
}

// Formas com a palavra-chave `step`
for i in range(0, 10) step 2 { print(i) }
for i in 0..10 step 2 { print(i) }

// Iterar sobre lista
for item in itens {
  print(item)
}
```

### while

```aura
let mut contador = 0
while contador < 5 {
  contador += 1
}
```

### until (while invertido)

```aura
until pronto {
  esperar(100)
}
```

### loop (infinito)

```aura
loop {
  let entrada = ler_entrada()
  if entrada == "sair" { break }
}
```

### break e continue

```aura
for i in range(100) {
  if i == 5 { break }
  if i % 2 == 0 { continue }
  print(i)
}
```

### Loops com rótulos

Use rótulos para break/continue em loops aninhados:

```aura
externo: for i in range(10) {
  interno: for j in range(10) {
    if i * j > 20 {
      break externo
    }
    print(f"{i}, {j}")
  }
}
```

Rótulos funcionam com `loop`, `for` e `while`:

```aura
tentar: loop {
  let entrada = ler_entrada()
  if entrada == "sair" { break tentar }
  processar(entrada)
}
```

---

## 12. Pattern Matching

### Matching basico

```aura
match comando {
  case "sair" { print("Adeus!") }
  case "ajuda" { print("Ajuda!") }
  case _ { print("Comando desconhecido") }
}
```

### Condicoes de guarda

```aura
match valor {
  case 0 { print("zero") }
  case 1 { print("um") }
  case n if n > 100 { print("grande") }
  case n if n > 0 { print("positivo") }
  case _ { print("outro") }
}
```

### Padroes de destructuring

```aura
// Padroes de lista
match numeros {
  case [] { print("Vazia") }
  case [x] { print(f"Unica: {x}") }
  case [primeiro, *resto] { print(f"Primeiro: {primeiro}") }
}

// Binding de variaveis
match resposta {
  case "ok" { processar() }
  case erro if erro.comeca_com("erro") { lidar_com_erro(erro) }
  case _ { registrar(resposta) }
}
```

---

## 13. Expressoes e Operadores

### Precedencia de operadores (maior para menor)

| Prec | Operador | Descricao |
|------|----------|-----------|
| 1 | `**` | Exponenciacao |
| 2 | `+x`, `-x`, `~x` | Mais unario, menos, NOT bit a bit |
| 3 | `*`, `/`, `%` | Multiplicacao, divisao, modulo |
| 4 | `+`, `-` | Soma, subtracao |
| 5 | `<<`, `>>` | Shifts bit a bit |
| 6 | `&` | AND bit a bit |
| 7 | `^` | XOR bit a bit |
| 8 | `\|` | OR bit a bit |
| 9 | `<`, `>`, `<=`, `>=` | Comparacoes |
| 10 | `==`, `!=`, `is`, `in` | Igualdade, identidade, pertence |
| 11 | `not` | NOT logico |
| 12 | `and` | AND logico |
| 13 | `or` | OR logico |
| 14 | `?:` | Elvis operator |
| 15 | `??` | Coalescencia nula |
| 16 | `? :` | Ternario condicional |
| 17 | `\|>` | Pipe |
| 18 | `=`, `+=`, etc. | Atribuicao |

### Ternario

```aura
let resultado = condicao ? "sim" : "nao"
```

### Elvis operator

```aura
let valor = potencialmente_nulo ?: valor_padrao
```

### Coalescencia nula

```aura
let nome_display = user?.nome ?? "Anonimo"
nome ??= "Nome Padrao"
```

### Operador pipe

```aura
let resultado = [1, 2, 3, 4, 5]
  |> filter((x) => x > 2)
  |> map((x) => x * 10)
  |> reduce((a, b) => a + b, 0)
```

### Operadores de range

```aura
1..10          // inclusivo: 1, 2, ..., 10
0..<100        // exclusivo: 0, 1, ..., 99
1..            // infinito
0..100 step 5  // 0, 5, 10, ..., 100
```

### Navegacao segura contra nulos

```aura
let cidade = user?.endereco?.cidade
let item = lista?[indice]
```

### Operador spread

```aura
let combinado = [*lista1, *lista2, extra]
let mesclado = {**padroes, **sobrescricoes}
```

### Compreensao de listas

```aura
let quadrados = [x * x for x in range(10)]
let pares = [x for x in range(20) if x % 2 == 0]
let matriz = [[i * j for j in range(3)] for i in range(3)]
```

### Compreensao de dicionarios

```aura
let tamanhos = {palavra: len(palavra) for palavra in palavras}
```

---

## 14. Colecoes

### Listas

```aura
let numeros = [1, 2, 3, 4, 5]
let mista = [1, "dois", 3.0, true]
let vazia = []
print(numeros[0])  // 1
print(len(numeros))  // 5
```

### Dicionarios

```aura
let usuario = {nome: "Alice", idade: 30, ativo: true}
print(usuario.nome)       // Alice
print(usuario["idade"])   // 30

let config = {"tamanho-max": 100, "timeout": 30}
```

### Conjuntos

```aura
let unico = {1, 2, 3, 2, 1}  // {1, 2, 3}
print(len(unico))  // 3
```

### Tuples

```aura
let vazio = ()
let unitario = (42,)
let par = (1, "ola")
```

---

## 15. Tratamento de Erros

### try/catch/finally

```aura
try {
  let resultado = operacao_arriscada()
} catch e {
  print("Erro: " + e)
}

// Excecoes tipadas
try {
  parse("invalido")
} catch SyntaxError {
  print("Erro de sintaxe encontrado")
}

// Forma completa
try {
  let arquivo = open("dados.txt")
  processar(arquivo)
} catch IOError {
  print("Arquivo nao encontrado")
} finally {
  print("Limpeza completa")
}
```

### Lançar excecoes

```aura
def validar(idade) {
  guard idade >= 0 else {
    throw ValueError("idade nao pode ser negativa")
  }
  return true
}
```

### Try como expressão

`try` pode ser usado como expressão que retorna um valor:

```aura
let resultado = try {
  parse_int(entrada)
} catch e {
  0
}

// Com finally
let dados = try {
  ler_arquivo(caminho)
} catch e {
  "padrao"
} finally {
  limpar()
}
```

---

## 16. Imports

Aura suporta tres formas de import, todas da biblioteca padrao:

```aura
// 1. Importar um modulo (acesse membros pelo nome do modulo)
import stdlib.math
print(stdlib.math.sqrt(16))  // 4.0

// 2. Importar um modulo com alias
import stdlib.math as m
print(m.sqrt(25))  // 5.0

// 3. Importar nomes especificos (com alias opcional)
from stdlib.math import sqrt, PI
from stdlib.math import sqrt as root
print(sqrt(36))  // 6.0

// Forma com chaves: equivalente a `from ... import ...`
import stdlib.math { sqrt, PI }
print(PI)  // 3.141592653589793
```

A forma com chaves (`import modulo { a, b }`) e um atalho que compila para
`from modulo import a, b`. Ela nao pode ser combinada com `as`; use uma das
formas acima. Nao existe separador de namespace `::` nem `*` na forma com
chaves — use `from stdlib.modulo import *` se precisar de wildcard.

---

## 17. Macros e Decorators

Decorators built-in estao disponiveis como macros em tempo de execucao:

```aura
// @debug - imprime entrada/saida com argumentos e valor de retorno
@debug
def calcular(x, y) -> int {
  return x + y
}

// @timeit - mede tempo de execucao
@timeit
def funcao_lenta() {
  // ... trabalho ...
}

// @memoize - armazena resultados em cache
@memoize
def fibonacci(n) -> int {
  if n < 2 { return n }
  return fibonacci(n - 1) + fibonacci(n - 2)
}

// @cache(maxsize=N) - cache com LRU
@cache(maxsize=256)
def busca_custa(chave) {
  return computar(chave)
}

// @must_return - assegura que a funcao retorna um valor
@must_return
def computar(x) -> int {
  return x * 2
}

// @deprecated(mensagem) - avisa ao usar
@deprecated("Use nova_funcao ao inves")
def funcao_antiga() {
  // ...
}
```

---

## 18. Comentarios

```aura
// Comentario de uma linha

/*
   Comentario de multiplas linhas
   abrange multiplas linhas
*/
```

> **Nota:** `//` sempre inicia um comentario de linha. Aura nao possui operador
> de divisao inteira (floor) de proposito; use casting inteiro sobre uma
> divisao float: `let q = int(total / count)`.

---

## Metodos de Strings

Strings suportam chamadas de metodo diretas:

```aura
let s = "Ola, Mundo"

s.to_upper()         // "OLA, MUNDO"
s.to_lower()         // "ola, mundo"
s.trim()             // remove espacos
s.trim_left()        // remove espacos iniciais
s.trim_right()       // remove espacos finais
s.starts_with("Ola") // true
s.ends_with("ndo")   // true
s.contains("ola")    // true
s.index_of("ola")    // 1
s.slice(0, 4)        // "Ola"
s.length()           // 12
s.is_alpha()         // false
s.is_digit()         // false
```

---

## Dicas de Performance

- Use `@memoize` ou `@cache` para funcoes puras custosas
- Prefira `range()` a criar listas grandes
- Use compreensoes de listas ao inves de map/filter para casos simples
- Use o operador `|>` pipe para transformacoes de dados legiveis
- Evite hierarquias profundas de classes; prefira composicao

---

## 19. Enums

Enums definem um conjunto de constantes nomeadas:

```aura
enum Direcao {
  Norte
  Sul
  Leste
  Oeste
}

let sentido = Direcao.Norte
print(sentido)  // Direcao.Norte
```

Enums com valores:

```aura
enum Status {
  Pendente = "pendente"
  Ativo = "ativo"
  Desativado = "desativado"
}

let atual = Status.Ativo
```

Enums com pattern matching:

```aura
enum Forma {
  Circulo(raio: float)
  Retangulo(largura: float, altura: float)
}

def area(forma) -> float {
  match forma {
    case Forma.Circulo(r) { return 3.14 * r * r }
    case Forma.Retangulo(l, a) { return l * a }
  }
}
```

---

## 20. Async / Await

Aura suporta funcoes assincronas e expressoes await:

```aura
async def buscar_dados(url) -> str {
  let resposta = await http_get(url)
  return resposta.corpo
}

async def principal() {
  let dados = await buscar_dados("https://exemplo.com")
  print(dados)
}
```

Executar codigo assincrono com `aura run` automaticamente envolve em um event loop.

Awaits concorrentes:

```aura
async def paralelo() {
  let a = await buscar("url1")
  let b = await buscar("url2")
  return [a, b]
}
```

---

## 21. Biblioteca Padrao

Aura inclui uma biblioteca padrao de modulos uteis.

### stdlib.json

Analise e serializacao JSON:

```aura
import stdlib.json

let dados = stdlib.json.loads('{"nome": "Aura"}')
let texto = stdlib.json.dumps(dados, indent=2)
let bonito = stdlib.json.pretty(dados)
let valido = stdlib.json.is_valid('{"ok": true}')
```

### stdlib.time

Utilitarios de data e hora:

```aura
import stdlib.time

let t = stdlib.time.now()        // timestamp (segundos)
let ms = stdlib.time.now_ms()    // timestamp (milissegundos)
let iso = stdlib.time.iso()      // string ISO 8601
stdlib.time.sleep(0.5)           // dormir 500ms
let decorrido = stdlib.time.elapsed(inicio)
```

### stdlib.io

Operacoes de arquivo e diretorio:

```aura
import stdlib.io

stdlib.io.write("saida.txt", "ola")
let conteudo = stdlib.io.read("saida.txt")
let linhas = stdlib.io.read_lines("dados.csv")

if stdlib.io.exists("config.json") {
  let cfg = stdlib.io.read("config.json")
}

stdlib.io.mkdir("saida")
stdlib.io.rm("temp.txt")
let arquivos = stdlib.io.ls(".")
```

### stdlib.math

Funcoes matematicas (30+):

```aura
import stdlib.math { sqrt, PI, sin, cos, floor, ceil }

let r = sqrt(16.0)     // 4.0
let c = cos(PI)         // -1.0
let n = floor(3.7)      // 3
```

### stdlib.string

Manipulacao de strings (40+):

```aura
import stdlib.string

let maiuscula = stdlib.string.upper("ola")   // "OLA"
let partes = stdlib.string.split("a,b,c", ",")  // ["a","b","c"]
```

### stdlib.collections

Utilitarios de listas, dicts e conjuntos:

```aura
import stdlib.collections

let mapeado = stdlib.collections.list_map([1,2,3], (x) => x * 2)
```

### stdlib.itertools

Utilitarios de iteracao:

```aura
import stdlib.itertools

let pares = stdlib.itertools.combinations([1,2,3], 2)
```
