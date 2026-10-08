# Keystone Collection Model — Decision Package

**Status:** RESOLVED — implemented. See **ADR-0005**
(`docs/adr/0005-keystone-collection-model.md`), which records the accepted
decision, and `docs/LANGUAGE_SPEC.md` §21 for the normative semantics. This
document is retained as the historical decision package (the analysis that led
to ADR-0005), not as current authority.

**Resolution.** The human selected Option B (full reopening) with a specific
Array design: `[T; N]` is the Array **type** (semicolon-separated, `N` a
compile-time length), the literal stays comma-separated and is contextually
realized as Array under an `[T; N]` expectation, and `[1; 2; 3]` is not an
Array literal. `Tuple`, `Set`, `List`, `Array`, and `Map` are now distinct
identities, implemented end-to-end.

**Authority basis:** `docs/LANGUAGE_SPEC.md` §5.1 (value universe), §5.2 (`Ty`),
§5.3 (property matrix), §5.4 (families), §11 (equality), §20 (collections),
§21 (tuples), §25 (JSON); `docs/grammar.md`; `docs/engineering/JSON_VALUE_ALGEBRA_DECISION.md`;
`tests/keystone_value_algebra.rs`.

**Why this package exists.** The human Keystone target reopens a *deliberate*
earlier decision: the value algebra should be able to represent distinct
identities for `List`, `Array`, `Tuple`, `Set`, and `Map` rather than forcing
`Tuple` into `List`. This is a language-design reopening, not a bug.

**Repository evidence (what actually exists — reproduced, not assumed):**

| Kind | Source syntax | AST | `Ty` | Family | Runtime `Value` | JSON |
|---|---|---|---|---|---|---|
| List | `[a, b]`, `[]`, comprehensions | `Expr::List` | `Ty::List(T)` | `Sequence` | `Value::List` | array |
| Map | `{k: v}`, `{:}`, comprehensions | `Expr::Map` | `Ty::Map(K,V)` | `Mapping` | `Value::Map` | object (string-keyed) |
| Tuple | `(a, b)` | `Expr::Tuple` (identity-free) | **none** | **none** | lowers to `Value::List` | array (via list) |
| Set | **absent** | **absent** | **absent** | **absent** | **absent** | **absent** |
| Array | **absent** | **absent** | **absent** | **absent** | **absent** | **absent** |
| Range | `a..b` | `Expr::Range` | `Ty::Named("range")` | `RangeLike` | `Value::Range` | rejected (E3001) |
| String | `"…"` | `Expr::Lit`/`FStr` | `Ty::String` | `Scalar`+`Sequence` | `Value::Str` | string |

**History search result (exhaustive):** a repo-wide search of Git-tracked
`docs/`, `tests/`, `examples/`, `website/`, and `src/` found **no** historical
`Set<…>`, `Array<…>`, `{a, b}` set-literal, or `[N; T]` array design. The only
mentions are negative: `docs/engineering/KEYSTONE_SCOPE_MANIFEST.md:154`
("Aura has no `Array` and no `Set`; `Tuple` is list sugar") and
`tests/keystone_value_algebra.rs:10`. **Therefore the operands/meaning of an
Array form cannot be recovered from repository evidence and MUST NOT be
invented** (prompt §13/§14).

---

## 1. The decision the human must make

Five identities are wanted; the private tension is that Aura's existing
delimiters are already committed:

* `[...]` is the **list** literal and the **list/index** type and the **index**
  operator. A second bracketed collection needs either a new inner token or a
  new type spelling.
