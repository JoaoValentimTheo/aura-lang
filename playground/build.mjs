#!/usr/bin/env node
// Build the versioned Aura Playground runtime artifacts and manifest.
//
// Usage:
//   node playground/build.mjs [--check]
//
// The script builds the `aura-playground-runtime` crate for
// `wasm32-unknown-unknown`, copies the artifact into an immutable
// `playground/runtimes/<version>/runtime.wasm`, computes its SHA-256, and
// writes `playground/runtimes/manifest.json`.
//
// Immutability: if `runtimes/<version>/runtime.wasm` already exists with a
// *different* hash, the script refuses to overwrite it (exit 1). An
// already-published version identity can never be silently replaced. Pass
// `--check` to verify the manifest matches the artifacts on disk without
// building (used by tests and CI).

import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, copyFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(here, "..");
const runtimeDir = join(here, "runtime");
const runtimesDir = join(here, "runtimes");
const manifestPath = join(runtimesDir, "manifest.json");
const args = new Set(process.argv.slice(2));
const checkOnly = args.has("--check");

const PLAYGROUND_API_VERSION = 1;

// The frozen 0.0.1 release predates the WebAssembly execution substrate (it
// spawns OS threads for parsing/execution), so it has no browser runtime. It
// is recorded as a historical, non-executable entry rather than fabricated.
const FROZEN_0_0_1 = {
  id: "0.0.1",
  release_version: "0.0.1",
  language_version: "0.0.1",
  runtime_version: null,
  host_abi_version: null,
  available: false,
  channel: "release",
  reason:
    "The published 0.0.1 predates the WebAssembly execution substrate and has no browser runtime. Selecting it will not execute any artifact.",
  artifact: null,
  sha256: null,
  bytes: null,
};

// The published, immutable 0.0.2 runtime artifact. Its hash and size are pinned
// here as the single authoritative identity for the published release: the
// build never regenerates it (the runtime crate version has moved on) and the
// `--check` path refuses to pass if the artifact on disk ever drifts from this
// record. Publishing a newer runtime means adding a *new* entry below, never
// editing this one.
const FROZEN_0_0_2 = {
  id: "0.0.2",
  release_version: "0.0.2",
  language_version: "0.0.1",
  runtime_version: "0.0.2",
  host_abi_version: 1,
  available: true,
  channel: "release",
  artifact: "0.0.2/aura_playground_runtime.wasm",
  sha256: "5a4ad3f7e3f786164d65df437d607e7ddd5e25947ea2c8dd9b436a5490b334ed",
  bytes: 1366621,
};

// The `0.2.0` release runtime artifact, pinned so it can never drift. It was
// the artifact this file generated while the runtime crate carried the release
// version; the crate has since advanced to a development pre-release on the
// same line, so the release identity is now pinned here permanently and the
// build refuses to regenerate it. Publishing a newer release means adding a
// *new* pinned entry, never editing this one.
const FROZEN_0_2_0 = {
  id: "0.2.0",
  release_version: "0.2.0",
  language_version: "0.2.0",
  runtime_version: "0.2.0",
  host_abi_version: 1,
  available: true,
  channel: "release",
  artifact: "0.2.0/aura_playground_runtime.wasm",
  sha256: "9937fd8094ef402b7a9233d02bd232405f75b9e70661404646fcda7cd295c5bc",
  bytes: 1654161,
};

// The final development artifact of the pre-0.2.0 line. It is preserved and
// addressable for as long as it remains in the manifest. Its identity is
// pinned so the artifact can never drift.
const HISTORICAL_DEV_0_0_2_DEV_30 = {
  id: "0.0.2-dev.30",
  release_version: "0.0.2",
  language_version: "0.0.1",
  runtime_version: "0.0.2-dev.30",
  host_abi_version: 1,
  available: true,
  channel: "development",
  artifact: "0.0.2-dev.30/aura_playground_runtime.wasm",
  sha256: "916a8282f7afcf67b89662af89d2f69cf562d9dbe41fe88cab1764a3ef19c578",
  bytes: 1654216,
};

// The first development artifact of the `0.2.0` line, superseded by
// `0.2.0-dev.2`. It predates the builtin-name value-namespace reservation, so
// it is preserved and selectable as a historical development runtime but is no
// longer the manifest default. Its identity is pinned so the artifact can
// never drift.
const HISTORICAL_DEV_0_2_0_DEV_1 = {
  id: "0.2.0-dev.1",
  release_version: "0.2.0",
  language_version: "0.2.0",
  runtime_version: "0.2.0-dev.1",
  host_abi_version: 1,
  available: true,
  channel: "development",
  artifact: "0.2.0-dev.1/aura_playground_runtime.wasm",
  sha256: "ba40e89c834896badfb17d5c72aa2dcb227907a7b5ba513c315ef2f2da0adf08",
  bytes: 1767068,
};

