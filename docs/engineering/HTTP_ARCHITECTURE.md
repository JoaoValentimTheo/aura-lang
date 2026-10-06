# HTTP Architecture (Keystone §21)

Status: implemented behind the non-default `http` feature. This document is
the architecture and security record; the behavior contract lives in
`docs/LANGUAGE_SPEC.md`.

## 1. Architecture

HTTP is **not** hardwired into the evaluator. It follows the capability
pipeline the rest of the language uses:

```
Aura API (`http_request`)
→ typed request/response layer            (this module)
→ Host capability (`Host::http_request`)  (src/host.rs)
→ HTTP provider (ureq + rustls)           (feature `http`)
→ network
```

The evaluator never opens a socket. A `Host` that does not implement the HTTP
capability returns `HostError::Unavailable`, which surfaces as `E5002`. A
restricted host therefore denies network access by *not providing it* — the
denial is a capability fact, not a language flag.

## 2. Feature gating

`http` is **not** a default feature:

- The default, WASM, and `--no-default-features` builds link no HTTP code and
  no network dependency at all. `cargo build` on the canonical no-default
  matrix (`--features cli,repl,json,regex,time`) has no network surface.
- Enabling `http` adds exactly one direct dependency (`ureq`, `rustls`
  backend, no OpenSSL). The transitive inventory is recorded in §5.

## 3. Capability model

`Host::http_request(&self, request) -> HostResult<HttpResponse>`.

| Host | HTTP |
|---|---|
| `StdHost` (native default) | available |
| `LimitedHost` (WASM, embedder-restricted) | `Unavailable` → `E5002` |
| `BrowserHost` (WASM playground) | `Unavailable` → `E5002` |
| A test/embedder host | whatever it implements |

Because the capability is a `Host` method with a default `Unavailable`
implementation, every existing host keeps compiling and denies network by
default. Adding a permissive capability requires a deliberate host selection.

## 4. Failure domains

The typed layer keeps the failure classes distinct, matching the five-level
separation of the exception architecture:

| Situation | Result |
|---|---|
| Host has no HTTP capability | `E5002` (capability unavailable) |
| Connection refused / DNS failure / TLS failure | `E4020` (I/O) with the provider's reason |
| Timeout | `E4020` with a timeout message |
| Response body over the limit | `E4020` (resource protection) |
| Malformed response the provider rejects | `E4020` |
| JSON body decoded with a wrong shape | `E4031` (via `json_decode_as`) |
| A `throw` in Aura | user exception, unchanged |

No network or decode failure becomes a catchable user exception: they are
ordinary diagnostics (level separation, E4/E5).

## 5. Dependency and security review

Direct dependency: `ureq` 3.x with `default-features = false`,
`features = ["rustls"]`.

- **TLS**: `rustls` (pure Rust; no OpenSSL, no vendored C TLS). `ring` is the
  crypto backend; it is compiled from source with C/asm, which is the
  standard rustls configuration.
- **No proxy environment reading by default**: ureq's default features (which
  include proxy/`native-tls` behaviors) are off.
- **Certificate roots**: `webpki-roots` (Mozilla's CA set), not the platform
  store, so trust behavior is deterministic across machines.
- **Redirects**: the typed layer does not follow redirects; a 3xx is returned
  to Aura as an ordinary response and the program decides. This avoids
  implicit authority expansion (a redirect to an unexpected host).
- **Request limits**: a response body is bounded (`MAX_HTTP_BODY_BYTES`, 8
  MiB). A larger body is a deterministic `E4020`, never unbounded memory.
- **Timeouts**: every request carries a connect and total timeout
  (`MAX_HTTP_TIMEOUT_MS`), so a hung peer cannot hold a program forever.
- **Methods**: only the documented set is reachable from Aura (`GET`, `POST`,
  `PUT`, `DELETE`, `PATCH`, `HEAD`). A request never carries ambient
  credentials; headers are exactly what the program passed.
- **SSRF**: this layer provides network *capability*, not network *policy*. An
  embedder that must restrict destinations does so at the `Host` boundary by
  implementing its own `http_request`. Documented, not pretended.
- **No secrets**: nothing reads the environment. `std::env` is never
  consulted for proxy, token, or CA configuration.

The dependency inventory (names only, from `Cargo.lock`, `http` enabled):
`aura-lang, base64, bytes, cc, cfg-if, getrandom, http, httparse, itoa,
libc, log, once_cell, percent-encoding, ring, rustls, rustls-pki-types,
rustls-webpki, shlex, subtle, untrusted, ureq, ureq-proto, utf8-zero,
webpki-roots, zeroize` (plus platform stubs).

## 6. What is not implemented

- No async runtime: requests are synchronous, matching the language.
- No streaming body API: a response body is bounded and materialized.
- No redirect following, no cookies, no connection pooling controls exposed
  to Aura, no proxy configuration.
- No WASM HTTP: the browser substrate denies the capability.
