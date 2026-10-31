// Deterministic fingerprint of the inputs that produce a Playground runtime.
//
// A runtime artifact's SHA-256 identifies the *output*. It says nothing about
// which sources produced it, so a committed artifact can silently drift from
// the tree it claims to represent. The fingerprint below digests the build
// inputs instead: every Rust source of the core and runtime crates, both crate
// manifests, both dependency lockfiles, and the pinned toolchain file.
//
// It deliberately excludes the artifact hash, the manifest, and any commit SHA.
// A value that embedded its own output would be self-referential and could
// never be verified; this one is a pure function of the inputs, so anyone can
// recompute it from a checkout.
//
// Invariant: identical inputs -> identical fingerprint, on any host. Paths are
// normalized to repo-relative POSIX form and sorted, file contents are hashed
// individually, and the record stream is newline-separated with NUL between
// path and digest so no path/content pair can be ambiguous.

import { createHash } from "node:crypto";
import { readFileSync, readdirSync } from "node:fs";
import { join, relative, sep } from "node:path";

/** Directories whose Rust sources are compiled into the runtime artifact. */
const SOURCE_ROOTS = ["src", "playground/runtime/src"];

/**
 * Non-source files that pin the build: crate manifests, dependency lockfiles
 * and the toolchain file. `Cargo.lock` changes can swap a transitive
 * dependency; `rust-toolchain.toml` changes the compiler. Both change the
 * emitted wasm without touching a single line of Aura source.
 */
const PINNED_INPUTS = [
  "Cargo.toml",
  "Cargo.lock",
  "playground/runtime/Cargo.toml",
  "playground/runtime/Cargo.lock",
  "rust-toolchain.toml",
];

/** Stream prefix; bump only if the record format itself changes. */
const FORMAT_TAG = "aura-runtime-build-inputs/v1";

function collectRustSources(dir, out) {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const p = join(dir, entry.name);
    if (entry.isDirectory()) collectRustSources(p, out);
    else if (entry.isFile() && entry.name.endsWith(".rs")) out.push(p);
  }
}

/**
 * The complete, sorted list of repo-relative paths that make up the fingerprint.
 * Exposed so tests and report tooling can name the exact input set instead of
 * re-deriving it.
 */
export function sourceFingerprintInputs(repoRoot) {
  const abs = [];
  for (const root of SOURCE_ROOTS) collectRustSources(join(repoRoot, root), abs);
  for (const rel of PINNED_INPUTS) abs.push(join(repoRoot, rel));
  return abs
    .map((p) => relative(repoRoot, p).split(sep).join("/"))
    .sort((a, b) => (a < b ? -1 : a > b ? 1 : 0));
}

/** SHA-256 over the canonical record stream of every build input. */
export function computeSourceFingerprint(repoRoot) {
  const hash = createHash("sha256");
  hash.update(`${FORMAT_TAG}\n`);
  for (const rel of sourceFingerprintInputs(repoRoot)) {
    const fileDigest = createHash("sha256")
      .update(readFileSync(join(repoRoot, ...rel.split("/"))))
      .digest("hex");
    hash.update(`${rel}\0${fileDigest}\n`);
  }
  return hash.digest("hex");
}
