---
description: Reconstruct current Aura repository state without editing.
agent: aura-orchestrator
---

Reconstruct the current Aura state from the repository. Inspect branch, HEAD,
`git status`, the relevant diff, recent log, the runtime manifest, and the
relevant artifact hashes (frozen `0.0.2`, current development runtime). Do not
edit.

Return a concise checkpoint and identify any discrepancy with the context you
were given. If the human supplied a HEAD or version, verify it against the
repository rather than trusting it.
