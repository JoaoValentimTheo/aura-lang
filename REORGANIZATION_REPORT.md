# Aura Reorganization Report

**Date:** 2026-09-20
**Branch:** `chore/reorganize-aura`
**Scope:** Documentation consistency, numbers alignment, metadata cleanup

---

## Baseline vs Final

| Metric | Baseline | Final | Change |
|--------|----------|-------|--------|
| Tests passing | 4,431 | 4,431 | No change |
| Tests skipped | 10 | 10 | No change |
| Ruff errors | 0 | 0 | No change |
| Mypy errors | 0 | 0 | No change |
| CLI commands | 17 | 17 | No change |
| aura check examples | All pass | All pass | No change |
| aura transpile diff | Empty | Empty | No change |

**Conclusion:** No behavioral changes. All changes are documentation and metadata only.

---

## Sources of Truth Table

| Fact | Source of Truth | Command |
|------|-----------------|---------|
| Current version | pyproject.toml + aura/__init__.py | `grep version pyproject.toml` |
| Number of CLI commands | aura/cli.py (build_parser) | `aura --help` |
| Number of stdlib modules | aura/stdlib/__init__.py (__all__) | `ls aura/stdlib/*.py \| wc -l` |
| Number of stdlib functions | aura/stdlib/*.py | `grep -E "^def " aura/stdlib/*.py \| wc -l` |
| Number of tests | pytest | `pytest -q 2>&1 \| tail -1` |
| Python minimum version | pyproject.toml (requires-python) | `grep requires-python pyproject.toml` |
| Syntax freeze version | CHANGELOG.md (0.2.0a1 entry) | Manual verification |
| Grammar source of truth | docs/language-reference/grammar.md | Stated in grammar.md header |
| Diagnostics source of truth | docs/ERRORS.md | Stated in ERRORS.md header |

---

## Changes Made

### Phase 1: Sources of Truth

**Version numbers updated (0.2.0a6 to 0.2.0a7) in 14 files:**
- README.md, docs/index.md
- docs/language-reference/index.md, index.pt_BR.md
- docs/language-reference/types.md, types.pt_BR.md
- docs/learn/00-introduction.md, 00-introduction.pt_BR.md
- docs/learn/01-installation.md, 01-installation.pt_BR.md
- docs/learn/index.md, index.pt_BR.md

**LSP/REPL version strings:**
- aura/lsp/server.py: 0.1.0 to 0.2.0
- aura/repl/engine.py: v0.5 to v0.2

**Numbers aligned across docs:**
- CLI commands: COMPLETENESS.md 14 to 17
- Stdlib modules: README 18 to 17, COMPLETENESS 10 to 17
- Stdlib functions: COMPLETENESS/DESIGN 210+ to 360+
- Test count: README 3,000+ to 4,400+, COMPLETENESS 2,700+ to 4,400+

**PyPI status contradictions resolved in COMPLETENESS.md:**
- Removed conflicting "pending" vs "published" statements
- Unified to "published on PyPI as aura-language"

**Async I/O status corrected in COMPLETENESS.md:**
- io module: 85% to 90% (native async helpers implemented)
- Async I/O: 30% to 70% (native helpers implemented)

### Phase 2: Repository Structure

**Root shims documented:**
- CONTRIBUTING.md: Added project structure section
- Documented that shims are not in wheel

**Training directory documented:**
- DESIGN.md: Added training/ to project tree

### Phase 5: Documentation

**DESIGN.md:**
- Project tree lists all 17 stdlib modules
- Macro System section documents compile-time macros (12 macros)
- CLI Commands table complete (17 commands)

**COMPLETENESS.md:**
- Fixed all contradictions
- Updated all counts and percentages

**docs/index.md:**
- Clearer navigation with reading order

### Phase 7: Project and Release

**CHANGELOG.md:** Added Unreleased section

**CONTRIBUTING.md:**
- Test categories and run commands
- Project structure and shim docs
- Syntax freeze note
- AUP and diagnostic contribution guides

**pyproject.toml:**
- Added classifiers (Python 3.10-3.13, MIT, OS Independent)
- Added keywords

---

## Findings Out of Scope (Not Changed)

These are historical documents or bugs that should not be fixed during
reorganization:

1. **AUDIT.md** (Finding E): Already has "Historical document" banner.
   Contains references to removed syntax (fn, init, implements, ANTLR).
   Left as-is since it is a historical record.

2. **AUDIT.md numbers** (Finding A): Historical counts (7 modules, 138+
   functions, 4,798 tests, 15 commands) are accurate for the time of
   the audit. Not changed.

3. **CHANGELOG.md 0.2.0a6 entry**: Says "4313 tests passing" which was
   accurate at time of release. Not changed.

4. **6,609-file corpus** (Finding J): Referenced in COMPLETENESS and
   AUDIT. The corpus was generated and later removed. COMPLETENESS
   reference was updated to remove it; AUDIT references left as
   historical record.

5. **grammar.md "0.1.0a5 onward"** (Finding F): This refers to when
   the grammar was written, not the syntax freeze date. The syntax
   freeze is 0.2.0a1. No conflict.

6. **tools/deps.py default version "0.1.0"**: This is intentional --
   it is the default version for new projects created by `aura init`.
   Not changed.

7. **PHASE8_MACHINE_CODE_RESEARCH.md**: Historical research document.
   Not changed.

---

## Decisions Required from Maintainer

1. **AUDIT.md**: Should the "Historical document" banner be strengthened
   with explicit warnings about removed syntax examples?

2. **SECURITY.md supported versions table**: Currently shows only
   0.1.0a5 as supported. Should it be updated to show current version?

3. **GitHub About field**: Suggested description (350 chars max):
   "Aura: a gradually-typed language that transpiles to Python. One spelling
   per construct, no synonyms. The whole Python ecosystem is one import away."
   URL: https://joaovalentimtheo.github.io/aura-lang/
   Topics: aura, transpiler, gradually-typed, python, language, compiler,
   source-to-source, developer-tools

4. **Root shims**: The root-level parser/, transpiler/, stdlib/, repl/,
   tools/ shims are development conveniences. Should they eventually be
   removed, or kept permanently for backward compatibility?

5. **training/ directory**: Currently undocumented in README. Should it
   be added to the project description?

---

## Risk Assessment

**Risk: None.** All changes are documentation and metadata only.
No code behavior was modified. No syntax was changed. No tests were
added or removed.

**Revert strategy:** `git revert` of the single commit on this branch.

---

## Verification Evidence

```
pytest -q: 4431 passed, 10 skipped, 2 warnings
ruff check aura/: All checks passed!
mypy aura/: Success: no issues found in 48 source files
aura --help: 17 commands listed
aura check examples/*.aura: All 16 examples pass
aura transpile examples/hello.aura: diff empty
wheel install: aura 0.2.0a7, runs correctly
sdist install: aura 0.2.0a7, runs correctly
```
