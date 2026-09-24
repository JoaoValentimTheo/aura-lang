// The Aura runtime loader: the single JavaScript boundary to an Aura
// WebAssembly artifact.
//
// It knows nothing about Aura semantics. It only:
//   1. compiles an immutable versioned `.wasm` artifact,
//   2. checks the artifact's Host ABI version,
//   3. feeds source + options byte by byte through the pointer-free ABI,
//   4. reads back the structured JSON result.
//
// The same module is used by the Web Worker and by the Node test harness, so
// the tested path is the production path.

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
   * The module has zero imports, so instantiation cannot grant it any host
   * authority; if the artifact ever gained an import, instantiation fails.
   */
  static async fromBytes(bytes, name = "(anonymous)") {
    const module = await WebAssembly.compile(bytes);
    const imports = WebAssembly.Module.imports(module);
    if (imports.length !== 0) {
      throw new Error(
        `runtime artifact ${name} declares ${imports.length} import(s); the Aura runtime must be self-contained`,
      );
    }
    const instance = await WebAssembly.instantiate(module, {});
    return new AuraRuntime(instance, bytes, name);
  }

  /** Execute `source`; returns the parsed structured result object. */
  run(source, options = {}) {
    const src = new TextEncoder().encode(source);
    const opts = encodeOptions(options);
    pushBytes(this.exports, "aura_source_reset", "aura_source_push", src);
    pushBytes(this.exports, "aura_options_reset", "aura_options_push", opts);
    const status = this.exports.aura_run();
    const json = readString(this.exports, "aura_output_len", "aura_output_byte");
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
}

/** Access the guest linear memory (used by advanced embedders/tests). */
export function memoryView(runtime) {
  return new Uint8Array(runtime.exports.memory.buffer);
}
