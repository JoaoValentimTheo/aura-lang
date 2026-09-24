# Feature 003 — Static Field-Type Propagation
## Independent Code Review

**Reviewed against:** frozen design `docs/FEATURE_003_DESIGN.md`, semantic
authority `docs/LANGUAGE_SPEC.md`, and the implementation diff on
`rewrite/v3-rust`.
**Verdict:** No speculative inference. Design boundary preserved.

---

## 1. Known-Struct Receiver Detection

`infer`'s `Expr::Field` arm tests `if let Ty::Named(sname) = self.infer(recv)`.
`Ty::Named` is produced for struct literals (`Expr::Construct`) and for
annotated/inferred struct bindings recorded in `value_types`. It is **not** a
blanket "any named thing" match: the field table lookup that follows is what
gates propagation.

**Verified:** `struct_fields` is populated **only** from `Item::Struct` (module
hoist and `with_declarations`). Enums populate `variant_payloads`; aliases
populate `alias_targets`; `range` is never a key. Therefore a non-struct
`Ty::Named` yields `None` → `Unknown` by construction. No special-casing is
required, and none was added.

## 2. Alias-Resolved Field Types

Struct field annotations are stored through
`annotation` → `resolve_type_expr` → `Ty::from_expr`, and in
`with_declarations` through the lenient equivalents. The inference arm reads the
stored `Ty` directly, so `type Id = int` with `id: Id` propagates `Ty::Int`.

**Verified by:** `field_read_resolves_transparent_aliases`; the receiver-side
alias path is covered by the same `value_types` restoration.

## 3. Nested Field Propagation

`u.address` infers `Ty::Named("Address")`; `(u.address).zip` then re-enters the
same arm and infers `int`. There is no separate nested-field algorithm.

**Verified by:** `field_read_propagates_through_nested_structs` and the manual
probe for `p.q.s` (both accept and mismatch directions).

## 4. Conservative Non-Struct / `Unknown` Boundary

The fallback path always returns `Ty::Unknown`, which every downstream check
treats as permissive. Reviewed cases: unannotated parameter, enum value, range,
list index (`[P{..}][0].x`), map lookup (`m["k"].x`), branch result
(`if … else …`), and a call whose return type is not declared. All remain
`Unknown`; none acquire a concrete type.

**Verified by:** `field_read_is_conservative_for_unproven_receivers`; the
property `unproven_field_receiver_stays_permissive`.

## 5. Missing-Field Diagnostics

Field validity and field-type inference are separate. The inference arm does not
fabricate a type for a missing field; it returns `Unknown`. The existing
validation arm (receiver-kind checking) and assignment path are untouched:
assigning a missing field remains `E2003`, and reading a missing field on a
known struct remains a runtime decision.

**Verified by:** `field_read_missing_field_keeps_existing_behavior`
(read is runtime `E2003`; write is check-time `E2003`).

## 6. Feature 001 Interaction

Argument checking consumes `infer(arg)` and `compatible_with`. A field read now
contributes its declared type, so a mismatch against an annotated parameter is
rejected through the existing path; a matching one passes. No special-case code
was added to user-call checking.

**Verified by:** `field_read_strengthens_function_argument_checking`; manual
probe `f(p.x)` vs `fn f(a: string)` → `E3001`.

## 7. Feature 002 Interaction

Named arguments are bound and then checked by the same Feature 001 machinery,
using `infer(arg.value)`. Field reads flow through unchanged; `bind_arguments`
and the named-argument binder were not modified.

**Verified by:** `field_read_strengthens_named_argument_checking`; manual probe
`f(b: p.x, a: 1)` → `E3001` on the wrong parameter.

## 8. REPL Cross-Submission Behavior

Reviewed the `GlobalDecl::Binding.ty` plumbing:

* `Stmt::Let` persists `ann.clone()`; `Item::Const` persists `None`.
* `with_declarations` restores `Some(t)` into `value_types[0]` in the second
  pass, after type names are registered, using the lenient resolver.
* Unannotated bindings remain `None` → `Unknown`; no inferred type is stored,
  so no inference is duplicated outside the checker.
* A failed submission returns before its declaration is pushed, so the session
  is not corrupted.

**Verified by:** `field_read_type_persists_across_submissions` and
`field_read_failed_binding_is_isolated`; manual REPL probes for the accept,
reject, and failure-isolation scenarios.

**Observation (limitation, not a defect):** persistence is annotation-driven;
an unannotated `let user = User { … }` does not carry a type across
submissions. This is consistent with the frozen decision to avoid speculative
inference.

## 9. No Evaluation-Order Changes

`fn infer(&self, e: &Expr) -> Ty` remains immutable-borrowed and performs only
hash lookups and clones. It never executes user code. The field arm neither
calls the interpreter nor mutates checker state. Evaluation order in the
runtime is untouched.

## 10. No Runtime Changes

The diff touches no runtime file. Field lookup semantics, storage, and error
behavior are unchanged; the checker is the only consumer of the new inference.

## 11. No Resource-Limit Changes

No constant (`MAX_AST_DEPTH`, `MAX_PARSE_DEPTH`, `MAX_CALL_FRAMES`, range cap)
was modified. Field inference adds a bounded number of hash lookups and cannot
recurse independently of the AST depth already enforced at parse time.

## 12. No False-Positive Static Inference

Adversarially reviewed the boundary: only `Ty::Named` receivers whose name
exists in `struct_fields` with the read field propagate a type. Enums, ranges,
aliases-as-values, primitives, collections, dynamic results, and branch joins
all fall through to `Unknown`, which is never rejected. No proof-by-structure
assumption was introduced; no structural/duck typing.

---

## Regression Risk

* Existing struct/enum tests (`tests/run.rs::structs_and_enums`) continue to
  pass; field arithmetic stays `int`.
* No existing test asserted a field read is `Unknown`, so no test was weakened.
* The full suite passes in both feature configurations.

## Conclusion

The implementation matches the frozen rule exactly, remains checker-only (plus
the necessary REPL annotation plumbing), introduces no new diagnostics or
resource semantics, preserves evaluation order and runtime behavior, and does
not over-infer. Approved for conformance.
