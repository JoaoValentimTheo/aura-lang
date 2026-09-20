---
layout: default
title: "Especificação da Gramática de Aura"
nav_exclude: true
---

[English](grammar.md) · [Português](grammar.pt_BR.md)

# Especificação da Gramática de Aura

Este documento é a **única fonte de verdade** para a sintaxe concreta de Aura. O
tokenizer escrito à mão e o parser de descida recursiva em
`aura/parser/to_ast.py` implementam esta gramática; quando eles discordam, um dos
dois é um bug.

A gramática é escrita em EBNF:

```
rule      = definition ;
terminals = "literal" | /regex/ ;
?x?       = optional ;
x*        = zero or more ;
x+        = one or more ;
x | y     = alternation ;
( ... )   = grouping ;
```

> **Status:** sintaxe canônica para Aura `0.1.0a5` em diante. Grafias redundantes
> removidas em `0.1.0a5` estão listadas em [Appendix A](#appendix-a-removed-spellings);
> elas **não** fazem parte da gramática.

---

## 1. Estrutura léxica

### 1.1 Espaço em branco e comentários

Espaço em branco é insignificante exceto como separador de tokens. Quebras de
linha **não** são significativas; os statements são delimitados pela gramática,
com `;` opcional.

```
comment        = line_comment | block_comment ;
line_comment   = "//" , { ? any char except newline ? } ;
block_comment  = "/*" , { ? any char ? } , "*/" ;
```

Comentários de bloco podem atravessar linhas. `//` sempre inicia um comentário de
linha; Aura não tem operador de divisão inteira.

### 1.2 Identificadores

```
identifier     = ( letter | "_" ) , { letter | digit | "_" } ;
letter         = ? Unicode letter ? ;
digit          = "0" .. "9" ;
```

Letras Unicode são aceitas (correspondendo a identificadores Python).
Identificadores são comparados com distinção de maiúsculas/minúsculas e não são
normalizados.

### 1.3 Keywords

Palavras reservadas que não podem ser usadas como identificadores:

```
let  mut  const  def  class  trait  enum  type  module  import  from
if  else  unless  guard  while  until  for  in  loop  break  continue
return  throw  try  catch  finally  with  match  case  assert
async  await  yield  spawn  true  false
public  private  protected  static  volatile  abstract  export
self  super
```

Palavras contextuais (reservadas apenas em posições específicas, usáveis como
identificadores em outros lugares): `fn` foi **removido**; `step`, `as`, `is`,
`and`, `or`, `not`, `not in`, `is not`, `then` são operadores/contextuais.
`extends` e `implements` são reconhecidos como palavras contextuais depois de um
nome de classe ou trait: `extends` é a única keyword de herança, e `implements` é
reportado como um erro apontado dizendo para usar `extends` em vez disso.

### 1.4 Literais

```
int_literal    = decimal_int | hex_int | octal_int | binary_int ;
decimal_int    = digit , { digit | "_" } ;
hex_int        = "0" , ( "x" | "X" ) , hex_digit , { hex_digit | "_" } ;
octal_int      = "0" , ( "o" | "O" ) , octal_digit , { octal_digit | "_" } ;
binary_int     = "0" , ( "b" | "B" ) , ( "0" | "1" ) , { "0" | "1" | "_" } ;

float_literal  = digit , { digit } , "." , digit , { digit } , [ exponent ]
               | digit , { digit } , exponent
               | "." , digit , { digit } , [ exponent ] ;
exponent       = ( "e" | "E" ) , [ "+" | "-" ] , digit , { digit } ;

bool_literal   = "true" | "false" ;
none_literal   = "none" ;

string_literal = [ prefix ] , ( short_string | triple_string ) ;
prefix         = ( "r" | "b" | "f" | "rb" | "br" ) ;   // case-insensitive
short_string   = '"' , { escaped_char | ? any except " ? } , '"'
               | "'" , { escaped_char | ? any except ' ? } , "'" ;
triple_string  = '"""' , { ? any ? } , '"""'
               | "'''" , { ? any ? } , "'''" ;
escaped_char   = "\\" , ( "n" | "t" | "r" | "0" | "\\" | '"' | "'"
                        | "b" | "f" | "v" | "a"
                        | "x" , hex_digit , hex_digit
                        | "u" , hex_digit , hex_digit , hex_digit , hex_digit ) ;
```

Um `.` seguido de um dígito sempre lexa como float. Consequentemente `a.5` é
`a` seguido de `0.5`; use `a[5]` ou `a.b5` para a intenção de membro/índice.

### 1.5 Operadores e pontuação

A ordem de correspondência mais longa importa. Operadores de três caracteres são
correspondidos antes dos de dois caracteres, que são correspondidos antes dos de
um caractere.

```
punctuation = "(" | ")" | "[" | "]" | "{" | "}" | "," | ":" | ";" | "."
            | "?" | "@" ;

three_char  = "..<" | "..." | "??=" | "**=" | "<<=" | ">>=" ;
two_char    = "==" | "!=" | "<=" | ">=" | "->" | "=>" | "+=" | "-=" | "*="
            | "/=" | "%=" | "&=" | "|=" | "^=" | "<<" | ">>" | ".." | "??"
            | "?:" | "?." | "?[" | "|>" | "**" ;
one_char    = "+" | "-" | "*" | "/" | "%" | "<" | ">" | "=" | "&" | "|"
            | "^" | "~" ;
```

> `&&`, `||`, `!` foram **removidos**; use `and`, `or`, `not`.

---

## 2. Estrutura do programa

```
program        = { statement } , EOF ;

statement      = decorated_statement
               | declaration
               | control_statement
               | simple_statement ;

decorated_statement = { decorator } , declaration ;

decorator      = "@" , identifier , [ "(" , [ arg_list ] , ")" ] ;
```

Um **programa executado como arquivo de entrada** (`aura run`) deve declarar um
`def main()` de nível superior; seus parâmetros são nenhum ou um único `args`
(erro `E310` quando ausente, `E311` quando a assinatura está errada). O runtime
invoca `main` com os argumentos de linha de comando, então uma chamada `main()`
à direita não é escrita. Um arquivo usado puramente como **módulo** (`import`ado
por outro arquivo) não precisa de `main`.

---

## 3. Declarações

```
declaration    = var_decl
               | const_decl
               | function_decl
               | class_decl
               | trait_decl
               | enum_decl
               | type_alias
               | module_decl
               | import_stmt ;

modifiers      = { "public" | "private" | "protected" | "static" | "volatile" | "abstract" } ;
```

Modificadores aparecem **antes** da keyword de declaração
(`private let x = 1`, não `let private x`). `export` **não** é um modificador
geral: ele aparece apenas no início de um membro de `module` (veja
[Modules](#37-modules)).

No nível de membro de classe/trait uma **visibilidade é obrigatória**: todo
membro deve começar com exatamente um de `public`, `private` ou `protected`
(omitir é o erro `E307`).

### 3.1 Variáveis

```
var_decl       = modifiers , "let" , [ "mut" ] , pattern , [ ":" , type ] , [ "=" , expression ] , [ ";" ] ;
const_decl     = [ modifiers ] , "const" , identifier , [ ":" , type ] , "=" , expression , [ ";" ] ;

pattern        = tuple_pattern | list_pattern | identifier ;
tuple_pattern  = "(" , pattern , { "," , pattern } , [ "," ] , ")" ;
list_pattern   = "[" , pattern , { "," , pattern } , [ "," ] , "]" ;
```

`let` declara um binding imutável; `let mut` declara um mutável.
Desestruturação é escrita com padrões de tupla/lista e pode incluir um elemento
rest `...name`:

```
let (a, b) = (1, 2)
let [first, ...rest] = [1, 2, 3]
```

### 3.2 Funções

```
function_decl  = modifiers , [ "async" ] , "def" , identifier , [ type_params ]
                 , "(" , [ param_list ] , ")" , [ "->" , type ] , block ;

type_params    = "[" , identifier , { "," , identifier } , "]" ;

param_list     = param , { "," , param } ;
param          = "*" | "**" , identifier
               | identifier , [ ":" , type ] , [ "=" , expression ] ;
```

O corpo é sempre um bloco com chaves. `def` é a única keyword de função
(`fn` foi removido).

### 3.3 Classes

```
class_decl     = modifiers , "class" , identifier , [ type_params ]
                 , [ "extends" , dotted_name , { "," , dotted_name } ]
                 , [ "(" , [ header_field_list ] , ")" ]
                 , "{" , { class_member } , "}" ;

abstract_decl  = "abstract" , class_decl ;        // an `abstract class`

header_field_list = header_field , { "," , header_field } ;
header_field   = { "public" | "private" | "protected" | "mut" | "let" [ "mut" ] }
                 , identifier , ( ":" , type | "=" , expression )
                 , [ "=" , expression ] ;
                 // at least one of `: type` or `= default` is required
                 // a required field may not follow an optional one

dotted_name    = identifier , { "." , identifier } ;

class_member   = member_prefixes , ( method | nested_class | field | const_field ) ;

member_prefixes = { visibility | "static" | "volatile" | "abstract" | decorator } ;
visibility     = "public" | "private" | "protected" ;   // mandatory on every member
decorator      = "@" , dotted_name , [ "(" , [ arg_list ] , ")" ] ;

method         = [ "async" ] , "def" , identifier , [ type_params ] , "(" , [ param_list ] , ")" , [ "->" , type ] , block ;
abstract_method = "abstract" , "def" , identifier , [ type_params ] , "(" , [ param_list ] , ")" , [ "->" , type ] , [ ";" ] ;
                 // no body: `{ ... }` or `= expr` here is an error
nested_class   = class_decl ;
field          = ( "let" , [ "mut" ] | "mut" ) , identifier , [ ":" , type ] , [ "=" , expression ] , [ ";" ] ;
const_field    = "const" , identifier , [ ":" , type ] , "=" , expression , [ ";" ] ;
```

Modificadores de membro e decorators podem aparecer em qualquer ordem e em
linhas separadas: `@staticmethod public def f` e `public @staticmethod def f` são
equivalentes. Um decorator em um campo é rejeitado durante o parse (`E320`).

A herança usa apenas `extends`; não há lista de bases entre parênteses nem
`implements`. Como o header é introduzido por `(`, um nome nu ali nunca é uma
classe base. Uma base pontuada (`class Model extends django.db.models.Model`) ou
um nome ligado por um `import` marca uma base que o Python possui: Aura então
deixa o construtor e o protocolo de atributos para a metaclasse da própria
biblioteca, então `enum.Enum`, `pydantic.BaseModel` e modelos ORM funcionam sem
alteração. Declarar header fields junto com tal base volta a ativar o construtor
gerado.

Toda base deve resolver para uma classe/trait declarada ou uma raiz de exceção
embutida. Uma base que não existe é `E314`; uma base listada duas vezes ou um
ciclo de `extends` é `E315`. Traits, `abstract class`es e classes com métodos
abstratos não implementados não podem ser instanciados (`E316`). `super.m()` em
um método abstrato (sem corpo) é `E321`.

Uma `abstract class` não pode ser instanciada e pode declarar membros
`abstract def` — assinaturas sem corpo. Uma classe concreta deve implementar todo
método abstrato que herda (de uma `abstract class` ou de um trait), ou é `E309`.
Um `abstract def` pode aparecer apenas em uma `abstract class`; um em uma classe
concreta também é `E309`. Uma classe tem exatamente um estilo de construtor:
header fields geram o construtor, então um header mais um `new` manual é erro de
sintaxe.

A sobrescrita é implícita: não há modificador `override` (`override def` é erro
de sintaxe). Um trait também não usa `abstract` — um método de trait sem corpo já
é abstrato (`trait T { abstract def f() }` é erro de sintaxe).

Header fields tornam-se campos de instância, parâmetros de construtor e
acessores. A visibilidade padrão é `private`, e campos são imutáveis a menos que
declarados `mut`. Um campo `let` no corpo é igualmente imutável; `let mut`/`mut`
ativa um setter. Um campo cujo default é um descriptor (um valor cuja classe
define `__get__`/`__set__`) permanece na classe, então o protocolo de descriptor
do Python executa no acesso à instância.

### 3.4 Traits

```
trait_decl     = modifiers , "trait" , identifier , [ type_params ]
                 , [ "extends" , dotted_name , { "," , dotted_name } ]
                 , "{" , { trait_member } , "}" ;

trait_member   = member_modifiers , ( method_signature | method_with_body | field | const_field ) ;
method_signature = "def" , identifier , "(" , [ param_list ] , ")" , [ "->" , type ] ;   // no body
method_with_body = method ;
```

Traits compilam para classes base abstratas: métodos apenas com assinatura tornam-se
`@abstractmethod`. Um trait estende outros traits com `extends`, e uma classe
concreta que omite qualquer método abstrato herdado é rejeitada em tempo de
compilação (E309).

### 3.5 Enums

```
enum_decl      = modifiers , "enum" , identifier , "{" , enum_member , { "," , enum_member } , [ "," ] , "}" ;
enum_member    = identifier , [ "=" , expression ] ;
```

Membros são separados por vírgulas. Valores autonumeram a partir do valor
inteiro anterior quando omitidos.

### 3.6 Aliases de tipo

```
type_alias     = "type" , identifier , [ type_params ] , "=" , type ;
```

### 3.7 Módulos

```
module_decl    = "module" , dotted_name , "{" , { module_member } , "}" ;
module_member  = [ "export" ] , statement
               | "export" , name_list , [ "from" , string ] ;   // re-export
name_list      = identifier , { "," , identifier } ;
```

Um membro de módulo é privado ao arquivo declarante a menos que prefixado com
`export`. `export` deve preceder uma declaração nomeada (`def`, `class`, `trait`,
`enum`, `type`, `let`, `const` ou um `module` aninhado), ou pode introduzir um
**re-export** — um nome nu (ou nomes separados por vírgula) resolvido contra um
arquivo-fonte irmão. O `from "module"` opcional nomeia essa fonte explicitamente.
Um caminho `from` deve ser um nome pontuado simples (sem `/`, `\`, `..` ou
caminhos absolutos). Membros não exportados são mangled no Python gerado, então a
privacidade é aplicada em tempo de execução assim como pelo rule checker
(`E308`), e o estado do módulo não é atribuível de fora (`E303`). Um re-export
irresolúvel é `E313`; um `main` dentro de um corpo de módulo é `E312`.

### 3.8 Imports

```
import_stmt    = "import" , import_target , { "," , import_target }
               | "from" , dotted_name , "import" , ( "*" | import_item , { "," , import_item } ) ;

import_target  = dotted_name , [ "as" , identifier ] , [ "{" , import_item , { "," , import_item } , "}" ] ;
import_item    = identifier , [ "as" , identifier ] ;
```

---

## 4. Tipos

Tipos são parseados para documentação e checagem; são apagados em tempo de
execução.

```
type           = union_type ;
union_type     = primary_type , { "|" , primary_type } ;
primary_type   = dotted_name , [ type_args ]
               | "(" , type , { "," , type } , ")" , [ "->" , type ]   // tuple/function
               | "[" , type_args , "]"                                 // list shorthand
               | type , "?"                                             // optional
               | "{" , field_type , { "," , field_type } , "}" ;        // structural
type_args      = "[" , type , { "," , type } , "]" ;
field_type     = identifier , ":" , type ;
```

A **forma de colchete `[T]` é canônica** para parâmetros e argumentos de tipo; a
forma `<T>` foi removida.

---

## 5. Statements

```
control_statement = if_stmt | unless_stmt | guard_stmt | while_stmt
                  | until_stmt | for_stmt | loop_stmt | match_stmt
                  | try_stmt ;

simple_statement  = "return" , [ expression ] , [ ";" ]
                  | "throw" , expression , [ ";" ]
                  | "break" , [ identifier ] , [ ";" ]
                  | "continue" , [ identifier ] , [ ";" ]
                  | "assert" , expression , [ "," , expression ] , [ ";" ]
                  | "yield" , [ expression ] , [ ";" ]
                  | "spawn" , expression , [ ";" ]
                  | expression , [ ";" ] ;
```

### 5.1 Condicionais

```
if_stmt        = "if" , expression , block , [ "else" , ( if_stmt | block ) ] ;
unless_stmt    = "unless" , expression , block , [ "else" , ( if_stmt | block ) ] ;
guard_stmt     = "guard" , expression , "else" , block ;
```

### 5.2 Loops

```
while_stmt     = [ label ] , "while" , expression , block ;
until_stmt     = [ label ] , "until" , expression , block ;
for_stmt       = [ label ] , "for" , pattern , "in" , expression , [ "step" , expression ] , block ;
loop_stmt      = [ label ] , "loop" , block ;
label          = identifier , ":" ;
```

### 5.3 Blocos e with

```
block          = "{" , { statement } , "}" ;
with_stmt      = [ "async" ] , "with" , with_item , { "," , with_item } , block ;
with_item      = expression , [ "as" , identifier ] ;
```

`with` usa `__enter__`/`__exit__`; `async with` usa o protocolo de corrotina
`__aenter__`/`__aexit__` e é válido apenas dentro de um corpo de `async def`,
assim como `await`.

### 5.4 Match

```
match_stmt     = "match" , expression , "{" , { match_case } , "}" ;
match_case     = "case" , pattern , [ "if" , expression ] , ( block | "->" , expression ) ;
```

### 5.5 Try

```
try_stmt       = "try" , block , { catch_clause } , [ "finally" , block ] ;
catch_clause   = "catch" , [ type , [ "as" , identifier ] ] , block
               | "catch" , "as" , identifier , block ;
```

Um `try` exige pelo menos uma cláusula `catch` ou um bloco `finally`.

Um identificador isolado antes de `{` é sempre um **tipo**: `catch TypeError { }`
filtra por tipo, e `catch as e { }` vincula toda exceção. A antiga forma ambígua
`catch e { }` (um binding nu) é erro de parse.

---

## 6. Expressões

### 6.1 Precedência (menor para maior)

| Nível | Operadores | Associatividade |
|-------|-----------|---------------|
| 1 | `=`, `+=`, `-=`, `*=`, `/=`, `%=`, `**=`, `&=`, `&#124;=`, `^=`, `<<=`, `>>=`, `??=`, `&#124;>` | direita (pipe esquerda) |
| 2 | `? :` (ternário) | direita |
| 3 | `or` | esquerda |
| 4 | `and` | esquerda |
| 5 | `==`, `!=`, `<`, `>`, `<=`, `>=`, `in`, `not in`, `is`, `is not` | esquerda |
| 6 | `&#124;` (bit a bit) | esquerda |
| 7 | `^` | esquerda |
| 8 | `&` | esquerda |
| 9 | `<<`, `>>` | esquerda |
| 10 | `..`, `..<` (range) | nenhuma |
| 11 | `??`, `?:` (coalescência) | esquerda |
| 12 | `+`, `-` | esquerda |
| 13 | `*`, `/`, `%`, `as` (cast) | esquerda |
| 14 | `**` | direita |
| 15 | unário `-`, `+`, `~`, `not`, `await`, `...` (spread) | prefixo |
| 16 | call, index, slice, member, safe-nav, struct-init | postfix |

Comparação (`==`, `<`, `in`, `is`, …) é **mais frouxa** que os operadores bit a
bit e os shifts, exatamente como em Python: `1 & 2 == 2` é `(1 & 2) == 2` e
`1 < 2 | 3` é `1 < (2 | 3)`. Comparação **não** encadeia como o `a < b < c` do
Python; use `a < b and b < c`.

`yield` é uma forma de prefixo em nível de statement, não parte de
`expression`: o parser o reconhece antes do loop Pratt e parseia seu operando na
precedência de expressão mais baixa, então `yield x + 1` produz `x + 1` e
`yield a, b` produz a tupla `(a, b)`.

### 6.2 Gramática de expressão

```
expression     = ternary ;
ternary        = or_expr , [ "?" , expression , ":" , expression ] ;
or_expr        = and_expr , { "or" , and_expr } ;
and_expr       = bitor_expr , { "and" , bitor_expr } ;
bitor_expr     = bitxor_expr , { "|" , bitxor_expr } ;
bitxor_expr    = bitand_expr , { "^" , bitand_expr } ;
bitand_expr    = shift_expr , { "&" , shift_expr } ;
shift_expr     = comparison , { ( "<<" | ">>" ) , comparison } ;
comparison     = range_expr , { comp_op , range_expr } ;
comp_op        = "==" | "!=" | "<" | ">" | "<=" | ">=" | "in" | "not in" | "is" | "is not" ;
range_expr     = coalesce , [ ( ".." | "..<" ) , coalesce , [ "step" , coalesce ] ] ;
coalesce       = additive , { ( "??" | "?:" ) , additive } ;
additive       = multiplicative , { ( "+" | "-" ) , multiplicative } ;
multiplicative = power_expr , { ( "*" | "/" | "%" ) , power_expr | "as" , type } ;
power_expr     = unary , [ "**" , unary ] ;
unary          = ( "-" | "+" | "~" | "not" | "await" | "..." ) , unary
               | postfix ;
postfix        = primary , { postfix_op } ;
postfix_op     = "(" , [ arg_list ] , ")"
               | "[" , slice , "]"
               | "?[" , slice , "]"
               | "." , identifier
               | "?." , identifier
               | "{" , struct_fields , "}" ;        // struct init (capitalized type)
```

### 6.3 Expressões primárias

```
primary        = literal
               | identifier
               | list_literal | set_literal | dict_literal | tuple_literal
               | lambda
               | comprehension
               | "(" , expression , ")"
               | if_expression | match_expression | try_expression
               | block_expression ;

literal        = int_literal | float_literal | bool_literal | none_literal | string_literal | f_string ;
```

### 6.4 Lambda

```
lambda         = ( identifier | "(" , [ param_list ] , ")" ) , "=>" , ( expression | block ) ;
```

### 6.5 Coleções e comprehensions

```
list_literal   = "[" , [ expression , { "," , expression } ] , "]" ;
set_literal    = "{" , expression , { "," , expression } , "}" ;
dict_literal   = "{" , [ dict_entry , { "," , dict_entry } ] , "}" ;
tuple_literal  = "(" , expression , "," , [ expression , { "," , expression } ] , ")" ;
dict_entry     = ( expression | identifier ) , ":" , expression ;

comprehension  = list_comp | set_comp | dict_comp | generator_expr ;
list_comp      = "[" , expression , "for" , pattern , "in" , expression , { "for" | "if" , ... } , "]" ;
set_comp       = "{" , expression , "for" , pattern , "in" , expression , { ... } , "}" ;
dict_comp      = "{" , expression , ":" , expression , "for" , pattern , "in" , expression , { ... } , "}" ;
generator_expr = "(" , expression , "for" , pattern , "in" , expression , { ... } , ")" ;
```

### 6.6 Struct init

```
struct_init    = type_name , "{" , [ struct_field , { "," , struct_field } ] , "}" ;
struct_field   = identifier , ":" , expression ;
```

Um `{` após um identificador **começando com maiúscula** é um struct init; caso
contrário é um bloco ou literal dict/set. Struct init é açúcar para
`TypeName(**{ ... })` (transpila para uma chamada de construtor).

### 6.7 Chamadas e spreads

```
arg_list       = argument , { "," , argument } ;
argument       = expression
               | identifier , "=" , expression        // keyword
               | "*" , expression                     // positional spread
               | "**" , expression                    // keyword spread
               | "..." , expression ;                 // adaptive spread
```

### 6.8 F-strings

```
f_string       = ( "f" | "F" ) , ( '"' | "'" | '"""' | "'''" ) , { f_part } , quote ;
f_part         = text | "{" , expression , [ ":" , format_spec ] , "}" ;
```

---

## 7. Formatação padrão

`aura format` normaliza para estas regras (veja `aura/tools/formatter.py`):

- Indentação de dois espaços por nível de bloco.
- Chaves ficam na mesma linha da construção; o `}` de fechamento fica sozinho.
- Um espaço ao redor de operadores binários; nenhum dentro de comentários `/`.
- `->` para tipos de retorno e setas de match-case.
- Espaço em branco à direita removido; sequências de espaços colapsadas.
- O conteúdo de literais de string nunca é reescrito.

---

## Appendix A: Removed spellings

As seguintes eram aceitas antes de `0.1.0a5` e agora são erros. Cada uma mapeia
para sua substituição canônica:

| Removed | Canonical | Notes |
|---------|-----------|-------|
| `fn` | `def` | single function keyword |
| `init` | `new` | one constructor name |
| `!x` | `not x` | one negation operator |
| `&&` | `and` | |
| `&#124;&#124;` | `or` | |
| `null` | `none` | one null literal |
| `volatily` | `volatile` | typo alias |
| `fn foo<T>()` / `class Foo<T>` | `foo[T]` / `Foo[T]` | brackets only |
| `case x => e` | `case x -> e` | `->` only |
| `let private x` | `private let x` | modifiers prefix the declaration |
| `catch Type` (ambiguous) | `catch Type as e` | explicit binding |

---

## Appendix B: Diagnostics

`aura check` reporta violações de regra com códigos estáveis (veja
`aura/transpiler/errors.py`). Violações de mutabilidade (reatribuir um `let` ou
`const`) são erros, não warnings — aplicadas identicamente por `aura run`,
`aura check` e o REPL.
