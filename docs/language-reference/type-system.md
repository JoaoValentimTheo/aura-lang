---
layout: default
title: "Type System and Type Checking"
parent: Aura Language Reference
nav_order: 12
---

[English](type-system.md) · [Português](type-system.pt_BR.md)

# Type System and Type Checking

**Status:** Stable (rules) · **Evidence:** `aura/transpiler/types.py`
(`TypeChecker` §518, `TypeInference` §285, `_parse_type_annotation` §1456),
`aura/transpiler/errors.py` (`ErrorCode`), `aura/transpiler/rules.py`,
`aura/cli.py` (`cmd_check` §200, `cmd_run` §480),
`aura/transpiler/transformers/statements.py` (`_aura_type_to_python` §770),
execution *probes*.

> This document avoids the vague term "strong typing". It describes **concrete
> behaviour**: what is accepted, what is rejected, when inference runs, what is
> guaranteed, and — crucially — **what is erased** because Aura transpiles to
> Python.

---

## 1. System classification

Aura is **gradually typed**:

- Annotations are **optional**. Anything the checker cannot prove is treated as
  `AnyType` and **accepted** (`TypeChecker` docstring, `types.py:518-527`).
- **Local inference** exists for literals and a fixed set of operators and
  builtins (`TypeInference`, §3); a bare variable only has a type once it is
  annotated or assigned (`_expr_type`, `types.py:1177-1186`).
- Subtyping is **nominal**: a `ClassType` compares by **name** only
  (`types.py:219-226`); declared bases are linked but method/argument
  compatibility is not walked structurally.
- Generics are **erased**: type arguments are not retained as parameterized
  types (`_parse_type_annotation` §7).

The message of this document is the **guarantee boundary**. Where the checker
does *not* prevent an operation, that is stated explicitly.

---

## 2. Where type checking happens (real pipeline)

```
Source ─▶ Tokenizer ─▶ Parser ─▶ AST (types stored as strings)
       ─▶ TypeChecker.check_program   (aura check, LSP, REPL)
       ─▶ RuleChecker.check_program   (aura run / transpile)
       ─▶ Transformer / emit          (annotations mostly erased)
```

**Type checking is not part of the run/transpile path.** `cmd_run`
(`cli.py:480-553`) and `cmd_transpile` run only the *mutability* and *rule*
checkers; `cmd_check` (`cli.py:200-238`) is the entry point that instantiates
`TypeChecker` and reports `E1xx`/`W103`/`E109`. The LSP (`aura/lsp/server.py`)
and the REPL (`aura/repl/engine.py`) also run it. So a program can `aura run`
with a type error that `aura check` reports.

### 2.1 The type checker has no "typed AST"

Annotations are `str` in the AST (`parse_type` returns a string). The checker
converts them on demand with `_parse_type_annotation` (`types.py:1456-1510`)
and keeps resolved bindings in a plain `context: dict[str, Type]`. Nothing is
attached back to the AST.

### 2.2 When diagnostics are produced

Diagnostics are appended as the checker walks (`_add`, `types.py:566-571`);
`check_program` returns `True` when `self.errors` is empty. Two severities
exist: `ERROR` (fails `aura check`) and `WARNING` (does not). The warning codes
`E109` and `W103` use `E`/`W` names but both carry `ErrorSeverity.WARNING`
(*probe*).

---

## 3. Type inference (`TypeInference`)

`TypeInference` (`types.py:285-511`) is **context-free** and deliberately
conservative. It returns `AnyType` for any node it does not model —
`Identifier`, `MemberExpr`, `IndexExpr`, `MatchExpr`, `TryExpr`, `BlockExpr`,
`PipeExpr` (`types.py:366-368`). `AnyType.is_compatible` is always `True`
(`types.py:97-99`), so an `Any` on **either** side silences a check.

