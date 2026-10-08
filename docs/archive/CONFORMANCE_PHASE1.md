# AURA LANGUAGE CONFORMANCE PASS — PHASE 1

## 1. Executive Result

The source-to-parser baseline has been reconstructed from implementation and
compared with the specification, formal grammar, guides, tests and history.
Confirmed contract violations are corrected, and a permanent conformance matrix
now exercises the affected surfaces. This is **not an unconditional clean
conformance certification**: separator policy, numeric separator placement,
several f-string edge rules and generic-head `::` continuation remain explicit
SPEC GAPs. No future syntax or TypeExpr nesting policy was selected.

The report distinguishes parsing from checking/execution. In particular, named
method arguments already have a parser representation; their restriction is
semantic. Similarly, duplicate declarations and many visibility errors are
not parser errors. Passing syntax tests does not certify those later layers.

The production changes concern strict source decoding, exact token/escape
spans, actual string closure, required pipeline operands, documented trailing
commas, interpolation token boundaries, return operands, map/block lookahead
and block separators. No checker, interpreter, standard-library, Python or
TypeExpr-limit redesign is included. Existing precision/rendering/equality/
bridge audits are not reopened.

## 2. Starting State

* Branch: `rewrite/v3-rust`.
* Starting HEAD: `f7c764f6b21ef895e2133d6cea397c543e736926`.
* Historical comparison baseline: `b2cf59f7aed307e8b74beaf79c717b939129aaf4`.
* Initial `git status --short`: empty. Branch, HEAD and 15-entry history
  matched the supplied expectations; no pre-existing tracked changes existed.
* Toolchain: pinned Rust 1.98.1; local Node 26.9.0; macOS native host.
* Both supplied artifact hashes and byte counts were independently verified.

Inventory was performed before editing: lexer/token enum, complete parser,
AST definitions, source-read boundaries, grammar/specification/contract,
roadmap, RFC index, generics/OOP references, website syntax guides, grammar,
lexer/parser/module/trait/generic/contract tests, corpus policy and inventory,
fuzz targets/seeds, artifact builder and CI workflow. The report does not treat
historical audit conclusions as proof. `docs/rfcs/` contains only its index,
not an accepted numbered RFC package.

## 3. Documentation Authority Model

At the starting revision, `LANGUAGE_SPEC.md` explicitly declared itself
normative and authoritative over other documents; §4 explicitly covers syntax.
`contract.md` already deferred to it. `GENERICS.md` also deferred to it.
However, `grammar.md` said the parser was always wrong on disagreement, while
`CONTRIBUTING.md` named only the contract as normative. CONF-DOC-1 records this
conflicting guidance; the hierarchy is now explicit:

1. `LANGUAGE_SPEC.md`: normative current syntax and semantics.
2. `grammar.md`: formal representation, with explicit unresolved discrepancies.
3. `contract.md`: compatibility promises, reconciled with the specification.
4. Guides and website references: explanatory; the website grammar is a manual
   duplicate guarded by a byte-for-byte test, not independently normative.
5. Roadmap and historical reports: current delivery status, future proposals
   and historical evidence, clearly separated.
6. RFCs: proposed/accepted design changes; proposals alone are not current syntax.

The website build generates HTML from content files; it does not generate the
Markdown grammar from the repository grammar. No website design was changed.
The existing claim that tests “parse every production” was not a mechanical
EBNF proof. It is replaced by a precise statement of coverage and sync checks.

### Document inventory

| Source | Role established or recorded |
|---|---|
| `CONTRIBUTING.md` | guide / workflow |
| `README.md` | guide / workflow |
| `docs/ARCHITECTURE_REVIEW.md` | historical design / audit evidence |
| `docs/AUDIT3_TYPE_NESTING_DECISION.md` | decision package (protected) |
| `docs/CORRECTIONS.md` | historical design / audit evidence |
| `docs/FEATURE_001_CODE_REVIEW.md` | historical design / audit evidence |
| `docs/FEATURE_001_DESIGN.md` | historical design / audit evidence |
| `docs/FEATURE_001_IMPLEMENTATION_REPORT.md` | historical design / audit evidence |
| `docs/FEATURE_002_CODE_REVIEW.md` | historical design / audit evidence |
| `docs/FEATURE_002_DESIGN.md` | historical design / audit evidence |
| `docs/FEATURE_002_IMPLEMENTATION_REPORT.md` | historical design / audit evidence |
| `docs/FEATURE_003_CODE_REVIEW.md` | historical design / audit evidence |
| `docs/FEATURE_003_DESIGN.md` | historical design / audit evidence |
| `docs/FEATURE_003_IMPLEMENTATION_REPORT.md` | historical design / audit evidence |
| `docs/FEATURE_004_DESIGN.md` | historical design / audit evidence |
| `docs/FEATURE_004_IMPLEMENTATION_REPORT.md` | historical design / audit evidence |
| `docs/FEATURE_005_DESIGN.md` | historical design / audit evidence |
| `docs/FEATURE_005_IMPLEMENTATION_REPORT.md` | historical design / audit evidence |
| `docs/FEATURE_006_DESIGN.md` | historical design / audit evidence |
| `docs/FEATURE_006_IMPLEMENTATION_REPORT.md` | historical design / audit evidence |
| `docs/FEATURE_H1_PATTERN_HARDENING_REPORT.md` | historical design / audit evidence |
| `docs/FEATURE_ROADMAP.md` | roadmap with historical design material |
| `docs/FINAL_SEMANTIC_RED_TEAM_REPORT.md` | historical design / audit evidence |
| `docs/GENERICS.md` | architecture reference, defers to specification |
| `docs/LANGUAGE_SPEC.md` | normative syntax and semantics |
| `docs/LANGUAGE_SPEC_CONFORMANCE_REPORT.md` | historical design / audit evidence |
| `docs/OOP.md` | architecture reference, defers to specification |
| `docs/POST_FEATURE_002_STACK_AUDIT.md` | historical design / audit evidence |
| `docs/PRE_MODULES_AUDIT.md` | historical design / audit evidence |
| `docs/RELEASE_0_0_X_DESIGN.md` | historical design / audit evidence |
| `docs/SEMANTIC_CLOSURE_REPORT.md` | historical design / audit evidence |
| `docs/SEMANTIC_FREEZE_AUDIT.md` | historical design / audit evidence |
| `docs/ZEN.md` | guide / workflow |
| `docs/contract.md` | compatibility contract, defers to specification |
| `docs/errors.md` | guide / workflow |
| `docs/grammar.md` | canonical EBNF |
| `docs/playground.md` | guide / workflow |
| `docs/rfcs/README.md` | RFC process (no accepted RFC files present) |
| `website/content/cli.md` | guide / manually duplicated reference |
| `website/content/first-program.md` | guide / manually duplicated reference |
| `website/content/getting-started.md` | guide / manually duplicated reference |
| `website/content/guide-basics.md` | guide / manually duplicated reference |
| `website/content/guide-collections.md` | guide / manually duplicated reference |
| `website/content/guide-control.md` | guide / manually duplicated reference |
| `website/content/guide-data.md` | guide / manually duplicated reference |
| `website/content/guide-errors.md` | guide / manually duplicated reference |
| `website/content/guide-functions.md` | guide / manually duplicated reference |
| `website/content/guide-generics.md` | guide / manually duplicated reference |
| `website/content/guide-io.md` | guide / manually duplicated reference |
| `website/content/guide-matching.md` | guide / manually duplicated reference |
| `website/content/guide-modules.md` | guide / manually duplicated reference |
| `website/content/guide-oop.md` | guide / manually duplicated reference |
| `website/content/install.md` | guide / manually duplicated reference |
| `website/content/playground-doc.md` | guide / manually duplicated reference |
| `website/content/reference-errors.md` | guide / manually duplicated reference |
| `website/content/reference-grammar.md` | guide / manually duplicated reference |
| `website/content/reference-limits.md` | guide / manually duplicated reference |
| `website/content/reference-operators.md` | guide / manually duplicated reference |
| `website/content/reference-stdlib.md` | guide / manually duplicated reference |
| `website/content/reference-types.md` | guide / manually duplicated reference |
| `website/content/repl.md` | guide / manually duplicated reference |
| `website/content/runtime-doc.md` | guide / manually duplicated reference |
| `website/content/zen.md` | guide / manually duplicated reference |

