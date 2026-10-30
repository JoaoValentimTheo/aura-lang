// Multi-file projects and HTTP, end to end (0.3.2-dev.8, Host ABI 2).
//
// B14: HTTP was reachable from a single source but not from a virtual
// *project*, because a multi-file program only had the synchronous
// `runProject` entry point — and a synchronous call cannot suspend, so there
// was nothing to suspend for. `aura_session_start_project` is the additive
// ABI 2 export that gives a project the same session the single-source path
// already had.
//
// This test proves the capability, not the symbol: the request is performed,
// the response body reaches a *non-entry* module, and the whole thing keeps
// working through the same resume contract. It also pins the feature-detect
// that lets a Host tell the two artifact generations apart without comparing
// ABI numbers.
//
// Usage: node project-http.test.mjs [runtime.wasm]

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

const server = createServer((req, res) => {
  const chunks = [];
  req.on("data", (c) => chunks.push(c));
  req.on("end", () => {
    const body = Buffer.concat(chunks).toString();
    if (req.url === "/json") {
      res.writeHead(200, { "content-type": "application/json" });
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

const wasmPath =
  process.argv[2] || join(repo, "runtimes/0.3.2-dev.8/aura_playground_runtime.wasm");
const label = process.argv[2] ? wasmPath : "0.3.2-dev.8";
const rt = await AuraRuntime.fromBytes(readFileSync(wasmPath), label);

check("dev.8 advertises sessions", rt.supportsSessions === true);
check(
  "dev.8 advertises project sessions",
  rt.supportsProjectSessions === true,
);

// The capability is additive to ABI 2: an artifact built before it must report
// `false` rather than throw, so a Host can fall back instead of failing.
{
  const older = await AuraRuntime.fromBytes(
    readFileSync(join(repo, "runtimes/0.3.2-dev.7/aura_playground_runtime.wasm")),
    "0.3.2-dev.7",
  );
  check("dev.7 still advertises sessions", older.supportsSessions === true);
  check(
    "dev.7 predates project sessions",
    older.supportsProjectSessions === false,
  );
  check(
    "both artifacts report the same ABI",
    older.abiVersion === rt.abiVersion,
    `${older.abiVersion} vs ${rt.abiVersion}`,
  );
  let threw = "";
  try {
    older.startProjectSession({ entry: "root", sources: [] });
  } catch (err) {
    threw = err.message;
  }
  check(
    "an artifact without the export refuses project sessions loudly",
    /project sessions/.test(threw),
    threw,
  );
}

// Everything below depends on the capability. On a pre-fix artifact this test
// must fail as a *statement about the artifact*, not as an uncaught throw.
if (!rt.supportsProjectSessions) {
  check("the artifact under test supports project sessions", false, label);
  process.exitCode = 1;
} else {

/** Build a two-module project: the entry calls into a child that speaks HTTP. */
function project(entryText, childText, childName = "child.aura") {
  return {
    entry: "root",
    sources: [
      {
        key: "root",
        name: "main.aura",
        text: entryText,
        children: [{ name: "child", key: "child" }],
      },
      { key: "child", name: childName, text: childText },
    ],
  };
}

/** Drive a project session, performing each HTTP effect via Node fetch. */
async function runProjectWithHttp(req, { grant = true } = {}) {
  let step = rt.startProjectSession(req, { args: [], stdin: null });
  let effects = 0;
  while (step.status === 3) {
    const effect = step.result;
    effects += 1;
    if (!grant) {
      step = rt.resumeSession({
        effect_id: effect.effect_id,
        ok: false,
        code: 5002,
        message: "denied",
      });
      continue;
    }
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

// ---- the actual defect B14 describes -------------------------------------
// The HTTP call lives in a *child* module, so the effect is produced by code
// that is neither the entry source nor the top-level scope. Before the fix
// this program could not run at all: `runProject` is synchronous, so `http_get`
// had nothing to suspend.
{
  const { step, effects } = await runProjectWithHttp(
    project(
      `fn main() {
    print("entry")
    print(child::fetch_name())
}`,
      `pub fn fetch_name() -> string {
    let r = http_get("${base}/json")
    let p = json_decode(r["body"])
    return p["name"]
}`,
    ),
  );
  check(
    "a child module can perform HTTP",
    step.result.status === "ok",
    JSON.stringify(step.result).slice(0, 300),
  );
  check(
    "the response body reaches the entry scope",
    step.result.stdout === "entry\npikachu\n",
    JSON.stringify(step.result.stdout),
  );
  check("exactly one effect was produced", effects === 1, String(effects));
}

// Request shape survives the project path unchanged.
{
  const { step } = await runProjectWithHttp(
    project(
      `fn main() {
    print(child::send())
}`,
      `pub fn send() -> string {
    let r = http_request("POST", "${base}/echo", {"body": "payload", "headers": [["X-Aura-Test", "sent"]]})
    return r["body"]
}`,
    ),
  );
  check(
    "method, body and headers all cross the project boundary",
    step.result.stdout === "POST payload sent\n",
    JSON.stringify(step.result.stdout),
  );
}

// 404 is an ordinary response in a project too.
{
  const { step } = await runProjectWithHttp(
    project(
      `fn main() {
    print(child::probe())
}`,
      `pub fn probe() -> int {
    let r = http_get("${base}/missing")
    return r["status"]
}`,
    ),
  );
  check(
    "a 404 is an ordinary response inside a project",
    step.result.stdout === "404\n",
    JSON.stringify(step.result.stdout),
  );
}

// A denied request must fail with E5002 in a project exactly as it does in a
// single source. The project path must not become a way to bypass consent.
{
  const { step, effects } = await runProjectWithHttp(
    project(
      `fn main() {
    print(child::fetch_name())
}`,
      `pub fn fetch_name() -> string {
    let r = http_get("${base}/json")
    return r["body"]
}`,
    ),
    { grant: false },
  );
  check(
    "a denied request fails with E5002 in a project",
    JSON.stringify(step.result).includes("E5002"),
    JSON.stringify(step.result).slice(0, 300),
  );
  check("the denied request still suspended once", effects === 1, String(effects));
}

// Diagnostics raised inside a child must name the child, not the entry. The
// synchronous path already did this; the session path must not regress it.
{
  const { step } = await runProjectWithHttp(
    project(
      `fn main() {
    print(child::broken())
}`,
      `pub fn broken() -> string {
    let x: int = "not an int"
    return x
}`,
      "child.aura",
    ),
  );
  const diags = step.result.diagnostics || [];
  check(
    "a child diagnostic is reported",
    diags.length > 0,
    JSON.stringify(step.result).slice(0, 300),
  );
  check(
    "the diagnostic names the child source",
    diags.length > 0 && diags[0].source === "child.aura",
    JSON.stringify(diags.map((d) => d.source)),
  );
}

// A single-source project must keep the main-less evaluation fallback: it has
// no `main`, so the project path falls back to module semantics rather than
// reporting E4027.
{
  const single = { entry: "root", sources: [{ key: "root", name: "main.aura", text: "1 + 2" }] };
  let step = rt.startProjectSession(single, { args: [], stdin: null });
  check(
    "a main-less single-source project uses the evaluation fallback",
    step.status === 0 && step.result.status === "ok",
    JSON.stringify({ status: step.status, result: step.result }).slice(0, 300),
  );
}

// A project with no `main` among several sources is a real error, and it must
// still be reported as one.
{
  const noMain = {
    entry: "root",
    sources: [
      { key: "root", name: "main.aura", text: "print(1)" },
      { key: "other", name: "other.aura", text: "pub fn f() -> int { return 1 }" },
    ],
  };
  const step = rt.startProjectSession(noMain, { args: [], stdin: null });
  check(
    "a multi-source project without `main` is rejected",
    step.result.status !== "ok",
    JSON.stringify(step.result).slice(0, 300),
  );
}

// The control: the synchronous project path — the only way to run a project
// before this change — cannot perform HTTP at all. Without this the test above
// would only prove that `startProjectSession` returns ok, not that it fixed
// anything.
{
  const req = project(
    `fn main() {
    print(child::fetch_name())
}`,
    `pub fn fetch_name() -> string {
    let r = http_get("${base}/json")
    return r["body"]
}`,
  );
  const sync = rt.runProject(req);
  check(
    "the synchronous project path still denies HTTP (E5002)",
    JSON.stringify(sync).includes("E5002"),
    JSON.stringify(sync).slice(0, 300),
  );
  check(
    "the synchronous project path produces no effect",
    sync.status !== "pending",
    JSON.stringify(sync.status),
  );
}

// B7 item 3 on the multi-module path: repeated headers.
//
// LANGUAGE_SPEC.md §25 states the per-Host divergence normatively: the browser
// `Headers` collection combines repeated occurrences into one `", "`-joined
// value per lowercase name, while the native host preserves duplicates as a
// list in wire order with the original casing. Both shapes are decoded by the
// same `json_to_response_value`, so the project path must reproduce them
// exactly rather than flattening or re-splitting. This suite is named as the
// multi-module contract in the spec's evidence list, so pinning the divergence
// here is what keeps that citation true.
//
// The two cases are driven by giving the *host* its own header pairs; the
// program under test is identical, so a difference in output can only come
// from the decode step.
{
  const headerProgram = project(
    `fn main() {
    print(child::shape())
}`,
    `pub fn shape() -> string {
    let r = http_get("${base}/json")
    return to_string(r["headers"]["x-dup"]) + "|" + to_string(len(r["header_lines"]))
}`,
  );

  // Resume a project session with an explicit, Host-supplied header list.
  const runWithHeaders = (headers) => {
    const first = rt.startProjectSession(headerProgram, { args: [], stdin: null });
    return rt.resumeSession({
      effect_id: first.result.effect_id,
      ok: true,
      response: { status: 200, headers, body: "{}" },
    });
  };

  // Browser: the platform already combined and lowercased the value.
  const browser = runWithHeaders([
    ["x-dup", "one, two"],
    ["x-mixed-case", "kept"],
  ]);
  check(
    "a combined browser header reaches a project as one string",
    browser.result.stdout === "one, two|2\n",
    JSON.stringify(browser.result.stdout),
  );

  // Native: duplicates arrive as separate pairs and must stay a list.
  const native = runWithHeaders([
    ["X-Dup", "one"],
    ["X-Dup", "two"],
    ["X-Mixed-Case", "kept"],
  ]);
  check(
    "duplicate native headers reach a project as a list",
    native.result.stdout === '["one", "two"]|3\n',
    JSON.stringify(native.result.stdout),
  );

  // The guard must be capable of failing: the two hosts disagree, so a decode
  // step that flattened both shapes would produce equal output here.
  check(
    "the two host header shapes do not collapse to one",
    browser.result.stdout !== native.result.stdout,
    JSON.stringify({ browser: browser.result.stdout, native: native.result.stdout }),
  );
}

// The single-source session path must be untouched by the project extension.
{
  const step = rt.startSession('fn main() { print("single") }', { args: [], stdin: null });
  check(
    "single-source sessions still work",
    step.status === 0 && step.result.stdout === "single\n",
    JSON.stringify(step.result).slice(0, 200),
  );
}

}

server.close();
console.log(`\nproject-http: ${passed} passed, ${failed} failed`);
process.exitCode = failed === 0 ? 0 : 1;
