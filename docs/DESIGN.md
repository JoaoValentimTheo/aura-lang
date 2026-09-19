---
layout: default
title: "Aura Transpiler - Architecture"
nav_order: 6
---

# Aura Transpiler - Architecture

Internal documentation for the Aura transpiler implementation.

---

## Overview

Aura is a **gradually-typed**, **functional-first** programming language that transpiles to Python 3. The transpiler converts `.aura` source files to executable Python code.

## Design Principles

1. **Simplicity**: Explicit syntax with minimal boilerplate
2. **Type Safety**: Optional type system without runtime overhead
3. **Functional First**: First-class lambdas, pipe operator, higher-order functions
4. **Python Integration**: Seamless interoperability with Python ecosystem
5. **Readability**: Generated Python code should be clean and debuggable

## Architecture

```
Source (.aura) --> Parser --> AST --> Transformer --> Python Code
                       |                    |
                       v                    v
                 Type Checker         Macro Expansion
```

### Components

| Component | Location | Purpose |
|-----------|----------|---------|
| AST Nodes | `aura/transpiler/ast.py` | AST node types and the protocol (dunder) map |
| Tokenizer | `aura/parser/to_ast.py` (`Tokenizer`) | Hand-written lexer |
| Parser | `aura/parser/to_ast.py` (`Parser`) | Recursive-descent + Pratt parser producing the AST |
| Transformer | `aura/transpiler/transformer.py` | AST to Python code |
| Expression Transformer | `aura/transpiler/transformers/expressions.py` | Expression handling |
| Statement Transformer | `aura/transpiler/transformers/statements.py` | Statement handling |
| Type System | `aura/transpiler/types.py` | Type classes, inference, checking |
| Macros | `aura/transpiler/macros.py` | Decorator-based macro system and preludes |
| Error Handling | `aura/transpiler/errors.py` | Coded `E##`/`W##` diagnostics with locations; catalogued in [ERRORS.md](ERRORS.md) |
| CLI | `aura/cli.py` (entry point `main.py`) | User interface |

See [AUDIT.md](AUDIT.md) for the findings and fixes from the code audit.

### Compilation Pipeline

1. **Lexical Analysis**: Source code is tokenized by the hand-written
   `Tokenizer` in `aura/parser/to_ast.py`
2. **Parsing**: Tokens are parsed directly into AST nodes by the
   recursive-descent `Parser` (there is no separate parse tree or ANTLR step)
3. **Type Checking**: Optional type validation (`aura/transpiler/types.py`)
4. **Macro Expansion**: Decorator macros are applied (`aura/transpiler/macros.py`)
5. **Transformation**: AST is converted to Python source code
6. **Execution**: Generated Python code is executed via `exec()` or saved to file

### AST Node Types

The AST (`aura/transpiler/ast.py`) defines the node types covering:

- **Declarations**: `VarDecl`, `ConstDecl`, `FunctionDecl`, `ClassDecl`, `TraitDecl`, `TypeDecl`, `EnumDecl`, `Module`
- **Statements**: `IfStmt`, `UnlessStmt`, `GuardStmt`, `WhileStmt`, `UntilStmt`, `ForStmt`, `LoopStmt`, `TryStmt`, `MatchStmt`, `ReturnStmt`, `BreakStmt`, `ContinueStmt`, `AssertStmt`, `WithStmt`
- **Expressions**: `BinaryOp`, `UnaryOp`, `CallExpr`, `MemberExpr`, `IndexExpr`, `SliceExpr`, `LambdaExpr`, `PipeExpr`, `CondExpr`, `ElvisExpr`, `CoalesceExpr`, `RangeExpr`, `ComprehensionExpr`, `SafeNavExpr`, `SpreadExpr`, `MatchExpr`, `TryExpr`
- **Literals**: `IntLiteral`, `FloatLiteral`, `StrLiteral`, `BoolLiteral`, `NoneLiteral`, `FStringLiteral`, `ListLiteral`, `DictLiteral`, `SetLiteral`, `TupleLiteral`
- **Patterns**: `LiteralPattern`, `IdentifierPattern`, `WildcardPattern`, `ConstructorPattern`, `ListPattern`, `DictPattern`, `OrPattern`, `AsPattern`
- **Types**: `SimpleType`, `GenericType`, `FunctionType`, `UnionType`, `OptionalType`, `StructuralType`