| Source | Inferred | Evidence |
|---|---|---|
| `42` / `3.14` / `"x"` | `int` / `float` / `str` | `types.py:318-325` |
| `b"x"` | `Any` | no bytes branch |
| `true` | `bool` | `types.py:326-327` |
| `none` | `none` | `types.py:328-329` |
| `[1, "x"]` | `[int]` (first element only) | `types.py:330-333` |
| `{1,2}` | `{int}` (`SetType`) | `types.py:334-337` |
| `(1,"x")` | `(int, str)` (`TupleType`) | `types.py:338-339` |
| `{a: 1}` | `{str: int}` (first pair only) | `types.py:340-345` |
| `1..10` | `[int]` | `types.py:346-347` |
| `x ?? y` | `strip-none(x)` | `types.py:474-475` |
| `x?.f` | `Optional(infer x)` | `types.py:476-477` |
| `a and b` | `bool` | `types.py:467-469` |
| `f(x)` for a known builtin | table-driven | `_BUILTIN_RETURNS` §375-442 |

`_infer_binary_op` (`types.py:444-480`): `+` on two `str` → `str`, on two
`list` → list of the union of elements, on two numerics → `int`/`float`; `/`
on two numerics → `float`; comparisons and boolean operators → `bool`;
bitwise on `bool` → `bool`, else `int`; anything else → `Any`.

**Return-type inference** does **not** happen here: an unannotated `def` has
`FunctionType.return_type = AnyType` (`types.py:836-838`), and a body is never
re-inferred to synthesize a return type. `def double(x) = x * 2` is accepted
with no return annotation and no inferred signature. (Contrast `types.md`
§10, which claims return inference — the checker does not implement it.)

---

## 4. Assignment and compatibility

Compatibility is `declared.is_compatible(actual)` (note the **direction**:
*expected* on the left, *got* on the right). The base implementation
(`types.py:97-99`) accepts on equality or when either side is `AnyType`.

| Case | Accepted? | Evidence |
|---|---|---|
| same type | ✅ | `Type.__eq__` per class |
| `anything` vs `any` | ✅ | `isinstance(other, AnyType)` / `isinstance(self, AnyType)` |
| `int → float` (widening) | ❌ | `IntType().is_compatible(FloatType())` is `False` (*probe*) |
| `bool → int` | ❌ | `BoolType().is_compatible(IntType())` is `False` (*probe*) |
| `int → int?` (make optional) | ✅ | `UnionType.is_compatible` is `any(...)` (*probe*) |
| **`int? → int`** (unwrap optional) | ✅ **unsoundly** | `UnionType({int,None}).is_compatible(int)` is `True` (*probe*) |
| `int &#124; str → int` | ✅ | union accepts each member |
| `int → int &#124; str` | ❌ (as *declared*) | `IntType().is_compatible(union)` is `False` (*probe*) |
| `[int] → [int]` | ✅ | `ListType.is_compatible` recurses element |
| `[int] → [str]` | ❌ | `ListType.is_compatible` recurses element |
| `{str:int} → {str:str}` | ❌ | `DictType.is_compatible` recurses both slots |
| structural `{name: str}` (any dict) | ✅ always | checker collapses braces to opaque `DictType` |
| any class pair | ✅ (name or inherent) | `ClassType.__eq__` is by name only |

There are **no numeric coercions**: an `int` literal assigned to a `float`
variable is `E101` (*probe*: `let y: float = 1` records `expected Float, got
Int` when the annotation is present and the value is a bare int literal).
Widening is not modeled.

### 4.1 Where assignment is checked

- **Variable** with annotation: `_check_var_decl` (`types.py:781-794`) → `E101`
  on mismatch; the declared type wins in `context` even after the error.
- **Constant**: `_check_const_decl` (`types.py:796-807`) → `E101`.
- **Return**: `_check_return` (`types.py:1267-1279`) → `E101` when the
  enclosing function has a **non-`Any`** annotated return type.
