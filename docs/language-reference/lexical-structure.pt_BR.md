---
layout: default
title: "Estrutura Léxica"
nav_exclude: true
---

[English](lexical-structure.md) · [Português](lexical-structure.pt_BR.md)

# Estrutura Léxica

**Status:** Stable · **Evidência:** `aura/parser/to_ast.py` (`Tokenizer`,
`to_ast.py:85-444`), `docs/language-reference/grammar.md` §1, `tests/test_tokenizer.py`

O tokenizer do Aura é **escrito à mão e de passada única**. Ele lê o código-fonte
UTF-8 da esquerda para a direita, ignora espaços em branco e comentários, e emite
uma lista plana de `Token(type, value, line, column)` (`to_ast.py:49-57`)
terminando em `EOF`. Não é gerado por ferramenta nem baseado em regex. Todo token
é um de: `IDENT`, `INT`, `FLOAT`, `STRING`, `RAWSTRING`, `FSTRING`, `OP`, `EOF`.

> **Nota de nível:** este documento descreve a *gramática léxica da linguagem* —
> quais sequências de caracteres formam tokens. O parser de descida recursiva
> que consome os tokens é descrito em [grammar.md](grammar.md); as grafias
> concretas de cada construção estão em [syntax.md](syntax.md).

---

## 1. Caracteres da fonte

- O arquivo é lido como texto **UTF-8**, um caractere por vez
  (`to_ast.py:162-175`).
- **Identificadores** começam com uma letra ASCII ou `_`, ou qualquer caractere
  Unicode que `str.isidentifier()` do Python aceite sozinho (`to_ast.py:29-33`),
  e continuam com alfanuméricos ASCII/`_` ou um caractere que seja continuação
  válida (`to_ast.py:36-43`). Não há escape de identificador nem restrição
  ASCII; identificadores são **comparados com distinção de maiúsculas/minúsculas**
  e não são normalizados.
- **Espaço em branco** (incluindo quebras de linha) é insignificante exceto como
  separador de tokens (`to_ast.py:167-175`). Quebras de linha **não** são tokens
  e não têm significado sintático para terminação de statement; os statements são
  delimitados pela gramática, com um `;` opcional.

Regras observáveis:

- `γ` é um identificador válido (*probe*); `_x` é um único `IDENT` (*probe*);
  a forma Unicode é normalizada para **NFC** antes de ser armazenada
  (`to_ast.py:212`).
- `f"…"`, `r"…"`, `b"…"`, `rb"…"`/`br"…"` são reconhecidos como *prefixos em uma
  string*, não como identificadores seguidos de uma string (`to_ast.py:218-278`).

### 1.1 Keywords (lista exaustiva — `_RESERVED_BINDING_NAMES`, `to_ast.py:522-530`)

Estas 53 palavras não podem ser usadas como nome de binding. Declarar uma com
`let` ou `const` é erro de parse (`_check_binding_name`, `to_ast.py:562-581`).

```
abstract  and       as        assert    async     await     break
case      catch     class     const     continue  def       else
enum      export    false     finally   fn        for       from
guard     if        import    in        is        let       loop
match     module    new       none      not       or        private
protected public    return    self      spawn     static    super
throw     trait     true      try       type      unless    until
volatile  while     with      yield
```

Notas:

- O conjunto de keywords é **sensível a maiúsculas/minúsculas**: `if` é keyword,
  `If` é identificador comum.
- `fn` **é reservado** e rejeitado com um erro apontado
  (`'fn' is not part of Aura; use 'def' instead`, `to_ast.py:816-818`) *e* como
  nome de binding (`to_ast.py:522-530`). Não existe `fn`.
- `and`, `or`, `not`, `is`, `in` são **operadores**, não nomes declaráveis.
- `true`, `false`, `none` são literais keyword, não identificadores.
- **A aplicação de nome de binding não é uniforme.** `let if = 1` é rejeitado,
  mas `def if() { }` e `def f(if) { }` parseiam (`_check_binding_name` é chamado
  apenas no caminho do nome de *variável*). Esta é uma assimetria observável do
  parser, não uma garantia da linguagem (*probe*).
- Literais com forma de Python também são capturados no local de binding:
  `let True = 1` →
  `'True' is not an Aura literal; write 'true' instead`
  (`_PYTHON_LITERAL_ALIASES`, `to_ast.py:533-537`).
- `volatily` é rejeitado pelo tokenizer com
  `'volatily' is not a keyword; did you mean 'volatile'?` (`to_ast.py:281-284`).

---

## 2. Comentários

| Forma | Regra | Evidência |
|---|---|---|
| Linha | `//` até o fim da linha | `to_ast.py:181-184` |
| Bloco | `/*` … `*/`, **não aninhável**, pode atravessar linhas | `to_ast.py:187-204` |

