#!/usr/bin/env bash
# Emit a CycloneDX-style software bill of materials for the shipped artifacts
# from `cargo metadata` (V1 supply-chain policy, section 53). No extra tooling
# is required, so this runs on any machine with a Rust toolchain and is
# reproducible from the committed lockfiles.
#
# Usage:
#   scripts/sbom.sh > sbom.json
#
# Scope: the core CLI/REPL workspace and the Playground WebAssembly runtime
# workspace (a separate workspace). Every resolved dependency is listed with
# name, version, and license; local workspace/path packages are tagged
# `aura:local=true`. Output is deterministic (sorted by name then version) for
# a fixed lockfile, so it can be diffed and attached to a release.

set -euo pipefail

REPO_ROOT=$(git rev-parse --show-toplevel)
cd "$REPO_ROOT"

META=$(mktemp)
RUNTIME_META=$(mktemp)
trap 'rm -f "$META" "$RUNTIME_META"' EXIT
cargo metadata --locked --format-version 1 > "$META"
cargo metadata --locked --manifest-path playground/runtime/Cargo.toml \
    --format-version 1 > "$RUNTIME_META"

python3 - "$META" "$RUNTIME_META" <<'PY'
import json
import subprocess
import sys


def components_for(meta, seen):
    members = set(meta["workspace_members"])
    resolve = meta.get("resolve") or {}
    resolved = {n["id"] for n in resolve.get("nodes", [])} if resolve else None
    out = []
    for pkg in sorted(meta["packages"], key=lambda p: (p["name"], p["version"])):
        if resolved is not None and pkg["id"] not in resolved:
            continue
        key = (pkg["name"], pkg["version"])
        if key in seen:
            continue
        seen.add(key)
        is_local = pkg["id"] in members or pkg["source"] is None
        component = {
            "type": "library",
            "name": pkg["name"],
            "version": pkg["version"],
            "purl": f"pkg:cargo/{pkg['name']}@{pkg['version']}",
            "properties": [{"name": "aura:local", "value": "true" if is_local else "false"}],
        }
        lic = pkg.get("license")
        if lic:
            component["licenses"] = [{"license": {"name": lic}}]
        elif pkg.get("license_file"):
            component["licenses"] = [{"license": {"name": pkg["license_file"]}}]
        if pkg.get("description"):
            component["description"] = pkg["description"]
        out.append(component)
    return out


seen = set()
components = []
for path in sys.argv[1:]:
    with open(path) as fh:
        components.extend(components_for(json.load(fh), seen))
components.sort(key=lambda c: (c["name"], c["version"]))

version = subprocess.check_output(
    ["grep", "-m1", "^version = ", "Cargo.toml"], text=True
).split('"')[1]

sbom = {
    "bomFormat": "CycloneDX",
    "specVersion": "1.5",
    "version": 1,
    "metadata": {
        "component": {"type": "application", "name": "aura-lang", "version": version},
        "properties": [
            {"name": "aura:workspaces", "value": "core, playground/runtime"},
            {"name": "aura:componentCount", "value": str(len(components))},
        ],
    },
    "components": components,
}
json.dump(sbom, sys.stdout, indent=2)
sys.stdout.write("\n")
PY
