// Browser-host session + HTTP transport, end to end (0.3.2-dev.5).
//
// This drives the *real* ABI 2 session protocol that the Worker uses, in Node:
// start a session, get a pending HTTP effect, perform a real request against a
// loopback server (the same shape the Worker's `fetch` produces), and resume.
// It proves the runtime side of browser HTTP without a browser, and the
// browser suite proves the Worker/browser side against the same protocol.
//
// The local server is deterministic and CORS-free (Node has no CORS), so this
// test is not a CORS test — the browser suite owns CORS.

import { createServer } from "node:http";
import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const repo = resolve(here, "../..");
const { AuraRuntime } = await import(join(repo, "web/runtime.mjs"));

let passed = 0;
let failed = 0;
function check(name, cond, detail) {
  if (cond) passed += 1;
  else {
    failed += 1;
    console.error(`FAIL ${name}${detail ? `: ${detail}` : ""}`);
  }
}

// A deterministic loopback server exercising every documented method.
const server = createServer((req, res) => {
  const chunks = [];
  req.on("data", (c) => chunks.push(c));
  req.on("end", () => {
    const body = Buffer.concat(chunks).toString();
    if (req.url === "/json") {
      res.writeHead(200, { "content-type": "application/json", "x-aura-test": "yes" });
      res.end(JSON.stringify({ name: "pikachu", id: 25 }));
      return;
    }
    if (req.url === "/echo") {
      res.writeHead(200, { "content-type": "text/plain" });
      res.end(`${req.method} ${body} ${req.headers["x-aura-test"] || ""}`);
      return;
    }
    res.writeHead(404, { "content-type": "text/plain" });
    res.end("nope");
  });
});
await new Promise((r) => server.listen(0, "127.0.0.1", r));
const port = server.address().port;
const base = `http://127.0.0.1:${port}`;

const wasm = readFileSync(join(repo, "runtimes/0.3.2-dev.5/aura_playground_runtime.wasm"));
const rt = await AuraRuntime.fromBytes(wasm, "0.3.2-dev.5");

check("dev.2 advertises sessions", rt.supportsSessions === true);
check("dev.2 ABI is 2", rt.abiVersion === 2, String(rt.abiVersion));

/** Drive a session, performing each HTTP effect via Node fetch. */
async function runWithHttp(source, { grant = true } = {}) {
  let step = rt.startSession(source, { args: [], stdin: null });
  let effects = 0;
  while (step.status === 3) {
    const effect = step.result;
    effects += 1;
    if (!grant) {
      step = rt.resumeSession({ effect_id: effect.effect_id, ok: false, code: 5002, message: "denied" });
      continue;
    }
    // Node fetch mirrors the Worker transport's request shape.
    try {
      const init = { method: effect.method, headers: new Headers() };
      for (const [k, v] of effect.headers || []) init.headers.append(k, v);
      if (typeof effect.body === "string") init.body = effect.body;
      const resp = await fetch(effect.url, init);
      const text = await resp.text();
      const headers = [];
      resp.headers.forEach((v, k) => headers.push([k, v]));
      step = rt.resumeSession({
        effect_id: effect.effect_id,
        ok: true,
        response: { status: resp.status, headers, body: text },
      });
    } catch (err) {
      step = rt.resumeSession({
        effect_id: effect.effect_id,
        ok: false,
        code: 4020,
        message: `fetch failed: ${err.message}`,
      });
    }
  }
  return { step, effects };
}

// GET → JSON → decode (the PokéAPI shape, against a local fixture).
{
  const { step, effects } = await runWithHttp(`fn main() {
    let r = http_get("${base}/json")
    print(r["status"])
    let p = json_decode(r["body"])
    print(p["name"])
}`);
  check("GET json completes ok", step.result.status === "ok", JSON.stringify(step.result));
  check("GET json stdout", step.result.stdout === "200\npikachu\n", JSON.stringify(step.result.stdout));
  check("GET json one effect", effects === 1, String(effects));
}

// Headers actually transmitted.
{
  const { step } = await runWithHttp(`fn main() {
    let r = http_request("GET", "${base}/echo", {"headers": [["X-Aura-Test", "sent"]]})
    print(r["body"])
}`);
  check(
    "request headers reach the server",
    step.result.stdout === "GET  sent\n",
    JSON.stringify(step.result.stdout),
  );
}

// POST body transmitted.
{
  const { step } = await runWithHttp(`fn main() {
    let r = http_request("POST", "${base}/echo", {"body": "payload"})
    print(r["body"])
}`);
  check(
    "POST body reaches the server",
    step.result.stdout === "POST payload \n",
    JSON.stringify(step.result.stdout),
  );
}

// 404 is an ordinary status, not a failure.
{
  const { step } = await runWithHttp(`fn main() {
    let r = http_get("${base}/missing")
    print(r["status"])
    print(r["body"])
}`);
  check("404 is an ordinary response", step.result.stdout === "404\nnope\n", JSON.stringify(step.result.stdout));
}

// Denied permission becomes E5002 and is not dispatched.
{
  const { step, effects } = await runWithHttp(
    `fn main() { let r = http_get("${base}/json")\n print(r["status"]) }`,
    { grant: false },
  );
  check(
    "denied request fails with E5002",
    JSON.stringify(step.result).includes("E5002"),
    JSON.stringify(step.result).slice(0, 200),
  );
  check("denied request still counted one effect", effects === 1, String(effects));
}

// Stale effect id is rejected.
{
  let step = rt.startSession(`fn main() { let r = http_get("${base}/json")\n print(r["status"]) }`, { args: [], stdin: null });
  check("pending before stale test", step.status === 3);
  const stale = rt.resumeSession({ effect_id: 9999, ok: true, response: { status: 200, headers: [], body: "x" } });
  check("stale effect id is rejected", stale.status !== 0 && stale.status !== 3, String(stale.status));
  // The session is still parked and can be resumed correctly.
  const good = rt.resumeSession({ effect_id: step.result.effect_id, ok: true, response: { status: 200, headers: [], body: "ok" } });
  check("correct resume after stale succeeds", good.result.status === "ok", JSON.stringify(good.result));
}

server.close();
console.log(`\nsession-http: ${passed} passed, ${failed} failed`);
process.exit(failed === 0 ? 0 : 1);
