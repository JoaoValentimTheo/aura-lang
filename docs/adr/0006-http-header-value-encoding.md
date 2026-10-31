# ADR-0006 — HTTP header value encoding is ISO-8859-1 in both directions

- **Status:** Accepted (Architecture Decision Council, 2026-11-01)
- **Supersedes:** nothing. Fills a gap: the specification constrained header
  *names* and *collection shape* but never header *value encoding*.
- **Related:** `docs/LANGUAGE_SPEC.md` §25 (the HTTP normative rule and the
  browser-divergence rule), `docs/engineering/HTTP_ARCHITECTURE.md` §8,
  `src/host.rs` (`HttpResponse.headers`, `http::perform`), `src/stdlib/ext.rs`
  (`build_request`, `response_value`), `playground/web/worker.js` (the request
  `Headers` construction and `response.headers.forEach`),
  `playground/runtime/src/lib.rs` (`headers`/`header_lines` projection),
  `tests/http.rs`.

## Context

HTTP/1.1 (RFC 9110 §5.5) defines a header field value as a sequence of
`field-vchar` characters, where `obs-text` is explicitly allowed and defined as
the octets `%x80-FF`. A conforming parser therefore hands the value to the
application as a byte string, not as UTF-8. Practically every transport decodes
those octets as **ISO-8859-1** (one octet, one code point), which is exactly
reversible and loses nothing.

Aura had no rule for this, so the two hosts disagreed — and the native host
lost data. Measured on the same response bytes
`X-Latin: caf\xe9` (octet `0xE9`, valid `obs-text`, invalid UTF-8):

| Direction | Native (measured) | Browser (measured) |
|---|---|---|
| Response value | `""` — length `0`, value silently erased | `"café"`, code points `[99, 97, 102, 233]` |
| Request value `"café"` | wire octets `0xC3 0xA9` (UTF-8) | wire octet `0xE9` (ISO-8859-1) |
| Request value `"日本語"` | accepted, sent as `\xE6\x97\xA5…` | `TypeError` → reported as `E4020` |

The decisive experiment is the round trip: read a non-ASCII header and send it
back unchanged. The browser reproduces the octet losslessly. The native host
read `""` and re-sent `x-latin: ` with an empty value — the value was
**destroyed** and no error was raised. Silent data loss on a security-adjacent
surface is a defect, not a divergence.

Root causes, both in the native provider:

- `src/host.rs` converted each response value with
  `value.to_str().unwrap_or("")`. `HeaderValue::to_str` returns `Err` if **any**
  octet fails `is_visible_ascii` (visible ASCII or `\t`), so every `obs-text`
  value collapsed to the empty string. The `AsBytes`/`as_bytes` view — the
  always-correct raw source — was never consulted.
- `src/host.rs` passed the `String` directly to `ureq`'s
  `RequestBuilder::header`, whose `HeaderValue: TryFrom<&String>` stores the
  string's **UTF-8** octets. No encoding step and no range check existed, and
  `src/stdlib/ext.rs` `parse_headers` validated only list *shape*.

Two facts bound the decision. First, the browser half is not free to change: the
`fetch` `Headers` API decodes response bytes as ISO-8859-1 and encodes request
values as **ByteString**, throwing `TypeError: Cannot convert argument to a
ByteString because the character at index N has a value of V which is greater
than 255` for any code point above `U+00FF`. `playground/web/worker.js` already
converts that throw into `E4020`, with the message
``invalid request header `${k}` ``. Second, `ureq`'s `RequestBuilder::header`
accepts any `V` where `HeaderValue: TryFrom<V>`, and `HeaderValue::from_bytes`
accepts `obs-text`; so a latin-1 value can be handed to the builder with no new
dependency.

## Decision

1. **Header field values are ISO-8859-1, in both directions, on every host.**
   Each octet `0x00-0xFF` corresponds to the code point of the same numeric
   value. The conversion is a pure widening and is therefore lossless and
   exactly reversible.