Generated outputs (`website/dist`, target directories), frozen release notes,
feature implementation reports and previous audit reports are not silently
rewritten to describe the new development revision. The protected AUDIT-3
package is unchanged.

## 4. Source-Text Contract

`lex`, `lex_at`, `parse`, `parse_expr` and `parse_stmt` accept `&str`. Parser
entry points copy into a Rust String before invoking the execution substrate.
The lexer then scans UTF-8 bytes. Invalid UTF-8 cannot exist in these APIs.
The CLI previously used `read_to_string` (file rejection without an E-code,
stdin rejection without a diagnostic); the WASM ABI and its native harness
used replacement decoding. They now share strict `lex::decode_source` through
the relevant host boundary. Invalid bytes anywhere in source produce E1001.
The browser encodes JavaScript strings with TextEncoder; its handling of
unpaired UTF-16 surrogates happens before Aura's byte boundary.

No encoding other than UTF-8 is supported, and there is no Unicode identifier
normalization. Identifiers are ASCII, case-sensitive, byte-exact. Canonically
equivalent Unicode strings remain different scalar sequences; neither `café`
nor `cafe\u0301` is a legal identifier, so this is not a pair of distinct legal
identifier spellings. Confusables from non-ASCII scripts are rejected outside
string/comment text.

In the table, “covered” means TEST GAP closed by
`source_text_and_identifiers`, `numeric_edges`, `comments_and_newlines`,
`byte_decoder_rejects_invalid_utf8_everywhere`, the CLI test and/or the Node
syntax suite. The source observations are documented without silently making
an unspecified edge a new normative feature.

| Input / position | Established contract or expectation | Final observed result / diagnostic and location | Classification/status |
|---|---|---|---|
| Empty bytes / empty `&str` | A file is zero or more items | Empty module; expression/statement entry points need a construct | TEST GAP, covered |
| ASCII space/tab/CR only | §3.4 insignificant whitespace | Empty module; EOF at input length | TEST GAP, covered |
| LF | §3.4 significant newline | Newline token, then EOF; empty module | TEST GAP, covered |
| `1\n` / `1\r\n` | CR ignored; LF terminates | Same AST; LF span 1..2 / 2..3 | TEST GAP, covered |
| `1\r` | CR is not LF | Int then EOF, no Newline | TEST GAP, covered |
| `# a\rb\n1` | Line comment ends at LF | `b` remains in comment; LF then Int | TEST GAP, covered |
| No final newline (`1`) | Existing inline/EOF examples | Expression accepted, EOF at 1..1 | TEST GAP, covered |
| Comments-only, including Unicode/NUL in comment | §3.5 discarded | Empty module | TEST GAP, covered |
| Invalid UTF-8 `FF`, inside quotes or comments, truncated multibyte sequence | §3.1 requires UTF-8 | E1001 before lexing at `valid_up_to..invalid_end`; no replacement accepted | PLATFORM DIVERGENCE, CONF-LEX-4 fixed |
| UTF-8 BOM at byte 0 | No BOM stripping in implementation | E1001 at 0..1 (first byte) | TEST GAP, observed/documented |
| BOM inside string/comment | Content not identifier syntax | Preserved in string / discarded comment | TEST GAP, lexical rule |
| NBSP U+00A0 outside literal | Not space/tab/CR/LF | E1001 at first UTF-8 byte | TEST GAP, covered |
| Zero-width U+200B / U+FEFF | Not an identifier/whitespace character | E1001 | TEST GAP, covered |
| U+2028 | Not LF | E1001 outside literal | TEST GAP, covered |
| NUL, VT, FF, DEL outside literal/comment | No corresponding token | E1001 | TEST GAP, covered |
| NUL, CR and other non-LF controls inside a plain string | String body content; `\0`/`\r` also supported | Preserved | TEST GAP, covered |
| `aura`, `Aura`, `_x`, `x1` | ASCII identifier rule | Ident, spellings distinct | TEST GAP, covered |
| `1x` | §3.6.1 number/name boundary | E1002, span 0..2 | TEST GAP, covered |
| `café`, decomposed `café`, `λ`, `变量`, emoji | Non-ASCII identifiers forbidden | E1001 at byte 3 / 4 / 0 / 0 / 0 | TEST GAP, covered |
| `pub`, `type`, all 29 KEYWORDS | Reserved | Dedicated tokens; name positions reject | TEST GAP, covered |
| `module`, `impl`, `trait`, `const`, `self` | Contextual | Ident tokens; parser recognizes dedicated contexts | GRAMMAR DRIFT, aligned |
| `where` | No where-clause syntax | Ordinary Ident | TEST GAP, covered |
| Unicode string/comment content | UTF-8 source text | Preserved / discarded, no normalization | TEST GAP, covered |
| Unescaped LF inside either quoted string form | §3.6.3–4 | E1004 before LF; CRLF likewise fails at LF | IMPLEMENTATION BUG closure edge fixed |
| Backslash + LF | Escape set / raw f-string distinction | Plain string E1003; f-string raw text can contain escaped LF | SPEC GAP on broader continuation policy; no new continuation |

Spans are half-open **byte** ranges. After the correction they exclude leading
trivia, and `lex_at` adds its base to every escape diagnostic. Invalid-character
spans can cover just the first byte of a multibyte scalar; consumers should not
assume the end is a Rust character boundary. User-facing `line_col` counts LF
and Unicode scalar columns at the valid start. EOF spans are zero-width.
There is no parser recovery: the first deterministic `Diag` is returned.

### Numeric contract

* Decimal, lowercase `0x`, `0b`, `0o`; no unary plus. Minus is its own token.
* `0`, leading zeros and `_` in decimal/radix digit runs are currently accepted.
  `1__0_` → 10 and `0x_f_f_` → 255. Exact underscore-placement policy is not
  normatively specified (CONF-LEX-6); it was not tightened.
* Floats have an initial decimal digit, fraction only if `.` is followed by a
  digit, and/or exponent `e`/`E` with optional sign and required digits.
  Exponent underscores reject: `1e1_0` is E1002. `1e309` is positive infinity.
* `.5` is Dot + Int and fails as an expression; `1.` is Int + Dot and fails
  without a member name; `1..2` is Int + DotDot + Int. `1...2` fails parsing.
* i64 maximum is accepted. Decimal maximum+1 lexes as IntMinMagnitude, accepted
  only directly under unary minus. The positive magnitude fails parsing E1002;
  `-(9223372036854775808)` is invalid. Larger decimal values and out-of-range
  radix values fail lexing E1002. No checker/runtime widening is involved.
* Empty radix digits, invalid radix digits, missing exponent digits and ASCII
  name adjacency are E1002. Non-ASCII adjacency ultimately fails E1001.

## 5. Lexer and Token Inventory

There are **79 Tok variants**, including the special minimum-integer magnitude,
Newline and EOF. `tests/syntax_tokens.tsv` enumerates every variant and concrete
spelling; its test checks the result and fails if enum variants are omitted.
This is separate from the parser-family tests: token production alone is not
proof of syntax acceptance.

The matrix was assembled from the token enum/Display spellings, actual lexer
branches, parser token references, specification and grammar. Implementations
below are in `src/lex/mod.rs`; parser consumer names are in `src/parse/mod.rs`.
The named test is in `tests/syntax_docs.rs`; value/error assertions also live in
`tests/lexer.rs` and `tests/syntax_conformance.rs`.