* `{...}` is a **block**, a **map**, a **struct construction**, a **match
  body**, and (in `impl`/`struct`/`enum`/`module`/`trait`) an **item list`.
  `map_ahead()` decides map-vs-block by a depth-0 `:`.
* `(a, b)` is currently the tuple-literal spelling and is normative list sugar.

### Option A — Minimal reopening: make `Tuple` distinct only

Admit a real `Tuple` identity; leave `Array` and `Set` out of 0.3.

* Keep `(a, b)` spelling; stop lowering it to `Value::List`.
* `Ty::Tuple(Vec<Ty>)`, `Value::Tuple(Vec<Value>)`, family `Sequence`.
* Adopt a **declaration-counting rule**: `(e)` stays a pure grouping (it is
  today), `(e,)` and `(e, …)` are tuples. This preserves every existing
  one-element grouping.
* Fixes the false statement "a tuple is a list" without opening `{…}`/`[…]`
  ambiguity.

*Cost:* removes the normative absorption; every `(a,b)`-as-list site (checker,
  runtime, JSON, AIS, tests, docs, website) must migrate atomically (prompt
  §11). Medium-large, well-bounded.

### Option B — Full reopening: distinct `List`, `Tuple`, `Array`, `Set`, `Map`

Adds collection kinds beyond Option A.

* **Tuple:** as Option A.
* **Set:** needs a spelling that does not collide with `{}` (block/map/struct/
  match/impl). Candidates:
  * `Set<…>` / `set{…}` — readable, but `{…}` after a head is a block in
    expression position today; requires a lookahead rule.
  * `#{…}` — unambiguous, but a new sigil.
  * **`{a, b}`** (the human's prior conceptual target): **ambiguous today** —
    `map_ahead()` returns false with no depth-0 `:`, so `{a, b}` parses as a
    block containing expression `a`, then a newline-or-`}` is expected and `,`
  is `E1006`. Admitting it requires `map_ahead()` to also accept a comma-set
  form and the block grammar to co-exist; disambiguation would rest on "no
  depth-0 `:` and at least one depth-0 `,` before `}`", which is decidable but
  must be specified and negatively tested against blocks, maps, struct
  construction, and match bodies.
* **Array:** the human target is conceptually `[N; T]`, and **semicolon is now
  reserved** for exactly this (prompt §6/§14). But repository evidence does not
  define `N` or `T`:
  * If `[N; T]` is a **type** (fixed-length homogeneous array of `N` elements
    of type `T`), then `N` is an integer literal, `T` a type, and a matching
    literal needs its own form.
  * If `[T; N]`-style (element type then size) it reads differently.
  * If `[v; N]` is a **repeat literal** (value `v` repeated `N` times), it is an
    expression, not a type.
  These are **three incompatible designs**; picking one silently would violate
  prompt §14. This is the single unresolved architectural question.

*Cost:* large. Full five-identity migration across syntax → AST → `TypeExpr`
→ `Ty` → family → checker → `Value` → equality → iteration → indexing →
mutation → patterns → display → JSON → Native/WASM → AIS → tests → docs
(prompt §11). Must be one atomic series, not a stray AST node.

### Option C — Declare the collection surface closed for 0.3

Keep `List`/`Map` (and `Tuple` as list sugar). Record that `Tuple`/`Array`/`Set`
distinctness is deferred to a later version.

*Cost:* zero; honest. But it does **not** satisfy the human's stated target.

---

## 2. Capability matrix (prompt §17) — required answers before any implementation

Every cell must be *decided*, never decided by implementation accident.

| Question | List | Map | Tuple (if real) | Set (if real) | Array (if real) | Range | String |
|---|---|---|---|---|---|---|---|
| Iterable? | yes | yes (keys) | yes | yes | yes | yes | yes (chars) |
| Indexable? | yes (int) | yes (key) | yes (int) | no | yes (int) | **no** | yes (int) |
| Sized (`len`)? | yes | yes | len = arity | yes | yes | yes | yes |
| Mutable? | shared | shared | **no (immutable)** | shared? | shared? | value | value |
| Fixed-length? | no | no | **yes** | no | **yes** | yes | no |
| Heterogeneous? | yes | keys homogeneous-capable, values yes | **yes** | elements must be hashable | **no (homogeneous)** | no | n/a |
| Key-capable? | no | n/a | no | no | no | no | no |
| JSON-representable? | array | object | **array (lossy? see §4)** | ? | array | rejected | string |
| Pattern-matchable? | yes (exact length) | no | ? | ? | ? | no | no |
| Orderable? (`<`) | no | no | no | no | no | no | yes |
| Equality model | structural length+elem | structural key-set | structural arity+elem | **membership (unordered)** | structural arity+elem | start/end | value |

**Must-decide questions (currently unanswered by any authority):**
1. Can a Tuple mutate? *Proposed:* no (fixed arity, immutable element slots).
2. Can an Array change length? *Proposed:* no (fixed length).
3. Can Set elements be mutable? *Proposed:* no — set membership requires a
   stable hash; mutable elements would corrupt the set.
4. What values can be Set members? *Proposed:* key-capable scalars only
   (`int`/`bool`/`string`), shared with `MapKey`.
5. Is Set iteration deterministic? *Proposed:* yes — a `BTreeSet`-like order
   (Aura's `Map` is already `BTreeMap`, so deterministic ascending order is the
   house style).
6. Does Tuple allow heterogeneous element types? *Proposed:* yes.
7. Does Array have homogeneous elements? *Proposed:* yes (that is its point
   versus Tuple).

---

## 3. Candidate syntax (only the parts with evidence)

| Kind | Spelling | Evidence / conflict |
|---|---|---|
| List | `[a, b]` | existing, canonical |
| Map | `{k: v}` | existing, canonical |
| Tuple | `(a, b)` | existing spelling; `map_ahead`-safe; only the *identity* changes |
| Set | `{a, b}` | **ambiguous** with block until `map_ahead` gains a comma-set branch; needs negative tests vs block/map/struct/match |
| Set | `#{a, b}` | unambiguous; new sigil |
| Array | `[N; T]` | `;` reserved; **meaning of `N`/`T` undefined by evidence** |

**Array is blocked pending a human answer to:** is the target
1. a **type** `[T; N]` (fixed-size array of `T`),
2. a **type** `[N; T]` (size first), or
3. a **repeat literal** `[v; N]`?

---

## 4. Deltas each option must touch (prompt §11 atomicity list)

For any collection kind admitted, *all* of these must change together:

```
source syntax -> AST -> TypeExpr (where representable) -> Ty -> TypeFamily
 -> checker relations -> runtime Value -> equality -> iteration -> indexing
 -> mutation -> patterns -> display/repr -> JSON -> Native/WASM -> AIS
 -> tests -> documentation
```

A partial change is forbidden. The current `Tuple` demonstrates the failure
mode: `Expr::Tuple` exists in the AST but is identity-free from the checker
onward, so it reports family `sequence`/value kind `list` (an identity it does
not have as a distinct kind).

### JSON anti-collapse rule (prompt §23/§24) applied to new kinds

`json_encode` must encode only values with an **exact** JSON representation and
reject the rest (the existing E3001 rule for variants/ranges/functions/
non-finite floats). For new kinds:

* **Tuple → JSON array**: *reversible?* No — decoding a JSON array yields a
  `List`, not a `Tuple`. So Tuple→array is an **accepted serialization**, not a
  reversible identity. It must be documented as such, or Tuple encoding should
  be rejected. *Decision required.*
* **Set → JSON array**: lossy for the same reason plus order; membership
  identity is not preserved. If admitted, define order and mark non-reversible.
* **Array → JSON array**: reversible only if decode target is `Array`.

### AIS three-level identity (prompt §50)

A real Tuple/Set/Array must project distinctly in AIS:
`type_name`, `families`, `value_kind` — e.g. `Tuple<int, string>` /
`["sequence"]` / `"tuple"`. AI clients must never infer family by parsing the
human type string.

---

## 5. Recommendation

1. **Decide the Array question first** (type vs repeat literal; `N`/`T`
   meaning). Nothing else can be finalized until then.
2. If the human wants a *bounded, coherent* 0.3, take **Option A** (real
   `Tuple`, `List`, `Map`; `Array`/`Set` deferred with a recorded decision).
   It removes the one documented falsehood ("a tuple is a list") with a
   well-bounded atomic migration and no delimiter ambiguity.
3. Take **Option B** only with explicit human confirmation of: the Set
   spelling, the Array meaning, and the capability-matrix answers in §2.

## 6. What is NOT proposed here

* No syntax is invented. No implementation is started.
* No change to `List`/`Map` semantics.
* No release, tag, or runtime publication.

**Decision owner:** human. **Blocking question:** §3 Array meaning and §2
capability answers.
