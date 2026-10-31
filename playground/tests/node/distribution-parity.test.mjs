// Distribution parity: the runtime bytes we actually ship must contain the
// behaviour of the sources we ship them from.
//
// Every other suite in this directory answers "does the engine built from this
// tree behave correctly?" -- `differential` and `syntax` exercise the freshly
// compiled wasm in `target/`, `integrity` checks that the committed artifact
// matches its manifest identity. None of them answers the question that
// actually matters to a user of the Playground: does the artifact versioned
// under `playground/runtimes/<id>/` behave like this tree?
//
// That gap is not hypothetical. `0.3.1` and `0.3.2-dev.8` were both published
// without the sort-ordering and f-string zero-padding corrections that were
// already present in `src/`, and every suite stayed green because all of them
// measured the fresh build. This suite closes the gap by running the audited
// regression corpus against the distributable artifact itself, and by
// requiring agreement with the native build of the same tree.
//
// The cases below transcribe tests that exist elsewhere in the repository
// (`tests/regressions.rs`, `tests/collections.rs`); they are repeated here as
// literal source because this suite must be able to run against a binary
// artifact with no compiler available.
//
// Usage:
//   node playground/tests/node/distribution-parity.test.mjs <artifact.wasm> <native-bin> [version-id]

import { readFileSync, writeFileSync, mkdtempSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { AuraRuntime } from "../../web/runtime.mjs";

const here = dirname(fileURLToPath(import.meta.url));

const wasmPath = process.argv[2];
const nativeBin = process.argv[3];
const versionId = process.argv[4] ?? wasmPath;
if (!wasmPath || !nativeBin) {
  console.error(
    "usage: distribution-parity.test.mjs <artifact.wasm> <native-bin> [version-id]",
  );
  process.exit(2);
}

// --- the two corpora ---------------------------------------------------------

// Audit-7 (total-order sort) and audit-8 (f-string zero padding) regressions.
// Each entry is [id, source, expected_stdout | null, expected_diagnostic | null].
// `__MONOTONIC_21__` asserts that the 21 printed values are non-decreasing
// without pinning the platform's formatting of the rounded inputs.
const CORRECTIONS = [
  [
    "A1 sort orders nan last",
    'fn main() { let nn = to_float("nan")\n print(sort([3.0, 1.0, nn, 2.0]))\n print(sort([1.0, nn, 3.0, 2.0]))\n print(sort([nn, 3.0, 1.0, 2.0])) }',
    "[1.0, 2.0, 3.0, nan]\n[1.0, 2.0, 3.0, nan]\n[1.0, 2.0, 3.0, nan]",
    null,
  ],
  [
    "A2 sort does not abort on a nan list",
    'fn main() { let nn = to_float("nan")\n let xs = [92.0, 3.0, 40.0, nn, 14.0, 51.0, nn, 25.0, 62.0, 36.0, 73.0, nn, 47.0, 84.0, nn, 58.0, 95.0, nn, 69.0, 6.0, nn]\n print(sort(xs)) }',
    "[3.0, 6.0, 14.0, 25.0, 36.0, 40.0, 47.0, 51.0, 58.0, 62.0, 69.0, 73.0, 84.0, 92.0, 95.0, nan, nan, nan, nan, nan, nan]",
    null,
  ],
  [
    "A3 sort rejects a union typed list",
    'fn main() { let xs: [int | string] = [92, "a", 3, 40, "b", 14, 51, "c", 25, 62, 36, 73, "d", 47, 84, "e", 58, 95, "f", 69, 6, "g"]\n print(sort(xs)) }',
    null,
    "E3001",
  ],
  [
    "A4 sort rejects mixed ordering classes (int/string)",
    'fn main() { print(sort([5, 1, "a", 3, 2])) }',
    null,
    "E3001",
  ],
  [
    "A4b sort rejects mixed ordering classes (int/bool)",
    "fn main() { print(sort([1, true, 2])) }",
    null,
    "E3001",
  ],
  [
    "A4c sort rejects mixed ordering classes (nested lists)",
    "fn main() { let xs = [[1], [2]]\n print(sort(xs)) }",
    null,
    "E3001",
  ],
  [
    "A5 sort compares mixed numerics exactly",
    "fn main() { print(sort([9007199254740993, 9007199254740992.0, 9007199254740992])) }",
    "[9007199254740992.0, 9007199254740992, 9007199254740993]",
    null,
  ],
  [
    "A6 sort does not abort on rounded mixed numerics",
    "fn main() { let xs = [9007199254740992.0, 9007199254740992.0, 9007199254740993, 9007199254740992, 9007199254740996.0, 9007199254740996.0, 9007199254740994.0, 9007199254740993, 9007199254740989, 9007199254740992, 9007199254740994.0, 9007199254740994.0, 9007199254740992.0, 9007199254740992.0, 9007199254740993, 9007199254740991, 9007199254740995, 9007199254740992.0, 9007199254740992.0, 9007199254740994.0, 9007199254740992]\n print(sort(xs)) }",
    "__MONOTONIC_21__",
    null,
  ],
  [
    "A7 sort preserves the existing orderings",
    'fn main() { print(sort([1.0, 2.0, to_float("inf"), 3.0]))\n print(sort([to_float("-inf"), 1, 0.0]))\n print(sort([0.0, to_float("-0.0")]))\n print(sort([to_float("-0.0"), 0.0]))\n print(sort([3, 1, 2]))\n print(sort([3.0, 1.0, 2.0])) }',
    "[1.0, 2.0, 3.0, inf]\n[-inf, 0.0, 1]\n[0.0, -0.0]\n[-0.0, 0.0]\n[1, 2, 3]\n[1.0, 2.0, 3.0]",
    null,
  ],
  [
    "A8 sort method matches the free function",
    'fn main() { let mut v = [3, 1, 2]\n print(v.sort())\n let nn = to_float("nan")\n let mut f = [2.0, nn, 1.0]\n print(f.sort()) }',
    "[1, 2, 3]\n[1.0, 2.0, nan]",
    null,
  ],
  [
    "A8b sort method matches the free function (rejection)",
    'fn main() { let mut v = [2, "a", 1]\n print(v.sort()) }',
    null,
    "E3001",
  ],
  [
    "B1 zero pad places the sign before the zeros",
    'fn main() { print(f"{-42:06d}")\n print(f"{-42:06}")\n print(f"{-42:+06d}")\n print(f"{42: 06d}") }',
    "-00042\n-00042\n-00042\n 00042",
    null,
  ],
  [
    "B2 zero pad is sign aware for floats",
    'fn main() { print(f"{-3.5:08.2f}")\n print(f"{-0.5:010.3f}")\n print(f"{-0.25:09.2%}")\n print(f"{-3.5:08.2e}")\n print(f"{-3.5:010.2e}") }',
    "-0003.50\n-00000.500\n-0025.00%\n-03.50e0\n-0003.50e0",
    null,
  ],
  [
    "B3 zero pad handles infinities",
    'fn main() { print(f"{1e400:08.2f}")\n print(f"{-1e400:08.2f}") }',
    "00000inf\n-0000inf",
    null,
  ],
  [
    "B4 explicit fill suppresses the zero flag",
    'fn main() { print(f"{-42:*>06d}")\n print(f"{-42:*<06d}") }',
    "***-42\n-42***",
    null,
  ],
  [
    "B5 bare alignment keeps the plain zero fill",
    'fn main() { print(f"{-42:0>6d}")\n print(f"{-42:<06d}")\n print(f"{-42:>06d}")\n print(f"{-42:^06d}")\n print(f"{-42:_>6d}") }',
    "000-42\n-42000\n000-42\n0-4200\n___-42",
    null,
  ],
  [
    "B6 zero pad leaves the plain width path alone",
    'fn main() { print(f"{42:06d}")\n print(f"{-42:6d}")\n print(f"{-123456:04d}") }',
    "000042\n   -42\n-123456",
    null,
  ],
  [
    "B7 zero pad still fills non numeric values",
    'fn main() { print(f"{true:06}")\n print(f"{[1, 2]:08}") }',
    "00true\n00[1, 2]",
    null,
  ],
];

// Corrections observable as a type diagnostic rather than as output. A
// published runtime that omits them does not merely print the wrong text: it
// executes programs the checker rejects, so a user is told their program is
// valid when the compiler says otherwise.
const SOUNDNESS = [
  [
    "S1 set.add rejects a foreign element type",
    'fn main() { let mut s: {int} = set{1}\n s.add("x") }',
    null,
    "E3001",
  ],
  [
    "S2 set.has rejects a foreign element type",
    'fn main() { let mut s: {int} = set{1}\n print(s.has("x")) }',
    null,
    "E3001",
  ],
  [
    "S3 set.remove rejects a foreign element type",
    'fn main() { let mut s: {int} = set{1}\n print(s.remove("x")) }',
    null,
    "E3001",
  ],
  [
    "S4 set.contains rejects a foreign element type",
    'fn main() { let mut s: {int} = set{1}\n print(s.contains("x")) }',
    null,
    "E3001",
  ],
  [
    "S5 set.add rejects a foreign element type on an inferred set",
    'fn main() { let mut s = set{1}\n s.add("x") }',
    null,
    "E3001",
  ],
  [
    "S6 set union of one element type is accepted",
    "fn main() { let mut s = set{1}\n s.add(2)\n print(s) }",
    "{1, 2}",
    null,
  ],
  [
    "R1 implicit tail value must match the return type",
    'fn f() -> int { "x" }\nfn main() { print(f()) }',
    null,
    "E3005",
  ],
  [
    "R2 implicit tail value of the right type is accepted",
    "fn f() -> int { 7 }\nfn main() { print(f()) }",
    "7",
    null,
  ],
  [
    "J1 malformed json is reported as E4031",
    'fn main() { let _v = json_decode("{not json") }',
    null,
    "E4031",
  ],
  [
    "J2 trailing garbage is reported as E4031",
    'fn main() { let _v = json_decode("1 2") }',
    null,
    "E4031",
  ],
  [
    "J3 json type error stays E3001",
    "fn main() { let _v = json_decode(42) }",
    null,
    "E3001",
  ],
  [
    "J4 deep json round trip",
    "fn main() { print(json_decode(json_encode([[1]]))) }",
    "[[1]]",
    null,
  ],
  [
    "G1 range(5, 5) is statically empty",
    "fn main() { let mut n = 0\n for _i in range(5, 5) { n = n + 1 }\n print(n) }",
    "0",
    null,
  ],
  [
    "G2 5..5 is statically empty",
    "fn main() { let mut n = 0\n for _i in 5..5 { n = n + 1 }\n print(n) }",
    "0",
    null,
  ],
  [
    "G3 range(9, 2) is statically empty",
    "fn main() { let mut n = 0\n for _i in range(9, 2) { n = n + 1 }\n print(n) }",
    "0",
    null,
  ],
  [
    "G4 range(-1, -1) is statically empty",
    "fn main() { let mut n = 0\n for _i in range(-1, -1) { n = n + 1 }\n print(n) }",
    "0",
    null,
  ],
  [
    "G5 a non empty range still iterates",
    "fn main() { let mut n = 0\n for _i in range(0, 3) { n = n + 1 }\n print(n) }",
    "3",
    null,
  ],
  [
    "T1 method type parameter resolved through bounds",
    "struct B { n: int }\nimpl B { fn id<T>(self, x: T) -> T { return x } }\nfn main() { print(B { n: 1 }.id(5)) }",
    "5",
    null,
  ],
  [
    "F1 f string exponent formatting is stable",
    'fn main() { print(f"{1.5e10:.2e}")\n print(f"{1.5e-10:.2e}") }',
    "1.50e10\n1.50e-10",
    null,
  ],
  [
    "F2 f string formats non finite floats without a trap",
    "fn main() { print(f\"{to_float('inf')}\")\n print(f\"{to_float('nan')}\") }",
    "inf\nnan",
    null,
  ],
];

// --- harness -----------------------------------------------------------------

const runtime = await AuraRuntime.fromBytes(readFileSync(wasmPath), "distribution-parity");
const dir = mkdtempSync(join(tmpdir(), "aura-distparity-"));
const optionsFile = join(dir, "options.bin");
writeFileSync(optionsFile, Buffer.from(new TextEncoder().encode("stdin-bytes 0\n")));

/** Run one source on the distributed artifact, turning a guest trap into data. */
function onArtifact(src) {
  try {
    return { trap: null, result: runtime.run(src) };
  } catch (err) {
    return { trap: err && err.message ? err.message : String(err), result: null };
  }
}

/** Run the same source through the native harness of the same tree. */
function onNative(name, src) {
  const file = join(dir, `${name.replace(/[^a-z0-9]+/gi, "_")}.aura`);
  writeFileSync(file, src);
  const json = execFileSync(nativeBin, [file, optionsFile], { encoding: "utf8" }).trim();
  return JSON.parse(json);
}

const trimEnd = (s) => (s ?? "").replace(/\n+$/, "");
const firstCode = (r) => r?.diagnostics?.[0]?.code_text ?? null;

/**
 * Compare one observation against the expectation.
 * Returns null when it holds, or a short reason when it does not.
 */
function mismatch(observation, expectedStdout, expectedCode) {
  if (observation.trap) return `trap: ${observation.trap}`;
  const code = firstCode(observation.result);
  if (expectedCode) {
    return code === expectedCode ? null : `expected ${expectedCode}, got ${code ?? "no diagnostic"}`;
  }
  if (code) return `unexpected diagnostic ${code}`;
  const out = trimEnd(observation.result.stdout);
  if (expectedStdout === "__MONOTONIC_21__") {
    const nums = out.replace(/^\[/, "").replace(/\]$/, "").split(", ").map(Number);
    if (nums.length !== 21) return `expected 21 elements, got ${nums.length}`;
    for (let i = 1; i < nums.length; i += 1) {
      if (!(nums[i - 1] <= nums[i])) return `output is not monotonic at element ${i}`;
    }
    return null;
  }
  return out === expectedStdout ? null : `stdout ${JSON.stringify(out)}`;
}

/**
 * Compare the artifact against the native build of the same sources.
 * Diagnostics are compared by code, never by human message wording.
 */
function divergence(observation, native) {
  if (observation.trap) return "traps where the native build does not";
  const wasmCode = firstCode(observation.result);
  const nativeCode = firstCode(native);
  if (wasmCode !== nativeCode) {
    return `diagnostic ${wasmCode ?? "none"} vs native ${nativeCode ?? "none"}`;
  }
  const wasmOut = trimEnd(observation.result.stdout);
  const nativeOut = trimEnd(native.stdout);
  return wasmOut === nativeOut
    ? null
    : `stdout ${JSON.stringify(wasmOut)} vs native ${JSON.stringify(nativeOut)}`;
}

// --- run ---------------------------------------------------------------------

let passed = 0;
let failed = 0;

function fail(id, reason) {
  failed += 1;
  console.error(`FAIL [${versionId}] ${id}\n  ${reason}\n  artifact: ${wasmPath}`);
}

for (const [id, src, expectedStdout, expectedCode] of [...CORRECTIONS, ...SOUNDNESS]) {
  const observation = onArtifact(src);
  const bad = mismatch(observation, expectedStdout, expectedCode);
  if (bad) {
    fail(id, bad);
    continue;
  }
  const diff = divergence(observation, onNative(id, src));
  if (diff) {
    fail(id, diff);
    continue;
  }
  passed += 1;
}

// A suite that silently measured the wrong file would prove nothing, so the
// artifact must also agree with the identity the caller claims it has.
if (runtime.runtimeVersion !== versionId) {
  fail("identity", `artifact reports ${runtime.runtimeVersion}, expected ${versionId}`);
} else {
  passed += 1;
}

console.log(`Distribution parity [${versionId}]: ${passed}/${passed + failed} passed`);
process.exitCode = failed === 0 ? 0 : 1;