| Token | Concrete spelling | Lexer implementation | Parser consumers | Spec | Grammar | Permanent token case | Status |
|---|---|---|---|---|---|---|---|
| `Int` | `123` | `lex::number` | `let_pattern_inner`, `pattern_inner`, `atom`, `starts_expr` | §3.6.1–2 | INT / FLOAT | `every_token_variant_has_a_lexical_case` | reconciled |
| `IntMinMagnitude` | `9223372036854775808` | `lex::number` | `unary`, `atom` | §3.6.1–2 | INT / FLOAT | `every_token_variant_has_a_lexical_case` | reconciled |
| `Float` | `1.5` | `lex::number` | `atom`, `starts_expr` | §3.6.1–2 | INT / FLOAT | `every_token_variant_has_a_lexical_case` | reconciled |
| `Str` | `"x"` | `lex::string / ident → raw_string_body` | `let_pattern_inner`, `pattern_inner`, `atom`, `starts_expr` | §3.6.3–4 | STRING / FSTRING | `every_token_variant_has_a_lexical_case` | reconciled |
| `FStr` | `f"{x}"` | `lex::string / ident → raw_string_body` | `atom`, `starts_expr` | §3.6.3–4 | STRING / FSTRING | `every_token_variant_has_a_lexical_case` | reconciled |
| `True` | `true` | `lex::ident` | `let_pattern_inner`, `pattern_inner`, `atom`, `is_keyword`, `starts_expr` | §3.2–3, §4 | lexical constraints / item, stmt, expr | `every_token_variant_has_a_lexical_case` | reconciled |
| `False` | `false` | `lex::ident` | `let_pattern_inner`, `pattern_inner`, `atom`, `is_keyword`, `starts_expr` | §3.2–3, §4 | lexical constraints / item, stmt, expr | `every_token_variant_has_a_lexical_case` | reconciled |
| `None` | `none` | `lex::ident` | `ty_member`, `let_pattern_inner`, `pattern_inner`, `atom`, `is_keyword`, `starts_expr` | §3.2–3, §4 | lexical constraints / item, stmt, expr | `every_token_variant_has_a_lexical_case` | reconciled |
| `Ident` | `name` | `lex::ident` | `at_module_block`, `at_impl_block`, `at_trait_block`, `at_const_decl`, `params`, `ident`, `ty_member`, `let_pattern_inner`, `pattern_inner`, `atom`, `cons_arg`, `try_lambda_params`, `starts_expr` | §3.2–3, §4 | lexical constraints / item, stmt, expr | `every_token_variant_has_a_lexical_case` | reconciled |
| `Let` | `let` | `lex::ident` | `item`, `stmt_inner`, `map_ahead`, `is_keyword` | §3.2–3, §4 | lexical constraints / item, stmt, expr | `every_token_variant_has_a_lexical_case` | reconciled |
| `Mut` | `mut` | `lex::ident` | `params`, `const_item`, `stmt_inner`, `atom`, `try_lambda_params`, `is_keyword` | §3.2–3, §4 | lexical constraints / item, stmt, expr | `every_token_variant_has_a_lexical_case` | reconciled |
| `Fn` | `fn` | `lex::ident` | `item`, `impl_item`, `trait_item`, `atom`, `is_keyword`, `starts_expr` | §3.2–3, §4 | lexical constraints / item, stmt, expr | `every_token_variant_has_a_lexical_case` | reconciled |
| `If` | `if` | `lex::ident` | `atom`, `is_keyword`, `starts_expr` | §3.2–3, §4 | lexical constraints / item, stmt, expr | `every_token_variant_has_a_lexical_case` | reconciled |
| `Else` | `else` | `lex::ident` | `atom`, `is_keyword` | §3.2–3, §4 | lexical constraints / item, stmt, expr | `every_token_variant_has_a_lexical_case` | reconciled |
| `Match` | `match` | `lex::ident` | `atom`, `is_keyword`, `starts_expr` | §3.2–3, §4 | lexical constraints / item, stmt, expr | `every_token_variant_has_a_lexical_case` | reconciled |
| `While` | `while` | `lex::ident` | `stmt_inner`, `map_ahead`, `is_keyword` | §3.2–3, §4 | lexical constraints / item, stmt, expr | `every_token_variant_has_a_lexical_case` | reconciled |
| `Loop` | `loop` | `lex::ident` | `stmt_inner`, `map_ahead`, `is_keyword` | §3.2–3, §4 | lexical constraints / item, stmt, expr | `every_token_variant_has_a_lexical_case` | reconciled |
| `For` | `for` | `lex::ident` | `at_impl_block`, `impl_item`, `stmt_inner`, `map_ahead`, `is_keyword` | §3.2–3, §4 | lexical constraints / item, stmt, expr | `every_token_variant_has_a_lexical_case` | reconciled |
| `In` | `in` | `lex::ident` | `stmt_inner`, `is_keyword` | §3.2–3, §4 | lexical constraints / item, stmt, expr | `every_token_variant_has_a_lexical_case` | reconciled |
| `Return` | `return` | `lex::ident` | `stmt_inner`, `map_ahead`, `is_keyword` | §3.2–3, §4 | lexical constraints / item, stmt, expr | `every_token_variant_has_a_lexical_case` | reconciled |
| `Break` | `break` | `lex::ident` | `stmt_inner`, `map_ahead`, `is_keyword` | §3.2–3, §4 | lexical constraints / item, stmt, expr | `every_token_variant_has_a_lexical_case` | reconciled |
| `Continue` | `continue` | `lex::ident` | `stmt_inner`, `map_ahead`, `is_keyword` | §3.2–3, §4 | lexical constraints / item, stmt, expr | `every_token_variant_has_a_lexical_case` | reconciled |
| `Use` | `use` | `lex::ident` | `item`, `is_keyword` | §3.2–3, §4 | lexical constraints / item, stmt, expr | `every_token_variant_has_a_lexical_case` | reconciled |
| `As` | `as` | `lex::ident` | `use_item`, `is_keyword` | §3.2–3, §4 | lexical constraints / item, stmt, expr | `every_token_variant_has_a_lexical_case` | reconciled |
| `Struct` | `struct` | `lex::ident` | `item`, `is_keyword` | §3.2–3, §4 | lexical constraints / item, stmt, expr | `every_token_variant_has_a_lexical_case` | reconciled |
| `Enum` | `enum` | `lex::ident` | `item`, `is_keyword` | §3.2–3, §4 | lexical constraints / item, stmt, expr | `every_token_variant_has_a_lexical_case` | reconciled |
| `Type` | `type` | `lex::ident` | `item`, `is_keyword` | §3.2–3, §4 | lexical constraints / item, stmt, expr | `every_token_variant_has_a_lexical_case` | reconciled |
| `And` | `and` | `lex::ident` | `infix`, `is_keyword` | §3.2–3, §4 | lexical constraints / item, stmt, expr | `every_token_variant_has_a_lexical_case` | reconciled |
| `Or` | `or` | `lex::ident` | `infix`, `is_keyword` | §3.2–3, §4 | lexical constraints / item, stmt, expr | `every_token_variant_has_a_lexical_case` | reconciled |
| `Not` | `not` | `lex::ident` | `unary`, `is_keyword`, `starts_expr` | §3.2–3, §4 | lexical constraints / item, stmt, expr | `every_token_variant_has_a_lexical_case` | reconciled |
| `Try` | `try` | `lex::ident` | `stmt_inner`, `map_ahead`, `is_keyword` | §3.2–3, §4 | lexical constraints / item, stmt, expr | `every_token_variant_has_a_lexical_case` | reconciled |
| `Catch` | `catch` | `lex::ident` | `stmt_inner`, `is_keyword` | §3.2–3, §4 | lexical constraints / item, stmt, expr | `every_token_variant_has_a_lexical_case` | reconciled |
| `Finally` | `finally` | `lex::ident` | `stmt_inner`, `is_keyword` | §3.2–3, §4 | lexical constraints / item, stmt, expr | `every_token_variant_has_a_lexical_case` | reconciled |
| `Throw` | `throw` | `lex::ident` | `stmt_inner`, `map_ahead`, `is_keyword` | §3.2–3, §4 | lexical constraints / item, stmt, expr | `every_token_variant_has_a_lexical_case` | reconciled |
| `Pub` | `pub` | `lex::ident` | `item`, `impl_item`, `trait_item`, `struct_item`, `is_keyword` | §3.2–3, §4 | lexical constraints / item, stmt, expr | `every_token_variant_has_a_lexical_case` | reconciled |
| `LParen` | `(` | `lex::punct` | `at_type_args`, `fn_item`, `trait_method_decl`, `method_item`, `enum_item`, `let_pattern_inner`, `pattern_inner`, `postfix`, `construct_positional`, `atom`, `map_ahead`, `starts_expr` | §4, §36 | productions (literal spelling) | `every_token_variant_has_a_lexical_case` | reconciled |
| `RParen` | `)` | `lex::punct` | `params`, `enum_item`, `let_pattern_inner`, `pattern_inner`, `construct_positional`, `call_args`, `atom`, `try_lambda_params`, `map_ahead` | §4, §36 | productions (literal spelling) | `every_token_variant_has_a_lexical_case` | reconciled |
| `LBracket` | `[` | `lex::punct` | `ty_member`, `let_pattern_inner`, `pattern_inner`, `postfix`, `atom`, `map_ahead`, `starts_expr` | §4, §36 | productions (literal spelling) | `every_token_variant_has_a_lexical_case` | reconciled |
| `RBracket` | `]` | `lex::punct` | `ty_member`, `let_pattern_inner`, `pattern_inner`, `postfix`, `atom`, `map_ahead` | §4, §36 | productions (literal spelling) | `every_token_variant_has_a_lexical_case` | reconciled |
| `LBrace` | `{` | `lex::punct` | `at_module_block`, `module_item`, `at_impl_block`, `at_trait_block`, `at_type_args`, `impl_item`, `trait_item`, `trait_method_decl`, `struct_item`, `enum_item`, `ty_member`, `block`, `construct_fields`, `atom`, `map_ahead`, `starts_expr` | §4, §36 | productions (literal spelling) | `every_token_variant_has_a_lexical_case` | reconciled |
| `RBrace` | `}` | `lex::punct` | `module_item`, `impl_item`, `trait_item`, `struct_item`, `enum_item`, `ty_member`, `block`, `construct_fields`, `atom`, `map_ahead` | §4, §36 | productions (literal spelling) | `every_token_variant_has_a_lexical_case` | reconciled |
| `Comma` | `,` | `lex::punct` | `opt_type_params`, `ty_args`, `decl_type_args`, `params`, `struct_item`, `enum_item`, `ty_member`, `let_pattern_inner`, `pattern_inner`, `construct_fields`, `construct_positional`, `call_args`, `atom`, `try_lambda_params` | §4, §36 | productions (literal spelling) | `every_token_variant_has_a_lexical_case` | reconciled |
| `Semi` | `;` | `lex::punct` | `end_stmt`, `at_const_decl`, `balanced_angles_from`, `block`, `atom`, `map_ahead` | §4, §36 | productions (literal spelling) | `every_token_variant_has_a_lexical_case` | reconciled |
| `Colon` | `:` | `lex::punct` | `at_const_decl`, `const_decl_item`, `opt_type_params`, `params`, `struct_item`, `const_item`, `ty_member`, `stmt_inner`, `construct_fields`, `atom`, `cons_arg`, `try_lambda_params`, `map_ahead` | §4, §36 | productions (literal spelling) | `every_token_variant_has_a_lexical_case` | reconciled |
| `ColonColon` | `::` | `lex::punct` | `at_impl_block`, `at_type_args`, `use_item`, `path_segments`, `ty_member`, `pattern_inner`, `atom` | §4, §36 | productions (literal spelling) | `every_token_variant_has_a_lexical_case` | reconciled |
| `Dot` | `.` | `lex::punct` | `use_item`, `postfix` | §4, §36 | productions (literal spelling) | `every_token_variant_has_a_lexical_case` | reconciled |
| `DotDot` | `..` | `lex::punct` | `expr_bp` | §4, §36 | productions (literal spelling) | `every_token_variant_has_a_lexical_case` | reconciled |
| `Arrow` | `->` | `lex::punct` | `opt_return_ty`, `atom`, `try_lambda_params` | §4, §36 | productions (literal spelling) | `every_token_variant_has_a_lexical_case` | reconciled |
| `Assign` | `=` | `lex::punct` | `at_const_decl`, `const_decl_item`, `alias_item`, `const_item`, `stmt_inner` | §4, §36 | productions (literal spelling) | `every_token_variant_has_a_lexical_case` | reconciled |
| `Plus` | `+` | `lex::punct` | `opt_type_params`, `infix` | §4, §36 | productions (literal spelling) | `every_token_variant_has_a_lexical_case` | reconciled |
| `Minus` | `-` | `lex::punct` | `unary`, `infix`, `starts_expr` | §4, §36 | productions (literal spelling) | `every_token_variant_has_a_lexical_case` | reconciled |
| `Star` | `*` | `lex::punct` | `infix` | §4, §36 | productions (literal spelling) | `every_token_variant_has_a_lexical_case` | reconciled |
| `Slash` | `/` | `lex::punct` | `infix` | §4, §36 | productions (literal spelling) | `every_token_variant_has_a_lexical_case` | reconciled |
| `Percent` | `%` | `lex::punct` | `infix` | §4, §36 | productions (literal spelling) | `every_token_variant_has_a_lexical_case` | reconciled |
| `Caret` | `^` | `lex::punct` | `infix` | §4, §36 | productions (literal spelling) | `every_token_variant_has_a_lexical_case` | reconciled |
| `EqEq` | `==` | `lex::punct` | `infix` | §4, §36 | productions (literal spelling) | `every_token_variant_has_a_lexical_case` | reconciled |
| `Ne` | `!=` | `lex::punct` | `infix` | §4, §36 | productions (literal spelling) | `every_token_variant_has_a_lexical_case` | reconciled |
| `Lt` | `<` | `lex::punct` | `at_impl_block`, `at_trait_block`, `at_type_params`, `balanced_angles_from`, `at_type_args`, `decl_type_args`, `ty_member`, `infix` | §4, §36 | productions (literal spelling) | `every_token_variant_has_a_lexical_case` | reconciled |
| `Le` | `<=` | `lex::punct` | `infix` | §4, §36 | productions (literal spelling) | `every_token_variant_has_a_lexical_case` | reconciled |
| `Gt` | `>` | `lex::punct` | `balanced_angles_from`, `expect_close_angle`, `opt_type_params`, `ty_args`, `decl_type_args`, `ty_member`, `infix` | §4, §36 | productions (literal spelling) | `every_token_variant_has_a_lexical_case` | reconciled |
| `Ge` | `>=` | `lex::punct` | `infix` | §4, §36 | productions (literal spelling) | `every_token_variant_has_a_lexical_case` | reconciled |
| `Pipe` | `\|>` | `lex::punct` | `infix` | §4, §36 | productions (literal spelling) | `every_token_variant_has_a_lexical_case` | reconciled |
| `Bar` | `\|` | `lex::punct` | `ty_inner`, `infix` | §4, §36 | productions (literal spelling) | `every_token_variant_has_a_lexical_case` | reconciled |
| `Amp` | `&` | `lex::punct` | `infix` | §4, §36 | productions (literal spelling) | `every_token_variant_has_a_lexical_case` | reconciled |
| `Tilde` | `~` | `lex::punct` | `unary`, `starts_expr` | §4, §36 | productions (literal spelling) | `every_token_variant_has_a_lexical_case` | reconciled |
| `Shl` | `<<` | `lex::punct` | `balanced_angles_from`, `infix` | §4, §36 | productions (literal spelling) | `every_token_variant_has_a_lexical_case` | reconciled |
| `Shr` | `>>` | `lex::punct` | `balanced_angles_from`, `expect_close_angle`, `opt_type_params`, `ty_args`, `decl_type_args`, `ty_member`, `infix` | §4, §36 | productions (literal spelling) | `every_token_variant_has_a_lexical_case` | reconciled |
| `PlusEq` | `+=` | `lex::punct` | `stmt_inner` | §4, §36 | productions (literal spelling) | `every_token_variant_has_a_lexical_case` | reconciled |
| `MinusEq` | `-=` | `lex::punct` | `stmt_inner` | §4, §36 | productions (literal spelling) | `every_token_variant_has_a_lexical_case` | reconciled |
| `StarEq` | `*=` | `lex::punct` | `stmt_inner` | §4, §36 | productions (literal spelling) | `every_token_variant_has_a_lexical_case` | reconciled |
| `SlashEq` | `/=` | `lex::punct` | `stmt_inner` | §4, §36 | productions (literal spelling) | `every_token_variant_has_a_lexical_case` | reconciled |
| `PercentEq` | `%=` | `lex::punct` | `stmt_inner` | §4, §36 | productions (literal spelling) | `every_token_variant_has_a_lexical_case` | reconciled |
| `CaretEq` | `^=` | `lex::punct` | `stmt_inner` | §4, §36 | productions (literal spelling) | `every_token_variant_has_a_lexical_case` | reconciled |
| `BarEq` | `\|=` | `lex::punct` | `stmt_inner` | §4, §36 | productions (literal spelling) | `every_token_variant_has_a_lexical_case` | reconciled |
| `AmpEq` | `&=` | `lex::punct` | `stmt_inner` | §4, §36 | productions (literal spelling) | `every_token_variant_has_a_lexical_case` | reconciled |
| `ShlEq` | `<<=` | `lex::punct` | `stmt_inner` | §4, §36 | productions (literal spelling) | `every_token_variant_has_a_lexical_case` | reconciled |
| `ShrEq` | `>>=` | `lex::punct` | `stmt_inner` | §4, §36 | productions (literal spelling) | `every_token_variant_has_a_lexical_case` | reconciled |
| `Newline` | `\n` | `lex::next_token` | `skip_newlines`, `end_stmt`, `at_const_decl`, `balanced_angles_from`, `block`, `expr_bp`, `map_ahead` | §3.4, §3.7 | nl / terminator | `every_token_variant_has_a_lexical_case` | reconciled |
| `Eof` | `(empty)` | `lex::next_token` | `expect_eof`, `module`, `module_item`, `at_const_decl`, `balanced_angles_from`, `impl_item`, `trait_item`, `block`, `expr_bp` | §2, §4.1 | file | `every_token_variant_has_a_lexical_case` | reconciled |

