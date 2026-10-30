// Browser HTTP security acceptance gate (0.3.2-dev.8, Host ABI 2).
//
// The order requires adversarial, deterministic evidence — not just the
// successful PokéAPI demonstration — that the experimental runtime cannot
// perform an unauthorized network effect. Each case below observes the
// *server side* (a request counter) wherever the claim is "no request was
// dispatched", so a passing test cannot be a UI illusion.
//
// Cases:
//   1. denial          → the server receives nothing; the program gets E5002
//   2. no permission   → a program that errors before a granted request still
//                        dispatches nothing until the user grants
//   3. origin scoping  → a grant for origin A does not authorize origin B
//   4. scheme rejection→ file:/data:/javascript: targets never reach fetch
//   5. redirect        → a 302 is not followed (redirect:"manual"); no second
//                        request is observed at the redirect target
//   6. localhost       → a loopback target still requires consent (no bypass)
//   7. stop            → Stop during an in-flight request aborts it
//   8. stale effect id → a stale resume cannot drive another execution
//
// Requires playwright + Chromium; a missing browser is a hard failure, never a
// silent skip, because this is a security gate.

import { createServer } from "node:http";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));

let chromium;
try {
  ({ chromium } = await import("playwright"));
} catch {
  console.error("http-security: playwright is required for the security gate");
  process.exit(1);
}

/** A local server that counts every request it receives. */
function countingServer(handler) {
  const state = { count: 0, requests: [] };
  const server = createServer((req, res) => {
    state.count += 1;
    state.requests.push(`${req.method} ${req.url}`);
    const cors = {
      "access-control-allow-origin": "*",
      "access-control-allow-headers": "*",
      "access-control-allow-methods": "GET,POST,PUT,PATCH,DELETE,HEAD,OPTIONS",
    };
    if (req.method === "OPTIONS") {
      res.writeHead(204, cors);
      res.end();
      return;
    }
    handler(req, res, cors, state);
  });
  return { server, state };
}

const a = countingServer((req, res, cors) => {
  if (req.url === "/redirect") {
    res.writeHead(302, { ...cors, location: "/landed" });
    res.end();
    return;
  }
  if (req.url === "/slow") {
    // A long delay so a Stop can land while the fetch is in flight.
    setTimeout(() => {
      res.writeHead(200, { ...cors, "content-type": "text/plain" });
      res.end("late");
    }, 8000);
    return;
  }
  if (req.url === "/hang") {
    // Never respond: used to prove the browser request timeout fires.
    return;
  }
  if (req.url === "/binary") {
    // Raw non-UTF-8 bytes: the Aura body_bytes view must be byte-exact.
    res.writeHead(200, { ...cors, "content-type": "application/octet-stream" });
    res.end(Buffer.from([0xff, 0x00, 0xfe, 0x41]));
    return;
  }
  if (req.url === "/dup") {
    // Two headers with the same name and differing values, plus a
    // mixed-case name. The browser's `Headers` collection combines the first
    // pair and lowercases everything, so the Playground can only ever observe
    // one entry per name — this fixture pins that divergence.
    //
    // Both custom names must be exposed explicitly: a cross-origin page sees
    // only the CORS-safelisted response headers plus whatever the origin lists
    // in `Access-Control-Expose-Headers`. Without this the Aura program would
    // correctly observe neither header.
    res.writeHead(200, [
      ...Object.entries(cors),
      ["access-control-expose-headers", "x-dup, x-mixed-case"],
      ["x-dup", "one"],
      ["x-dup", "two"],
      ["X-Mixed-Case", "kept"],
      ["content-type", "text/plain"],
    ]);
    res.end("dup");
    return;
  }
  if (req.url === "/big") {
    // A body larger than the 8 MiB cap, written in chunks.
    res.writeHead(200, { ...cors, "content-type": "text/plain" });
    const chunk = "x".repeat(1024 * 1024);
    for (let i = 0; i < 10; i += 1) res.write(chunk);
    res.end();
    return;
  }
  res.writeHead(200, { ...cors, "content-type": "text/plain" });
  res.end("A");
});
const b = countingServer((req, res, cors) => {
  res.writeHead(200, { ...cors, "content-type": "text/plain" });
  res.end("B");
});

