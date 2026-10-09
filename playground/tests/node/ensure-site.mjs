// Ensure a built website exists so the browser HTTP/security suites can serve
// the real Playground page. The Playground CI job does not build the site, so
// these tests build it on demand (idempotent; the website build is fast and
// only runs when `website/dist` is absent or stale).
//
// This is test infrastructure, not production code: it never runs during a
// normal build, only when a browser suite needs a served page.

import { existsSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const repo = resolve(here, "../../..");

/** Ensure `website/dist` exists; build it if not. Returns the repo root. */
export function ensureSite() {
  const dist = join(repo, "website", "dist");
  const home = join(dist, "index.html");
  if (!existsSync(home)) {
    execFileSync("node", [join(repo, "website", "build.mjs")], {
      cwd: repo,
      stdio: "inherit",
    });
  }
  return repo;
}
