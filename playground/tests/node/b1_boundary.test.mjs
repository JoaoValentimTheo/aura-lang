// B-1R5 substrate boundary: the freshly built WebAssembly runtime is the
// machine-backed candidate substrate, compiled from this repository's source.
// There is no OS thread and no stack-size knob on wasm, so this is the
// substrate the B-1 defect class actually manifests on — and the one where the
// explicit-continuation machine must be authoritative.
//
// Contract: every mainstream recursion shape runs to a depth of 510 frames
// (main included, the language limit is 512), rejects one past it with a
// structured E4011, and NEVER traps on the engine stack below the language
// limit. The same holds for module-mode evaluation (no `main`, no frame), where
// 512 simultaneous user calls are legal. A rejected E4011 leaves the instance
// usable for later runs.
//
// This complements the differential harness: that compares the frozen release
// artifact (recursive engine) with the fresh native build (machine) at depths
// where both agree; this test pins the boundary on the fresh wasm itself.
//
// Usage: node playground/tests/node/b1_boundary.test.mjs <fresh-runtime.wasm>

import { readFileSync } from "node:fs";
import { AuraRuntime } from "../../web/runtime.mjs";

const wasmPath = process.argv[2];
if (!wasmPath) {
  console.error("usage: b1_boundary.test.mjs <fresh-runtime.wasm>");
  process.exit(2);
}

const runtime = await AuraRuntime.fromBytes(readFileSync(wasmPath), "b1-boundary");

let passed = 0;
let failed = 0;

function check(name, cond, detail) {
  if (cond) {
    passed += 1;
  } else {
    failed += 1;
    console.error(`FAIL ${name}${detail ? `: ${detail}` : ""}`);
  }
}

/** Run on wasm, reporting a guest trap as a hard, distinguishable failure. */
function run(source) {
  try {
    return { result: runtime.run(source, { args: [], stdin: "" }), trap: null };
  } catch (err) {
    return { result: null, trap: err && err.message ? err.message : String(err) };
  }
}

function e4011(r) {
  return (
    r.result !== null &&
    r.result.status !== "ok" &&
    (r.result.diagnostics || []).some((d) => d.code === 4011)
  );
}

function okStdout(r, want) {
  return r.result !== null && r.result.status === "ok" && r.result.stdout === want;
}

function okResult(r, want) {
  return r.result !== null && r.result.status === "ok" && r.result.result === want;
}

// The shape matrix from the differential recursion battery: the common
// recursive forms, each of which traps below the language limit on the frozen
// release artifact. `expected` is the observably correct stdout for an even
// depth; every depth below is even.
const shapes = {
  "else-block": (n) => `fn count(n: int) -> int {\n  if n <= 0 {\n    return 0\n  } else {\n    return count(n - 1) + 1\n  }\n}\nfn main() { print(count(${n})) }\n`,
  "match-arm": (n) => `fn count(n: int) -> int {\n  match n {\n    0 -> 0,\n    _ -> count(n - 1) + 1\n  }\n}\nfn main() { print(count(${n})) }\n`,
  closure: (n) => `fn main() {\n  let go = (self_rec, n: int) -> { if n <= 0 { return 0 } else { return self_rec(self_rec, n - 1) + 1 } }\n  print(go(go, ${n}))\n}\n`,
  "in-source module": (n) => `module inner {\n  pub fn count(n: int) -> int {\n    if n <= 0 {\n      return 0\n    } else {\n      return count(n - 1) + 1\n    }\n  }\n}\nfn main() { print(inner::count(${n})) }\n`,
  method: (n) => `struct C { }\nimpl C {\n  fn count(self, n: int) -> int {\n    if n <= 0 { return 0 }\n    return self.count(n - 1) + 1\n  }\n}\nfn main() { let c = C { }\n print(c.count(${n})) }\n`,
  mutual: (n) => `fn even(n) { if n == 0 { return 1 }\n return odd(n - 1) }\nfn odd(n) { if n == 0 { return 0 }\n return even(n - 1) }\nfn main() { print(even(${n})) }\n`,
  "for-body": (n) => `fn count(n: int) -> int {\n  if n <= 0 { return 0 }\n  let mut r = 0\n  for i in [0] { r = count(n - 1) + 1 }\n  return r\n}\nfn main() { print(count(${n})) }\n`,
  "if-else chain": (n) => `fn f(n: int) -> int {\n  if n <= 0 { return 0 }\n  if n % 2 == 0 {\n    return f(n - 1) + 1\n  } else {\n    return f(n - 1) + 1\n  }\n}\nfn main() { print(f(${n})) }\n`,
};

