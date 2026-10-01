#!/usr/bin/env bash
# Emit a machine-readable release manifest for the current tree (V1 release
# policy, section 98). It records the identities a release is expected to
# describe so a downstream consumer can verify them without reading prose.
#
# Usage:
#   scripts/release-manifest.sh > release-manifest.json
#
# Fields:
#   release_version         package version (Cargo.toml / aura::VERSION)
#   language_version        aura::LANGUAGE_VERSION (<= release_version, ADR-0001)
#   commit                  the current git HEAD
#   playground_api_version  the Playground manifest API version
#   current_runtime         the manifest's current runtime id + its identities
#   supported_python        the ADR-0003 CI-tested CPython matrix
#   runtimes                every runtime artifact identity from the manifest

set -euo pipefail

REPO_ROOT=$(git rev-parse --show-toplevel)
cd "$REPO_ROOT"

release_version=$(grep -m1 '^version = ' Cargo.toml | sed -E 's/^version = "(.*)"/\1/')
language_version=$(grep -oE 'LANGUAGE_VERSION: &str = "[^"]+"' src/lib.rs | sed -E 's/.*"(.*)"/\1/')
commit=$(git rev-parse HEAD)
# A hash of the dependency closures: the exact pinned inputs a build used. This
# is self-contained provenance (no external attestation service required) and
# lets a consumer tie an artifact to the lockfiles that produced it.
if command -v sha256sum >/dev/null 2>&1; then
    lock_hash=$(cat Cargo.lock playground/runtime/Cargo.lock | sha256sum | cut -d' ' -f1)
else
    lock_hash=$(cat Cargo.lock playground/runtime/Cargo.lock | shasum -a 256 | cut -d' ' -f1)
fi

# Invariant (ADR-0001): language must not be ahead of release.
lowest=$(printf '%s\n%s\n' "$release_version" "$language_version" | sort -V | head -n1)
if [ "$lowest" != "$language_version" ]; then
    echo "LANGUAGE_VERSION ($language_version) is ahead of release version ($release_version)" >&2
    exit 1
fi

python3 - "$release_version" "$language_version" "$commit" "$lock_hash" <<'PY'
import json
import re
import sys

release_version, language_version, commit, lock_hash = sys.argv[1:5]

with open("playground/runtimes/manifest.json") as f:
    runtime_manifest = json.load(f)

runtimes = [
    {
        "id": v["id"],
        "channel": v.get("channel"),
        "language_version": v.get("language_version"),
        "host_abi_version": v.get("host_abi_version"),
        "available": v.get("available"),
        "artifact": v.get("artifact"),
        "sha256": v.get("sha256"),
        "bytes": v.get("bytes"),
    }
    for v in runtime_manifest["versions"]
]

current_id = runtime_manifest.get("current")
current = next((r for r in runtimes if r["id"] == current_id), None)

# ADR-0003: surface exactly the CI-tested tier. The doc states the matrix as
# "Linux 3.10-3.13" and "macOS 3.12" on the TESTED line; parse the set of
# `platform X.Y` tokens only from that line so best-effort tiers are excluded.
supported_python = []
tested_line = ""
with open("docs/CPYTHON_COMPATIBILITY_TARGET.md") as f:
    for line in f:
        if "TESTED" in line:
            tested_line = line
            break
for platform, pattern in (
    ("linux", r"Linux[^;|]*"),
    ("macos", r"macOS[^;|]*"),
    ("windows", r"Windows[^;|]*"),
):
    segment = re.search(pattern, tested_line)
    if segment:
        for ver in re.findall(r"3\.\d+", segment.group(0)):
            supported_python.append({"platform": platform, "python": ver})

manifest = {
    "release_version": release_version,
    "language_version": language_version,
    "commit": commit,
    "dependency_lock_sha256": lock_hash,
    "playground_api_version": runtime_manifest.get("playground_api_version"),
    "current_runtime": current,
    "supported_python": supported_python,
    "runtimes": runtimes,
}
print(json.dumps(manifest, indent=2))
PY