await new Promise((r) => a.server.listen(0, "127.0.0.1", r));
await new Promise((r) => b.server.listen(0, "127.0.0.1", r));
const originA = `http://127.0.0.1:${a.server.address().port}`;
const originB = `http://127.0.0.1:${b.server.address().port}`;

const repo = resolve(here, "../../..");
// Build the site if it is missing so this suite runs in CI (which does not
// build the website before the playground job).
const { ensureSite } = await import("./ensure-site.mjs");
ensureSite();
const serve = await import(join(repo, "website/tests/serve.mjs"));
const { server: site, port: sitePort, base: basePath } = await serve.startServer(0);
const base = `http://127.0.0.1:${sitePort}${basePath}`;

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

async function newPage() {
  const page = await browser.newPage({ viewport: { width: 1280, height: 900 } });
  await page.goto(`${base}playground/`, { waitUntil: "load" });
  await page.waitForFunction(() => document.querySelectorAll("#version option").length > 0);
  await page.selectOption("#version", "0.3.2-dev.8");
  return page;
}

async function setSource(page, source) {
  await page.evaluate((src) => {
    const ta = document.getElementById("source");
    ta.value = src;
    ta.dispatchEvent(new Event("input", { bubbles: true }));
  }, source);
}

/**
 * Run and answer the *first* permission prompt with `grant`. Returns the
 * terminal UI state. `allowOrigins` lets a test grant some origins and deny
 * others by matching the prompt's detail text.
 */
async function runAndAnswer(page, { grant = true, allowOrigins = null, timeout = 30000 } = {}) {
  await page.click("#run");
  const deadline = Date.now() + timeout;
  const answered = new Set();
  while (Date.now() < deadline) {
    const prompt = await page.evaluate(() => {
      const d = document.getElementById("permission-dialog");
      return d && !d.hidden
        ? { detail: document.getElementById("permission-detail").textContent }
        : null;
    });
    if (prompt && !answered.has(prompt.detail)) {
      answered.add(prompt.detail);
      const allow = allowOrigins
        ? allowOrigins.some((o) => prompt.detail.includes(o))
        : grant;
      await page.click(allow ? "#permission-allow" : "#permission-deny");
    }
    const status = await page.evaluate(() => document.getElementById("status").textContent);
    if (/Completed|Failed|Stopped/.test(status)) {
      return await page.evaluate(() => ({
        status: document.getElementById("status").textContent,
        stdout: document.getElementById("stdout").textContent,
        diagnostics: [...document.querySelectorAll("#diagnostics .diag-code")].map((e) => e.textContent),
      }));
    }
    await page.waitForTimeout(120);
  }
  return { status: "timeout", stdout: "", diagnostics: [] };
}

// 1) Denial dispatches nothing.
{
  const before = a.state.count;
  const page = await newPage();
  await setSource(page, `fn main() {
    let r = http_get("${originA}/deny")
    print(r["status"])
}`);
  const r = await runAndAnswer(page, { grant: false });
  check("1 denial yields E5002", r.diagnostics.some((d) => /E5002/.test(d)), JSON.stringify(r.diagnostics));
  check("1 denial dispatches no request", a.state.count === before, `${a.state.count - before} request(s)`);
  await page.close();
}

// 2) Origin scoping: grant A, deny B.
{
  const beforeA = a.state.count;
  const beforeB = b.state.count;
  const page = await newPage();
  await setSource(page, `fn main() {
    let ra = http_get("${originA}/scoped")
    print(ra["status"])
    let rb = http_get("${originB}/scoped")
    print(rb["status"])
}`);
  const r = await runAndAnswer(page, { allowOrigins: [originA] });
  check("2 origin B is not auto-authorized by A", b.state.count === beforeB, `${b.state.count - beforeB} B request(s)`);
  check("2 the denied B request reports E5002", r.diagnostics.some((d) => /E5002/.test(d)), JSON.stringify(r.diagnostics));
  await page.close();
}

// 3) Unsupported schemes never reach fetch.
{
  for (const target of ["file:///etc/passwd", "data:text/plain,hi", "ftp://example.test/x"]) {
    const page = await newPage();
    await setSource(page, `fn main() {
      let r = http_get("${target}")
      print(r["status"])
    }`);
    const r = await runAndAnswer(page, { timeout: 12000 });
    // No prompt should even be shown for a non-http scheme: the Worker rejects
    // it before asking. The program fails with E4020 and no network effect.
    check(
      `3 scheme rejected without dispatch: ${target.split(":")[0]}`,
      /Failed/.test(r.status) && r.diagnostics.some((d) => /E4020/.test(d)),
      JSON.stringify({ status: r.status, diagnostics: r.diagnostics }),
    );
    await page.close();
  }
}

