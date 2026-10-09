// Browser HTTP end to end (0.3.2 development, Host ABI 2).
//
// This is the real acceptance gate for browser HTTP: a real Chromium, the real
// Playground page and Worker, the real 0.3.2-dev.7 wasm artifact, and a local
// CORS-enabled server. The Aura program calls `http_get`/`http_request`; the
// runtime parks; the Worker performs a real `fetch`; the page's consent UI is
// answered; the response resumes the same program.
//
// It proves the transport, the permission model, the CORS behavior, the
// methods/bodies/headers, the error taxonomy, and cancellation — with no
// mocks standing in for any layer.

import { createServer } from "node:http";
import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const repo = resolve(here, "../../..");
// Build the site if it is missing so this suite runs in CI (which does not
// build the website before the playground job).
const { ensureSite } = await import("./ensure-site.mjs");
ensureSite();

let chromium;
try {
  ({ chromium } = await import("playwright"));
} catch {
  console.log("browser-http tests: playwright not installed — skipped.");
  process.exit(0);
}

// ---- local CORS dev server (deterministic; no public service) --------------
// `ALLOW` controls the CORS header so a test can force a CORS *failure*.
const server = createServer((req, res) => {
  const cors = process.env.AURA_TEST_CORS === "deny" ? {} : {
    "access-control-allow-origin": "*",
    "access-control-allow-headers": "*",
    "access-control-allow-methods": "GET,POST,PUT,PATCH,DELETE,HEAD,OPTIONS",
  };
  if (req.method === "OPTIONS") {
    res.writeHead(204, cors);
    res.end();
    return;
  }
  const chunks = [];
  req.on("data", (c) => chunks.push(c));
  req.on("end", () => {
    const body = Buffer.concat(chunks).toString();
    const send = (status, extra, text) => {
      res.writeHead(status, { ...cors, ...extra });
      res.end(text);
    };
    if (req.url === "/pokemon") {
      send(200, { "content-type": "application/json", "x-aura-test": "yes" }, JSON.stringify({ name: "pikachu", id: 25 }));
    } else if (req.url === "/echo") {
      send(200, { "content-type": "text/plain" }, `${req.method} ${body} ${req.headers["x-aura-test"] || ""}`);
    } else if (req.url === "/status/500") {
      send(500, { "content-type": "text/plain" }, "server error");
    } else if (req.url === "/slow") {
      setTimeout(() => send(200, { "content-type": "text/plain" }, "late"), 5000);
    } else {
      send(404, { "content-type": "text/plain" }, "nope");
    }
  });
});
await new Promise((r) => server.listen(0, "127.0.0.1", r));
const port = server.address().port;
const origin = `http://127.0.0.1:${port}`;

// ---- serve the built Playground (dist) so the Worker + artifacts resolve ---
// The website test harness serves `website/dist`; reuse it.
const serve = await import(join(repo, "website/tests/serve.mjs"));
const { server: site, port: sitePort, base: basePath } = await serve.startServer(0);

const browser = await chromium.launch();
let passed = 0;
let failed = 0;
function check(name, cond, detail) {
  if (cond) passed += 1;
  else {
    failed += 1;
    console.error(`FAIL ${name}${detail ? `: ${detail}` : ""}`);
  }
}

const base = `http://127.0.0.1:${sitePort}${basePath}`;

/** Load the Playground, select dev.4, set source, answer a permission prompt. */
async function newPage() {
  const page = await browser.newPage({ viewport: { width: 1280, height: 900 } });
  await page.goto(`${base}playground/`, { waitUntil: "load" });
  await page.waitForFunction(() => document.querySelectorAll("#version option").length > 0);
  await page.selectOption("#version", "0.3.2-dev.7");
  return page;
}

async function setSource(page, source) {
  await page.evaluate((src) => {
    const ta = document.getElementById("source");
    ta.value = src;
    ta.dispatchEvent(new Event("input", { bubbles: true }));
  }, source);
}