```
comment       = line_comment | block_comment ;
line_comment  = "//" , { ? any char except newline ? } ;
block_comment = "/*" , { ? any char ? } , "*/" ;
```

Regras observáveis:

- Um comentário de bloco **termina no primeiro `*/`**. `/* a /* b */ let x = 1`
  fecha no primeiro `*/` e então parseia `let x = 1`; aninhamento **não** existe
  (*probe*).
- Um comentário de bloco não terminado levanta
  `SyntaxError: unterminated block comment` (`to_ast.py:198-202`).
- Comentários são **descartados**; eles não aparecem no fluxo de tokens e não há
  metadados de doc-comment (`to_ast.py:181-204`).
- `//` **sempre** inicia um comentário de linha. Aura deliberadamente não tem
  operador de divisão inteira; a divisão inteira é escrita `int(total / count)`
  (`syntax.md`8).

---

## 3. Literais numéricos

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
```

Evidência: `to_ast.py:288-372`; `grammar.md` §1.4.

Regras observáveis:

- **Underscores são separadores de dígitos** dentro de literais decimal,
  hexadecimal, octal e binário; são removidos antes da conversão. `1_000_000` →
  `1000000`, `0xFF_FF` → `65535` (*probe*).
- **Prefixos de base são `0x`/`0X`, `0o`/`0O`, `0b`/`0B`** (`to_ast.py:294-316`).
  `0xFF` → 255, `0o755` → 493, `0b1010` → 10 (*probe*).
- Um literal de base malformado levanta `Invalid base-N integer literal '…'`
  (`to_ast.py:309-313`).
- **Notação científica** exige um dígito após `e`/`E` (opcionalmente com sinal):
  `2.5e10`, `1.5e-5`, `1E+3`, `.5e2` são floats (*probe*).
- **Um `.` seguido de um dígito é um float.** `1.5` é um só `FLOAT`; `x..y` é
  `x`, `..`, `y` (um range, não um float), e `a.5` é `a` seguido do float
  `0.5` (`grammar.md` §1.4; `tests/test_tokenizer.py::test_float_vs_range`).
- `1.` (ponto não seguido de dígito) **não** é um float: ele lexa como `INT 1`
  e então `OP .` (*probe*).
- Um float com ponto inicial é válido: `.5` → `0.5`, `.5e2` → `50.0` (*probe*).
- **Não há sufixo sem sinal/tipado** nem sufixo `1f`/`1d`/`1L` (`grammar.md`
  §1.4; o tokenizer não tem tal ramo).

---

## 4. Strings

```
string_literal = [ prefix ] , ( short_string | triple_string ) ;
prefix         = ( "r" | "b" | "f" | "rb" | "br" ) ;        // case-insensitive
short_string   = '"' , { char | escaped_char } , '"'
               | "'" , { char | escaped_char } , "'" ;
triple_string  = '"""' , { ? any ? } , '"""'
               | "'''" , { ? any ? } , "'''" ;
```

Evidência: `to_ast.py:214-278, 374-418`; `grammar.md` §1.4, §6.8.

Regras observáveis:

- **Os estilos de aspas são intercambiáveis**: `"…"` e `'…'` produzem um token
  `STRING` (`to_ast.py:375`, `tests/test_tokenizer.py::test_literals`).
- **Strings de aspas triplas** `"""…"""` e `'''…'''` atravessam linhas e produzem
  um token `STRING` (`to_ast.py:379-398`).
- **Escapes** (`_ESCAPES`, `to_ast.py:106-110`): `\n \t \r \0 \\ \" \' \b \f \v
  \a`, mais `\xHH`, `\uHHHH` e `\UHHHHHHHH` (`to_ast.py:112-160`).
  Um surrogate isolado em `\u`/`\U` fica como o caractere `u`/`U` em vez de
  levantar erro (`to_ast.py:140-141`).
- **Escapes desconhecidos mantêm o caractere escapado**: `"\q"` → `"q"` (*probe*).
- `f"…"` produz um token `FSTRING` carregando `(quote, raw_text)`; as partes são
  parseadas depois, de modo que o código Aura interpolado é transformado
  (`to_ast.py:218-243`; `tests/test_tokenizer.py::test_f_strings`). F-strings
  suportam chaves aninhadas e aspas triplas (`grammar.md` §6.8).
- **Prefixos raw/bytes** `r`, `b`, `rb`, `br` (qualquer caixa) produzem um token
  `RAWSTRING` com o literal mantido **verbatim** — backslashes não são
  re-decodificados (`to_ast.py:245-278`). Exemplo: `r"\d"` armazena `r"\d"`
  exatamente (*probe*).
- Strings curtas/triplas/raw **não terminadas** levantam
  `SyntaxError: unterminated string literal` (`to_ast.py:269-272, 389-393,
  409-413`).

---

## 5. Literais booleanos e none

| Grafia | Token | Evidência |
|---|---|---|
| `true` | `BoolLiteral(True)` | `to_ast.py:2631-2633` |
| `false` | `BoolLiteral(False)` | `to_ast.py:2634-2636` |
| `none` | `NoneLiteral()` | `to_ast.py:2654-2656` |

`null` **não** é Aura: `'null' is not part of Aura; use 'none' instead`
(`to_ast.py:2637-2639`). As grafias Python `True`, `False`, `None` são
rejeitadas como literais (`to_ast.py:533-537`).

---

## 6. Operadores e pontuação

O tokenizer usa **maximal munch**: operadores de três caracteres, depois de dois,
depois de um (`to_ast.py:420-441`).

```
punctuation = "(" | ")" | "[" | "]" | "{" | "}" | "," | ":" | ";" | "." | "?" | "@" ;
three_char  = "..<" | "..." | "??=" | "**=" | "<<=" | ">>=" ;
two_char    = "==" | "!=" | "<=" | ">=" | "->" | "=>" | "+=" | "-=" | "*=" | "/="
            | "%=" | "&=" | "|=" | "^=" | "<<" | ">>" | ".." | "??" | "?:" | "?."
            | "?[" | "|>" | "**" ;
