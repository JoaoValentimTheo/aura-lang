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

// ---------------------------------------------------------------------------
// Stdout capture bound on the fresh machine-backed wasm
// ---------------------------------------------------------------------------
//
// The host's `MAX_STDOUT_BYTES` (1 MiB) bound is an application resource
// policy, not language semantics: `print` emits through the host, which
// accepts a write that lands on the bound and refuses whole any write that
// would cross it with `E4020`. This pins the exact boundary on the fresh
// machine-backed artifact: the observed Playground incident (`for i in
// 1..10000000 { print(i) }` → E4020) must be a structured diagnostic with
// bounded capture, never an unbounded buffer, a trap, or a partial write.

const LIMIT = 1024 * 1024;
const byteLen = (s) => Buffer.byteLength(s, "utf8");

function runWithStdin(source, stdin) {
  try {
    return {
      result: runtime.run(source, { args: [], stdin }),
      trap: null,
    };
  } catch (err) {
    return { result: null, trap: err && err.message ? err.message : String(err) };
  }
}

function hasE4020(r) {
  return (
    r.result !== null &&
    r.result.status !== "ok" &&
    (r.result.diagnostics || []).some((d) => d.code === 4020)
  );
}

{
  // `print(s)` writes `s`’s bytes plus a newline: stdin of `LIMIT - 2` lands
  // one byte below the bound, `LIMIT - 1` lands exactly on it.
  const src = `fn main() {\n  let s = read_line()\n  print(s)\n}\n`;

  const below = runWithStdin(src, "a".repeat(LIMIT - 2));
  check(
    "stdout below the bound is captured whole",
    !below.trap && below.result.status === "ok" &&
      byteLen(below.result.stdout) === LIMIT - 1,
    below.trap ?? JSON.stringify(below.result),
  );

  const at = runWithStdin(src, "a".repeat(LIMIT - 1));
  check(
    "stdout landing exactly on the bound is accepted",
    !at.trap && at.result.status === "ok" && byteLen(at.result.stdout) === LIMIT,
    at.trap ?? JSON.stringify(at.result),
  );

  const over = runWithStdin(src, "a".repeat(LIMIT));
  check(
    "one oversized write is refused whole with E4020",
    !over.trap && hasE4020(over) && byteLen(over.result.stdout) === 0,
    over.trap ?? JSON.stringify(over.result),
  );

  // Bytes accepted before the crossing write survive; the refused write
  // contributes nothing.
  const partial = runWithStdin(
    `fn main() {\n  print("before")\n  let s = read_line()\n  print(s)\n  print("after")\n}\n`,
    "a".repeat(LIMIT),
  );
  check(
    "prior output is retained and the refused write adds nothing",
    !partial.trap && hasE4020(partial) &&
      partial.result.stdout.startsWith("before\n") &&
      !partial.result.stdout.includes("after"),
    partial.trap ?? JSON.stringify({
      status: partial.result.status,
      bytes: byteLen(partial.result.stdout),
    }),
  );

  // UTF-8: a 4-byte write landing exactly on the bound is valid and intact;
  // one byte over is refused whole, leaving the valid prefix.
  const utfAt = runWithStdin(
    `fn main() {\n  let s = read_line()\n  print(s)\n  print("€")\n}\n`,
    "a".repeat(LIMIT - 5),
  );
  check(
    "UTF-8 write landing exactly on the bound is intact",
    !utfAt.trap && utfAt.result.status === "ok" &&
      byteLen(utfAt.result.stdout) === LIMIT &&
      utfAt.result.stdout.endsWith("€\n"),
    utfAt.trap ?? JSON.stringify({ status: utfAt.result.status, bytes: byteLen(utfAt.result.stdout) }),
  );

  const utfOver = runWithStdin(
    `fn main() {\n  let s = read_line()\n  print(s)\n  print("€")\n}\n`,
    "a".repeat(LIMIT - 4),
  );
  check(
    "UTF-8 write one byte over is refused whole",
    !utfOver.trap && hasE4020(utfOver) &&
      byteLen(utfOver.result.stdout) === LIMIT - 3,
    utfOver.trap ?? JSON.stringify({ status: utfOver.result.status, bytes: byteLen(utfOver.result.stdout) }),
  );

  // E4020 is fatal: catch cannot intercept it; finally still runs.
  const fatal = runWithStdin(
    `fn main() {\n  let s = read_line()\n  try {\n    print(s)\n  } catch e {\n    print("caught")\n  } finally {\n    print("finally-visible")\n  }\n}\n`,
    "a".repeat(LIMIT),
  );
  check(
    "E4020 is not catchable and finally still runs",
    !fatal.trap && hasE4020(fatal) &&
      !fatal.result.stdout.includes("caught") &&
      fatal.result.stdout.endsWith("finally-visible\n"),
    fatal.trap ?? JSON.stringify(fatal.result.stdout.slice(-32)),
  );

  // The budget is per execution: a second run may again land on the bound.
  const again = runWithStdin(src, "a".repeat(LIMIT - 1));
  check(
    "the bound is per execution (rerun lands on it again)",
    !again.trap && again.result.status === "ok" && byteLen(again.result.stdout) === LIMIT,
    again.trap ?? JSON.stringify({ status: again.result.status, bytes: byteLen(again.result.stdout) }),
  );

  // The reported incident shape: a runaway print loop terminates with a
  // structured E4020 and bounded capture, never a trap.
  const runaway = run(
    `fn main() {\n  for i in 0..600000 {\n    print(i)\n  }\n}\n`,
  );
  check(
    "runaway print loop: structured E4020, bounded capture",
    !runaway.trap && hasE4020(runaway) &&
      byteLen(runaway.result.stdout) <= LIMIT,
    runaway.trap ?? JSON.stringify(runaway.result.diagnostics),
  );

  // Long computation with tiny output is unaffected by the capture bound.
  const compute = run(
    `fn main() {\n  let mut x = 0\n  for i in 0..2000000 {\n    x = x + 1\n  }\n  print(x)\n}\n`,
  );
  check(
    "long computation with tiny output completes",
    !compute.trap && okStdout(compute, "2000000\n"),
    compute.trap ?? JSON.stringify(compute.result),
  );
}

console.log(`\nB-1 substrate boundary: ${passed} passed, ${failed} failed`);
process.exit(failed === 0 ? 0 : 1);
