// Release-version drift guard.
//
// The public site must present exactly the release identity the repository
// actually ships. This test fails when any of those authorities disagree:
//
//   * Cargo.toml `[package] version`            (the release/CLI identity)
//   * src/lib.rs `LANGUAGE_VERSION`             (the language contract)
//   * playground/runtime/Cargo.toml version     (the runtime crate identity)
//   * playground/runtimes/manifest.json         (current runtime + language)
//   * website/site.config.mjs                   (the site's version model)
//   * website/content/docs.mjs                  (the docs version model)
//
// It also guards the claims those versions produce:
//
//   * `previousRelease` must not equal `currentRelease`;
//   * `developmentVersion` must not equal `currentRelease`, and may be null;
//   * the runtime selector's default must be the current release identity;
//   * the generated releases/runtime pages must name the current release and
//     must not present a stale one as "Current";
//   * the generated footer must not render `null` as a development label;
//   * a development runtime must never be presented as the current release.
//
// No network access: everything is read from the repository.
//
// Usage: node website/tests/release-version.test.mjs

import { readFileSync, existsSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const repo = resolve(here, "../..");

let passed = 0;
let failed = 0;
function check(name, cond, detail) {
  if (cond) passed += 1;
  else {
    failed += 1;
    console.error(`FAIL ${name}${detail ? `: ${detail}` : ""}`);
  }
}

function readCargoVersion(relativePath) {
  const text = readFileSync(join(repo, relativePath), "utf8");
  const pkg = text.match(/^\[package\][\s\S]*?^version\s*=\s*"([^"]+)"/m);
  return pkg ? pkg[1] : null;
}

function readLanguageVersion() {
  const text = readFileSync(join(repo, "src/lib.rs"), "utf8");
  const m = text.match(/LANGUAGE_VERSION\s*:\s*&str\s*=\s*"([^"]+)"/);
  return m ? m[1] : null;
}

const { site } = await import(join(repo, "website/site.config.mjs"));
const docs = await import(join(repo, "website/content/docs.mjs"));
const manifest = JSON.parse(
  readFileSync(join(repo, "playground/runtimes/manifest.json"), "utf8"),
);

const cliVersion = readCargoVersion("Cargo.toml");
const runtimeCrateVersion = readCargoVersion("playground/runtime/Cargo.toml");
const languageVersion = readLanguageVersion();

// -- identity extraction ------------------------------------------------------

check("Cargo.toml package version readable", typeof cliVersion === "string", `${cliVersion}`);
check("runtime crate version readable", typeof runtimeCrateVersion === "string", `${runtimeCrateVersion}`);
check("src/lib.rs LANGUAGE_VERSION readable", typeof languageVersion === "string", `${languageVersion}`);

const currentEntry = manifest.versions.find((v) => v.id === manifest.current);
check("manifest current entry exists", Boolean(currentEntry), manifest.current);
check("manifest current entry is available", Boolean(currentEntry && currentEntry.available));
check(
  "manifest current entry is a release channel",
  Boolean(currentEntry && currentEntry.channel === "release"),
  currentEntry && currentEntry.channel,
);

// -- cross-authority agreement ------------------------------------------------

