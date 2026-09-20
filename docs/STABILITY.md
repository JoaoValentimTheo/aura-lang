---
layout: default
title: "Stability and Compatibility"
nav_order: 9
---

# Stability and Compatibility

This document defines what is stable, what is frozen, and what may change
in Aura before the 1.0 release. It is a **proposal** — the final contract
will be approved by the maintainer before 1.0-beta.

---

## 1. What is frozen

### Syntax (frozen since 0.2.0a1)

The grammar is frozen. No syntax changes will be made before 1.0. The
canonical source of truth is
[grammar.md](language-reference/grammar.md). Every grammar change will be
recorded in CHANGELOG.md.

**Already removed (will not return):**

| Removed construct | Replacement |
|-------------------|-------------|
| `class X(Y)` inheritance | `class X extends Y` |
| `implements Trait` | `class X extends Trait` |
| `fn(x) { }` function syntax | `def f(x) { }` |
| `//` as floor division | `int(a / b)` |
| `//=` as floor-division assign | `a = int(a / b)` |
| `if a then b else c` | `a ? b : c` |
| `extends` with `class X(Y)` syntax | `class X extends Y` |

If you encounter removed syntax in documentation or examples, open an
issue.

---

## 2. What becomes stable at 1.0

The following surfaces are **unstable today** but will receive stability
guarantees once 1.0 ships. Changes to these surfaces after 1.0 will follow
the deprecation policy in §4.

### 2.1 Standard library public API

Every function, class, and constant documented in
[docs/language-reference/](language-reference/index.md) and re-exported by
`aura.stdlib` is considered public. Internal names (prefixed with `_` or
living in `crypto_backend.py`) are explicitly **not** public.

**Public modules and their surface (summary):**

