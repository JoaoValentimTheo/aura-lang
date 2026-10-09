# Python interoperability

Aura can call CPython through the optional **`py`** feature (PyO3). This is real
interoperability — not a Python-compatibility claim about Aura itself. Aura is
not Python; it can *call* Python when built with `py`.

> **Security.** The `py` feature grants arbitrary Python code execution with
> full host authority. It is **not** a sandbox, and none is claimed. Do not run
> untrusted Python through it. Builds without `py` link no CPython at all
> (`aura version` reports the same version; `py_*` calls report `E5002`).

## Enabling it

Build or install with the `py` feature (see [Installing Aura](/docs/install/)).

```
cargo build --release --features py
```

`py_version()` reports the linked interpreter; `py_eval`, `py_import`, and
`py_call` cross the boundary.

```aura
fn main() {
    print(py_version())
    print(py_eval("1 + 2 * 3"))
    print(py_call("math", "sqrt", 16.0))
}
```

## Value conversion

Conversion matches on **type identity**, not on whether a coercion would
succeed: a value crosses only when it is an instance of the builtin scalar
(`bool`/`int`/`float`/`str`). A user object implementing `__float__`,
`__index__`, `__bool__`, or `__str__` is **not** silently coerced — it takes the
`repr` fallback like any other opaque object. This prevents silent precision
loss and stops a raising dunder from being swallowed.

| Aura | Python |
|---|---|
| `int` | `int` (in `i64` range; outside is `E4013`) |
| `float` | `float` (including `inf`, `-inf`, `nan`) |
| `bool` | `bool` |
| `string` | `str` (UTF-8, including astral scalars) |
| `list` | `list` |
| `map` | `dict` (keys must be `string`/`int`/`bool`) |
| `none` | `None` |
| other Python objects | Aura `string` of their `repr` |
| non-mappable Aura values (struct/enum/fn/range) | rejected `E5002` |

A Python dict key that is not a genuine `bool`/`int`/`str` (for example a user
object with `__index__`) is rejected `E5002`, never coerced.

## Exceptions and diagnostics

A Python exception crossing into Aura is a diagnostic (`E5001`) that carries
the **Aura call-site** source location, not `1:1`. Failures are fatal and
uncatchable in v1, and a failed call leaves the interpreter usable for later
calls.

## Supported CPython versions

We advertise only what CI verifies:

- **TESTED:** Linux 3.10, 3.11, 3.12, 3.13; macOS 3.12; Windows 3.12.
- **SUPPORTED (best effort):** other CPython 3.10–3.13 lines.
- **UNSUPPORTED:** CPython ≤ 3.9; ≥ 3.14 until tested.

The full normative contract is in
[`docs/CPYTHON_COMPATIBILITY_TARGET.md`](https://github.com/JoaoValentimTheo/aura-lang/blob/v0.3.1/docs/CPYTHON_COMPATIBILITY_TARGET.md).
