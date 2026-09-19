# Aura Grammar Specification

This document is the **single source of truth** for Aura's concrete syntax. The
hand-written tokenizer and recursive-descent parser in
`aura/parser/to_ast.py` implement this grammar; when they disagree, one of them
is a bug.

The grammar is written in EBNF:

```
rule      = definition ;
terminals = "literal" | /regex/ ;
?x?       = optional ;
x*        = zero or more ;
x+        = one or more ;
x | y     = alternation ;
( ... )   = grouping ;
```

> **Status:** canonical syntax for Aura `0.1.0a5` onward. Redundant spellings
> removed in `0.1.0a5` are listed in [Appendix A](#appendix-a-removed-spellings);
> they are **not** part of the grammar.

---

## 1. Lexical structure

### 1.1 Whitespace and comments

Whitespace is insignificant except as a token separator. Newlines are
**not** significant; statements are delimited by grammar, with optional `;`.

```
comment        = line_comment | block_comment ;
line_comment   = "//" , { ? any char except newline ? } ;
block_comment  = "/*" , { ? any char ? } , "*/" ;
```

Block comments may span lines. `//` always starts a line comment; Aura has no
floor-division operator.

### 1.2 Identifiers

```
identifier     = ( letter | "_" ) , { letter | digit | "_" } ;
letter         = ? Unicode letter ? ;
digit          = "0" .. "9" ;
```

Unicode letters are accepted (matching Python identifiers). Identifiers are
matched case-sensitively and are not normalized.

### 1.3 Keywords

Reserved words that may not be used as identifiers:

```
let  mut  const  def  class  trait  enum  type  module  import  from
if  else  unless  guard  while  until  for  in  loop  break  continue
return  throw  try  catch  finally  with  match  case  assert
async  await  yield  spawn  true  false
public  private  protected  static  volatile  abstract  export
self  super
```

Contextual words (reserved only in specific positions, usable as identifiers
elsewhere): `fn` is **removed**; `step`, `as`, `is`, `and`, `or`, `not`, `not in`,
`is not`, `then` are operators/contextual. `extends` and `implements` are
matched as contextual words after a class or trait name: `extends` is the only
inheritance keyword, and `implements` is reported as a pointed error telling you
to use `extends` instead.

### 1.4 Literals

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

A `.` followed by a digit always lexes as a float. Consequently `a.5` is
`a` followed by `0.5`; use `a[5]` or `a.b5` for member/index intent.

### 1.5 Operators and punctuation

Longest-match order matters. Three-character operators are matched before
two-character, which are matched before one-character.

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

> `&&`, `||`, `!` are **removed**; use `and`, `or`, `not`.

---

## 2. Program structure

```
program        = { statement } , EOF ;

statement      = decorated_statement
               | declaration
               | control_statement
               | simple_statement ;

decorated_statement = { decorator } , declaration ;

decorator      = "@" , identifier , [ "(" , [ arg_list ] , ")" ] ;
```

A **program executed as an entry file** (`aura run`) must declare a top-level
`def main()`; its parameters are either none or a single `args` (error `E310`
when missing, `E311` when the signature is wrong). The runtime invokes `main`
with the command-line arguments, so a trailing `main()` call is not written. A
file used purely as a **module** (`import`ed by another file) needs no `main`.

---

## 3. Declarations

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

Modifiers appear **before** the declaration keyword
(`private let x = 1`, not `let private x`). `export` is **not** a general
modifier: it appears only at the start of a `module` member (see
[Modules](#37-modules)).

At class/trait member level a **visibility is mandatory**: every member must
begin with exactly one of `public`, `private` or `protected` (omitting it is
error `E307`).

### 3.1 Variables

```
var_decl       = modifiers , "let" , [ "mut" ] , pattern , [ ":" , type ] , [ "=" , expression ] , [ ";" ] ;
const_decl     = [ modifiers ] , "const" , identifier , [ ":" , type ] , "=" , expression , [ ";" ] ;

pattern        = tuple_pattern | list_pattern | identifier ;
tuple_pattern  = "(" , pattern , { "," , pattern } , [ "," ] , ")" ;
list_pattern   = "[" , pattern , { "," , pattern } , [ "," ] , "]" ;
```

`let` declares an immutable binding; `let mut` declares a mutable one.
Destructuring is written with tuple/list patterns and may include a rest
element `...name`:

```
let (a, b) = (1, 2)
let [first, ...rest] = [1, 2, 3]
```

### 3.2 Functions

```
function_decl  = modifiers , [ "async" ] , "def" , identifier , [ type_params ]
                 , "(" , [ param_list ] , ")" , [ "->" , type ] , block ;

type_params    = "[" , identifier , { "," , identifier } , "]" ;

param_list     = param , { "," , param } ;
param          = "*" | "**" , identifier
               | identifier , [ ":" , type ] , [ "=" , expression ] ;
```

The body is always a brace block. `def` is the only function keyword
(`fn` is removed).

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

Member modifiers and decorators may appear in any order and on separate lines:
`@staticmethod public def f` and `public @staticmethod def f` are equivalent.
A decorator on a field is rejected while parsing (`E320`).

Inheritance uses `extends` only; there is no parenthesised base list and no
`implements`. Since the header is introduced by `(`, a bare name there is never
a base class. A dotted base (`class Model extends django.db.models.Model`) or a
name bound by an `import` marks a base Python owns: Aura then leaves the
constructor and attribute protocol to the library's own metaclass, so
`enum.Enum`, `pydantic.BaseModel` and ORM models work unchanged. Declaring
header fields alongside such a base opts back in to the generated constructor.

Every base must resolve to a declared class/trait or a builtin exception root.
A base that does not exist is `E314`; a base listed twice or an `extends` cycle
is `E315`. Traits, `abstract class`es and classes with unimplemented abstract
methods cannot be instantiated (`E316`). `super.m()` on an abstract (body-less)
method is `E321`.

An `abstract class` cannot be instantiated and may declare `abstract def`
members — signatures with no body. A concrete class must implement every
abstract method it inherits (from an `abstract class` or a trait), or it is
`E309`. An `abstract def` may appear only in an `abstract class`; one in a
concrete class is also `E309`. A class has exactly one constructor style:
header fields generate the constructor, so a header plus a manual `new` is a
syntax error.

Overriding is implicit: there is no `override` modifier (`override def` is a
syntax error). A trait does not use `abstract` either — a body-less trait method
is already abstract (`trait T { abstract def f() }` is a syntax error).

Header fields become instance fields, constructor parameters and accessors.
The default visibility is `private`, and fields are immutable unless declared
`mut`. A `let` field in the body is likewise immutable; `let mut`/`mut` opts
into a setter. A field whose default is a descriptor (a value whose class
defines `__get__`/`__set__`) stays on the class, so Python's descriptor
protocol runs on instance access.

### 3.4 Traits

```
trait_decl     = modifiers , "trait" , identifier , [ type_params ]
                 , [ "extends" , dotted_name , { "," , dotted_name } ]
                 , "{" , { trait_member } , "}" ;

trait_member   = member_modifiers , ( method_signature | method_with_body | field | const_field ) ;
method_signature = "def" , identifier , "(" , [ param_list ] , ")" , [ "->" , type ] ;   // no body
method_with_body = method ;
```

Traits compile to abstract base classes: signature-only methods become
`@abstractmethod`. A trait extends other traits with `extends`, and a concrete
class that omits any inherited abstract method is rejected at compile time
(E309).

### 3.5 Enums

```
enum_decl      = modifiers , "enum" , identifier , "{" , enum_member , { "," , enum_member } , [ "," ] , "}" ;
enum_member    = identifier , [ "=" , expression ] ;
```

Members are separated by commas. Values auto-number from the previous
integer value when omitted.

### 3.6 Type aliases

```
type_alias     = "type" , identifier , [ type_params ] , "=" , type ;
```

### 3.7 Modules

```
module_decl    = "module" , dotted_name , "{" , { module_member } , "}" ;
module_member  = [ "export" ] , statement
               | "export" , name_list , [ "from" , string ] ;   // re-export
name_list      = identifier , { "," , identifier } ;
```

A module member is private to the declaring file unless prefixed with `export`.
`export` must precede a named declaration (`def`, `class`, `trait`, `enum`,
`type`, `let`, `const` or a nested `module`), or it may introduce a **re-export**
— a bare name (or comma-separated names) resolved against a sibling source
file. The optional `from "module"` names that source explicitly. A `from` path
must be a plain dotted name (no `/`, `\`, `..` or absolute paths). Non-exported
members are mangled in the generated Python, so privacy is enforced at runtime
as well as by the rule checker (`E308`), and module state is not assignable from
outside (`E303`). An unresolvable re-export is `E313`; a `main` inside a module
body is `E312`.

### 3.8 Imports

```
import_stmt    = "import" , import_target , { "," , import_target }
               | "from" , dotted_name , "import" , ( "*" | import_item , { "," , import_item } ) ;

import_target  = dotted_name , [ "as" , identifier ] , [ "{" , import_item , { "," , import_item } , "}" ] ;
import_item    = identifier , [ "as" , identifier ] ;
```

---

## 4. Types

Types are parsed for documentation and checking; they are erased at runtime.

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

The **`[T]` bracket form is canonical** for type parameters and type
arguments; the `<T>` form is removed.

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

### 5.1 Conditionals

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

### 5.3 Blocks and with

```
block          = "{" , { statement } , "}" ;
with_stmt      = [ "async" ] , "with" , with_item , { "," , with_item } , block ;
with_item      = expression , [ "as" , identifier ] ;
```

`with` uses `__enter__`/`__exit__`; `async with` uses the coroutine protocol
`__aenter__`/`__aexit__` and is only valid inside an `async def` body, just as
`await` is.

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

A `try` requires at least one `catch` clause or a `finally` block.

A lone identifier before `{` is always a **type**: `catch TypeError { }` filters
by type, and `catch as e { }` binds every exception. The old ambiguous
`catch e { }` form (a bare binding) is a parse error.

---

## 6. Expressions

### 6.1 Precedence (lowest to highest)

| Level | Operators | Associativity |
|-------|-----------|---------------|
| 1 | `=`, `+=`, `-=`, `*=`, `/=`, `%=`, `**=`, `&=`, `|=`, `^=`, `<<=`, `>>=`, `??=`, `\|>` | right (pipe left) |
| 2 | `? :` (ternary) | right |
| 3 | `or` | left |
| 4 | `and` | left |
| 5 | `\|` (bitwise) | left |
| 5.2 | `^` | left |
| 5.4 | `&` | left |
| 5.5 | `<<`, `>>` | left |
| 6 | `==`, `!=`, `<`, `>`, `<=`, `>=`, `in`, `not in`, `is`, `is not` | left |
| 7 | `..`, `..<` (range) | none |
| 8 | `??`, `?:` (coalescing) | left |
| 9 | `+`, `-` | left |
| 10 | `*`, `/`, `%`, `as` (cast) | left |
| 11 | `**` | right |
| 12 | unary `-`, `+`, `~`, `not`, `await`, `...` (spread) | prefix |
| 13 | call, index, slice, member, safe-nav, struct-init | postfix |

`yield` is a statement-level prefix form, not part of `expression`: the parser
recognizes it before the Pratt loop and parses its operand at the lowest
expression precedence, so `yield x + 1` yields `x + 1` and `yield a, b` yields
the tuple `(a, b)`.

### 6.2 Expression grammar

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

### 6.3 Primary expressions

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

### 6.5 Collections and comprehensions

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

A `{` after an **uppercase-leading** identifier is a struct init; otherwise it
is a block or dict/set literal. Struct init is sugar for
`TypeName(**{ ... })` (transpiles to a constructor call).

### 6.7 Calls and spreads

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

## 7. Standard formatting

`aura format` normalizes to these rules (see `aura/tools/formatter.py`):

- Two-space indentation per block level.
- Braces stay on the same line as the construct; closing `}` stands alone.
- One space around binary operators; none inside `/` comments.
- `->` for return types and match-case arrows.
- Trailing whitespace removed; runs of spaces collapsed.
- String literal contents are never rewritten.

---

## Appendix A: Removed spellings

The following were accepted before `0.1.0a5` and are now errors. Each maps to
its canonical replacement:

| Removed | Canonical | Notes |
|---------|-----------|-------|
| `fn` | `def` | single function keyword |
| `init` | `new` | one constructor name |
| `!x` | `not x` | one negation operator |
| `&&` | `and` | |
| `\|\|` | `or` | |
| `null` | `none` | one null literal |
| `volatily` | `volatile` | typo alias |
| `fn foo<T>()` / `class Foo<T>` | `foo[T]` / `Foo[T]` | brackets only |
| `case x => e` | `case x -> e` | `->` only |
| `let private x` | `private let x` | modifiers prefix the declaration |
| `catch Type` (ambiguous) | `catch Type as e` | explicit binding |

---

## Appendix B: Diagnostics

`aura check` reports rule violations with stable codes (see
`aura/transpiler/errors.py`). Mutability violations (reassigning a `let` or
`const`) are errors, not warnings — enforced identically by `aura run`,
`aura check`, and the REPL.