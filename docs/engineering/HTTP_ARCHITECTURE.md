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

## 7. PokéAPI dogfood (Keystone §11)

`tests/pokeapi_dogfood.rs` exercises the full chain — Aura program → HTTP →
`Host::http_request` capability → real transport → JSON → typed decode →
Struct/optional values — in two layers:

- **Deterministic (CI):** a loopback server serves captured PokéAPI
  documents through the real transport and the real decoders. Runs in the
  normal test matrix; never depends on a third party.
- **Live (opt-in):** `AURA_LIVE_POKEAPI=1 cargo test --locked --all-features
  --test pokeapi_dogfood -- --ignored` runs the same program against
  `https://pokeapi.co`, proving the fixture matches the live shape.

The deterministic layer proves: a 200 decodes typed; nested objects and
arrays resolve; an absent optional field (`string | none`) decodes to `none`;
a 404 stays a plain status with a decodable body; a shape mismatch is
`E4031`, is not `try`-catchable, and an undeclared field is rejected rather
than ignored.

**Recorded finding (not a defect).** PokéAPI's `types[].type` key is the Aura
reserved word `type` (`LANGUAGE_SPEC.md` §7), and `json_decode_as` requires a
JSON key to equal a declared field name exactly. The typed decoder therefore
cannot consume that specific nested object; the dogfood reads it through the
permissive `json_decode` map path instead. A field-rename or raw-identifier
mechanism would be new language syntax and is left to the human gate rather
than invented here.

**Environment note.** In the container this campaign ran in, outbound TLS is
throttled to ~30 s for processes other than the allowlisted `curl`/`node`
(the Aura/BrowserHost path times out at `MAX_HTTP_TIMEOUT_MS`). The live test
therefore cannot complete here; that is an environment fact, not a repository
defect. The deterministic layer exercises the identical Aura-side code path
with the identical response bytes.

## 8. Native foundations repaired (0.3.2 development)

Four verified defects in the native path were reproduced and fixed in this
campaign. The public response shape change is **additive only**: no existing
key changed meaning, so no compatibility decision is required.

- **Outgoing headers are applied.** `HttpRequest.headers` was populated by the
  stdlib but never applied to the outgoing `ureq` request, so a program that
  set headers silently sent none. The dispatch now applies every header in
  order; `tests/http.rs::outgoing_request_headers_reach_the_wire` captures the
  raw request on a loopback socket and asserts the header is on the wire.

- **Binary-safe body.** The response body was decoded with
  `String::from_utf8_lossy`, silently corrupting non-UTF-8 payloads.
  `HttpResponse` now carries `body_bytes: Vec<u8>` (authoritative) alongside
  the `body` text view. Aura sees `body_bytes` (a `[int]` of 0..=255) and
  `body` (the text convenience). `body_bytes_is_the_exact_binary_payload`
  pins a payload with bytes ≥ 0x80.

- **Repeated headers preserved.** The old conversion comma-joined every
  duplicate, which is wrong for `Set-Cookie`. `headers` now maps a name
  occurring once to its value and a name occurring more than once to a
  **list** of every occurrence in order; a new `header_lines` list preserves
  the exact wire order and casing. `repeated_headers_are_preserved_not_comma_joined`
  pins two `Set-Cookie` headers.

- **Method/body policy reported.** A body supplied for `GET`/`HEAD` (which by
  contract carry none) was silently discarded. It is now a caller error
  (`E4020`), dispatched only after the check, with no request sent.
  `DELETE` with a body uses `ureq`'s documented `force_send_body` escape
  hatch; `POST`/`PUT`/`PATCH` always send (an empty body when none is given).

The browser substrate still denies HTTP with `E5002`: browser HTTP requires the
cross-boundary parking driver described in
`PLAYGROUND_032_CAMPAIGN.md`, which is a pending human design decision. This
document does not claim browser HTTP.
