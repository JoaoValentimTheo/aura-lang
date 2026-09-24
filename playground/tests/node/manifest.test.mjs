// Versioning and immutability tests for the Playground runtime manifest.
//
// Verifies that:
//   * every available version resolves to an artifact whose SHA-256 matches
//     the manifest (immutable identity);
//   * the manifest's declared Host ABI version matches the artifact's actual
//     exported ABI (the entry is real, not decorative);
//   * the artifact's own runtime version matches the manifest entry;
//   * historical entries cannot be silently overwritten (build.mjs --check);
//   * the frozen 0.0.1 entry is honest: it is marked unavailable and has no
//     artifact.
//
// Usage: node playground/tests/node/manifest.test.mjs

import { createHash } from "node:crypto";
import { readFileSync, existsSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const playground = resolve(here, "../..");
const runtimesDir = join(playground, "runtimes");
const manifest = JSON.parse(readFileSync(join(runtimesDir, "manifest.json"), "utf8"));

const { AuraRuntime } = await import(join(playground, "web", "runtime.mjs"));

let passed = 0;
let failed = 0;
function check(name, cond, detail) {
  if (cond) passed += 1;
  else {
    failed += 1;
    console.error(`FAIL ${name}${detail ? `: ${detail}` : ""}`);
  }
}

check("manifest has a current pointer", typeof manifest.current === "string");
check(
  "current points at an available version",
  manifest.versions.some((v) => v.id === manifest.current && v.available),
);

// Every available version must resolve, hash-match, and ABI-match.
for (const entry of manifest.versions) {
  if (!entry.available) {
    check(`unavailable ${entry.id} has no artifact`, entry.artifact === null);
    check(`unavailable ${entry.id} states why`, typeof entry.reason === "string");
    continue;
  }
  const path = join(runtimesDir, entry.artifact);
  check(`artifact exists for ${entry.id}`, existsSync(path));
  const bytes = readFileSync(path);
  const hash = createHash("sha256").update(bytes).digest("hex");
  check(`sha256 matches for ${entry.id}`, hash === entry.sha256, `${hash} != ${entry.sha256}`);
  check(`size matches for ${entry.id}`, bytes.byteLength === entry.bytes);

  const runtime = await AuraRuntime.fromBytes(new Uint8Array(bytes), entry.id);
  check(
    `ABI matches for ${entry.id}`,
    runtime.abiVersion === entry.host_abi_version,
    `${runtime.abiVersion} != ${entry.host_abi_version}`,
  );
  check(
    `runtime version matches for ${entry.id}`,
    runtime.runtimeVersion === entry.runtime_version,
    `${runtime.runtimeVersion} != ${entry.runtime_version}`,
  );
  check(
    `language version matches for ${entry.id}`,
    runtime.languageVersion === entry.language_version,
    `${runtime.languageVersion} != ${entry.language_version}`,
  );
  // A version entry records the release it belongs to, separately from the
  // language version, so the two cannot be conflated.
  check(`release version present for ${entry.id}`, typeof entry.release_version === "string");
  check(
    `runtime version equals release version for ${entry.id}`,
    entry.runtime_version === entry.release_version,
    `${entry.runtime_version} != ${entry.release_version}`,
  );
}

// The frozen 0.0.1 entry must be present and honest.
const v001 = manifest.versions.find((v) => v.id === "0.0.1");
check("0.0.1 entry present", !!v001);
check("0.0.1 is marked unavailable", v001 && v001.available === false);

// `build.mjs --check` re-derives hashes from disk and fails on drift.
try {
  execFileSync("node", [join(playground, "build.mjs"), "--check"], { stdio: "pipe" });
  check("build --check passes", true);
} catch (err) {
  check("build --check passes", false, String(err.stdout || err.message));
}

console.log(`\nManifest: ${passed} passed, ${failed} failed`);
process.exit(failed === 0 ? 0 : 1);
