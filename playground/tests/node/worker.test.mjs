// Worker lifecycle and isolation tests.
//
// Exercises the properties the prompt calls out explicitly:
//   * normal completion, Aura errors, deep recursion, parser limits;
//   * runaway program terminated by Worker termination;
//   * repeated Run/Stop/Run with no stale-result corruption;
//   * generation ids: a terminated run's late messages are ignored.
//
// These drive the page's own orchestration (`app.js`) through the browser, so
// the Worker life cycle under test is exactly the production one.
//
// Usage: node playground/tests/node/worker.test.mjs

import { startServer } from "./serve.mjs";

let chromium;
try {
  ({ chromium } = await import("playwright"));
} catch {
  console.error("worker.test.mjs: `playwright` is not installed; skipping.");
  process.exit(0);
}

const { server, port } = await startServer(0);
const base = `http://127.0.0.1:${port}/`;
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

const page = await browser.newPage();
const errors = [];
page.on("pageerror", (e) => errors.push(e.message));
await page.goto(base);
await page.waitForFunction(() => document.querySelectorAll("#version option").length > 0);

async function run(src, timeout = 15000) {
  await page.fill("#source", src);
  await page.click("#run");
  await page.waitForFunction(
    () => {
      const s = document.getElementById("status").textContent;
      return s !== "running…" && !s.startsWith("running (");
    },
    { timeout },
  );
  return page.evaluate(() => ({
    stdout: document.getElementById("stdout").textContent,
    status: document.getElementById("status").textContent,
    diagnostics: [...document.querySelectorAll("#diagnostics li:not(.empty)")].map(
      (li) => li.textContent,
    ),
  }));
}

// 1. normal completion
let r = await run('fn main() { print("ok") }');
check("normal completion", r.stdout === "ok\n" && r.status === "ok", JSON.stringify(r));

// 2. Aura error
r = await run("fn main() { print(1 / 0) }");
check("division by zero E4007", r.diagnostics.some((d) => d.includes("E4007")), JSON.stringify(r));

// 3. deep recursion -> E4011, not a trap
r = await run("fn f() { f() }\nfn main() { f() }");
check("deep recursion E4011", r.diagnostics.some((d) => d.includes("E4011")), JSON.stringify(r));

// 4. parser/backstop limit -> E1015, not a trap
r = await run(`fn main() { print(${"[".repeat(400)}1${"]".repeat(400)}) }`);
check("parser limit E1015", r.diagnostics.some((d) => d.includes("E1015")), JSON.stringify(r));

// 5. runaway program + stop, repeatedly
for (let i = 0; i < 5; i += 1) {
  await page.fill("#source", "fn main() { while true {} }");
  await page.click("#run");
  await page.waitForFunction(() => document.getElementById("status").textContent.startsWith("running"), {
    timeout: 5000,
  });
  await page.click("#stop");
  await page.waitForFunction(() => document.getElementById("status").textContent === "stopped");
  r = await run(`fn main() { print("cycle ${i}") }`);
  check(`run after stop ${i}`, r.stdout === `cycle ${i}\n`, JSON.stringify(r));
}

// 6. rapid restart: start a slow program then immediately restart; the new run
//    must win and the old result must never appear.
await page.fill("#source", "fn main() {\n let mut s = 0\n for i in range(0, 2000000) { s = s + i }\n print(s)\n}");
await page.click("#run");
await page.fill("#source", 'fn main() { print("second") }');
await page.click("#run");
await page.waitForFunction(() => document.getElementById("status").textContent === "ok", {
  timeout: 10000,
});
r = await page.evaluate(() => ({ stdout: document.getElementById("stdout").textContent }));
check("restart result is the new run", r.stdout === "second\n", JSON.stringify(r));

// 7. many sequential runs do not accumulate failures
let ok = true;
for (let i = 0; i < 20; i += 1) {
  const rr = await run("fn main() { print(1 + 1) }");
  if (rr.stdout !== "2\n" || rr.status !== "ok") {
    ok = false;
    break;
  }
}
check("20 sequential runs stable", ok);

check("no page errors", errors.length === 0, errors.join("; "));

await browser.close();
server.close();
console.log(`\nWorker: ${passed} passed, ${failed} failed`);
process.exit(failed === 0 ? 0 : 1);
