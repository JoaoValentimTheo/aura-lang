---
layout: default
title: "Aura Diagnostics Reference"
nav_order: 4
---

# Aura Diagnostics Reference

Single source of truth for every diagnostic the Aura toolchain can report.
`aura/transpiler/errors.py` mirrors this document; a test
(`tests/test_diagnostics.py`) asserts the two stay in sync.

A diagnostic is either an **error** (prefix `E`, stops compilation / fails the
command) or a **warning** (prefix `W`, reported but non-fatal).

## Format

```
<file>:<line>:<column>: <severity> [<code>]
  <message>
  hint: <how to fix it>
```

The `file:line:column` prefix is omitted only when no location is known (for
example, a whole-program rule). Every checker attaches a real
`SourceLocation`.

## Numbering

| Range       | Area                                        |
|-------------|---------------------------------------------|
| `E0xx`      | Lexical and syntax errors                   |
| `E1xx`      | Type errors                                 |
| `E3xx`      | Semantic / structural errors                |
| `E4xx`      | I/O, configuration, environment             |
| `E9xx`      | Fatal / internal                            |
| `W0xx`      | Style and formatting warnings               |
| `W1xx`      | Unused / redundant-code warnings            |
| `W2xx`      | Semantic warnings                           |

---

## Errors — lexical and syntax (`E0xx`)

| Code | Name | Message | Emitted by |
|------|------|---------|------------|
| `E004` | `INVALID_SYNTAX` | Construct is not allowed here | parser, rule checker |

The parser reports lexical and syntax errors by raising a `SyntaxError` that
carries `line`/`column`/`filename`. These are surfaced directly by the CLI and
the LSP rather than as a separate code.

## Errors — types (`E1xx`)

| Code | Name | Message | Emitted by |
|------|------|---------|------------|
| `E101` | `TYPE_MISMATCH` | `expected <T>, got <U>` | type checker |
| `E105` | `WRONG_ARGUMENT_COUNT` | function called with the wrong arity | type checker |
| `E106` | `WRONG_ARGUMENT_TYPE` | argument of the wrong type | type checker |
| `E108` | `INCOMPATIBLE_OPERANDS` | operator used on incompatible operands | type checker |
| `E109` | `NON_EXHAUSTIVE_MATCH` | `'match' over <subject> is not exhaustive` | type checker (warning) |
| `E110` | `UNKNOWN_TYPE_CONSTRAINT` | `type parameter '<name>' has unknown constraint '<constraint>'` | type checker |

Undefined-name and non-callable diagnostics are not emitted: Aura is gradually
typed and resolves names at runtime, so a name the checker cannot prove is left
to Python rather than reported. See "Removed codes" below.

## Errors — semantic / structural (`E3xx`)

| Code | Name | Message | Emitted by |
|------|------|---------|------------|
| `E301` | `DUPLICATE_DEFINITION` | name already defined in this scope | rule checker |
| `E302` | `UNREACHABLE_CODE` | statement is unreachable | rule checker |
| `E303` | `REASSIGN_IMMUTABLE` | reassigning a `let`/`const` binding, a class `const` member, or module state from outside | mutability checker, rule checker |
| `E307` | `MISSING_VISIBILITY` | class/trait member has no visibility | rule checker |
| `E308` | `INACCESSIBLE_MEMBER` | non-public class member or non-exported module member accessed from outside | rule checker |
| `E309` | `UNIMPLEMENTED_ABSTRACT` | concrete class missing an abstract method | rule checker |
| `E310` | `MISSING_MAIN` | entry file has no `main` | rule checker |
| `E311` | `INVALID_MAIN` | `main` has a bad signature | rule checker |
| `E312` | `MAIN_IN_MODULE` | `main` declared inside a `module` body (never runs) | rule checker |
| `E313` | `UNRESOLVED_REEXPORT` | a `module` re-exports a name no sibling source defines | rule checker |
| `E314` | `UNKNOWN_BASE_CLASS` | `extends` names a base that does not exist | rule checker |
| `E315` | `INVALID_INHERITANCE` | duplicate base or circular inheritance | rule checker |
| `E316` | `INSTANTIATE_ABSTRACT` | instantiating a trait, an `abstract class`, or a class with unimplemented abstract methods | rule checker |
| `E317` | `SELF_IN_STATIC` | `self`/`cls` used in a static method | rule checker |
| `E318` | `UNKNOWN_LABEL` | `break`/`continue` names a label that is not an enclosing loop | rule checker |
| `E319` | `USED_BEFORE_DECLARED` | a local is used before its declaration | mutability checker |
| `E320` | `DECORATOR_ON_FIELD` | a method decorator applied to a field | parser (SyntaxError) |
| `E321` | `ABSTRACT_SUPER_CALL` | `super.method()` targets an abstract method with no implementation | rule checker |

## Errors — I/O and configuration (`E4xx`)

There is **no** code in this range. I/O and configuration failures (a missing
file, an unreadable path, a bad project layout) are reported by the CLI as a
plain message with a non-zero exit status, not as a numbered diagnostic.

## Errors — fatal (`E9xx`)

| Code | Name | Message | Emitted by |
|------|------|---------|------------|
| `E999` | `FATAL` | too many errors; compilation stopped | error collector |

---

## Warnings — style (`W0xx`)

Produced by `aura lint`. Non-zero exit unless `--allow-warnings` is passed.

| Code | Name | Message | Emitted by |
|------|------|---------|------------|
| `W001` | `LINE_TOO_LONG` | line exceeds the recommended width | linter |
| `W002` | `TRAILING_WHITESPACE` | line has trailing whitespace | linter |
| `W003` | `NAMING_CONVENTION` | identifier is not `snake_case` / `PascalCase` | linter |
| `W004` | `SPACING` | multiple spaces after a keyword | linter |

## Warnings — unused / redundant (`W1xx`)

| Code | Name | Message | Emitted by |
|------|------|---------|------------|
| `W103` | `UNUSED_TYPE_PARAMETER` | generic type parameter is never used | type checker |

Unused-variable and unused-import warnings are not emitted; see "Removed
codes" below.

## Warnings — semantic (`W2xx`)

Reserved for future semantic warnings; none are emitted today.

---

## Removed codes

These were defined historically but never emitted; they are removed to keep the
catalogue honest.

| Code | Reason |
|------|--------|
| `E201`–`E204` | runtime faults are surfaced as native Python exceptions; no Aura diagnostic is emitted |
| `E401`, `E402` | I/O and configuration failures are printed by the CLI as plain messages with a non-zero exit status; no numbered diagnostic is emitted |
| `E304` | `MISSING_RETURN` is not enforced; the type checker reports `E101` instead |
| `W101` | `UNUSED_VARIABLE` was documented but never emitted; removed rather than shipped as half-working |
| `W102` | `UNUSED_IMPORT` was documented but never emitted; removed rather than shipped as half-working |