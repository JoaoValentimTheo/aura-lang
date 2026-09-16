# Security Policy

## Reporting a vulnerability

Please do **not** open a public issue for security problems. Report them
privately via GitHub's [private vulnerability reporting](https://github.com/JoaoValentimTheo/aura-lang/security/advisories/new)
or by emailing the maintainers. Include a reproducer and the affected version.

We aim to acknowledge reports within a few days and to coordinate disclosure.

## Supported versions

Aura is alpha software. Security fixes land on the latest release.

| Version | Supported |
|---------|-----------|
| 0.1.0a5 | yes       |
| < 0.1.0a5 | no      |

## Trust model — read this first

Aura transpiles to Python and then executes that Python. **Running an Aura
program is equivalent to running arbitrary Python** with the privileges of the
user running it. There is no sandbox.

Consequences:

- `aura run untrusted.aura` executes arbitrary code. Do not run untrusted Aura
  source.
- `import` in Aura imports any Python module or installed package.
- `stdlib.python` (`python.eval`, `python.exec_code`, `python.import_module`,
  `python.add_path`) is an explicit, documented escape hatch to full Python.
- `stdlib.io`, `stdlib.os` and `stdlib.json` accept arbitrary file paths; there
  is no filesystem jail in the language runtime. (The bundled `aura_ide`
  workspace applies a sandbox when it shells out.)
- `stdlib.os.env()` returns environment variables; pass an allowlist to avoid
  leaking secrets.

## Hardening in place

The standard library includes defense-in-depth for common pitfalls:

- **HTTP** (`stdlib.http`): only `http`/`https` schemes; loopback, link-local,
  private and reserved addresses are rejected (`AURA_HTTP_ALLOW_PRIVATE=1` to
  opt out); redirects are **not** followed blindly (each hop is re-validated);
  response bodies are capped (`AURA_HTTP_MAX_BYTES`, default 32 MiB).
- **JSON** (`stdlib.json`): `dump`/`dumps`/`pretty` reject `NaN`/`Infinity`
  (`allow_nan=False`), so output is always valid JSON.
- **Dependencies** (`aura.tools.deps`): `aura add`/`aura install` validate names
  and reject specifiers that could smuggle a pip option; manifest values are
  escaped against TOML injection.
- **LSP** (`aura.lsp`): incoming messages are size-bounded and the open-document
  cache is capped.

## Cryptography

Cryptography lives in `aura.stdlib.crypto` (see `docs/AUP.md` and the module
docstring). Design principles:

- **No custom primitives for real security.** The bundled pure-Python post-
  quantum reference backend (ML-KEM/Kyber, ML-DSA/Dilithium, SLH-DSA/SPHINCS+)
  is provided for **interoperability, education and testing**, not for
  production secrets. Install the `pqc` extra (`pip install "aura-language[pqc]"`)
  to use vetted implementations from `cryptography` when available.
- **Constant-time comparison** (`constant_time_compare`) is used for secrets.
- **Randomness** comes from `secrets`/`os.urandom`, never `random`.

If a primitive is unavailable, the module raises a clear error rather than
silently degrading to an insecure fallback.

## Dependency policy

The runtime has no mandatory third-party dependencies. Optional extras:

- `dev`: test and lint tooling.
- `pqc`: post-quantum cryptography backend.

## Scope

In scope: injection (TOML/JSON/command), SSRF, path traversal in bundled tools,
unsafe deserialization, weak randomness, and cryptographic misuse.

Out of scope: that Aura programs can execute arbitrary Python (by design), and
the security of code a user chooses to run.