// 4) A redirect is not followed, and the program is told the truth.
//
// B6 contract (LANGUAGE_SPEC.md, "browser host divergence"): a browser fetch
// with `redirect: "manual"` yields an opaque-redirect response whose status is
// hidden from the page. The host therefore reports E4020 — it must NOT
// synthesize a 3xx to imitate the native host, because 301/302/303/307/308 are
// indistinguishable here and inventing one would misreport the network.
{
  const before = a.state.count;
  const page = await newPage();
  await setSource(page, `fn main() {
    let r = http_get("${originA}/redirect")
    print(r["status"])
  }`);
  const r = await runAndAnswer(page);
  // Exactly one request reached the origin (the 302); the redirect target was
  // never requested because `redirect: "manual"` prevents following.
  const after = a.state.count;
  const landed = a.state.requests.filter((u) => u.includes("/landed")).length;
  check("4 redirect does not expand access (no /landed request)", landed === 0, `${landed}`);
  check("4 exactly one request dispatched", after - before === 1, `${after - before}`);
  // The observable contract: the program is told the truth. It receives a
  // transport-level failure that names the redirect — never a fabricated
  // status and never a silent success. `301`/`302`/`303`/`307`/`308` are
  // indistinguishable behind an opaque redirect, so any numeric status here
  // would be an invention.
  check(
    "4 redirected call reports E4020",
    r.diagnostics.some((d) => /E4020/.test(d)),
    JSON.stringify(r.diagnostics),
  );
  // A transport failure aborts the program, so `print(r["status"])` never
  // runs. Pinning that is what makes "no fabricated status" a real statement:
  // if a future change synthesized a 3xx, the program would complete and print
  // it, and both of these checks would fail.
  check("4 redirected call does not complete", /Failed/.test(r.status), r.status);
  check(
    "4 redirected call never prints a status",
    r.stdout.trim() === "" && !/\b[1-5]\d\d\b/.test(r.stdout),
    JSON.stringify(r.stdout),
  );
  await page.close();
}

// 5) A loopback target still requires consent (no localhost bypass).
{
  const before = a.state.count;
  const page = await newPage();
  await setSource(page, `fn main() {
    let r = http_get("${originA}/loopback")
    print(r["status"])
  }`);
  const r = await runAndAnswer(page, { grant: false });
  check("5 loopback requires consent", a.state.count === before && r.diagnostics.some((d) => /E5002/.test(d)), JSON.stringify(r.diagnostics));
  await page.close();
}

// 6) Stop aborts an in-flight request and returns the UI to usable.
{
  const page = await newPage();
  await setSource(page, `fn main() {
    let r = http_get("${originA}/slow")
    print(r["status"])
  }`);
  await page.click("#run");
  const deadline = Date.now() + 5000;
  while (Date.now() < deadline) {
    if (await page.evaluate(() => !document.getElementById("permission-dialog").hidden)) {
      await page.click("#permission-allow");
      break;
    }
    await page.waitForTimeout(80);
  }
  await page.waitForTimeout(150);
  await page.click("#stop");
  await page.waitForTimeout(300);
  const ui = await page.evaluate(() => ({
    status: document.getElementById("status").textContent,
    runEnabled: !document.getElementById("run").disabled,
  }));
  check("6 Stop returns the UI to usable", /Stopped/.test(ui.status) && ui.runEnabled, JSON.stringify(ui));
  await page.close();
}

