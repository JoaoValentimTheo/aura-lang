# Aura Core Freeze

This is a **status and evidence** record, not a semantic authority.
`docs/LANGUAGE_SPEC.md` remains the single normative source; where the two
disagree, the spec wins.

## Scope of Aura Core

Lexer, parser, AST, in-source name resolution, scopes, in-source modules,
visibility, aliases, the type system (primitives, structural collections,
unions, static generics, generic aliases/structs/enums/functions/methods/
traits, trait bounds), functions, lambdas and closures, overloads, structs,
enums, patterns, `match`, control flow, exceptions, operators, ranges,
f-strings, lists, generic maps, `items()`, list and map comprehensions,
mutation/reference semantics, equality, rendering, the standard library, the
host boundary, the Python bridge, the JSON boundary, the CLI, the REPL, the
native runtime, the WASM runtime, the Playground, the current documentation and
website, runtime artifacts, and CI.

## Explicitly outside Core

Filesystem-backed source modules and everything built on them — `mod.aura`,
multi-file discovery, module graphs, package roots, on-disk cross-file
diagnostics — plus package management, an LSP, a formatter product, a debugger,
a workspace/project system, async, concurrency, bytecode/optimizers/native
codegen, macros, reflection, dynamic dispatch, inheritance, trait objects,
classes, and new package tooling.

## Implementation status

Every construct Aura claims to possess has one documented syntax, complete
semantics, parser support, checker behavior, runtime behavior, deterministic
evaluation, stable diagnostics, regression coverage, defined native/WASM
behavior, and current documentation/website representation. Every intentionally
absent capability is classified explicitly in `LANGUAGE_SPEC.md` §34 as
**DELIBERATELY ABSENT** or **POST-CORE RFC**.

## Final collection design

* Lists are `[T]`; maps are `{K: V}`. No nominal `List<T>`/`Map<K, V>`
  built-in; `type Map<K, V> = {K: V}` is an ordinary alias.
* Map keys are a key-capable scalar: `string`, `int`, or `bool` (or a union
  whose every member is). `float`, `none`, and containers/structs/enums/
  ranges/functions are not key-capable. A generic key parameter is admissible
  in a declaration and must be key-capable when instantiated.
* Literal inference folds every statically known component into a union; a
  known expected type is checked against every component.
* `m.items()` yields `[[k, v], ...]` (eager snapshot, ascending key order).
  Pairs are two-element lists — Aura has no tuple type.
* Comprehensions: `[v for p in it]`, `[v for p in it if c]`,
  `{k: v for p in it}`, and the filtered map form; one generator, optional
  filter, eager, single iterable evaluation, non-leaking bindings.

## Resolved Core SPEC GAPs

All nine are closed with a decided rule, implementation, tests, and docs:

| Gap | Decision |
| --- | --- |
| CONF-PARSE-8 (separators) | A real newline/`;` separator is required between statements and items; only the boundary before `}`/EOF is zero-width. |
| Numeric underscores | `_` only between two valid digits of the same component. |
| F-string outer edges | A lone `}` is `E1006`; `}}` is the only literal `}`. |
| CONF-GRAM-4 (generic-head `::`) | A generic head continues through `::` to a qualified variant; type arguments bind to the enum head. |
| CONF-RESOLVE-6 (variant scope) | Variant tags are unique per module, not globally. |
| CONF-RESOLVE-7 (nested visibility) | `pub module` is a real boundary; a descendant may reach an ancestor's private items, an ancestor may not reach a descendant's private items. |
| CONF-RESOLVE-8 (`pub use`) | Public re-exports are implemented, visibility-checked, non-escalating, transitive, and terminating. |
| CONF-RESOLVE-9 (module aliases) | `use path as Alias` binds a lexical, resolution-only, collision-checked module alias. |
| CONF-RESOLVE-10 (canonical collision) | A type and a variant that collapse to one canonical name are rejected deterministically. |

The legacy dotted `use a.b` spelling is removed in favor of `::` (development
compatibility tightening).

## AUDIT-3 status

Follow-up 3 / AUDIT-3 remains DECISION-PENDING; no code or doc changes beyond the existing decision package; property test AST-limit explicitly excludes TypeExpr-heavy inputs pending that decision.

## Runtime artifact

The current Core development runtime is `0.0.2-dev.30`:

- path: `playground/runtimes/0.0.2-dev.30/aura_playground_runtime.wasm`
- bytes: `1,652,786`
- SHA-256: `f4e887770d94662544fe339e0d340a68ea4d16cc37f05d73fd46b588a7896387`
- wasm imports: `0`
- reproducibility: byte-identical across independent clean-target builds

The frozen public `0.0.2` artifact
(`1,366,621` / `5a4ad3f7e3f786164d65df437d607e7ddd5e25947ea2c8dd9b436a5490b334ed`)
and the historical `dev.23`–`dev.29` artifacts are immutable and are preserved
unchanged.

## Validation categories

Rust (all-features, no-default, MSRV), corpus/property/fuzz, CLI subprocess,
REPL session, module/resolver, generics, collections/maps, comprehensions,
Python bridge, native/WASM differential, Playground node and browser suites,
website build/links/examples/a11y/browser, and clean-target WASM
reproducibility. Local validation is not GitHub CI; CI is verified only by an
observed run on the exact pushed SHA.

## Next phase

**AURA FILESYSTEM MODULE SYSTEM.** `mod.aura`, physical file discovery, the
module graph, cross-file imports, cycles, source ownership, path diagnostics,
native host loading, and a browser/virtual-filesystem strategy. None of this is
implemented. It must begin only after Core Freeze, and it must feed the *same*
in-source-style logical module tree, canonical resolver, checker, and runtime —
no second module semantics.
