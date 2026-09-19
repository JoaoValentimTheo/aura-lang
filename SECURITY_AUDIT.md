# Security Audit — Aura Transpiler

Date: 2026-09-18
Auditor: Automated + Manual Review
Scope: Aura compiler/transpiler, CLI, LSP, stdlib (HTTP, IO, OS, crypto)

## Threat Model

### Assets
- **Source code confidentiality**: Aura source files on disk
- **Execution integrity**: Transpiled Python must behave as the Aura source intends
- **Developer machine**: The CLI process, filesystem, network
- **LSP server**: Editor integration, document contents

### Actors
- **Malicious Aura source**: A `.aura` file crafted to exploit the transpiler
- **Malicious HTTP server**: Responds with oversized bodies, SSRF payloads, redirects to private hosts
- **Supply chain**: A PyPI package installed via `aura add` that contains malicious code

### Trust Boundaries
1. Aura source → Parser/Tokenizer (untrusted input)
2. Transpiled Python → `exec()` (code generation)
3. HTTP stdlib → Network (untrusted servers)
4. Import hook → Filesystem (sibling `.aura` files)
5. LSP → Editor documents (untrusted content)

---

## Findings

### HIGH — Information Disclosure: Traceback Leaks

| Location | Issue | Status |
|---|---|---|
| `aura/lsp/server.py:199` | `traceback.format_exc()` sent as JSON-RPC error to editor | **FIXED** — replaced with sanitized message |
| `aura/cli.py:185` (cmd_transpile) | `traceback.print_exc()` to stderr | **FIXED** — removed |
| `aura/cli.py:557` (cmd_run) | `traceback.print_exc()` to stderr | **FIXED** — removed, exception type shown instead |

### HIGH — HTTP Response Body DoS

| Location | Issue | Status |
|---|---|---|
| `aura/stdlib/http.py:40,151` | `AURA_HTTP_MAX_BYTES=0` disables the 32 MiB cap | **FIXED** — 0 now falls back to default; only positive values accepted |
| `aura/stdlib/http.py:149` | `_read_limited()` with 0/None reads unlimited | **FIXED** — fallback 1 MiB safety cap |

### HIGH — LSP Bypasses MAX_SOURCE_BYTES

| Location | Issue | Status |
|---|---|---|
| `aura/lsp/server.py:126` | `_parsed()` calls `Parser(Tokenizer(text))` without size check | **FIXED** — size check added |

### MEDIUM — Venv Packages Not Importable at Runtime

| Location | Issue | Status |
|---|---|---|
| `aura/cli.py:106` | `_install_aura_imports()` sets up Aura import hook but does not add venv `site-packages` to `sys.path` | **FIXED** — venv site-packages prepended |
| `aura/tools/debugger.py:60` | Debugger `exec()` without venv path setup | **FIXED** — venv site-packages prepended |

### MEDIUM — SSRF TOCTOU (Theoretical)

The `requests` backend follows redirects manually, re-validating each hop. The `urllib` backend uses `_SafeRedirectHandler` that calls `_validate_url()` before each redirect. Both are correct in design. A theoretical TOCTOU exists between DNS validation and connect, but this is inherent to any DNS-based SSRF guard and not practically exploitable. **No fix needed** — documented as accepted risk.

### MEDIUM — Recursion DoS Budget

The `recursion_budget` context manager scales the limit to source size. Deeply nested constructs (>10K levels) could still exhaust the budget. The 16 MiB `MAX_SOURCE_BYTES` cap provides an upper bound. **Mitigated** by existing caps.

### LOW — Raw String Handling

Raw strings (`r"..."`) are parsed with correct backslash-quote handling (line 260 in `to_ast.py`). The `raw_literal` is passed verbatim to the transpiler, which outputs it as-is. This is safe because the same string literal is valid in both Aura and Python. **No vulnerability found**.

### LOW — Environment Variable Handling

`AURA_VENV` is used to override the venv directory. The value is used only for path construction (`Path(override)`) and is not passed to shell commands. Path traversal (`..`) in `AURA_VENV` could point outside the project, but this requires the attacker to control the environment. **Accepted risk** — environment variables are trusted input.

### LOW — Import Hook Security

The `AuraFinder` in `aura/transpiler/importer.py` resolves `.aura` files only within the project directory and its parents. Path traversal components (`..`) and empty names are rejected. Symlinks pointing outside the search roots are not followed. **Mitigated**.

### BY DESIGN — Code Execution via Transpilation

Aura transpiles to Python and executes via `exec()`. Any Aura code can:
- Read/write files via `stdlib.io` and `stdlib.os`
- Execute arbitrary Python via `stdlib.python.eval()` / `exec_code()`
- Make network requests via `stdlib.http`
- Spawn threads via `stdlib.threading`

This is by design — Aura is a general-purpose language, not a sandbox. The security model assumes Aura source code is trusted. **No fix needed**.

---

## Mitigations Summary

| # | Finding | Severity | Fix Applied |
|---|---|---|---|
| 1 | LSP traceback leak | HIGH | Sanitized error message |
| 2 | CLI traceback leaks | HIGH | Removed `traceback.print_exc()` |
| 3 | HTTP body DoS (0 = unlimited) | HIGH | Enforce positive values only |
| 4 | LSP bypasses size limit | HIGH | Added MAX_SOURCE_BYTES check |
| 5 | Venv packages not importable | MEDIUM | Added site-packages to sys.path |
| 6 | SSRF TOCTOU | MEDIUM | Accepted (inherent to DNS) |
| 7 | Recursion DoS | MEDIUM | Mitigated by MAX_SOURCE_BYTES |
| 8 | Raw string injection | LOW | No vulnerability found |
| 9 | Env var path traversal | LOW | Accepted (env is trusted) |

---

## Recommendations for Future Phases

1. **Add `--debug` flag** to CLI for verbose tracebacks during development
2. **Implement Content-Security-Policy** equivalent for HTTP stdlib (restrict schemes, methods)
3. **Add fuzzing CI** with Atheris/Hypothesis for parser and transpiler
4. **Consider process isolation** for `exec()` (subprocess sandboxing)
5. **Audit crypto stdlib** for timing attacks and key handling
