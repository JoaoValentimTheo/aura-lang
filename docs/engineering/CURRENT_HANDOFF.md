# Current Engineering Handoff

## Directive 008 — NEBULA 0.3.2 campaign (`G02`, authorised)

Active task: **G02**, the P0 RFC reconstruction. `G00` and `G01` are closed.
No implementation is authorized before `G02` accepts the P0 RFC set.

```
WAVE: G02 -> IN_PROGRESS (2 of 12 P0 RFCs authored, three decisions outstanding)
STATUS: G02 IN_PROGRESS (three human decisions outstanding, see DECISÕES)
HEAD: 73a35d9b7ef988a1d0ddbaa0f34df0b74f7f12bd (merge of PR #48)
PRs: #43 MERGED (G00, head ab9d5ae1) -> 7bd1d6fb;
     #44 MERGED (G00 closure + G01 authorisation, head 5cf4f8d9) -> 390f763c;
     #45 MERGED (G01 dependency record, head 1ce88863) -> 6f075882;
     #46 MERGED (G02 RFCs 0002+0003, head 66d90635) -> 6cc96bf5;
     #47 MERGED (G02 RFC record, head f35b9b90) -> fe0e7312;
     #48 MERGED (tag-scope correction, head ed0f7b86) -> 73a35d9b;
     0 open PRs
CHECKS: PR #43 21/21 pass; PR #44 21/21 pass; PR #45 21/21 pass;
        PR #46 21/21 pass; PR #47 21/21 pass; PR #48 21/21 pass;
        post-merge CI 41248873353 on 7bd1d6fb = completed/success;
        Pages 41248873720 = completed/success;
        post-merge CI 41251300785 on 6f075882 = completed/success;
        Pages 41251300794 on 6f075882 = completed/success;
        post-merge Pages 41257608935 on fe0e7312 = completed/success;
        post-merge CI 41257609033 on fe0e7312 = completed/success on rerun
        attempt 2 (attempt 1 failed one job, see NOTE-RERUN);
        post-merge CI 41258890857 on 73a35d9b = queued at record time;
        post-merge Pages 41258890831 on 73a35d9b = queued at record time
TESTS_REALMENTE_EXECUTADOS: see "Commands actually executed" below
PROVAS: baseline reconciled; G00+G01+G01-record diffs are docs/script files
        only; src/, playground/runtime/, Cargo.toml, Cargo.lock,
        playground/runtimes/ byte-identical to c4f76907; manifest.json sha256
        unchanged; manifest.current 0.3.1; Cargo.toml 0.3.1; 0 tags at head;
        0 0.3.2 tags; repo floor cargo +1.83.0 check --locked --all-features =
        exit 0
RISCOS: R-01 try-as-expression ambiguity (gates G03 and every Result API);
        R-02 @ needs a token class that does not exist; R-03 net-new DB
        dependency surface; R-03a EVERY candidate DB driver breaks MSRV 1.83
        except rusqlite 0.37.0 (proven by build, campaign doc 1.5.1);
        R-04 no authorized DB credentials; R-06 py is a full CPython process,
        not a sandbox; R-10 E2013 blocks a prelude Result (campaign doc 1.7.1,
        decision 10.2); R-11 try in tail position is statically none while
        dynamically value-producing, and none bypasses the implicit-return check
        that "return none" fails (campaign doc 1.7.2, decision 10.3); repair
        validated (1477/1477 tests, clippy 0, 125-file corpus zero status
        changes) and then REVERTED - src/check/mod.rs is byte-identical to
        baseline, sha256 1232b687...8f532
DECISÕES: NONE TAKEN. Three open HUMAN decisions, all recorded as decision
        packages in the campaign document, none resolved by the agent.
        10.1 MSRV 1.83 versus the required DB drivers: three lawful outcomes
        (SQLite-only with documented NO_GO / authorised MSRV bump /
        feature-gated drivers with an amended 18.2 floor); two require approval
        under 2.5. G07 onward is gated on this.
        10.2 Result<T,E> versus enum-tag SCOPE, E2013: the spec contradicts
        itself (18.1/26 say programme-wide; 12/27/33.14 say per module; the
        compiler is per module). Answer D1a (which clause is normative) FIRST,
        then D1b (E2013 message wording) and D1c (placement): top-level Result
        reserving Ok/Err at ROOT scope only (needs an ADR); module-scoped
        qualified Std::Result::Ok; or declaring per-module normative and
        retiring the global clauses. Blocks NBL-030, G03 and every Result API
        in G07-G17.
        10.3 try-as-expression (D1 grammar: add try_expr to atom / use a block /
        defer) and D2, the validated three-part tail-try checker repair. Blocks
        NBL-011, NBL-031 and G03.
        No approval is implied anywhere; the three documents Directive 008 names
        as inputs are absent and must be reconstructed with explicit provenance
        notes.
PRÓXIMA ONDA AUTORIZADA: G02 only. G02 is documentation-only and is NOT blocked
        by the 10.1 MSRV decision, which first bites at G07.
```

### G00 closure evidence

- **Merged.** PR **#43**, branch `docs/nebula-campaign-g00`, head `ab9d5ae1`,
  merged into `rewrite/v3-rust` as **`7bd1d6fb`** at 2026-10-31T22:35:22Z
  (merge commit, matching the convention of PR #41 `ba8204ac` and PR #42
  `c4f76907`; branch deleted after merge).
- **Remote CI green on the PR:** **21/21** checks `pass`, 0 failing, 0 pending.
- **Diff is documentation only:** 4 files, +653/−8 —
  `docs/engineering/NEBULA_CAMPAIGN.md` (new, 509 lines), `AGENT_STATE.md`,
  `docs/engineering/CURRENT_HANDOFF.md`, `scripts/agent-state.sh`.
- **Frozen artifacts untouched.** `playground/runtimes/` diff is 0 lines and
  the trees are blob-identical (47 tracked entries each side);
  `manifest.json` sha256 `cbe7ad91…23b837` on both commits.
- **No source change.** `git diff c4f76907 7bd1d6fb -- src/ playground/runtime/
  Cargo.toml Cargo.lock` is empty.
- **No release action.** `manifest.current` still `"0.3.1"`, `Cargo.toml` still
  `0.3.1`, zero tags point at head, zero `0.3.2` tags exist locally or
  remotely. Nothing was published or promoted.

### G01 closure evidence — dependency study and the MSRV blocker

**What G01 delivered.** The dependency table, the roles/ownership map (campaign
doc §9), the ID'd backlog (§5, now with `NBL-004`), the risk register (§6, now
with `R-03a`), and the evidence criteria (§7). G01 added **no code**; `GATE G01`
forbids it before the critical contracts exist, and the only file touched is
documentation.

**The finding.** Every candidate database driver was *compiled* — not looked up
— in a throwaway crate declaring `rust-version = "1.83"`, with a lockfile
resolved by Cargo 1.98.1 and then `cargo +1.83.0 check --locked`:

| Driver | Version probed | Declared MSRV | Result on 1.83 | First blocker |
|---|---|---|---|---|
| rusqlite | 0.37.0 `bundled` | not declared | **PASS** (0) | — |
| rusqlite | 0.40.2 | 1.85 | FAIL (101) | `hashlink 0.12.2` |
| tokio-postgres | 0.7.13 | 1.81 declared | FAIL (101) | `block-buffer 0.12.1` |
| sqlx | 0.7.4 | not declared | FAIL (101) | `icu_provider 2.4.0` |
| mysql_async | 0.31.3 | not declared | FAIL (101) | `getrandom 0.4.3` |
| mongodb | 3.5.2 | **1.83 declared and correct** | FAIL (101) | `getrandom 0.4.3` |
| libsql | 0.9.24 | not declared | FAIL (101) | transitive |

**Why pinning does not rescue it.** Two hard requirements cannot be satisfied
by any 1.83-compatible release: `digest` requires `block-buffer = "^0.12"`, and
every release in that line is edition 2024 / MSRV 1.85 (the last usable,
`0.10.4`, is outside the range and `cargo update --precise` refuses it);
`icu_normalizer` and `icu_properties` require `icu_provider = "^2.4.0"`, where
`2.4.0`/`2.3.0` are edition 2024 / 1.88 and only `2.1.0`/`2.0.0` are ≤1.83 and
thus out of range. Pinning each blocker in turn on a minimal `sqlx 0.7.4` graph
(188 packages, 5 edition-2024) merely surfaced the next one, ending at
`icu_provider`, which is unsatisfiable. The same attempt on `mongodb 3.5.2`
produced a graph that resolved yet compiled with **1638 errors**.

**The sharpest datum.** `mongodb 3.5.2` declares `rust-version = "1.83"` and is
edition 2021 — an honest declaration — and still cannot build, because MSRV
declarations are not enforced across the transitive graph. This is `AGENTS.md`
rule 12 in practice: an untested environment assumption is where a newly wired
gate fails first.

**Gate mechanics that decide the options** (all reproduced): an optional driver
dependency that is *disabled* compiles on 1.83 with exit 0; the *same* project
with the feature enabled fails with exit 101. A Cargo-1.98 lockfile containing
edition-2024 packages plus `--locked --no-default-features` gives exit 0, while
`--locked --all-features` gives exit 101. This is why feature-gating is
mechanically viable — and also why it cannot be adopted silently, since it
changes what the published `--all-features` floor means.

**Baseline confirmed green.** `touch src/lib.rs && cargo +1.83.0 check --locked
--all-features` → `Checking aura-lang v0.3.1 … Finished`, exit 0. Real
recompilation, not a cache hit.

**Escalated, not resolved.** Campaign doc §10.1 records the three lawful
outcomes and states that the agent selected none, because raising the MSRV and
weakening a published validation command are both on the §2.5 human-approval
list. `libsql` is excluded from every option: it fails on 1.83 at all versions
probed.

**Not run locally.** `cargo audit` is not installed on this machine, so no local
audit claim is made; the CI `audit` job (`continue-on-error: true`) covers the
three lockfiles and both npm trees. Reported as not run, not as passing.

### What changed

- Added `docs/engineering/NEBULA_CAMPAIGN.md` — campaign plan: verified
  baseline, P0/P1/P2 scope, G00–G19 wave table with verbatim gates, six-state
  vocabulary, ID'd backlog (34 sparse identifiers spanning `NBL-001`…`NBL-190`),
  risks `R-01`…`R-09`, evidence criteria, SQLGuard controls `SG-01`…`SG-08`,
  delegation roles, blocking policy, checkpoint format, and the mandatory STOP.
- Fixed `scripts/agent-state.sh` `## Writer` extraction (P3). It required an
  exact line, so `NONE (single-writer discipline; …)` was reported as
  `WRITER=UNKNOWN`. Now matches the leading token: `WRITER=UNKNOWN` →
  `WRITER=NONE`. Verified against five value shapes; the new logic is a strict
  superset of the old. Policy unchanged. `CURRENT_TRACK`/`CURRENT_PHASE`/
  `AUDIT3` remain brittle by choice — out of the trivial-fix scope.

### Two claims that failed re-verification and were corrected

1. A §0 provenance sentence asserting `docs/engineering/` contains no
   `*NEBULA*` file was true at the baseline and made false by the document
   itself. It now states the fact at `c4f76907` and distinguishes this plan
   from the three absent inputs.
2. `NBL-001`…`NBL-190` read as a dense range. There are **34** distinct
   identifiers spanning that gap; §5 now says so explicitly.

### Commands actually executed

```text
git fetch --prune && git status --short        # clean, 0 open PRs at start
bash scripts/agent-state.sh                    # WRITER=UNKNOWN -> WRITER=NONE
node -e '<hash every manifest artifact>'       # 8 verified, 0 bad, 0 missing
git ls-files playground/runtimes | wc -l       # 47 (manifest.json + 46 .wasm)
git status --short playground/runtimes         # empty
git diff --stat c4f76907 ab9d5ae1              # 4 files, +653/-8
git diff c4f76907 ab9d5ae1 -- playground/runtimes/   # 0 lines
gh pr checks 43                                # 21 pass, 0 fail, 0 pending
gh pr merge 43 --merge --delete-branch         # merged as 7bd1d6fb
gh pr checks 44                                # 21 pass, 0 fail, 0 pending
gh pr merge 44 --merge --delete-branch         # merged as 390f763c
gh run view 41248873353                        # completed / success
gh run view 41248873720                        # completed / success (Pages)
git diff --name-status 7bd1d6fb 390f763c       # docs only
aura check / aura run probes in fresh dirs, each verified with od -c
```

G01 dependency study — every result below is a real compile, not a lookup:

```text
touch src/lib.rs && cargo +1.83.0 check --locked --all-features
    -> Checking aura-lang v0.3.1, Finished               exit 0
/tmp/msrv_rusq   rusqlite 0.37.0 bundled, cargo +1.83.0 check --locked
    -> Checking rusqlite v0.37.0, Finished               exit 0
/tmp/msrv_pg     tokio-postgres 0.7.13, default-features=false
    -> block-buffer-0.12.1/Cargo.toml: edition2024 required   exit 101
/tmp/msrv_count  sqlx 0.7.4 (188 pkgs, 5 edition-2024)
    -> edition2024 required; first blocker getrandom-0.4.3    exit 101
cargo update -p idna_adapter --precise 1.2.1   # applied
cargo update -p hashbrown --precise 0.15.5     # error: ambiguous
cargo update -p icu_provider --precise 2.0.0   # error: requires ^2.4.0
/tmp/msrv_mongo2 mongodb 3.5.2 pinned graph    # 1638 compile errors
/tmp/msrv_probe  optional dep disabled         # exit 0
/tmp/msrv_probe  same project, feature enabled # exit 101
```

`cargo audit` was **not run locally** — it is not installed. No local audit
claim is made.

### Environment constraint carried forward

The repository's custom `Aura *` agents (`Aura Adversarial Reviewer`,
`Aura Artifact Verifier`, and by extension the rest) **cannot be launched** in
this environment: they are pinned to
`customendpoint/Robin/deepseek-ai/deepseek-v4.1-flash-1017`, which is not
available. Per `AGENTS.md` the review protocol degrades cleanly, so the
adversarial and artifact-integrity reviews for G00 and G01 were executed
directly by the lead and the degradation is stated rather than faked. Future
sessions should not retry the delegation blindly.

### G01 integration evidence

