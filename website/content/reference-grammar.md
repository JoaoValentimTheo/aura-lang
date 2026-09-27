# Grammar

This is the canonical Aura syntax, reproduced from the repository's
`docs/grammar.md`. The parser implements exactly this grammar, and
`tests/grammar.rs` parses every production below. When the parser and this
grammar disagree, the parser is wrong.

```ebnf
(* ---------------------------------------------------------------- file *)
file            = { NEWLINE | item } EOF ;
item            = [ "pub" ] ( fn_decl | struct_decl | enum_decl
                            | type_alias | use_decl | const_decl
                            | impl_decl | trait_decl )
                | expr_stmt ;

use_decl        = "use" IDENT { "." IDENT } terminator ;

(* ---------------------------------------------------------- declarations *)
(* A declaration may introduce ordered generic type parameters after its name:
   `fn f<T>(...)`, `struct S<T> {`, `trait T<A> {`, `impl<A> S<A> {`,
   `type A<T> = ...`. A bound is `T: Trait` or `T: A + B`. In these positions
   `<` is a generic bracket; in expressions it stays comparison/shift. *)
type_params     = "<" type_param { "," type_param } ">" ;
type_param      = IDENT [ ":" IDENT { "+" IDENT } ] ;

fn_decl         = "fn" IDENT [ type_params ] "(" [ params ] ")" [ "->" type ] block ;
params          = param { "," param } ;
param           = IDENT [ ":" type ] ;

(* behavior block: a struct may have several, which merge into one method
   surface and may add overloads; methods take the receiver `self` as their
   first parameter. `impl` here is contextual — an `impl StructName {` item or
   an `impl TraitName for StructName {` item — and `self` has receiver meaning
   only in this position; both stay ordinary identifiers elsewhere. *)
impl_decl       = "impl" [ type_params ] type_head [ "for" type_head ] "{"
                  { NEWLINE | [ "pub" ] method_decl } "}" ;
type_head       = IDENT { "::" IDENT } [ "<" type { "," type } ">" ] ;
method_decl     = "fn" IDENT [ type_params ]
                  "(" receiver [ "," param { "," param } [ "," ] ] ")"
                  [ "->" type ] block ;
receiver        = [ "mut" ] "self" ;

(* trait: a behavioral contract. Signatures only — no bodies, fields, or
   associated items. `trait` is contextual and stays an ordinary identifier
   outside this item form. *)
trait_decl      = "trait" IDENT [ type_params ] "{"
                  { NEWLINE | [ "pub" ] trait_method } "}" ;
trait_method    = "fn" IDENT [ type_params ]
                  "(" receiver [ "," param { "," param } [ "," ] ] ")"
                  [ "->" type ] terminator ;

struct_decl     = "struct" IDENT [ type_params ] "{"
                  [ field { "," field } [ "," ] ] "}" ;
field           = IDENT ":" type ;

enum_decl       = "enum" IDENT [ type_params ] "{"
                  [ variant { "," variant } [ "," ] ] "}" ;
variant         = IDENT [ "(" [ type { "," type } ] ")" ] ;

type_alias      = "type" IDENT [ type_params ] "=" type terminator ;

const_decl      = "const" NAME [ ":" type ] "=" expr terminator
                | "let" IDENT [ ":" type ] "=" expr terminator ;
NAME            = UPPER { LETTER | DIGIT | "_" } ;
(* `const NAME = e` is the canonical module constant (NAME uppercase). A
   top-level `let` is the same module constant, kept for compatibility. *)

(* ----------------------------------------------------------------- types *)
type            = type_member { "|" type_member } ;
type_member     = "int" | "float" | "bool" | "string" | "none"
                | "[" type "]"
                | "{" type ":" type "}"
                | IDENT [ "<" type { "," type } ">" ]
                | IDENT { "::" IDENT } [ "<" type { "," type } ">" ] ;

(* ------------------------------------------------------------- statements *)
block           = "{" { terminator | stmt } "}" ;
terminator      = NEWLINE | ";" ;

stmt            = let_stmt | assign_or_expr | return_stmt | throw_stmt
                | break_stmt | continue_stmt | while_stmt | loop_stmt
                | for_stmt | try_stmt ;

let_stmt        = "let" [ "mut" ] let_pattern [ ":" type ] "=" expr terminator ;
let_pattern     = IDENT | let_list_pattern | let_variant_pattern ;
let_list_pattern    = "[" [ let_pattern { "," let_pattern } [ "," ] ] "]" ;
let_variant_pattern = IDENT [ "(" [ let_pattern { "," let_pattern } ] ")" ] ;
assign_or_expr  = expr [ assign_op expr ] terminator ;
assign_op       = "=" | "+=" | "-=" | "*=" | "/=" | "%=" | "^="
                | "&=" | "|=" | "<<=" | ">>=" ;
return_stmt     = "return" [ expr ] terminator ;
throw_stmt      = "throw" expr terminator ;
break_stmt      = "break" terminator ;
continue_stmt   = "continue" terminator ;
while_stmt      = "while" expr block ;
loop_stmt       = "loop" block ;
for_stmt        = "for" pattern "in" expr block ;
try_stmt        = "try" block "catch" IDENT block [ "finally" block ] ;

(* ------------------------------------------------------------ expressions *)
expr            = pipe ;
pipe            = logic_or { "|>" logic_or } ;
logic_or        = logic_and { "or" logic_and } ;
logic_and       = equality { "and" equality } ;
equality        = comparison { ( "==" | "!=" ) comparison } ;
comparison      = bit_or { ( "<" | "<=" | ">" | ">=" ) bit_or } ;
bit_or          = bit_and { "|" bit_and } ;
bit_and         = shift { "&" shift } ;
shift           = range { ( "<<" | ">>" ) range } ;
range           = additive [ ".." range ] ;
additive        = multiplicative { ( "+" | "-" ) multiplicative } ;
multiplicative  = power { ( "*" | "/" | "%" ) power } ;
power           = unary [ "^" power ] ;
unary           = ( "-" | "not" | "~" ) unary | postfix ;
postfix         = atom { call_or_member } ;
call_or_member  = "(" [ call_args ] ")"
                | "[" expr "]"
                | "." IDENT [ "<" type { "," type } ">" ] "(" [ call_args ] ")" ;
call_args       = arg { "," arg } [ "," ] ;
arg             = [ IDENT ":" ] expr ;

atom            = INT | FLOAT | STRING | FSTRING
                | "true" | "false" | "none"
                | IDENT [ "<" type { "," type } ">" ] "(" [ call_args ] ")"
                | IDENT "{" [ field_init { "," field_init } ] "}"
                | "(" expr ")"
                | "(" expr "," [ expr { "," expr } [ "," ] ] ")"
                | lambda
                | list | map | block_expr
                | if_expr | match_expr ;

lambda          = [ "fn" ] "(" [ lambda_param { "," lambda_param } [ "," ] ] ")" "->" expr
                | "fn" [ "mut" ] IDENT "->" expr ;
lambda_param    = [ "mut" ] IDENT [ ":" type ] ;
list            = "[" [ expr { "," expr } [ "," ] ] "]" ;
map             = "{" entry { "," entry } [ "," ] "}" | "{" ":" "}" ;
entry           = expr ":" expr ;
block_expr      = block ;
field_init      = IDENT ":" expr ;

if_expr         = "if" expr block [ "else" expr ] ;
match_expr      = "match" expr "{" { match_arm } "}" ;
match_arm       = pattern [ "if" expr ] "->" ( block | expr terminator ) ;

(* --------------------------------------------------------------- patterns *)
pattern         = literal_pattern | bind_pattern | list_pattern
                | variant_pattern ;
literal_pattern = INT | STRING | "true" | "false" | "none" ;
bind_pattern    = IDENT | "_" ;
list_pattern    = "[" [ pattern { "," pattern } [ "," ] ] "]" ;
variant_pattern = IDENT [ "(" [ pattern { "," pattern } ] ")" ] ;

(* ----------------------------------------------------------------- f-strings *)
FSTRING         = 'f"' { fchar | "{{" | "}}" | "{" expr [ ":" format_spec ] "}" } '"' ;
format_spec     = [ [ fchar ] ( "<" | ">" | "^" ) ] [ "+" | "-" | " " ]
                  [ digits ] [ "." digits ] [ type ] ;
type            = "d" | "b" | "o" | "x" | "X" | "f" | "F" | "e" | "E" | "%" ;
```

## Notes

* `else if` is the nested form `if A { X } else { if B { Y } else { Z } }`.
* `else`, `catch`, and `finally` must be on the same line as the preceding `}`.
* `&&`/`||` are not operators (they lex as two `&`/`|` tokens and fail to
  parse); `!` is a lexical error.
* Bitwise operators `&`, `|`, `~`, `<<`, `>>` exist on `int`; there is no XOR
  (`^` is exponentiation) and no `++`/`--`.
* A trailing comma is accepted before every closing delimiter.
* A number may not be immediately followed by a name: `1abc` is `E1002`.
* `let` requires an initializer (`E2005`); `const NAME = e` is the canonical
  module constant and its name is uppercase.
* An f-string interpolates `{expr}`, escapes braces with `{{`/`}}`, and accepts
  a small format spec after `:` (`{x:.2f}`, `{n:>6}`, `{n:06d}`, `{n:x}`); it
  has no `{x=}` or `{x!r}`.
* `module Name { ... }` declares an in-source module; items are private by default and exported with `pub`; `use path [as Alias]` imports a name; `::` is the path separator.
* A parenthesized comma list is list sugar; Aura has no distinct tuple value.
