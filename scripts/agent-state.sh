#!/bin/sh

set -eu

repo_root=$(git rev-parse --show-toplevel)
cd "$repo_root"

branch=$(git branch --show-current)
local_head=$(git rev-parse HEAD)
remote_head=$(git rev-parse origin/rewrite/v3-rust 2>/dev/null || printf '%s' UNAVAILABLE)

if git diff --cached --quiet --; then
    staged=NO
else
    staged=YES
fi

if git diff --quiet -- && git diff --cached --quiet -- && test -z "$(git ls-files --others --exclude-standard)"; then
    dirty=NO
else
    dirty=YES
fi

current_track=$(sed -n '/^## Current Track$/{n;n;p;q;}' AGENT_STATE.md)
current_phase=$(sed -n '/^## Verified Closed$/,/^## /{/FSM-P6/p;}' AGENT_STATE.md | head -n 1 | sed 's/^- //')
audit3=$(sed -n '/^## Human Decisions$/,/^## /{/ADR-0004/p;}' AGENT_STATE.md | head -n 1 | sed 's/^- //')

codex_state=CLEAN
if ! git diff --quiet -- .codex/config.toml; then
    codex_state=MODIFIED
fi

writer=$(sed -n '/^## Writer$/,/^## /{/^NONE$/p;/^WEB-GPT$/p;}' AGENT_STATE.md | head -n 1)
if test -z "$writer"; then
    writer=UNKNOWN
fi

s_state=ABSENT
if test -e s; then
    s_bytes=$(wc -c < s | tr -d ' ')
    if git ls-files --error-unmatch s >/dev/null 2>&1; then
        s_state="TRACKED:${s_bytes}B"
    else
        s_state="UNTRACKED:${s_bytes}B"
    fi
fi

# Minimal authority guard: the resume set must exist and be non-empty so a
# session cannot silently lose its current-state handoff.
authority=MISSING
if test -s AGENTS.md && test -s AGENT_STATE.md \
    && test -s docs/engineering/CURRENT_HANDOFF.md; then
    authority=OK
fi

printf '%s\n' 'AURA AGENT PREFLIGHT'
printf 'BRANCH=%s\n' "$branch"
printf 'LOCAL_HEAD=%s\n' "$local_head"
printf 'REMOTE_HEAD=%s\n' "$remote_head"
printf 'STAGED=%s\n' "$staged"
printf 'DIRTY=%s\n' "$dirty"
printf 'CURRENT_TRACK=%s\n' "$current_track"
printf 'CURRENT_PHASE=%s\n' "$current_phase"
printf 'AUDIT3=%s\n' "$audit3"
printf 'PROJECT_CODEX_CONFIG=%s\n' "$codex_state"
printf 'WRITER=%s\n' "$writer"
printf 'S_STATE=%s\n' "$s_state"
printf 'AUTHORITY=%s\n' "$authority"
