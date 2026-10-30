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
 * The newest mtime under `path`, or 0 when it does not exist. Directories are
 * walked recursively so a change to any source file counts; a plain file is
 * its own answer.
 */
function newestMtime(path) {
  if (!existsSync(path)) return 0;
  const stat = statSync(path);
  if (!stat.isDirectory()) return stat.mtimeMs;
  let newest = 0;
  for (const entry of readdirSync(path, { withFileTypes: true })) {
    const mtime = newestMtime(join(path, entry.name));
    if (mtime > newest) newest = mtime;
  }
  return newest;
}

/**
 * True when the served copy is older than the sources it was built from.
 *
 * The browser suites serve `website/dist`, which the build generates from
 * `website/pages`, `website/lib`, `website/assets`, the site config, and the
 * Playground's `web` and `runtimes` trees. A stale `dist` therefore runs *old*
 * Playground code and old HTML: a suite can pass against JavaScript that no
 * longer exists in the tree, or fail against a fix that has not been rebuilt.
 *
 * Comparing against the newest file anywhere under `dist` (rather than one
 * copied subdirectory) also covers assets and runtimes, which are copied to
 * other paths and would otherwise go unchecked. The build rewrites every file
 * with a fresh mtime, so any source newer than the whole output means stale.
 */
function isStale() {
  const dist = join(repo, "website", "dist");
  if (!existsSync(join(dist, "index.html"))) return true;
  if (!existsSync(join(dist, "playground", "web"))) return true;
  const newestSource = Math.max(
    newestMtime(join(repo, "playground", "web")),
    newestMtime(join(repo, "playground", "runtimes")),
    newestMtime(join(repo, "website", "pages")),
    newestMtime(join(repo, "website", "lib")),
    newestMtime(join(repo, "website", "assets")),
    newestMtime(join(repo, "website", "build.mjs")),
    newestMtime(join(repo, "website", "site.config.mjs")),
  );
  return newestSource > newestMtime(dist);
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