| Non-token category | Spelling | Implementation | Parser use | Spec / grammar | Tests | Status |
|---|---|---|---|---|---|---|
| Line comment | `#…LF/EOF` | `skip_trivia` | none; LF retained | §3.5 / lexical constraints | comments tests | Reconciled |
| Block comment | `<!-- … --!>` | `multiline_comment` | none; internal LF discarded | §3.5 / lexical constraints | comments tests, existing corpus | Reconciled; non-nesting |
| Error | no Tok variant | `Result::Err(Diag)` | stops before parsing | §2 / lexical constraints | malformed cases | Reconciled |
| Contextual words | module, impl, trait, const, self | Ident | item lookahead / receiver | §3.3, §4, §17, §28 | grammar_family_matrix | Reconciled |
| Ordinary type words | int, float, bool, string | Ident | `ty_member` maps primitives | §4.3 | type matrix | Reconciled |
| `&&`, `||`, `++`, `--` | pairs of existing tokens | `punct` | no combined operators | §4.5 | existing operators tests | Deliberate absence; `--x` is two unary minuses |

`fn`, `mut`, `use`, `pub`, `type`, `match`, `if`, `else`, `for`, `while`,
`try`, `catch`, `finally` are hard keywords. `module`, `trait`, `impl`,
`where` are not. `const` and `self` are also non-reserved. There is no
Unicode name class, comment token, error token, `where` token, tuple token,
function-type token or separate generic-bracket token.

