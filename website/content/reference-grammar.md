# Grammar

`LANGUAGE_SPEC.md` is the normative syntax and semantic contract. This file
is its formal syntax representation; `contract.md` records compatibility
promises. Guides and the website explain that contract. Historical reports and
roadmaps do not override it. Changes to the language require the RFC process in
`CONTRIBUTING.md`. A disagreement must be investigated against that hierarchy,
not automatically resolved in favor of either the parser or this file.

This grammar describes the current development language. The immutable 0.0.2
runtime is historical. `tests/grammar.rs` and `tests/syntax_conformance.rs`
exercise productions and negative cases; they are not a generated proof that
an arbitrary EBNF and parser agree. The website copy's grammar body is
checked line-for-line against this file (the page title differs by design).

## Productions

Spaces, tabs, CR, and comments are discarded. NEWLINE is LF, including the LF
of CRLF. `nl` below means zero or more NEWLINE tokens. Literal punctuation is
quoted; uppercase lexical names are defined in the lexical section.

```ebnf
file            = nl { item nl } EOF ;
item            = [ "pub" ] ( fn_decl | struct_decl | enum_decl | type_alias
                            | module_decl | use_decl | const_decl | trait_decl )
                | impl_decl | expr_stmt ;
module_decl     = "module" IDENT "{" nl { item nl } "}" ;
use_decl        = "use" path [ "as" IDENT ] statement_end ;
path            = IDENT { "::" IDENT } ;
nl              = { NEWLINE } ;
terminator      = NEWLINE ;
statement_end   = terminator | END_BOUNDARY ;
expr_stmt       = expr statement_end ;

fn_decl         = "fn" IDENT [ type_params ] "(" params ")"
                  [ "->" type ] block ;
params          = nl [ param nl { "," nl param nl } [ "," nl ] ] ;
param           = [ "mut" ] IDENT [ ":" type ] ;
type_params     = "<" type_param { "," type_param } [ "," ] ">" ;
type_param      = IDENT [ ":" bound { "+" bound } ] ;
bound           = IDENT [ type_args ] ;
type_args       = "<" nl type nl { "," nl type nl } [ "," nl ] ">" ;
type_head       = path [ type_args ] ;

impl_decl       = "impl" [ type_params ] type_head [ "for" type_head ]
                  "{" nl { [ "pub" ] method_decl nl } "}" ;
method_decl     = "fn" IDENT [ type_params ] "(" method_params ")"
                  [ "->" type ] block ;
method_params   = nl receiver nl { "," nl param nl } [ "," nl ] ;
receiver        = [ "mut" ] "self" [ ":" type ] ;
trait_decl      = "trait" IDENT [ type_params ]
                  "{" nl { [ "pub" ] trait_method nl } "}" ;
trait_method    = "fn" IDENT [ type_params ] "(" method_params ")"
                  [ "->" type ] statement_end ;

struct_decl     = "struct" IDENT [ type_params ] "{" nl
                  [ field nl { "," nl field nl } [ "," nl ] ] "}" ;
field           = [ "pub" ] IDENT ":" type ;
enum_decl       = "enum" IDENT [ type_params ] "{" nl
                  [ variant nl { "," nl variant nl } [ "," nl ] ] "}" ;
variant         = IDENT [ "(" nl [ type nl { "," nl type nl }
                  [ "," nl ] ] ")" ] ;
type_alias      = "type" IDENT [ type_params ] "=" type statement_end ;
const_decl      = "const" UPPER_NAME [ ":" type ] "=" expr statement_end
                | "let" IDENT [ ":" type ] "=" expr statement_end ;

type            = type_member { "|" type_member } ;
type_member     = "none" | "[" type "]" | "[" type ";" INT "]"
                | "(" type "," nl [ type nl { "," nl type nl }
                  [ "," nl ] ] ")"
                | "{" type "}" | "{" type ":" type "}"
                | path [ type_args ] ;

block           = "{" { terminator | stmt } "}" ;
stmt            = let_stmt | assign_or_expr | return_stmt | throw_stmt
                | break_stmt | continue_stmt | while_stmt | loop_stmt
                | for_stmt | try_stmt ;
let_stmt        = "let" [ "mut" ] BIND_NAME [ ":" type ] "=" expr statement_end
                | "let" let_destructure "=" expr statement_end ;
let_destructure = let_list_pattern | let_variant_pattern ;
let_pattern     = BIND_NAME | let_destructure ;
let_list_pattern = "[" nl [ let_pattern nl { "," nl let_pattern nl }
                   [ "," nl ] ] "]" ;
let_variant_pattern = UPPER_NAME
                | IDENT "(" nl [ let_pattern nl { "," nl let_pattern nl }
                  [ "," nl ] ] ")" ;
assign_or_expr  = expr [ assign_op expr ] statement_end ;
assign_op       = "=" | "+=" | "-=" | "*=" | "/=" | "%=" | "^="
                | "&=" | "|=" | "<<=" | ">>=" ;
return_stmt     = "return" [ expr ] statement_end ;
throw_stmt      = "throw" expr statement_end ;
break_stmt      = "break" statement_end ;
continue_stmt   = "continue" statement_end ;
while_stmt      = "while" expr block ;
loop_stmt       = "loop" block ;
for_stmt        = "for" pattern "in" expr block ;
try_stmt        = "try" block "catch" pattern block [ "finally" block ]
                  [ terminator ] ;

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
postfix         = atom { "(" call_args ")" | "[" expr "]"
                | "." IDENT [ [ type_args ] "(" call_args ")" ] } ;
call_args       = nl [ arg nl { "," nl arg nl } [ "," nl ] ] ;
arg             = [ IDENT ":" ] expr ;
atom            = INT | FLOAT | STRING | FSTRING | "true" | "false" | "none"
                | path [ type_args ( "(" call_args ")" | struct_body ) ]
                | UPPER_PATH ( "(" ctor_args ")" | struct_body )
                | "(" expr ")"
                | "(" expr "," nl [ expr nl { "," nl expr nl }
                  [ "," nl ] ] ")"
                | lambda | list | map | set | block | if_expr | match_expr ;
ctor_args       = nl [ arg nl { "," nl arg nl } [ "," nl ] ] ;
struct_body     = "{" nl [ field_init nl { "," nl field_init nl }
                  [ "," nl ] ] "}" ;
field_init      = IDENT ":" expr ;
lambda          = [ "fn" ] "(" params ")" "->" expr
                | "fn" [ "mut" ] IDENT "->" expr ;
list            = "[" nl ( "]" | list_body ) ;
list_body       = expr nl ( list_comp | list_rest ) ;
list_comp       = "for" pattern "in" expr [ "if" expr ] "]" ;
list_rest       = { "," nl expr nl } [ "," nl ] "]" ;
map             = "{" nl ( ":" nl | map_body ) "}" ;
map_body        = entry nl ( map_comp | map_rest ) ;
map_comp        = "for" pattern "in" expr [ "if" expr ] "}" ;
map_rest        = { "," nl entry nl } [ "," nl ] "}" ;
entry           = expr ":" expr ;
set             = "{" nl expr nl { "," nl expr nl } [ "," nl ] "}"
                | "set" "{" nl ( "}" | expr nl { "," nl expr nl }
                  [ "," nl ] "}" ) ;
if_expr         = "if" expr block [ "else" expr ] ;
match_expr      = "match" expr "{" nl { match_arm nl } "}" ;
match_arm       = pattern [ "if" expr ] "->"
                  ( block | expr [ terminator ] ) [ "," ] ;

pattern         = literal_pattern | BIND_PATH | UPPER_PATH | list_pattern
                | tuple_pattern
                | path "(" nl [ pattern nl { "," nl pattern nl }
                  [ "," nl ] ] ")" ;
literal_pattern = INT | STRING | "true" | "false" | "none" ;
list_pattern    = "[" nl [ pattern nl { "," nl pattern nl }
                  [ "," nl ] ] "]" ;
tuple_pattern   = "(" nl pattern nl { "," nl pattern nl }
                  [ "," nl ] ")" ;

format_spec     = [ [ FILL ] ( "<" | ">" | "^" ) ] [ "+" | "-" | " " ]
                  [ DIGITS ] [ "." DIGITS ] [ format_type ] ;
format_type     = "d" | "b" | "o" | "x" | "X" | "f" | "F" | "e" | "E" | "%" ;
```