- **Arguments**: `_check_call_expr` (`types.py:1431-1448`) → `E106`.
- **Arity**: `E105` for too many, or fewer than the required minimum
  (`types.py:1408-1430`).
- **Conditions** (`if`/`while`/`until`/`assert`/ternary/match guard):
  `_check_condition` (`types.py:1333-1342`) → `E101` when the inferred type is
  one of `int/float/str/list/dict/set/tuple/none`.
- **Operators**: `_check_binary_op` (`types.py:1344-1390`) → `E108`.

An annotation with an **unknown name** (`let x: Unknwn = 1`) is **not** an
error: `_parse_type_annotation` returns `AnyType` (`types.py:1492-1494`), and
the declaration is accepted (*probe*). This is intentional gradual typing.

---

## 5. Narrowing (union / optional)

`_narrowings` (`types.py:1030-1074`) recognizes exactly two forms, both on a
**bare identifier** on the left:

| Condition | then-branch | else-branch |
|---|---|---|
| `x != none` / `x is not none` | `strip-none(x)` | `none` |
| `x == none` / `x is none` | `none` | `strip-none(x)` |
| `x is T` (T a type name) | `T` | unchanged |
| `x is not T` | unchanged | `T` |

Narrowing is applied by `_with_narrowing` only to that branch's body and is
restored afterwards (`types.py:1017-1028`). There is **no** narrowing via
`and`/`or`, a ternary, a `guard` condition, or a re-assignment. `strip-none`
removes `None` from a `UnionType` set (`types.py:499-503`).

Because `int? → int` already passes §4, narrowing changes **analysis precision
only**, not acceptance: `let y: str = x` for `x: str?` is accepted even without
narrowing (*probe*). The check that *is* real is the reverse direction (a
non-optional value used where an optional is expected — always accepted).

---

## 6. Return types

- A declared return type non-`Any` is checked against the inferred type at each
  `return` (`E101`).
- An **unannotated** function has return type `Any`; every `return` is accepted.
- A function with a declared non-`Any` return and **no** `return` anywhere is
  **not** reported by the type checker (`E304 MISSING_RETURN` is documented as
  not enforced; the *Removed codes* table in [ERRORS.md](../ERRORS.md)). The emitted Python simply returns `None`.
- Return-type inference from the body is **not** implemented (§3).

```aura
def f() -> int { return "x" }   // E101: expected Int, got String
def g() -> int { }              // accepted; returns None at runtime
def h() { return "x" }          // no annotation; no check
```

---

## 7. Generics and constraints

- **Erased.** A type parameter resolves to `TypeVariable` only **inside** its
  declaring scope (`types.py:811-815, 871-874`). `TypeVariable.is_compatible`
  is unconditionally `True` (`types.py:277-279`), so a `T`-typed parameter
  accepts anything.
- `Box[int]` is **not** a parameterized `ClassType`; the bracket name falls
  through to `AnyType` (`types.py:1492-1494`). Generic arguments are not
  checked anywhere.
- **Constraints are only name-resolved.** `_check_type_constraints`
  (`types.py:653-691`) reports `E110` when a constraint is not a builtin, a
  declared class/trait, or a union of those. It does **not** check that a
  supplied type argument satisfies the constraint — call-site enforcement is
  **UNSPECIFIED/absent**.
- A declared-but-unused parameter is the warning `W103`
  (`UNUSED_TYPE_PARAMETER`, `types.py:899-915`).
- The emitted Python for a generic class does preserve machinery:
  `class Box(_aura_Generic[_aura_TypeVar('T')])` (*probe*) — but this is emit,
  not checking.

---

## 8. What the checker enforces, per code

Codes live in `aura/transpiler/errors.py`; the `E1xx`/`W1xx` catalog is mirrored
in `docs/ERRORS.md` and guarded by `tests/test_diagnostics.py`.