// The second development artifact of the `0.2.0` line, superseded by
// `0.2.1-dev.5`. It carries the builtin-name reservation but predates the
// `0.2.1` version line (ADR-0001), so it is preserved and selectable as a
// historical development runtime but is no longer the manifest default. Its
// identity is pinned so the artifact can never drift. (The intermediate
// `0.2.1-dev.1`..`0.2.1-dev.4` are preserved on disk unreferenced, per the
// convention that only the final development runtime of a line is manifested:
// `.1` predates ADR-0004, `.2` predates the train-1 parser hardening, `.3`
// predates the re-verification checker fixes, and `.4` predates the TD-15
// diagnostic-location change.)
const HISTORICAL_DEV_0_2_0_DEV_2 = {
  id: "0.2.0-dev.2",
  release_version: "0.2.0",
  language_version: "0.2.0",
  runtime_version: "0.2.0-dev.2",
  host_abi_version: 1,
  available: true,
  channel: "development",
  artifact: "0.2.0-dev.2/aura_playground_runtime.wasm",
  sha256: "b69f212bf3f1d8df41b66ad249bf9c9829015b06459569fd2b765a5596b66c06",
  bytes: 1767723,
};

function sha256(buf) {
  return createHash("sha256").update(buf).digest("hex");
}

function readCrateVersion(crateDir) {
  const toml = readFileSync(join(crateDir, "Cargo.toml"), "utf8");
  const m = toml.match(/^version\s*=\s*"([^"]+)"/m);
  if (!m) throw new Error(`no version in ${crateDir}/Cargo.toml`);
  return m[1];
}

/**
 * Read `LANGUAGE_VERSION` from the core crate source. The language semantics
 * version is independent of the release version: 0.0.2 ships the frozen 0.0.1
 * language. Reading the constant (rather than assuming the package version)
 * keeps the manifest honest if the two ever diverge.
 */
function readLanguageVersion() {
  const src = readFileSync(join(repoRoot, "src", "lib.rs"), "utf8");
  const m = src.match(/LANGUAGE_VERSION:\s*&str\s*=\s*"([^"]+)"/);
  if (!m) throw new Error("cannot read LANGUAGE_VERSION from src/lib.rs");
  return m[1];
}

/** Read the release version from the core crate's Cargo.toml. */
function readReleaseVersion() {
  return readCrateVersion(repoRoot);
}

const runtimeVersion = readCrateVersion(runtimeDir);
const languageVersion = readLanguageVersion();
const releaseVersion = readReleaseVersion();
const artifactName = "aura_playground_runtime.wasm";
const wasmPath = join(
  runtimeDir,
  "target",
  "wasm32-unknown-unknown",
  "release",
  artifactName,
);

if (checkOnly) {
  if (!existsSync(manifestPath)) {
    console.error("build: manifest missing; run without --check first");
    process.exit(1);
  }
  const manifest = JSON.parse(readFileSync(manifestPath, "utf8"));
  {
    for (const entry of manifest.versions) {
      if (!entry.available) continue;
      const p = join(runtimesDir, entry.artifact);
      if (!existsSync(p)) {
        console.error(`build --check: missing artifact ${entry.artifact}`);
        process.exit(1);
      }
      const bytes = readFileSync(p);
      const hash = sha256(bytes);
      if (hash !== entry.sha256) {
        console.error(
          `build --check: hash mismatch for ${entry.id}: manifest ${entry.sha256} != disk ${hash}`,
        );
        process.exit(1);
      }
      if (bytes.byteLength !== entry.bytes) {
        console.error(
          `build --check: size mismatch for ${entry.id}: manifest ${entry.bytes} != disk ${bytes.byteLength}`,
        );
        process.exit(1);
      }
      // A release-channel artifact is frozen: its identity is pinned in source
      // and must match the manifest exactly. A drift here means someone edited
      // a historical artifact or the pinned identity, and the build refuses to
      // proceed.
      if (entry.channel === "release") {
        const pinned = [FROZEN_0_0_1, FROZEN_0_0_2, FROZEN_0_2_0].find((f) => f.id === entry.id);
        if (pinned && entry.available) {
          if (pinned.sha256 !== entry.sha256 || pinned.bytes !== entry.bytes) {
            console.error(
              `build --check: release ${entry.id} is frozen; manifest identity does not match the pinned identity`,
            );
            process.exit(1);
          }
        }
      }
    }
  }
  // Every frozen release artifact must still be present, byte-for-byte, even
  // if something removed it from the manifest.
  for (const frozen of [FROZEN_0_0_2, FROZEN_0_2_0]) {
    const p = join(runtimesDir, frozen.artifact);
    if (!existsSync(p)) {
      console.error(`build --check: frozen release artifact missing: ${frozen.artifact}`);
      process.exit(1);
    }
    const hash = sha256(readFileSync(p));
    if (hash !== frozen.sha256) {
      console.error(
        `build --check: frozen release artifact ${frozen.artifact} changed on disk: ${hash}`,
      );
      process.exit(1);
    }
  }
  console.log(`build --check: manifest matches ${manifest.versions.length} version(s)`);
  process.exit(0);
}