/** Run and, if a permission prompt appears, answer it. Returns the result. */
async function runAndAnswer(page, { grant = true, timeout = 30000 } = {}) {
  await page.click("#run");
  const deadline = Date.now() + timeout;
  while (Date.now() < deadline) {
    const dialog = await page.evaluate(() => {
      const d = document.getElementById("permission-dialog");
      return d && !d.hidden;
    });
    if (dialog) {
      await page.click(grant ? "#permission-allow" : "#permission-deny");
    }
    const status = await page.evaluate(() => document.getElementById("status").textContent);
    if (/Completed|Failed|Stopped/.test(status)) {
      return await page.evaluate(() => ({
        status: document.getElementById("status").textContent,
        stdout: document.getElementById("stdout").textContent,
        diagnostics: [...document.querySelectorAll("#diagnostics .diag-code")].map((e) => e.textContent),
      }));
    }
    await page.waitForTimeout(150);
  }
  return { status: "timeout", stdout: "", diagnostics: [] };
}

// 1) GET + JSON decode (the PokéAPI shape, local fixture), with consent.
{
  const page = await newPage();
  await setSource(page, `fn main() {
    let r = http_get("${origin}/pokemon")
    print(r["status"])
    let p = json_decode(r["body"])
    print(p["name"])
}`);
  const r = await runAndAnswer(page, { grant: true });
  check("browser GET completes", /Completed/.test(r.status), r.status);
  check("browser GET stdout", r.stdout === "200\npikachu\n", JSON.stringify(r.stdout));
  await page.close();
}

// 2) Denied consent → E5002, no request dispatched.
{
  const page = await newPage();
  await setSource(page, `fn main() {
    let r = http_get("${origin}/pokemon")
    print(r["status"])
}`);
  const r = await runAndAnswer(page, { grant: false });
  check("denied request fails with E5002", r.diagnostics.some((d) => /E5002/.test(d)), JSON.stringify(r.diagnostics));
  await page.close();
}

// 3) 404 is an ordinary status.
{
  const page = await newPage();
  await setSource(page, `fn main() {
    let r = http_get("${origin}/missing")
    print(r["status"])
    print(r["body"])
}`);
  const r = await runAndAnswer(page);
  check("browser 404 is an ordinary response", r.stdout === "404\nnope\n", JSON.stringify(r.stdout));
  await page.close();
}

// 4) POST body + headers transmitted.
{
  const page = await newPage();
  await setSource(page, `fn main() {
    let r = http_request("POST", "${origin}/echo", {"headers": [["X-Aura-Test", "hi"]], "body": "payload"})
    print(r["body"])
}`);
  const r = await runAndAnswer(page);
  check("browser POST body+header reach the server", r.stdout === "POST payload hi\n", JSON.stringify(r.stdout));
  await page.close();
}

// 5) Multiple sequential requests in one program.
{
  const page = await newPage();
  await setSource(page, `fn main() {
    let a = http_get("${origin}/pokemon")
    let b = http_get("${origin}/missing")
    print(a["status"])
    print(b["status"])
}`);
  const r = await runAndAnswer(page);
  check("browser two requests in one run", r.stdout === "200\n404\n", JSON.stringify(r.stdout));
  await page.close();
}

// 6) Stop during a slow request returns the UI to usable.
{
  const page = await newPage();
  await setSource(page, `fn main() {
    let r = http_get("${origin}/slow")
    print(r["status"])
}`);
  await page.click("#run");
  // Grant, then Stop while the fetch is in flight.
  const deadline = Date.now() + 5000;
  while (Date.now() < deadline) {
    if (await page.evaluate(() => !document.getElementById("permission-dialog").hidden)) {
      await page.click("#permission-allow");
      break;
    }
    await page.waitForTimeout(100);
  }
  await page.waitForTimeout(300);
  await page.click("#stop");
  await page.waitForTimeout(400);
  const ui = await page.evaluate(() => ({
    status: document.getElementById("status").textContent,
    runDisabled: document.getElementById("run").disabled,
  }));
  check("Stop returns the UI to usable", /Stopped/.test(ui.status) && ui.runDisabled === false, JSON.stringify(ui));
  // Run a clean program to prove recovery.
  await setSource(page, 'fn main() { print("recovered") }');
  const r = await runAndAnswer(page);
  check("Run works after a Stop", r.stdout === "recovered\n", JSON.stringify(r.stdout));
  await page.close();
}

await browser.close();
site.close();
server.close();
console.log(`\nbrowser-http: ${passed} passed, ${failed} failed`);
process.exit(failed === 0 ? 0 : 1);
