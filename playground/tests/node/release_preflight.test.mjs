// Release-preflight regression tests.
//
// A defect found while publishing v0.2.1: `scripts/release-preflight.sh`'s
// tag-immutability check ran *inside* the tag-triggered release workflow, where
// the tag necessarily already exists, so the check always failed and every
// tag-triggered release was unpublishable. The fix adds `--on-tag`, which
// verifies the tag points at the released commit instead of requiring the tag
// to be free.
//
// These tests build a throwaway git repository with the minimal files the
// preflight reads and exercise the four meaningful tag states, so the release
// path cannot regress silently again.
//
// Usage: node playground/tests/node/release_preflight.test.mjs

import { execFileSync } from "node:child_process";
import { mkdtempSync, mkdirSync, writeFileSync, copyFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const repo = resolve(here, "../../..");
const script = join(repo, "scripts", "release-preflight.sh");

let passed = 0;
let failed = 0;
function check(name, cond, detail) {
  if (cond) passed += 1;
  else {
    failed += 1;
    console.error(`FAIL ${name}${detail ? `: ${detail}` : ""}`);
  }
}

function git(cwd, ...args) {
  return execFileSync("git", args, { cwd, encoding: "utf8" });
}

// Run the preflight and capture exit status + combined output.
function preflight(dir, ...args) {
  try {
    const out = execFileSync(script, args, {
      cwd: dir,
      encoding: "utf8",
      stdio: ["ignore", "pipe", "pipe"],
    });
    return { code: 0, out };
  } catch (err) {
    return {
      code: err.status ?? 1,
      out: `${err.stdout ?? ""}${err.stderr ?? ""}`,
    };
  }
}

const VERSION = "9.9.9";
const dir = mkdtempSync(join(tmpdir(), "aura-preflight-"));

try {
  // Minimal repository the preflight reads.
  git(dir, "init", "-q");
  git(dir, "config", "user.email", "test@example.invalid");
  git(dir, "config", "user.name", "preflight test");
  git(dir, "config", "commit.gpgsign", "false");

  writeFileSync(join(dir, "Cargo.toml"), `[package]\nname = "aura-lang"\nversion = "${VERSION}"\n`);
  writeFileSync(
    join(dir, "Cargo.lock"),
    `version = 3\n\n[[package]]\nname = "aura-lang"\nversion = "${VERSION}"\n`,
  );
  mkdirSync(join(dir, "src"), { recursive: true });
  writeFileSync(
    join(dir, "src", "lib.rs"),
    `pub const VERSION: &str = env!("CARGO_PKG_VERSION");\npub const LANGUAGE_VERSION: &str = "${VERSION}";\n`,
  );
  mkdirSync(join(dir, "playground", "runtimes", VERSION), { recursive: true });
  writeFileSync(join(dir, "playground", "runtimes", VERSION, "aura_playground_runtime.wasm"), "wasm");
  mkdirSync(join(dir, "docs", "release-notes"), { recursive: true });
  writeFileSync(join(dir, "docs", "release-notes", `v${VERSION}.md`), "# notes\n");
  mkdirSync(join(dir, "scripts"), { recursive: true });
  copyFileSync(script, join(dir, "scripts", "release-preflight.sh"));
  git(dir, "add", "-A");
  git(dir, "commit", "-q", "-m", "fixture");

  // 1. Pre-tag, all prerequisites present: `--tag` succeeds and the tag is free.
  const before = preflight(dir, VERSION, "--tag");
  check("pre-tag --tag passes when the tag is free", before.code === 0, before.out);
  check("pre-tag reports the tag free", /tag v.* is free/.test(before.out), before.out);

  // 2. Tag now exists at HEAD. A *pre-tag* check (`--tag`) must refuse to reuse
  //    the identity — this is the immutability guard.
  git(dir, "tag", "-a", `v${VERSION}`, "-m", `v${VERSION}`);
  const reuse = preflight(dir, VERSION, "--tag");
  check("pre-tag --tag fails once the tag exists", reuse.code !== 0, reuse.out);
  check("pre-tag failure names immutability", /already exists/.test(reuse.out), reuse.out);

  // 3. The tag-triggered workflow context (`--on-tag`) at the tagged commit must
  //    pass: this is the exact path that was broken.
  const onTag = preflight(dir, VERSION, "--on-tag");
  check("--on-tag passes when the tag points at HEAD", onTag.code === 0, onTag.out);
  check("--on-tag confirms tag/commit agreement", /points at the released commit/.test(onTag.out), onTag.out);

  // 4. A tag that does not point at the released commit is rejected by `--on-tag`.
  writeFileSync(join(dir, "extra.txt"), "advance\n");
  git(dir, "add", "-A");
  git(dir, "commit", "-q", "-m", "advance past the tag");
  const mismatch = preflight(dir, VERSION, "--on-tag");
  check("--on-tag fails when the tag does not point at HEAD", mismatch.code !== 0, mismatch.out);
  check("--on-tag mismatch is explicit", /does not point at HEAD/.test(mismatch.out), mismatch.out);

  // 5. A missing tag in `--on-tag` mode is also rejected.
  const missing = preflight(dir, "8.8.8", "--on-tag");
  check("--on-tag fails when the tag is absent", missing.code !== 0, missing.out);

  // 6. `--on-tag` implies tag mode: a missing release runtime artifact is fatal.
  mkdirSync(join(dir, "playground", "runtimes", "8.8.8"), { recursive: true });
  const noArtifact = preflight(dir, "8.8.8", "--on-tag");
  check(
    "--on-tag treats a missing release runtime as fatal",
    noArtifact.code !== 0 && /missing release runtime artifact/.test(noArtifact.out),
    noArtifact.out,
  );
} finally {
  rmSync(dir, { recursive: true, force: true });
}

console.log(`release-preflight: ${passed} passed, ${failed} failed`);
if (failed > 0) process.exit(1);
