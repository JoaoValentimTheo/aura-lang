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
| `E001` | `SYNTAX_ERROR` | Generic malformed source | parser |
| `E002` | `UNEXPECTED_TOKEN` | `Expected <x> but got <y>` | parser |
| `E003` | `UNEXPECTED_EOF` | `Unexpected end of file` | parser |
| `E004` | `INVALID_SYNTAX` | Construct is not allowed here | parser, rule checker |

## Errors — types (`E1xx`)

| Code | Name | Message | Emitted by |
|------|------|---------|------------|
| `E101` | `TYPE_MISMATCH` | `expected <T>, got <U>` | type checker |
| `E102` | `UNDEFINED_VARIABLE` | `Undefined variable '<name>'` | type checker |
| `E103` | `UNDEFINED_FUNCTION` | `Undefined function '<name>'` | type checker |
| `E104` | `UNDEFINED_CLASS` | `Undefined class '<name>'` | type checker |
| `E105` | `WRONG_ARGUMENT_COUNT` | function called with the wrong arity | type checker |
| `E106` | `WRONG_ARGUMENT_TYPE` | argument of the wrong type | type checker |
| `E107` | `CANNOT_CALL_NON_FUNCTION` | calling a value that is not callable | type checker |
| `E108` | `INCOMPATIBLE_OPERANDS` | operator used on incompatible operands | type checker |
| `E109` | `NON_EXHAUSTIVE_MATCH` | `'match' over <subject> is not exhaustive` | type checker (warning) |
| `E110` | `UNKNOWN_TYPE_CONSTRAINT` | `type parameter '<name>' has unknown constraint '<constraint>'` | type checker |

## Errors — semantic / structural (`E3xx`)

| Code | Name | Message | Emitted by |
|------|------|---------|------------|
| `E301` | `DUPLICATE_DEFINITION` | name already defined in this scope | rule checker |
| `E302` | `UNREACHABLE_CODE` | statement is unreachable | rule checker |
| `E303` | `REASSIGN_IMMUTABLE` | reassigning a `let`/`const` binding | mutability checker |
| `E307` | `MISSING_VISIBILITY` | class/trait member has no visibility | rule checker |
| `E308` | `INACCESSIBLE_MEMBER` | non-public member accessed from outside | rule checker |
| `E309` | `UNIMPLEMENTED_ABSTRACT` | concrete class missing an abstract method | rule checker |
| `E310` | `MISSING_MAIN` | entry file has no `main` | rule checker |
| `E311` | `INVALID_MAIN` | `main` has a bad signature | rule checker |

## Errors — I/O and configuration (`E4xx`)

| Code | Name | Message | Emitted by |
|------|------|---------|------------|
| `E401` | `IO_ERROR` | file not found / unreadable / unwritable | CLI |
| `E402` | `CONFIGURATION_ERROR` | invalid `aura.toml` or dependency entry | CLI / deps |

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
| `W101` | `UNUSED_VARIABLE` | variable is declared but never used | rule checker |
| `W102` | `UNUSED_IMPORT` | import is never used | rule checker |
| `W103` | `UNUSED_TYPE_PARAMETER` | generic type parameter is never used | type checker |

## Warnings — semantic (`W2xx`)

Reserved for future semantic warnings; none are emitted today.

---

## Removed codes

These were defined historically but never emitted; they are removed to keep the
catalogue honest.

| Code | Reason |
|------|--------|
| `E201`–`E204` | runtime faults are surfaced as native Python exceptions; no Aura diagnostic is emitted |
| `E304` | `MISSING_RETURN` is not enforced; the type checker reports `E101` instead |
| `E305` | superseded by `W101` |
| `E306` | superseded by `W102` |