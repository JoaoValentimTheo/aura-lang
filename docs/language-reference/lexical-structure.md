---
layout: default
title: "Lexical Structure"
parent: Aura Language Reference
nav_order: 6
---

[English](lexical-structure.md) | [Português](lexical-structure.pt_BR.md)

# Lexical Structure

**Status:** Stable · **Evidence:** `aura/parser/to_ast.py` (`Tokenizer`,
`to_ast.py:85-444`), `docs/language-reference/grammar.md` §1, `tests/test_tokenizer.py`

The Aura tokenizer is **hand-written and single-pass**. It reads the UTF-8
source left to right, skips whitespace and comments, and emits a flat list of
`Token(type, value, line, column)` (`to_ast.py:49-57`) ending in `EOF`. It is
not tool-generated nor regex-based. Every token is one of: `IDENT`, `INT`,
`FLOAT`, `STRING`, `RAWSTRING`, `FSTRING`, `OP`, `EOF`.

> **Level note:** this document describes the *lexical grammar of the language*
> — which character sequences form tokens. The recursive-descent parser that
> consumes the tokens is described in [grammar.md](grammar.md); the concrete
> spellings of each construct are in [syntax.md](syntax.md).

---

## 1. Source characters

- The file is read as **UTF-8** text, a character at a time
  (`to_ast.py:162-175`).
- **Identifiers** start with an ASCII letter or `_`, or any Unicode character
  that Python's `str.isidentifier()` accepts on its own (`to_ast.py:29-33`), and
  continue with ASCII alphanumerics/`_` or a character that is a valid
  continuation (`to_ast.py:36-43`). There is no identifier escape and no ASCII
  restriction; identifiers are **matched case-sensitively** and are not
  normalized.
- **Whitespace** (including newlines) is insignificant except as a token
  separator (`to_ast.py:167-175`). Newlines are **not** tokens and have no
  syntactic meaning for statement termination; statements are delimited by the
  grammar, with an optional `;`.

Observable rules:

- `γ` is a valid identifier (*probe*); `_x` is a single `IDENT` (*probe*);
  the Unicode form is normalized to **NFC** before being stored
  (`to_ast.py:212`).
- `f"…"`, `r"…"`, `b"…"`, `rb"…"`/`br"…"` are recognized as *prefixes on a
  string*, not as identifiers followed by a string (`to_ast.py:218-278`).

### 1.1 Keywords (exhaustive list — `_RESERVED_BINDING_NAMES`, `to_ast.py:522-530`)

These 53 words may not be used as a binding name. Declaring one with `let` or
`const` is a parse error (`_check_binding_name`, `to_ast.py:562-581`).

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

Notes:

- The keyword set is **case-sensitive**: `if` is a keyword, `If` is an
  ordinary identifier.
- `fn` **is reserved** and rejected with a pointed error
  (`'fn' is not part of Aura; use 'def' instead`, `to_ast.py:816-818`) *and* as
  a binding name (`to_ast.py:522-530`). There is no `fn`.
- `and`, `or`, `not`, `is`, `in` are **operators**, not declarable names.
- `true`, `false`, `none` are keyword literals, not identifiers.
- **Binding-name enforcement is not uniform.** `let if = 1` is rejected, but
  `def if() { }` and `def f(if) { }` parse (`_check_binding_name` is called on
  the *variable* name path only). This is an observable parser asymmetry, not a
  language guarantee (*probe*).
- Python-shaped literals are caught at the binding site too: `let True = 1` →
  `'True' is not an Aura literal; write 'true' instead`
  (`_PYTHON_LITERAL_ALIASES`, `to_ast.py:533-537`).
- `volatily` is rejected by the tokenizer with
  `'volatily' is not a keyword; did you mean 'volatile'?` (`to_ast.py:281-284`).

---

## 2. Comments

| Form | Rule | Evidence |
|---|---|---|
| Line | `//` to the end of the line | `to_ast.py:181-184` |
| Block | `/*` … `*/`, **not nestable**, may cross lines | `to_ast.py:187-204` |

```
comment       = line_comment | block_comment ;
line_comment  = "//" , { ? any char except newline ? } ;
block_comment = "/*" , { ? any char ? } , "*/" ;
```

