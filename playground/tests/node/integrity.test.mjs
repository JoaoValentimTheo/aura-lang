// Load-time SHA-256 integrity verification tests.
//
// The manifest records each immutable artifact's SHA-256. `build.mjs --check`
// verifies it at build time, and `manifest.test.mjs` verifies it on disk — but
// the *loader* (`web/runtime.mjs`, used by the Worker and the Node harness)
// must verify the bytes it actually fetched, before `WebAssembly.instantiate`,
// or the "versioned, immutable artifacts identified by a recorded hash" claim
// is only true on disk and not at the point where untrusted bytes could be
// served.
//
// These tests exercise the loader's verification directly:
//   * a correctly hashed artifact loads (no regression);
//   * a corrupted, truncated, or empty byte stream is REJECTED before
//     instantiation with a structured `RuntimeIntegrityError`;
//   * the platform `crypto.subtle` digest and the vendored pure-JS fallback
//     agree byte-for-byte, so the fallback cannot silently diverge;
//   * the check runs for every available manifest entry (release and
//     development).
//
// Usage: node playground/tests/node/integrity.test.mjs

import { readFileSync } from "node:fs";
import { join, resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const playground = resolve(here, "../..");
const runtimesDir = join(playground, "runtimes");
const manifest = JSON.parse(readFileSync(join(runtimesDir, "manifest.json"), "utf8"));

const { AuraRuntime, RuntimeIntegrityError, sha256Hex } = await import(
  join(playground, "web", "runtime.mjs")
);

let passed = 0;
let failed = 0;
function check(name, cond, detail) {
  if (cond) passed += 1;
  else {
    failed += 1;
    console.error(`FAIL ${name}${detail ? `: ${detail}` : ""}`);
  }
}

const available = manifest.versions.filter((v) => v.available);
check("manifest has an available release and development entry", available.length >= 2);

// --- 1. A correctly hashed artifact loads normally (no regression). ---------
for (const entry of available) {
  const bytes = new Uint8Array(readFileSync(join(runtimesDir, entry.artifact)));
  const runtime = await AuraRuntime.fromBytes(bytes, entry.id, { expectedSha256: entry.sha256 });
  check(`verified load succeeds for ${entry.id}`, runtime.runtimeVersion === entry.runtime_version);
}

// --- 2. Corrupted / truncated / empty bytes are rejected before instantiate. -
const target = available.find((v) => v.channel === "development") || available[0];
const goodBytes = new Uint8Array(readFileSync(join(runtimesDir, target.artifact)));

const corrupt = goodBytes.slice();
corrupt[Math.floor(corrupt.length / 2)] ^= 0xff; // flip a bit in the body
const truncated = goodBytes.slice(0, Math.floor(goodBytes.length / 2));
const empty = new Uint8Array(0);

for (const [label, bytes] of [
  ["bit-flipped", corrupt],
  ["truncated", truncated],
  ["empty", empty],
]) {
  let threw = null;
  try {
    await AuraRuntime.fromBytes(bytes, label, { expectedSha256: target.sha256 });
  } catch (e) {
    threw = e;
  }
  check(
    `${label} artifact is rejected`,
    threw instanceof RuntimeIntegrityError,
    threw ? `${threw.name}: ${threw.message}` : "no error thrown",
  );
  check(
    `${label} rejection carries a structured code`,
    threw && threw.code === "RUNTIME_INTEGRITY",
  );
  check(
    `${label} rejection reports the mismatch, not a raw JS exception`,
    threw && typeof threw.expected === "string" && typeof threw.actual === "string",
  );
}

// A wrong-but-well-formed hash (a stale manifest entry) must also fail.
{
  let threw = null;
  try {
    await AuraRuntime.fromBytes(goodBytes, target.id, { expectedSha256: "0".repeat(64) });
  } catch (e) {
    threw = e;
  }
  check("stale/incorrect expected hash is rejected", threw instanceof RuntimeIntegrityError);
}

// An artifact whose bytes are valid but whose declared hash is for different
// bytes must not be silently accepted just because it compiles.
{
  const other = available.find((v) => v !== target);
  let threw = null;
  try {
    await AuraRuntime.fromBytes(goodBytes, "swap", { expectedSha256: other.sha256 });
  } catch (e) {
    threw = e;
  }
  check("artifact swapped for another version is rejected", threw instanceof RuntimeIntegrityError);
}

// --- 3. The check runs for every available entry (release and development). --
const channels = new Set(available.map((v) => v.channel || "release"));
for (const entry of available) {
  const bytes = new Uint8Array(readFileSync(join(runtimesDir, entry.artifact)));
  const hex = await sha256Hex(bytes);
  check(
    `loader digest matches the manifest for ${entry.id} (${entry.channel || "release"})`,
    hex === entry.sha256,
    `${hex} != ${entry.sha256}`,
  );
}
check("both release and development channels are covered", channels.size >= 2);

// --- 4. The pure-JS fallback agrees with the platform digest. ---------------
// Force the fallback by masking `crypto.subtle`, then compare against the real
// digest over several lengths (empty, sub-block, exact block, multi-block) so
// padding and the length field are exercised.
{
  const realDigest = async (b) => {
    const d = await globalThis.crypto.subtle.digest("SHA-256", b);
    return [...new Uint8Array(d)].map((x) => x.toString(16).padStart(2, "0")).join("");
  };
  const samples = [
    new Uint8Array(0),
    new TextEncoder().encode("abc"),
    new Uint8Array(55),
    new Uint8Array(56),
    new Uint8Array(63),
    new Uint8Array(64),
    new Uint8Array(65),
    new Uint8Array(1000).fill(7),
    goodBytes,
  ];
  for (const s of samples) {
    const real = await realDigest(s);
    const saved = globalThis.crypto;
    // Mask SubtleCrypto so sha256Hex takes the fallback path.
    Object.defineProperty(globalThis, "crypto", { value: { subtle: undefined }, configurable: true });
    const fallback = await sha256Hex(s);
    Object.defineProperty(globalThis, "crypto", { value: saved, configurable: true });
    check(
      `pure fallback matches platform digest (len ${s.length})`,
      fallback === real,
      `${fallback} != ${real}`,
    );
  }
}

// An omitted hash means "no verification" (low-level callers), and must still
// load a valid artifact — this is the escape hatch the manifest path never uses.
{
  const runtime = await AuraRuntime.fromBytes(goodBytes, "unverified");
  check("omitting expectedSha256 loads without verification", !!runtime.exports.aura_run);
}

console.log(`\nIntegrity: ${passed} passed, ${failed} failed`);
process.exit(failed === 0 ? 0 : 1);