### Macro System

Built-in macros are implemented as Python decorators injected as a runtime prelude:

| Macro | Purpose |
|-------|---------|
| `@debug` | Log function entry/exit with arguments and return value |
| `@timeit` | Measure and print execution time |
| `@memoize` | Cache function results (unbounded) |
| `@cache(maxsize=N)` | Cache with LRU eviction |
| `@must_return` | Assert function returns a value |
| `@deprecated` / `@deprecated("msg")` | Warn when function is used |

`@property`, `@staticmethod` and `@classmethod` are handled directly by the
parser/transformer (they are language-level decorators, not prelude macros).

The prelude is injected automatically only when a macro is used.

### Type System

The type system (`transpiler/types.py`) provides 15 type classes:

- `Type` (base), `AnyType`, `NeverType`, `NoneType`
- `IntType`, `FloatType`, `StrType`, `BoolType`
- `ListType`, `DictType`, `SetType`, `TupleType`
- `FunctionType`, `ClassType`, `UnionType`, `TypeVariable`

Features:
- Type inference from literals and operations
- Union type compatibility
- Generic type variables
- Structural type matching

## Project Structure

The installable package is `aura/`; top-level `parser/`, `transpiler/`,
`stdlib/`, `repl/` and `tools/` are thin compatibility shims for source
checkouts and existing tests.

```
aura-lang/
├── aura/                   # Installable package (pip install .)
│   ├── cli.py              # Console entry point (`aura` command)
│   ├── runtime.py          # stdlib namespace aliases for generated code
│   ├── parser/
│   │   └── to_ast.py       # Tokenizer + recursive-descent parser
│   ├── transpiler/         # Core transpilation logic
│   │   ├── ast.py          # AST node definitions + protocol (dunder) map
│   │   ├── transformer.py  # Main AST to Python transformer
│   │   ├── semantics.py    # Mutability checker
│   │   ├── importer.py     # Local .aura import hook
│   │   ├── types.py        # Type system (15 type classes)
│   │   ├── macros.py       # Runtime prelude (6 macros)
│   │   ├── errors.py       # Error collection and formatting
│   │   └── transformers/   # Modular transformers
│   │       ├── expressions.py
│   │       └── statements.py
│   ├── stdlib/             # Standard library (210+ functions)
│   │   ├── collections.py  # List, dict, set utilities
│   │   ├── itertools.py    # Iterator utilities
│   │   ├── math.py         # Mathematical functions
│   │   ├── string.py       # String manipulation
│   │   ├── json.py, time.py, io.py
│   │   ├── regex.py        # Regular expressions
│   │   ├── os.py           # Environment, paths, process info
│   │   └── http.py         # HTTP client (stdlib urllib, optional requests)
│   ├── repl/               # Interactive engine
│   ├── lsp/                # Language server (JSON-RPC over stdio)
│   └── tools/              # Formatter, deps, release, debugger, generators
├── examples/               # Working example programs
├── tests/                  # Test suite (static, runtime, regression, fuzz)
├── docs/                   # Documentation
├── main.py                 # Backward-compatible CLI shim
└── pyproject.toml          # Packaging metadata + console script
```

## CLI Commands

Installed as `aura <command>`; the same commands work via `python3 main.py`.

| Command | Description |
|---------|-------------|
| `aura transpile <file>` | Convert Aura to Python (stdout) |
| `aura transpile <file> -o <out>` | Convert Aura to Python (file) |
| `aura check <file>` | Type + mutability checks |
| `aura format <file>` | Format source code |
| `aura lint <file>` | Check style warnings |
| `aura run <file>` | Transpile and execute |
| `aura run <file> -v` | Run with Python code output |
| `aura test <dir>` | Run `.aura` files |
| `aura repl` | Start interactive REPL |
| `aura init [name]` | Create `aura.toml` and `src/main.aura` |
| `aura add <pkg>` | Add and install a dependency |
| `aura install` | Install dependencies from `aura.toml` |
| `aura deps` | List declared dependencies |
| `aura version [bump]` | Show or bump the version |
| `aura debug <file> [-t]` | Run under the trace debugger |
| `aura lsp` | Start the language server (stdio) |

## Dependencies

- Python 3.10+ (the runtime has no third-party dependencies; `tomli` is
  installed automatically on 3.10 for reading `aura.toml`)