Observable rules:

- A block comment **ends at the first `*/`**. `/* a /* b */ let x = 1` closes at
  the first `*/` and then parses `let x = 1`; nesting does **not** exist
  (*probe*).
- An unterminated block comment raises `SyntaxError: unterminated block comment`
  (`to_ast.py:198-202`).
- Comments are **discarded**; they do not appear in the token stream and there
  is no doc-comment metadata (`to_ast.py:181-204`).
- `//` **always** starts a line comment. Aura deliberately has no floor-division
  operator; integer division is written `int(total / count)`
  (`syntax.md`8).

---

## 3. Numeric literals

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

Evidence: `to_ast.py:288-372`; `grammar.md` §1.4.

Observable rules:

- **Underscores are digit separators** inside decimal, hex, octal and binary
  literals; they are stripped before conversion. `1_000_000` → `1000000`,
  `0xFF_FF` → `65535` (*probe*).
- **Radix prefixes are `0x`/`0X`, `0o`/`0O`, `0b`/`0B`** (`to_ast.py:294-316`).
  `0xFF` → 255, `0o755` → 493, `0b1010` → 10 (*probe*).
- A malformed radix literal raises `Invalid base-N integer literal '…'`
  (`to_ast.py:309-313`).
- **Scientific notation** requires a digit after `e`/`E` (optionally signed):
  `2.5e10`, `1.5e-5`, `1E+3`, `.5e2` are floats (*probe*).
- **A `.` followed by a digit is a float.** `1.5` is one `FLOAT`; `x..y` is
  `x`, `..`, `y` (a range, not a float), and `a.5` is `a` followed by the float
  `0.5` (`grammar.md` §1.4; `tests/test_tokenizer.py::test_float_vs_range`).
- `1.` (dot not followed by a digit) is **not** a float: it lexes as `INT 1`
  then `OP .` (*probe*).
- A leading-dot float is valid: `.5` → `0.5`, `.5e2` → `50.0` (*probe*).
- There is **no unsigned/typed suffix** and no `1f`/`1d`/`1L` suffix (`grammar.md`
  §1.4; the tokenizer has no such branch).

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

Evidence: `to_ast.py:214-278, 374-418`; `grammar.md` §1.4, §6.8.

Observable rules:

- **Quote styles are interchangeable**: `"…"` and `'…'` both produce a `STRING`
  token (`to_ast.py:375`, `tests/test_tokenizer.py::test_literals`).
- **Triple-quoted strings** `"""…"""` and `'''…'''` span lines and produce a
  `STRING` token (`to_ast.py:379-398`).
- **Escapes** (`_ESCAPES`, `to_ast.py:106-110`): `\n \t \r \0 \\ \" \' \b \f \v
  \a`, plus `\xHH`, `\uHHHH` and `\UHHHHHHHH` (`to_ast.py:112-160`).
  A lone surrogate in `\u`/`\U` is left as the character `u`/`U` rather than
  raising (`to_ast.py:140-141`).
- **Unknown escapes keep the escaped character**: `"\q"` → `"q"` (*probe*).
- `f"…"` produces an `FSTRING` token carrying `(quote, raw_text)`; the parts are
  parsed later so interpolated Aura code is transformed (`to_ast.py:218-243`;
  `tests/test_tokenizer.py::test_f_strings`). F-strings support nested braces and
  triple quotes (`grammar.md` §6.8).
- **Raw/bytes prefixes** `r`, `b`, `rb`, `br` (any case) produce a `RAWSTRING`
  token with the literal kept **verbatim** — backslashes are not re-decoded
  (`to_ast.py:245-278`). Example: `r"\d"` stores `r"\d"` exactly (*probe*).
- **Unterminated** short/triple/raw strings raise
  `SyntaxError: unterminated string literal` (`to_ast.py:269-272, 389-393,
  409-413`).

---

## 5. Boolean and none literals

| Spelling | Token | Evidence |
|---|---|---|
| `true` | `BoolLiteral(True)` | `to_ast.py:2631-2633` |
| `false` | `BoolLiteral(False)` | `to_ast.py:2634-2636` |
| `none` | `NoneLiteral()` | `to_ast.py:2654-2656` |