## 6. Grammar Reconstruction

The implementation hierarchy is:

* `parse` → lex → `module` → repeated `item`: optional visibility, contextual
  module/const/trait/impl lookahead, then function/struct/enum/alias/use/top-let
  declarations or expressions. Nested modules recursively contain items.
* `block` → repeated statements: let/binding or destructuring, assignment,
  expression, return/throw/break/continue, while/loop/for, try/catch/finally.
  Item declarations are not statements; named nested functions are invalid.
* `expr` → Pratt `expr_bp` → recursive unary → postfix → atom. `if`, `match`,
  lambdas, collection literals and blocks are expression atoms. Pipelines
  desugar to calls/methods or retain an Expr::Pipe for a callable expression.
* Postfix loops add calls, index reads, fields and methods. Qualified paths
  join `::` segments. Uppercase last segments select bare constructors;
  generic call syntax has its own explicit-argument branch.
* `ty` → union of `ty_member`: primitive, named/qualified/application, list or
  map. There is no function, tuple, nullable-`?`, reference or lifetime syntax.
  All existing type-depth accounting and native/WASM backstops are unchanged.
* `pattern` accepts integer/string/bool/none, binding/wildcard, exact lists and
  variants, including qualified variants. `let_pattern` is the restricted
  no-literal form and does not consume qualified paths.
* F-string token contents are parsed into raw literal parts and expression /
  format-spec parts. Quote-aware/comment-aware brace scanning and `::` handling
  now preserve real expression boundaries. Outer quote handling remains lexical.

`docs/grammar.md` contains the full production map, including lexical and
contextual side conditions rather than pretending a context-free excerpt
captures capitalization, generic lookahead or semantic named-argument rules.
Its excerpts are synchronized back into normative §4 and the website.

### Delimiter and trailing-form matrix

| List / context | Empty | Trailing comma | LF layout | Consumer / coverage |
|---|---|---|---|---|
| Call and method args | yes | optional | between elements/delimiters | call_args; delimiter matrix |
| Function/lambda params | yes | optional | between elements/delimiters, corrected | params / try_lambda_params |
| Method/trait params | no (self required) | optional, including self alone | same as params | params(true) |
| Generic params | no | optional | lookahead requires single-line header | opt_type_params |
| Generic args, type position | no | optional, including nested `>>` | yes around arguments | ty_member / decl_type_args |
| Generic args, expression position | no | optional | single-line lookahead; adjacent `<` | ty_args / at_type_args |
| List literal / list pattern / let-list pattern | yes | optional | yes at element boundaries, corrected after final comma | atom / pattern / let_pattern |
| Parenthesized list sugar | no | optional; `(x,)` one element | after first comma and later elements | atom |
| Group `(expr)` | no | comma changes it into list sugar | no general expression continuation | atom |
| Map literal | `{:}` only | optional for nonempty maps | yes between entries | map_ahead / atom |
| Struct fields / initializers | yes | optional | yes | struct_item / construct_fields |
| Enum variants | yes | optional | yes | enum_item |
| Enum payload types / variant pattern payloads | yes | optional, corrected | yes, corrected | enum_item / pattern / let_pattern |
| Match arms | yes | optional comma or semicolon | LF between arms | atom Match |
| Import list | no such construct | n/a | no multiline use path | use_item |
| Trait/impl members | yes | comma forbidden between members | LF between members | trait_item / impl_item |
| Type union | no | comma not a union separator | no general LF around bar | ty_inner |

Empty separators without elements (`f(,)`, `[1,,]`, `S {,}`, `{:,}`) remain
invalid. A closing delimiter does not imply universal newline continuation:
for example an operator followed by LF still fails, including inside a list.
Qualified generic-head continuation, nested function-type annotations and
rest patterns are not added by these fixes.

## 7. Specification ↔ Grammar ↔ Parser Matrix

Tests abbreviated SC are `tests/syntax_conformance.rs`; SD is
`tests/syntax_docs.rs`. All rows are parser syntax unless marked later phase.
The website grammar covers every current production below; guide columns
name additional explanatory sources. “Aligned” includes the confirmed
corrections recorded in §9, not a claim that the starting files agreed.

