---
layout: default
title: "RFCs"
nav_order: 12
---

# RFCs (Request for Comments)

This directory tracks proposals for post-1.0 syntax and language changes.
Since Aura's syntax is frozen until 1.0, these proposals are recorded for
future consideration but will not be implemented before the stable release.

---

## Process

1. **Open an issue** using the [Feature request / RFC](https://github.com/JoaoValentimTheo/aura-lang/issues/new?template=feature_rfc.md) template.
   Label it `post-1.0-syntax` if it requires a grammar change.

2. **Discussion**: the community discusses the proposal in the issue.

3. **Decision**: the maintainer accepts, rejects defers the proposal.

4. **If accepted pre-1.0**: the proposal is recorded here as a numbered RFC
   file (`docs/rfcs/NNNN-title.md`) and linked from the issue.

5. **If accepted post-1.0**: the proposal is implemented directly, with the
   issue as the tracking record.

---

## RFC format

Each RFC file should contain:

```markdown
# RFC NNNN: Title

- **Status:** Draft / Accepted / Rejected / Deferred
- **Created:** YYYY-MM-DD
- **Issue:** #NNN
- **Author:** ...

## Summary

## Motivation

## Detailed design

## Alternatives considered

## Impact
```

---

## Index

| RFC | Title | Status |
|-----|-------|--------|
| — | No RFCs yet | — |

Post-1.0 syntax changes will be listed here as they are accepted.