// Shape bodies stop one frame short of the limit (`main` is frame 1), and at
// the boundary must be a structured diagnostic — never a trap.
const WORKING_DEPTHS = [250, 400, 510];
for (const [name, mk] of Object.entries(shapes)) {
  for (const depth of WORKING_DEPTHS) {
    const r = run(mk(depth));
    if (r.trap) {
      check(`${name} @${depth}`, false, `guest trap: ${r.trap}`);
    } else {
      // `mutual` alternates parity; every working depth here is even, so the
      // entry `even(n)` reaches `0` and returns 1.
      const want = `${name === "mutual" ? 1 : depth}\n`;
      check(`${name} @${depth}`, okStdout(r, want), JSON.stringify(r.result));
    }
  }
  const over = run(mk(511));
  if (over.trap) {
    check(`${name} @511 E4011`, false, `guest trap: ${over.trap}`);
  } else {
    check(`${name} @511 E4011`, e4011(over), JSON.stringify(over.result));
  }
}

// Module mode has no `main` frame, so 512 simultaneous user calls are legal
// (`helper(511)` recurses 512 times) and frame 513 (`helper(512)`) is the first
// rejection. These exercise the module-capture path on the substrate
// (top-level expression and `const` initializer).
const helper = "fn helper(n) { if n <= 0 { return 0 }\n return 1 + helper(n - 1) }\n";
for (const depth of [510, 511]) {
  const expr = run(`${helper}helper(${depth})\n`);
  check(
    `module expr helper(${depth})`,
    okResult(expr, String(depth)),
    expr.trap ? `guest trap: ${expr.trap}` : JSON.stringify(expr.result),
  );
  const konst = run(`${helper}const X = helper(${depth})\nX\n`);
  check(
    `module const helper(${depth})`,
    okResult(konst, String(depth)),
    konst.trap ? `guest trap: ${konst.trap}` : JSON.stringify(konst.result),
  );
}
for (const depth of [512, 513]) {
  const expr = run(`${helper}helper(${depth})\n`);
  check(
    `module expr helper(${depth}) E4011`,
    !expr.trap && e4011(expr),
    expr.trap ? `guest trap: ${expr.trap}` : JSON.stringify(expr.result),
  );
  const konst = run(`${helper}const X = helper(${depth})\nX\n`);
  check(
    `module const helper(${depth}) E4011`,
    !konst.trap && e4011(konst),
    konst.trap ? `guest trap: ${konst.trap}` : JSON.stringify(konst.result),
  );
}

// A rejected E4011 must leave the instance usable: run a known-good program
// after the rejections above and require a normal result on the same instance.
{
  const recovery = run("fn main() { print(1 + 2) }");
  check(
    "recovery after E4011",
    okStdout(recovery, "3\n"),
    recovery.trap ? `guest trap: ${recovery.trap}` : JSON.stringify(recovery.result),
  );
}

// Pattern-helper residual calibration (B-1R6 review Finding F).
//
// `bind_pattern`/`match_pattern` recurse on the host stack proportional to
// pattern nesting; the wasm parser's substrate-calibrated recursion budget
// (`aura::parse::parse_recursion_budget`, 768 on wasm) is what keeps accepted
// input below the trap. This probe pins the invariant mechanically: a deeply
// nested pattern at the top of the accepted range must bind WITHOUT trapping,
// and one past the range must be the structured E1015 — never a guest trap.
//
// This is the safety contract for the documented residual. If pattern parser
// frames grow without re-calibrating the budget, the deep-accept probe will
// trap and this test fails loudly (which is exactly the required behavior).
const patternDepth = (n) => "[".repeat(n) + "x" + "]".repeat(n);
{
  // 700 is comfortably inside the 765-deep empirically accepted range on the
  // 4 MiB wasm stack but far above the 256 AST-depth limit, so it exercises
  // exactly the parser-budget-bounded pattern path.
  const accepted = run(
    `fn grow(v) { return [v] }\nfn main() { let mut v = 1\n let mut i = 0\n while i < 700 { v = grow(v)\n i = i + 1 }\n let ${patternDepth(700)} = v\n print("bound") }\n`,
  );
  check(
    "deep pattern @700 binds without trap",
    okStdout(accepted, "bound\n"),
    accepted.trap
      ? `guest trap: ${accepted.trap}`
      : JSON.stringify(accepted.result),
  );

  const rejected = run(
    `fn grow(v) { return [v] }\nfn main() { let mut v = 1\n let mut i = 0\n while i < 766 { v = grow(v)\n i = i + 1 }\n let ${patternDepth(766)} = v\n print("bound") }\n`,
  );
  check(
    "over-depth pattern @766 is E1015, not a trap",
    !rejected.trap && rejected.result !== null &&
      rejected.result.status !== "ok" &&
      (rejected.result.diagnostics || []).some((d) => d.code === 1015),
    rejected.trap
      ? `guest trap: ${rejected.trap}`
      : JSON.stringify(rejected.result),
  );
}

console.log(`\nB-1 substrate boundary: ${passed} passed, ${failed} failed`);
process.exit(failed === 0 ? 0 : 1);
