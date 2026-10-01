// Release-manifest generator tests (V1 release policy, section 98).
//
// `scripts/release-manifest.sh` emits a machine-readable description of what a
// release is expected to contain. These tests verify the generator produces a
// parseable manifest whose identities agree with the repository's canonical
// sources (Cargo.toml, src/lib.rs, the runtime manifest) and preserve the
// ADR-0001 invariant `language_version <= release_version`.
//
// Usage: node playground/tests/node/release_manifest.test.mjs

import { readFileSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const repo = resolve(here, "../../..");

let passed = 0;
let failed = 0;
function check(name, cond, detail) {
  if (cond) passed += 1;
  else {
    failed += 1;
    console.error(`FAIL ${name}${detail ? `: ${detail}` : ""}`);
  }
}

const out = execFileSync(join(repo, "scripts/release-manifest.sh"), {
  cwd: repo,
  encoding: "utf8",
});
const manifest = JSON.parse(out);

const cargoVersion = readFileSync(join(repo, "Cargo.toml"), "utf8").match(
  /^version = "(.*)"/m,
)[1];
const libRs = readFileSync(join(repo, "src/lib.rs"), "utf8");
const langVersion = libRs.match(/LANGUAGE_VERSION: &str = "(.*)"/)[1];
const runtimeManifest = JSON.parse(
  readFileSync(join(repo, "playground/runtimes/manifest.json"), "utf8"),
);

check(
  "release_version matches Cargo.toml",
  manifest.release_version === cargoVersion,
  `${manifest.release_version} != ${cargoVersion}`,
);
check(
  "language_version matches src/lib.rs",
  manifest.language_version === langVersion,
  `${manifest.language_version} != ${langVersion}`,
);
check("commit is a 40-hex SHA", /^[0-9a-f]{40}$/.test(manifest.commit));
check(
  "dependency_lock_sha256 is a 64-hex SHA",
  /^[0-9a-f]{64}$/.test(manifest.dependency_lock_sha256),
);
check(
  "playground_api_version matches the runtime manifest",
  manifest.playground_api_version === runtimeManifest.playground_api_version,
);

// ADR-0001: language must never be ahead of release.
const vnum = (s) => s.split(".").map(Number);
const [lr, ll] = [vnum(manifest.release_version), vnum(manifest.language_version)];
const langLeRelease =
  ll[0] < lr[0] ||
  (ll[0] === lr[0] && (ll[1] < lr[1] || (ll[1] === lr[1] && (ll[2] ?? 0) <= (lr[2] ?? 0))));
check("language_version <= release_version", langLeRelease);

check(
  "current_runtime matches the manifest current pointer",
  manifest.current_runtime && manifest.current_runtime.id === runtimeManifest.current,
);
check(
  "every runtime entry carries an id and channel",
  manifest.runtimes.every((r) => r.id && r.channel),
);
check(
  "available runtimes carry an artifact and sha256",
  manifest.runtimes
    .filter((r) => r.available)
    .every((r) => typeof r.artifact === "string" && /^[0-9a-f]{64}$/.test(r.sha256)),
);
check(
  "supported_python lists the ADR-0003 tested Linux lines",
  ["3.10", "3.11", "3.12", "3.13"].every((v) =>
    manifest.supported_python.some((p) => p.platform === "linux" && p.python === v),
  ),
);
check(
  "supported_python lists macOS 3.12",
  manifest.supported_python.some((p) => p.platform === "macos" && p.python === "3.12"),
);
check(
  "supported_python lists Windows 3.12",
  manifest.supported_python.some((p) => p.platform === "windows" && p.python === "3.12"),
);

console.log(`release-manifest: ${passed} passed, ${failed} failed`);
if (failed > 0) process.exit(1);