`null` is **not** Aura: `'null' is not part of Aura; use 'none' instead`
(`to_ast.py:2637-2639`). The Python spellings `True`, `False`, `None` are
rejected as literals (`to_ast.py:533-537`).

---

## 6. Operators and punctuation

The tokenizer uses **maximal munch**: three-character operators, then
two-character, then one-character (`to_ast.py:420-441`).

```
punctuation = "(" | ")" | "[" | "]" | "{" | "}" | "," | ":" | ";" | "." | "?" | "@" ;
three_char  = "..<" | "..." | "??=" | "**=" | "<<=" | ">>=" ;
two_char    = "==" | "!=" | "<=" | ">=" | "->" | "=>" | "+=" | "-=" | "*=" | "/="
            | "%=" | "&=" | "|=" | "^=" | "<<" | ">>" | ".." | "??" | "?:" | "?."
            | "?[" | "|>" | "**" ;
one_char    = "+" | "-" | "*" | "/" | "%" | "<" | ">" | "=" | "&" | "|" | "^" | "~" ;
```

> `&&`, `||` and `!` are still **tokenized** but rejected by the parser with a
> pointed message (`to_ast.py:432`, `2435-2437`, `2497-2502`). They are not part
> of Aura; use `and`, `or`, `not`.

A complete operator token table (`to_ast.py:420-441`, verified by *probe*):

| Text | Kind | Text | Kind |
|---|---|---|---|
| `(` `)` `[` `]` `{` `}` | delimiter | `..<` | range (exclusive) |
| `,` `:` `;` `.` | delimiter | `...` | spread marker |
| `?` `@` `~` | punctuation | `??=` | coalescing assignment |
| `+` `-` `*` `/` `%` | arithmetic | `**=` | power assignment |
| `==` `!=` `<` `>` `<=` `>=` | comparison | `<<=` `>>=` | shift assignment |
| `+=` `-=` `*=` `/=` `%=` | compound assign | `&= \|= ^=` | compound assign |
| `<<` `>>` | shift | `..` | range (inclusive) |
| `->` | return / case arrow | `=>` | lambda arrow |
| `??` | null coalescing | `?:` | Elvis |
| `?.` | safe navigation | `?[` | safe index |
| `\|>` | pipe | `**` | power |
| `& \| ^` | bitwise | | |

`@` introduces decorators (`grammar.md` §2, §3.3).

---

## 7. Lexical error table

| Message | Cause | Evidence |
|---|---|---|
| `unterminated block comment` | `/*` without `*/` | `to_ast.py:198-202` |
| `unterminated string literal` | `"`, `"""`, `'`, `'''`, `r"…` without a closing quote | `to_ast.py:269-272, 389-393, 409-413` |
| `Invalid base-N integer literal '…'` | malformed `0x`/`0o`/`0b` digits | `to_ast.py:309-313` |
| `'volatily' is not a keyword; did you mean 'volatile'?` | typo alias | `to_ast.py:281-284` |

All are raised as `SyntaxError` annotated with line and column
(`to_ast.py:94-104, 583-593`).

---

## 8. Lexical tokens the syntax does not use

These character sequences lex successfully but have no parser production and
produce a positioned error when reached in an expression position:

| Spelling | Result | Evidence |
|---|---|---|
| `!x` | `'!' is not part of Aura; use 'not' instead` | `to_ast.py:2435-2437` |
| `a && b` | `'&&' is not part of Aura; use 'and' instead` | `to_ast.py:2497-2502` |
| `a \|\| b` | `'\|\|' is not part of Aura; use 'or' instead` | `to_ast.py:2497-2502` |
| `1...10` | `unexpected '...' in expression; use '..' or '..<' for ranges` | `to_ast.py:2491-2494` |
| `*x` / `**x` / `...x` in a value position | `'…' spread is not allowed in an expression` | `to_ast.py:2456-2466` |
| `x is "a"` | `'is' compares identity; use '==' (or '!=') …` | `to_ast.py:2567-2577` |

See [syntax.md](syntax.md) for the complete anti-form table.
