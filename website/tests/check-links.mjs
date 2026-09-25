// Static link and asset validation for the built website.
//
// Scans every generated HTML file and verifies that:
//   * internal hrefs resolve to a real file in the build;
//   * asset references (css/js/svg/wasm) exist;
//   * the Playground's manifest and worker are present where the controller
//     expects them;
//   * no page links to a missing route.
//
// Usage: node website/tests/check-links.mjs [distDir] [--base=<path>]
//
// The base is resolved through the same shared helper the generator uses, so
// the checker and the build can never disagree about the deployment base.

import { readFileSync, readdirSync, statSync, existsSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { resolveBase } from "../lib/base.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const args = process.argv.slice(2);
const base = resolveBase(args, process.env);
const distArg = args.find((a) => !a.startsWith("--"));
const dist = resolve(distArg || join(here, "..", "dist"));

// Strip the configured base prefix from site-absolute URLs so a non-root
// build can be validated against the same dist directory.
function stripBase(path) {
  const b = base.endsWith("/") ? base : `${base}/`;
  if (b !== "/" && path.startsWith(b)) return "/" + path.slice(b.length);
  return path;
}

if (!existsSync(dist)) {
  console.error("check-links: dist not built; run `node website/build.mjs`");
  process.exit(2);
}

function walk(dir, out = []) {
  for (const name of readdirSync(dir)) {
    const p = join(dir, name);
    const st = statSync(p);
    if (st.isDirectory()) walk(p, out);
    else out.push(p);
  }
  return out;
}

const files = walk(dist);
const htmlFiles = files.filter((f) => f.endsWith(".html"));

/** Map an href to a file on disk, resolving relative URLs against `fromFile`. */
function resolveTarget(href, fromFile) {
  // A URL with a path is resolved against the containing file's directory;
  // a site-absolute URL against the build root.
  const targetDir = dirname(fromFile);
  const target = href.startsWith("/")
    ? join(dist, stripBase(href).replace(/^\//, ""))
    : join(targetDir, href);
  if (href.endsWith("/") || href === "" || target.endsWith("/")) {
    return join(target, "index.html");
  }
  return target;
}

let errors = 0;
let checked = 0;

for (const file of htmlFiles) {
  const html = readFileSync(file, "utf8");
  const hrefs = [...html.matchAll(/(?:href|src)="([^"]+)"/g)].map((m) => m[1]);
  for (const raw of hrefs) {
    if (
      raw.startsWith("http://") ||
      raw.startsWith("https://") ||
      raw.startsWith("mailto:") ||
      raw.startsWith("#") ||
      raw.startsWith("data:")
    ) {
      continue;
    }
    checked += 1;
    // Strip hash and query for file resolution.
    const clean = raw.split("#")[0].split("?")[0];
    if (clean === "") continue;
    const target = resolveTarget(clean, file);
    if (!existsSync(target)) {
      errors += 1;
      console.error(`BROKEN ${file.replace(dist, "")} -> ${raw}`);
    }
  }
}

// The Playground controller expects these exact paths relative to /playground/.
const required = [
  "playground/index.html",
  "playground/web/app.js",
  "playground/web/worker.js",
  "playground/web/runtime.mjs",
  "playground/runtimes/manifest.json",
  "robots.txt",
  "sitemap.xml",
  "404.html",
  ".nojekyll",
  "assets/tokens.css",
  "assets/styles.css",
  "assets/site.js",
  "assets/favicon.svg",
  "assets/social-card.svg",
];
for (const r of required) {
  if (!existsSync(join(dist, r))) {
    errors += 1;
    console.error(`MISSING required build file: ${r}`);
  }
}

// Every artifact named by the manifest must exist.
try {
  const manifest = JSON.parse(
    readFileSync(join(dist, "playground/runtimes/manifest.json"), "utf8"),
  );
  for (const v of manifest.versions) {
    if (!v.available) continue;
    const p = join(dist, "playground/runtimes", v.artifact);
    if (!existsSync(p)) {
      errors += 1;
      console.error(`MISSING runtime artifact: ${v.artifact}`);
    }
  }
} catch (err) {
  errors += 1;
  console.error(`cannot read manifest: ${err.message}`);
}

console.log(
  `\nLinks: ${checked - errors} checked OK across ${htmlFiles.length} pages, ${errors} error(s)`,
);
process.exit(errors === 0 ? 0 : 1);
