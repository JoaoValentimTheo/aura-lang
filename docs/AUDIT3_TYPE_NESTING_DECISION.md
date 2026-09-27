# DECISION REQUIRING HUMAN APPROVAL — Type-annotation nesting limit

**Status:** OPEN — do not implement until approved.
**Finding:** AUDIT-3 (pre-modules audit, `docs/PRE_MODULES_AUDIT.md` §1).
**Nature:** a change to the *frozen* language line (the 256-node semantic AST
limit, `LANGUAGE_SPEC.md` §31.1/§31.2). It is therefore a spec decision, not a
routine fix.

---

## 1. The problem

`enforce_depth` / `check_expr_depth` (`src/parse/mod.rs`) walk **expressions
and statements** but never descend `TypeExpr` nodes. Type nesting is therefore
bounded only by the substrate-calibrated parser recursion backstop
(`parse_recursion_budget`), which differs by substrate. The consequence is a
native/WASM **acceptance** divergence for deeply nested type annotations — the
one asymmetry inside a system otherwise designed to guarantee "valid under the
semantic limit ⇒ accepted on every substrate".

## 2. Reconfirmed numbers (fresh reproduction, boundary-specific)

Nested `Box<Box<…<int>…>>` in a function parameter annotation, tested one level
at a time:

| Substrate | last accepted | first rejected | diagnostic | host failure |
|---|---|---|---|---|
| native (CLI, release) | **2047** | **2048** | `E1015` | none |
| WebAssembly (dev artifact) | **767** | **768** | `E1015` | none |

N-1/N/N+1 confirmed independently on each substrate. Both ceilings are well
above the semantic limit of 256, and neither ever traps or aborts.

## 3. Real-usage findings (does tightening break anything real?)

A scanner measured the deepest nested **type application** (`Ident<…<…>`) in
every legitimate Aura source in the repo — `examples/*.aura`, the website
examples, guide and doc code fences, and all test source-string literals:

* The deepest nesting in any **Aura program** (the four files in `examples/`)
  is **0** (they use no generic types at all).
* The deepest anywhere is **3**, and only in a Rust type table inside prose
  (`docs/SEMANTIC_FREEZE_AUDIT.md`), not in executable Aura.

So no real Aura program in this repository nests a type more than one level,
and none comes close to 256. Tightening the type-nesting bound to 256 would
break **no program in the codebase**; the only affected inputs are adversarial
or synthetic.

---

## 4. OPTION A — count type nodes toward the existing 256-node limit

Extend `enforce_depth` (and the parse-time node accounting) to descend
`TypeExpr` nodes so they count toward the same 256-node semantic budget as
every other node kind.

**Pro**
* Restores the "valid under 256 ⇒ valid on every substrate" invariant that the
  entire architecture is built around; the native/WASM divergence disappears
  permanently rather than being documented away.
* Consistent with how every other node kind is already counted; no special
  case remains.
* The affected inputs are purely adversarial (deepest real usage is 0–1), so
  the practical breakage surface is empty in this repo.

**Con**
* A *breaking change* for any hypothetical program with type nesting between
  257 and 2047 levels that currently compiles natively. Based on §3, that
  breakage is not realistic: no such program exists in the repo, and writing
  one by hand requires hundreds of nested `Box<…>` wrappers with no purpose.
* Requires touching the parser's depth accounting for type positions, which is
  a semantic-line change and demands the full review/approval path plus
  regression coverage at N-1/N/N+1 on both substrates.

---

## 5. OPTION B — document the substrate-dependent bound, change no code

Leave `enforce_depth` as-is. Formally document, in `LANGUAGE_SPEC.md` and
`website/content/reference-limits.md`, that **type-annotation nesting is bounded
by the substrate-calibrated parser backstop, not by the semantic 256-node
limit**, and state the WASM ceiling (768) as the portable safe bound for anyone
who needs substrate-independent behavior.

**Pro**
* Zero risk of breaking any existing program; no semantic change at all.
* Honest: it makes the actual, already-shipped behavior the documented
  contract rather than pretending the limit is uniform.
* The divergence is *safe* either way: over-limit input is a structured
  `E1015` on both substrates, never a trap or abort (verified).

**Con**
* Permanently leaves one documented case where "accepted on native, rejected on
  WASM" is true — the single asymmetry inside a system otherwise designed
  specifically to prevent exactly that.
* Every future reader of §31.2 must learn the type-position exception.

---

## 6. Recommendation

**Option A** is recommended, for consistency and to eliminate the last
substrate asymmetry, gated on the §3 evidence that it breaks no real program.
If the project prefers to keep the frozen line untouched until the filesystem
module system lands, **Option B** is the safe alternative and is fully
reversible later. Either way the permanent differential sweep (§7.5 of the
audit) already locks the current curve, so a later change is immediately
verifiable.

**This is a recommendation, not a decision. Do not implement until approved.**

---

## 7. If approved (implementation checklist for the chosen option)

1. Cross `TypeExpr` nodes into the semantic depth accounting (Option A) — or
   the spec/`reference-limits` prose (Option B).
2. Regression tests at N-1/N/N+1 on **both** substrates.
3. Update the permanent TypeExpr sweep expectations in
   `playground/tests/node/differential.test.mjs` (Option A would collapse the
   two ceilings to one).
4. Update `LANGUAGE_SPEC.md` §31.1/§31.2 (and `reference-limits.md`).
5. Advance the development runtime (`0.0.2-dev.22` → next) if the parser
   changes; regenerate the artifact and manifest. **Never touch `0.0.2`.**
6. Full validation (suite, clippy, fmt, wasm imports = 0, differential, CI).