check(
  "site.releaseVersion matches the CLI package version",
  site.releaseVersion === cliVersion,
  `${site.releaseVersion} != ${cliVersion}`,
);
check(
  "site.languageVersion matches src/lib.rs LANGUAGE_VERSION",
  site.languageVersion === languageVersion,
  `${site.languageVersion} != ${languageVersion}`,
);
check(
  "site.runtimeVersion matches the runtime crate version",
  site.runtimeVersion === runtimeCrateVersion,
  `${site.runtimeVersion} != ${runtimeCrateVersion}`,
);
check(
  "site.releaseVersion matches site.currentRelease",
  site.releaseVersion === site.currentRelease,
  `${site.releaseVersion} != ${site.currentRelease}`,
);
check(
  "docs.docsVersion matches the site release",
  docs.docsVersion === site.releaseVersion,
  `${docs.docsVersion} != ${site.releaseVersion}`,
);
check(
  "docs.docsLanguageVersion matches the site language version",
  docs.docsLanguageVersion === site.languageVersion,
  `${docs.docsLanguageVersion} != ${site.languageVersion}`,
);
check(
  "manifest current runtime id matches the site release",
  manifest.current === site.releaseVersion,
  `${manifest.current} != ${site.releaseVersion}`,
);
check(
  "manifest current language_version matches the site language version",
  currentEntry && currentEntry.language_version === site.languageVersion,
  `${currentEntry && currentEntry.language_version} != ${site.languageVersion}`,
);
check(
  "manifest current release_version matches the site release",
  currentEntry && currentEntry.release_version === site.releaseVersion,
  `${currentEntry && currentEntry.release_version} != ${site.releaseVersion}`,
);

// -- identity invariants ------------------------------------------------------

check(
  "previousRelease is different from currentRelease",
  site.previousRelease && site.previousRelease !== site.currentRelease,
  `${site.previousRelease} vs ${site.currentRelease}`,
);
check(
  "developmentVersion is null or differs from currentRelease",
  site.developmentVersion === null || site.developmentVersion !== site.currentRelease,
  `${site.developmentVersion} vs ${site.currentRelease}`,
);
check(
  "developmentVersion (when set) differs from the manifest current",
  site.developmentVersion === null || site.developmentVersion !== manifest.current,
  `${site.developmentVersion} vs ${manifest.current}`,
);
check(
  "earliestRelease is the earliest published version",
  site.earliestRelease === "0.0.1",
  site.earliestRelease,
);

// The manifest must keep the previous release addressable and immutable.
const previousEntry = manifest.versions.find((v) => v.id === site.previousRelease);
check("previous release is still in the manifest", Boolean(previousEntry), site.previousRelease);
check(
  "previous release remains available",
  Boolean(previousEntry && previousEntry.available),
  site.previousRelease,
);

// A development runtime must never be the manifest default/current release.
const currentIsDevelopment = manifest.versions.some(
  (v) => v.id === manifest.current && v.channel === "development",
);
check("manifest current is never a development runtime", currentIsDevelopment === false);

// -- generated output ---------------------------------------------------------

const dist = join(repo, "website/dist");
if (existsSync(dist)) {
  const releasesHtml = readFileSync(join(dist, "releases/index.html"), "utf8");
  const runtimeHtml = readFileSync(join(dist, "runtime/index.html"), "utf8");
  const indexHtml = readFileSync(join(dist, "index.html"), "utf8");

  check(
    "generated releases page names the current release",
    releasesHtml.includes(site.currentRelease),
    site.currentRelease,
  );
  check(
    "generated releases page marks the current release as Latest",
    /chip[^>]*>\s*Latest\s*<\/span>/i.test(releasesHtml) ||
      releasesHtml.includes(">Latest<"),
    "no Latest chip found",
  );
  check(
    "generated runtime page names the current release runtime",
    runtimeHtml.includes(site.currentRelease),
    site.currentRelease,
  );
  check(
    "generated footer renders the stable release and no null development label",
    indexHtml.includes(`Stable <strong>${site.currentRelease}</strong>`) &&
      !/development <strong>null<\/strong>/.test(indexHtml),
    "footer mismatch",
  );
  check(
    "generated home hero names the current release",
    indexHtml.includes(`Aura ${site.currentRelease}`),
    site.currentRelease,
  );

  // The published dev.30 runtime is historical, never "current".
  if (existsSync(join(dist, "runtime/index.html"))) {
    check(
      "historical dev runtime is not labelled current",
      !/0\.0\.2-dev\.30[^\n]*current release/.test(runtimeHtml),
      "dev runtime presented as current",
    );
  }
} else {
  console.log("release-version: dist not built; skipping generated-output checks");
}

console.log(`\nrelease-version: ${passed} passed, ${failed} failed`);
process.exit(failed === 0 ? 0 : 1);