| Code | Name | Detects | Site |
|---|---|---|---|
| `E101` | `TYPE_MISMATCH` | variable/constant/return/condition declared-vs-actual mismatch | `types.py:788,803,1276,1338` |
| `E105` | `WRONG_ARGUMENT_COUNT` | too many args, or fewer than required | `types.py:1416,1427` |
| `E106` | `WRONG_ARGUMENT_TYPE` | argument incompatible with a declared param type | `types.py:1445` |
| `E108` | `INCOMPATIBLE_OPERANDS` | `+`/`-`/`*`/`/`/`%`/`**`, comparisons, bitwise on wrong operands | `types.py:1360,1371,1379,1387` |
| `E109` | `NON_EXHAUSTIVE_MATCH` | non-exhaustive `match` over `bool`/enum/int/str (**warning**) | `types.py:1134-1161` |
| `E110` | `UNKNOWN_TYPE_CONSTRAINT` | generic constraint name does not resolve | `types.py:666,686` |
| `W103` | `UNUSED_TYPE_PARAMETER` | class type parameter never used (**warning**) | `types.py:911-915` |

All six `E1xx` codes are the **entire** type-error surface. `E0xx` are parser
syntax errors; `E3xx` are structural rules from `RuleChecker`.

### 8.1 What is **not** enforced (erased or unchecked)

- type-argument compatibility (`Box[int]` vs `Box[str]`);
- element types of list/dict/set literals **in assignments**:
  `let a: [int] = [1, "x"]` is accepted (*probe*);
- structural brace-type shapes: any `dict` matches `{name: str, age: int}`;
- numeric widening/narrowing and `bool`/numeric coercion (no coercion model);
- operator legality on user classes (no `__add__`-style resolution);
- undefined names, non-callable calls, unknown type names (left to Python);
- `bytes` as a checked type (resolves to `Any`);
- type aliases (`TypeDecl` is skipped, `types.py:719-720`);
- mutability — that is the separate mutability checker (`E303`);
- abstract/visibility/inheritance — `RuleChecker` (`E309`/`E316`/`E308`/…).

---

## 9. Erasure: annotations and the Python emit

Aura transpiles to Python. `_aura_type_to_python`
(`statements.py:770-798`) maps an annotation to a *runtime name*; the
annotation does **not** reach Python as a type check.

| Aura annotation | Python emitted |
|---|---|
| `int`/`float`/`str`/`bool`/`bytes` | `int`/`float`/`str`/`bool`/`bytes` |
| `none` (or `null`) | `None` |
| `any` | `object` |
| `Never` | `type(None)` |
| `[T]` | `list` |
| `{K: V}` | `dict` |
| `[T]` uniform braces | `list` |
| `(A) -> B` or other `->` | `object` |
| a union `T &#124; U` | the **literal** `T &#124; U` text |
| a bare name | that name |

Consequences, verified by *probe*:

- `let x: int = 1` emits exactly `x = 1` — **no annotation**.
- `def f(a: int) -> str { ... }` emits `def f(a):` — parameter and return
  annotations are **dropped**.
- `let y: str? = none` emits `y = None`.

So the *only* type information that survives to runtime is (a) the generic
machinery on generic classes, (b) the best-effort alias/name emitted for
`type`, and (c) the union text. Whether an annotation is a **compile-time
check** or an **erased annotation** therefore matters: the checks in §8 run
only under `aura check` / LSP / REPL, and never at `aura run`.

---

## 10. Not a guarantee: quick list

- Assignment compatibility is **not** a safety net: unknown names and `Any`
  silently pass; the optional-unwrap direction is accepted.
- Inference is **context-free** and first-element-only for collections.
- Generics are erased on both sides; constraints are cosmetic.
- Structural types are opaque dicts.
- The checker runs at `aura check`, not at `aura run`/`aura transpile`.

Where a rule is not stated above, it is **UNSPECIFIED** rather than assumed.
