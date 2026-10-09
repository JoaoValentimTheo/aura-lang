// The Aura runtime loader: the single JavaScript boundary to an Aura
// WebAssembly artifact.
//
// It knows nothing about Aura semantics. It only:
//   1. verifies the artifact bytes against the manifest's recorded SHA-256,
//   2. compiles the immutable versioned `.wasm` artifact,
//   3. checks the artifact's Host ABI version,
//   4. feeds source + options byte by byte through the pointer-free ABI,
//   5. reads back the structured JSON result.
//
// The same module is used by the Web Worker and by the Node test harness, so
// the tested path is the production path.

/**
 * A structured integrity failure: the fetched artifact's SHA-256 did not match
 * the hash the manifest records for that immutable version. Thrown before
 * compilation/instantiation, so a mismatched artifact never executes.
 */
export class RuntimeIntegrityError extends Error {
  constructor(message, { artifact = "(anonymous)", expected = null, actual = null } = {}) {
    super(message);
    this.name = "RuntimeIntegrityError";
    this.code = "RUNTIME_INTEGRITY";
    this.artifact = artifact;
    this.expected = expected;
    this.actual = actual;
  }
}

/** Rotate a 32-bit word right by `n` bits. */
function rotr(x, n) {
  return (x >>> n) | (x << (32 - n));
}