- **Merged.** PR **#45**, branch `docs/nebula-g01-dependencies`, head
  `1ce88863`, merged into `rewrite/v3-rust` as **`6f075882`** with **21/21**
  remote checks green. Base was `390f763c` (PR #44).
- **Diff is documentation only.** `git diff --name-status 390f763c 6f075882`
  reports exactly three files: `AGENT_STATE.md`,
  `docs/engineering/CURRENT_HANDOFF.md`, `docs/engineering/NEBULA_CAMPAIGN.md`.
  `git diff --stat 390f763c 6f075882 -- src/ playground/runtimes/
  playground/runtime/ Cargo.toml Cargo.lock fuzz/` is **0 lines**.
- **Protected identity unchanged.** `manifest.json` sha256 still
  `cbe7ad9153ab47a362a899f8e533fa2d41d9e81131bcb9a702a1243c6c23b837`;
  `manifest.current` still `"0.3.1"`; `Cargo.toml` still `version = "0.3.1"`;
  47 tracked files under `playground/runtimes/`; 5 tags total and **0**
  matching `0.3.2`.
- **Content.** §1.5.1 records the per-driver build results, the two
  irreparable semver constraints (`block-buffer ^0.12`, `icu_provider
  ^2.4.0`), the edition-2024 census, and the non-convergence proof. §10.1
  records the open human decision with three lawful outcomes and states that
  **none was chosen**. §9.1 records the custom-agent outage.

### G02 evidence — two P0 RFCs authored, three decisions escalated

**Integrated.** PR **#46** (`docs/nebula-g02-rfcs`, head `66d90635`) merged into
`rewrite/v3-rust` as **`6cc96bf5`** at 2026-11-01T00:27:28Z, with **21/21**
remote checks green. The merged diff is 5 files / +1274 / −55, all
documentation; `src/`, `playground/runtime/`, `playground/runtimes/`,
`Cargo.toml` and `Cargo.lock` are **byte-identical** to `6f075882`,
`manifest.json` sha256 still `cbe7ad91…23b837`, `manifest.current` still
`"0.3.1"`, `Cargo.toml` still `0.3.1`, and no `0.3.2` tag exists.

**Files.** `docs/rfcs/0002-result-and-recoverable-errors.md` (305 lines) and
`docs/rfcs/0003-try-as-value-expression.md` (486 lines). Neither is accepted,
and neither adds source code.

**RFC 0002 — `Result<T,E>`.** The decisive finding is that the directive's §2.4
vocabulary (`Result<T,E>` with `Ok(T)`/`Err(E)`) needs **no grammar change**:
generic enums already work. `enum Res<T, E> { Ok(T), Bad(E) }` + `Res::Ok(7)` +
match prints `7`; `Err` works as a tag; `Result` in a struct field,
`[R<int, string>]` inside a list, and `-> R<int, string>` return positions all
work; `R::Ok(1, 2)` is `E3001: variant 'Ok' expects 1 value(s), got 2`. Grammar
delta is **None**.

Three claims in the first draft were falsified by probe and retracted:
`Result` **is** usable as a user type name (there is no builtin type registry —
`grep -rn 'builtin_type\|prelude_enum\|inject_enum' src/` returns nothing and
no `std/` directory exists); matching is **not** exhaustive
(`LANGUAGE_SPEC.md:2471–2472`: "no exhaustiveness requirement … if no arm
matches, the result is `E4029` at runtime"); and enum tags are **not** scoped to
their enum.

**Correction — the tag-scope premise was wrong, and the spec contradicts
itself.** The first version of this section said tags are unique
"programme-wide" and that a top-level `Result` "permanently claims `Ok` and
`Err` for every user enum in the compilation unit". Both statements are false.
`LANGUAGE_SPEC.md` asserts **both** rules: `:2407` (§18.1) and `:3281` (§26) say
programme-wide; `:3399` (§27, `CONF-RESOLVE-6`), `:1556` (§12) and `:3897`
(§33.14, a *Frozen Design Decision*) say **per module**. The compiler implements
per module and prefixes tags with the module path
(`src/resolve.rs:1452-1455`), so the diagnostic for a duplicate inside
`module M` reports the tag as **`M::Ok`**.

Probes (one `.aura` file per directory): duplicate tags at root scope → `E2013`;
duplicate tags inside one module → `E2013` reported as `M::Ok`; root `Ok` plus
`module M { Ok }` in **both declaration orders** → exit 0; sibling modules →
exit 0; nested modules → exit 0; root `Ok(1)` and `M::Ok("s")` printed together
→ exit 0, prints `Ok(1) M::Ok("s")`. A root-scope `enum Result { Ok, Err }` plus
a root-scope `enum Parsed { Ok }` is `E2013` **even though `Result` is never
constructed**.

`E2013`'s message text also misdescribes the rule: `src/check/mod.rs:1378` emits
"variant names must be unique across the program" for a scope-local check. No
test matches the string (`tests/grammar.rs` matches `codes::DUPLICATE_VARIANT`)
and `docs/errors.md:37` records only the code, a title and a trigger, so fixing
the wording is low-risk — but it is escalated as `D1b` because the correct
wording depends on `D1a`.

The blocker is therefore `R-10`, but its shape changed. A top-level `Result`
claims `Ok`/`Err` for the **root scope only**; modules escape the rule. A
`use`-imported module enum still does **not** bring its tags into unqualified
scope (`E3002`), and braced variant import (`use Std::Result::{Ok, Err}`) is a
parse error (`E1006`). Qualified construction works: `Std::Ok(7)`,
`Std::Result::Ok(7)`, and `Result::Ok(7)` after `use Std::Result` all print `7`.
No ADR governs the tag rule.

**Prior art, recorded so this is not claimed as a discovery:**
`docs/engineering/EXCEPTION_SYNTAX_DECISION_PACKAGE.md:192` (verification pass
2026-10-06) already documented the per-module behaviour with bidirectional
execution evidence, including sibling modules each declaring an `Ok`. That
package's scope was the exception system, so it recorded the behaviour without
raising the spec contradiction. What is new is that the two normative clauses
were never reconciled and the diagnostic still asserts the losing one.

**RFC 0003 — `try` as a value expression.** Two independent results.

*Grammar (D1).* `try` is statement-only: `docs/grammar.md:75-77` puts
`try_stmt` in the statement alternation, `:100` is `expr = pipe`, and `:118-124`
lists `block`, `if_expr`, `match_expr` in `atom` but **not** `try`. Probes:
`let x = try {…} catch e {…}`, `print(try …)`, and `if (try …) == 1` all report
`E1006: expected an expression, found 'try'`; `return try {…}` reports a
**different** error, `E1006: expected a newline between statements, found
'try'`, because in `return` position `try` parses as a *statement*. That
divergence is the ambiguity the brief forbids resolving heuristically. Blocks
are already expressions (`let a = { 1 }` prints `1`), so a zero-grammar-change
route exists.

*Pre-existing checker defect (D2), `R-11`.* A tail `try` already yields a value
at run time — `fn f() -> int { try { 41 } catch _ { 99 } }` prints `41` and the
`throw` variant prints `99` — and `src/run/mod.rs:1264` shows it is deliberate
(`Ok(flow) => Ok(flow)`; `finally` replaces only a non-`Ctl::Val` outcome).
The checker disagrees on three counts: `block_value_ty`
(`src/check/mod.rs:2357`) types a `Stmt::Try` tail as `Ty::None`, making `try`
the only construct statically `none` but dynamically value-producing;
`block_tail_mismatch` (`:5893`) short-circuits on `Ty::None` **before** calling
`return_compatible`, so `fn f() -> int { none }` is accepted while
`fn f() -> int { return none }` is `E3005`; and the same function's
`let Some(Stmt::Expr(..)) = body.last() else { … }` means a **direct** `try`
tail is never compared at all.

The consequence is end-to-end unsoundness: `fn f() -> string { try { 5 } catch
_ { 6 } }` with `let s: string = f()` checks clean and prints `5`.

**Repair validated, then withheld.** Three coupled parts (a `Stmt::Try` arm in
`block_value_ty`, removal of `Ty::None` from the short-circuit, and a
`Stmt::Try` arm in `block_tail_mismatch`). Every partial configuration was
built and measured, and each part is independently necessary. Under the full
repair: `q1`–`q6` become `E3005` with diagnostics **byte-identical** to the
explicit-`return` oracle; `r1`/`r3` (the generic constructor idiom and
`-> none { none }`) are unregressed; `r2`/`r6` are repaired; the `s1`/`s4`/`s5`
false positives are gone; `m1`/`h4`/`i1`/`p3`/`j6` are rejected at the function
boundary; `c4`, `d2`, `d3`, `d5`, `d6`, `e1`, `e3`, `e4`, `e5`, `e6`, `e8`,
`g5`, `g6` all still pass. Two intended flips: `i2` reports `none | int`
(accurate) and `j5` (`-> none` returning `5`) reports `E3005`, consistent with
the long-standing `r4`. `p2` turned out to be a latent false negative
(`-> [int]` returning the integer `5`), not a working program.

Gates under the full repair: `cargo test --locked --all-targets --all-features`
gives **1477 passed, 0 failed**; `cargo clippy --locked --all-targets
--all-features -- -D warnings` gives **exit 0**; a sweep of all **125** tracked
`.aura` files gives **17 non-zero before and 17 after — zero status changes**.
The source was then restored to baseline: sha256
`1232b6874dd2e9c4f556085b4f3b240c120e6dedf77baace9a8e5d6aa738f532`, and
`git diff --stat -- src/check/mod.rs` is empty. `GATE G02` forbids coding the
affected surface before acceptance, so the repair is evidence, not a commit.

**Spec-sanctioned exemption — not part of the defect.**
`LANGUAGE_SPEC.md:1819–1826` states that a body ending in a declaration, a
loop, or a `return` has no implicit value and is not judged.
`fn f() -> int { print(1) }` and `fn f() -> int { }` being accepted is
therefore correct and must survive any future repair.

**Probe methodology correction.** An apparent "`aura check` parses sibling
files" defect was investigated and **falsified**:
`docs/FILESYSTEM_MODULES_DESIGN.md` is a frozen contract whose `:399` decides
"eager structural discovery" and whose `:774` matrix row makes an unused
malformed reachable child a parser failure. Every probe in this wave therefore
uses one `.aura` file per directory. Recorded as a rule in campaign doc §7.

**NOTE-RERUN — a confirmed CI flake, not a regression.** The post-merge CI run
`41257609033` on `fe0e7312` failed **exactly one** of 21 jobs: `website (static
build + examples + links)`, in `website/tests/browser.test.mjs:627`
(`browserContext.waitForEvent: Timeout 30000ms exceeded while waiting for event
"page"`). That assertion opens a popup via a modifier click and races a 30 s
Playwright timeout; it is a timing flake. It cannot have been content-induced:
the commit range `6cc96bf5..fe0e7312` touches **no** file under `website/`
(0 lines), and the same job passed on PR #47 and on the push run for `6cc96bf5`.
`gh run rerun 41257609033 --failed` reached attempt 2 =
`completed`/`success`, confirming pre-existence without any code change.
Do not "fix" a future recurrence without re-proving it this way first.

### Next exact action

Continue **G02**: reconstruct the remaining P0 RFC set (`NBL-012`…`NBL-022`)
under `docs/rfcs/`, using `docs/rfcs/0001-exception-catch-selection.md` as the
structural template and campaign doc §2.2 as the required section list. Each
RFC carries an explicit provenance note stating that its nominal source
document was absent and that no approval is implied.

`NBL-010` (RFC 0002) and `NBL-011` (RFC 0003) are authored. RFC 0002 was
corrected after its tag-scope premise was falsified, recorded at `fe0e7312`
and integrated as `73a35d9b` (PR #48); the campaign
now escalates `D1a` (which of the two contradictory spec clauses governs
`E2013`), `D1b` (`E2013`'s message text) and `D1c` (placement). Answer `D1a`
before `D1c`. Still unauthored:
`NBL-012` inference and error propagation; `NBL-013` diagnostics contract and
`aura explain`; `NBL-014` AIS capability set; `NBL-015` `Aura.toml` / `aura
init` / `std::os`; `NBL-016` Host database capability and protected profile;
`NBL-017` `entity struct` and metadata; `NBL-018` CRUD and transactions;
`NBL-019` AQL/0.1; `NBL-020` Repository in traits; `NBL-021` SQLGuard/0.1
SG-01…SG-08; `NBL-022` SQL and Mongo driver contract. `NBL-030` (prelude
`Result` placement) is blocked by §10.2 and must not be authored as if the
decision were taken.

**Do not code any surface these RFCs describe, and do not resolve §10.1, §10.2,
or §10.3.** G02 is documentation-only and is **not** blocked by the §10.1 MSRV
decision, which first bites at G07. `GATE G02` requires the P0 RFCs to be
accepted first.

## Directive 007 — runtime distribution parity (`0.3.2-dev.9`, `PASS`)

- **The distributed WASM carried its own, older semantics.** Freeze point
  `31346b53` last touched `playground/runtime/`; sixteen `src/` corrections
  landed after it with no matching artifact. On the 18 verbatim regressions in
  `tests/regressions.rs`, `0.3.1` and `0.3.2-dev.8` each scored **4/18** (11
  wrong, 3 traps) while native `HEAD` scored 18/18. Both stale artifacts also
  **accepted four type-soundness violations** the current checker rejects.
- **New immutable identity `0.3.2-dev.9`:** 2 010 781 B, ABI 2,
  sha256 `8de654f0acc894595975ac1ec89c9e239a4ef89904ed3a9dd65a8af2589112b5`,
  source fingerprint
  `0e252f8f40a6c04592864fa9755e4d0c3983ed5ab2a7b4eac4163ee48df5652b`.
  A genuine clean rebuild (`target/wasm32-unknown-unknown` **and** the runtime
  directory removed) is byte-identical. That is **host-scoped**, not a
  cross-host reproducibility proof. `cargo clean -p` removes 0 files here and
  is not a valid clean-rebuild proof (no root `[workspace]`).
- **Parity proven on the bytes that ship, not on `target/`:** `dev.9` **18/18**
  with 0 wrong and 0 traps; 20 supplementary cases **20/20**, agreeing with
  native 20/20 (dev.8 managed 12/20).
- **Protection is four-layered.** `playground/source-fingerprint.mjs` hashes the
  34 build inputs (both Cargo manifest/lock pairs, `rust-toolchain.toml`, 29
  `.rs` files) self-reference-free. `build.mjs` records it as
  `source_fingerprint`; `--check` enforces **presence**, while
  `--check --require-source-match` enforces **equality** and runs on **tag
  builds only** in `release.yml`. An ordinary PR touching `src/` must stay
  green, so equality there would be wrong. `distribution-parity.test.mjs`
  (**39/39** on dev.9, 17/39 on dev.8 and `0.3.1`) is wired into `run-all.mjs`.
  Five suites that hardcoded `dev.8` now **derive** the identity from
  `playground/runtime/Cargo.toml`.
- **Silent-pass hole found and closed:** the provenance loop only inspects
  entries that exist, so a manifest *omitting* the current identity passed
  `--require-source-match` vacuously. `sawRuntimeIdentity` now fails it.
  Mutation-verified: entry removed → plain exit 0, strict exit 1.
- **Not a silent substitution.** `31346b53` itself replaced the `dev.7`
  manifest entry with `dev.8` while leaving the `dev.7` directory on disk; this
  follows that convention. `0.3.1` and `0.3.2-dev.8` bytes are untouched and
  still loadable; they are simply no longer the line's advertised preview.
- **Manifest unchanged in the ways that matter:** `current` is still `0.3.1`,
  `PROMOTE_DEV_TO_DEFAULT` is still `false`. No tag, no publication, no frozen
  artifact overwritten, no language-semantics change.
- **Pre-existing flake, not a regression:**
  `tests/bench.rs::module_graph_scales_subquadratically` failed once against
  its `3.2x` ceiling. This work changes no Rust file, and **base `85b42fa5`
  itself fails the same guard on 2 of 3 full-suite runs** (on a *different*
  case, `lex(long_string)`, at 4.22x and 3.24x). The ratio guard is
  load-sensitive. It was **not** weakened.
- **Full validation green:** fmt, clippy `-D warnings`, all-features and
  no-default-features suites, MSRV `1.83.0`, fuzz `check`, bounded fuzz
  (parser 429 338 runs, checker 256 777, runtime 729 — no crashes),
  `run-all.mjs` all suites, website build + suite, both `--check` modes, 9/9
  artifacts hash+size verified.
- **Delivered and merged.** PR **#41**
  (`directive-007-runtime-distribution-parity`, head `ce1b2538`) merged as
  **`ba8204ac`** with **21/21 checks `SUCCESS`**. Post-merge CI
  [`41240576369`](https://github.com/JoaoValentimTheo/aura-lang/actions/runs/41240576369)
  is green on all 21 jobs at `ba8204ac`, and the Pages deploy
  [`41240576317`](https://github.com/JoaoValentimTheo/aura-lang/actions/runs/41240576317)
  succeeded. Budget used: **1 of 2** authorized PRs.
- **The `macos-latest` flake behaved as diagnosed, not as claimed.** PR #41
  first reported that job red twice on **different** bench cases
  (`parse(deep_expr)` 3.35x, then `module_graph(children)` 3.35x); a re-run
  passed, and the post-merge run on the identical tree was green. Prior CI
  history had already failed the same `tests/bench.rs:252` guard on an
  unrelated commit (`47232a79`, run `41110991797`). The guard was **not**
  touched. The canonical branch is not protected, so nothing was bypassed —
  the required checks genuinely ran and passed.
- Report:
  [`docs/engineering/RUNTIME_DISTRIBUTION_PARITY.md`](docs/engineering/RUNTIME_DISTRIBUTION_PARITY.md).
- **Next action:** nothing pending. Directive 008 is **not** authorized. Any
  tag, stable publication, or promotion of `dev.9` to `current` requires a
  separate explicit human decision.

## Directive 006 — release readiness assessed (`INCOMPLETE`, `NO-GO` to distribute)

**Status:** assessment delivered; **no code changed**. Report:
[`RELEASE_CANDIDATE_READINESS.md`](./RELEASE_CANDIDATE_READINESS.md), shipped by
**PR #40** — one documentation PR, the full Directive 006 budget.

- Reference identity **`3347ff6c`**; worktree clean; local == remote; 38 merged
  PRs; stable `0.3.1`; preview `0.3.2-dev.8`; tags unchanged.
- **Material finding:** the two selectable Playground WASM runtimes are stale
  **semantically**. The 18 verbatim regressions in `tests/regressions.rs` score
  `0.3.1` **4/18 PASS / 11 WRONG / 3 TRAP**, `0.3.2-dev.8` **4/18 / 11 / 3**,
  native HEAD **18/18**. Both artifacts **accept four type-soundness
  violations** native HEAD rejects (`E3001` ×3 for wrong `set` element types,
  `E3005` for a return-type mismatch), and one diagnostic diverged
  (`E3001` → `E4031`). `0.3.2-dev.8` fixes nothing vs `0.3.1` despite ABI 2.
- Freeze point **`31346b53`** is the last commit touching `playground/runtime/`
  and the only commit that created `runtimes/0.3.2-dev.8/`; nothing under
  `playground/runtimes/` has changed since. **16 `src/` and 79 total commits are
  unshipped.** `build.mjs` forbids overwriting an identity ⇒ a corrected
  candidate needs a **new dev identity**, which requires human authorization.
- This is **not** a source-tree regression and does **not** contradict Directive
  005: the local matrix at `3347ff6c` is green and remote CI is 23/23 (CI run
  `41225313680` 21/21, Pages `41225313675` 2/2). Directive 005's scope never
  covered distributed artifacts.
- Frozen-artifact integrity re-verified independently: **8/8** available
  artifacts match `manifest.json` sha256 and byte length exactly. Their *bytes*
  are intact; what is stale is their *provenance relative to source*.
- Rebuild determinism was shown on this host only (4 identical builds,
  `5499a73384…`, 2 010 781 B). That is **not** a cross-host reproducibility
  claim; the manifest/source hash difference is source drift.
- Verdict **`NO-GO` to distribute the existing artifacts**, `HUMAN DECISION
  REQUIRED` to build a corrected candidate. Six pending decisions in §10 of the
  report; **nothing** tagged, published, or promoted.

**Exact next action: none — this is a human decision point.** Do not rebuild the
runtime, bump `playground/runtime/Cargo.toml`, correct `CHANGELOG.md:12-16`, add
the staleness CI gate, or open any follow-up PR without a fresh human directive.
Each of those is enumerated as a separate authorization in the report.

## CAMPAIGN 003 CLOSED — Directive 005 controlled closure (`COMPLETE`, `GO`)

**Status:** the stabilization campaign is **closed**. Report:
[`CAMPAIGN_003_CLOSURE.md`](./CAMPAIGN_003_CLOSURE.md).

- Branch `rewrite/v3-rust` at **`86e2b5ba`** = `origin/rewrite/v3-rust` (merge of
  **PR #39**, this closure); worktree clean; crate and published stable version
  both **`0.3.1`** (Keystone).
- Six-area bounded audit: **six `PASS`, zero P0/P1**. Validation floor fully
  executed and green (1477 passed / 0 failed / 14 ignored across 66 suites;
  `fmt`; `clippy -D warnings`; MSRV 1.83; Miri 132/0; no-default-features
  matrix; `artifact-smoke` 12/0; all five fuzz targets clean with an empty
  `fuzz/artifacts`; Playground and all seven website suites; frozen runtimes
  8/8 `sha=OK bytes=OK`). Remote CI on the audit commit `8cc9ed32`: 21/21 jobs
  green (run `41217451481`), website deploy `41217451457` green. Post-merge CI on
  `86e2b5ba`: 21/21 jobs green (run `41223357639`), website deploy
  `41223357675` green.
- **Zero** corrective PRs were opened under Directive 005 (budget 0/6). The one
  corrective candidate raised (sibling module contamination) was **falsified**
  against `docs/FILESYSTEM_MODULES_DESIGN.md` — eager structural discovery is
  the frozen intended contract. Do not re-open it.
- Verdict **`GO`** covers release-candidate *preparation* only. Promoting a
  release candidate to stable, publishing a runtime, or moving a tag remain
  human-gated and were not performed.
- Deferred P2/P3 items (TD-01, TD-03, TD-04, TD-07, TD-09, TD-10, TD-11, TD-13,
  TD-17, TD-19, TD-22, TD-24) and residual risks (R-15 unbounded allocation
  growth, R-10 frozen artifacts stale-by-policy, R-12 compatibility note,
  R-07/R-08/R-14) are recorded in the report. **None was implemented.**

**Exact next action: none.** Directive 005 forbids starting a new campaign or
continuing to hunt for low-priority defects. The next step is a human decision
about release-candidate preparation. Do not open a corrective PR, do not begin
language features, do not touch frozen artifacts, and do not publish — unless a
fresh human directive explicitly authorizes it.

## Closed task — CORE FIRST: a statically empty `range` is not statically empty — merged `00aeb188` (PR #38)

**Status:** merged as `00aeb188`; twenty-one remote checks green,
`mergeStateStatus=CLEAN`. Post-merge diff `29736de7..00aeb188` contains exactly
the five intended files (`src/check/mod.rs`, `tests/keystone_types.rs`,
`CHANGELOG.md`, this handoff, `AGENT_STATE.md`), and the working tree is clean.
Nothing open; the next CORE FIRST finding is not yet selected — and Directive
005 closed the campaign before one was chosen.

### The defect

`statically_empty_iterable` in `src/check/mod.rs` decides whether a `for` body
can ever run. It recognized the empty literal shapes (`[]`, `{:}`, `set{}`) and
an `Expr::Range` whose two bounds were `Lit::Int` values **equal** to each
other — and nothing else. A twelve-case probe measured the consequence:

| shape | before | after |
|---|---|---|
| `range(0, 0)` | `E3006` ✗ | accepted |
| `range(3, 3)` | `E3006` ✗ | accepted |
| `range(n, n)` | `E3006` ✗ | accepted |
| `range(5, 2)` | `E3006` ✗ | accepted |
| `range(0)` | `E3006` ✗ | accepted |
| `range(-5)` | `E3006` ✗ | accepted |
| `5..2` | `E3006` ✗ | accepted |
| `1..-1` | `E3006` ✗ | accepted |
| `0..0` | accepted ✓ | accepted |
| `range(0, 1)` | `E3006` ✓ | `E3006` |
| `range(1)` | `E3006` ✓ | `E3006` |
| `0..1` | `E3006` ✓ | `E3006` |

Eight false positives, four correct. Each false positive rejects a sound
`-> never` function: the body of `for _x in range(n, n) { return }` cannot run,
so the function cannot return, and the trailing `throw` diverges.

### Why this is a defect and not a specification choice

The spec states the correct behaviour in four places, and the code contradicted
all four: §22.1 requires the literal and builtin spellings to be "statically
equivalent as well as dynamically equal" (line ~2802); §22.1 says "an empty or
descending range (`b <= a`) is empty" (~2809); §22.1 defines `range(n)` as
`range(0, n)` (~2784); §5.2 names `range(n, n)` explicitly as a statically
empty iterable whose `for` body is unreachable (~1054). No spec edit
accompanies this fix — the spec was the evidence.

### The fix

`range_bound` classifies a bound as either an exact `Lit(i64)` (with a unary
minus folded, since the parser only pre-folds `-9223372036854775808`) or a
`Name`, and `range_bounds_are_empty` answers `end <= start` for two literals
and "the same name on both sides" for two names. A name pair is sound because
assignments are statements: two reads of one binding inside one argument list
observe the same value. Everything else — a call, a field, an index, two
different names — may ascend and stays non-empty. `builtin_range_args` accepts
only a bare `Name("range")` callee with no type arguments and no named
arguments, then treats `range(end)` as `range(0, end)`. `range` is a reserved
name (`E1009`), so no name resolution is needed. `statically_empty_iterable` is
a free function, so the `expr_inner` frame hazard (REG-1) is untouched.

### Next exact action

Done. Committed `77cb34d6`, pushed `fix/statically-empty-range-reachability`,
opened PR #38, obtained 21/21 green checks, and merged with
`gh pr merge 38 --merge --delete-branch`. Select the next CORE FIRST finding
from the lead list in `AGENT_STATE.md`.

---

## Previous task — CORE FIRST: native HTTP header values are erased in one direction and UTF-8 encoded in the other — merged `fb3e8dee` (PR #37)

**Status:** merged as `fb3e8dee`; twenty-one remote checks green,
`mergeStateStatus=CLEAN`. Post-merge diff `4cbf6afb..fb3e8dee` contains exactly
the twelve intended files.

### The defect

The two HTTP hosts disagreed about what a header value is. `fetch` treats a
header value as a byte string and widens it to ISO-8859-1; the native `ureq`
path treated it as UTF-8 text. Four measurements on a loopback server
established this, and each one is a distinct symptom of the same root cause:

| # | Probe | Browser host | Native host (before) |
|---|---|---|---|
| A | response `x-latin: caf\xE9` | `[99,97,102,233]` = `café` | `""` — the value is gone |
| B | request value `café` | octet `0xE9` | `0xC3 0xA9` (UTF-8) |
| C | request value `\u4e16` | `TypeError` → `E4020` | silently sends `0xE4 0xB8 0x96` |
| D | response re-sent as a request header | `x-latin: caf\xE9` | `x-latin: ` — empty on the wire |

Two independent root causes, both in `src/host.rs`:

1. **Response direction: erasure.** The response map built each value with
   `value.to_str().unwrap_or("")`. `HeaderValue::to_str` returns `Err` for any
   byte with the high bit set, so a legal `obs-text` value (RFC 9110 §5.5
   permits `%x80-FF`) became the empty string — not an error, not a lossy
   replacement, just silence. The same erased string fed `header_lines`, which
   is why symptom D round-trips the corruption end to end.
2. **Request direction: wrong encoding.** Values were handed to `ureq` as
   `&str`, which sends them UTF-8 encoded. The reference host sends the
   ISO-8859-1 octet, so the same Aura program put different bytes on the wire
   depending on which host ran it.

Symptom C is the one that decides the contract: the browser **already** refuses
a code point above `U+00FF`, and `playground/web/worker.js:174-199` has always
surfaced that `TypeError` as `E4020`. Native success in that case was therefore
not a capability but a divergence.

### The fix — ISO-8859-1 in both directions, browser as the reference host

`docs/adr/0006-http-header-value-encoding.md` is the governing contract; this
section records the implementation.

1. `src/host.rs` — three helpers above `perform`:
   - `latin1_text(&[u8]) -> String` — octet to code point, one-to-one. This is
     a lossless widening, so **no `header_bytes` key is needed**: unlike `body`,
     whose text view is bounded by UTF-8 validity, the text view here is exact.
     `String::from_utf8_lossy` is the wrong tool — it yields `U+FFFD` where the
     correct answer is `U+00E9`.
   - `latin1_octets(&str) -> Result<Vec<u8>, char>` — the inverse; `Err` carries
     the first code point above `U+00FF`.
   - `apply_headers<B>(builder, headers) -> Result<_, _>` — encodes and applies
     once, replacing the two closures (`apply`/`apply_body`) that each arm used
     to duplicate.
2. `src/host.rs` — all twelve match arms in `perform` call `apply_headers(...)?`
   instead of building a request inline, and the response map uses
   `latin1_text(value.as_bytes())` instead of `to_str().unwrap_or("")`.
3. `src/stdlib/ext.rs` — `build_request` range-checks every header value and
   returns `E4020` naming the header and the code point (`U+XXXX`), placed
   after the method/body check and before `HttpRequest` is constructed, so no
   request is dispatched.

**Why the bound is enforced twice on purpose.** `build_request` is documented as
"Shared by the synchronous and resumable paths so validation never diverges",
and `src/stdlib/ext.rs` is gated on `http-api` while the provider in
`src/host.rs` is gated on `http`. The check in `build_request` therefore also
runs in browser/WASM builds that never compile the native provider, and
`latin1_octets` independently stops a foreign `HttpRequest` from smuggling
UTF-8 onto the native wire.

### The tests — three new, all proven to fail without the fix

`tests/http.rs`, all `#[cfg(feature = "http")]` via the file-level gate:

| Test | Asserts | Pre-fix symptom |
|---|---|---|
| `non_ascii_response_header_value_is_preserved` | `caf\xE9` arrives as the four-character string `café`, and `header_lines` carries the identical value | got `"\n0\nplain\n\n"` |
| `outgoing_non_ascii_header_value_uses_latin1_octets` | the raw request bytes contain `0xE9`, not `0xC3 0xA9` | wire carried UTF-8 `café` |
| `header_value_above_latin1_is_reported_not_sent` | `E4020` and no bytes on the wire | returned `"200\n"` |

`CapturingServer::request_bytes()` was added because the existing
`request_text()` is lossy UTF-8 and therefore *cannot* distinguish `0xE9` from
`0xC3 0xA9` — the two encodings decode to the same `String` through it.

### Compatibility

This is a **wire-octet change**, recorded rather than assumed. A native program
that sends non-ASCII request header values now emits ISO-8859-1 where it
previously emitted UTF-8; one that sends a value above `U+00FF` now gets
`E4020` where it previously succeeded by accident. Both directions move toward
the browser, which is the reference host, and the second case was never
portable. `CHANGELOG.md` states this under `[Unreleased] / Fixed`.

### Files that do not change, and why

- `playground/web/worker.js:308-310` — the browser already decodes response
  values as ISO-8859-1, so the runtime receives `café` and needs nothing.
- `playground/runtime/src/lib.rs:1473-1512` — `headers`/`header_lines` projection
  is encoding-agnostic; it reads `as_str()` off an already-decoded string.
- `playground/web/worker.js:174-199` — already reports `E4020` for an
  out-of-range request value. The native fix adopts that code rather than
  inventing one.

### Next exact action

Done. Committed `21f1b539`, pushed, opened PR #37, obtained 21/21 green checks
and merged with `gh pr merge 37 --merge --delete-branch`.

---

## Previous task — CORE FIRST: JSON decode depth is asymmetric with encode, and reports the wrong error family — merged `4cbf6afb` (PR #36)

**Status:** merged as `4cbf6afb`; twenty-one remote checks green,
`mergeStateStatus=CLEAN`. Post-merge diff `4aff6d5c..4cbf6afb` contains exactly
the eight intended files.

### The defect

### The defect

`json_encode` and `json_decode` disagreed about the same document. Round-tripping
a 200-deep recursive value produced JSON that Aura then refused to read back:

```
struct Node { child: Node | none }
fn build(n: int) -> Node {
    if n == 0 { return Node { child: none } }
    return Node { child: build(n - 1) }
}
fn main() {
    let d = build(200)
    let s = json_encode(d)
    printf(len(s))          # 2014 bytes of legal JSON
    printf(json_decode(s))  # E3001: recursion limit exceeded at line 1 column 1144
}
```

Binary search pinned the wall: **depth ≤ 126 decoded, depth ≥ 127 did not**. Two
independent root causes, both real:

1. **Depth.** `to_json_depth` (`src/stdlib/ext.rs:61`) stops at
   `MAX_VALUE_DEPTH` = 512 and writes `null`, so the encoder was *deliberately*
   bounded at 512. The decoder used `serde_json::from_str`, whose `Deserializer`
   carries `remaining_depth: 128` (`de.rs:63`) since serde has no stack-size
   knowledge. Aura therefore emitted documents it could not parse. Worse, the
   guard in `from_json_depth` was **dead code for deep documents** — serde
   rejected them before the conversion ran, so the comment claiming *"serde_json's
   parser already limits nesting before this point; the bound keeps the
   conversion total"* named exactly the dependency the defect violated.
2. **Error family.** `json_decode` reported malformed or over-deep documents as
   `E3001` (`TYPE_MISMATCH`, a *checker* code) while `json_decode_as` and typed
   decode reported `E4031` (`DECODE_MISMATCH`, *runtime*). Same document, two
   codes. Spec §2.2 puts nesting/recursion in `E4xxx` and §30.1 reserves `E3xxx`
   for type-level checks, so `E3001` was wrong on both counts.

### The fix — one shared gate for all three decode entry points

1. `Cargo.toml:16` — `serde_json` features gain `unbounded_depth`, because
   `Deserializer::disable_recursion_limit` is gated on it.
2. `src/stdlib/ext.rs` — new `fn parse_json(text: &str, span: Span)`:
   - `max_nesting(bytes)` — allocation-free single pass over the bytes that
     tracks string/escape state and returns the deepest `[`/`{` nesting. `O(len)`,
     no recursion, cannot exhaust the stack on a hostile document.
   - Rejects with `E4031` before the parser runs if that depth exceeds
     `MAX_VALUE_DEPTH`.
   - Then `Deserializer::from_str` → `disable_recursion_limit()` →
     `Value::deserialize(&mut de)` → **`de.end()`**.
   - Every failure becomes `decode_err` (`E4031`) with `malformed JSON: {e}`.
3. `src/stdlib/ext.rs` — all three `serde_json::from_str` call sites
   (`decode_typed`, `json_decode`, `json_decode_as`) now route through
   `parse_json(...)?`. `json_decode`'s non-string argument keeps `err`
   (`E3001` is correct for a genuine type error).
4. `src/error.rs:431–437` — `DECODE_MISMATCH`'s doc comment widened to cover both
   entry points, the nesting bound and the pre-scan.

**Why `de.end()` is mandatory:** `serde_json::from_str` rejects trailing garbage;
`Deserializer::from_str` + `Value::deserialize` does not. Probe `"1 2"` →
`E4031: malformed JSON: trailing characters at line 1 column 3`, so the
rejection `from_str` provided is preserved.

**Why the pre-scan makes the escape hatch safe:** `disable_recursion_limit`
removes serde's own guard *entirely*. `max_nesting` reinstates a bound that is
cheap, strictly more permissive-correct than 128 (every document that parses has
exactly the nesting the scan reports), and now equal to the encoder's. Aura
executes on a dedicated 64 MiB stack (`src/lib.rs:103` `INTERP_STACK`), so 512
levels of serde recursion (~64 KiB) is trivial. `serde_json::Value` has **no**
`Drop` impl (`carefully_drop_nested_arrays` exists only in a doc example), so
dropping a deep `Value` is an ordinary recursive drop bounded by the same 512.

### The tests — three new, all proven to fail without the fix

`tests/regressions.rs`, `BH1-07..09`, all `#[cfg(feature = "json")]`:

| Test | Asserts |
|---|---|
| `bh1_07_deep_json_round_trip_is_lossless` | `build(200)` → decode → re-encode is **byte-identical** |
| `bh1_08_decode_depth_bound_matches_encode` | depth 512 decodes; depth 513 is `E4031`; the encoder's `null` past the bound still decodes |
| `bh1_09_decode_failures_share_the_runtime_code` | both entry points give `E4031` on malformed input; `"1 2"` rejected; a 100 000-deep hostile document is a clean `E4031` with no crash; `json_decode(42)` **still** `E3001` |

With the three source files reverted (`git checkout -- src/stdlib/ext.rs Cargo.toml
src/error.rs`), all three fail with the original symptom —
`Diag { code: 3001, message: "recursion limit exceeded at line 1 column 1144" }`,
`left: Err(3001) / right: Ok(())`, `left: 3001 / right: 4031`. Restored, all
three pass, and the whole `bh1` family is 9/9.

### Documentation and reachability

- `docs/LANGUAGE_SPEC.md` §30.2 — the missing `E4031` row.
- `docs/LANGUAGE_SPEC.md` §24 — new **"Normative rule (JSON decode depth)"**; the
  typed-decode rule now says both entry points report mismatch as `E4031`.
- `docs/LANGUAGE_SPEC.md` §31.6 — decode-depth normative rule; evidence line
  repointed to include `parse_json`.
- `docs/errors.md:74` — `E4031` retitled from "Typed JSON decode mismatch" to
  "JSON decode mismatch" and widened.
- `tests/grammar.rs` — a `DECODE_MISMATCH` entry in `error_samples()`, so §30.4's
  "every §30.2 code is reachable" claim and
  `every_documented_error_code_is_reachable` both still hold.

Explicitly **not** swept into `E4031`: the non-string-key rule and the arity
rule in §24. Those are genuine type errors and keep `E3001`.

### Next exact action

Merged. Next work item is recorded in the current task section above.

---

## Previous task — CORE FIRST: method calls on a type parameter resolve by bounds, not name existence — merged `4aff6d5c` (PR #35)

**Status:** merged as `4aff6d5c`; twenty-one remote checks green,
`mergeStateStatus=CLEAN`. Post-merge diff `1fa01db0..4aff6d5c` contains exactly
the five intended files.

### The defect

`Expr::Method` on a receiver of type `Ty::Param` had no method table, so
`src/check/mod.rs` fell through to a conservative fallback that asked only
whether a method *named* `name` existed on **any** type in the program. Trait
bounds were never consulted. Two coupled sub-bugs:

| # | Shape | Before | Contract |
|---|---|---|---|
| A | `trait Show { fn show(self) -> string }` + `fn render<T: Show>(x: T) { return x.show() }`, **no impl anywhere** | `E2003 no method 'show' on any type` | §36.4 / `GENERICS.md` §5: a bound *grants* the call |
| B | `fn render<T>(x: T)` (no bound) calling `x.show()`, with an unrelated `struct A` declaring an inherent `show` | accepted; failure deferred to a runtime `no method` error | a bound is required, so this must be `E2003` |

Declaration order was irrelevant (`impl` before or after the generic `fn` both
worked), and a bounded generic that is never instantiated was rejected too — the
discriminator was pure name existence. A third candidate, `x.to_string()` on a
bare `T`, was **dismissed**: `to_string` and `equals` are free functions, not
methods, so `E2003` is correct there.

### The fix — one new helper, one four-line call site

1. `src/check/mod.rs` — new `fn check_param_method_call(&mut self, site:
   MethodSite<'_>, pname, name, r, args, span)` just before `resolve_method_sig`.
   It collects the `FnSig`s the parameter's bounds grant for `name`; on an empty
   set it keeps the pre-existing built-in path and emits a precise `E2003`
   naming the parameter; on a non-empty set it runs the normal visible-signature
   resolution, overload diagnostics, `check_struct_method_args`, and `mut self`
   mutability enforcement.
2. `src/check/mod.rs` — a compact `Ty::Param` arm between the `Ty::Union` arm
   and the untouched conservative fallback. The fallback and its `on any type`
   wording are byte-identical for every non-parameter receiver, which is why the
   single golden row that pins that wording (`tests/oracle/r3c1_golden.tsv:110`,
   receiver `[1]`, a `Ty::List`) still passes.

### REG-1 — a real regression found and fixed in the same change

The first version inlined the arm body into `expr_inner`. That grew its
debug-build stack frame enough that
`tests/repl.rs:230 repl_nesting_boundary_is_a_diagnostic_not_a_crash` aborted
with `signal: 6 (SIGABRT)` on the 256-deep nested-list case before the `E1015`
nesting diagnostic could fire. Baseline was clean (54/54 without the fix), so
the regression was genuinely introduced. Extracting the body into the helper
restored the frame: `cargo test --locked --test repl` → 54 passed, 0 failed.
Clippy's `too_many_arguments` (8/7, fatal under `-D warnings`) was resolved by
bundling `recv` and `ty_args` into the existing `MethodSite<'a>` struct.

### The tests — five new, two proven to fail without the fix

`tests/generics.rs` (46 tests total, all pass):
`a_bound_grants_its_methods_without_any_implementation` (one- and two-method
traits), `an_unconstrained_parameter_cannot_call_an_unrelated_struct_method`,
`a_bound_does_not_grant_methods_outside_it`,
`builtin_methods_stay_callable_on_a_type_parameter`,
`an_impl_bound_propagates_to_its_field_parameter`.
With `git stash push -- src/check/mod.rs`, two of them fail with the exact old
behaviour (`Diag { code: 2003, message: "no method \`show\` on any type" }` and
`expected the program to be rejected: "A\n"`). The pre-existing
`bounds_are_checked_at_the_call_site` passed only because it bundled an
`impl Show for A` with the generic call, registering a global `show`.

### Validation actually executed

```
cargo fmt --all -- --check                                          FMT OK
cargo clippy --locked --all-targets --all-features -- -D warnings   clean
cargo test --locked --all-targets --all-features                    66 binaries, 0 failed
cargo test --locked --all-targets --no-default-features \
  --features cli,repl,json,regex,time                               66 binaries, 0 failed
cargo +1.83.0 check --locked --all-features                         ok
cargo +nightly check --manifest-path fuzz/Cargo.toml --all-targets  ok
node playground/tests/node/run-all.mjs                              all suites pass
node playground/build.mjs --check                                   manifest matches 9 versions
node website/build.mjs                                              46 pages
node website/tests/run-all.mjs                                      pass
```

Not run locally: Miri, fuzz smoke, browser Playwright — all CI-only.

### Next exact action

Merged. Next work item is recorded in the current task section above.

---

## Previous task — CORE FIRST: f-string zero padding lands outside the sign — merged `1fa01db0` (PR #34)

**Status:** merged. Twenty-one remote checks green,
`mergeStateStatus=CLEAN`. Post-merge diff `da8d6273..1fa01db0` contains exactly
the twelve intended files.

### The defect

The `0` flag was stored only as a fill character (`out.fill = Some('0')`). No
bit distinguished "the user wrote `0` before the width" from "the user wrote an
explicit fill", and `format_value` inserted the sign (step 3) **before**
applying generic fill padding (step 4). The zeros therefore landed outside the
sign. Four coupled symptoms, one root cause each:

| # | Input | Before | After (Python) |
|---|---|---|---|
| A | `f"{-42:06d}"` | `000-42` | `-00042` |
| B | `f"{-3.5:08.2f}"` / `f"{-0.5:010.3f}"` / `f"{-0.25:09.2%}"` | `000-3.50` / `0000-0.500` / `00-25.00%` | `-0003.50` / `-00000.500` / `-0025.00%` |
| C | `f"{-42:*>06d}"` | `000-42` — the `*` fill is silently overwritten | `***-42` |
| D | spec line 400 | described the defective rule and gave no sign rule | rewritten |

Symptom B also reaches ±infinity: `1e400` overflows to `inf` at parse time, so
`f"{-1e400:08.2f}"` must render `-0000inf`.

### The fix — three code sites

1. `src/ast/mod.rs` — `FormatSpec` gains `pub zero_pad: bool` between `sign` and
   `width`. Source-compatible: the only other construction site
   (`src/run/iterative.rs:5617`) uses `..FormatSpec::default()`.
2. `src/parse/mod.rs` — `parse_format_spec` sets `zero_pad` **only** when the
   flag is bare (`out.fill.is_none()`). The parser still sets
   `fill = Some('0')` and `align = Right` unconditionally so the pre-existing
   non-numeric fill and every bare-align rendering stay bit-identical.
3. `src/run/mod.rs` — `format_value` step 4 branches on `spec.zero_pad && numeric`,
   splits the sign off by byte, and pads zeros between sign and digits.
   Everything else falls through the old path untouched.

`format_value` has **one** implementation (`src/run/mod.rs:2849`) shared by the
tree-walker and the iterative evaluator, so the fix needed no duplication.

### CPython's actual rule, established empirically

The flag is honoured **only** when no explicit `fill`+`align` pair precedes it.
A bare alignment does *not* suppress it:

```
'>06d'  -> '000-42'   '<06d'  -> '-42000'   '^06d'  -> '0-4200'
'*>06d' -> '***-42'   '*<06d' -> '-42***'   '0>6d'  -> '000-42'
'06d'   -> '-00042'   '+06d'   -> '-00042'   '0=6d'  -> '-00042'
```

`=` alignment is `E1006` in Aura — a pre-existing, documented divergence, left
alone.

### Tests

Seven `audit8_*` tests appended to `tests/regressions.rs`. **Four fail against
`da8d6273`**, proven by stashing `src/` and re-running; all seven pass after the
fix. The other three are guards that pass both ways:

| Test | Pre-fix |
|---|---|
| `audit8_zero_pad_places_the_sign_before_the_zeros` | FAILS |
| `audit8_zero_pad_is_sign_aware_for_floats` | FAILS |
| `audit8_zero_pad_handles_infinities` | FAILS |
| `audit8_explicit_fill_suppresses_the_zero_flag` | FAILS |
| `audit8_bare_alignment_keeps_the_plain_zero_fill` | passes (guard) |
| `audit8_zero_pad_leaves_the_plain_width_path_alone` | passes (guard) |
| `audit8_zero_pad_still_fills_non_numeric_values` | passes (guard) |

Six `fstring_value` oracle generators in `tests/oracle/r3b.rs` and six
regenerated rows in `tests/oracle/r3b7_golden.tsv`; three new assertions in
`tests/contract_sync.rs`; one playground differential case.

### Judgement calls surfaced rather than made silently

- **`f"{true:06}"` renders `00true`** and `f"{[1, 2]:08}"` renders `00[1, 2]`.
  Aura's zero flag was never restricted to numbers, and Python cannot arbitrate
  because its `bool` *is* an `int` (`format(True, '06')` is `'000001'`). This is
  a deliberate Aura choice, now written into the spec and locked by a test.
- **`f"{-255:08x}"` → `ffffffffffffffff01` is *not* fixed.** It is the
  `as u64` cast of the digits, and **no spec sentence governs negative integers
  in non-decimal presentation**. Unspecified surface, not a confirmed defect;
  it needs a specification decision first.
- **`e`-presentation prints `e0`, not `e+00`** (`f"{-3.5:08.2e}"` → `-03.50e0` —
  one zero, because the text is seven characters). Already normatively tested
  at `tests/contract_sync.rs:257,269,272`; unchanged.

### Validation actually executed

```
cargo fmt --all -- --check                                          FMT OK
cargo clippy --locked --all-targets --all-features -- -D warnings   clean
cargo test --locked --all-targets --all-features                    66 binaries, 0 failed
cargo test --locked --all-targets --no-default-features \
  --features cli,repl,json,regex,time                               1352 passed, 0 failed
cargo +1.83.0 check --locked --all-features                         ok
cargo +nightly check --manifest-path fuzz/Cargo.toml --all-targets  ok
node playground/tests/node/run-all.mjs                              all suites pass
node playground/build.mjs --check                                   manifest matches 9 versions
node website/build.mjs                                              46 pages
node website/tests/run-all.mjs                                      pass
```

---

## Previous task — CORE FIRST: `sort` is not a total order (one root cause, five symptoms) — merged `da8d6273` (PR #33)

**Status:** merged. Twenty-one remote checks green, `mergeStateStatus=CLEAN`.
Post-merge diff `a089fd45..da8d6273` contains exactly the eight intended files.

### The defect

`sort` was implemented as `cmp_val(..).unwrap_or(Equal)` in **two** places
(`src/stdlib/mod.rs` — the `sort` builtin and the `list.sort` method, identical
lines). `cmp_val` returns `None` for two different reasons, and mapping both to
`Equal` produces an **intransitive comparator**:

1. `NaN` — `partial_cmp` yields `None`;
2. operands of different ordering classes — `comparable_with` yields `false`.

The intransitivity surfaces five ways, three of them P1 aborts:

| # | Input | Before | Severity |
|---|---|---|---|
| A | `sort([3.0, 1.0, nan, 2.0])` | `[1.0, 3.0, nan, 2.0]` — `NaN`'s position is arbitrary | deterministic misordering |
| B | 21-element `NaN` list | `E4999` abort, `smallsort.rs:854` | P1 |
| C | 22-element `[int \| string]` list | `E4999` identical; small lists silently misorder | P1 |
| D | 21 elements mixing `int`/`float` around `2^53` | `E4999`, **no `NaN`, no type mismatch** | P1 |
| E | `[5, 1, "a", 3, 2]` | `Equal` for incomparable pairs; §12 requires `E3001` | spec violation |

Symptom D is the one that would have survived a `NaN`-only patch: `cmp_val`
orders `int` against `float` through `as f64` (§10.4's rounding promotion),
so above `2^53` distinct integers collapse onto one `double` and the order is
intransitive with nothing unordered in sight.

**The checker cannot prevent this.** `Ty::orderable_with` (`src/types.rs:545–571`)
returns `None` — "undecidable" — for `Ty::Union`, and
`src/check/mod.rs:4067–4073` performs no orderability check on `sort` at all.
An `[int | string]` list is legal to construct, so the panic is reachable from a
valid program; runtime handling is mandatory.

### Why this is a spec correction, not just a code fix

Spec 1588–1590 literally read "`sort`, `min`, and `max` … treat an unordered
pair as equal and preserve source order (a stable fallback)". That sentence is
satisfiable for the *pairwise* `min`/`max` — both are correct and were not
touched — but it **cannot** be satisfied by a *global* sort: a stable sort over
a non-total order does not terminate in a defined permutation. The normative
text was part of the defect. Lines 1592–1593 cited
`tests/regressions.rs::f05_*` as evidence, but `f05_orderability_is_checked_once`
only asserts the *static* orderability claim — nothing covered the runtime
sentence.

### The fix

- `Value::sort_cmp` (`src/run/value.rs`) — a total order where `None` means
  only "incomparable"; plus `float_sort_cmp` and `int_float_cmp` helpers.
- `sort_class` + `sorted` (`src/stdlib/mod.rs`) — validate the ordering class
  up front and raise `E3001`, so the `sort_by` closure is infallible.
- Both call sites rewired; the two defective `cmp_val` lines are gone.

Four decisions, each written into the corrected spec:

1. `NaN` sorts after every ordered number; two `NaN`s compare equal and the sort
   stays stable.
2. Mixed `int`/`float` compare **exactly**, not through the rounding promotion,
   so `sort` is deliberately **finer than `==`** above `2^53`. The comparison
   operators are unchanged and remain internally consistent but lossy there
   (`9007199254740993 == 9007199254740992.0` is `true`); the spec now states the
   divergence explicitly instead of leaving it implied.
3. `-0.0` and `0.0` compare equal (§10.3), so stability preserves their source
   order rather than imposing `total_cmp`'s `-0.0` first.
4. Elements outside every ordering class, and pairs from different classes, are
   `E3001` — always at runtime.

### Tests

Eight `audit7_*` tests in `tests/regressions.rs` (appended after 1568).
**Seven fail against `a089fd45`**, including all three `E4999` aborts reproducing
verbatim at `smallsort.rs:854`; all eight pass after the fix. The eighth,
`audit7_sort_preserves_the_existing_orderings`, is the guard — it passes both
before and after, which is exactly why it belongs in the same suite.

### Validation

Full local floor green: `cargo fmt --all -- --check`,
`cargo clippy --locked --all-targets --all-features -- -D warnings`,
`cargo test --locked --all-targets --all-features` (66 binaries),
`--no-default-features --features cli,repl,json,regex,time`,
`cargo +1.83.0 check --locked --all-features`,
`cargo +nightly check --manifest-path fuzz/Cargo.toml --all-targets`,
`node playground/tests/node/run-all.mjs`, `node playground/build.mjs --check`,
`node website/build.mjs`, `node website/tests/run-all.mjs`.
Not run locally: Miri, fuzz smoke, browser Playwright — all remote-only.

### Probes closed as NOT defects this round

Unicode `len`/indexing (character-based and consistent); range indexing
(unsupported, `E3001`); `sum` overflow (`E4013`); ±infinity sorting;
negative-zero sort order; `min`/`max` with incomparable types (returns a value
per rows 2992–2993 — making them raise would be a spec *change*);
the `2^53` comparison operators (internally consistent, merely lossy); the
diagnostic wording (the backticked `` `int` `` form comes from the checker layer,
while `src/run/mod.rs:2739–2756` uses bare `type_name()` — `sorted()`
already matches the runtime idiom).

---

## Previous task — CORE FIRST probe round: semantics sweep (no defect found)

**Status:** complete, no confirmed defect. No code change is pending from this
round. The next work item is a *new* reproduced defect.

### What was done

Thirty-plus CLI probes were run against a rebuilt `--release` binary, each in its
own fresh directory (`/tmp/probe.sh <name> <source>` creates `/tmp/pb/<name>/`).
Directory scanning for modules means one stray `.aura` file poisons every path
in that directory, so **never reuse a probe directory** — this is the single
biggest source of false findings.

Every observed behaviour matched `docs/LANGUAGE_SPEC.md`. Three apparent
anomalies were my own syntax errors, not defects:

- **Lambda syntax.** `|x| e` is not Aura. The production is `(x) -> e`,
  `(x: int) -> e`, `() -> e`, or `fn x -> e`, with an optional block body
  (spec 724, 1807–1821). `E1006: expected an expression, found \`|\`` is correct.
- **`enum`/`match` layout.** Variants and arms need newlines between them
  (spec 555). A single-line `enum E { A, B, C }` is `E1006` by design.
- **Zero-payload variants.** Spec 2389 requires `A()` with empty parentheses. A
  bare `Color::Red` in expression position is `E2003` and that is correct;
  `Color::Red()` yields `Red`. Do not "fix" this.

Two further leads were checked against the spec and are **not** defects:
`list<fn() -> int>` / `-> fn(int) -> int` are rejected with `E1006: expected a
type, found \`fn\``, and `xs.sorted()` / `xs.index_of(…)` are `E2003` because
neither is in the §24.1 method inventory.

### Probe results

| Probe | Program | Observed | Verdict |
|---|---|---|---|
| `p1` | `xs[-1]` | `30` | correct (spec 1420, 2473) |
| `p2` | `"héllo"[1]` | `é` | correct, char-based |
| `p3` | `xs[-4]` | `E4019: list index -4 out of range` | correct |
| `p4` | `m["missing"]` | `E2003: map has no key` | correct (spec 2668) |
| `p5` | `t[0]`, `t[-1]` | `1`, `3` | correct |
| `p6` | `-7%3`, `7%-3`, `-7/3`, `7/-3` | `-1`,`1`,`-2`,`-2` | correct (truncating) |
| `p7`,`p10`–`p12` | `i64::MAX+1`, `2^63`, `i64::MIN/-1`, `i64::MIN%-1` | `E4013` | correct, no Rust panic escapes |
| `p8` | `1 / 0` | `E4007` | correct |
| `p9` | `-9223372036854775808` | prints | correct |
| `p13`,`p18` | `list<fn() -> int>`, `-> fn(int) -> int` | `E1006: expected a type, found \`fn\`` | limitation, not in the type grammar |
| `p19` | `() -> { n = n + 1 }` ×2 | `2` | correct |
| `p20` | `(a:int)->a+1`, `(a,b)->a+b`, `fn a->a*2` | `2`,`5`,`8` | correct |
| `p17`,`p21`,`p28`,`p37` | mutating a non-`mut` binding | `E2001` | correct |
| `p23` | `for` binding captured | `0 1 2` | correct — fresh cell per iteration |
| `p24` | `while` captured | `3 3 3` | correct — one shared cell |
| `p25` | `-> any` | `E3002: unknown type \`any\`` | correct |
| `p26`,`p27` | `down(1000)`, infinite recursion | `E4011: call depth limit exceeded` | correct (512 frames) |
| `p29` | `"x" * 5` | `E3001` | correct |
| `p31` | map/set iteration | `a b c` / `1 2 3` | correct — sorted |
| `p32` | `\u{1F600}` | `E1003` | correct — not in the escape table |
| `p33` | trait + `<T: Shape>` | `9` | correct |
| `p34`,`p43` | `xs[1..]`, `xs[1..3]` | `E1006` / `E3001: cannot index list with range` | correct — no slicing |
| `p35`,`p38`–`p42`,`p44`,`p46` | bare variant | `E2003` | **correct — spec 2389 needs `()`** |
| `p47`,`p48` | `Color::Red()`, `f(E::C())` | `Red`, `3` | correct |
| `p50`,`p51` | generic struct + `id<T>` | `1`,`hi`,`[1, 2]` | correct |
| `p53` | `filter`/`map`/`reduce` | `[2,4]`,`[2,4,6,8,10]`,`15` | correct |
| `p52`,`p54` | `P::sum`, `p |> P::scaled(2)` | `E2003` | correct — spec 756 excludes associated fns |
| `p55` | `3 |> add(4)`, `3 |> ((x)->x*2)` | `7`, `6` | correct |
| `p56`,`p57` | `p |> p.scaled(10)`, `xs |> xs.len()` | `E3001` | correct — the pipe *prepends* the LHS |
| `p58` | `xs |> len()` | `3` | correct |
| `p60`,`p61` | `pub a, b` / cross-module private field | `1`,`2` / `E2018` | correct (spec 3320–3327) |
| `p62`,`p63` | `xs.sorted()`, `ys.index_of(2)` | `E2003: no method …` | correct — not in §24.1 |

### TD-13 is closed — do not re-open

`TECHNICAL_DEBT.md:41` already marks the REPL O(N²) row as a legitimate future
feature with no correctness impact, and PR #21 delivered the incremental path:
`src/repl.rs` uses `Checker::session()` plus
`extend_declarations(&state.decls[watermark..])` (line 429) and keeps the full
`Checker::with_declarations` rebuild only as the rollback path (line 544).
`extend_declarations` (`src/check/mod.rs:791–940`) already replicates the
`value_types` asymmetry the equivalence obstacle depended on — see its own
comment at 925–928. `tests/repl.rs::repl_incremental_state_is_not_worse_than_quadratic`
guards it.

### Open question — cyclic `pub use`

Probe `r1` from the previous round (`module a { pub use b::f } module b { pub use
a::f }`) reports `E2019` naming whichever module navigation reached first. It
terminates deterministically, satisfying the letter of §27, but the message
describes a *missing* name rather than a *cycle*. Deciding whether that warrants
a dedicated diagnostic needs a spec reading first — do not change it on
intuition alone.

---


## Previous task — CORE FIRST: `pub use` re-export resolution (three defects) — merged `cff9d4a5` (PR #28)

**Merged as PR #28 (`cff9d4a5`), CI 21/21 green on round 1.**


`docs/LANGUAGE_SPEC.md` §27 (3311–3318) states that `pub use` makes the imported
name an exported name of the current module, that a private declaration cannot be
published through a public re-export (`E2018`), and that **"a re-export chain
resolves transitively and terminates deterministically."**

`Resolver::lookup_qualified` — the expression/`::`-path resolver — honours that
chain via `follow_reexport`. The `use`-import path did not. Three independent
defects fell out of that asymmetry. Defects A and B were fixed first; validating
them exposed C, which proved the first two fixes incomplete. All three are in.

### Defect A — the `use` path bound the alias, not the declaration site

`apply_use` called `resolve_use_path` and bound the returned canonical name
directly. For

```aura
module a { pub fn f() -> int { return 1 } }
module b { pub use a::f }
use b::f
fn main() { print(f()) }
```

the binding became `b::f` — a name no item is declared under. It failed only at
first use, as `E2003` (`undefined function \`b::f\``), quoting the *alias*. The
diagnostic's quoted name is the evidence: `b::f` means the chain was not
followed; `a::f` would mean it was.

**Fix:** wrap the lookup in `follow_reexport`, exactly as `lookup_qualified`
already does. `follow_reexport` is cycle-safe (bounded by `reexports.len() + 1`,
breaks on self-reference), so the new caller inherits that safety.

**Wrong test expectation caught.** The enum-variant assertion initially expected
`b::A(2)` and failed with `left: "a::A(2)"`. Three fresh probe directories
(`use b` + `b::A(2)`, `use a::E::A` + `A(2)`, `use b::A` + `A(2)`) all print
**`a::A(2)`** — a variant's canonical identity is its declaration site (§27,
3329–3335). The assertion, not the fix, was wrong; it now expects `a::A(2)`.

### Defect B — an import could run before the re-export it names existed

`apply_imports` applied a module's own imports *before* recursing into its
children, and `src/module_graph.rs:723` pushes child `Item::Module` *after* the
parent's own items. So a root `use facade::g`, where `facade` says
`pub use inner::g`, resolved before `facade`'s `pub use` had been registered and
failed `E2019` (`` `g` is not declared in module `facade` ``).

This contradicted `collect_module`'s own doc comment (552–559), which promises
imports are applied "only after **every** module and item in the file has been
declared, so an import may name an item declared later or in a sibling module
regardless of source order." The comment was literally false.

### Defect C — a sibling-module chain was source-order dependent

Reproducer `t_rev` (only the module declaration order differs from `t_fwd`):

```aura
use m0::f
module m0 { pub use m1::f }
module m1 { pub use m2::f }
module m2 { pub fn f() -> int { return 5 } }
fn main() { print(f()) }
```

Before: `error[E2019]: \`f\` is not declared in module \`m1\``. The diagnostic
naming the **middle** hop is the evidence that navigation stopped one link short.
`t_fwd` (same modules, reversed order) printed `5`. Both now print `5`.

Two causes, both required:

1. **Missing `reexports` consult in navigation.** `resolve_use_path_inner` was
   asymmetric: its single-segment branch returned `follow_reexport(c)` —
   reexport-aware — but the multi-segment navigation loop's final-segment branch
   returned a bare `Ok(canonical)` on an `items`/`module_paths` hit, never
   consulting `self.reexports` and never following the hop. `apply_use` mirrors a
   public re-export into `items` **only** when the canonical is already present
   (855–868), so it cannot bootstrap a chain whose tail is not yet declared —
   which is why consulting `reexports` is necessary, not merely tidy. The branch
   now checks `items` **or** `reexports` and returns `follow_reexport(&canonical)`
   from either.
2. **Per-level deferral could not converge.** Defect B's fix deferred `E2019`
   imports and retried them once at the end of *that module's own recursion
   level*. But the dependency crosses sibling modules in either source order:
   `m0` re-exporting `m1::f` while `m1` re-exports `m2::f` needs `m1`'s import to
   have run, and `m1`'s own level completes *earlier* than the root's retry — so a
   module-local retry waits for work that has already finished and cannot make
   progress. Deferrals now bubble to a single driver at the top of
   `apply_imports`, which iterates a **bounded fixpoint**: each round must settle
   at least one pending import for progress to be real, so `pending.len() + 1`
   rounds bound the search; a genuinely unknown name survives them all and still
   reports `E2019` with correct provenance.

### Why the fix is safe

- **Deferring cannot corrupt state.** All five `UNKNOWN_MODULE` raise sites are
  strictly *pre-mutation* (`apply_use` 773–782, `resolve_use_path_inner`
  1050/1102/1121/1128), so a deferred import leaves no partial binding, cannot
  double-push `applied_imports`, and cannot insert into
  `type_scope`/`value_scope`/`variant_scope`.
- **`activate_item` runs again before every attempt**, including each retry.
  `resolve_sourced` wraps a `collect_module` failure with `attach_source`, which
  reads `active_source`. Recursing into a child leaves it pointing at the child's
  last item, so without re-activation a retried root error would be attributed to
  the wrong file — breaking `resolver_error_in_child_keeps_child_provenance`
  (`tests/module_graph.rs:384–405`), which still passes.
- **No new `Resolver` field**, so `reset_transient` and the REPL path are
  untouched. The fixpoint state is all function-local.
- **The `pub use` `E2018` check stays hoisted** above the first mutation (done for
  defect A/B), so the newly reachable navigation paths cannot leave a binding
  behind on rejection.
- **Cycles terminate.** `follow_reexport` breaks on self-reference, so a cyclic
  `pub use` (probe `r1`) reports `E2019` deterministically instead of looping.

### Regression coverage added

`tests/modules.rs` (in-source modules, no filesystem fixtures):

- `use_through_a_re_exporting_module_binds_the_real_target` — re-exported `fn`,
  argument-bearing `fn` with signature preserved, two-hop chain `c → b → a`,
  re-exported `struct` used as an annotation, re-exported enum variant printing
  `a::A(2)`, rename through a re-export (`use b::f as g`), and a
  direct-declaration positive control.
- `use_through_a_re_export_keeps_the_private_target_rejected` — `E2018` negative
  control, proving the new `follow_reexport` call cannot launder visibility.
- `a_transitive_re_export_chain_resolves_in_every_source_order` — the three-module
  sibling chain in **both** declaration orders, an 8-hop chain, a mixed chain
  whose middle hop also declares its own item, a cycle, a genuinely missing name,
  and a private tail.

`tests/module_graph.rs`:

- `use_through_an_external_re_export_resolves_regardless_of_source_order` — runs
  the same logical graph in both registration orders, which is what pins defect B
  (order-independence) separately from defect A.
- `a_transitive_re_export_across_sibling_sources_resolves_in_every_order` — the
  three-source sibling chain in both registration orders, plus a missing name
  across files still reporting `E2019`. This is what pins defect C.

**Both new tests were proved to fail against the pre-fix `src/resolve.rs`** by
copying the fixed file aside, `git stash push -- src/resolve.rs`, re-running, then
restoring. That is the only evidence that a regression test actually guards
anything, so do not skip it when extending this work.

### Validation actually run

| Command | Result |
|---|---|
| `cargo test --locked --test modules` | 23 passed, 0 failed |
| `cargo test --locked --test module_graph` | 40 passed, 0 failed |
| `cargo test --locked --test checker` | 59 passed, 0 failed |
| `cargo test --locked --test keystone_types` | 48 passed, 0 failed |
| `cargo test --locked --test source_provenance` | 14 passed, 0 failed |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --locked --all-targets --all-features -- -D warnings` | clean |
| `cargo test --locked --all-targets --all-features` | all green (40 targets) |
| `cargo test --locked --all-targets --no-default-features --features cli,repl,json,regex,time` | all green |
| `RUSTFLAGS="-D warnings" cargo check --locked --lib --no-default-features --features cli` | clean |
| `cargo +1.83.0 check --locked --all-features` | clean |
| `cargo +nightly check --manifest-path fuzz/Cargo.toml --all-targets` | clean |
| `node playground/tests/node/run-all.mjs` | all green (`http-security` 26/26) |
| `node playground/build.mjs --check` | manifest matches 9 version(s) |
| `node website/build.mjs` | 46 pages |
| `node website/tests/run-all.mjs` | all green |

Thirteen CLI probes in fresh `/tmp/tc*` directories confirmed, with each probe in
its own directory (directory scanning for modules means one stray `.aura` file
poisons every path there — never reuse a probe directory):

| Probe | Expected | Observed |
|---|---|---|
| `t_rev` sibling chain, declaration order | `5` | `5` (was `E2019` on `m1`) |
| `t_fwd` same, reversed | `5` | `5` |
| `s_rev` / `s_fwd` nested-module variants | `5` | `5` |
| `s_nest` doubly nested | `6` | `6` |
| `r3` 8-hop chain, reverse order | `77` | `77` (was `E2019`) |
| `diamond` two paths to one `fn` | `9` | `9` |
| `mixed` middle hop also declares its own item | `7`, `8` | `7`, `8` |
| `r1` cyclic `pub use` | terminates | `E2019` |
| `r2` self re-export in the `fn`'s own module | rejected | `E2007` |
| `modpath` final segment is a real private module | rejected | `E2018` |
| `private_tail` chain ending on a private item | rejected | `E2018` |
| `missing` genuinely absent deep name | rejected | `E2019` |

### No spec change

The code violated §27 and `collect_module`'s own doc comment; the fix satisfies
both. No normative text was amended.

### Open question — cyclic `pub use`

Probe `r1` (`module a { pub use b::f } module b { pub use a::f }`) reports
`E2019` naming whichever module navigation reached first. It terminates
deterministically, so it satisfies the letter of §27, but the message describes a
*missing* name rather than a *cycle*. Deciding whether that warrants a dedicated
diagnostic needs a spec reading first — do not change it on intuition alone.

---

## Previous task — CORE FIRST: return-type tail validation — merged

**Merged as PR #26 (`b843dc85`), CI 21/21 green on round 1.** No red gate, no
retry, no gate weakened. `mergeStateStatus` was `CLEAN` and `reviewDecision`
empty, so no external human approval was required or simulated.

### The finding — the tail position was never validated

A body's implicit value is the value of its last statement (§15.1), and §6.4
already normatively defined that value's *type*, but the checker never compared
it with the declared return type. Three defects shared the root cause, plus one
adjacent gap:

```aura
fn f() -> int { "s" }          // accepted; a release build printed `s` for
                               // `let x: int = f()`
fn f() -> int { return }       // accepted; the `return` arm only inspected Some(v)
fn f() -> [int; 3] { [1, 2] }  // accepted; the tail never consulted
                               // `check_array_literal`, so arity went unchecked
```

### The fix

- A `tail_expect: Option<Ty>` field, set by both body checkers (the free
  function and `check_method_body`) and taken at the top of `block_with`.
- **A field is required, not a post-`block` call.** `finish_scope` pops the body
  scope, so `fn f() -> int { let x = "s"\nx }` would resolve `x` to `Unknown`
  and pass silently. The expectation is handed to the body's own `block_with`,
  which judges the tail while the scope is still live. Taking it right after
  `push()` also stops nested blocks from re-consuming an enclosing expectation,
  and the mechanism needs no call-site change.
- A new `block_tail_mismatch(&self, body, expected) -> Result<Option<Diag>>`
  skips a non-value tail (a body ending in a declaration, loop, `return`, or
  nothing), applies contextual Array realization in tail position (§21.2), and
  reports `E3005` on a real mismatch. It is `&self` and returns the diagnostic,
  so `block_with` stays the single place that decides scope teardown.
- **`-> never` is excluded at the assignment site,** not inside the helper:
  `fn bad() -> never { print("oops") }` must stay `E3006`, and a naive tail check
  would emit `E3005` for `fn bad() -> never { "s" }` because `print` infers
  `Unknown` and `return_compatible` accepts `Unknown`. A bare `return` under
  `-> never` likewise keeps `E3006`, while `return 1` under `-> never` still
  yields `E3005` as `never_is_the_bottom_type` requires.

### Defect found in the diff before shipping

The first version used `?` on `block_tail_mismatch`. That helper can propagate an
`Err` from `check_array_literal` — arity and element-type failures return `Err`,
not `Ok(false)` — so the `?` returned without calling `self.pop()` and leaked a
scope frame on every rejected Array tail literal. The final code matches on the
`Result` and pops in both non-`Ok(None)` arms. **The tail check runs after the
statement loop's own error handling, so its failures must redo the pop
explicitly.**

### Contract judgement — stated, not silently resolved

`AGENTS.md` says "Do not silently resolve a genuine specification choice." This
is not one. §15.1 makes "a body's value is its last statement" normative, §6.4
normatively defines the tail's value type, and §15.2 constrained only `return`.
§6.2 enumerated six enforced positions and simply omitted the tail expression:
a **documentation-completeness gap**, and the deviation produced unsound
programs. `docs/LANGUAGE_SPEC.md` is updated in three coordinated places (§6.2
bullet, two §15.2 normative rules, §21.2 contextual-Array list) so the contract
and the checker agree.

### Guards and evidence

- `tests/checker.rs` — four tests: mismatch, accepted controls, Array
  realization, bare `return` under an annotation.
- `tests/methods.rs::method_return_typing` — three appended cases covering the
  `check_method_body` half: a bad tail, a bare `return` under `-> int`, and a
  positive `self.x` tail.
- Red before the fix: `cargo test --locked --test checker` → `56 passed; 3
  failed`, all three reporting `left: Ok(())`.
- 47 behavioural probes against a rebuilt release binary, one fresh directory
  each. Newly rejected: a mismatched tail and `{ true }`; `string` from `1`;
  `if`/`else` joins (`int | string`); struct, tuple, method, and call-through
  mismatches; a body-local binding in tail position (annotated and inferred); a
  generic `-> T` with a concrete tail and with a concrete `return`; `fn main() ->
  int`; `-> none` with a value; an aliased `Age`; `int | none` with a value; and
  Array arity/element failures in tail position.
- Still accepted: matching tails, `print(...)` tails (`Unknown` stays permissive
  per §2.3), empty bodies, `-> int | none` with an empty body, `-> never` with
  `throw`, `for`/`loop` tails, matching arrays/unions/tuples, and generic and
  alias bodies that genuinely match.
- Full local floor, each command with its result in the PR body: `fmt`,
  `clippy -D warnings`, `test --all-targets --all-features`,
  `test --no-default-features --features cli,repl,json,regex,time`,
  `RUSTFLAGS="-D warnings" check --lib --no-default-features --features cli`,
  `+1.83.0 check --all-features`, `+nightly` fuzz-target check, playground suite
  and build check, website build and suite.
- Remote CI on the PR → **21/21 pass**, round 1.

### Resume point

HEAD is `b843dc85`. **Exact next action:** continue CORE FIRST with the next
reproduced defect — lexer/parser ambiguities, precedence and error recovery;
checker scopes, generics and mutability; evaluator control flow, closures and
resource limits; collection bounds; module cycles; runtime panic and
Host/Python/WASM capability boundaries. One PR per finding, each carrying a
reproducible case, a regression test that fails before the fix, the fix, and a
passing test. Keep the no-`repl` lib check in the floor. Do not begin language
features frozen for stabilization. B-1R8 and any runtime publication remain
separately human-gated.

**Open residual lead:** lambda-tail immunisation is proven by `Expr::Lambda`
taking `return_type` via `mem::take`, not by a live probe — both probe attempts
hit an unrelated `E2008` unused-parameter diagnostic first. A regression test
using a used or `_`-prefixed parameter would close that gap.

## Previous task — CORE FIRST: set key-method element typing — merged

**Merged as PR #24 (`02c3c897`), CI 21/21 green on round 1.** No red gate, no
retry, no gate weakened. `mergeStateStatus` was `CLEAN` and `reviewDecision`
empty, so no external human approval was required or simulated.

### The finding — the whole set key-method surface was unchecked

`TypeClass::Set` had **no element branch at all** in `Checker::check_method_call`.
Every set key method was therefore accepted with an argument of any type, while
the structurally identical map key methods were correctly rejected with `E3001`.
`List.push` was checked correctly, so the asymmetry — not a missing feature —
was the defect.

```aura
let mut s = set{1}
s.add("x")      // accepted; ran to completion producing {1, "x"}
s.has("x")      // accepted; silently returned false, no diagnostic
s.remove(1.5)   // accepted; failed only at runtime
```

`E3001` appeared only later and downstream, where the user no longer sees the
cause. Reproduced with an explicit `{int}` annotation, a declared return type,
an `{int}` parameter, and a receiver returned from an `{int}`-returning
function — all four accepted the foreign value.

Root cause: `Set.add` (`src/stdlib/signatures.rs:1205`) and `Set.remove`
(`:1214`) are registered with `params: vec![Param::ANY]` and
`returns: Ty::Unknown`, and `has`/`contains` likewise. The coarse registry
cannot express "the same type the receiver holds", so the checker has to.

### The fix

- `check_element_assignable` gained a `container: &str` noun so the diagnostic
  can name list, array, or set. Its four pre-existing call sites pass
  `"list"`/`"list"`/`"list"`/`"array"`; their messages are unchanged byte for
  byte, verified by the pre-existing list/array tests still passing.
- Two branches added in `check_method_call`, after the list-`push` branch and
  before the generic parameter loop: `Set` + `add`, and `Set` +
  `has`/`contains`/`remove`. Both reject a non-key-capable argument **first**,
  so the diagnostic is correct even when the receiver's element type is
  `Unknown`, then check agreement against the proven element type.
- **`Ty::key_type_error()` was deliberately not reused.** It says "map key", so
  reusing it for sets would make the message wrong for one of the two
  containers. The new branches emit set-specific wording matching the message
  the set-literal path already produces.

### Guards

`tests/collections.rs`:

- `typed_set_mutation_obeys_the_element_type` — annotated, inferred and
  declared-return receivers reject a foreign element; same-type and union
  receivers still accept; `add(1.5)` and `add([1])` reject.
- `typed_set_key_methods_obey_the_element_type` — `has`/`contains`/`remove`
  reject both a foreign element type and a non-key-capable type across all
  three methods; same-type queries still evaluate; an untyped `set{}` receiver
  produces no false rejection.

Both were written first and failed with `expected the program to be rejected:
""`. `tests/keystone_collections.rs:182` (`{1, 2}.has(2)`) is the only
pre-existing set-method assertion and still passes.

### Documentation gap closed

`docs/LANGUAGE_SPEC.md` had **no `set` method table at all** — the roster
existed only in code. §24.1 now has one beside the string/list/map tables, and
§21.4 states the check-time typing contract normatively.

### Deliberately not treated as defects

`list.contains`, `array.contains`, `tuple.contains` and `range.contains` also
take `Param::ANY` and return `false` for a foreign argument. That is the
documented contract (spec §24.1 "1 any") and it cannot corrupt state, so it was
left alone. `reduce`'s `Param::ANY` accumulator *is* checked in practice,
because overload resolution rejects a string initializer against the numeric
`+` in the body.

### Validation executed on this tree

- `cargo test --locked --test collections` → 10 passed, 0 failed
- `cargo test --locked --all-targets --all-features` → 66 suites ok, 0 failed
- `cargo fmt --all -- --check` → clean
- `cargo clippy --locked --all-targets --all-features -- -D warnings` → clean
- `cargo test --locked --all-targets --no-default-features --features cli,repl,json,regex,time` → all ok
- `RUSTFLAGS="-D warnings" cargo check --locked --lib --no-default-features --features cli` → clean
- `cargo +1.83.0 check --locked --all-features` → clean
- `node playground/tests/node/run-all.mjs` → complete; `node playground/build.mjs --check` → manifest matches 9 version(s)
- `node website/tests/run-all.mjs` → complete
- live reproducers with a rebuilt `target/release/aura`, each in its own fresh directory → the four new `E3001` messages, and the positive paths unchanged
- remote CI on the PR → **21/21 pass**, round 1

### Resume point

HEAD is `02c3c897`. **Exact next action:** continue CORE FIRST with the next
reproduced defect — lexer/parser ambiguities, precedence and error recovery;
checker scopes, generics and mutability; evaluator control flow, closures and
resource limits; collection bounds; module cycles; runtime panic and
Host/Python/WASM capability boundaries. One PR per finding, each carrying a
reproducible case, a regression test that fails before the fix, the fix, and a
passing test. Keep the no-`repl` lib check in the floor. Do not begin language
features frozen for stabilization. B-1R8 and any runtime publication remain
separately human-gated.

### Already probed and conformant — do not re-investigate

- HOF arity and argument order: `reduce` is `(list, fn, init)`, not
  `(list, init, fn)`.
- `zip` truncates to the shorter input, as spec §24.1 states.
- `map`, `filter`, `enumerate` behave as documented.
- Closures capture by reference with per-call cells; a factory produces
  independent counters.
- `regex_*` rejects malformed patterns with `E3001` plus the parse error, and
  matches in linear time on adversarial input (the `regex` crate does not
  backtrack).
- `contains` with a foreign argument on list/array/tuple/range returns `false`
  as documented.

## Previous task — CORE FIRST: resolver type-parameter leak — merged

**Merged as PR #22 (`92f5cf18`), CI 21/21 green on round 1.** No red gate, no
retry, no gate weakened. `mergeStateStatus` was `CLEAN` and `reviewDecision`
empty, so no external human approval was required or simulated.

`rewrite/v3-rust` is the **default branch** and HEAD is **`b843dc85`** (the
merge of PR #26). Twenty-five PRs are merged this campaign: #2, #3, #4, #5, #6, #7,
#8, #9 (B14), #10 (PR template), #11 (TD-24 cost guard), #12 (CHANGELOG
`[Unreleased]` + TD-25), #13 (website suite gate + TD-26), #14 (TD-27 Playwright
gate), #15 (state sync), #16 (B7 project-header contract + HTTP transport docs +
`test:abi`), #17 (validation orchestrator self-tests in CI), #18 (state sync for
#15–#17), #19 (browser consent scoped to the mutation class), #20 (Directive
004: the seven agents), #21 (CORE FIRST: REPL session state incremental, TD-13
closed, session APIs feature-gated), #22 (CORE FIRST: resolver type-parameter
frame popped on the error path), #23 (state sync), #24 (CORE FIRST: set
key-method element typing).
The Directive 003-R authorization recorded in
`AGENTS.md` permits push, PR creation and merge, workflow administration, and
default-branch changes; force-push, history removal, tag moves, frozen-WASM
replacement, gate weakening, and unapproved stable releases remain forbidden.
Never report a gate as green unless it actually executed.

### The finding — a rejected declaration poisons the session

`Resolver::rewrite_item` and `Resolver::rewrite_member` push a generic-parameter
frame before rewriting a declaration's body and pop it afterwards, with the pop
written as a plain statement at the end of the arm. Every arm has fallible work
between the two: `?` on `rewrite_params`, `rewrite_type`, `rewrite_block`,
`rewrite_field`, `rewrite_variant`, `rewrite_member`, `canonical_type`. When one
of those returns `Err`, the `?` leaves the function before the `pop` runs, so the
frame — and every generic name in it — stays on `type_param_scope` for the rest
of the session.

That stack is load-bearing for path canonicalization. `flatten` calls
`rewrite_type`, which asks `is_type_param(name)` first; a `true` answer means
"compile-time placeholder, do not canonicalize". A leaked name therefore stops a
real, declared type from being canonicalized, and the reference reaches the
checker as a bare unknown type. The user sees an unknown-type error on a line
that is perfectly valid, and the diagnostic for the *actual* mistake is the only
correct one in the output.

Reproducer (`/tmp/v1/min1.txt`, one line per submission; the `impl` targets a
private struct so its rewrite fails with `E2018` after `Q` was pushed):

```aura
module m { pub struct Q { v: int }
struct Priv { v: int } }
impl<Q> m::Priv { fn get(self) -> int { return self.v } }
use m::Q
fn takes(x: Q) -> int { return x.v }
:quit
```

Before the fix the final submission reported `E3002: unknown type `Q``; after it,
the correct `E2018: field `v` of `m::Q` is private; it cannot be read from this
module`. Five controls (no `impl`; `impl` alone; `impl` with a non-colliding
parameter name; a `struct`/`enum`/`alias`/`trait` with a failing body and the
same colliding name) all produced the correct diagnostic both before and after,
which is what pins the cause to the leaked frame rather than to the generic
syntax in general.

### The fix

An RAII guard, `TypeParamFrame<'a>`, holds `&'a Resolver` and pops on `Drop`; each
of the seven push sites binds `let _frame = TypeParamFrame::new(self);`
immediately after `push_type_params`. The pop is then unconditional, including on
the error path, and `let _` keeps the binding alive to end of scope without
triggering `unused_variables`.

The guard borrows the resolver immutably, which is sufficient because
`type_param_scope` is a `RefCell`: `push`, `pop` and `is_type_param` take `&self`
and borrow only for the duration of the call, so no `RefCell` borrow is held
across the rewritten work.

Deliberately **not** the fix: clearing `type_param_scope` in `reset_transient`.
That is what the checker does for its own stack, and it hides a per-declaration
invariant violation behind a per-submission reset — the stack would still be
wrong for the remainder of the submission that leaked it, and any future caller
that resolves more than one declaration per reset would inherit the bug. The
guard fixes the invariant instead of masking it.

### The checker's identical shape — verified safe, do not re-litigate

`src/check/mod.rs` has the same unguarded push/pop pattern at nine sites
(`hoist` struct/enum/alias/fn, trait and impl registration, the body-phase `fn`
and its nested block). It is **not** a bug and is **not** touched here, for three
independent reasons:

1. `Checker::check` (1067–1074) is strictly fail-fast: `for … { self.item(item)?; }`
   returns on the first `Err`, so a leak can never be observed by a later item in
   the same file.
2. There is no multi-diagnostic mode anywhere in the crate — no `Vec<Diag>`, no
   `errors.push`, no `collect_diagnostics` in `src/check/mod.rs`, `src/lib.rs` or
   `src/main.rs`. The three catch sites that look like they continue
   (`block_with` 5565–5581, the `for` loop 5961–5968, `try` 5986–5990) each
   `return Err(e)` immediately after popping.
3. `Checker::reset_transient` clears `current_type_params` (1046–1047) and the
   REPL calls it before every submission (`src/repl.rs:341-342`, `455-456`), so
   the stack is empty when each diagnostic is produced.

`Resolver::reset_transient` (367–373) clears only `applied_imports`,
`item_sources`, `active_source` and `resolved_sources` — that asymmetry, not a
difference in the push/pop code, is why the resolver leak is observable and the
checker one is not. An earlier session concluded the checker leaked too; that was
reproduced in a directory already holding a stray `.aura` fixture and was wrong.
Note the fixture hazard: `compile_file_with_mode` discovers module sources by
scanning the target file's directory, so one stray `.aura` file makes every
`aura check` in that directory report *its* error regardless of the path given.
One fresh directory per probe file.

### Guards

`tests/repl.rs::a_failed_generic_declaration_does_not_leak_its_type_parameters`
pins the behaviour: it asserts the `E2018` for the private struct is present, that
`E3002` is absent, and that the private-field diagnostic is the one reported. It
fails on the pre-fix tree at the `E3002` assertion.

### Validation executed on this tree

Executed locally on `2d7f6356` before the merge; remote CI then re-ran the same
surfaces.

- `cargo test --locked --test repl` → 54 passed, 0 failed (53 before the new test)
- `cargo test --locked --all-targets --all-features` → 66 suites pass, 0 failed
- `cargo test --locked --all-targets --no-default-features --features cli,repl,json,regex,time` → 66 suites pass, 0 failed
- `cargo clippy --locked --all-targets --all-features -- -D warnings` → clean
- `cargo fmt --all -- --check` → clean
- `RUSTFLAGS="-D warnings" cargo check --locked --lib --no-default-features --features cli` → clean
- the `/tmp/v1` reproducer and its controls, each in its own fresh directory → correct `E2018` everywhere, no `E3002`
- remote CI on the PR → **21/21 pass**, round 1

### Resume point

HEAD is `92f5cf18`. **Exact next action:** continue CORE FIRST with the next
reproduced defect — the `Stmt::Let` partial-mutation leak test, then
lexer/parser ambiguities, precedence and error recovery; checker
scopes, generics and mutability; evaluator control flow, closures and resource
limits; collection bounds; module cycles; runtime panic and Host/Python/WASM
capability boundaries. One PR per finding, each carrying a reproducible case, a
regression test that fails before the fix, the fix, and a passing test. Keep the
no-`repl` lib check in the floor. Do not begin language features frozen for
stabilization. B-1R8 and any runtime publication remain separately human-gated.

## Previous task — CORE FIRST: REPL session linearization (TD-13) — merged

Historical record of PR #21. HEAD was `2fe944fb` at the time.

### The finding

The ADENDO "CORE FIRST" made the directive-authorized REPL O(N²) defect the
first core priority. It was reproduced, quantified, localized, fixed, and
pinned. Branch `perf/repl-linear-session` off `38f5fed5`, merged as **PR #21
(`2fe944fb`), CI 21/21 green**.

**Symptom.** Release profile, min-of-3, distinct declarations: the `let` path at
N=250/500/1000/2000/4000 cost 0.174/0.337/1.174/4.548/**18.299 s**; the `fn`
path at N=4000 cost **98.270 s**. Cost scaled with the number of **distinct**
declarations — shadowing the same name stayed linear — and a marginal-cost probe
(extra `1+1` submissions over a fixed declaration count) showed a **constant
~658 µs per submission at every depth**, which is a fixed per-submission cost
multiplied by session depth.

**Root cause.** `SessionEngine::eval_line` called `Checker::with_declarations`
on every submission, rebuilding the whole checker from every prior declaration,
so submission *k* did O(k) work.

### The fix — three layers plus two follow-on narrowings

1. **Incremental checker.** `SessionEngine` owns one `Checker` for the session;
   `reset_transient()` clears per-submission frames and
   `extend_declarations(&decls[applied..])` registers only the new declarations.
   The module path needs no `extend_declarations` call — `check_repl_mode` →
   `check` → `hoist` + annotation pass + `item()` already register every table.
2. **`abort_submission` (found by the oracle).** With a cached checker a failing
   submission leaves partial mutations. `fn k(int)` twice then `k(1)` reported
   `E3001 ... 2 overloads accept` instead of `1`, because `hoist` pushed the
   duplicate signature before the duplicate scan rejected it. The helper rebuilds
   the checker from the session on the three paths that can fail *after*
   mutating: stmt-path `check_stmt`, module-path `check_repl_mode`, and
   module-path execution. Resolve failures are deliberately not hooked — nothing
   has mutated yet — and neither are the `executed == false` stmt arms, because
   the persistence block that follows must still record the user's binding.
3. **`session_value_names` symmetry.** `hoist` never wrote the table that
   `extend_declarations` maintains, so the cross-kind clash checks in the
   `Item::Fn` and `Item::Const` arms silently stopped firing across submissions:
   `const X = 1` then `const X = 2` went from `E2007`+`1` to `2`, and `fn X` then
   `const X` from `E2007`+`<fn>` to `2`. Both writes added.
4. **Q1 — duplicate-scan narrowing.** The overload duplicate scan re-examined
   every name in `self.functions` on every submission, but a duplicate can only
   be created by a signature pushed during the *current* call: `self.functions`
   has exactly three write sites and `add_global` pushes only into an empty set.
   The scan now drains a `rescan_functions: HashSet<String>` and scans only those
   names, leaving the comparator, the nested loop, the `active_source` side
   effect and the `E2007` diagnostic byte-identical.
5. **Q2 — persisted resolver.** `Resolver` now lives in `SessionState` instead of
   being rebuilt per submission. `extend_session` seeds items and imports past
   `seeded_items`/`seeded_imports` watermarks held on the resolver, which makes
   the move idempotent by construction; `reset_transient` clears only
   `applied_imports`, `item_sources`, `active_source`, `resolved_sources`.
   Imports must **never** be re-applied — `apply_use`'s `name_visible_locally`
   check would raise a spurious `E2007` — and the watermarks must **not** be
   reset or the quadratic cost returns. Every other resolver table is
   monotone/append-only and safe to persist.

### Measured result

| Build | `let` N=4000 | `fn` N=4000 |
|---|---|---|
| pre-fix | 18.299 s | 98.270 s (and 6.621 s from a mid-campaign baseline) |
| after Q1 | — | 2.734 s |
| **after Q2 (shipped)** | **0.238 s** | **0.411 s** |

`fn` path N=4000: **6.621 → 0.411 s = 16.1×**, doubling ratio 3.88× → 2.47×;
against the pre-fix binary **98.270 → 0.411 s = 239×**. The `let` path at N=4000
is **93.5×** faster than its pre-fix baseline. Shape to N=16000 for `fn`:
0.080/0.148/0.295/0.763/2.008 s (1.86/1.99/2.59/2.63×) — linear modulo cut-off
effects.

### Method notes worth keeping

- **`sample`-based profiling does not work in this harness.** It attaches to the
  shell wrapper, not the `aura` child, and yields `???`/`start`/`__wait4`
  frames. In-source phase instrumentation behind an env var (12 `AtomicU64`
  counters) is the reliable substitute; it attributed the module path at N=2000
  to **check 56% / resolve 43%**, with exec/decl/items ~0%.
- **Ablate data, never control flow.** An early `AURA_SKIP_SCAN` used
  `return Ok(())` inside an inner block, which returns from the entire enclosing
  function and disables everything after it; the measurement was confounded and
  had to be re-run by substituting `Vec::new()` for the block's input instead.
- **The combined ablation is what proved the model.** Neither Q1 nor Q2 alone
  linearizes the `fn` path (each ablation alone left it quadratic); ablating both
  together gave 0.044/0.078/0.149/0.313 s, i.e. linear — the two contributors are
  additive and their sum is necessary *and* sufficient. An earlier
  "no dominant driver" reading was overturned by exactly that experiment.
- **The `aura` binary needs an explicit `repl` subcommand.** Piping stdin to
  `target/release/aura` with no arguments prints the usage banner and exits 0 in
  ~13 ms, so a naive timing probe silently reports meaningless numbers at every
  depth.

### Guards

- `tests/repl_cost.rs` (new, untracked until this commit): a counting global
  allocator asserts a submission's allocation volume does not grow with session
  depth. It measured **4.19× for 4× depth** before the fix and passes now. It
  *cannot* see CPU-only costs, which is precisely why Q1 and Q2 are guarded by
  wall clock in `tests/bench.rs` instead.
- `tests/bench.rs`: `repl_incremental_state_is_linear_in_session_depth` —
  rewritten and renamed from the old weaker "no worse than quadratic" TD-13
  guard (base 800, shared 3.2× limit; the quadratic path measured 3.33–3.51
  there), and `repl_function_declarations_are_linear_in_session_depth` — new,
  covering the module path that Q1 and Q2 actually change (base 800; the
  rescanning path measured 4.18× at that base, the incremental path ~1.9).
- `tests/repl.rs`: two new tests pin the cross-kind `E2007`s
  (`session_const_after_same_named_function_is_rejected`,
  `session_function_after_same_named_const_is_rejected`). All three such tests,
  including the pre-existing `const_persists_and_cannot_be_redefined`, were
  proven to fail without layer 3.

### Equivalence evidence

A 25-case REPL oracle (stdout + diagnostics per submission) and a 6-case
session-names oracle both report **zero divergence** against the pre-fix binary —
checked after each layer, after Q1, after Q2, and again after the `SessionState`
refactor. `SessionState` groups `decls`/`checker`/`applied`/`resolver` so
`abort_submission` fits clippy's 7-argument limit honestly, not via
`#[allow(clippy::too_many_arguments)]`.

### Validation actually executed on the shipped tree

- `cargo fmt --all -- --check` → clean
- `cargo clippy --locked --all-targets --all-features -- -D warnings` → clean
- `cargo test --locked --all-targets --all-features` → 66 suites pass, 0 failed
- `cargo test --locked --all-targets --no-default-features --features cli,repl,json,regex,time` → 66 suites pass, 0 failed
- `cargo test --locked --test repl --test repl_cost --test bench --features repl` → 53 / 3 / 15 passed, 2 ignored
- `cargo +1.83.0 check --locked --all-features` → clean
- `RUSTFLAGS="-D warnings" cargo check --locked --lib --no-default-features --features cli` → clean (plus `cli,json,regex,time`, `http-api`, `cli,evaluator-oracle`, `json,regex,time`, `py`: all clean)
- `cargo +nightly check --manifest-path fuzz/Cargo.toml --all-targets` → clean
- `cargo clippy --manifest-path playground/runtime/Cargo.toml --all-targets -- -D warnings` → clean
- `cargo build --manifest-path playground/runtime/Cargo.toml --release --target wasm32-unknown-unknown` → clean
- `node playground/tests/node/run-all.mjs` → all suites 0 failed
- `RUSTDOCFLAGS="-D warnings" cargo test --doc` → 0 tests, clean (both feature sets)
- both oracles → 25/25 and 6/6 identical against `/tmp/p_before.json` and `/tmp/sn_before2.json`

Not run locally: miri (deferred to remote CI, which runs it green), browser
suites. Neither touches this change.

### Full validation floor — permanent lesson from PR #21

The first push of this branch was **5-failed / 16-passed** on remote CI, and every
failure reduced to one real defect in the change: `Checker::session`,
`Checker::reset_transient`, `Resolver::session` and `Resolver::reset_transient`
are `pub(crate)` and reachable only from `src/repl.rs`, which `src/lib.rs:30`
gates with `#[cfg(feature = "repl")]`. In any build **without** `repl` they are
dead code, and CI's `RUSTFLAGS: -D warnings` makes that a hard build error.

**Why the local floor missed it:** every command in it enabled `repl` —
`--all-features` does, and so did the "CI feature set"
`cli,repl,json,regex,time`. Nothing in the matrix ever built the library without
that feature. Jobs that do, and therefore caught it: `miri`
(`--lib --no-default-features --features cli`), `language contract`
(`cargo run --no-default-features --features cli -- version`), `fuzz smoke`,
`playground (wasm runtime)`'s "Lint the runtime crate", and the `website` job's
`playground/tests/node/run-all.mjs` step.

**The rule:** a `pub(crate)` helper reachable only from a feature-gated module is
dead code in every other feature set, and `--all-features` validation cannot see
it. The floor now includes

```
RUSTFLAGS="-D warnings" cargo check --locked --lib --no-default-features --features cli
```

Use `--lib`, **not** `--all-targets`: `tests/cli.rs` uses `serde_json`
unconditionally (lines 795 and 916) and fails with 24 `E0433` errors under
`--all-targets --no-default-features --features cli`. That breakage is
**pre-existing** — verified with `git show --name-only 404980e5`, which does not
list `tests/cli.rs` — and CI never builds that combination. Recorded as a known
gap, not a regression of this PR.

The alternative fixes were rejected deliberately: making the four methods `pub`
widens the public API to silence a lint, and `#[allow(dead_code)]` weakens a
warning to buy green. Both are forbidden by the PR template and Directive 003-R.
Gating with `#[cfg(feature = "repl")]` is the honest statement of the fact that
these methods only exist for the session lifecycle. One consequence: the
intra-doc link `[`Checker::reset_transient`]` inside `Checker::session` had to
become a plain code span, because a link target that is feature-gated away would
otherwise fail `rustdoc::broken-intra-doc-links` under `-D warnings`.

Also worth knowing: `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --lib` reports
**8 pre-existing** broken intra-doc links, all in `src/run/` (`RunSession`,
`session_impl::Session`, private `Interp` methods). CI never runs `cargo doc`, and
none of those files are touched here, but do not mistake them for a new failure.

### Resume point

PR #21 is **merged** (`2fe944fb`, CI 21/21 green on round 2). The
`#[cfg(feature = "repl")]` gating commit is `3a42deee`; the performance work is
`404980e5`. Both state documents are reconciled to the merge SHA and the writer
is released (`NONE`).

**Exact next action:** continue CORE FIRST with the next reproduced defect. The
smallest open item is the `Stmt::Let` partial-mutation leak test; then
lexer/parser ambiguities, precedence and error recovery; checker scopes, generics
and mutability; evaluator control flow, closures and resource limits; collection
bounds; module cycles; runtime panic and Host/Python/WASM capability boundaries.
One PR per finding, each carrying a reproducible case, a regression test that
fails before the fix, the fix, and a passing test. Keep the no-`repl` lib check
in the floor. Do not begin language features frozen for stabilization. B-1R8 and
any runtime publication remain separately human-gated.

### Deliberate non-fixes

- The O(Σ set²) → O(Σ set) keyed duplicate-scan rewrite is deferred: `Ty` derives
  `Eq` but not `Hash`, so a key must be `(usize, Vec<Option<Ty>>)` compared with
  `Eq`.

## Directive 004 — Kilo → GitHub Copilot agent migration (merged, `38f5fed5`)

Merged as PR #20 (`dc16b4fe` + `d931f888`), CI 21/21 green including miri, branch
deleted. Kept here because the `400` diagnosis is expensive to rediscover.

### The `400 The requested model is not supported` diagnosis

The directive assumed the error was a single misconfiguration. Controlled
experiment showed **three independent root causes**; the first two are
configuration, the third is a VS Code architecture boundary.

**A — Agent Host hardcoded default.** `agentHostMain.js` declares
`WS="gpt-5.6-luna"`, used as the default for
`chat.agentHost.copilot.executionSubagent.model` and returned by
`_getExecutionSubagentModel()` when unset. `gpt-5.6-luna` is not registered in
this BYOK-only deployment, so an unpinned `explore` dispatch returns `400`.

**B — SDK built-in agent YAMLs.** `@github/copilot/sdk/definitions/
{explore,task}.agent.yaml` declare `model: claude-haiku-4.5`. Pinning exactly
that id reproduces the `400` verbatim, and overriding the
`github.copilot.chat.exploreAgent.model` setting does not change the outcome.

**C — external platform limitation, not a repo defect.** `MEn`/`TEn` in
`workbench.desktop.main.js` split customization payloads by source:

```js
function TEn(s){return(s.type==="skill"||s.type==="agent")&&(s.source===Dr.local||s.source===Dr.user)}
let w=v.filter(k=>!TEn(k));   // → synced.bundle
let C=v.filter(TEn);          // → standalone.bundle
```

`.github/agents/*.agent.md` resolve as `type:"agent", source:local`, so they are
excluded from the synced plugin by construction. The standalone authority is
`<sessionType>-<hash>-standalone`; **no such plugin directory is ever written
to disk** (`ls ~/Library/Application Support/Code/agentPlugins/` shows four
`vscode-synced-customization-*` authorities and no `-standalone`),
`agenthost.log` records exactly one synced sync and one
`appliedCount=0` customization event, `renderer.log` never logs a standalone
push, and the Rust `runtime.node` never scans `.github/agents/` (its only two
occurrences are a docs URL and a system-prompt *prohibition*). The Agent Host's
`customAgents` is therefore structurally empty and `agent_type: aura-scout`
returns the seven builtin names.

**Consequence:** the seven agents are fully valid and fully discovered by the
workbench (`customizationsDebug.log`: `agents 7 0 2 9`, all seven paths, zero
rejections), and an explicit `model:` pin works for builtin agent types. What
cannot work in this VS Code build is dispatching a *custom* agent by
`agent_type` in the Agent Host harness. **Do not re-investigate C.** No
repository change can alter it.

Merged and closed; nothing about Directive 004 remains open.

**PR #13 — merged `19cc4f22`.** It closed the last red suite on the default
branch and repaired the gate that hid it. CI evidence: the `website` job ran
all twelve suites and logged zero `FAIL` lines (`stdlib-consistency` 123/0,
`docs-consistency` 138/0, `ais-doc` 43/1 skipped, `playground-parity` 93/0,
`semver` 52/0, `release-manifest` 12/0, `release-preflight` 32/0, `sbom` 9/0,
browser suites self-skipped per TD-27).

- **The default branch was red through eleven green PRs.**
  `website/tests/stdlib-consistency.test.mjs` asserted
  `/native-only/i.test(stdlibDoc)`, but `11aeb093` (B6/B7) had deliberately
  changed `website/content/reference-stdlib.md` to "host-owned" because B14
  gave the browser Playground HTTP on Host ABI 2. Assertion and doc had
  contradicted each other, undetected, since that commit. Proved pre-existing
  at base `c9f88665` by detached checkout before touching anything.
- **Root cause of the invisibility — TD-26.** The `website` CI job named four
  suites by hand and never invoked `website/tests/run-all.mjs`, so eight
  suites ran only when someone remembered. Of those eight, only
  `stdlib-consistency` was failing (measured, not assumed: `docs-consistency`
  137, `ais-doc` 89, `aurea` 39, `playground-parity` 93 passed; `nav-fit` and
  `aurea-browser` self-skip without Playwright).
- **The fix is bidirectional.** The stale assertion was replaced by two
  direction-guarded assertions — `host-owned` present **and** `native-only`
  absent, plus the browser session path present — so drift is caught in both
  directions. Mutation-verified both ways. An assertion that merely demands
  current wording cannot detect drift.
- **`guide-io.md` was also stale**, not just the test. The browser column
  claimed the filesystem's `native-only` status applied to HTTP and gave
  `E5002` unconditionally. Corrected to distinguish the default `0.3.1`
  runtime (ABI 1, no browser HTTP, `E5002`) from the ABI 2 dev line. The
  filesystem `native-only` claim on line 46 is **correct and untouched**;
  `strings` on the frozen wasm confirmed the ABI-1 virtual project really
  does deny HTTP.
- **Wiring the gate paid for itself immediately.** The first PR carrying
  `run-all.mjs` went red on `docs-consistency`, which verifies each normative
  page linked from the docs index still exists at the release tag. CI checks
  out at `fetch-depth: 1` without tags, so `v0.3.1` was absent and all four
  `git ls-tree` checks failed. Reproduced exactly with
  `git clone --depth=1 --no-tags file://$PWD` (zero tags, 133 passed /
  4 failed); fixed by `fetch-depth: 0` on the job (753 commits, 20 MB `.git`,
  on a job that already builds 46 pages) plus a tag-resolvability preflight
  that reports a missing tag once as itself instead of four times as missing
  documentation. Mutation-verified: tagless 133/5, tags restored 138/0.
- **TD-27 (OPEN, v0.3.2).** No CI job installs Playwright, so browser, a11y,
  browser-HTTP and http-security suites have always skipped — including B14's
  browser HTTP path. The `ci.yml` comment claiming they "run under Chromium
  when Playwright is available" was false and is corrected. Deliberately not
  wired here: adding a browser download changes the job's runtime and failure
  modes and deserves its own change with its own evidence.

**Exact next action:** PR #16 completed the B7 project-header contract, so the
item that used to be listed here is done. Directive 003-R §4 is now fully
closed: B6 is a formalized divergence pinned by `http-security.test.mjs`
("4 redirected call reports E4020", plus "never prints a status"), B7's
single-module and project-session header contracts are both pinned, B14 shipped
as `0.3.2-dev.8`, and TD-23 is behaviourally verified (scheduled run
`41037980226` executed 30 min per front-end target plus a 5 min runtime sweep,
96 min wall time, no crash artifacts). Continue §6 on the remaining
high-yield surfaces — Host/Python/WASM security, benchmark regressions,
fuzz-target coverage, differential/property suites, supply chain, and any
`test:*`-style script or suite that no runner invokes. Two such rot instances
are already fixed (`test:abi` in #16, `scripts/test_validate.py` in #17); an
audit of both test trees found every remaining suite wired, so look next for
*assertions* that cannot fail rather than for unwired files.

**TD-27 — merged `9548e4df`.** Closed the last unproven surface in CI. The
browser suites had never executed: no job installed Playwright, and nine suites
treated a missing dependency as *success*, so a broken `npm ci` or a failed
browser download left CI green. The fix is a shared
`website/tests/playwright-deps.mjs` helper (`detect` / `detectAll` /
`strictRequested` / `skipExitCode`), `AURA_REQUIRE_BROWSER=1` on both jobs, and
real installs with `--with-deps`. Evidence is behavioural: the `playground` job
logged `Cache not found`, downloaded Chrome for Testing 153.0.8010.12 plus
FFmpeg and Headless Shell, ran the apt step on ubuntu-24.04, and executed six
browser suites (199 assertions, zero skipped); the `website` job executed four
(897 assertions). The download path had never run before because a warm local
cache made `playwright install` a no-op locally. No gate was weakened.

Doctrines #1-#33 from across the campaign are now durable in `AGENTS.md`
(`## Hard-won rules`, 28 numbered rules in three groups), so they no longer
depend on session history.

Authoritative for **the active task and the exact next action**. Current
repository state lives in `AGENT_STATE.md`; operating rules in `AGENTS.md`.
Keep this file short — it is read at the start of every session.

## HISTORICAL — FEATURE DEVELOPMENT FROZEN; READ-ONLY AUDIT (2026-10-09)

**AURA FEATURE DEVELOPMENT — FROZEN FOR STABILIZATION.** The 0.3.2 Playground
development preview is published: `rewrite/v3-rust` at `9f8743ee` (= remote),
CI 20/20 green, Pages deployed, live Playground verified (stable 0.3.1 runs;
dev.5 does large-output accounting and real HTTP after consent; PokéAPI →
`200\npikachu\n`).

No new syntax / stdlib / Host capability / Playground / ABI work. Correctness
and security defects reproduced against the frozen surface remain eligible for
narrow fixes.

**Next campaign (this session, read-only first):**
1. A read-only engineering audit — findings in
   `docs/engineering/STABILIZATION_AUDIT.md`, each with file/location,
   governing authority, minimal reproducer, expected vs observed, severity,
   evidence, smallest safe correction, and a regression plan; classified
   CONFIRMED / FALSIFIED / DECISION REQUIRED / INSUFFICIENT EVIDENCE.
2. An **AIS Agent-Native RFC** (design only; no implementation) —
   `docs/engineering/AIS_AGENT_NATIVE_RFC.md`.

No production code is modified during the audit stage.

## HISTORICAL — AURA 0.3.2 PLAYGROUND & BROWSER HTTP (2026-10-09, PREVIEW)

The full campaign record is `docs/engineering/PLAYGROUND_032_CAMPAIGN.md`.

- **D1 + D2 implemented.** `Machine` is a transient view over
  `(&mut Interp, &mut MachineState)`; `PendingEffect` parks and resumes the
  same machine through `NativeOutcome::Suspend`. `src/run/session.rs` owns the
  interpreter (no self-reference). Host ABI 2 (`aura_session_*`, ABI `2`,
  status `3` = pending effect, per-run `effect_id`) is live; `runtime.mjs`
  feature-detects it and never sends it to a frozen ABI-1 artifact.
- **Browser HTTP works end to end.** `http_get`/`http_request` (with
  `headers`/`body`/`timeout_ms`) park on a Host effect; the Worker performs
  `fetch` after the page's per-origin consent. Denial → `E5002`; CORS
  rejection → `E4020`; `credentials:"omit"`, `redirect:"manual"`,
  `referrerPolicy:"no-referrer"`; no proxy.
- **Runtime identity:** `0.3.2-dev.5` (ABI 2) at the time of writing; later
  superseded by `0.3.2-dev.7` (ABI 2), which is the current preview. Neither
  is the public default (`manifest.current = 0.3.1`).
  `0.3.1` and all frozen artifacts are byte-identical; `v0.3.1` unmoved.
- **Evidence:** Rust `session` 14/0, all-features matrix 63/63; Node
  `session-http` 13/0; Chromium `browser-http` 8/0; live PokéAPI →
  `200\npikachu\n`; full website + Playground suites green.
- **Exact next action:** a human reviews the deferred binary-value-model (B)
  decision and the promotion/publishing of the development runtime. Nothing is
  pushed, tagged, released, or deployed.

## HISTORICAL — AURA WEBSITE AUREA MIGRATION (2026-10-09)

A full website redesign onto **Aurea** (Aura's own design system) plus a
reconciliation of all active website documentation against the published Aura
0.3.1 contract. Scope: `website/**`, the relevant `docs/**` content mirrors, and
website tooling/tests. `src/**`, `playground/runtimes/**`, `Cargo.*`, tags, and
frozen artifacts were **not** modified (verified byte-identical).

- Aurea is the **sole** design system. `website/assets/aurea.css` holds the
  canonical `--au-*` tokens + dark-default/light-inversion themes + primitives;
  `website/assets/styles.css` is Aurea-owned layout/component/docs styling with
  no tokens of its own. The Material 3 `website/assets/tokens.css` was deleted
  and removed from `lib/layout.mjs`; no `--md-*` reference remains. The
  standalone Playground stylesheet was rethemed onto the same `--au-*` tokens.
- Documentation reconciled to 0.3.1 (collections, `never`, pattern catch,
  semicolon rule, nested-array-call limitation, canonical typed-JSON, HTTP
  capability); the standard-library inventory is centralized in
  `website/content/stdlib.mjs` and consumed by the landing + reference pages.
- New guards: `tests/aurea.test.mjs` (extended), `tests/aurea-browser.test.mjs`,
  `tests/stdlib-consistency.test.mjs`, `tests/docs-consistency.test.mjs`.
- Follow-up (same session): fixed the reported 768px three-row navigation with a
  tiered, measured-fit app bar (`tests/nav-fit.test.mjs`), and unified the
  integrated + standalone Playground on one shared markup
  (`playground/web/workbench.mjs`) and stylesheet
  (`playground/web/workbench.css`), locked by
  `tests/playground-parity.test.mjs`. Both suites and the full website +
  Playground suites are green locally; nothing is pushed.
- Follow-up: reorganized the documentation manifest into six meaning-based
  groups and added the AIS (`/docs/ais/`) and MCP (`/docs/mcp/`) pages under a
  Semantic Tooling group, grounded in `src/ais.rs`/`src/mcp.rs` with examples
  generated from the released compiler. New guards: `tests/ais-doc.test.mjs`
  (real-binary fixtures + golden JSON) and extended
  `tests/docs-consistency.test.mjs`. Surfaced in CLI/Tools/Architecture. Full
  website suite green (46 pages); nothing pushed. Website publication is the
  next human-authorized gate.

**Exact next action:** the human reviews the committed website work and
authorizes (or declines) a push. A push to this branch triggers `pages.yml` and
deploys the site — confirm acceptance before pushing. **Do not push without
explicit authorization**, and **do not start Aura 0.3.2**. Git reality at this
checkpoint: local = remote = `b909066f`, `v0.3.1` = `b09cbef2`.

## AURA 0.3.1 KEYSTONE — PUBLISHED (2026-10-08)

The 0.3.1 campaign is **closed**. Aura 0.3.1 (codename Keystone) is published:

- Source tag `v0.3.1` → `b09cbef2` (pushed `rewrite/v3-rust`; local = remote).
- GitHub Release: https://github.com/JoaoValentimTheo/aura-lang/releases/tag/v0.3.1
  with platform tarballs (Linux/Windows/macOS), the immutable
  `aura-playground-runtime-0.3.1.wasm` (+ checksum), `release-manifest.json`,
  and `sbom.json`.
- Exact-SHA CI (run 37864894819): **20/20 jobs green**, including the previously
  red `playground (wasm runtime)`.
- Pages deploy (run 37864894705): success; live site
  https://joaovalentimtheo.github.io/aura-lang/ serves `Aura 0.3.1 Keystone`.
- New runtime: `0.3.1`, language `0.3.1`, codename Keystone, ABI 1, sha256
  `96e778ae50bd261f76d49adf04b8650cc3b60678a14c0e0ac555fb54400256ab`,
  1,951,520 bytes. Frozen `0.0.2`/`0.2.0`/`0.2.1` byte-for-byte unchanged;
  tags `v0.0.2`/`v0.2.0`/`v0.2.1` unmoved.
- `0.3.1` is a language release (release = language), per ADR-0001, because
  Keystone changes observable behavior (distinct collection identities,
  contextual Arrays, canonical type-position `json_decode_as`, no general
  semicolon sequencing).

**Exact next action:** none for 0.3.1. The campaign is frozen. The next line,
`0.3.2`, is planned but **not started**; do not begin it (or HTTP expansion, or
local macOS CI migration) without a separate, explicit human instruction.
Publication of any 0.3.2 artifact requires a fresh authorization.

## Validation infrastructure (2026-10-08, local, unpushed)

The pathological macOS validation wall time was **root-caused and fixed**, not
merely made observable.

- **Root cause.** The first execution of a *freshly linked* Rust test binary
  costs 30-65 s wall / ~0.00 s user CPU. It is macOS's security assessment of
  the ad-hoc linker-signed Mach-O image. Reproduced with unique binaries:
  a fresh binary in `target/debug/deps` (758,385 entries) first-executes in
  35-62 s; the *same* binary copied to a small scratch dir first-executes in
  0.4-0.9 s; the second exec anywhere is 0.01 s. Directory size alone is not
  the cause (750k empty files = 4 s); the freshly lined image in the huge deps
  directory is.
- **Fix.** `scripts/validate.py` now discovers the full test surface from
  `cargo metadata` plus the real artifact stream (lib unit tests, all
  integration tests, the bin harness, doctests; the previously skipped lib
  target is now included), compiles each configuration once with live 5 s
  progress and a separate compile timeout, **stages each freshly linked test
  binary into `target/validate-run/` and executes it there**, runs each target
  as its own process group with a per-target deadline (SIGTERM then bounded
  SIGKILL), persists results atomically and incrementally (resumable even
  under `--fresh`), and keys the cache on the artifact digest + runtime-input
  content digest + toolchain + target + feature config. Regression tests:
  `python3 scripts/test_validate.py` (18 tests, ~4 s).
- **Measured result.** All three configurations now run end-to-end in **~369 s
  total** (all-features 97 s, canonical 97 s, bare 44 s; fresh builds):
  `PASS=185 FAIL=0 TIMEOUT=0 SKIP=1 COMPILE_FAILURE=0`. The single SKIP is the
  `aura` bin target under `bare` (`required-features = ["cli"]`), reported as
  SKIP, not FAIL. Previously the matrix exceeded a 600 s shell timeout with no
  output.
- **Real defect surfaced.** `tests/module_graph.rs`'s
  `typed_decode_type_argument_resolves_in_a_child_module` called
  `json_decode_as` with no `#[cfg(feature = "json")]` gate, so it failed under
  `--no-default-features`. Gated; bare `module_graph` is 37/0 and
  all-features 38/0.
- **Historical evidence preserved.** The prior all-features 59/59 PASS cache is
  archived at `target/validate-cache/results.pre-repair.json.bak`; it is not
  presented as a fresh run.

## Program

B-1 — WebAssembly call-frame implementation nonconformance. The released
runtime traps on the JavaScript engine stack below the normative 512-frame
language limit instead of reporting `E4011`. The chosen remediation is an
explicit-continuation (iterative) evaluator over the existing AST
(`docs/engineering/ITERATIVE_EVALUATOR_DESIGN.md`, Option E1).

## Current status

- B-1: **REMOTELY CLOSED** at `bb736fc` (inside pushed history; remote tip
  `089dffe` records the closure). The evaluator migration and production
  cutover are pushed; exact-SHA CI green (all 20 jobs) and Pages deployed.
  Released `0.2.1` runtime still contains the defect; the freshly built
  machine-backed runtime holds the language boundary.
- POST-B1 RUNTIME/WASM EDGE CLOSURE: **REMOTELY CLOSED** at `e576238`
  (pushed 2026-10-05; exact-SHA CI 20/20 green and Pages deployed after a
  GitHub Actions runner incident delayed the first attempts). The stdout
  capture-bound contract is pinned in `playground/runtime/tests/execute.rs`,
  `tests/host.rs`, and `playground/tests/node/b1_boundary.test.mjs`, and
  documented in `docs/playground.md` §2.
- AURA 0.3 KEYSTONE SUPER-TRANSACTION: **CLOSURE RANGE PUSHED THROUGH
  `b221e6a9`**; the remote is no longer at `e576238`. Manifest
  `docs/engineering/KEYSTONE_SCOPE_MANIFEST.md`, ledger
  `docs/engineering/KEYSTONE_JEV_LEDGER.md`, RFC
  `docs/rfcs/0001-exception-catch-selection.md`. Landed: exceptions E1-E6
  (pattern catch, raise-site spans, `Aura` namespace reservation), types
  (`never`, precise `none`, optional narrowing), unused analysis `E2008`,
  same-revision Native/WASM differential, HTTP capability, typed JSON
  decoding, structured diagnostics + CLI color, Aurea + overflow invariants,
  codename metadata, tooling reconciliation.
- **REMOTE CI IS RED AT `b221e6a9`** (run `37639795923`, job
  `playground (wasm runtime)`): `b1_boundary.test.mjs` reports 44/19 with
  grouping/f-string `guest trap: Maximum call stack size exceeded` /
  `memory access out of bounds`. The cause is a parser that bounds grouping
  and f-string interpolation recursion. The fix is committed locally as the
  two commits **ahead** of the remote (`04783231`, `e7505408`) and is
  **unpushed**. On the local tip the fresh-wasm boundary suite is **63/0**,
  the full playground suite is green (differential 233/0, browser 98/0,
  worker 12/0), the native matrix is green (all-features 57 suites + 166 lib;
  bare and canonical 57+57; lib 124/166), clippy (both configs) / fmt / MSRV
  1.83 / nightly fuzz check all exit 0, and the website suite is green
  (browser 380/0, a11y 70/0, links 2499 OK, release-version 29/0). Frozen
  `0.0.2`/`0.2.0`/`0.2.1` byte-identical; `v0.2.1` = `3f5f8702…` unmoved;
  `.kilo/**` protected churn untouched; no tag/release/deploy.
- KEYSTONE CLOSURE CAMPAIGN (2026-10-07, pushed through `b221e6a9`): committed
  `2f99a18` (third/fourth/fifth-review checker hardening), `c510215`
  (Value-algebra anti-collapse + type families + AIS delivery + MCP),
  `5abcd0d` (PokéAPI dogfood), `31c77be` (Playground FSM), `6e3ff89` (AIS
  delta fix + properties), `1932047` (Jev discovery + differential harness),
  `b4aabb1` (spec version drift), `ffc701f` (website drift), `a5f182c` +
  `4a36681` (regression seeds, fuzz lock), `6893457` (AIS narrowing proofs).
  Real defects fixed with regression evidence: `json_encode` silently
  collapsing unrepresentable kinds to `null`/payload; the Playground status
  starting as the non-vocabulary `idle`; AIS delta reporting an unrelated
  insertion as a semantic change to every following symbol; the spec header
  claiming language version 0.0.1; the website claiming no development line
  exists. Campaign premise about `;` **falsified with evidence**: the
  canonical grammar, spec §3.7, CONF-PARSE-8 and `tests/syntax_conformance.rs`
  all define `;` as a statement terminator, and `[1; 3]` is `E1006` — no
  grammar change was made.
- Remaining closure work: none required for the publication gate beyond the
  human gates below. Final validation on this exact source: all-features
  1333/0 (176 suites), no-default canonical 1252/0, bare --no-default-features
  1054/0, clippy -D warnings / fmt / MSRV 1.83 / nightly fuzz check all exit 0,
  playground suite green (differential 233/0, browser 98/0, boundary 63/0,
  worker 12/0, multi-file 42/0, cache 7/0), website suite green (browser
  380/0, a11y 70/0, links 2499 OK across 43 pages, release-version 29/0).
  Frozen `0.0.2`/`0.2.0`/`0.2.1` byte-identical; `v0.2.1` = `3f5f8702…`
  unmoved; `.kilo/**` protected churn untouched; no tag/release/deploy.
  **Exact next action:** the branch is at the publication gate. A push must
  be explicitly authorized by the human; if authorized, `git push` normally
  (never force), then verify the exact pushed SHA in CI. Note that CI's
  `pages.yml` triggers on push to these paths and will deploy the website —
  that is the repository's existing behavior, so a push publishes the site;
  confirm the human accepts that consequence before the push.
  Five adversarial review passes total (J1–J12 in the ledger); each finding
  was independently reproduced against the built CLI before being fixed:
  second (committed `45271b2`), then the third/fourth/fifth passes hardening
  `never` reachability, shadowed-callee divergence, transitive closure
  capture, literal conditions, `try`-`finally`, statically-empty iterables,
  eager value-position divergence, and `-> never` method divergence.
  `tests/keystone_types.rs` is 48/48. The full validation floor is green on
  this state (all-features 1247/0, canonical 1172/0, bare 1037/0,
  clippy/fmt/MSRV/fuzz-check clean, playground + website suites green).
  Remaining human gates: shadowing policy, package-manager model, runtime
  publication/codenames.
- PRE-0.3 FOUNDATION SUPER-TRANSACTION: **PUSHED THROUGH `b221e6a9`**.
  Iteration ledger and
  artifacts: `docs/engineering/RUNTIME_ARCHITECTURE.md`,
  `EXCEPTION_ARCHITECTURE.md`, `CAPABILITY_MODEL.md`,
  `EMBEDDED_PYTHON_ARCHITECTURE.md`, `CRITICAL_SYSTEM_PROFILE.md`,
  `MODULE_ARCHITECTURE.md`, `FUTURE_EXTENSION_BOUNDARIES.md`,
  `DIAGNOSTIC_TAXONOMY.md`, plus `tests/stdlib_coverage.rs` and the extended
  transport-limit, poison-recovery, and benchmark coverage.
- B-1R1 (design): COMPLETE.
- B-1R2 (differential oracle): COMPLETE and mutation-validated.
- B-1R3A-ARCH-1 (Arc AST sharing): RESOLVED.
- B-1R3A (machine skeleton + first executable subset): **COMPLETE AND REMOTELY
  CLOSED** at `c9ade0b` (exact-SHA CI green; website deployed).
- B-1R3B.1 (unary `-`/`not`/`~`): **COMPLETE AND PUSHED** at `1a12b84`.
- B-1R3B.2 (eager binary operators): **COMPLETE AND PUSHED** at `1a12b84`.
- B-1R3B.3 (short-circuit `and`/`or`): **COMPLETE AND PUSHED** at `2f34b9c` via
  the intermediate commits `58a1b16`/`31e313a`/`2f34b9c`.
- B-1R3B.4.1 (list/tuple construction): **COMPLETE AND PUSHED** at `efc66bd`
  (remote-closed).
- B-1R3B.4.2 (map construction): **COMPLETE AND REMOTELY CLOSED** at `5e70677`.
- B-1R3B.5 (range construction): **COMPLETE AND REMOTELY CLOSED** at `cf17689`.
- B-1R3B.6 (index / field reads): **COMPLETE AND REMOTELY CLOSED** at `cf17689`.
- B-1R3B.7 (f-strings): **COMPLETE AND REMOTELY CLOSED** at `52124a0` (pushed
  after a passing adversarial push gate; exact-SHA CI and Pages green; local =
  tracking = server at `52124a0`, ahead/behind 0/0).
- B-1R3B.8 (milestone completion audit + dependency graph): **COMPLETE AND
  REMOTELY CLOSED** at `9cb5e28` — `docs/engineering/B1R3B8_COMPLETION_AUDIT.md`.
- B-1R3C–B-1R3F (Call, Method, Construct, Lambda, Pipe, Assign, LetPattern,
  While, Loop, For, ListComp, MapComp, Match, Try): **COMPLETE AND PUSHED** in
  the B-1 closure range (tip `bb736fc`).
- Production cutover `62dd592`: **COMPLETE AND PUSHED** in the closure range.
  Every production entry point runs the machine; the residual recursive seams
  were closed at `11abad2` (free `aura::execute_with`, REPL `Item::Const`,
  Playground `execute`/`run_module_capture`).
- B-1R4 (full differential): DONE (whole-corpus `engines_agree`; differential
  228/228; syntax conformance 43/43).
- B-1R5 (substrate boundary): PARTIALLY DONE — fresh machine-backed wasm pinned
  by `playground/tests/node/b1_boundary.test.mjs` (63 checks: the frame/pattern
  boundary matrix, the pattern-depth calibration, and the post-B1 stdout
  capture-bound checks) and native CLI/REPL canaries. The Chromium main-thread
  and production-Worker boundary matrix needs the fresh artifact published to
  close (human-gated; frozen `playground/runtimes/**`).
- B-1R6 (red team): DONE — independent read-only review of the full range; all
  fifteen claims confirmed with no falsification; six minor findings, A/B/C/D/F
  fixed in `39caf6f` (field-receiver and tuple order differentials,
  `#[doc(hidden)]` on `Interp::run`, callback-confinement tripwire, wasm
  pattern-depth calibration pin); E did not reproduce.
- B-1R7 (full validation gate): DONE — including the exact-SHA CI run on
  `bb736fc` (all 20 jobs green) after the additive miri fix.
- B-1R8 (recursive-engine removal + oracle switch removal): NOT STARTED —
  human-gated; requires a release decision.

See `AGENT_STATE.md` for the exact SHAs and ahead/behind.

## Production vs experimental engine

- Production/default/CLI/REPL/Playground/library: **explicit-continuation
  machine** since the local cutover `62dd592`. The recursive evaluator is
  retained only as the differential reference (`Compilation::execute_recursive*`)
  and the rollback path.
- The machine handles **every** `Expr` (21) and `Stmt` (12) variant with no
  recursive fallback and no unsupported sentinel: literals, name lookup,
  expression statements, blocks, `let` shadowing and `let` patterns, assignment
  (simple/compound, name/index/field targets), `if`/`while`/`loop`/`for` (lazy
  ranges), `break`/`continue` boundaries, `return`/`throw`, unary/binary/
  short-circuit operators, list/tuple/map/range construction, index/field
  reads, f-strings, user/native/closure/method calls (including the resumable
  `map`/`filter`/`reduce` callback protocol), struct/enum construction,
  lambdas, pipes, list/map comprehensions, `match`, and `try`/`catch`/`finally`.
  The whole corpus is required to agree between engines
  (`tests/evaluator_oracle.rs::engines_agree`).

## Completed in B-1R3B.7 (remote-closed at `52124a0`)

- `src/run/iterative.rs` — added `Cont::FStrNext { parts, index, spec, out, env }`,
  `Machine::start_fstring`/`advance_fstring`, and the `Expr::FStr` arm in
  `start_expr` plus the `Cont::FStrNext` `resume` arm. Parts are consumed left
  to right, exactly once each, mirroring `Interp::eval_inner`'s `Expr::FStr`
  arm: each `FPart::Lit` is appended verbatim (raw text; `{{`/`}}` already
  resolved by the parser), each `FPart::Expr` is evaluated exactly once and
  appended with `v.display()` (no spec) or `Interp::format_value` (with a
  spec). The accumulator `String` lives in the continuation, so nesting does
  not grow the Rust stack. A control signal or diagnostic from an interpolation
  aborts the whole f-string and propagates unchanged, so later parts never run
  and no partial string is observable. No recursive AST evaluation and no
  fallback.
- Oracle: new `tests/oracle/r3b7_golden.tsv` (LF-pinned; 112 supported cases)
  and differential tests `r3b7_fstring_supported_subset_agrees`,
  `iterative_fstring_unsupported_fails_explicitly`, `r3b7_iterative_golden_matches`,
  plus `r3b7_golden_has_lf_pin` and 11 crate-internal f-string unit tests.
  Boundary movements: `fstring` removed from the R3A unsupported set
  (`r3a_golden.tsv` −1 row); `and_skip_fstring`/`or_skip_fstring` moved from the
  R3B.3 supported set into the R3B.7 set (`r3b3_golden.tsv` −2 rows);
  `nested_unsupported_element` (R3B.4.1), `fstring_value` (R3B.4.2), and
  `fstring_index` (R3B.6) moved out of their unsupported sets into supported
  R3B.7 cases. The main `golden.tsv` is byte-unchanged.
- Verified: empty/text-only/escaped-brace shapes; raw (undecoded) literal text;
  the stringification matrix (none/bool/int/float/string/list/tuple-sugar/map/
  range/`<fn>`); the format mini-language (`d b o x X f F e E %`, sign, width,
  fill/align, zero-pad, precision) and its `E3001` type and `E4013`
  precision/width bounds; Unicode; one-level nested f-strings; interpolation
  source order with first-error-wins; control-signal abort; `and`/`or` skipping
  of a whole f-string; composition with List/Map/Range/Index/Field/block/let/
  if; program-mode frame boundary; `E4999` no-fallback unsupported surface;
  host-stack/AST-depth safety and `expr_depth` restoration. A deliberate
  `display`→`debug_repr` stringification mutation was detected by seven R3B.7
  cases and reverted byte-exactly.
- Honest gaps: exactly-once is structural, not differentially falsifiable in the
  current subset (the only side-effecting interpolation constructs — calls,
  assignment, `print` — are still unsupported). Two-level f-string nesting is
  not expressible in the grammar (the innermost level would need the outer
  delimiter, which terminates it). Struct-instance/variant stringification is
  unreachable (`Expr::Construct` unsupported).

## Completed in B-1R3B.6 (remote-closed at `cf17689`)

- `src/run/iterative.rs` — added `Cont::IndexTarget { idx, env, span }`,
  `Cont::IndexApply { base, span }`, and `Cont::FieldReceiver { name, span }`,
  the `Expr::Index` / `Expr::Field` arms in `start_expr`, and the three `resume`
  arms. Index evaluates the target first and exactly once, then the index
  exactly once, then applies the lookup through `Interp::index_get`; a control
  signal from either operand is redelivered and the lookup never runs. Field
  evaluates the receiver once and resolves it exactly like `eval_inner`'s
  `Expr::Field` arm (struct field read with the method-not-a-value `E2003`
  guard, or zero-argument builtin method dispatch). Because struct construction
  is `Expr::Construct` (still unsupported), `Value::Instance` field reads are
  unreachable in this subset; the reachable field surface is the builtin method
  registry and its `E2003` unknown-member diagnostic. No recursive AST
  evaluation and no fallback.
- Oracle: new `tests/oracle/r3b6_golden.tsv` (LF-pinned; 55 index + 23 field =
  78 supported cases) and differential tests
  `r3b6_index_supported_subset_agrees`, `r3b6_field_supported_subset_agrees`,
  `iterative_index_field_unsupported_fails_explicitly`,
  `r3b6_iterative_golden_matches`, plus `r3b6_golden_has_lf_pin`. Boundary
  movement: `index` removed from the R3A unsupported set (now supported;
  `r3a_golden.tsv` −1 row). `field_access` stays in R3A unsupported: it is a
  struct literal via `Expr::Construct`. The main `golden.tsv` is byte-unchanged.
- Verified: target-before-index order and exactly-once; list/tuple/string/map
  lookup; negative-index normalization; `E4019` out-of-range; `E2003` missing
  map key and unknown member; `E3001` non-key-capable map key, unsupported
  base/index combination, and check-time map key-kind mismatch; receiver error
  wins over member resolution; control-signal propagation from target, index,
  and receiver; nested Index/Field and Range composition; program-mode frame
  boundary (Index and Field); unsupported→`E4999` with no fallback; and
  host-stack/AST-depth safety. A deliberate base/index-swap mutation was
  detected by three R3B.6 tests and reverted byte-exactly.
- Historical note (now resolved): struct-instance field reads and
  struct-instance indexing could not be differentially exercised until
  `Expr::Construct` landed in B-1R3C.3, and exactly-once was structural until
  calls/assignment/`print` landed in B-1R3C.1/B-1R3D.1; both gaps are closed by
  the R3C–R3F migration and the `print`-observable order cases in
  `tests/oracle/r3c1.rs`.

## Completed in B-1R3B.5 (remote-closed at `cf17689`)

- `src/run/iterative.rs` — added `Cont::RangeStart { end, env, span }` and
  `Cont::RangeEnd { start, span }`, the `Expr::Range` arm in `start_expr`, and
  the two `resume` arms. The start operand is evaluated first and exactly once;
  its completion retains the value (unvalidated) and schedules the end exactly
  once. **Both** operands complete before either bound is validated, and
  validation is start-first, exactly like `Interp::eval_inner`'s `Expr::Range`
  arm (`src/run/mod.rs:1583`): a non-int bound is `E3001` with the recursive
  message (`range start/end expects an int, found {type}`) at the range
  expression's span, and a start error beats a runtime end error, while an end
  signal/error preempts a runtime-invalid start (the end is evaluated first).
  A valid pair becomes the same `Value::Range(RangeVal { start, end })` that
  `range(a, b)` builds (half-open, end-exclusive). A control signal from either
  operand aborts construction and propagates unchanged; no partial Range
  escapes. No recursive AST evaluation and no fallback.
- Oracle: new `tests/oracle/r3b5_golden.tsv` (LF-pinned; 55 supported cases)
  and differential tests `r3b5_range_supported_subset_agrees`,
  `iterative_range_unsupported_fails_explicitly`, `r3b5_iterative_golden_matches`,
  plus `r3b5_golden_has_lf_pin`. Boundary movements: `range_literal` removed
  from the R3A unsupported set (`r3a_golden.tsv` −1 row); `and_required_range`
  removed from the R3B.3 unsupported set and replaced by six supported range
  short-circuit cases (`r3b3_golden.tsv` +5 rows); `range_element` moved from
  the R3B.4.1 unsupported set into the supported set plus a new program-mode
  `main_list_range` (`r3b4_golden.tsv` +2 rows); `range_value` moved from the
  R3B.4.2 unsupported set into the supported set plus a new program-mode
  `main_map_range` (`r3b42_golden.tsv` +2 rows). The main `golden.tsv` is
  byte-unchanged.
- Verified: start-before-end order and exactly-once scheduling; both-bounds
  evaluated then start-first validation; `E3001` runtime/check-time split;
  `E3001` at the exact range span; control propagation from either bound;
  `i64` extremes (`len()` saturates), descending/equal/empty, negative bounds;
  list/map/if/let/block/program composition; no fallback on unsupported bounds
  (`E4999`); AST-depth `E1015` below/above the limit; deep nesting host-stack
  safe. A deliberate start/end endpoint-swap mutation was detected by the R3B.5
  differential + golden tests and the composite R3B.4/R3B.4.2 tests, then
  reverted byte-exactly.
- Honest gap: exactly-once is structural but not differentially falsifiable in
  the current subset, because the only side-effecting bound constructs
  (calls/assignment/print) are still unsupported (recorded in
  `src/run/iterative.rs`).

## Completed in B-1R3B.4.2 (remote-closed at `5e70677`)

- `src/run/iterative.rs` — added `Cont::MapKeyNext { entries, index, out, env }`
  and `Cont::MapValueNext { key, entries, index, out, env }`, `Machine::start_map`,
  and the `Expr::Map` arm in `start_expr`. Entries run in source order; for each
  entry the key is evaluated and validated with `MapKey::from_value` **before**
  its value is scheduled, exactly like `Interp::eval_inner`'s `Expr::Map` arm
  (`src/run/mod.rs:1464`). An invalid runtime key is `E3001` with the exact
  recursive message at the key expression's span; the value never runs. A
  control signal or diagnostic from a key or value aborts the map and propagates
  unchanged, so later entries never run and no partial map is observable.
  Duplicate keys keep the last value (`BTreeMap::insert`), matching recursion.
  Empty `{:}` completes immediately. The accumulated `BTreeMap` lives in the
  continuations; no `Map`-sized or nested Rust recursion. No recursive AST
  evaluation and no fallback.
- Oracle: new `tests/oracle/r3b42_golden.tsv` (LF-pinned; 57 supported cases:
  49 value-mode and 8 program-mode; 57 golden rows) and differential tests
  `r3b42_map_supported_subset_agrees`,
  `iterative_map_unsupported_fails_explicitly`,
  `r3b42_iterative_golden_matches`. Boundary movements: `map_literal` removed
  from the R3A unsupported set (now supported); `or_required_map`/`and_lhs_map`/
  `or_lhs_map` moved from the R3B.3 unsupported set into the supported set;
  `map_element` moved from the R3B.4.1 unsupported set into the supported set
  (new rows in `r3b3_golden.tsv`/`r3b4_golden.tsv`; `r3a_golden.tsv` −1 row).
  The main `golden.tsv` is byte-unchanged.
- Push-blocker remediation: the R3B.4.1 `main_map_element` program-mode case
  (list containing a Map through a real frame boundary) was dropped without
  replacement when map elements moved to supported coverage; it is restored in
  the R3B.4.2 supported set (`r3b42-program/main_map_element`) with a guard test,
  and `tests/oracle/r3b42_golden.tsv` is now pinned `text eol=lf` in
  `.gitattributes` like every sibling golden.
- Verified: key-before-value and entry-by-entry order; exactly-once key/value;
  runtime invalid-key `E3001` at the key span with value suppression;
  first-error-wins; key/value control-signal abort with later-entry and
  later-unsupported skipping; last-wins duplicate keys; nested maps,
  Map-in-List, List-in-Map; `{:}`; unsupported keys/values → `E4999` with no
  fallback; AST-depth `E1015` below/above limit; deep nesting through a real
  frame boundary and mixed Map/List nesting host-stack safe; and a deliberate
  key/value-order mutation detected by the map oracle tests and reverted.

## Completed in B-1R3B.4.1 (pushed at `efc66bd`)

- `src/run/iterative.rs` — added `Cont::ListNext { items, index, out, env }`,
  `Machine::start_list`, the `Expr::List`/`Expr::Tuple` arm in `start_expr`, and
  the `Cont::ListNext` arm in `resume`. Elements are evaluated left to right,
  exactly once each, with the same environment; a non-`Val` completion from an
  element aborts the list and is redelivered unchanged (later elements are
  never scheduled); completed elements become `Value::list`. `Expr::Tuple`
  shares the list path because the recursive arm is byte-identical and
  `LANGUAGE_SPEC.md` §21 froze `(a, b)` as list sugar **at that time**. No
  recursive AST evaluation and no fallback.
  > **SUPERSEDED (2026-10-08):** the Keystone collection decision (ADR-0005)
  > makes `Expr::Tuple` a distinct `Value::tuple`; `start_expr` now dispatches
  > `Expr::List`/`Expr::Tuple`/`Expr::Set` to `start_seq`/`start_set`, and an
  > `[a, b]` literal under an `[T; N]` expectation realizes a `Value::array`.
- Oracle: new `tests/oracle/r3b4_golden.tsv` (LF-pinned; 37 supported cases)
  and differential tests `r3b4_list_supported_subset_agrees`,
  `iterative_list_unsupported_fails_explicitly`,
  `r3b4_iterative_golden_matches`. `list_literal`/`tuple_literal` moved out of
  the R3A unsupported set (and `r3a_golden.tsv` regenerated accordingly);
  `and`/`or` list-operand cases moved from the R3B.3 unsupported set into the
  supported set (6 new rows in `r3b3_golden.tsv`). The main `golden.tsv` is
  byte-unchanged.
- Verified: order/exactly-once (first error wins; later-error cases), element
  control-signal abort with unsupported-later-element skipping, exact
  diagnostics/spans, nested lists, AST-depth `E1015` below/above the limit,
  deep nesting through a real frame boundary host-stack safe, and a
  deliberate element-order mutation detected by four oracle tests and reverted
  byte-for-byte.

## Completed in B-1R3B.3 (pushed at `2f34b9c`)

- `src/run/iterative.rs` — added `Cont::ShortCircuitLeft { op, rhs, env }` and
  `Cont::ShortCircuitRight`; the machine evaluates the left operand exactly
  once, and only on completion decides from `Value::truthy()` whether the right
  operand is required (`and`: truthy left; `or`: falsy left). A skipped right
  operand is never scheduled (no stdout, mutation, diagnostic, or signal); a
  required right is evaluated exactly once. The result always mirrors the
  recursive arms: `Bool(false)` / `Bool(true)` on the deciding left, otherwise
  `Bool(rhs.truthy())`; control signals propagate via the `val!`-equivalent
  redelivery path. No recursive AST evaluation and no fallback.
- Oracle: new `tests/oracle/r3b3_golden.tsv` (LF-pinned) and differential tests
  `r3b3_short_circuit_supported_subset_agrees`,
  `iterative_short_circuit_unsupported_fails_explicitly`, and
  `r3b3_iterative_golden_matches`. `and_bool`, `or_bool`, and `eager_over_and`
  moved out of the R3B.2 unsupported set (now supported); the call-operand case
  stays. The main `golden.tsv` and all earlier iteratives goldens are unchanged.
- Verified: skipped-RHS error suppression (E4007/E4013/E3001/E4999 must not
  occur), required-RHS exact diagnostics, control-signal propagation, deep
  chains host-stack safe, `and`/`or` decision mutations rejected by the oracle.

## Completed in B-1R3B.2 (pushed at `1a12b84`)

- `src/run/iterative.rs` — added `Cont::BinaryLeft { op, rhs, env, span }` and
  `Cont::BinRight { op, lv, span }`; the machine evaluates the left operand,
  retains it, evaluates the right exactly once, then applies the operator only
  after both complete. Operator semantics are delegated to `Interp::binary`
  (the recursive engine's value-level helper, which never evaluates an AST
  node), so arithmetic, overflow (`E4013`), divide-by-zero (`E4007`), bitwise,
  shift-range, comparison, and equality behavior are identical by construction.
  `and`/`or` (B-1R3B.3) remain unsupported and never fall back.
- Oracle: new `tests/oracle/r3b2_golden.tsv` (LF-pinned) and differential tests
  `r3b2_binary_supported_subset_agrees`, `iterative_binary_unsupported_fails_explicitly`,
  `r3b2_iterative_golden_matches`. The R3A unsupported case `binary_add` was
  removed (now supported) and `r3a_golden.tsv` regenerated accordingly; the main
  `golden.tsv` is unchanged.
- Deliberate operand-swap mutation was detected by both R3B.2 tests and reverted.

## Completed in B-1R3B.1 (local)

- `src/run/iterative.rs` — added `Cont::UnaryApply { op, span }`; the operand is
  evaluated once and the operator is applied at its completion. `apply_unary`
  mirrors `Interp::eval`'s `Expr::Unary` arm exactly (checked `int` negation with
  `E4013`, `not` over `truthy()`, `~` on `int` only, both type errors `E3001`
  with the same messages). Operand control signals (`return`/`throw`/…) propagate
  without applying the operator; a deep unary chain is stack-safe.
- Oracle: `tests/oracle/r3b.rs` + `tests/oracle/r3b_golden.tsv` (LF-pinned) and
  differential tests `r3b_unary_supported_subset_agrees`,
  `iterative_unary_unsupported_fails_explicitly`, `r3b_iterative_golden_matches`.

## Completed in B-1R3A (remotely closed at `c9ade0b`)

- `src/run/iterative.rs` — explicit `Machine`/`Ctrl`/`Cont`/`UserFrame`/
  `FrameBoundary`; one `loop` over `Ctrl` with a `Vec<Cont>` stack; 512/513
  frame accounting and the `E1015` AST-depth guard, unit-tested.
- Oracle extended with an identified R3A subset (`tests/oracle/r3a.rs`),
  strict supported-subset equality, an explicit no-fallback test, and a
  full-field iterative golden (`tests/oracle/r3a_golden.tsv`, LF-pinned).

## Current invariants — do not break

- AST sharing uses `std::sync::Arc` (not `Rc`); runtime `Env`/`Value`/`Closure`
  deliberately stay `Rc`. Do not mechanically convert runtime `Rc` to `Arc`.
- The main oracle golden `tests/oracle/golden.tsv` is byte-exact and must not
  change unless an intended observable semantic change is being made.
- Frozen release runtimes (`0.0.2`, `0.2.0`, `0.2.1`) are immutable, byte for
  byte.
- No `unsafe`; no manual `Send`/`Sync`.
- B-1 is REMOTELY CLOSED: the full migration and the 512-frame boundary hold
  on every substrate (fresh wasm proven; frozen `0.2.1` remains the historical
  artifact). Do not reopen B-1 without a new human gate.

## Do not touch

- `playground/runtimes/**` (frozen artifacts).
- The `v0.2.1` tag or any release.
- Untracked/pre-existing dirty state (see `AGENT_STATE.md` protected local
  state), including `.kilo/**`.
- The website, unless a current public statement is demonstrably false.

## Relevant authority

- Design: `docs/engineering/ITERATIVE_EVALUATOR_DESIGN.md`
- Oracle: `docs/engineering/B1R2_DIFFERENTIAL_ORACLE.md`
- Arc decision: `docs/B1R3A_AST_SHARING_DECISION.md`
- B-1 decision package: `docs/WASM_CALL_FRAME_LIMIT_DECISION.md`
- Normative semantics: `docs/LANGUAGE_SPEC.md`

## Required tests for B-1R work

- `cargo test --locked --features evaluator-oracle --test evaluator_oracle`
- `cargo test --locked --features evaluator-oracle --lib iterative`
- `cargo fmt --all -- --check`
- `cargo clippy --locked --all-targets --all-features -- -D warnings`

At milestone closure, run the full validation floor in `AGENTS.md`.

## Phase ledger — B-1R3B (values and operators)

R3B is **COMPLETE**: every planned microphase (R3B.1—R3B.7) is implemented,
differentially oracled, and remotely closed; R3B.8 audited the milestone. The
B-1R3C–B-1R3F evaluator migration and the production cutover `62dd592` are
implemented, oracle-covered, validated, **pushed, and remote-closed** as part
of the B-1 closure range (tip `bb736fc`).

- R3B.1 unary operators — **COMPLETE AND PUSHED**
- R3B.2 binary operators — **COMPLETE AND PUSHED**
- R3B.3 short-circuit / evaluation order — **COMPLETE AND PUSHED**
- R3B.4.1 list / tuple construction — **COMPLETE AND PUSHED**
- R3B.4.2 map construction — **COMPLETE AND REMOTELY CLOSED**
- R3B.4.3 (reserved: map-key admissibility / nesting if the code shows a
  distinct boundary)
- R3B.5 range — **COMPLETE AND REMOTELY CLOSED**
- R3B.6 index / field reads — **COMPLETE AND REMOTELY CLOSED**
- R3B.7 f-strings — **COMPLETE AND REMOTELY CLOSED**
- R3B.8 milestone completion audit — **COMPLETE (audit only)**; see
  `docs/engineering/B1R3B8_COMPLETION_AUDIT.md`

Use multi-reviewer analysis mainly at milestone closure, not after each
microphase.

## PRE-0.3 iteration ledger (2026-10-05)

Compact record of the Pre-0.3 Foundation super-transaction. Every iteration
was run under the mandatory Jev PRE/POST loop; evidence and dispositions only
(no chain-of-thought).

| # | Workstream | Question / action | Evidence before | Jev PRE | Verification | Jev POST | Commit |
|---|---|---|---|---|---|---|---|
| I1 | A/J/K/H/P/O reconnaissance | Inventory repo order, module identity, stdlib, limits, unsafe/panic, supply chain | two read-only subagent inventories + lead verification (falsified the `[0]`-index concern: test-only) | proceed 0.99 | − (read-only) | accept 0.99 | — |
| I2 | P/Host | Poisoned locks silently drop bytes? | host.rs:568-578 / lib.rs:195-199 confirm silent drop | proceed 0.92 (hidden-corruption p=0.07) | host 17 tests incl. new poison regression; playground execute 19 | commit 0.99 (risk 0.15) | `5edab88` |
| I3 | H | Triplicated `MAX_AST_DEPTH`, magic range/sleep limits | parse:163, run:34, check:152; run:1223 local const; host:370 magic | proceed 0.97 | boundaries 34; oracle 48; fmt/clippy clean | commit 1.0 | `2fd1433` |
| I4 | O | Unused `serde`/`arbitrary` deps | `cargo tree -i serde` = self-only; no derives | proceed 0.92 | all feature configs + 51 suites + fuzz nightly + locks | commit 0.94 | `3186340` |
| I5 | A/25 | Stale "production recursive" claims; divergent errors reference | 7 docs wrong; website duplicate | proceed 0.91 (spec-rewording risk flagged 0.51, mitigated by wording) | syntax_docs 3 (new sync test), grammar 6, website build/tests | commit 0.98 | `c1ffb18` |
| I6 | G | No evaluator performance baseline | bench.rs had shape guards only | proceed 1.0 | 8 stages measured linear; guards 14 pass; fmt/clippy | commit 0.99 | `9151df1` |
| I7 | C | Exception family/custom-exception foundation | throw/catch reality; TryResult snapshots | proceed 0.99 (preemption risk 0.4 → design-only markers strengthened) | doc cross-checked | commit 0.65→ (low, revised wording) | `34c5878` |
| I8 | D/E/F | Capability model, embedded-Python boundary, critical profile | Host trait, pyo3 inventory, resource table | proceed 0.96 | all claims cross-checked to source | accept 0.95 | `8ac9b93` |
| I9 | K/J/L/M/N | Stdlib coverage gaps; module map; AI/Dart/quantum boundaries | inventory found untested surfaces; cycle-coverage suspicion falsified | proceed 0.92 | stdlib_coverage 11 (2 expectation fixes to match documented surface) | commit 0.94 | `a1a5aa4` |
| I10 | WASM | Remaining transport limits untested | args/project/key/name limits had no direct tests | commit scope approved | execute 22, virtual_project 24 (1 expectation corrected to E4020) | commit 0.99 | `eb1e339` |
| I11 | I | Diagnostic taxonomy; E4099/E5003 undocumented | 46 constants / 45 codes; doc gap | proceed 0.99 | sync test green; grammar 6 | (folded) | `d4becc3` |
| I12 | B | Runtime architecture record; state reconciliation | entry points verified with file:line | commit scope approved | production_routing exists; agent-state preflight OK | commit 0.96 | `5b74ebb` |
| I13 | Residuals/§21 | Dispose 5 residuals; deliberate divergence | fresh-wasm pattern + f-string sweeps: no traps | commit scope approved | 3 mutations detected and reverted byte-exact (sha256); wasm 63 checks | commit 0.93 | `0e34fed`, `ef920a8` |
| I14 | Validation/review | Full matrix + independent adversarial review | — | — | 52+52 suites, clippy×2, MSRV, fuzz, wasm 63, node, website, python, miri 117; review: claim not falsified, 7 minor doc corrections + 2 test-precision nits + 1 mirror-drift surface | accept 0.98 (corrections) | `1d38b49` |

Jev totals: PRE consultations: 13; POST classifications: 12; disagreements:
1 (I7: Jev 0.65 commit vs deterministic evidence that the doc was
design-only and non-normative — proceeded, strengthened the non-normative
markers first); Jev-driven extra investigations: 3 (I2 hidden-corruption
question, I5 spec-rewording caution, I7 preemption caution).

## PRE-0.3 adversarial re-audit + human syntax gate (2026-10-06)

Bootstrap (verified, not assumed): branch `rewrite/v3-rust`; local HEAD
`1102b23` (then `150befe` after the repair below); remote
`e576238f7adb52f6ab6d18431602e3fbc2bd3636`; ahead 15 / behind 0; range
`e576238..1102b23` intact; staged none; protected `.kilo/**` churn matches the
documented state; `.codex/**` not tracked; root `s` absent; frozen runtime
hashes byte-identical; tag `v0.2.1` = `3f5f8702`; declared version `0.2.1`.

| # | Workstream | Question / action | Evidence before | Jev PRE | Verification | Jev POST | Commit |
|---|---|---|---|---|---|---|---|
| A1 | Bootstrap | Is the checkpoint true in Git? | local/remote SHAs, reflog linear, frozen hashes, tags | proceed 0.85 (divergence 0.21, defect 0.26) | all 18 audited claims checked; 0 production defects | accept | — |
| A2 | F1 in-range defect | Merged doc-comment delimiter `tests/host.rs:383` | introduced by `2fd1433`; single occurrence | proceed 0.97 | fmt clean; host 19/19 | accept | `150befe` |
| A3 | F2 bare-config hygiene | 13 tests in 8 suites fail bare `--no-default-features` | every failing file unchanged since remote `e576238`; canonical configs 52/52 | future_work 0.99 | reproduced (exit 101); recorded as TD-20 | accept | (doc only, below) |
| A4 | Routing mutation | Does the tripwire catch a rerouted seam? | `src/lib.rs:443` temporarily → `interp.run` | reuse A1 gate | `production_routing` FAILED (2≠3); reverted, sha256 byte-identical | accept | (reverted) |
| A5 | Decision package | Verify candidate claims by execution | 21 probes on the built CLI | A/B `needs_more` 0.60, span 0.65 → revised claims | A and B already constructible; nominal cross-module identity verified; C needs new grammar; catch-selection/raise-site-span are the real gaps | accept (revised) | `docs/engineering/EXCEPTION_SYNTAX_DECISION_PACKAGE.md` |

Validation executed in this pass: all-features 52/52 suites; canonical
`--no-default-features --features cli,repl,json,regex,time` 52/52; evaluator
oracle 228/228 differential; fresh-WASM boundary 63/63; syntax conformance
43/43 zero imports; browser 66; worker 12; multi-file 42; cache 7; `fmt`
clean; clippy `--all-features` and bare both `-D warnings` clean; MSRV
`+1.83.0 check --all-features` green; nightly fuzz `check --all-targets`
green; Miri 117/117; four fuzz targets ~890k runs clean; website build + tests
green.

**Outcome A stands: no public exception syntax was chosen or implemented;
E1–E6 remain the human gate.** The verified, refined decision package is
`docs/engineering/EXCEPTION_SYNTAX_DECISION_PACKAGE.md`; it supersedes the
prior session's untracked `plans/EXCEPTION_SYNTAX_DECISION_PACKAGE.md`
(sha256 `647e0e24…`; its content is preserved as a strict subset). TD-20
records the pre-existing bare-config test-hygiene gap (not repaired: outside
the audited range and every affected configuration is unsupported/
documented-against).

Independent adversarial review (fresh read-only session, 2026-10-05):
completion claim **not falsified** across 13 attack areas (frozen artifacts,
protected state, commit coherence, uncommitted state, no-push, doc claims,
resource contracts, poison soundness, semantic safety, test quality, oracle
golden, dual authority, validation honesty). Findings: 7 minor
documentation-accuracy corrections, all applied in this iteration (TryResult
variant names, FrameBoundary location, entry-point citation, taxonomy
sampling wording, PERFORMANCE min-vs-average wording, stale "53" counts,
state-ledger pointer). One pre-existing non-blocking note retained: the
Playground `project.js` mirrors four transport limits without a mechanical
sync test (documented as non-authoritative; runtime refusal is the decision).



| # | Residual | Disposition |
|---|---|---|
| 1 | Real browser/Worker validation of a fresh machine-backed runtime | **Publication-gated** (unchanged). Frozen `playground/runtimes/**` is immutable; the fresh-wasm Node boundary is pinned at 63 checks; the Chromium/Worker matrix closes only when a new runtime is published (human-gated). |
| 2 | `bind_pattern`/`match_pattern` bounded host recursion | **Verified safe and pinned.** Fresh-wasm calibration: deep pattern @700 binds without trap; @766 is structured `E1015`. AST-bounded (`MAX_AST_DEPTH` + substrate parser budget). Recorded in `docs/engineering/RUNTIME_ARCHITECTURE.md` §3. |
| 3 | f-string sub-parser budget composition | **Investigated empirically; no trap found.** Additive frontier sweep (outer/inner grouping, two-level nested f-strings) on fresh wasm is ok-or-`E1015`; grammar bounds nesting depth. Now pinned by 10 fresh-wasm checks in `playground/tests/node/b1_boundary.test.mjs` so a future parser-frame change cannot reintroduce a trap silently. |
| 4 | Poisoned-mutex silent ignore in `BrowserHost` | **FIXED** in `5edab88`: all output/result locks recover via `PoisonError::into_inner`; regression test deliberately poisons the buffer and asserts bytes survive. |
| 5 | Recursive-engine removal | **Human-gated** (B-1R8). The recursive evaluator remains the differential reference (`execution_recursive*`, oracle `engines_agree`); no production entry reaches it (`tests/production_routing.rs`). |

## Exact next action

0. **Aura 0.3 Keystone final collection model + typed-JSON API (2026-10-08,
   LOCAL, UNPUSHED).** Local tip is **25 ahead / 0 behind** remote `b221e6a9`.
   The campaign commits: `d5bb2fbd` remove general semicolon statement/item
   sequencing (human override; `;` reserved for the Array type); `49ef5298`
   project dynamic ranges as `range_like`/`range`; `36c3d49f` archive historical
   docs; `d0ff3076` implement distinct `List`/`Array`/`Tuple`/`Set`/`Map`
   identities end-to-end (`[T; N]` array type, contextual bracket-literal
   realization, no implicit List↔Array conversion, real Tuple, Set with
   `SetLike`); `86eb6925` finalize the docs (ADR-0005, spec §21, grammar +
   website mirrors) and add the observable sharded validator
   `scripts/validate.py`; `ce8deb0a` implement the final
   `json_decode_as(text, Type)` direct-type API (new `Expr::TypeRef`, canonical
   type grammar, static result type, string form kept as a normalized
   compatibility spelling). **The collection model and typed-JSON API are now
   implemented**, superseding the earlier "no tuple / no array / no set" and
   string-type decisions. Frozen runtimes verified byte-identical; `v0.2.1`
   unmoved. **Nothing pushed.** **Exact next action:** the human decides whether
   to authorize a push of the 25-commit range (a push deploys the site —
   confirm) and reviews the implemented collection model and typed-JSON API for
   any change of decision. Do not push without explicit authorization.

1. B-1 and the post-B1 runtime/WASM edge closure are **remotely closed**. The
   Pre-0.3 Foundation super-transaction and the 2026-10-06 adversarial re-audit
   are **complete and pushed** through `b221e6a9` (the former local range
   `e576238..150befe` is now on the remote). The exception feature is
   **decided and implemented**: RFC 0001 (`docs/rfcs/0001-exception-catch-selection.md`)
   is **Accepted** — the record of the Keystone human authorization — and
   pattern-based catch selection, raise-site spans, and the reserved `Aura`
   namespace are implemented and regression-tested (`tests/catch_syntax.rs`);
   `KEYSTONE_SCOPE_MANIFEST.md` §G2 records the same disposition.
   `docs/engineering/EXCEPTION_SYNTAX_DECISION_PACKAGE.md` is retained as the
   **superseded historical** decision package (it records Outcome A, the state
   that existed *before* the authorization); it is not current authority.
2. The Keystone closure range is pushed through `b221e6a9`. Only the 2-commit
   parser wasm-safety repair (`04783231`, `e7505408`) is unpushed; pushing it
   requires explicit human authorization.
3. B-1R8 (remove the recursive engine and the oracle switch) and any runtime
   publication remain separately human-gated.
4. Keep frozen runtimes, `v0.2.1`, and `.kilo/**` untouched.
5. Publication-gated validation debt: the Chromium/Worker boundary against a
   machine-backed runtime awaits a published runtime (release-gated); the
   fresh-wasm boundary is already pinned in Node (63 checks).
6. Pre-0.3 decisions awaiting the human: exception syntax/catching (E1–E6 in
   `docs/engineering/EXCEPTION_ARCHITECTURE.md` §8) and embedded-CPython
   packaging (P1–P5 in `docs/engineering/EMBEDDED_PYTHON_ARCHITECTURE.md` §9).
   Bare-config test-feature gating is tracked as TD-20 (low, v1.1).

## Stop conditions

Stop and report instead of improvising if:

- Git reality contradicts `AGENT_STATE.md`;
- production behavior would change;
- the language contract and current behavior disagree in a way that needs a
  semantic decision (produce a decision package instead);
- a frozen artifact would be touched;
- a genuine oracle blind spot is found that cannot be closed within the slice.