| Module | Public items |
|--------|-------------|
| `stdlib.math` | `PI`, `E`, `TAU`, `INF`, `NAN`, `abs`, `min`, `max`, `round`, `floor`, `ceil`, `sqrt`, `pow`, `log`, `log10`, `log2`, `exp`, `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `atan2`, `sinh`, `cosh`, `tanh`, `degrees`, `radians`, `gcd`, `lcm`, `factorial`, `comb`, `perm`, `is_finite`, `is_infinite`, `is_nan`, `copysign`, `fabs`, `fmod`, `fsum`, `prod`, `remainder`, `dist`, `hypot` |
| `stdlib.string` | `upper`, `lower`, `title`, `capitalize`, `reverse`, `trim`, `trim_left`, `trim_right`, `pad_left`, `pad_right`, `pad`, `repeat_string`, `split`, `join`, `starts_with`, `ends_with`, `contains`, `index_of`, `last_index_of`, `replace`, `slice_string`, `substring`, `char_at`, `format_string`, `is_empty`, `is_blank`, `length`, `bytes_from_string`, `string_from_bytes`, `unicode_at`, `char_from_code`, `is_alpha`, `is_alphanumeric`, `is_digit`, `is_space`, `is_lower`, `is_upper`, `is_numeric`, `lines`, `unlines`, `codes`, `from_codes` |
| `stdlib.collections` | `AuraDict`, `list_map`, `list_filter`, `list_reduce`, `list_find`, `list_any`, `list_all`, `list_take`, `list_drop`, `list_zip`, `list_flatten`, `list_unique`, `list_sort`, `list_reverse`, `list_chunk`, `dict_get`, `dict_keys`, `dict_values`, `dict_items`, `dict_merge`, `dict_filter`, `dict_map`, `set_union`, `set_intersection`, `set_difference`, `map`, `filter`, `reduce`, `take`, `drop` |
| `stdlib.itertools` | `range_iter`, `cycle`, `repeat`, `chain`, `combinations`, `permutations`, `product`, `count`, `enumerate_iter`, `islice`, `takewhile`, `dropwhile`, `groupby`, `filterfalse`, `starmap`, `tee`, `zip_longest`, `pairwise` |
| `stdlib.json` | `loads`, `dumps`, `load`, `dump`, `pretty`, `parse`, `stringify`, `is_valid`, `merge` |
| `stdlib.time` | `now`, `now_ms`, `sleep`, `clock`, `monotonic`, `perf_counter`, `strftime`, `parse`, `iso`, `timestamp`, `elapsed` |
| `stdlib.io` | `read`, `write`, `append`, `exists`, `is_file`, `is_dir`, `mkdir`, `ls`, `rm`, `rename`, `extension`, `basename`, `dirname`, `join`, `read_lines`, `write_lines`, `copy`, `size`, `touch` + `*_async` variants |
| `stdlib.os` | `get_env`, `set_env`, `unset_env`, `env`, `cwd`, `chdir`, `home`, `temp_dir`, `path_join`, `path_abspath`, `path_exists`, `path_is_file`, `path_is_dir`, `path_basename`, `path_dirname`, `path_split`, `path_splitext`, `path_normpath`, `sep`, `linesep`, `name`, `platform`, `pid`, `listdir`, `walk`, `makedirs`, `remove`, `rename`, `get_size` |
| `stdlib.http` | `request`, `get`, `post`, `put`, `delete`, `get_json`, `post_json`, `quote`, `unquote`, `build_url` + async variants |
| `stdlib.crypto` | `sha256`, `sha512`, `sha3_256`, `sha3_512`, `shake_256`, `blake2b`, `digest`, `available_hashes`, `hmac_sha256`, `hmac_sha3_256`, `hmac_sha512`, `hkdf_sha256`, `random_bytes`, `random_hex`, `random_int`, `constant_time_compare`, `kem_keypair`, `kem_encapsulate`, `kem_decapsulate`, `dsa_keypair`, `dsa_sign`, `dsa_verify`, `algorithms`, `backend_info`, `is_production_backend`, `require_production_backend` |
| `stdlib.regex` | `match`, `full_match`, `search`, `find_all`, `find_iter`, `split`, `replace`, `replace_fn`, `subn`, `groups`, `group_dict`, `group`, `escape`, `compile_pattern`, `purge`, `error`, `IGNORECASE`, `MULTILINE`, `DOTALL`, `VERBOSE`, `ASCII`, `UNICODE` |
| `stdlib.python` | `import_module`, `load`, `reload`, `is_available`, `eval`, `exec_code`, `compile_source`, `call`, `getattr`, `setattr`, `hasattr`, `dir`, `type_name`, `is_module`, `is_callable`, `is_class`, `is_instance`, `to_aura`, `to_python`, `builtins`, `py_builtins`, `interpreter_version`, `add_path`, `site_packages`, `modules`, `ModuleProxy` |
| `stdlib.testing` | `test`, `case`, `skip`, `equal`, `not_equal`, `is_true`, `is_false`, `is_none`, `is_not_none`, `contains`, `not_contains`, `starts_with`, `ends_with`, `raises`, `greater`, `greater_or_equal`, `less`, `less_or_equal`, `approx_equal`, `in_range`, `has_length`, `is_empty`, `fail`, `run_all`, `tests`, `clear`, `TestFailure` |
| `stdlib.macros` | `Macro`, `MacroError`, `MacroRegistry`, `Quote`, `contains_identifier`, `default_registry`, `gensym`, `is_identifier_named`, `literal_value`, `name_of`, `BUILTIN_COMPILE_TIME_MACROS`, `builtin_names` |
| `stdlib.threading` | `Thread`, `spawn`, `thread`, `lock`, `rlock`, `event`, `condition`, `semaphore`, `barrier`, `local`, `current_name`, `active_count`, `enumerate_threads`, `main_thread`, `get_ident`, `stack_size`, `run_many`, `map_concurrent` |
| `stdlib.asyncio` | `run`, `sleep`, `create_task`, `gather`, `wait_for`, `wait`, `as_completed`, `queue`, `lock`, `event`, `semaphore`, `current_task`, `all_tasks`, `is_running`, `new_event_loop`, `set_event_loop`, `get_event_loop` |

A function not listed here but present in a stdlib module is internal and
may change without notice.

### 2.2 CLI commands and flags

All 17 subcommands and their documented flags are part of the public API.
Undocumented flags are internal and may change.

| Subcommand | Stable flags |
|------------|-------------|
| `aura run` | `-v`, `--no-main` |
| `aura check` | `-v` |
| `aura transpile` | `-o`, `-v` |
| `aura format` | `-i`, `-o`, `--width` |
| `aura lint` | `-w`/`--allow-warnings` |
| `aura test` | `-v`, `-p`/`--pattern` |
| `aura repl` | (none) |
| `aura init` | `--no-venv` |
| `aura add` | `-V`/`--version`, `-D`/`--dev`, `--no-install` |
| `aura remove` | `--uninstall` |
| `aura install` | `--upgrade` |
| `aura deps` | `--lock` |
| `aura venv` | `-f`/`--force`, `--python`, `--no-install` |
| `aura doctor` | (none) |
| `aura version` | `major`, `minor`, `patch`, `x.y.z` |
| `aura debug` | `-t`/`--trace`, `-c`/`--show-code` |
| `aura lsp` | (none) |

### 2.3 Diagnostic codes (E##/W##)

All `E##` and `W##` codes documented in
[docs/ERRORS.md](ERRORS.md) are part of the public API. Code numbers and
severity levels are stable. Codes may be added (in the next available slot)
but never removed or renumbered. Removed codes (E201–E204, E304, E401,
E402, W101, W102) are documented as historical and will never be reused.

