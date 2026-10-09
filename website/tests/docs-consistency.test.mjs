// Documentation consistency mechanism.
//
// A lightweight guard against the documentation drifting from the shipped
// repository, in the spirit of §14 of the redesign brief. It verifies, without
// a documentation compiler:
//
//   * every documentation page declared in `content/docs.mjs` has a Markdown
//     file that exists and is reachable in the built site;
//   * every documented route resolves to a real `index.html` in the build;
//   * the website release identity agrees with the published metadata
//     (`site.config.mjs` vs `content/docs.mjs` vs the runtime manifest);
//   * the normative sidebar links are release-pinned (`blob/v<version>/…`),
//     not floating `blob/main/…` links;
//   * each pinned normative target genuinely exists at the released tag;
//   * historical pages keep their version context.
//
// Usage: node website/tests/docs-consistency.test.mjs

import { readFileSync, existsSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const repo = resolve(here, "../..");
const website = join(repo, "website");

let passed = 0;
let failed = 0;
function check(name, cond, detail) {
  if (cond) passed += 1;
  else {
    failed += 1;
    console.error(`FAIL ${name}${detail ? `: ${detail}` : ""}`);
  }
}

const { site } = await import(join(website, "site.config.mjs"));
const docs = await import(join(website, "content/docs.mjs"));
const manifest = JSON.parse(
  readFileSync(join(repo, "playground/runtimes/manifest.json"), "utf8"),
);

/* ------------------------------------------------ version identity parity */
check("docs version agrees with the site release", docs.docsVersion === site.releaseVersion);
check(
  "docs language version agrees with the site language version",
  docs.docsLanguageVersion === site.languageVersion,
);
check(
  "runtime manifest current matches the site runtime version",
  manifest.current === site.runtimeVersion,
  `manifest=${manifest.current} site=${site.runtimeVersion}`,
);

/* ------------------------------------------------ declared pages exist */
const all = docs.allDocs();
check("documentation manifest is non-empty", all.length > 0, `${all.length}`);
for (const doc of all) {
  const file = join(website, "content", doc.file);
  check(`doc file exists: ${doc.file}`, existsSync(file), file);
  const dist = join(website, "dist", "docs", doc.slug, "index.html");
  check(`doc route built: docs/${doc.slug}/`, existsSync(dist), dist);
}

/* ------------------------------------------------ duplicates */
const slugs = all.map((d) => d.slug);
check("no duplicate documentation slugs", new Set(slugs).size === slugs.length);

/* ------------------------------------------------ release-pinned links */
const docsPage = readFileSync(join(website, "pages", "docs.mjs"), "utf8");
check(
  "normative links are release-pinned",
  !/\/blob\/main\//.test(docsPage),
  "a blob/main/ link survives",
);
check(
  "normative links use the released tag",
  /blob\/v\$\{docsVersion\}\//.test(docsPage),
);

// Each pinned normative target must exist at the released tag.
const pinned = [...docsPage.matchAll(/docs\/([A-Za-z0-9_.\-]+\.md)/g)].map((m) => m[1]);
const uniquePinned = [...new Set(pinned)];
check("some normative targets are pinned", uniquePinned.length >= 4, uniquePinned.join(","));
for (const target of uniquePinned) {
  let exists = false;
  try {
    const out = execFileSync(
      "git",
      ["ls-tree", "-r", "--name-only", `v${site.releaseVersion}`, "--", `docs/${target}`],
      { cwd: repo, encoding: "utf8" },
    );
    exists = out.trim().length > 0;
  } catch {
    exists = false;
  }
  check(
    `pinned normative target exists at v${site.releaseVersion}: docs/${target}`,
    exists,
  );
}

/* ------------------------------------- no floating developer-branch links */
// Every repository documentation link in content/pages must be release-pinned
// (or point at an intentionally stable resource), never at a mutable branch.
import { readdirSync } from "node:fs";
const sourceDirs = [join(website, "content"), join(website, "pages")];
const floating = [];
for (const dir of sourceDirs) {
  for (const name of readdirSync(dir)) {
    if (!/\.(md|mjs)$/.test(name)) continue;
    const text = readFileSync(join(dir, name), "utf8");
    const re = /github\.com\/JoaoValentimTheo\/aura-lang\/blob\/([^/\s")]+)/g;
    for (const m of text.matchAll(re)) {
      if (m[1] === "main" || m[1] === "master" || m[1].startsWith("rewrite/")) {
        floating.push(`${name}: branch ${m[1]}`);
      }
    }
  }
}
check("no floating branch links in docs sources", floating.length === 0, floating.join("; "));

/* ------------------------------------------------ generated build */
const distDocs = join(website, "dist", "docs", "getting-started", "index.html");
if (existsSync(distDocs)) {
  const html = readFileSync(distDocs, "utf8");
  check(
    "generated docs shell has no floating blob/main normative link",
    !/\/blob\/main\/docs\//.test(html),
  );
  check(
    "generated docs shell pins normative links to the release",
    html.includes(`/blob/v${site.releaseVersion}/docs/`),
  );
}

/* ------------------------------------------------ historical context */
const known = readFileSync(join(website, "content", "known-limitations.md"), "utf8");
check(
  "known-limitations frames 0.3.2 as planned/not started",
  /0\.3\.2/.test(known) && /(planned|not started)/i.test(known),
);
const migration = readFileSync(join(website, "content", "migration-0-3-1.md"), "utf8");
check(
  "migration-0-3-1 documents the released transition",
  /0\.3\.1/.test(migration) && /0\.2\.1/.test(migration),
);

console.log(`\ndocs-consistency: ${passed} passed, ${failed} failed`);
process.exit(failed === 0 ? 0 : 1);
