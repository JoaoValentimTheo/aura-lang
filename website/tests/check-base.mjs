// Deployment-base regression guard.
//
// The website's internal URLs are generated through one base abstraction. This
// test fails if any generated HTML contains a root-relative internal link that
// bypasses the configured base — the exact class of defect that made the
// GitHub Pages project deployment serve CSS and links from the domain root.
//
// It also proves the abstraction works for both deployment targets:
//   * project Pages build  --base=/aura-lang/  → every internal URL is based
//   * custom-domain build  --base=/            → internal URLs are root paths
//
// Usage: node website/tests/check-base.mjs [distDir] [--base=<path>]
//
// External URLs (https:, mailto:), fragments (#…), and protocol-relative
// (//…) URLs are intentionally exempt.

import { readFileSync, readdirSync, statSync, existsSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { resolveBase } from "../lib/base.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const args = process.argv.slice(2);
const base = resolveBase(args, process.env);
const distArg = args.find((a) => !a.startsWith("--"));
const dist = resolve(distArg || join(here, "..", "dist"));

if (!existsSync(dist)) {
  console.error("check-base: dist not built; run `node website/build.mjs`");
  process.exit(2);
}

function walk(dir, out = []) {
  for (const name of readdirSync(dir)) {
    const p = join(dir, name);
    if (statSync(p).isDirectory()) walk(p, out);
    else if (p.endsWith(".html")) out.push(p);
  }
  return out;
}

const htmlFiles = walk(dist);
const b = base.endsWith("/") ? base : `${base}/`;
let violations = 0;
let checked = 0;

for (const file of htmlFiles) {
  const html = readFileSync(file, "utf8");
  const refs = [...html.matchAll(/(?:href|src)="([^"]+)"/g)].map((m) => m[1]);
  for (const raw of refs) {
    if (
      raw.startsWith("http://") ||
      raw.startsWith("https://") ||
      raw.startsWith("//") ||
      raw.startsWith("#") ||
      raw.startsWith("mailto:") ||
      raw.startsWith("data:")
    ) {
      continue;
    }
    if (!raw.startsWith("/")) {
      // Relative URLs resolve against the page path, which already lives under
      // the base; they are base-correct by construction.
      continue;
    }
    checked += 1;
    if (b !== "/" && !raw.startsWith(b)) {
      violations += 1;
      console.error(`BASE BYPASS ${file.replace(dist, "")} -> ${raw} (expected prefix ${b})`);
    }
    if (b === "/") {
      // Root build: a `/aura-lang/…` link would be wrong.
      if (raw.startsWith("/aura-lang/")) {
        violations += 1;
        console.error(`UNEXPECTED PROJECT BASE ${file.replace(dist, "")} -> ${raw}`);
      }
    }
  }
}

// The stylesheet and worker must also be base-correct, since they were the
// visible symptoms of the original defect.
const home = join(dist, "index.html");
if (existsSync(home)) {
  const html = readFileSync(home, "utf8");
  if (!html.includes(`${b}assets/tokens.css`)) {
    violations += 1;
    console.error(`homepage does not link ${b}assets/tokens.css`);
  }
  if (!html.includes(`${b}assets/styles.css`)) {
    violations += 1;
    console.error(`homepage does not link ${b}assets/styles.css`);
  }
}

console.log(
  `\nBase: ${base} — ${checked} root-relative refs checked across ${htmlFiles.length} pages, ${violations} violation(s)`,
);
process.exit(violations === 0 ? 0 : 1);
