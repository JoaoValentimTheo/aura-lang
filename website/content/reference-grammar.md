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
                            | impl_decl )
                | expr_stmt ;

use_decl        = "use" IDENT { "." IDENT } terminator ;

(* ---------------------------------------------------------- declarations *)
fn_decl         = "fn" IDENT "(" [ params ] ")" [ "->" type ] block ;
params          = param { "," param } ;
param           = IDENT [ ":" type ] ;

(* behavior block: one per struct; methods take the receiver `self` as their
   first parameter. `impl` here is contextual — an `impl StructName {` item — and
   `self` has receiver meaning only in this position; both stay ordinary
   identifiers elsewhere. *)
impl_decl       = "impl" IDENT "{" { NEWLINE | [ "pub" ] method_decl } "}" ;
method_decl     = "fn" IDENT "(" "self" [ "," param { "," param } ] ")"
                  [ "->" type ] block ;

struct_decl     = "struct" IDENT "{" [ field { "," field } [ "," ] ] "}" ;
field           = IDENT ":" type ;

enum_decl       = "enum" IDENT "{" [ variant { "," variant } [ "," ] ] "}" ;
variant         = IDENT [ "(" [ type { "," type } ] ")" ] ;

type_alias      = "type" IDENT "=" type terminator ;

const_decl      = "let" IDENT [ ":" type ] "=" expr terminator ;

(* ----------------------------------------------------------------- types *)
type            = type_member { "|" type_member } ;
type_member     = "int" | "float" | "bool" | "string" | "none"
                | "[" type "]"
                | "{" type ":" type "}"
                | IDENT ;

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
assign_op       = "=" | "+=" | "-=" | "*=" | "/=" ;
return_stmt     = "return" [ expr ] terminator ;
throw_stmt      = "throw" expr terminator ;
break_stmt      = "break" terminator ;
continue_stmt   = "continue" terminator ;
while_stmt      = "while" expr block ;
loop_stmt       = "loop" block ;
for_stmt        = "for" pattern "in" expr block ;
try_stmt        = "try" block "catch" IDENT "->" block [ "finally" block ] ;

(* ------------------------------------------------------------ expressions *)
expr            = pipe ;
pipe            = logic_or { "|>" logic_or } ;
logic_or        = logic_and { "or" logic_and } ;
logic_and       = equality { "and" equality } ;
equality        = comparison { ( "==" | "!=" ) comparison } ;
comparison      = range { ( "<" | "<=" | ">" | ">=" ) range } ;
range           = additive [ ".." range ] ;
additive        = multiplicative { ( "+" | "-" ) multiplicative } ;
multiplicative  = power { ( "*" | "/" | "%" ) power } ;
power           = unary [ "^" power ] ;
unary           = ( "-" | "not" ) unary | postfix ;
postfix         = atom { call_or_member } ;
call_or_member  = "(" [ call_args ] ")"
                | "[" expr "]"
                | "." IDENT [ "(" [ call_args ] ")" ] ;
call_args       = arg { "," arg } ;
arg             = [ IDENT ":" ] expr ;

atom            = INT | FLOAT | STRING | FSTRING
                | "true" | "false" | "none"
                | IDENT "(" [ call_args ] ")"
                | IDENT "{" [ field_init { "," field_init } ] "}"
                | "(" expr ")"
                | "(" expr "," [ expr { "," expr } [ "," ] ] ")"
                | lambda
                | list | map | block_expr
                | if_expr | match_expr ;

lambda          = [ "fn" ] "(" [ IDENT { "," IDENT } ] ")" "->" expr
                | "fn" IDENT "->" expr ;
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
FSTRING         = 'f"' { fchar | "{{" | "}}" | "{" expr "}" } '"' ;
```

## Notes

* `else if` is the nested form `if A { X } else { if B { Y } else { Z } }`.
* `else`, `catch`, and `finally` must be on the same line as the preceding `}`.
* `&&`, `||`, and `!` do not exist (`E1001`).
* A number may not be immediately followed by a name: `1abc` is `E1002`.
* `let` requires an initializer (`E2005`).
* `use` and `pub` are reserved and inert (they parse but do nothing).
* A parenthesized comma list is list sugar; Aura has no distinct tuple value.
