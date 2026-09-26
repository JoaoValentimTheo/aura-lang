# Aura — canonical grammar (EBNF)

This file is the **source of truth** for Aura v3 syntax. The parser in
`src/parse/mod.rs` implements exactly this grammar; `tests/grammar.rs`
parses every production below. When the parser and this file disagree, the
parser is wrong.

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
(* top-level `let` is a module constant; `let mut` is rejected *)

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
(* `|>` is left associative and lower precedence than every binary operator.
   The right operand is a full `pipe` operand, and `x |> f(a, b)` desugars to
   `f(x, a, b)`: a call's first argument becomes the left operand. `x |> f`
   desugars to `f(x)` when `f` is a callable value. The desugaring is applied
   at parse time; `Expr::Pipe` survives only for an operand that is not a call
   or method. *)
pipe            = logic_or { "|>" logic_or } ;
logic_or        = logic_and { "or" logic_and } ;
logic_and       = equality { "and" equality } ;
equality        = comparison { ( "==" | "!=" ) comparison } ;
comparison      = range { ( "<" | "<=" | ">" | ">=" ) range } ;
range           = additive [ ".." range ] ;       (* Rust-style range *)
additive        = multiplicative { ( "+" | "-" ) multiplicative } ;
multiplicative  = power { ( "*" | "/" | "%" ) power } ;
power           = unary [ "^" power ] ;              (* right associative *)
unary           = ( "-" | "not" ) unary | postfix ;
postfix         = atom { call_or_member } ;
call_or_member  = "(" [ call_args ] ")"
                | "[" expr "]"
                | "." IDENT [ "(" [ call_args ] ")" ] ;
call_args       = arg { "," arg } ;             (* positional, then named *)
arg             = [ IDENT ":" ] expr ;

atom            = INT | FLOAT | STRING | FSTRING
                | "true" | "false" | "none"
                | IDENT "(" [ call_args ] ")"          (* call *)
                | IDENT "(" [ ctor_args ] ")"          (* variant *)
                | IDENT "{" [ field_init { "," field_init } ] "}"  (* struct *)
                | "(" expr ")"
                | "(" expr "," [ expr { "," expr } [ "," ] ] ")"          (* list sugar *)
                | lambda
                | list | map | block_expr
                | if_expr | match_expr ;

lambda          = [ "fn" ] "(" [ IDENT { "," IDENT } ] ")" "->" expr
                | "fn" IDENT "->" expr ;
list            = "[" [ expr { "," expr } [ "," ] ] "]" ;
map             = "{" entry { "," entry } [ "," ] "}" | "{" ":" "}" ;
entry           = expr ":" expr ;
block_expr      = block ;
ctor_args       = ctor_arg { "," ctor_arg } ;
ctor_arg        = [ IDENT ":" ] expr ;
field_init      = IDENT ":" expr ;

if_expr         = "if" expr block [ "else" expr ] ;   (* `else` optional *)
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

* `else if` is supported: `if A { X } else if B { Y } else { Z }` is the
  nested form `if A { X } else { if B { Y } else { Z } }`. `else` accepts an
  expression, so the existing `if_expr` production already covers it.
* `else`, `catch`, and `finally` must appear on the same line as the closing
  `}` of the block they follow; a newline before them is `E1006`.
* `&&`, `||`, and `!` do not exist as operators (`E1001`).
* A number may not be immediately followed by a name: `1abc` is `E1002`.
* `let` requires an initializer (`E2005`).
* Patterns bind lowercase names; a capitalized name is a variant.
* `use` and `pub` are **reserved and inert** in this version: they parse but
  have no effect (see `docs/contract.md` §10).
* `type Name = T` is a transparent alias: it is validated but does not create
  a distinct nominal type.
* A type expression is a `|`-separated union of one or more members; `T | none`
  is the one-member-plus-`none` case. Member order and duplicates are
  normalized away; a union containing `none` is permissive.
* `a..b` is a range literal, equivalent to `range(a, b)`; `..` is a distinct
  token, never part of a float. It binds looser than arithmetic and tighter
  than comparison.
* `#` line comments and `<!-- ... --!>` multiline comments are discarded by
  the lexer; an unterminated multiline comment is `E1005`.
* `(a, b)` creates a list of two elements; Aura has no distinct tuple value.
* Call arguments may be named (`f(x: 1)`). All positional arguments must come
  before all named arguments; a positional argument after a named one is
  `E1006`. Named arguments are supported only for directly resolved top-level
  functions (`E3001` otherwise); see `docs/contract.md` §6.
