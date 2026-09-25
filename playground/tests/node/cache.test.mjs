// Regression: the runtime selector must reflect the *current* manifest even
// when a browser has a fresh, cached copy of an older manifest.
//
// The runtime manifest is mutable metadata: it changes whenever a runtime
// version is added. GitHub Pages serves it with `Cache-Control: max-age=600`,
// so a plain `fetch("./runtimes/manifest.json")` can be served from the
// browser cache for up to ten minutes after a deploy. That is exactly how a
// newly published `0.0.2-dev` runtime went missing from the live selector:
// the page rendered the cached two-entry manifest instead of the deployed
// three-entry one.
//
// This test reproduces that flow against a real browser:
//   1. serve a two-entry manifest (0.0.1, 0.0.2) with `max-age=600`;
//   2. load the Playground so the browser caches that manifest;
//   3. switch the server to the three-entry manifest (adding 0.0.2-dev);
//   4. reload the Playground;
//   5. prove all three entries render and 0.0.2-dev is current.
//
// A selector that used a cacheable fetch would still show two entries at
// step 5 and fail here; the revalidating fetch shows three.
//
// Usage: node playground/tests/node/cache.test.mjs
// Requires the `playwright` package with Chromium installed.

import { createServer } from "node:http";
import { readFile, stat } from "node:fs/promises";
import { extname, normalize, join, resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

let chromium;
try {
  ({ chromium } = await import("playwright"));
} catch {
  console.error("cache.test.mjs: `playwright` is not installed; skipping.");
  process.exit(0);
}

const here = dirname(fileURLToPath(import.meta.url));
const playground = resolve(here, "../..");

const TYPES = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".mjs": "text/javascript; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".json": "application/json; charset=utf-8",
  ".wasm": "application/wasm",
};

const MANIFEST_V1 = JSON.stringify(
  {
    playground_api_version: 1,
    current: "0.0.2",
    versions: [
      {
        id: "0.0.1",
        release_version: "0.0.1",
        language_version: "0.0.1",
        runtime_version: null,
        host_abi_version: null,
        available: false,
        channel: "release",
        reason: "The published 0.0.1 predates the WebAssembly execution substrate.",
        artifact: null,
        sha256: null,
        bytes: null,
      },
      {
        id: "0.0.2",
        release_version: "0.0.2",
        language_version: "0.0.1",
        runtime_version: "0.0.2",
        host_abi_version: 1,
        available: true,
        channel: "release",
        artifact: "0.0.2/aura_playground_runtime.wasm",
        sha256: "x",
        bytes: 1,
      },
    ],
  },
  null,
  2,
);

const MANIFEST_V2 = JSON.stringify(
  {
    playground_api_version: 1,
    current: "0.0.2-dev",
    versions: [
      ...JSON.parse(MANIFEST_V1).versions,
      {
        id: "0.0.2-dev",
        release_version: "0.0.2",
        language_version: "0.0.1",
        runtime_version: "0.0.2-dev",
        host_abi_version: 1,
        available: true,
        channel: "development",
        artifact: "0.0.2-dev/aura_playground_runtime.wasm",
        sha256: "y",
        bytes: 2,
      },
    ],
  },
  null,
  2,
);

// The server serves real files from the Playground, but the manifest is
// served from an in-memory body so the test can flip it mid-flight. It sets
// the same `max-age=600` the real host sends, which is what makes a
// non-revalidating fetch go stale.
let manifestBody = MANIFEST_V1;

const server = createServer(async (req, res) => {
  try {
    const url = new URL(req.url, "http://localhost");
    let pathname = decodeURIComponent(url.pathname);
    if (pathname === "/") pathname = "/index.html";
    if (pathname === "/runtimes/manifest.json") {
      res.writeHead(200, {
        "content-type": "application/json; charset=utf-8",
        "cache-control": "max-age=600",
      });
      res.end(manifestBody);
      return;
    }
    const target = join(playground, normalize(pathname).replace(/^(\.\.[/\\])+/, ""));
    const info = await stat(target).catch(() => null);
    if (!info || info.isDirectory()) {
      res.writeHead(404).end("not found");
      return;
    }
    res.writeHead(200, { "content-type": TYPES[extname(target)] || "application/octet-stream" });
    res.end(await readFile(target));
  } catch (err) {
    res.writeHead(500).end(String(err));
  }
});

const { port } = await new Promise((resolveServer) => {
  server.listen(0, "127.0.0.1", () => resolveServer(server.address()));
});
const base = `http://127.0.0.1:${port}/`;

const browser = await chromium.launch();
const context = await browser.newContext();
const page = await context.newPage();

let passed = 0;
let failed = 0;
function check(name, cond, detail) {
  if (cond) passed += 1;
  else {
    failed += 1;
    console.error(`FAIL ${name}${detail ? `: ${detail}` : ""}`);
  }
}

async function selectorValues() {
  return page.evaluate(() =>
    [...document.querySelectorAll("#version option")].map((o) => o.value),
  );
}

// 1. First load caches the two-entry manifest.
await page.goto(base, { waitUntil: "networkidle" });
await page.waitForFunction(() => document.querySelectorAll("#version option").length > 0);
const first = await selectorValues();
check("initial load shows two entries", first.length === 2, JSON.stringify(first));
check("initial load has no 0.0.2-dev", !first.includes("0.0.2-dev"), JSON.stringify(first));

// 2. The deploy adds the development runtime.
manifestBody = MANIFEST_V2;

// 3. Reload: the selector must reflect the deployed manifest, not the cached
//    two-entry copy.
await page.reload({ waitUntil: "networkidle" });
await page.waitForFunction(() => document.querySelectorAll("#version option").length > 0);
const second = await selectorValues();
check(
  "reload picks up the newly published 0.0.2-dev",
  second.includes("0.0.2-dev"),
  JSON.stringify(second),
);
check("reload still lists 0.0.1 and 0.0.2", second.includes("0.0.1") && second.includes("0.0.2"));

const selected = await page.evaluate(() => document.getElementById("version").value);
check("0.0.2-dev is selected as current", selected === "0.0.2-dev", selected);
const devDisabled = await page.evaluate(
  () => document.querySelector('#version option[value="0.0.2-dev"]')?.disabled ?? true,
);
check("0.0.2-dev is selectable", devDisabled === false);
const oldDisabled = await page.evaluate(
  () => document.querySelector('#version option[value="0.0.1"]')?.disabled ?? false,
);
check("0.0.1 remains unavailable", oldDisabled === true);

await browser.close();
server.close();

console.log(`\nCache: ${passed} passed, ${failed} failed`);
process.exit(failed === 0 ? 0 : 1);
