# Feature 003 — Static Field-Type Propagation
## Implementation Report

**Baseline:** `0a91a77` (`docs: define Feature 003 semantics`)
**Branch:** `rewrite/v3-rust`
**Status:** Implemented and conforming.

---

## Repository State

```
HEAD (pre-commit): 0a91a77
Branch:            rewrite/v3-rust
Implementation:    uncommitted (this change set)
Push:              NOT PERFORMED
```

---

## Implemented Rule

```
receiver : Ty::Named(S),  S has field f : T   =>   infer(receiver.f) = T
anything else                                 =>   infer(receiver.f) = Ty::Unknown
```

`T` is the field's **already alias-resolved** declared type. The rule is
strictly conservative: it adds no speculative inference, and the runtime
remains authoritative for field access.

---

## Implementation Scope

The change is centered on the checker and the minimal REPL declaration
plumbing needed for cross-submission behavior. No parser, AST, runtime,
stdlib, grammar, Python, or resource-limit change.

### `src/check/mod.rs`

* **`infer`'s `Expr::Field` arm.** Replaced the `Ty::Unknown` group entry with
  a dedicated arm:

  ```rust
  Expr::Field(recv, name, _) => {
      if let Ty::Named(sname) = self.infer(recv) {
          if let Some(fty) = self.struct_fields.get(&sname).and_then(|m| m.get(name)) {
              return fty.clone();
          }
      }
      Ty::Unknown
  }
  ```

  `struct_fields` already stores alias-resolved declared types, so no
  resolution work happens in the inference path. `infer` remains `&self`
  (pure) and never evaluates.

* **`GlobalDecl::Binding` gains `ty: Option<TypeExpr>`.** The variant now
  carries the binding's declared annotation so it can be restored across REPL
  submissions. `with_declarations` restores `Some(t)` into `value_types[0]` in
  its **second pass** (after struct/enum/alias names are registered), using
  the same `resolve_type_expr_lenient` / `Ty::from_expr_lenient` path used for
  struct fields and function parameters. `with_globals` supplies `ty: None`.
  Unannotated bindings stay `None` and therefore infer `Unknown` exactly as
  they do within a single submission.

The existing `Expr::Field` **validation** arm (the receiver-kind check that
reports `E2003` for unknown methods on known receiver kinds) is untouched.
Field validity and field-type inference remain separate concerns.

### `src/repl.rs`

* `Stmt::Let` persists `ty: ann.clone()` when recording a binding.
* `declarations_of` records `ty: None` for `Item::Const` (top-level `let` is
  an immutable module constant with no carried annotation).

---

## REPL Binding Annotation Persistence

**Why it was required.** `infer(Expr::Name)` determines a name's type solely
from `value_types`. Across submissions the REPL rebuilds the checker from
`decls: Vec<GlobalDecl>`, the only persisted state. Before this change,
`GlobalDecl::Binding` carried only `{ name, mutable }`, so an annotated
`let user: User = ...` lost its type; in a later submission `user` inferred
`Unknown` and the new field rule had no `Ty::Named` receiver to consult.

**Why it does not duplicate or conflict.** `value_types` is the one canonical
state `infer` reads for binding names. `struct_fields` is keyed by type name
and `functions` holds signatures — neither overlaps binding names. Restoring
the annotation mirrors the existing, frozen mechanism used to persist
`FnSig.params`. Only the **declared annotation** is carried, never an inferred
type, so no inference is duplicated outside the checker. Unannotated bindings
remain `Unknown` in the session, consistent with the conservative boundary.

---

## Type Semantics

* **Known struct receiver + known field** → the field's declared,
  alias-resolved type.
* **Aliases** → resolved by the existing transparent-alias machinery
  (`type Id = int`; `id: Id` stores `Ty::Int`).
* **Nested reads** (`u.address.zip`) → repeated application of the same rule.
* **Missing field** → no fabricated type; the read stays `Unknown` and the
  existing missing-field diagnostic behavior is unchanged. Assigning a missing
  field remains `E2003`.
* **Non-struct / `Unknown` receivers** → `Unknown`; no rejection.

---

## Runtime Contract

Unchanged. The runtime performs field lookup exactly as before; the checker
gains only more precise static typing. `infer` is pure and never evaluates, so
evaluation order is untouched. No runtime field-type annotation was added, and
no storage representation changed.