2. **Response side: decode, never drop.** A received value's octets are
   decoded one-to-one into the corresponding code points. A value is *never*
   discarded, and a value that is not valid UTF-8 is not an error — `obs-text`
   is part of the grammar. `X-Latin: caf\xe9` is the string `"café"` (four
   characters), on both hosts.

3. **Request side: encode, and bound.** Each code point of a value is emitted
   as the single octet of the same numeric value. A code point above `U+00FF`
   has no ISO-8859-1 representation and is reported as `E4020` with a message
   naming the offending header. This matches the browser, which cannot encode
   such a value at all.

4. **The check lives in `build_request`, not in the host.** `build_request` in
   `src/stdlib/ext.rs` is the documented single validation point shared by the
   synchronous and resumable paths ("so validation never diverges"), and it is
   gated on `http-api` while the native provider is gated on `http`. Locating
   the check there makes it run on browser builds too — builds that never
   compile `src/host.rs` — so no host relies on its transport's own guard.

5. **No `header_bytes` view is added to the response map.** Unlike `body`,
   whose text view is genuinely constrained by UTF-8 validity, the ISO-8859-1
   header view is already exact and reversible, so a second key would be
   redundant. The response map keeps exactly `status`, `body`, `body_bytes`,
   `headers`, and `header_lines`.

6. **`header_lines` and `headers` carry the identical decoded values.** They
   differ only in shape (flat ordered list vs. lowercased-name map), never in
   value encoding, so a program cannot observe two encodings of the same
   exchange.

## Consequences

- The native host no longer erases any received header value. This is strictly
  additive for correct programs and is a bug fix for programs that previously
  saw `""`.
- A native program that today sends a header value containing a code point
  above `U+00FF` will now receive `E4020` instead of silently sending UTF-8
  octets. This is the intended parity: the browser host already fails on that
  input, so the pre-change native behaviour was the divergent one. Any program
  needing to transport non-Latin-1 bytes must encode them (base64, percent
  encoding) as the wire format requires, on both hosts alike.
- For a request value that is ASCII, nothing changes: the octets are identical.
  For a value containing `U+0080-U+00FF`, the wire octets change from the UTF-8
  sequence to the single latin-1 octet. This is a compatibility-relevant wire
  change and is recorded as such in `CHANGELOG.md`.
- The browser substrate needs no change for the response direction: `worker.js`
  already receives ISO-8859-1-decoded strings from `Headers`, so
  `playground/runtime/src/lib.rs` and `worker.js` are unchanged. The request
  direction likewise keeps its existing `E4020` guard, which the new native
  check now mirrors.
- The specification gains a normative encoding rule and an `E4020`
  out-of-range rule; both hosts are now described by one contract with no
  encoding divergence to document alongside the three sanctioned ones.

## Evidence

Reproducible measurements (raw sockets and `fetch`, macOS, `undici`/`Node`):

- Native response erasure: a loopback server sending `X-Latin: caf\xe9`
  produced `len(r["headers"]["x-latin"]) == 0`. After the fix it is `4` and the
  value equals `"café"`.
- Native round trip: reading that value and re-sending it produced the wire
  line `x-latin: \r\n` — the value was gone. After the fix the octet `0xE9`
  round-trips unchanged.
- Native request octets: `"café"` was sent as `0xC3 0xA9`; `"日本語"` was
  accepted and sent as `\xE6\x97\xA5…`. After the fix the first is `0xE9` and
  the second is `E4020`.
- Browser response decode: `[99, 97, 102, 233]` — ISO-8859-1, unchanged by this
  decision, and the measurement the native side now matches.
- Browser request octet: `0xE9` for `"café"`; `TypeError` above `U+00FF`,
  surfaced by `playground/web/worker.js` as `E4020`.

Regression tests pinning the contract live in `tests/http.rs`
(`non_ascii_response_header_value_is_preserved`,
`outgoing_non_ascii_header_value_uses_latin1_octets`,
`header_value_above_latin1_is_reported_not_sent`).
