# Value-Algebra Decision — `json_encode` of unrepresentable kinds

**Status:** DECIDED AND IMPLEMENTED in the Aura 0.3 Keystone development line
(this commit), on the principle already normative for map keys. This document
records the evidence, the alternatives, and why no silent approximation
survives into the 0.3 contract. It is a decision record, not a second
specification: the normative rule lives in `docs/LANGUAGE_SPEC.md` §24 (JSON
builtin notes under the standard-library section).

## Finding

`json_encode` mapped every value kind with no exact JSON form to `null`, and
flattened enum variants to their payload:

| Aura value | Old encoding | Collides with |
|---|---|---|
| `0..3` (range) | `null` | `none`, over-depth truncation |
| `len` (native fn) / `() -> 1` (closure) | `null` | `none` |
| `to_float("nan")`, `to_float("inf")` | `null` | `none` |
| `A(1)` of `enum E { A(int), B(int) }` | `1` | `1`, and `B(1)` |
| `B()` of `enum E { A, B }` | `"B"` | `"B"` |
| `B(1, 2)` | `[1, 2]` | the list `[1, 2]` |

Reproduced against the built CLI:

```aura
print(json_encode(0..3))              // null
print(json_encode(len))               // null
print(json_encode(to_float("nan")))   // null
print(json_encode(A(1)) == json_encode(1))     // true (tag erased)
print(json_encode(B()) == json_encode("B"))    // true (kind erased)
```

Because `null` is also the encoding of `none` (and of the documented
over-depth truncation, spec §31.6), `json_decode(json_encode(x))` collapsed
`0..3`, a closure, `nan`, and `none` into a single value. Flattening variants
made two distinct values of one enum (`A(1)` and `B(1)`) encode identically.
`json_decode` has no variant kind at all and `json_decode_as` resolves only
struct/map/list/scalar spellings, so no decode path could reconstruct the lost
identity.

The released `0.2.1` artifact contains the same behavior. Historical artifacts
are immutable and are not modified; this decision governs the 0.3 development
line only.

## Prior normative stance (unchanged, and the model for this decision)

The map-key rule already settles the project's stance for the analogous case:

> `json_encode` encodes a `string`-keyed Aura map as a JSON object unchanged,
> but a map with any non-string key is a deterministic `E3001`: the key is
> never stringified, which would collapse distinct keys such as `1` and `"1"`.

That rule refuses an approximation that would lose identity. `json_encode` of
a range/function/non-finite-float/variant is the same failure shape one level
up: the *value* kind has no exact form, and the convenient approximation
(`null`, or the payload) loses identity.

## Decision

`json_encode` encodes exactly the kinds JSON represents without loss — `none`,
`bool`, `int`, finite `float`, `string`, lists, string-keyed maps, and structs.
Every other kind — enum variant, `range`, function (closure or native), and
non-finite float — is a deterministic `E3001` naming the offending kind. The
rule applies at every depth. Over-depth and node-budget truncation (spec
§31.6) is unchanged: it is a host-safety bound on an otherwise representable
value.

This is the single canonical path: one rule, stated once, applied uniformly to
the top-level value and to any nested value. It introduces no JSON shape the
language does not already define.

## Alternatives considered

1. **Tagged variant encoding** (`{"tag":"A","payload":[1]}`). Preserves
   variant identity but invents a JSON shape the spec never defined, changes
   more released behavior, and still has no decoder counterpart — the
   asymmetry remains, only smaller. Rejected as invented syntax.
2. **Keep the lossy flattening as documented legacy.** Preserves the released
   bytes but leaves a proved identity collapse in the 0.3 contract, directly
   contradicting the Keystone anti-collapse invariant. Rejected.
3. **Stringify unrepresentable kinds** (`"0..3"`, `"<fn>"`). Collides with
   real strings (`"0..3"`, `"<fn>"`) and gives a value a representation the
   decoder cannot invert. Rejected.

## Evidence

* Rule: `src/stdlib/ext.rs` (`to_json_depth`).
* Regression tests: `tests/keystone_json.rs`
  (`json_encode_rejects_a_range_rather_than_emitting_null`,
  `json_encode_rejects_a_function_rather_than_emitting_null`,
  `json_encode_rejects_a_variant_rather_than_flattening_it`,
  `json_encode_rejects_a_non_finite_float_rather_than_emitting_null`,
  `json_encode_rejects_a_nested_unrepresentable_value`,
  `json_encode_still_encodes_representable_values_unchanged`,
  `json_encode_rejection_is_an_ordinary_diagnostic`).
* Unchanged precedent: `tests/maps.rs::json_does_not_stringify_non_string_keys`.
* Normative text: `docs/LANGUAGE_SPEC.md`, JSON builtin notes.
* Frozen artifacts: not modified; `playground/runtimes/0.2.1` bytes unchanged.