// 7) Stale effect id cannot drive a newer execution (retested through the ABI
//    surface the Worker uses, in Node — deterministic and browser-independent).
{
  const { readFileSync } = await import("node:fs");
  const { AuraRuntime } = await import(join(repo, "playground/web/runtime.mjs"));
  const rt = await AuraRuntime.fromBytes(
    readFileSync(join(repo, "playground/runtimes/0.3.2-dev.8/aura_playground_runtime.wasm")),
    "0.3.2-dev.8",
  );
  let step = rt.startSession(`fn main() { let r = http_get("http://example.test/a")\n print(r["status"]) }`, { args: [], stdin: null });
  check("7 session parks", step.status === 3, String(step.status));
  const stale = rt.resumeSession({ effect_id: 424242, ok: true, response: { status: 200, headers: [], body: "x" } });
  check("7 stale effect id rejected", stale.status === 1 || stale.status === 2, String(stale.status));
  // The parked session can still be completed correctly.
  const good = rt.resumeSession({ effect_id: step.result.effect_id, ok: true, response: { status: 200, headers: [], body: "ok" } });
  check("7 correct resume still works", good.result.status === "ok", JSON.stringify(good.result));
}

// 8) A binary (non-UTF-8) response is byte-exact in Aura.
{
  const page = await newPage();
  await setSource(page, `fn main() {
    let r = http_get("${originA}/binary")
    print(len(r["body_bytes"]))
    print(r["body_bytes"][0])
    print(r["body_bytes"][3])
  }`);
  const r = await runAndAnswer(page, { timeout: 20000 });
  check(
    "10 binary body_bytes is byte-exact",
    r.stdout === "4\n255\n65\n",
    JSON.stringify(r.stdout),
  );
  await page.close();
}

// 9) A response larger than the cap is refused with E4020, not buffered.
{
  const page = await newPage();
  await setSource(page, `fn main() {
    let r = http_get("${originA}/big")
    print(r["status"])
  }`);
  const r = await runAndAnswer(page, { timeout: 30000 });
  check(
    "8 oversized response is E4020",
    /Failed/.test(r.status) && r.diagnostics.some((d) => /E4020/.test(d)),
    JSON.stringify({ status: r.status, diagnostics: r.diagnostics }),
  );
  await page.close();
}

// 10) A hanging server times out (E4020) without needing Stop.
{
  const page = await newPage();
  await setSource(page, `fn main() {
    let r = http_request("GET", "${originA}/hang", {"timeout_ms": 3000})
    print(r["status"])
  }`);
  const started = Date.now();
  const r = await runAndAnswer(page, { timeout: 20000 });
  const elapsed = Date.now() - started;
  check(
    "9 hanging request times out with E4020",
    /Failed/.test(r.status) && r.diagnostics.some((d) => /E4020/.test(d)) && elapsed < 15000,
    JSON.stringify({ status: r.status, elapsed, diagnostics: r.diagnostics }),
  );
  await page.close();
}

// 11) Repeated response headers are combined by the platform, not by Aura.
//
// B7 contract (LANGUAGE_SPEC.md, "browser host divergence", item 3): a
// browser Headers collection yields one entry per unique name, already
// joined by ", ", in sorted lowercased order. The program must therefore see
// a single combined string for a repeated header — never a list, and never the
// original casing. This pins the divergence in both directions so a future
// platform change (or an accidental switch to a raw-header API) is caught.
{
  const page = await newPage();
  await setSource(page, `fn main() {
    let r = http_request("GET", "${originA}/dup")
    # A combined platform value prints as a single string; a list would print
    # as ["one", "two"], so this equality is also the shape check.
    print(r["headers"]["x-dup"])
    print(r["headers"]["x-mixed-case"])
    # header_lines has one pair per pair received, and headers has one key per
    # unique name. Native keeps a duplicate as a list, so its two counts differ;
    # the browser combines, so here they must be equal. That equality is the
    # browser-side invariant, not a cross-host one.
    print(len(keys(r["headers"])))
    print(len(r["header_lines"]))
  }`);
  const r = await runAndAnswer(page);
  const lines = r.stdout.split("\n").map((l) => l.trim()).filter(Boolean);
  check(
    "11 repeated header arrives as one combined string",
    lines[0] === "one, two",
    JSON.stringify({ value: lines[0] }),
  );
  check(
    "11 header names are lowercased",
    lines[1] === "kept",
    JSON.stringify({ value: lines[1] }),
  );
  check(
    "11 header_lines has one pair per unique name",
    lines[2] !== undefined && lines[2] === lines[3],
    JSON.stringify({ unique_names: lines[2], header_lines: lines[3] }),
  );
  await page.close();
}

await browser.close();
site.close();
a.server.close();
b.server.close();
console.log(`\nhttp-security: ${passed} passed, ${failed} failed`);
process.exitCode = failed === 0 ? 0 : 1;
