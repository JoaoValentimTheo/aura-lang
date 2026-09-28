#!/usr/bin/env bash
# Aura Agent Manager setup script.
# Runs once when an Agent Manager worktree is created, imported, or promoted,
# from the worktree directory, with WORKTREE_PATH and REPO_PATH set.
set -euo pipefail

echo "[aura-setup] worktree: ${WORKTREE_PATH:-<unset>}"
echo "[aura-setup] repo:     ${REPO_PATH:-<unset>}"

# Copy non-secret local env files if the main repo has them and the worktree
# does not. Never overwrite an existing file.
for f in .env .env.local; do
  if [ -f "$REPO_PATH/$f" ] && [ ! -f "$WORKTREE_PATH/$f" ]; then
    cp "$REPO_PATH/$f" "$WORKTREE_PATH/$f"
    echo "[aura-setup] copied $f"
  fi
done

# Warm the Rust build cache so the first validation run is not cold. Failures
# are non-fatal: the worktree remains available for inspection.
if command -v cargo >/dev/null 2>&1; then
  echo "[aura-setup] cargo fetch"
  (cd "$WORKTREE_PATH" && cargo fetch --locked) || echo "[aura-setup] cargo fetch skipped"
fi

echo "[aura-setup] done"
