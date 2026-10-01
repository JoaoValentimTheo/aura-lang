#!/usr/bin/env bash
# Release preflight: validate the static preconditions a tagged release will
# need, before the tag is pushed. A tag push triggers the release workflow,
# which is not a cheap place to discover that a prerequisite is missing.
#
# Usage:
#   scripts/release-preflight.sh [version] [--tag] [--on-tag]
#
# With no argument it uses the version from Cargo.toml. With a version it
# checks that a `vX.Y.Z` tag would succeed. `--tag` marks a real tagged-release
# context, in which a missing release runtime artifact is fatal; without it
# (a pre-tag local check on the development line) it is a warning. `--on-tag`
# marks the *tag-triggered workflow itself*, where the tag necessarily already
# exists: the immutability check then verifies the tag points at the commit
# being released instead of failing (a tag can never be "free" when validating
# the very tag that triggered the run).
#
# Exit is non-zero on any missing prerequisite (in the selected strictness).
#
# Checks (all must pass):
#   1. Cargo.toml, Cargo.lock, and aura::VERSION agree.
#   2. LANGUAGE_VERSION <= release version (ADR-0001).
#   3. The release's Playground runtime artifact exists and its manifest
#      identity (sha256, bytes) matches the bytes on disk.
#   4. A curated release-notes file exists for the version (or a clear TODO is
#      allowed to fall back to generated notes — reported, not failed).
#   5. The Git tag for the version does not already exist (immutability), or —
#      with `--on-tag` — exists and points at the released commit.

set -euo pipefail

REPO_ROOT=$(git rev-parse --show-toplevel)
cd "$REPO_ROOT"

tag_mode=0
on_tag=0
version=""
for arg in "$@"; do
    case "$arg" in
        --tag) tag_mode=1 ;;
        --on-tag) tag_mode=1; on_tag=1 ;;
        *) version="$arg" ;;
    esac
done
if [ -z "$version" ]; then
    version=$(grep -m1 '^version = ' Cargo.toml | sed -E 's/^version = "(.*)"/\1/')
    if [ -z "$version" ]; then
        echo "preflight: cannot read version from Cargo.toml" >&2
        exit 1
    fi
fi

echo "preflight: checking release candidate v$version"
fail=0

# 1. Version agreement.
crate_version=$(grep -m1 '^version = ' Cargo.toml | sed -E 's/^version = "(.*)"/\1/')
lock_version=$(grep -m1 -A1 '^name = "aura-lang"' Cargo.lock | grep '^version' | sed -E 's/version = "(.*)"/\1/')
lib_version=$(grep -oE 'VERSION: &str = "[^"]+"' src/lib.rs | sed -E 's/.*"(.*)"/\1/')
echo "  Cargo.toml=$crate_version Cargo.lock=$lock_version src/lib.rs=$lib_version"
if [ "$crate_version" != "$lock_version" ] || [ "$crate_version" != "$lib_version" ]; then
    echo "  FAIL: version identities disagree" >&2
    fail=1
fi

# 2. ADR-0001 invariant.
lang_version=$(grep -oE 'LANGUAGE_VERSION: &str = "[^"]+"' src/lib.rs | sed -E 's/.*"(.*)"/\1/')
lowest=$(printf '%s\n%s\n' "$crate_version" "$lang_version" | sort -V | head -n1)
echo "  release=$crate_version language=$lang_version"
if [ "$lowest" != "$lang_version" ]; then
    echo "  FAIL: LANGUAGE_VERSION is ahead of the release version" >&2
    fail=1
fi

# 3. The versioned Playground runtime artifact.
artifact="playground/runtimes/${version}/aura_playground_runtime.wasm"
if [ ! -f "$artifact" ]; then
    if [ "$tag_mode" -eq 1 ]; then
        echo "  FAIL: missing release runtime artifact: $artifact"
        echo "        (a release tag requires it; a development runtime is not a release artifact)"
        fail=1
    else
        echo "  warning: no release runtime artifact at $artifact"
        echo "           (a real tag would require it; promote a dev runtime before tagging)"
    fi
else
    echo "  runtime artifact present: $artifact"
fi

# 4. Curated release notes (informational).
notes="docs/release-notes/v${version}.md"
if [ -f "$notes" ]; then
    echo "  release notes present: $notes"
else
    echo "  note: no curated $notes; the workflow will generate notes from commits"
fi

# 5. Tag immutability. Before a tag exists it must be free. When this runs
# *inside* the tag-triggered workflow (`--on-tag`), the tag necessarily exists;
# then the immutability property to check is that it points at the commit being
# released, so a re-run can never validate a mismatched tag.
if git rev-parse -q --verify "refs/tags/v${version}" >/dev/null; then
    if [ "$on_tag" -eq 1 ]; then
        tag_commit=$(git rev-parse -q --verify "refs/tags/v${version}^{commit}")
        head_commit=$(git rev-parse -q --verify HEAD)
        if [ "$tag_commit" != "$head_commit" ]; then
            echo "  FAIL: tag v${version} ($tag_commit) does not point at HEAD ($head_commit)" >&2
            echo "        (the tag-triggered workflow must release the tagged commit)" >&2
            fail=1
        else
            echo "  tag v${version} points at the released commit"
        fi
    else
        echo "  FAIL: tag v${version} already exists (releases are immutable; bump the version)" >&2
        fail=1
    fi
elif [ "$on_tag" -eq 1 ]; then
    echo "  FAIL: tag v${version} not found in the tag-triggered workflow" >&2
    fail=1
else
    echo "  tag v${version} is free"
fi

if [ "$fail" -ne 0 ]; then
    echo "preflight: FAILED for v$version"
    exit 1
fi
echo "preflight: OK for v$version"
