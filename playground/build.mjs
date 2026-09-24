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
  language_version: "0.0.1",
  runtime_version: null,
  host_abi_version: null,
  available: false,
  reason:
    "The published 0.0.1 predates the WebAssembly execution substrate and has no browser runtime. Selecting it will not execute any artifact.",
  artifact: null,
  sha256: null,
  bytes: null,
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

function readLanguageVersion() {
  const toml = readFileSync(join(repoRoot, "Cargo.toml"), "utf8");
  const m = toml.match(/^version\s*=\s*"([^"]+)"/m);
  if (!m) throw new Error("no version in repo Cargo.toml");
  return m[1];
}

const runtimeVersion = readCrateVersion(runtimeDir);
const languageVersion = readLanguageVersion();
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

const manifest = {
  playground_api_version: PLAYGROUND_API_VERSION,
  current: runtimeVersion,
  versions: [
    FROZEN_0_0_1,
    {
      id: runtimeVersion,
      language_version: languageVersion,
      runtime_version: runtimeVersion,
      host_abi_version: abiVersion,
      available: true,
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
