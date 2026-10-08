// Phase-1 explicit syntax oracle plus full native/WASM diagnostic parity.
import assert from "node:assert/strict";
import { readFileSync, writeFileSync, mkdtempSync, rmSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { execFileSync } from "node:child_process";
import { AuraRuntime } from "../../web/runtime.mjs";
const [wasmPath, nativeBin] = process.argv.slice(2);
const bytes = readFileSync(wasmPath);
const runtime = await AuraRuntime.fromBytes(bytes, "syntax");
assert.deepEqual(WebAssembly.Module.imports(await WebAssembly.compile(bytes)), []);
const { instance } = await WebAssembly.instantiate(bytes, {});
const abi = instance.exports;
function runBytes(source) {
  abi.aura_source_reset();
  abi.aura_options_reset();
  for (const b of source) abi.aura_source_push(b, 1);
  abi.aura_run();
  return JSON.parse(new TextDecoder().decode(Uint8Array.from({length: abi.aura_output_len()}, (_, i) => abi.aura_output_byte(i))));
}
const cases = [
  ["dangling pipeline", "1 |>", 1006],
  ["pipeline newline", "1 |>\nnext", 1006],
  ["opening quote", '"', 1004],
  ["escaped closing quote", '"x\\"', 1004],
  ["escape location", "  '\\q'", 1003],
  ["f-string trivia location", 'fn main() { print(  f"{nope}") }', 2003],
  ["UTF8 string", 'fn main() { print("café λ 🐍") }', "café λ 🐍\n"],
  ["BOM", '\ufeff1', 1001],
  ["NBSP interpolation", 'f"{\u00a0x}"', 1001],
  ["trailing commas", 'enum E { A(int,), }\nfn main() { let A(x,) = A(3,)\n print([x,\n]) }', "[3]\n"],
  ["quote in format fill", `fn main() { print(f"{1:'>3}") }`, "''1\n"],
  ["quoted braces", `fn main() { print(f"{'{'} {'}'}") }`, "{ }\n"],
  ["path interpolation", 'module m { pub const N = 4 }\nfn main() { print(f"{m::N}") }', "4\n"],
  ["spaced comparison", 'fn main() { let f=1\nlet int=2\nlet x=false\nprint(f <int>(x)) }', "true\n"],
  ["return bitnot", 'fn f() { return ~0 }\nfn main() { print(f()) }', "-1\n"],
  ["annotated block", 'fn main() { print({let x: int = 1\nx}) }', "1\n"],
  ["newline separators", 'fn main() {\n print(1)\n print(2)\n}', "1\n2\n"],
  ["semicolon is reserved", 'fn main() { print(1); print(2) }', 1006],
  ["list-rest absent", 'fn main() { let [a, ..b] = [1,2] }', 1006],
  ["list literal", 'fn main() { print([1, 2]) }', "[1, 2]\n"],
  ["array contextual", 'fn main() { let a: [int; 2] = [1, 2]\nprint(a) }', "[1, 2]\n"],
  ["array length mismatch", 'fn main() { let a: [int; 2] = [1, 2, 3] }', 3001],
  ["array element mismatch", 'fn main() { let a: [int; 2] = [1, "x"] }', 3001],
  ["array bad length type", 'fn main() { let a: [int; x] = [1, 2] }', 1006],
  ["tuple literal", 'fn main() { print((1, "x")) }', '(1, "x")\n'],
  ["one-element tuple", 'fn main() { print((1,)) }', "(1,)\n"],
  ["tuple not list", 'fn main() { print((1, 2) == [1, 2]) }', "false\n"],
  ["set literal", 'fn main() { print({1, 2, 2, 3}) }', "{1, 2, 3}\n"],
  ["empty set", 'fn main() { print(set{}) }', "{}\n"],
  ["method named arguments already parse", 'fn main() { print("x".contains(value: "x")) }', 3001],
];
const dir = mkdtempSync(join(tmpdir(), "aura-syntax-"));
let checked = 0;
function compare(source, wasm, expected, label) {
  const file = join(dir, "source.aura");
  writeFileSync(file, source);
  const native = JSON.parse(execFileSync(nativeBin, [file], { encoding: "utf8" }));
  // `runtime.run()` decorates its result with the loader-internal `abiStatus`
  // word; the native harness prints only the structured result document. Strip
  // the loader-only field so the comparison is exactly the result contract.
  const { abiStatus: _abiStatus, ...wasmDoc } = wasm;
  assert.deepEqual(wasmDoc, native, label); // includes message, line and column
  if (typeof expected === "number") assert.equal(wasm.diagnostics[0]?.code, expected, label);
  else { assert.equal(wasm.status, "ok", label); assert.equal(wasm.stdout, expected, label); }
  checked++;
}
try {
  for (const [name, source, expected] of cases) {
    for (const text of [source, source.replaceAll("\n", "\r\n")]) compare(text, runtime.run(text), expected, name);
  }
  for (const source of [Buffer.from([255]), Buffer.from([39,255,39]), Buffer.from([35,255]), Buffer.from([60,33,45,45,255,45,45,33,62]), Buffer.from([39,226,130])]) {
    compare(source, runBytes(source), 1001, "invalid UTF-8 byte boundary");
  }
} finally { rmSync(dir, { recursive: true, force: true }); }
console.log(`Syntax conformance: ${checked} explicit native/WASM comparisons passed; zero imports.`);