---

## REPL Behavior

With the `GlobalDecl::Binding.ty` plumbing, a declared struct binding's type
persists across submissions. A field read in a later submission participates
in existing checks:

```
struct User { age: int }
let user: User = User { age: 3 }
fn takes_str(x: string) { return x }
takes_str(user.age)        # E3001: parameter `x` expects `string`, found `int`
```

A failed submission does not corrupt the session; struct, binding, and function
metadata remain usable. No new REPL state model was introduced.

---

## Pipeline Behavior

Unchanged. A field read used in a pipeline inherits its improved type through
ordinary expression/call semantics; no Feature-003-specific pipeline logic
exists.

---

## Resource Limits

No new resource semantics. `MAX_AST_DEPTH` (256, `E1015`), the parser
recursion backstop (2048 frames), `MAX_CALL_FRAMES` (512, `E4011`), and the
10,000,000-element range cap (`E4013`) are unchanged. Field inference performs
at most three hash lookups per read and cannot recurse without bound beyond the
already-enforced AST depth.

---

## Diagnostics

* **No new error codes.**
* Existing `E2003` (unknown field on assignment / unknown method) unchanged.
* `E2005` (missing initializer) unchanged.
* Mismatches discovered through the improved inference report through the
  existing Feature 001/002 argument path (`E3001`), return checking (`E3005`),
  and construction (`E3001`).

---

## Specification Changes

`docs/LANGUAGE_SPEC.md` amended in exactly three places, per the frozen design:

* **§17.5** — replaced the "does not track the static type of a field read"
  sentence with the normative field-read propagation rule and conservative
  fallback.
* **§34.2** — narrowed the "Field reads infer `Unknown`" bullet to state the
  rule applies only when the receiver's type is a known struct.
* **§33** — added frozen decision 19 (struct field reads propagate declared
  types; never evaluates; never changes evaluation order).

`docs/contract.md` gained one bullet under "Beyond annotations" describing the
field-read typing rule and the separate field-validity concern. It does not
duplicate or contradict the specification. `docs/grammar.md` and `README.md`
are unchanged (no syntax change).

---

## Tests Added

`tests/regressions.rs` (8 tests):

* `field_read_infers_declared_type_for_primitives`
* `field_read_resolves_transparent_aliases`
* `field_read_propagates_through_nested_structs`
* `field_read_strengthens_function_argument_checking`
* `field_read_strengthens_named_argument_checking`
* `field_read_flows_into_return_construction_and_ordering`
* `field_read_is_conservative_for_unproven_receivers`
* `field_read_missing_field_keeps_existing_behavior`
* `field_read_hoisting_and_recursion`

`tests/property.rs` (2 properties):

* `field_read_type_agrees_with_declaration` — accepted iff the declared field
  type equals the binding annotation (no `int`/`float` coercion); runs when
  accepted.
* `unproven_field_receiver_stays_permissive` — an unproven receiver's field
  read stays `Unknown`.

`tests/repl.rs` (2 tests):

* `field_read_type_persists_across_submissions`
* `field_read_failed_binding_is_isolated`

---

## Compatibility Impact

Feature 003 is a **semantic tightening with source-compatibility impact**, the
same class as Feature 001.

* Every previously **valid** program keeps running.
* Programs previously accepted only because a field read was `Unknown` — e.g.
  `let y: string = p.x` where `p.x: int` — are now rejected at check time with
  the existing `E3001` (or `E3005` for returns). These are intended
  consequences of the frozen design and are not weakened.
* Unannotated or otherwise unproven receivers, indexed reads, and map lookups
  are unaffected.

---

## Known Limitations

* REPL binding-type persistence is annotation-driven. An unannotated
  `let user = User { ... }` still infers `Unknown` in later submissions and
  does not participate in field-type checking across submissions. This matches
  the deliberate decision to avoid duplicating inference and preserves the
  conservative `Unknown` boundary.
* Indexed-read propagation, map-value inference, branch-join inference,
  static `none`, and user-method return inference remain out of scope.

---

## Unchanged Runtime / Grammar / Resource Semantics

* Runtime: unchanged.
* Grammar: unchanged.
* AST: unchanged.
* Stdlib: unchanged.
* Python bridge: unchanged.
* Resource limits: unchanged.
* Error codes: no new codes; no changed meanings.