// --- build the wasm artifact -------------------------------------------------
console.log(`building aura-playground-runtime v${runtimeVersion} for wasm32-unknown-unknown…`);
execFileSync(
  "cargo",
  ["build", "--release", "--target", "wasm32-unknown-unknown"],
  { cwd: runtimeDir, stdio: "inherit" },
);

if (!existsSync(wasmPath)) {
  console.error(`build: expected artifact at ${wasmPath}`);
  process.exit(1);
}

const wasm = readFileSync(wasmPath);
const hash = sha256(wasm);
const versionDir = join(runtimesDir, runtimeVersion);
const destPath = join(versionDir, artifactName);

// --- immutability guard ------------------------------------------------------
if (existsSync(destPath)) {
  const existing = readFileSync(destPath);
  const existingHash = sha256(existing);
  if (existingHash !== hash) {
    console.error(
      `build: refusing to overwrite immutable runtime ${runtimeVersion}\n` +
        `  existing sha256 ${existingHash}\n` +
        `  new      sha256 ${hash}\n` +
        `Historical runtime artifacts must never be silently replaced. ` +
        `Bump the runtime crate version instead.`,
    );
    process.exit(1);
  }
  console.log(`build: runtime ${runtimeVersion} unchanged (sha256 ${hash.slice(0, 12)}…)`);
} else {
  mkdirSync(versionDir, { recursive: true });
  copyFileSync(wasmPath, destPath);
  console.log(`build: wrote ${destPath}`);
}

// Read the ABI version out of the built module's exported string tables by
// parsing the manifest-independent constants is not possible without running
// it; instead the loader validates the ABI at load time. Record the version
// the runtime crate declares.
const runtimeSource = readFileSync(join(runtimeDir, "src", "lib.rs"), "utf8");
const abiMatch = runtimeSource.match(/ABI_VERSION:\s*u32\s*=\s*(\d+)/);
if (!abiMatch) throw new Error("cannot read ABI_VERSION from runtime source");
const abiVersion = Number(abiMatch[1]);

// The built crate version is the *current* runtime. A version without a
// pre-release suffix (`-dev`, `-rc`, …) is a published release; a version with
// one is a development runtime on its release line. This keeps the channel
// honest without hardcoding which version is current.
const isRelease = !runtimeVersion.includes("-");
const channel = isRelease ? "release" : "development";

const manifest = {
  playground_api_version: PLAYGROUND_API_VERSION,
  // The generated runtime is the selector's default while it is under
  // development; every frozen and historical entry remains present, honest,
  // and selectable, and is never substituted silently.
  current: runtimeVersion,
  versions: [
    FROZEN_0_0_1,
    FROZEN_0_0_2,
    HISTORICAL_DEV_0_0_2_DEV_30,
    HISTORICAL_DEV_0_2_0_DEV_1,
    HISTORICAL_DEV_0_2_0_DEV_2,
    FROZEN_0_2_0,
    {
      id: runtimeVersion,
      // A release entry's runtime version equals its release version; a
      // development runtime carries the release line it belongs to
      // (`release_version`) while its own identity is the pre-release
      // `runtime_version`. The channel makes the distinction explicit so the
      // UI never presents a development build as a release.
      release_version: isRelease ? runtimeVersion : releaseVersion,
      language_version: languageVersion,
      runtime_version: runtimeVersion,
      host_abi_version: abiVersion,
      available: true,
      channel,
      artifact: `${runtimeVersion}/${artifactName}`,
      sha256: hash,
      bytes: wasm.byteLength,
    },
  ],
};

writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);
console.log(
  `build: manifest written (current=${manifest.current}, ${manifest.versions.length} versions)`,
);