## Lexical and contextual constraints

* `IDENT` is `[A-Za-z_][A-Za-z_0-9]*`, excluding the 29 hard keywords in
  specification §3.3. Identifiers are case-sensitive and ASCII only. `int`,
  `float`, `bool`, and `string` are identifiers mapped to primitive types in
  type positions and reject type arguments. `none` is a hard keyword.
  A map type `{K: V}` is syntactically unconstrained in the key position; the
  semantic requirement that `K` be key-capable (`string`, `int`, `bool`, or a
  union of these) is enforced by the checker (specification §5.2).
* `UPPER_NAME` begins with `[A-Z]`; the rest may contain lowercase letters.
  `BIND_NAME` begins with lowercase ASCII or `_`. `UPPER_PATH` / `BIND_PATH`
  are paths classified by their **last** segment. `_` is the wildcard binding.
* `module`, `impl`, `trait`, and `const` are contextual identifiers at item
  position; `self` is contextual as a method's first parameter. `where` is an
  ordinary identifier and introduces no clause. `let` is always reserved.
  `module IDENT {` needs no capitalization. `trait` requires an uppercase
  name; `impl` requires an uppercase first head or a `::`-qualified head.
  Context recognition requires the header tokens without intervening NEWLINE.
* Generic declaration lists are nonempty and currently recognized by balanced
  single-line lookahead. Type arguments in type positions can contain `nl`;
  expression arguments require adjacent `<` and single-line balanced lookahead
  followed by `(` or `{`. A `>>` token splits into two `>` tokens only in
  generic positions; `>=` and `>>=` do not split. A generic head followed by
  `::` continues to a qualified variant (`Result<int, string>::Ok(1)`); the
  type arguments bind to the enum head, not the tag. No `where` clause exists.