one_char    = "+" | "-" | "*" | "/" | "%" | "<" | ">" | "=" | "&" | "|" | "^" | "~" ;
```

> `&&`, `||` e `!` ainda são **tokenizados** mas rejeitados pelo parser com uma
> mensagem apontada (`to_ast.py:432`, `2435-2437`, `2497-2502`). Eles não fazem
> parte de Aura; use `and`, `or`, `not`.

Uma tabela completa de tokens de operador (`to_ast.py:420-441`, verificada por
*probe*):

| Texto | Tipo | Texto | Tipo |
|---|---|---|---|
| `(` `)` `[` `]` `{` `}` | delimitador | `..<` | range (exclusivo) |
| `,` `:` `;` `.` | delimitador | `...` | marcador de spread |
| `?` `@` `~` | pontuação | `??=` | atribuição de coalescência |
| `+` `-` `*` `/` `%` | aritmético | `**=` | atribuição de potência |
| `==` `!=` `<` `>` `<=` `>=` | comparação | `<<=` `>>=` | atribuição de shift |
| `+=` `-=` `*=` `/=` `%=` | atrib. composta | `&= &#124;= ^=` | atrib. composta |
| `<<` `>>` | shift | `..` | range (inclusivo) |
| `->` | seta de retorno / case | `=>` | seta de lambda |
| `??` | coalescência de nulo | `?:` | Elvis |
| `?.` | navegação segura | `?[` | índice seguro |
| `&#124;>` | pipe | `**` | potência |
| `& &#124; ^` | bit a bit | | |

`@` introduz decorators (`grammar.md` §2, §3.3).

---

## 7. Tabela de erros léxicos

| Mensagem | Causa | Evidência |
|---|---|---|
| `unterminated block comment` | `/*` sem `*/` | `to_ast.py:198-202` |
| `unterminated string literal` | `"`, `"""`, `'`, `'''`, `r"…` sem aspas de fechamento | `to_ast.py:269-272, 389-393, 409-413` |
| `Invalid base-N integer literal '…'` | dígitos `0x`/`0o`/`0b` malformados | `to_ast.py:309-313` |
| `'volatily' is not a keyword; did you mean 'volatile'?` | alias com erro de digitação | `to_ast.py:281-284` |

Todos são levantados como `SyntaxError` anotado com linha e coluna
(`to_ast.py:94-104, 583-593`).

---

## 8. Tokens léxicos que a sintaxe não usa

Estas sequências de caracteres lexam com sucesso mas não têm produção no parser e
produzem um erro posicionado quando alcançadas em posição de expressão:

| Grafia | Resultado | Evidência |
|---|---|---|
| `!x` | `'!' is not part of Aura; use 'not' instead` | `to_ast.py:2435-2437` |
| `a && b` | `'&&' is not part of Aura; use 'and' instead` | `to_ast.py:2497-2502` |
| `a &#124;&#124; b` | `'&#124;&#124;' is not part of Aura; use 'or' instead` | `to_ast.py:2497-2502` |
| `1...10` | `unexpected '...' in expression; use '..' or '..<' for ranges` | `to_ast.py:2491-2494` |
| `*x` / `**x` / `...x` em posição de valor | `'…' spread is not allowed in an expression` | `to_ast.py:2456-2466` |
| `x is "a"` | `'is' compares identity; use '==' (or '!=') …` | `to_ast.py:2567-2577` |

Veja [syntax.md](syntax.md) para a tabela completa de anti-formas.