| Construct | Implementation syntax / consumer | LANGUAGE_SPEC | grammar.md production | Website guide/reference | Coverage | Status |
|---|---|---|---|---|---|---|
| File/root | repeated items, LF, EOF / module | §4.1 | file, item | reference-grammar | SC roots/text; grammar.rs | aligned; separators pending |
| Source module | `module IDENT { items }` | §§4.1,27–28 | module_decl | guide-modules | SC families; modules.rs | omission corrected |
| Visibility | optional pub on named items/fields/methods, not impl | §§4.1,27 | item, field, members | guide-modules/data | SC positive/negative; modules.rs | aligned |
| Import / alias | mixed dot/`::` path, optional as | §§4.1,27 | use_decl | guide-modules | SC families; modules.rs | grammar corrected |
| Function | optional generics, mut/annotated params, return, body | §§4.2,15,36 | fn_decl, params | guide-functions/generics | SC families/comma; parser.rs | spec excerpt corrected |
| Struct | generics, pub typed fields | §§4.2,17,36 | struct_decl, field | guide-data/generics | SC families; generics.rs | aligned |
| Enum | optional generic params, optional payload types | §§4.2,18 | enum_decl, variant | guide-data | SC families/comma; generics.rs | payload comma corrected |
| Alias | generic transparent type target | §§4.2–3,7,36 | type_alias | reference-types | SC type cases; generics.rs | aligned |
| Const | contextual const UpperName, optional annotation, initializer | §§4.2,26 | const_decl | guide-basics | SC families; contract_sync.rs | uppercase first character, not all caps |
| Top-level let | immutable named constant; mut forbidden | §§4.2,26 | const_decl | guide-basics | SC roots/families | aligned |
| Trait | uppercase contextual name; signature-only methods | §§17.7,36 | trait_decl, trait_method | guide-data/oop/generics | SC families; traits.rs | aligned |
| Impl | inherent or trait-for-type; optional params; methods only | §§17.6–7,36 | impl_decl | guide-oop/generics | SC families; methods.rs | aligned |
| Receiver | first `[mut] self [: type]` | §17.6 | receiver | guide-data/oop | SC families/comma; methods.rs | syntax accepts annotation |
| Generic params/bounds | nonempty `<T: A<U> + B,>` | §36 | type_params, bound | guide-generics | SC families/comma | grammar coverage corrected |
| Generic applications | type args / contextual adjacent call args | §36.1 | type_args, type_head, atom | guide-generics | SC adjacency/type cases | adjacency fixed; `::` suffix gap |
| Binding | `let [mut] name [: type] = expr` | §§4.4,16 | let_stmt | guide-basics | SC statements; parser.rs | aligned |
| Let destructuring | list/variant/name, no literals/mut/annotation | §§4.4,4.7 | let_pattern | guide-matching | SC patterns; parser.rs | grammar overacceptance corrected |
| Assignment | expr target, op, expr at stmt level | §§4.4,24 | assign_or_expr | reference-operators | SC precedence/statements; mutation.rs | target validity checked later |
| Return/throw | return optional expr, throw required expr | §§4.4,14 | return_stmt, throw_stmt | guide-control/errors | SC return/statements | bitnot return fixed |
| Break/continue | keyword, end | §§4.4,14 | break_stmt, continue_stmt | guide-control | SC statements; checker.rs | loop validity later |
| While/loop/for | block bodies; for uses general pattern | §§4.4,14 | loop productions | guide-control | SC statements; parser.rs | aligned |
| Try/catch/finally | mandatory catch binding; optional finally | §§4.4,14.5 | try_stmt | guide-errors | SC statements/newlines; catch_syntax.rs | aligned |
| If / else-if | condition block, optional else expression | §§4.5,14 | if_expr | guide-control | SC newlines; parser.rs | aligned |
| Match | pattern, guard, arrow, block/expr, separators | §§4.5–6,19 | match_expr, match_arm | guide-matching | SC patterns/list length | arm separators documented |
| Lambda | parenthesized params optional fn; `fn [mut] x -> e` | §15.4 | lambda | guide-functions | SC statements/comma | no generic lambda declarations |
| Call | postfix callable or named call | §§4.5,15 | postfix, call_args | guide-functions | SC precedence/negative args | named-method eligibility later |
| Field / method | dot name, optional args/generic args | §§4.5,17 | postfix | reference-operators | SC postfix; methods.rs | missing field alternative restored |
| Index | `expr[expr]` | §§4.5,20 | postfix | guide-collections | SC statements/postfix | no slicing |
| Constructor | uppercase bare call / uppercase brace literal | §§4.5,17–18,24 | atom, struct_body | guide-data | SC comma; grammar.rs | generic branch explicit |
| Names / paths | IDENT and qualified path | §§4.5,27 | path, atom | guide-modules | token/SC families | bare name omission corrected |
| Lists / parenthesized lists | `[items]`, `(a,)`, `(a,b)` | §§4.5,20–21 | list, atom | guide-collections | SC comma; parser.rs | no distinct tuple |
| Maps / blocks | `{:}`, `{k:v}`, `{ statements }` | §§4.5,20 | map, block | guide-collections | SC block/comma | annotated block lookahead fixed |
| Patterns | literal/bind/wildcard/list/variant | §§4.6–7,19 | pattern, let_pattern | guide-matching | SC pairs; parser.rs | exact list length verified |
| Type syntax | primitive/path/application/list/map/union | §§4.3,36 | type, type_member | reference-types | SC pairs; generics.rs | no function TypeExpr |
| Strings / f-strings | either quote; optional f/F; interpolation | §3.6 | lexical constraints, format_spec | guide-basics, reference-operators | SC string/interpolation; lexer.rs | fixes plus explicit SPEC GAPs |
| Operators / range / pipe | complete Pratt levels below | §§4.5,22–23 | expr through unary | reference-operators | SC 19 shape cases; parser.rs | precedence agrees |
| Resource boundaries | existing AST and recursion checks | §31 | implementation constraint | reference-limits | existing boundaries/corpus/property tests | AUDIT-3 excluded; no new ceiling |

## 8. Operator Precedence and Associativity

Implementation-derived, weakest to strongest. Assignment is **not** an
expression operator. Every binary spelling is represented below.

| Level | Operator(s) | Pratt left/right power | Association |
|---|---|---|---|
| 1 | `\|>` | 1 / 2 | left; call/method desugaring |
| 2 | `or` | 3 / 4 | left |
| 3 | `and` | 5 / 6 | left |
| 4 | `==`, `!=` | 7 / 8 | left |
| 5 | `<`, `<=`, `>`, `>=` | 9 / 10 | left |
| 6 | `\|` | 11 / 12 | left |
| 7 | `&` | 13 / 14 | left |
| 8 | `<<`, `>>` | 15 / 16 | left |
| 9 | `..` | 17 / 17 | right |
| 10 | `+`, `-` | 20 / 21 | left |
| 11 | `*`, `/`, `%` | 22 / 23 | left |
| 12 | `^` | 25 / 24 | right |
| 13 | prefix `-`, `not`, `~` | recursive unary | right |
| 14 | call, member, method, index | postfix loop | left |
| 15 | atom | atom dispatch | n/a |

The specification table, formal productions and website operator table agree.
The stale prose that put bitwise/shift operations inside an unparenthesized
range bound was corrected: they bind **outside** the range. Tests distinguish
all neighboring levels, unary/call binding, pipeline desugaring and
associativity. `a..b..c` is `a..(b..c)`; chained comparisons parse leftward,
not as a special mathematical chain. `-a ^ b` is `(-a) ^ b`.
`a |> f() + b` pipes into the whole additive RHS, and `a |> f |> g` becomes
`g(f(a))`. No ordering or association was changed.

## 9. Confirmed Findings

Each row states what the baseline did, how it was reproduced and corrected, and
which permanent test now guards it. “Repro” names the assertion or algorithm,
not a one-off command. Classifications come from the taxonomy in the heading:
CONFIRMED FIXED BUG, DOC DRIFT, GRAMMAR DRIFT, TEST GAP, PLATFORM DIVERGENCE,
SPEC GAP, DESIGN LIMITATION, RFC CANDIDATE, DECISION-PENDING.