* Explicitly parameterized uppercase calls produce `Expr::Call`; bare uppercase
  calls produce `Expr::Construct`. Resolution is subsequent to parsing.
* `INT` covers decimal and lowercase `0x`/`0b`/`0o` forms (§3.6.1). The special
  decimal magnitude 9223372036854775808 is valid only directly after unary `-`.
  It is not an accepted positive literal pattern. `FLOAT` requires an initial
  digit and a fraction and/or exponent (§3.6.2). There is no unary `+`.
* `STRING` uses either quote delimiter and the closed escape set in §3.6.3.
  `FSTRING` uses `f` or `F` and either quote, with raw literal text, `{{`/`}}`,
  and `{ expr [ ":" format_spec ] }`. Quotes matching the outer delimiter
  end the lexical string unless escaped. An interpolation respects nested
  delimiters, quoted text, comments and `::` paths. `FILL` is one character;
  `DIGITS` is one or more ASCII digits. Formatting width/precision safety
  bounds remain those already established in §31; this pass does not change them.
* The outer f-string quote rule, a lone literal `}`, and numeric underscore
  placement have incomplete normative definitions. Their observed behavior is
  recorded as SPEC GAP in the conformance report, not promoted to new syntax.
* Positional arguments precede named arguments in calls and method calls.
  Constructor arguments are parsed separately; binding and eligibility of named
  arguments belong to the checker. Named method arguments parse today but
  are rejected semantically (`E3001`), so they are not a missing token form.
* A small set of builtins take a **type argument** in a fixed position
  (currently only `json_decode_as(text, Type)`). That position is parsed with
  the canonical type grammar and produces an `Expr::TypeRef` node, not a value
  expression; the type is resolved once by the parser and never re-parsed at
  runtime. A string literal in that position is accepted as a compatibility
  spelling and normalized to the same node. Every other argument is an ordinary
  expression.
* Bare names, grouping, indexing and fields are expressions. `()` alone is
  invalid; `() -> e` is a lambda, `(x,)` is a one-element tuple. Assignment is
  a statement, not an expression. No function TypeExpr syntax,
  struct pattern, negative/float pattern, rest pattern or inclusive range exists.
* Collections: `[T]` is a List, `[T; N]` is a fixed-length Array (`N` a
  non-negative integer literal), `(T, U)` is a Tuple, `{T}` is a Set, `{K: V}`
  is a Map. A bracket literal `[a, b]` is a List unless an expected `[T; N]`
  makes it an Array (contextual realization, not conversion). `{a, b}` (a
  depth-0 comma, no depth-0 colon) is a Set; `{a}` and `{}` remain blocks;
  `set{}` is the empty Set.
* List and tuple patterns match exact length. `let` destructuring forbids `mut`,
  annotations and literals; unlike general patterns, it does not accept
  qualified variant paths.
* Newlines are accepted only in the explicit `nl` positions above. `else`,
  `catch`, and `finally` must directly follow the previous closing brace without
  NEWLINE. Operators, including pipelines, need their next operand on the same
  line. Multiline-comment internal newlines produce no NEWLINE token.
* `END_BOUNDARY` is a zero-width end before `}` or EOF, as established by the
  specification's inline examples. CONF-PARSE-8 is CLOSED: a real separator
  (newline) is required between statements and items, and the parser
  rejects absent separators between adjacent items/statements (`1 2`,
  `let x = 1 let y = 2`). The only zero-width end is `END_BOUNDARY` before `}`
  or EOF. A comment is whitespace, not a separator.
* Aura has **no general statement/item semicolon separator**: `;` is a reserved
  token and is rejected in every general separator position (`let a = 1; let b
  = 2` is `E1006`). Semicolon is reserved for an explicit collection/array
  grammar use and is not a sequencing operator.
* `pub impl`, local item declarations, import lists, repeated modifiers, default
  arguments, variadic parameters and method bodies inside traits are invalid.
  The module path separator is `::`; the historical dotted `use a.b` spelling is
  rejected. `use path as Alias` aliases an import.
* `&&` and `||` lex as two bitwise tokens and fail to parse. `!` alone is E1001.
  `^` is power, not XOR; `--x` consists of two unary minuses, not decrement.
* `#` comments run through the byte before LF or EOF. `<!--` comments close at
  the first `--!>`, do not nest and discard internal LF. Unterminated block
  comments are E1005. EOF is a real token; errors are `Result::Err(Diag)`, never
  an error token. See the conformance report for the complete token matrix.
