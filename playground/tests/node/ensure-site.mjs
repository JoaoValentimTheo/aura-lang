// Ensure a built website exists so the browser HTTP/security suites can serve
// the real Playground page. The Playground CI job does not build the site, so
// these tests build it on demand (idempotent; the website build is fast and
// only runs when `website/dist` is absent or stale).
//
// This is test infrastructure, not production code: it never runs during a
// normal build, only when a browser suite needs a served page.

import { existsSync, readdirSync, statSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const repo = resolve(here, "../../..");

/**
 * The newest mtime under `dir`, or 0 when it does not exist. Recursive so a
 * change to any source file counts.
 */
function newestMtime(dir) {
  if (!existsSync(dir)) return 0;
  let newest = 0;
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const full = join(dir, entry.name);
    const mtime = entry.isDirectory() ? newestMtime(full) : statSync(full).mtimeMs;
    if (mtime > newest) newest = mtime;
  }
  return newest;
}

/**
 * True when the served copy is older than the sources it was built from.
 *
 * The browser suites serve `website/dist`, and the build copies
 * `playground/web` into it verbatim. A stale `dist` therefore runs *old*
 * Playground code: a suite can pass against JavaScript that no longer exists
 * in the tree, or fail against a fix that has not been rebuilt. Comparing
 * mtimes catches both directions.
 */
function isStale() {
  const dist = join(repo, "website", "dist");
  const served = join(dist, "playground", "web");
  if (!existsSync(join(dist, "index.html"))) return true;
  if (!existsSync(served)) return true;
  const newestSource = Math.max(
    newestMtime(join(repo, "playground", "web")),
    newestMtime(join(repo, "website", "pages")),
    newestMtime(join(repo, "website", "lib")),
  );
  return newestSource > newestMtime(served);
}

/** Ensure `website/dist` exists and reflects the current sources. */
export function ensureSite() {
  if (isStale()) {
    execFileSync("node", [join(repo, "website", "build.mjs")], {
      cwd: repo,
      stdio: "inherit",
    });
  }
  return repo;
}