### 2.4 `aura.toml` schema

The `[project]` and `[dependencies]` sections of `aura.toml` are part of
the public API. New keys may be added in future minor versions, but
existing keys will not be removed or renamed before 2.0.

| Section | Key | Status |
|---------|-----|--------|
| `[project]` | `name` | Stable at 1.0 |
| `[project]` | `version` | Stable at 1.0 |
| `[dependencies]` | `<package>` | Stable at 1.0 |
| `[dependencies.dev]` | `<package>` | Stable at 1.0 |

### 2.5 LSP protocol

The LSP server (`aura lsp`) exposes these capabilities, which will be
stable at 1.0:

- `textDocument/hover`
- `textDocument/completion` (trigger characters: `.`, `:`)
- `textDocument/documentSymbol`
- `textDocument/definition`
- `textDocument/references`
- `textDocument/rename` (with prepare)
- `textDocument/formatting`

---

## 3. What is NOT stable

The following are **internal implementation details** and may change at any
time without notice or deprecation:

- **Generated Python code**: the format, naming, prelude helpers (`_*`
  functions/classes), and import structure of the Python output. Use
  `aura run` or `aura transpile` as a black box. Inspecting the output with
  `-v` is fine for debugging; depending on its format is not.
- **AST node names and structure**: `aura/transpiler/ast.py` is internal.
- **Parser internals**: `aura/parser/to_ast.py` tokenization and parse
  state are internal.
- **Module namespace mapping**: `aura/runtime.py` sys.modules aliases are
  an implementation detail.
- **Error message text**: the human-readable text of diagnostics may change;
  only the code (E##/W##) and severity are stable.
- **Exit codes**: the numeric exit codes of CLI commands are not yet
  standardized and may change before 1.0.

---

## 4. Versioning and deprecation policy

### Semantic versioning

Aura follows SemVer with pre-release labels:

```
0.Y.ZaN    — alpha (API may break between minor versions)
0.Y.ZbN    — beta (API changes only with justification)
0.Y.ZrcN   — release candidate (only critical fixes)
1.0.0      — first stable release
```

### Pre-1.0 rules

- **Minor version** (0.2 → 0.3): may include breaking changes to unstable
  surfaces (stdlib, CLI, diagnostics). Breaking changes are documented in
  CHANGELOG.md under a "Breaking" heading.
- **Patch version** (0.2.0a7 → 0.2.0a8): bug fixes only. No API changes.
- **Alpha/bump** (a7 → a8): may include breaking changes to unstable
  surfaces if the minor version has not changed.

### Post-1.0 deprecation process

Once 1.0 ships, deprecations follow this process:

1. **Deprecation introduced**: the old API continues to work but emits a
   `DeprecationWarning` (or CLI warning) naming the replacement and the
   version where removal is scheduled.
2. **Minimum exposure**: a deprecated API must remain available for at least
   two minor versions (e.g., deprecated in 1.1 → removed in 1.3 or later).
3. **Removal**: the deprecated API is removed. The removal is documented in
   CHANGELOG.md under "Removed".

Exceptions: security fixes may break backward compatibility without
deprecation if the affected behavior is a vulnerability.

---

## 5. Exit criteria: alpha → beta → rc → 1.0

### Alpha (current)

Alpha means the language and tooling are usable but the API is unstable.
Breaking changes between minor versions are expected.

**Remaining alpha work:**

- [ ] All documented stdlib functions have tests (currently ~95%)
- [ ] Grammar compliance test suite passing (task 2)
- [ ] Documentation code blocks compile (task 3)
- [ ] Fuzz campaign with zero unresolved crashes (task 4)
- [ ] Security audit complete with no open critical findings (task 5)
- [ ] 4+ real-world example programs passing CI (task 8)

### Beta

Beta means the API is frozen. Only bug fixes and additive changes are
permitted. Beta is entered when:

- [ ] All items in the alpha checklist are done
- [ ] STABILITY.md is approved by the maintainer
- [ ] No open issues labeled `breaking-change`
- [ ] 100% of documented stdlib functions have passing tests
- [ ] Release engineering pipeline is verified end-to-end (task 6)

### Release candidate

RC means the release is code-complete. Only critical (security/data-loss)
fixes are permitted. RC is entered when:

- [ ] Beta period has lasted at least 2 weeks
- [ ] No open issues labeled `critical` or `security`
- [ ] Full test suite passes on all supported Python versions (3.10–3.13)
- [ ] Documentation is complete and reviewed

### 1.0

Stable. The API contract in this document is in effect.

---

## 6. How to propose changes to this contract

Open a PR modifying this file with a clear rationale. For syntax changes
post-1.0, use the RFC process in
[docs/rfcs/](rfcs/). For stdlib or CLI changes, open an issue with the
`proposal` label.