// SHA-256 round constants: the first 32 bits of the fractional parts of the
// cube roots of the first 64 primes.
const SHA256_K = [
  0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
  0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
  0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
  0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
  0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
  0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
  0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
  0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

/**
 * A self-contained SHA-256 over a byte array, returning lowercase hex.
 *
 * Used only as a fallback when the platform has no `crypto.subtle` (a
 * non-secure browser context). It is validated byte-for-byte against the
 * platform digest by the integrity tests, so the two implementations cannot
 * diverge unnoticed.
 */
function sha256HexPure(bytes) {
  const h = [
    0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
  ];
  const len = bytes.length;
  const bitLenHi = Math.floor(len / 0x20000000);
  const bitLenLo = (len << 3) >>> 0;
  // Padded length: message + 0x80 + zeros + 8-byte big-endian bit length.
  const withPad = ((len + 9 + 63) >> 6) << 6;
  const buf = new Uint8Array(withPad);
  buf.set(bytes);
  buf[len] = 0x80;
  const dv = new DataView(buf.buffer);
  dv.setUint32(withPad - 8, bitLenHi);
  dv.setUint32(withPad - 4, bitLenLo);

  const w = new Uint32Array(64);
  for (let off = 0; off < withPad; off += 64) {
    for (let i = 0; i < 16; i += 1) w[i] = dv.getUint32(off + i * 4);
    for (let i = 16; i < 64; i += 1) {
      const s0 = rotr(w[i - 15], 7) ^ rotr(w[i - 15], 18) ^ (w[i - 15] >>> 3);
      const s1 = rotr(w[i - 2], 17) ^ rotr(w[i - 2], 19) ^ (w[i - 2] >>> 10);
      w[i] = (w[i - 16] + s0 + w[i - 7] + s1) >>> 0;
    }
    let [a, b, c, d, e, f, g, hh] = h;
    for (let i = 0; i < 64; i += 1) {
      const S1 = rotr(e, 6) ^ rotr(e, 11) ^ rotr(e, 25);
      const ch = (e & f) ^ (~e & g);
      const t1 = (hh + S1 + ch + SHA256_K[i] + w[i]) >>> 0;
      const S0 = rotr(a, 2) ^ rotr(a, 13) ^ rotr(a, 22);
      const maj = (a & b) ^ (a & c) ^ (b & c);
      const t2 = (S0 + maj) >>> 0;
      hh = g;
      g = f;
      f = e;
      e = (d + t1) >>> 0;
      d = c;
      c = b;
      b = a;
      a = (t1 + t2) >>> 0;
    }
    h[0] = (h[0] + a) >>> 0;
    h[1] = (h[1] + b) >>> 0;
    h[2] = (h[2] + c) >>> 0;
    h[3] = (h[3] + d) >>> 0;
    h[4] = (h[4] + e) >>> 0;
    h[5] = (h[5] + f) >>> 0;
    h[6] = (h[6] + g) >>> 0;
    h[7] = (h[7] + hh) >>> 0;
  }
  return h.map((x) => x.toString(16).padStart(8, "0")).join("");
}

/**
 * SHA-256 of a byte array, lowercase hex.
 *
 * Prefers the platform `crypto.subtle` (available in browsers on a secure
 * context and in Node); falls back to the vendored implementation above when
 * it is unavailable, so verification is never silently skipped.
 */
export async function sha256Hex(bytes) {
  const subtle = globalThis.crypto && globalThis.crypto.subtle;
  if (subtle && typeof subtle.digest === "function") {
    const digest = await subtle.digest("SHA-256", bytes);
    return [...new Uint8Array(digest)].map((b) => b.toString(16).padStart(2, "0")).join("");
  }
  return sha256HexPure(bytes);
}

/** Verify the artifact exports the required ABI symbols. */
function assertAbi(exports, name) {
  const required = [
    "aura_abi_version",
    "aura_version_len",
    "aura_version_byte",
    "aura_runtime_version_len",
    "aura_runtime_version_byte",
    "aura_source_reset",
    "aura_source_push",
    "aura_options_reset",
    "aura_options_push",
    "aura_run",
    "aura_output_len",
    "aura_output_byte",
    "memory",
  ];
  for (const sym of required) {
    if (!(sym in exports)) {
      throw new Error(`runtime artifact ${name} is missing ABI export \`${sym}\``);
    }
  }
}

/** Read a UTF-8 string from an export pair (`*_len`, `*_byte`). */
function readString(exports, lenFn, byteFn) {
  const len = exports[lenFn]();
  const bytes = new Uint8Array(len);
  for (let i = 0; i < len; i += 1) {
    bytes[i] = exports[byteFn](i);
  }
  return new TextDecoder("utf-8").decode(bytes);
}

/** Feed a byte array to the guest through a `reset`/`push(word, nbytes)` pair. */
function pushBytes(exports, resetFn, pushFn, bytes) {
  exports[resetFn]();
  for (const b of bytes) {
    exports[pushFn](b, 1);
  }
}

function parseRuntimeResult(json, status) {
  let parsed;
  try {
    parsed = JSON.parse(json);
  } catch (e) {
    return {
      status: "internal",
      stdout: "",
      result: null,
      diagnostics: [
        {
          code: 4999,
          code_text: "E4999",
          message: `runtime produced unreadable result: ${e.message}`,
          line: 1,
          column: 1,
        },
      ],
      abiStatus: status,
    };
  }
  parsed.abiStatus = status;
  return parsed;
}

/** Encode Playground options into the runtime's line protocol. */
function encodeOptions({ args = [], stdin = null } = {}) {
  const chunks = [];
  const enc = new TextEncoder();
  for (const a of args) {
    const line = enc.encode(`arg ${a}\n`);
    chunks.push(line);
  }
  if (stdin !== null && stdin !== undefined) {
    const body = enc.encode(stdin);
    chunks.push(enc.encode(`stdin-bytes ${body.length}\n`));
    chunks.push(body);
  }
  const total = chunks.reduce((n, c) => n + c.length, 0);
  const out = new Uint8Array(total);
  let off = 0;
  for (const c of chunks) {
    out.set(c, off);
    off += c.length;
  }
  return out;
}

/**
 * Verified artifacts, keyed by the expected SHA-256. A hit means the exact
 * bytes for that immutable hash were already verified and compiled in this
 * process, so a repeat load of the same version skips both the digest and the
 * compile. The key is the *expected* hash (never a URL or a version label), so
 * a cache hit can never mask a mismatched artifact.
 */
const verifiedHashes = new Map();

/**
 * A loaded, immutable Aura runtime artifact.
 *
 * Constructing one compiles the bytes; the ABI version and language version
 * are read immediately so a mismatched artifact is rejected before any program
 * runs.
 */
export class AuraRuntime {
  constructor(instance, bytes, name) {
    this.instance = instance;
    this.exports = instance.exports;
    this.name = name;
    this.byteLength = bytes.byteLength;
    assertAbi(this.exports, name);
    this.abiVersion = this.exports.aura_abi_version();
    this.languageVersion = readString(this.exports, "aura_version_len", "aura_version_byte");
    this.runtimeVersion = readString(
      this.exports,
      "aura_runtime_version_len",
      "aura_runtime_version_byte",
    );
  }

  /**
   * Compile a runtime from raw bytes.
   *
   * When `expectedSha256` is provided, the bytes' SHA-256 MUST match it or a
   * {@link RuntimeIntegrityError} is thrown *before* compilation, so a
   * corrupted or substituted artifact can never be instantiated. The Playground
   * loader always supplies the manifest-declared hash; low-level callers may
   * omit it (no verification) at their own risk.
   *
   * The digest is computed on **every** call with an expected hash — it is the
   * security check and must see the actual bytes. Only the compiled runtime is
   * cached, keyed by the verified hash, so a repeat load of the same immutable
   * version skips compilation but can never skip verification.
   *
   * The module has zero imports, so instantiation cannot grant it any host
   * authority; if the artifact ever gained an import, instantiation fails.
   */
  static async fromBytes(bytes, name = "(anonymous)", { expectedSha256 = null } = {}) {
    if (expectedSha256) {
      const actual = await sha256Hex(bytes);
      if (actual !== expectedSha256) {
        throw new RuntimeIntegrityError(
          `runtime artifact ${name} failed integrity verification: ` +
            `expected sha256 ${expectedSha256}, computed ${actual}`,
          { artifact: name, expected: expectedSha256, actual },
        );
      }
      // Verified: a previously compiled instance for these exact bytes is
      // safe to reuse (the hash is over the bytes we just hashed).
      const cached = verifiedHashes.get(expectedSha256);
      if (cached) {
        return cached;
      }
    }
    const module = await WebAssembly.compile(bytes);
    const imports = WebAssembly.Module.imports(module);
    if (imports.length !== 0) {
      throw new Error(
        `runtime artifact ${name} declares ${imports.length} import(s); the Aura runtime must be self-contained`,
      );
    }
    const instance = await WebAssembly.instantiate(module, {});
    const runtime = new AuraRuntime(instance, bytes, name);
    if (expectedSha256) {
      verifiedHashes.set(expectedSha256, runtime);
    }
    return runtime;
  }

  /** Execute `source`; returns the parsed structured result object. */
  run(source, options = {}) {
    const src = new TextEncoder().encode(source);
    const opts = encodeOptions(options);
    pushBytes(this.exports, "aura_source_reset", "aura_source_push", src);
    pushBytes(this.exports, "aura_options_reset", "aura_options_push", opts);
    const status = this.exports.aura_run();
    const json = readString(this.exports, "aura_output_len", "aura_output_byte");
    return parseRuntimeResult(json, status);
  }

  /**
   * Execute a caller-supplied virtual multi-source Aura project.
   *
   * This is an additive Host ABI 1 capability. Historical ABI-1 runtimes may
   * not expose it; callers can feature-detect `runProject` support without
   * affecting the existing single-source `run` path.
   */
  runProject(project, options = {}) {
    const required = ["aura_project_reset", "aura_project_push", "aura_run_project"];
    for (const sym of required) {
      if (!(sym in this.exports)) {
        throw new Error(`runtime artifact ${this.name} does not support virtual projects`);
      }
    }
    const request = new TextEncoder().encode(JSON.stringify(project));
    const opts = encodeOptions(options);
    pushBytes(this.exports, "aura_project_reset", "aura_project_push", request);
    pushBytes(this.exports, "aura_options_reset", "aura_options_push", opts);
    const status = this.exports.aura_run_project();
    const json = readString(this.exports, "aura_output_len", "aura_output_byte");
    return parseRuntimeResult(json, status);
  }

  /**
   * Whether this artifact advertises the Host ABI 2 resumable-session surface.
   *
   * Feature detection replaces inference: a historical ABI-1 artifact simply
   * lacks these exports, so the browser transport must fall back to the
   * synchronous path (and deny HTTP with `E5002`) rather than send it a
   * command it does not understand.
   */
  get supportsSessions() {
    return SESSION_EXPORTS.every((sym) => sym in this.exports);
  }

  /**
   * Start a resumable session over `source` (Host ABI 2).
   *
   * Returns `{ status, result }` where `status` is the raw ABI status code and
   * `result` is the parsed JSON payload. `status === 3` (`PENDING_EFFECT`)
   * means the program suspended and `result` is the effect payload the caller
   * must perform before calling {@link AuraRuntime#resumeSession}.
   *
   * The runtime keeps the parked interpreter in its own slot until the next
   * `start`/`reset`, so the session survives between calls without the caller
   * holding any wasm state.
   */
  startSession(source, options = {}) {
    this.#assertSessions();
    const src = new TextEncoder().encode(source);
    const opts = encodeOptions(options);
    this.exports.aura_session_reset();
    pushBytes(this.exports, "aura_source_reset", "aura_source_push", src);
    pushBytes(this.exports, "aura_options_reset", "aura_options_push", opts);
    const status = this.exports.aura_session_start();
    const json = readString(this.exports, "aura_output_len", "aura_output_byte");
    return { status, result: parseSessionPayload(json, status) };
  }

  /**
   * Resume the parked session with the completion of its pending effect
   * (Host ABI 2). `payload` is the effect result object the transport built.
   */
  resumeSession(payload) {
    this.#assertSessions();
    const bytes = new TextEncoder().encode(JSON.stringify(payload));
    pushBytes(
      this.exports,
      "aura_session_resume_reset",
      "aura_session_resume_push",
      bytes,
    );
    const status = this.exports.aura_session_resume();
    const json = readString(this.exports, "aura_output_len", "aura_output_byte");
    return { status, result: parseSessionPayload(json, status) };
  }

  /** Abort and discard any parked session. */
  resetSession() {
    if (!this.supportsSessions) return;
    this.exports.aura_session_reset();
  }

  #assertSessions() {
    if (!this.supportsSessions) {
      throw new Error(
        `runtime artifact ${this.name} does not support resumable sessions (Host ABI 2)`,
      );
    }
  }
}

/** The exports a runtime must provide to run resumable sessions (Host ABI 2). */
const SESSION_EXPORTS = [
  "aura_session_reset",
  "aura_session_start",
  "aura_session_resume_reset",
  "aura_session_resume_push",
  "aura_session_resume",
];

/** The ABI status code for a pending Host effect (Host ABI 2). */
export const SESSION_PENDING_EFFECT = 3;

/**
 * Parse a session payload: a pending effect is returned verbatim (it is the
 * effect payload), while a terminal step is parsed like a normal result.
 */
function parseSessionPayload(json, status) {
  if (status === SESSION_PENDING_EFFECT) {
    try {
      return JSON.parse(json);
    } catch (e) {
      return {
        status: "internal",
        diagnostics: [
          {
            code: 0,
            codeText: "E0",
            message: `runtime produced an unreadable effect payload: ${e.message}`,
            line: 1,
            column: 1,
          },
        ],
        abiStatus: status,
      };
    }
  }
  return parseRuntimeResult(json, status);
}

/** Access the guest linear memory (used by advanced embedders/tests). */
export function memoryView(runtime) {
  return new Uint8Array(runtime.exports.memory.buffer);
}