| ID | Classification | Reproduction and root cause | Fix | Regression coverage |
|---|---|---|---|---|
| CONF-LEX-1 | CONFIRMED FIXED BUG | `lex("\"")`, `lex("'")`, `lex("f\"")`, `lex("F'")`, `lex("\"abc\\\"")` returned a token or the wrong code. `raw_string_body` scanned to EOF/newline and only checked “last byte was the quote”, so an escaped final quote passed. | `raw_string_body(quote, start)` returns `E1004` at EOF/newline and only succeeds on a real closing quote; `ident` (f-strings) and `string` share it. | `strings_require_real_closing_quotes`; `syntax/conf-lex-1-quote.aura` (`E1004`) |
| CONF-LEX-2 | CONFIRMED FIXED BUG | Escape diagnostics used the post-lexer `pos` as a base, so `lex_at` and multi-line inputs reported the wrong span; `\q` was reported at the string start. | `unescape(raw, base)` walks `char_indices`, so the span is `base + offset` and covers the full `\`+char; `string` passes `self.base + start + 1`. | `escape_and_token_spans_are_source_local` (checks `src[span] == "\\q"` and `lex_at(...,40) == 41..43`); `syntax/conf-lex-2-escape.aura` (`E1003`) |
| CONF-LEX-3 | CONFIRMED FIXED BUG | f-string interpolation expression spans were shifted because trailing trivia was folded into the token span; diagnostics pointed a few bytes early. | `skip_trivia` now runs before recording `start`, so a token span begins at the token; the f-string scanner computes `base + inner_start` from a real source offset. | `escape_and_token_spans_are_source_local` (prefix `" "`, `"<!-- λ --!> "`); `syntax/conf-lex-3-fspan.aura` (`E2003`) |
| CONF-LEX-4 | PLATFORM DIVERGENCE | CLI rejected malformed UTF-8 without an E-code and the WASM byte ABI replacement-decoded it; the lexer API itself cannot see invalid bytes. | `lex::decode_source` validates strictly and returns `E1001` at the invalid sequence; the CLI file/stdin path and the Playground `execute_bytes` (WASM ABI + native harness) share it. | `byte_decoder_rejects_invalid_utf8_everywhere`; `cli_rejects_invalid_utf8_in_file_and_stdin`; Node byte-boundary cases |
| CONF-LEX-5 | TEST GAP | There was no permanent assertion that every `Tok` variant maps to a concrete spelling. | `tests/syntax_tokens.tsv` lists all 79 variants; `every_token_variant_has_a_lexical_case` lexes each and diffs the set against the enum body. | `syntax_docs.rs::every_token_variant_has_a_lexical_case` |
| CONF-LEX-6 | SPEC GAP | `1__0_` → 10 and `0x_f_f_` → 255 are accepted; underscore placement is not normatively defined. | Not changed. Recorded in §4 and §18. | `numeric_edges` pins the observed values |
| CONF-PARSE-1 | CONFIRMED FIXED BUG | `1 |>`, `1 \|>\n`, `1 \|> # eof` accepted a pipeline with no RHS: the Pratt loop broke on EOF/newline before checking the operand. | Removed the `if op.is_none() && Eof/Newline { break }` escape; a missing RHS now raises `E1006` at the operator. RHS stays same-line. | `pipeline_requires_rhs_in_all_entry_points`; `syntax/conf-parse-1-pipe.aura` (`E1006`) |
| CONF-PARSE-2 | CONFIRMED FIXED BUG | Several delimiter lists rejected a trailing comma or newline layout the spec already promised (params, generics, enum payloads, variant patterns, list/let-list, nested `>>`). | Added `nl` handling and trailing-comma acceptance in `params`, `opt_type_params`, `decl_type_args`, `ty_args`, enum payloads, `pattern`/`let_pattern` variant and list forms, call args, list atoms and parenthesized sugar; nested closes use `expect_close_angle`. | `delimiter_comma_matrix`; `syntax/conf-parse-2-comma.aura` (`[3]\n`) |
| CONF-PARSE-3 | CONFIRMED FIXED BUG | Interpolation scanning was textual: a brace inside a quoted string/comment confused closure, so `f"{'{'}"` failed. | `expression_syntax` skips strings, line/block comments and tracks delimiter depth for both closure and separator detection. | `interpolation_respects_expression_tokens`; `syntax/conf-parse-3-braces.aura` (`{\n`) |
| CONF-PARSE-4 | CONFIRMED FIXED BUG | A top-level `:` inside a `::` path was treated as a format separator, so `f"{m::value}"` mis-split. | Both the closure and `split_format_spec` ignore `:` adjacent to `:`; comments/strings are skipped first. | `interpolation_respects_expression_tokens`; `syntax/conf-parse-4-path.aura` (`4\n`) |
| CONF-PARSE-5 | CONFIRMED FIXED BUG | `<` looked source-adjacent to a name when trivia (space/comment) preceded it, because spans included leading trivia. | Token spans exclude leading trivia (CONF-LEX-3); generic adjacency is now true source adjacency. | `generic_adjacency_is_real_source_adjacency`; `syntax/conf-parse-5-adjacency.aura` (`true\n`) |
| CONF-PARSE-6 | CONFIRMED FIXED BUG | `return ~0` lost its operand: `Tok::Tilde` was missing from `starts_expr`, so the statement ended after `return`. | Added `Tok::Tilde` to `starts_expr`. | `return_accepts_every_unary_expression`; `syntax/conf-parse-6-return.aura` (`-1\n`) |
| CONF-PARSE-7 | CONFIRMED FIXED BUG | `{ let x: int = 1; x }` was misclassified as a map because statement-level `:` counted as map evidence. | `map_ahead` returns false at depth 0 on statement keywords and `;`. | `annotated_block_is_not_misclassified_as_a_map`; `syntax/conf-parse-7-block.aura` (`1\n`) |
| CONF-PARSE-8 | SPEC GAP | The parser accepts separator-free adjacency (`1 2`, `let x = 1 let y = 2`) that the normative separator rule does not establish. | Not changed; preserved and documented without granting normative status. | `comments_and_newlines` pins the observation |
| CONF-PARSE-9 | CONFIRMED FIXED BUG | Repeated semicolons (`{;;}`, `{ ;;print(1);; }`) were not accepted by the block loop. | `block` skips `Newline \| Semi` runs before each statement/close. | `comments_and_newlines`; `syntax/conf-parse-9-semi.aura` (`1\n`) |
| CONF-GRAM-4 | SPEC GAP | A generic head followed by `::` has lookahead recognition but no implemented continuation; qualified generic-call syntax is inconsistent. | Not redesigned. Documented in `grammar.md`, `GENERICS.md`; qualified calls use `module::function<T>(...)`. | No new syntax accepted; existing qualified-path tests unchanged |
| CONF-DOC-1 | DOC DRIFT | `LANGUAGE_SPEC.md` declared itself normative; `grammar.md` said the parser is always wrong on disagreement; `CONTRIBUTING.md` named only the contract. | Explicit six-level authority hierarchy in grammar/CONTRIBUTING/spec (§3). | `syntax_docs.rs::website_and_spec_grammar_stay_synchronized` |

## 10. Documentation Drift Corrected

* **Authority (CONF-DOC-1).** The conflicting “parser is always wrong” and
  “contract only” guidance was replaced by the hierarchy in §3, in
  `docs/grammar.md`, `CONTRIBUTING.md` and `docs/LANGUAGE_SPEC.md` §4.
* **Grammar omissions.** `module_decl` was absent; `use_decl` lacked `as` and
  current path form; visibility, bounds, trait/method signatures, receiver
  annotation, qualified paths and bare-name atoms were missing or stale;
  `type` was overloaded for both the union production and the format type;
  block/statement separators, multi-semicolon blocks and the trailing-comma
  layout were not represented. `docs/grammar.md` was rewritten and the new
  bodies were synchronized into `LANGUAGE_SPEC.md` §4 and
  `website/content/reference-grammar.md` (which a byte-equality test guards).
* **Operator/range prose.** The specification paragraph that denied bitwise
  operators, `%= ^= &= |= <<= >>=` and multiline comments was stale against
  the delivered language; it was corrected to state current syntax and keep
  only the genuinely absent forms (XOR, `++`/`--`, tuple type).
* **Roadmap.** `docs/FEATURE_ROADMAP.md` now separates **DELIVERED**,
  **CURRENT LIMITATION**, **DELIBERATE DESIGN DECISION**, **RFC CANDIDATE**,
  **DECISION-PENDING** and historical material, and corrected the multiline
  comment terminator, `Tok::As`, modules and generics status.
* **Guides/reference.** `docs/GENERICS.md`, `docs/contract.md`,
  `website/content/guide-data.md` were corrected where they asserted absent
  visibility/bounds or stale syntax.

The normative excerpts now carry the `nl`, name-class, lookahead and
`END_BOUNDARY` conventions explicitly, so a context-free excerpt no longer
implies rules the parser does not implement.

## 11. Parser/Lexer Corrections

| File | Correction | Scope guard |
|---|---|---|
| `src/lex/mod.rs` | `decode_source`; trivia-free spans via `skip_trivia`; `raw_string_body` requires a real closer and advances whole scalars; `unescape` uses a base offset | No new token, operator or keyword; ASCII classes unchanged |
| `src/parse/mod.rs` | dangling-pipeline escape removed; trailing comma/`nl` in delimiter lists; `expect_close_angle` for nested `>>`; `map_ahead` statement guard; `Tok::Tilde` starts an expression; source-aware f-string scanning | No precedence change; no `nl` outside grammar `nl` positions; no TypeExpr change |
| `src/main.rs` | file/stdin read as bytes through `decode_source`, rendered with `render_with_source` | Same exit codes; only malformed UTF-8 changes |
| `playground/runtime/src/lib.rs` | `execute_bytes` strict-decodes before `execute`; WASM `aura_run` uses it | Byte ABI only; valid UTF-8 path unchanged; no imports |
| `playground/runtime/src/bin/aura-playground-native.rs` | native harness uses `execute_bytes` | Differential parity source |
| `fuzz/fuzz_targets/{lexer,parser}.rs` | exercise `decode_source` on arbitrary bytes | No architecture change |

No change touched the checker, interpreter, standard library, Python bridge,
equality/render budgets, or the TypeExpr limits.

## 12. Test and Corpus Additions

| Category | Count | Location |
|---|---|---|
| Rust `#[test]` functions in the new syntax matrix | 20 | `tests/syntax_conformance.rs` |
| Rust documentation/inventory tests | 2 | `tests/syntax_docs.rs` |
| Token inventory rows | 79 | `tests/syntax_tokens.tsv` |
| New syntax corpus fixtures | 11 | `tests/corpus/syntax/*.aura` |
| Corpus registry rows added | 11 | `tests/corpus.rs` |
| Fuzz seeds added | 6 | `fuzz/seeds/{lexer,parser}/conf-*` |
| Node explicit native/WASM comparisons | 43 | `playground/tests/node/syntax.test.mjs` |

