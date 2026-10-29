// SemVer precedence tests for `scripts/semver.py`.
//
// Three release paths (preflight, manifest generator, release workflow) used
// `sort -V`, which is not SemVer: it ranks a prerelease above the release it
// leads to, treats build metadata as significant, and prints both lines for
// equal versions so `| head -n1` picks one arbitrarily. They now delegate to
// one comparator. These tests pin its contract so the paths cannot drift apart
// again, and pin ADR-0001 (`LANGUAGE_VERSION <= RELEASE_VERSION`) against
// prerelease/stable pairs specifically.
//
// Usage: node playground/tests/node/semver.test.mjs

import { execFileSync } from "node:child_process";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const repo = resolve(here, "../../..");
const semver = join(repo, "scripts", "semver.py");

let passed = 0;
let failed = 0;
function check(name, cond, detail) {
  if (cond) passed += 1;
  else {
    failed += 1;
    console.error(`FAIL ${name}${detail ? `: ${detail}` : ""}`);
  }
}

function run(...args) {
  try {
    return {
      code: 0,
      out: execFileSync("python3", [semver, ...args], {
        cwd: repo,
        encoding: "utf8",
        stdio: ["ignore", "pipe", "pipe"],
      }),
      err: "",
    };
  } catch (error) {
    return { code: error.status ?? 1, out: error.stdout ?? "", err: error.stderr ?? "" };
  }
}

function compare(a, b) {
  const result = run("compare", a, b);
  return result.code === 0 ? Number(result.out.trim()) : `exit ${result.code}`;
}

// Semantic Versioning 2.0.0, section 11.4.1: this exact ordering is normative.
const ordered = [
  "1.0.0-alpha",
  "1.0.0-alpha.1",
  "1.0.0-alpha.beta",
  "1.0.0-beta",
  "1.0.0-beta.2",
  "1.0.0-beta.11",
  "1.0.0-rc.1",
  "1.0.0",
];
for (let i = 0; i < ordered.length - 1; i += 1) {
  const [a, b] = [ordered[i], ordered[i + 1]];
  check(`specification order: ${a} < ${b}`, compare(a, b) === -1, `got ${compare(a, b)}`);
  check(`specification order: ${b} > ${a}`, compare(b, a) === 1, `got ${compare(b, a)}`);
}

// The specific regression `sort -V` caused: it ranked the prerelease above the
// release it leads to, which inverts ADR-0001 for every development line.
check("prerelease precedes its release", compare("0.3.2-dev.7", "0.3.2") === -1);
check("release follows its prerelease", compare("0.3.2", "0.3.2-dev.7") === 1);
check("prerelease is below the next patch", compare("0.3.2-rc.1", "0.3.1") === 1);
check("stable release is above the previous release", compare("0.3.2", "0.3.1") === 1);

check("core fields compare numerically", compare("1.10.0", "1.9.0") === 1);
check("major dominates minor", compare("2.0.0", "1.99.99") === 1);
check("prerelease numeric identifiers compare numerically", compare("1.0.0-alpha.10", "1.0.0-alpha.2") === 1);
check("numeric identifier ranks below alphanumeric", compare("1.0.0-alpha.1", "1.0.0-alpha.beta") === -1);
check("more prerelease fields rank higher", compare("1.0.0-alpha", "1.0.0-alpha.1") === -1);

// Section 10: build metadata is ignored when determining precedence.
check("build metadata is ignored (equal)", compare("1.0.0+build.2", "1.0.0+build.9") === 0);
check("build metadata is ignored on a prerelease", compare("1.0.0-rc.1+a", "1.0.0-rc.1+b") === 0);
check("identical versions compare equal", compare("1.0.0", "1.0.0") === 0);

// Section 9/2: leading zeroes are invalid in the core and in numeric
// prerelease identifiers.
for (const bad of ["1.0", "1.0.0.0", "01.0.0", "1.02.0", "1.0.0-alpha.02", "1.0.0-", "1.0.0+", "v1.0.0", "1.0.0x", ""]) {
  const result = run("compare", bad, "1.0.0");
  check(`invalid version ${JSON.stringify(bad)} exits 2`, result.code === 2, `exit ${result.code}`);
}
check("invalid version prints no comparison", run("compare", "nope", "1.0.0").out === "");

// `check` encodes ADR-0001 and distinguishes its failure modes by exit status,
// which is what the preflight, the manifest, and release.yml branch on.
const checkCase = (name, release, language, expected) => {
  const result = run("check", release, language);
  check(name, result.code === expected, `exit ${result.code}, expected ${expected}`);
};
checkCase("equal identities pass", "0.3.1", "0.3.1", 0);
checkCase("release ahead of language passes", "0.3.2", "0.3.1", 0);
checkCase("stable release accepts older prerelease language", "0.3.2", "0.3.1-alpha.1", 0);
checkCase("language ahead of release fails", "0.3.1", "0.3.2", 3);
checkCase("prerelease language cannot claim its release", "0.3.2-dev.7", "0.3.2", 3);
checkCase("prerelease language cannot claim the next release", "0.3.2-dev.7", "0.3.3", 3);
checkCase("invalid release exits 2", "0.3", "0.3.1", 2);
checkCase("invalid language exits 2", "0.3.1", "0.3.1x", 2);
checkCase("larger prerelease set is ahead", "0.3.2-dev.7", "0.3.2-dev.8", 3);

// `--quiet` must suppress the module's own diagnostics: a caller that prints
// its own message must not emit two competing explanations.
const quiet = run("check", "0.3.2-dev.7", "0.3.2", "--quiet");
check("quiet check keeps the exit status", quiet.code === 3, `exit ${quiet.code}`);
check("quiet check suppresses diagnostics", quiet.err === "", quiet.err);
check(
  "loud check explains the failure",
  /ahead of release/.test(run("check", "0.3.2-dev.7", "0.3.2").err),
);

check("usage error without a subcommand", run().code === 64);
check("usage error on an unknown subcommand", run("bogus").code === 64);
check("usage error on missing operands", run("compare", "1.0.0").code === 64);

console.log(`semver: ${passed} passed, ${failed} failed`);
if (failed > 0) process.exit(1);
