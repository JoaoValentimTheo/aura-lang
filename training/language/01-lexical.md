---
title: "01 — Lexical Structure"
---

# Lexical Structure

The Aura tokenizer is hand-written, single-pass, and reads UTF-8 source left to right. It produces a flat list of tokens: `IDENT`, `INT`, `FLOAT`, `STRING`, `RAWSTRING`, `FSTRING`, `OP`, `EOF`.

See `aura/parser/to_ast.py:85-444` (Tokenizer), `to_ast.py:546-2942` (Parser).

## Source Characters

- UTF-8 text, one character at a time.
- Identifiers start with an ASCII letter or `_`, or any Unicode character Python's `str.isidentifier()` accepts. Continuation allows ASCII alphanumerics, `_`, or XID_Continue characters.
- Identifiers are **NFC-normalized** before storage (`to_ast.py:212`).
- Whitespace (including newlines) is insignificant except as a token separator.

## Comments

| Form | Rule |
|------|------|
| Line | `//` to end of line |
| Block | `/* ... */`, not nestable, may span lines |

```aura
// single-line comment
/* block comment
   may span lines */
```

Block comments end at the first `*/`. Nesting does not exist — `/* a /* b */` closes at the inner `*/`. Unterminated block comments raise `SyntaxError`.

`//` always starts a line comment. Aura deliberately has no floor-division operator.

## Numeric Literals

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

Rules:

- Underscores are digit separators: `1_000_000` → `1000000`, `0xFF_FF` → `65535`.
- Radix prefixes: `0x`/`0X` (hex), `0o`/`0O` (octal), `0b`/`0B` (binary).
- Scientific notation: `2.5e10`, `1.5e-5`, `.5e2`.
- A `.` followed by a digit is a float: `1.5` is one `FLOAT` token.
- `1.` (dot not followed by a digit) is **not** a float — it lexes as `INT 1` then `OP .`.
- No unsigned/typed suffixes (`1f`, `1d`, `1L` do not exist).

## Strings

```
string_literal = [ prefix ] , ( short_string | triple_string ) ;
prefix         = "r" | "b" | "f" | "rb" | "br" ;
short_string   = '"' , { char | escaped_char } , '"' | "'" , { char | escaped_char } , "'" ;
triple_string  = '"""' , { ? any ? } , '"""' | "'''" , { ? any ? } , "'''" ;
```

| Feature | Detail |
|---------|--------|
| Quote interchangeability | `"..."` and `'...'` are equivalent |
| Triple-quoted | `"""..."""` and `'''...'''` span lines |
| F-strings | `f"x{1+2}"` — interpolated code is Aura |
| Raw strings | `r"\d"` — backslashes are not decoded |
| Byte strings | `b"..."` — literal bytes |
| Compound prefixes | `rb""`, `br""` (any case) |

Escapes: `\n \t \r \0 \\ \" \' \b \f \v \a`, plus `\xHH`, `\uHHHH`, `\UHHHHHHHH`. Unknown escapes keep the escaped character: `"\q"` → `"q"`.

## Boolean and None Literals

| Spelling | Compiles to |
|----------|-------------|
| `true` | `True` |
| `false` | `False` |
| `none` | `None` |

`null` is rejected with `"null" is not part of Aura; use "none"`. The Python spellings `True`, `False`, `None` are rejected as literals.

## Operators and Punctuation

The tokenizer uses **maximal munch**: three-character operators, then two-character, then one-character (`to_ast.py:420-441`).

| Category | Operators |
|----------|-----------|
| Arithmetic | `+` `-` `*` `/` `%` `**` |
| Comparison | `==` `!=` `<` `>` `<=` `>=` |
| Logical | `and` `or` `not` (not `&&`/`||`/`!`) |
| Bitwise | `&` `\|` `^` `~` `<<` `>>` |
| Assignment | `=` `+=` `-=` `*=` `/=` `%=` `**=` `&=` `\|=` `^=` `<<=` `>>=` `??=` |
| Range | `..` `..<` |
| Null handling | `??` `?:` `?.` `?[` `??=` |
| Pipe | `\|>` |
| Other | `->` `=>` `...` `@` `?` |

`&&`, `||`, and `!` are tokenized but rejected by the parser with pointed messages telling you to use `and`/`or`/`not`.

## Gotcha

- `1.` is not a float; it lexes as `INT 1` then `OP .`.
- `...` is a spread marker or rejected token, not a range. Use `..` or `..<`.
- `a && b` and `a || b` parse successfully but the parser rejects them — use `and`/`or`.
- `is` against any literal except `none` is a parse error — use `==` instead.

## Anti-pattern

Do not use `//` for floor division. It is a line comment. Use `int(total / count)` for integer division.

## Gotcha

- `volatily` is caught by the tokenizer with a typo-suggestion error.
- Unterminated strings and block comments produce `SyntaxError` with line/column info.
