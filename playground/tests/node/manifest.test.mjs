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

  const runtime = await AuraRuntime.fromBytes(new Uint8Array(bytes), entry.id, {
    expectedSha256: entry.sha256,
  });
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
  // Codename metadata (Keystone §34): the field exists on every entry. A
  // string names a human-approved release line; null is the explicit
  // "awaiting a human choice" placeholder, never an invented name.
  check(
    `codename field present for ${entry.id}`,
    entry.codename === null || typeof entry.codename === "string",
    JSON.stringify(entry.codename),
  );
  if (entry.channel === "development") {
    // A development runtime is on a release line but is not itself the
    // release: its identity is a pre-release of that line, never equal to it.
    check(
      `development runtime ${entry.id} is a pre-release of its release line`,
      entry.id !== entry.release_version && entry.id.startsWith(`${entry.release_version}-`),
      `${entry.id} vs ${entry.release_version}`,
    );
  } else {
    check(
      `runtime version equals release version for ${entry.id}`,
      entry.runtime_version === entry.release_version,
      `${entry.runtime_version} != ${entry.release_version}`,
    );
  }
}

// The frozen release artifacts are pinned and must never move. Recompute each
// release-channel hash independently of the manifest so an edited manifest
// cannot hide an edited artifact.
const PINNED_RELEASES = {
  "0.0.2": {
    sha256: "5a4ad3f7e3f786164d65df437d607e7ddd5e25947ea2c8dd9b436a5490b334ed",
    bytes: 1366621,
  },
};
for (const [id, want] of Object.entries(PINNED_RELEASES)) {
  const p = join(runtimesDir, id, "aura_playground_runtime.wasm");
  check(`frozen release ${id} present`, existsSync(p));
  if (existsSync(p)) {
    const bytes = readFileSync(p);
    const hash = createHash("sha256").update(bytes).digest("hex");
    check(`frozen release ${id} sha256 unchanged`, hash === want.sha256, `${hash} != ${want.sha256}`);
    check(`frozen release ${id} size unchanged`, bytes.byteLength === want.bytes);
  }
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

// ---------------------------------------------------------------------------
// Codename / channel metadata (Keystone §34)
// ---------------------------------------------------------------------------

// A codename names a release *line* and must never be invented for a line the
// human has not named. The 0.3 line is the human-approved "Keystone" line; its
// published 0.3.1 release runtime must carry that codename. Pre-Keystone lines
// carry no invented codename.
let codenameFailures = 0;
for (const entry of manifest.versions) {
  const line = (entry.release_version || entry.id).split(".").slice(0, 2).join(".");
  if (line === "0.3") {
    if (entry.codename !== "Keystone") {
      codenameFailures += 1;
      console.error(`FAIL ${entry.id}: the 0.3 line must carry codename "Keystone"`);
    }
  } else if (entry.codename !== null && typeof entry.codename !== "string") {
    codenameFailures += 1;
    console.error(`FAIL ${entry.id}: codename must be a string or null`);
  }
}
check("the Keystone line carries its codename and others do not invent one", codenameFailures === 0);

// Keystone 0.3.1 is the current published release and must be present as an
// available release-channel entry carrying the Keystone codename.
const keystone = manifest.versions.find((v) => v.id === "0.3.1");
check("the 0.3.1 Keystone release runtime is published", !!keystone && keystone.available);
check(
  "the 0.3.1 release carries the Keystone codename",
  keystone && keystone.codename === "Keystone",
  keystone && JSON.stringify(keystone.codename),
);
check(
  "the 0.3.1 release is a release-channel artifact",
  keystone && keystone.channel === "release",
);
check(
  "the 0.3.1 release implements the 0.3.1 language",
  keystone && keystone.language_version === "0.3.1",
  keystone && keystone.language_version,
);

// The channel vocabulary is exactly release|development: the UI maps a
// superseded development entry to a Beta group, but the stored channel stays
// one of the two honest values.
let channelFailures = 0;
for (const entry of manifest.versions) {
  if (entry.channel !== "release" && entry.channel !== "development") {
    channelFailures += 1;
    console.error(`FAIL ${entry.id}: unknown channel ${JSON.stringify(entry.channel)}`);
  }
}
check("every entry uses the channel vocabulary", channelFailures === 0);

console.log(`\nManifest: ${passed} passed, ${failed} failed`);
process.exit(failed === 0 ? 0 : 1);
