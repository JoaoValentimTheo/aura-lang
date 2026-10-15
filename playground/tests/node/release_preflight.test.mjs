// Release-preflight regression tests.
//
// A defect found while publishing v0.2.1: `scripts/release-preflight.sh`'s
// tag-immutability check ran *inside* the tag-triggered release workflow, where
// the tag necessarily already exists, so the check always failed and every
// tag-triggered release was unpublishable. The fix adds `--on-tag`, which
// verifies the tag points at the released commit instead of requiring the tag
// to be free.
//
// These tests use a throwaway git repository to exercise version identities,
// SemVer precedence, runtime artifacts, and tag immutability.
//
// Usage: node playground/tests/node/release_preflight.test.mjs

import { execFileSync } from "node:child_process";
import { mkdtempSync, mkdirSync, writeFileSync, copyFileSync, rmSync, unlinkSync } from "node:fs";
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

  // Use only throwaway manifest/source identities. Never touch real manifests,
  // tags, or published artifacts; each case overwrites all relevant inputs.
  function setVersions(release = VERSION, language = VERSION, lock = release, source = null) {
    writeFileSync(join(dir, "Cargo.toml"), `[package]\nname = "aura-lang"\nversion = "${release}"\n`);
    writeFileSync(join(dir, "Cargo.lock"), `version = 3\n\n[[package]]\nname = "aura-lang"\nversion = "${lock}"\n`);
    writeFileSync(
      join(dir, "src", "lib.rs"),
      source ?? `pub const VERSION: &str = env!("CARGO_PKG_VERSION");\npub const LANGUAGE_VERSION: &str = "${language}";\n`,
    );
    mkdirSync(join(dir, "playground", "runtimes", release), { recursive: true });
    writeFileSync(join(dir, "playground", "runtimes", release, "aura_playground_runtime.wasm"), "wasm");
  }
  function versionCase(name, release, language, expectedCode, expectedMessage) {
    setVersions(release, language);
    const result = preflight(dir, release, "--tag");
    check(name, result.code === expectedCode && expectedMessage.test(result.out), result.out);
  }

  versionCase("equal release/language identities pass", VERSION, VERSION, 0, /preflight: OK/);
  versionCase("release newer than language passes", VERSION, "9.9.8", 0, /preflight: OK/);
  versionCase("language newer than release fails", VERSION, "9.9.10", 1, /LANGUAGE_VERSION is ahead/);
  versionCase("stable release accepts older prerelease language", VERSION, "9.9.9-alpha.1", 0, /preflight: OK/);
  versionCase("prerelease cannot claim newer stable language", "9.9.9-alpha.1", VERSION, 1, /LANGUAGE_VERSION is ahead/);
  versionCase("prerelease numeric 10 exceeds numeric 2", "9.9.9-alpha.10", "9.9.9-alpha.2", 0, /preflight: OK/);
  versionCase("prerelease numeric 2 precedes numeric 10", "9.9.9-alpha.2", "9.9.9-alpha.10", 1, /LANGUAGE_VERSION is ahead/);
  versionCase("numeric prerelease identifier precedes alphanumeric", "9.9.9-alpha.beta", "9.9.9-alpha.2", 0, /preflight: OK/);
  versionCase("build metadata does not affect precedence", "9.9.9+build.2", "9.9.9+build.9", 0, /preflight: OK/);
  versionCase("invalid leading-zero prerelease is rejected", VERSION, "9.9.9-alpha.02", 1, /not valid SemVer/);
  versionCase("invalid language SemVer is rejected", VERSION, "9.9.9x", 1, /not valid SemVer/);
  versionCase("invalid release SemVer is rejected", "9.9.09", "9.9.8", 1, /not valid SemVer/);

  setVersions(VERSION, "9.9.8", "9.9.8");
  const lockMismatch = preflight(dir, VERSION, "--tag");
  check("manifest/lock mismatch is rejected", lockMismatch.code === 1 && /version identities disagree/.test(lockMismatch.out), lockMismatch.out);

  for (const [name, source] of [
    ["missing VERSION", `pub const LANGUAGE_VERSION: &str = "${VERSION}";\n`],
    ["literal VERSION", `pub const VERSION: &str = "${VERSION}";\npub const LANGUAGE_VERSION: &str = "${VERSION}";\n`],
    ["duplicate VERSION", `pub const VERSION: &str = env!("CARGO_PKG_VERSION");\npub const VERSION: &str = env!("CARGO_PKG_VERSION");\npub const LANGUAGE_VERSION: &str = "${VERSION}";\n`],
  ]) {
    setVersions(VERSION, VERSION, VERSION, source);
    const result = preflight(dir, VERSION, "--tag");
    check(`${name} source declaration is rejected`, result.code === 1 && /expected exactly one aura::VERSION/.test(result.out), result.out);
  }
  setVersions(VERSION, VERSION, VERSION, `pub const VERSION: &str = env!("CARGO_PKG_VERSION");\n`);
  const missingLanguage = preflight(dir, VERSION, "--tag");
  check("missing LANGUAGE_VERSION is rejected", missingLanguage.code === 1 && /missing or invalid LANGUAGE_VERSION/.test(missingLanguage.out), missingLanguage.out);

  setVersions();
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "aura-lang"\n');
  const missingCargoVersion = preflight(dir);
  check("missing Cargo version is rejected", missingCargoVersion.code === 1 && /cannot read version from Cargo.toml/.test(missingCargoVersion.out), missingCargoVersion.out);
  setVersions();
  for (const file of ["Cargo.lock", "src/lib.rs"]) {
    unlinkSync(join(dir, file));
    const result = preflight(dir, VERSION, "--tag");
    check(`missing ${file} is rejected`, result.code === 1 && result.out.includes(`missing required version source: ${file}`), result.out);
    setVersions();
  }

  setVersions();
  const wrongRequestedVersion = preflight(dir, "8.8.8", "--tag");
  check("requested tag version must match Cargo", wrongRequestedVersion.code === 1 && /does not match Cargo.toml/.test(wrongRequestedVersion.out), wrongRequestedVersion.out);

  const artifactPath = join(dir, "playground", "runtimes", VERSION, "aura_playground_runtime.wasm");
  unlinkSync(artifactPath);
  const missingArtifact = preflight(dir, VERSION, "--tag");
  check("strict preflight rejects missing versioned artifact", missingArtifact.code === 1 && /missing release runtime artifact/.test(missingArtifact.out), missingArtifact.out);
  writeFileSync(artifactPath, "wasm");
  setVersions();

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