The Node figure is measured: 19 cases × 2 line-ending forms + 5 invalid-UTF-8
byte buffers × 1 = 43. The corpus registry is checked bidirectionally by
`corpus_inventory_is_complete_and_sound` (nothing missing on disk, nothing on
disk missing from the table), so no duplicate or dead row can persist.

`property_hardening.rs` recreates a one-byte `s` file as a side effect of its
generated `write_file` call. This is pre-existing, not introduced by Phase 1;
the transient artifact is removed after validation and is not committed. It is
recorded in §18 rather than fixed by broadening this phase.

## 13. Future RFC Readiness

These are **not implemented** and were not selected here. Each states its
current obstacle and the subsystems a future change would touch.

1. **Multiline pipelines.** Current rule: a pipeline RHS must be on the same
   line (§4, `expr_bp`). Obstacle: the newline is a statement separator, so a
   continuation needs an explicit bracket/indent rule or a lexer change.
   Touches `parse::expr_bp`, `docs/grammar.md` `pipe`, the separator policy
   (CONF-PARSE-8) and any formatter. Compatibility: existing same-line programs
   are unaffected; the question is only whether to allow a leading pipe.
2. **List-rest patterns (`[head, ..tail]`).** Current rule: list patterns are
   exact-length (§4.6–7). Obstacle: there is no rest token and matching would
   change exhaustiveness/length semantics. Touches `pattern`/`let_pattern`,
   the checker's match coverage and `Ty::List`. Compatibility: none for the new
   form; existing exact lists must keep their meaning.
3. **Range step/direction.** Current rule: `a..b` is half-open with no step
   (§22). Obstacle: a third argument needs either a new token or a stdlib-only
   helper. Touches `Expr::Range`, `for` desugaring, `range`/materialization
   bounds (§31) and overflow behavior. Compatibility: stdlib-only `range(a,b,s)`
   would avoid grammar change.
4. **Named method arguments.** Current rule: the parser already accepts
   `receiver.method(x: 1)`; the checker rejects it semantically (`E3001`).
   Obstacle: method overload resolution keys on ordered input types and has no
   named-argument binding. Touches the checker's call resolution and the method
   signature registry. Compatibility: purely additive if binding is by name.

## 14. AUDIT-3 Status

Follow-up 3 / AUDIT-3 remains DECISION-PENDING; no code or doc changes beyond the existing decision package; property test AST-limit explicitly excludes TypeExpr-heavy inputs pending that decision.

The native ceiling (accept through 2047, reject 2048 with `E1015`) and the WASM
ceiling (accept through 767, reject 768 with `E1015`) are unchanged and are not
normalized in this phase. `docs/AUDIT3_TYPE_NESTING_DECISION.md` is untouched,
and `tests/property_hardening.rs` PROPERTY 3 excludes TypeExpr-heavy inputs.
Resolving the nesting requires an explicit human token
(`DECISION APPROVED: OPTION A` or `DECISION APPROVED: OPTION B`) that this
phase did not receive.

## 15. Artifact Integrity

| Artifact | Path | Bytes | SHA-256 |
|---|---|---|---|
| Frozen 0.0.2 | `playground/runtimes/0.0.2/aura_playground_runtime.wasm` | 1,366,621 | `5a4ad3f7e3f786164d65df437d607e7ddd5e25947ea2c8dd9b436a5490b334ed` |
| dev.23 | `playground/runtimes/0.0.2-dev.23/aura_playground_runtime.wasm` | 1,604,958 | `71072150e67384120c63e22d6176f3683110735b84f74723bea315f79778a528` |
| dev.24 | `playground/runtimes/0.0.2-dev.24/aura_playground_runtime.wasm` | 1,604,902 | `16882fe60d7fa52f9e204b3841cc59764c79f50c1068f33b7c3e3350f1adfd39` |

Both pre-existing artifacts are byte-identical to the supplied identities.
dev.24 was rebuilt from the current source and is byte-for-byte reproducible
(see §16). Its manifest entry records `current: 0.0.2-dev.24`, the hash and size
above, `runtime_version: 0.0.2-dev.24`, `host_abi_version: 1`; the website build
stages the same bytes. dev.23 is not overwritten and is not reused for the
changed bytes. No release, tag or version beyond dev.24 was created.

## 16. Validation

All commands below were run on the current worktree (branch `rewrite/v3-rust`,
macOS native host, pinned Rust toolchain and Node).

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | pass (no output) |
| `cargo test --locked --all-features --test syntax_conformance --test syntax_docs --test corpus` | 24 passed (20 + 2 + 2) |
| `cargo test --locked --all-targets --all-features` | 635 passed, 0 failed, 31 test binaries |
| `cargo clippy --locked --all-targets --all-features -- -D warnings` | pass |
| `cargo test --locked --all-targets --no-default-features --features cli,repl,json,regex,time` | 626 passed, 0 failed |
| `node playground/tests/node/run-all.mjs` | manifest 26, ABI 67, integrity 27, differential 195, syntax conformance 43, browser 53, worker 12, cache 7 — all 0 failed |
| `node playground/build.mjs --check` | `manifest matches 3 version(s)` |
| `cargo build --locked --manifest-path playground/runtime/Cargo.toml --release --target wasm32-unknown-unknown` | rebuilt hash = dev.24 hash (reproducible) |
| `node website/tests/run-all.mjs` | examples, links, base 2085 refs, browser 343, a11y 70, reused playground suite — all 0 failed |

Test accounting is reported by category in §12; totals here are measured, not
summed across categories. The Node syntax suite confirms zero WebAssembly
imports. The differential harness reports native ceiling 2048 / WASM ceiling
768 with zero host failures, consistent with the unchanged AUDIT-3 boundary.

## 17. Git and CI State

* Branch: `rewrite/v3-rust`; starting HEAD `f7c764f6b21ef895e2133d6cea397c543e736926`.
* The Astra changes were uncommitted at recovery. They are grouped into coherent
  commits at the end of this phase (see the continuation result): strict source
  decoding/spans, parser grammar alignment, tests/corpus, documentation
  reconciliation, and the dev.24 runtime staging.
* No history was rewritten, no force push, no tag, no release.
* CI run details (workflow, run ID, jobs, result, deploy) are recorded in the
  continuation result after the push.

## 18. Remaining Known Limitations

* **CONF-PARSE-8 — general separator policy (SPEC GAP).** The parser accepts
  adjacent statements/items without a separator. Not legitimized and not
  tightened.
* **Numeric underscore placement (SPEC GAP).** `1__0_`, `0x_f_f_` accepted;
  policy undefined.
* **F-string outer edge rules (SPEC GAP).** Outer-quote selection, a lone
  literal `}`, and broader continuation edge cases are incompletely specified;
  Python semantics were not imported.
* **CONF-GRAM-4 — generic-head `::` continuation (SPEC GAP).** Lookahead
  recognizes the head; the continuation is unimplemented.
* **Named method arguments (DESIGN LIMITATION).** Parse today, rejected
  semantically (`E3001`); an RFC candidate, not a parser gap.
* **Test hygiene (DESIGN LIMITATION).** `property_hardening.rs` writes a
  transient `s` file; pre-existing and out of Phase-1 scope.
* **TypeExpr nesting (DECISION-PENDING).** Native 2047/2048, WASM 767/768,
  unchanged.

## 19. Phase-2 Handoff

Phase 2 does **not** start here. This phase establishes only the verified
starting point for a semantic layer above syntax:

```
AST invariants
  → module/name resolution
  → visibility
  → scopes
  → semantic checker boundary
```

Ready inputs for Phase 2: a grammar re-derived from the implementation and
synchronized across spec/grammar/website; an authority hierarchy that makes
`LANGUAGE_SPEC.md` normative; a permanent syntax matrix plus token/corpus
inventory; strict byte handling at every host boundary; and an explicit list of
the syntax SPEC GAPs above that name resolution must not silently resolve.
Phase 2 must treat the separator policy, numeric-underscore policy, f-string
outer edges and generic-head `::` continuation as open, and must not assume a
TypeExpr nesting policy until AUDIT-3 is approved.
