# RFC 0005 — Inference and Error-Propagation Rules

- **Status:** Proposed — reconstructed for the NEBULA 0.3.2 campaign (Directive
  008, gate G02). **Not accepted.** It records a two-part checker repair that
  gate G02 forbids implementing before authorization (§2.3). No known residual
  of this defect class survives the repair (§2.4 records the disproof of an
  earlier draft's claim that one did).
- **Scope:** how a generic call's substitution is turned into a call-site
  result type, for both free functions and methods, and what that means for
  error propagation into annotated bindings, return positions, argument
  positions, and downstream operators. **Grammar delta: none** (§3).
- **Authorities:** `docs/LANGUAGE_SPEC.md` §36.3 Inference and substitution
  (`:4048-4059`), §36.5 Structures, methods, and traits (`:4067-4074`);
  `docs/GENERICS.md` §4 Inference and substitution (`:168-193`), §7 Generic
  methods (`:231-257`), §12 Errors (`:330-343`); `src/check/mod.rs`
  (`instantiate_return_ty` `:4497`, `instantiate_method_return_ty` absent,
  `check_struct_method_args` `:4882`); `src/types.rs` (`Ty::erase_params`
  `:244`, `Ty::compatible_with` `:342`).
- **Human decisions:** one. **D1** — authorize the two-part checker repair
  (§2.1, §2.2). It is not resolved here. Provenance is stated in §0.

## 0. Provenance

`NBL-012` in `docs/engineering/NEBULA_CAMPAIGN.md` requires an RFC covering
"inference and error propagation rules". No pre-existing Nebula artifact (RFC
draft, inventory, contract) existed in the tree when G02 began; §0.4 of the
campaign document records that absence. This RFC is therefore a
**reconstruction from repository evidence**, not a transcription of an approved
design. Every claim below is backed by a probe recorded in §6; nothing is
asserted from the brief alone.

The defect was found by probing, not by reading. §1 records what the code does,
§2 records the repair that was engineered, verified in both directions, proven
output-neutral on the whole corpus, and then **reverted** because gate G02
forbids coding a surface an RFC describes before the RFC is accepted.

Five corrections were applied to earlier working notes during this research and
are recorded here rather than silently dropped. Each was a published claim that a
later probe falsified:

1. An early draft described the defect as "generic calls return `Unknown`". That
   is too broad: **explicit type arguments already work correctly** (§1.4). The
   defect is confined to *inferred* substitution, and to `Ty::Param` occurrences
   in the *return* type.
2. An early probe batch reported that the one-line fix also repaired generic
   method calls. It does not; a **second, independent defect** exists in method
   dispatch (§1.5). The one-line fix was measured against a binary that was
   already patched (a comparison harness error), which masked the second
   defect. The corrected comparison is in §6.1.
3. An early draft of **this RFC** claimed the method receiver-binding repair left
   a residual defect, and escalated a second decision (`D2`) for it. That claim
   was false; the repair closes the shape. §2.4 records the disproof.
4. An early draft listed the argument-position probe as
   `take(id(1))` with `fn take(s: string) -> int { return 1 }`. That program
   fails `E2008` "parameter `s` is never used" **before** any type check, on both
   binaries, so it returned `1` on both sides and proved nothing. §6.1 carries
   the corrected program.
5. An early draft listed the compatible-annotation method probe `m_own_ok` as
   `1 | 1` (rejected both before and after). It is `0 | 0` — accepted both times.
   Publishing it as `1 | 1` would have described a working program as broken.

Correction 3 is the reason §2.4 exists in its present form. The shape it names is
retained as a **negative control** rather than deleted, because the claim it
disproves is exactly the kind a reviewer would otherwise re-derive.

## 1. Motivation

### 1.1 The normative promise

Two documents state the same rule.

`docs/LANGUAGE_SPEC.md` §36.3 (`:4048-4058`):

> **Normative rule.** A call builds a substitution by structurally matching
> each argument's inferred type against the parameter's declared type. `T` binds
> to the actual type; `[T]` matches `[A]`; `{string: T}` matches `{string: A}`;
> `Name<T>` matches `Name<A>` by element. A concrete declared type requires
> compatibility. `Unknown` never forces a binding and never becomes a concrete
> type: an unbound parameter in a value position is `Unknown`.

`docs/GENERICS.md` §4 (`:183-185`):

> The inferred return type is the declared return type with `σ` applied. An
> unbound parameter in the return type is `Unknown` (never an arbitrary
> concrete type).

Both sentences are about what happens when a parameter is **unbound**. When a
parameter **is** bound by the arguments, the return type must be the declared
return type with the *bound* `σ` applied. The implementation computes `σ` and
then throws it away.

### 1.2 The defect: the substitution is computed and discarded

`src/check/mod.rs:4497-4532`:

```rust
    fn instantiate_return_ty(&self, sig: &FnSig, args: &[Arg], ty_args: &[TypeExpr]) -> Ty {
        let Some(ret) = &sig.ret else {
            return Ty::Unknown;
        };
        if !ret.has_param() {
            return ret.clone();
        }
        let mut sigma: HashMap<String, Ty> = HashMap::new();
        if !ty_args.is_empty() {
            for ((p, _), t) in sig.type_params.iter().zip(ty_args) {
                if let Ok(t) = self.annotation(t, Span::default()) {
                    sigma.insert(p.clone(), t);
                }
            }
        }
        let actual: Vec<Ty> = args.iter().map(|a| self.infer(&a.value)).collect();
        let n_params = sig.params.len();
        let offset = n_params.saturating_sub(actual.len());
        let mut pseudo: Vec<ParamSig> = Vec::new();
        for (i, p) in sig.params.iter().enumerate().skip(offset) {
            if i - offset < actual.len() {
                pseudo.push(p.clone());
            }
        }
        let _ = self.infer_substitution(&pseudo, &actual, &sigma);
        ret.erase_params(&sigma)
    }
```

The call on the third-to-last line is the defect. `infer_substitution`
(`:3172-3189`) returns `Option<HashMap<String, Ty>>` — the substitution
extended with every binding the arguments imply. Binding it to `_` **discards
it**. The next line then calls `ret.erase_params(&sigma)` with the *original*,
still-unbound `sigma`.

`Ty::erase_params` (`src/types.rs:244-261`) maps each `Ty::Param(n)` through
`sigma.get(n).cloned().unwrap_or(Ty::Unknown)` (`:246`). With an empty `sigma`
every return parameter becomes `Ty::Unknown`. And `Ty::compatible_with`
(`src/types.rs:342-419`) treats `Unknown` as compatible with everything. So the
result type of every *inferred* generic call silently satisfies every
annotation.

This is not a deliberate design. The surrounding code says the opposite: the
line above it derives `pseudo`/`actual` pairs specifically so the substitution
*can* be computed ("so a return-only parameter is bound by the arguments"), and
`infer_substitution` is captured correctly at the other two of its four call
sites (`:3473`, `:4874`).

### 1.3 What that costs

A program the checker accepts can fail at run time, or — worse — run to
completion with the wrong type.

```aura
fn id<T>(x: T) -> T { return x }

fn main() {
    let a = id(1)
    let b: string = a
    print(b)
}
```

`aura check` exits **0**. `aura run` prints `1` and exits 0: the binding is
declared `string` and holds an `int`. Add a use of the declared type and the
runtime aborts instead:

```aura
fn main() {
    let a = id(1)
    print(a.len())
}
```

`aura check` exits **0**; `aura run` exits **1** with
`error[E2003]: int has no method `len``. The checker accepted what the runtime
rejects.

### 1.4 What already works — and must keep working

The defect is narrower than it first appears, and the fix must not disturb the
parts that are correct:

| Shape | Baseline behaviour |
|---|---|
| `id<int>(1)` assigned to `string` | **Rejected** — explicit type args seed `sigma` in the `ty_args` loop *before* the discarded call, so `erase_params` binds correctly |
| Generic struct construction `let b: Box<string> = Box { value: 1 }` | **Rejected** |
| Generic field access `let s: string = b.value` on `Box { value: 1 }` | **Rejected** |
| Concretely-typed generic return `fn k<T>(_x: T) -> int` | **Correct** — hits the `!ret.has_param()` early return |
| Argument-position unification `fn use_it<T>(xs: [T])`, `use_it([1, 2])` | **Correct** |

The last row is worth naming precisely: argument positions *do* enforce. A
non-generic `take(s: string)` rejects `take(id(1))` at baseline only because
`id` itself is non-generic there. Once `id` is generic, the argument type is
`Unknown` and the call is accepted — §1.3's defect propagating into a second
position.

### 1.5 The second, independent defect: method dispatch never binds the receiver

Fix §1.2 alone and a generic *method* call is still unsound.
`src/check/mod.rs:4101-4114`, `Expr::Method`:

```rust
            Expr::Method(recv, name, args, ty_args, _) => {
                let recv_ty = self.infer(recv);
                if let Some(sname) = self.struct_name_of(&recv_ty) {
                    if let Some(set) = self.method_set(&sname, name) {
                        let visible: Vec<FnSig> = set
                            .iter()
                            .filter(|s| self.method_visible(s))
                            .cloned()
                            .collect();
                        if let Some(i) = self.resolve_method_sig(&visible, args) {
                            return self.instantiate_return_ty(&visible[i], args, ty_args);
                        }
                    }
                    return Ty::Unknown;
```

`instantiate_return_ty` receives `args`, which for a method call **excludes the
receiver**. Inside it, `offset = n_params.saturating_sub(actual.len())` then
skips past `sig.params[0]` — the receiver — so an `impl<T> Box<T>` parameter can
only ever bind from `ty_args`, which is normally empty. The `impl` parameter is
always unbound.

```aura
struct Box<T> { value: T }
impl<T> Box<T> { fn get(self) -> T { return self.value } }

fn main() {
    let b = Box { value: 1 }
    let s: string = b.get()
    print(s)
}
```

Baseline: `check` **0**, `run` **0**, prints `1`. With the §1.2 repair applied:
still `check` **0**. The receiver-binding defect is orthogonal.

**This is an implementation gap, not a missing design.** The same file already
performs the correct binding for method *arguments*, in
`check_struct_method_args` (`:4882-4931`):

```rust
            // Bind the impl's parameters from the receiver type.
            let recv_args = Self::ty_args_of(recv);
            if let Some(recv_head) = recv.nominal_head() {
                if let Some(declared) = self.type_type_params.get(&recv_head) {
                    for (p, a) in declared.iter().zip(&recv_args) {
                        sigma.insert(p.clone(), a.clone());
                    }
                }
            }
```

and it already receives the receiver through a purpose-built `MethodSite`
struct (`:511-517`). The return-type path simply does not use either.

### 1.6 Blast radius

**Corpus.** 125 tracked `.aura` files; 36 contain a generic function, 56 generic
function declarations and 7 other generic declarations in total.
`tests/corpus/generics/` holds 8 fixtures; none exercises a return-only
parameter bound from arguments.

**Test suite.** `tests/generics.rs` is 690 lines with **46** `#[test]` functions.
**Nothing enshrines the defect.** The nearest-looking test,
`unbound_parameter_does_not_become_concrete` (`:79`), declares
`fn empty<T>() -> int` — a *concrete* return — and therefore exercises the
`!ret.has_param()` early return, not this path. The two method tests
(`generic_method_uses_impl_parameter` `:138`, `generic_method_may_add_its_own_parameter`
`:147`) assert only runtime output (`"3\n"`, `"z\n"`), never a check verdict.

**Downstream positions.** The `Unknown` result does not merely satisfy the
immediate annotation; it disables checking in every consumer:

| Position | Probe | Baseline | After repair |
|---|---|---|---|
| Annotated binding | `let b: string = a` where `a = id(1)` | accepted | **rejected** |
| Direct annotated binding | `let b: string = id(1)` | accepted | **rejected** |
| Return position | `fn g() -> string { return id(1) }` | accepted | **rejected** |
| Argument position | `take(id(1))` with `take(s: string)` | accepted | **rejected** |
| Ordered comparison | `if a < "s"` on `a = id(1)` | accepted | **rejected** |
| Method call | `a.push("s")` on `a = id(1)` | accepted | **rejected** |
| Subscript | `a["k"]` on `a = id(1)` | accepted | **still accepted** (§7.2) |
| Equality | `a == "s"` on `a = id(1)` | accepted | **still accepted** — unchecked for all types |

Two rows are exceptions, and neither is this RFC's defect. Subscript inference
ignores the operand type; `5["k"]` is accepted with no generic call in the
program at all. `==` is not type-checked for *any* operand; `let a = 1; let c:
bool = a == "s"` is accepted at baseline. §7.2 records both, so that this table is
not read as a promise the patch does not keep.

**Runtime reach.** All were confirmed to fail under `aura run` (`E2003`) or to
run silently wrong. `let ys: [string] = wrap(1); print(ys)` prints `[1]` and
exits 0 — a program whose declared types are false.

## 2. Decision

One decision is requested. It is **withheld** here; gate G02 forbids acting on it
before a human resolves it.

### 2.1 D1a — authorize the free-function repair

Replace the discarded binding with a captured one in `instantiate_return_ty`:

```rust
        let sigma = match self.infer_substitution(&pseudo, &actual, &sigma) {
            Some(bound) => bound,
            None => return ret.erase_params(&sigma),
        };
        ret.erase_params(&sigma)
```

Four inserted lines, one removed. The `None` arm is the existing behaviour: if
unification fails the call was already diagnosed elsewhere, and the return type
stays `Unknown` so no second error is produced.

This change is what the specification already promises. It is not a new rule.

### 2.2 D1b — authorize the method receiver-binding repair

Add a method-aware instantiation that binds `sig.params[0]` from the receiver
and the rest from `args`, mirroring `check_struct_method_args`:

```rust
    fn instantiate_method_return_ty(
        &self,
        sig: &FnSig,
        recv: &Ty,
        args: &[Arg],
        ty_args: &[TypeExpr],
    ) -> Ty {
        let Some(ret) = &sig.ret else {
            return Ty::Unknown;
        };
        if !ret.has_param() {
            return ret.clone();
        }
        let mut sigma: HashMap<String, Ty> = HashMap::new();
        if !ty_args.is_empty() {
            for ((p, _), t) in sig.type_params.iter().zip(ty_args) {
                if let Ok(t) = self.annotation(t, Span::default()) {
                    sigma.insert(p.clone(), t);
                }
            }
        }
        if let Some(recv_head) = recv.nominal_head() {
            if let Some(declared) = self.type_type_params.get(&recv_head) {
                for (p, a) in declared.iter().zip(&Self::ty_args_of(recv)) {
                    sigma.insert(p.clone(), a.clone());
                }
            }
        }
        let actual: Vec<Ty> = args.iter().map(|a| self.infer(&a.value)).collect();
        let rest_params: Vec<ParamSig> = sig.params[1..].to_vec();
        let sigma = match self.infer_substitution(&rest_params, &actual, &sigma) {
            Some(bound) => bound,
            None => return ret.erase_params(&sigma),
        };
        ret.erase_params(&sigma)
    }
```

and dispatch `Expr::Method` to it. This duplicates the binding logic that
`check_struct_method_args` already contains; a cleaner factoring would extract
the shared part. Whether to do that is a code-quality choice for the
implementer, not a semantic one.

### 2.3 Sequencing

D1a and D1b are independent and were verified independently (§6.2). They may
land together or in sequence. Neither may land before acceptance.

### 2.4 The `self` shape — a negative control, not a residual

An earlier draft of this RFC claimed that D1a + D1b still leave the `impl<T>`
method shape wrong and escalated it as a second human decision, **D2**. That
claim was false, and it is recorded here rather than deleted because the way it
failed is itself the evidence.

The draft's authority was a probe report reading `FIX=0` for

```aura
struct Box<T> { value: T }
impl<T> Box<T> { fn get(self) -> T { return self.value } }

fn main() {
    let b = Box { value: 1 }
    let s: string = b.get()
    print(s)
}
```

against the receiver-aware binary. Re-running the same program in a fresh
`mktemp -d` against the two *saved* binaries — baseline `966462ad…`, fixed
`fb230ce5…`, provably distinct (§6.1) — gives **`FIX=1`**:
``error[E3001]: `s` is annotated as `string` but its value is `int` ``. The
`FIX=0` reading came from an execution that never re-ran the patch's second
part, not from the program.

The shape is therefore a **negative control that D1b closes**, and it is
reported that way in §6.1 (`m_get` `0 → 1`, `m_get_ok` `0 → 0`, `m_own`
`0 → 1`, `m_own_ok` `0 → 0`). The compatible-annotation arms matter more than
the rejecting ones: a repair that rejected correct programs would be worse than
the defect, and these arms show it does not.

**No residual of this class is known.** §7.1 states what was swept to support
that, and §7.2–§7.4 record the shapes that are real but out of scope.

There is **no D2**.

## 3. Grammar delta

**None.** `docs/grammar.md` is unchanged. `type_params`, explicit type
arguments, `method_decl`, and `impl` blocks already parse. The defect is
entirely inside the checker; §36.1's `<`/`>` disambiguation rule is unaffected.

## 4. Semantics

### 4.1 What is already correct and must not change

* **Explicit type arguments.** `id<int>(1)` into `string` is rejected and must
  stay rejected.
* **The `!ret.has_param()` early return.** A generic whose declared return is
  concrete (`fn k<T>(_x: T) -> int`) is correct and must stay correct.
* **Struct construction and field access.** `Box { value: 1 }` into
  `Box<string>`, and reading `.value` as `string`, are both rejected correctly.
* **Argument-position unification.** `use_it([1, 2])` against `[T]` binds `T`.
* **The unbound-parameter rule.** A parameter that genuinely cannot be bound
  must still erase to `Unknown`, not to a guess. The `None` arm preserves this.

### 4.2 What the repair changes

* An **inferred** generic call's result type is the declared return type with
  the **bound** `σ` applied, per §36.3/§4.
* An **inferred** generic method call's result type additionally binds the
  enclosing `impl`'s parameters from the receiver type, per §36.5 ("a generic
  `impl<T> Box<T>` binds `T` for its method bodies") — which the runtime already
  honours.
* As a consequence, every position in §1.6 rejects **except** subscript and
  equality, which are separate pre-existing holes (§7.2) and not residuals of
  this repair.

### 4.3 Deliberately not changed

* **No exhaustiveness rule is introduced.** `LANGUAGE_SPEC.md:2471-2472` states
  there is no exhaustiveness requirement; `E4029` reports it at run time. That
  is RFC 0002's territory and is untouched here.
* **No new diagnostic code.** The repair reuses `E3001`/`E3005`, which the
  specification already assigns to type mismatch and return mismatch.
* **No monomorphization, no dictionaries.** §36.7 erasure is unchanged.
* **No change to overload resolution.** `resolve_method_sig` and
  `method_visible` are called exactly as before.

## 5. Migration

In principle this is a **soundness repair**, so it can reject programs that
previously compiled. In practice it is **observably a no-op on everything in the
tree**:

* The whole corpus of 125 tracked `.aura` files produces **byte-identical**
  output and identical exit status before and after (§6.3).
* The full suite is 1477 passed / 0 failed before and after (§6.3).
* No test in `tests/generics.rs` asserts a check verdict for an inferred
  generic call (§1.6).

A downstream user whose program relied on `Unknown` satisfying an annotation
would see a new `E3001`/`E3005`. That is the intended effect: such a program was
already unsound. There is no deprecation window, no opt-out, and no version bump
implied by this RFC.

## 6. Enforcement

### 6.1 Evidence — the discriminating probes

Every probe below was run in its own `mktemp -d` with exactly one `main.aura`,
comparing a **baseline** binary against a **fixed** binary by `shasum`. An
earlier comparison harness reused `target/debug/aura` for both sides and
therefore compared the fixed binary with itself; that entire reading was
discarded and redone. The two binaries are provably distinct:
baseline `966462ad66f9d3f4e60b071a629506264d52f12be8a126123127a1acfc08eb89`
(17 748 120 B), fixed
`fb230ce5c5dfb8546df06df604bfc88952b2d1e78a3a4dad091f572a26888aae`
(17 773 816 B).

| Probe | Shape | Baseline | Fixed | Verdict |
|---|---|---|---|---|
| `g_bad` | `let a = id(1)` then `let b: string = a` | 0 | **1** | repaired — `E3001` |
| `g_good` | `let b: string = id("hi")` | 0 | 0 | no regression |
| `g_unann` | `print(id(1))` | 0 | 0 | no regression |
| `g_explicit` | `let b: string = id<int>(1)` | 1 | 1 | negative control held |
| `g_control` | non-generic `id(x: int)` into `string` | 1 | 1 | negative control held |
| `g_pair` | `let ys: [string] = pair(1)` where `pair<T>(x: T) -> [T]` | 0 | **1** | repaired — `E3001` |
| `ret_pos` | `fn g() -> string { return id(1) }` | 0 | **1** | repaired — `E3005` |
| `arg_pos` | `take(id(1))` where `fn take(s: string) -> int { return s.len() }` | 0 | **1** | repaired — `E3001` |
| `struct_fld` | `let s: string = b.value` on `Box { value: 1 }` | 1 | 1 | unchanged |
| `gs_ann` | `let b: Box<string> = Box { value: 1 }` | 1 | 1 | unchanged |
| `builtin_len` | `print(id(1).len())` | 0 | **1** | repaired — `E2003` |
| `m_get` | `impl<T> Box<T> { fn get(self) -> T }`, `let s: string = b.get()` | 0 | **1** | repaired by D1b — `E3001` |
| `m_get_ok` | same, `let s: int = b.get()` | 0 | 0 | no regression |
| `m_mine` | `impl<A> Pair<A> { fn conv(self, x: A) -> A }`, `let s: string = p.conv(2)` | 0 | **1** | repaired by D1b |
| `m_own` | `fn replace<U>(self, v: U) -> U`, `let s: string = b.replace(1)` | 0 | **1** | repaired by D1b |
| `m_own_ok` | same, `let s: string = b.replace("z")` | 0 | 0 | no regression |
| `mmine_recv` | `impl<A> Pair<A> { fn get(self) -> A }`, `let s: string = p.get()` | 0 | **1** | repaired by D1b — receiver binding |
| `lt` | `let a = id(1)` then `if a < "s"` | 0 | **1** | repaired |
| `method` | `id(1).push(2)` | 0 | **1** | repaired — `E2003` |
| `idx_subscr` | `let a = id(1)` then `a["k"]` | 0 | 0 | **not repaired** — §7.2 |
| `ret_only_unbound` | `fn make<T>() -> T`, `let s: string = make()` | 0 | 0 | **not repaired** — §7.1 |

`lt` replaced an earlier `cmp` row that used `==`. That was a defect in the
probe, not in the repair: Aura does **not** type-check `==` even non-generically
(`let a = 1; let c: bool = a == "s"` is accepted, exit 0). `cmp` therefore
returned `0 | 0` and measured an unrelated pre-existing hole. `<` *is* checked —
non-generically `1 < "s"` is `E3001` at baseline — so the generic form is a valid
discriminator, and it is the row published here.

`m_mine` was also reshaped. As first written it declared `struct B<T>` whose
single field was `int`, so `T` was unconstrained by construction and never bound
from the receiver; the shape could not have exercised D1b. `Pair<A> { first: A }`
does, and `mmine_recv` isolates the receiver-binding path with no arguments at
all.

Four corrections to earlier versions of this table are worth naming, because all
four were published and all four were wrong:

* `arg_pos` was listed as `take(id(1))` with `take(s: string)` whose body never
  read `s`. That program fails **`E2008` "parameter `s` is never used"** at
  baseline, so it returned 1 on both sides and proved nothing. The body must use
  the parameter, as above; the corrected probe returns **0 → 1**.
* `m_own_ok` was listed as `1 | 1`. It is **`0 | 0`** — the compatible
  annotation was always accepted, and the repair leaves it accepted. Publishing
  it as `1 | 1` would have mislabelled a working program as a pre-existing
  rejection.
* `cmp` was listed as `id(1) == "s"` with `0 → 1`. Re-running it gave `0 → 0`,
  and the reason is that `==` is unchecked for **all** types, generic or not. The
  row now uses `<`.
* `m_mine` was listed with a `struct B<T>` whose field was `int`, leaving `T`
  unconstrained; it never exercised receiver binding and scored `0 → 0`. The row
  now uses `Pair<A> { first: A }`.

### 6.2 Mutation verification in **both** directions

Each part was removed and the probe re-run, so neither is redundant:

| Mutation | Effect | Result |
|---|---|---|
| D1a not captured (`let _ =`, original) | `g_bad` | **0** — defect present |
| D1a captured but not passed to `erase_params` | `g_bad` | **0** — defect present |
| D1a captured and passed | `g_bad` | **1** — repaired |
| D1b receiver binding removed | `m_get` | **0** — defect present |
| D1b receiver binding present | `m_get` | **1** — repaired |

A "guard only" variant (`if …is_none() { return … }`, keeping the unbound
`sigma`) was also tried and does not repair anything: capturing the binding is
load-bearing.

### 6.3 Whole-suite and whole-corpus gates

Run with the full repair applied:

* `cargo test --locked --all-targets --all-features` → **1477 passed, 0 failed**
  across 66 test binaries.
* `cargo clippy --locked --all-targets --all-features -- -D warnings` → clean.
* Corpus, 125 tracked `.aura` files, each checked in its own directory against
  both binaries: baseline **108 rc=0 / 17 rc=1**, fixed **108 rc=0 / 17 rc=1**,
  and the full stdout streams are **byte-identical**.

The corpus result is the load-bearing neutrality evidence: the repair changes
nothing that exists today, so the risk of landing it is the risk of the new
rejections it enables, all of which are §6.1's probes.

### 6.4 Regression coverage this RFC requires before the repair may land

Tests must be added to `tests/generics.rs` asserting the **check verdict**, not
only the runtime output:

1. An inferred generic call assigned to an incompatible annotation is rejected
   (`g_bad`, `g_direct`).
2. The same call assigned to a compatible annotation is accepted (`g_good`).
3. A return-only parameter is bound from the arguments (`g_bad` without the
   intermediate binding).
4. A `[T]`-returning generic assigned to `[string]` from an `int` argument is
   rejected (`g_pair`).
5. An inferred generic call in return position is rejected (`ret_pos`).
6. An inferred generic call in argument position is rejected (`arg_pos`). The
   callee must actually read its parameter, or `E2008` masks the probe.
7. A generic method whose return is the `impl` parameter is rejected against an
   incompatible annotation (`m_get`) and accepted against a compatible one
   (`m_get_ok`).
8. A generic method adding its own parameter is rejected against an
   incompatible annotation (`m_own`) and accepted against a compatible one
   (`m_own_ok`).
9. The two negative controls (`g_explicit`, `g_control`) still reject.
10. `struct_fld`, `gs_ann`, and the concretely-typed generic return still
    behave as they do today.
11. A return-position parameter that no argument binds (`fn make<T>() -> T`)
    stays **accepted** — §7.1 — so the repair is not over-tightened into
    rejecting §36.3-legal programs.

Do **not** add a test asserting that `a["k"]` on `a = id(1)` is rejected. The
repair does not make that reject (§7.2); a test asserting it would fail and
would be testing unrelated code.

### 6.5 What must not be enforced yet

* Do not add an assertion for a §2.4 residual: there is none, and §2.4 explains
  why an earlier draft said otherwise.
* Do not add an exhaustiveness or `Result` requirement here; that is RFC 0002.
* Do not change `instantiate_return_ty`'s `None` arm into a diagnostic. A failed
  unification already produces its own error; emitting a second would be a
  diagnostic duplication defect of the kind RFC 0004 exists to prevent.

## 7. Open items

**7.1 Whether any residual survives the repair — no, as far as probing reaches.**
Beyond §6.1's matrix, two further sweeps were run against the same two binaries,
each probe isolated in its own directory:

* **Edge shapes around the `self` binding.** A generic receiver whose own type
  argument is a struct (`Box<Box<int>>`), a chained method call
  (`b.get().get()`), a method returning a `float` argument's type, an
  `impl<A, B>` two-parameter receiver, and the bare-`self`-return case were all
  measured: every incompatible-annotation arm moves `0 → 1`, and every
  compatible-annotation arm stays `0 → 0`.
* **Legitimate programs.** Eight programs that a stricter checker would be at
  risk of rejecting — `[T]`-returning, `{string: T}`-returning, tuple-returning,
  explicit type argument, nested `[T]`-returning, method chain, and two
  `print(id(…))` calls — returned `0 → 0` with **byte-identical output** under
  `run`.

The one accepted shape the repair does not reject is a return-position parameter
that no argument can bind:

```aura
fn make<T>() -> T { throw "x" }
fn main() { let s: string = make()
  print(s) }
```

This is `check` 0 / `run` 1 both before and after, and the run-time failure is
`E4026` — the `throw`, not a type error. It is **not** a residual of this
defect: §36.3 states that an unbound parameter in a value position *is*
`Unknown`, and `Unknown` is compatible with every annotation by design. Both
"fixes" available here — inventing a type for `T` or rejecting the call — would
contradict §36.3 or reject a spec-legal program. §7.3 records the question that
remains genuinely open.

**7.2 Two further holes, both generic-independent and both out of scope.**
Neither is caused by, nor repaired by, this RFC's patch. Both are recorded so
that §1.6's exceptions are not read as residuals.

*Indexing is never type-checked.*

| Probe | Program | `check` | `run` |
|---|---|---|---|
| `idx_list_ok` | `let a = [1, 2]` then `a[0]` | 0 | 0 — prints `1` |
| `idx_nongen` | `let a = 5` then `a["k"]` | 0 | **1** — `E3001: cannot index int with string` |
| `idx_bool` | `let a = true` then `a[0]` | 0 | **1** — `cannot index bool with int` |
| `idx_float` | `let a = 2.5` then `a[0]` | 0 | **1** — `cannot index float with int` |
| `idx_subscr` | `let a = id(1)` then `a["k"]` | 0 | **1** — same as `idx_nongen` |

The non-generic rows are the point: this happens with no generic call anywhere in
the program, and `idx_subscr` is identical to its non-generic twin. `Expr::Index`
inference is unrelated code and needs its own finding.

*Equality is never type-checked, and unlike indexing it does not even fail at
run time.*

| Probe | Program | `check` | `run` | Result |
|---|---|---|---|---|
| `eq_mixed` | `print(1 == "s")` | 0 | 0 | prints `false` |
| `ne_mixed` | `print(1 != "s")` | 0 | 0 | prints `true` |
| `eq_bool` | `print(true == "s")` | 0 | 0 | prints `false` |
| `eq_ann` | `let c: bool = 1 == "s"` | 0 | 0 | prints `false` |
| `lt_mixed` | `print(1 < "s")` | **1** | **1** | `E3001` |

Ordered comparison *is* checked; `==`/`!=` are not, at any type. This is a
silent-wrong-answer hole, strictly worse than the indexing one, and it is why
`cmp` is not a valid discriminator for this RFC (§6.1). It may deserve priority
over D1 on its own merits, but it is a different subsystem and is not bundled
here.

**7.3 Whether `Unknown` should be an error rather than compatible with
everything.** `Ty::compatible_with` accepting `Unknown` is what makes this class
of defect silent rather than loud, and it is what §7.1's unbound-parameter shape
rests on. Changing it is far broader than this RFC and is not proposed. If it
were ever changed, §7.1's shape would begin to reject and the spec's §36.3
"unbound parameter is `Unknown`" sentence would need amending first.

**7.4 Whether to factor the shared binding.** `check_struct_method_args` and the
proposed `instantiate_method_return_ty` would contain the same receiver-binding
loop. Extracting it is a refactor with no semantic content; it is left to the
implementer.
