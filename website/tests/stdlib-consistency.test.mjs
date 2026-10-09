// Standard-library documentation consistency.
//
// The website documents builtins and methods in `content/stdlib.mjs`. The
// authoritative registry is `src/stdlib/signatures.rs`, which both the checker
// and the runtime consult. This test extracts the registry and fails if the
// website documents a builtin or method that the registry does not define (or
// documents it under the wrong feature gate), so the docs cannot drift from
// the shipped language.
//
// It also enforces the Keystone typed-JSON and HTTP contracts:
//   * `json_decode_as` is documented with a type position and `E4031`;
//   * HTTP is documented as feature-gated / native-only and no fabricated
//     `http::` / session / cookie / streaming API is claimed.
//
// No network access. Usage: node website/tests/stdlib-consistency.test.mjs

import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const repo = resolve(here, "../..");
const rust = readFileSync(join(repo, "src/stdlib/signatures.rs"), "utf8");

let passed = 0;
let failed = 0;
function check(name, cond, detail) {
  if (cond) passed += 1;
  else {
    failed += 1;
    console.error(`FAIL ${name}${detail ? `: ${detail}` : ""}`);
  }
}

/* --------------------------------------------- parse the Rust registry */
// A `Signature { … }` or `MethodSig { … }` block, preceded optionally by a
// `#[cfg(feature = "…")]` attribute. We only need the name, the optional
// feature gate, and (for methods) the receiver class.
function parseBlocks(source, keyword) {
  const blocks = [];
  const re = new RegExp(
    `(#\\[cfg\\(feature = "([a-z]+)"\\)\\]\\s*)?${keyword}\\s*\\{([\\s\\S]*?)\\n\\s*\\}`,
    "g",
  );
  for (const m of source.matchAll(re)) {
    const feature = m[2] || null;
    const body = m[3];
    const name = body.match(/name:\s*"([^"]+)"/)?.[1];
    const receiver = body.match(/receiver:\s*TypeClass::(\w+)/)?.[1];
    if (name) blocks.push({ name, feature, receiver });
  }
  return blocks;
}

const builtins = parseBlocks(rust, "Signature");
const methodSigs = parseBlocks(rust, "MethodSig");

check("registry builtins parsed", builtins.length > 20, `${builtins.length}`);
check("registry methods parsed", methodSigs.length > 20, `${methodSigs.length}`);

const builtinNames = new Set(builtins.map((b) => b.name));
const builtinFeature = new Map();
for (const b of builtins) builtinFeature.set(b.name, b.feature);

const methodsByReceiver = new Map();
for (const m of methodSigs) {
  const key = m.receiver;
  if (!methodsByReceiver.has(key)) methodsByReceiver.set(key, new Set());
  methodsByReceiver.get(key).add(m.name);
}

/* ------------------------------------- documented names must exist */
const lib = await import(join(repo, "website/content/stdlib.mjs"));
const documented = lib.documentedBuiltinNames();
for (const name of documented) {
  check(`documented builtin exists in registry: ${name}`, builtinNames.has(name));
}

/* ----------------------------- documented feature modules are correctly gated */
for (const mod of lib.featureModules) {
  for (const [name] of mod.functions) {
    const gate = builtinFeature.get(name);
    if (mod.gated) {
      // A gated module's functions must carry the matching feature gate.
      check(
        `gated ${mod.name} builtin ${name} is behind feature "${mod.name}"`,
        gate === mod.name,
        `gate=${gate}`,
      );
    } else if (name === "http_request" || name === "http_get") {
      check(`unexpected gated builtin in ungated module: ${name}`, false);
    } else {
      // An ungated function may still be behind a feature (json/regex/time);
      // it must not be behind `http`.
      check(
        `ungated ${mod.name} builtin ${name} is not http-only`,
        gate !== "http",
        `gate=${gate}`,
      );
    }
  }
}

/* ------------------------------------- documented methods must exist */
// Map a documented receiver name to its Rust `TypeClass` variant.
const RECEIVER_TO_CLASS = {
  string: "Str",
  list: "List",
  array: "Array",
  tuple: "Tuple",
  set: "Set",
  map: "Map",
  range: "Range",
};
for (const m of lib.methods) {
  const rustReceiver = RECEIVER_TO_CLASS[m.receiver];
  const set = methodsByReceiver.get(rustReceiver);
  check(`receiver has a method table: ${m.receiver}`, Boolean(set));
  for (const raw of m.names) {
    const name = raw.replace(/\(.*$/, "").trim();
    check(
      `documented method exists: ${m.receiver}.${name}`,
      Boolean(set && set.has(name)),
      set ? [...set].join(",") : "no receiver",
    );
  }
}

/* ------------------------------------- typed-JSON contract */
const typesDoc = readFileSync(join(repo, "website/content/reference-types.md"), "utf8");
const stdlibDoc = readFileSync(join(repo, "website/content/reference-stdlib.md"), "utf8");
check(
  "json_decode_as documented with a type position",
  /json_decode_as\(text,\s*Type\)/.test(stdlibDoc) ||
    /json_decode_as\(text,\s*Type\)/.test(JSON.stringify(lib.featureModules)),
);
check("typed JSON documents strict E4031", /E4031/.test(stdlibDoc));
check("E4031 is the typed-decode code in the registry docs", /E4031/.test(readFileSync(join(repo, "docs/errors.md"), "utf8")));

/* ------------------------------------- HTTP contract */
check(
  "HTTP documented as feature-gated",
  /`http` feature|feature-gated/i.test(stdlibDoc),
);
check(
  "HTTP documented as native-only",
  /native-only/i.test(stdlibDoc),
);
check(
  "HTTP documents E5002 for an absent host capability",
  /E5002/.test(stdlibDoc),
);
check("HTTP documents E4020 transport failure", /E4020/.test(stdlibDoc));

// No fabricated future API may be claimed as available.
const allDocs =
  stdlibDoc +
  readFileSync(join(repo, "website/content/guide-io.md"), "utf8") +
  readFileSync(join(repo, "website/content/known-limitations.md"), "utf8");
for (const fabricated of [
  "http::get",
  "HTTP Session",
  "cookie jar",
  "multipart upload",
  "streaming download",
]) {
  // A mention is fine only if it explicitly frames the item as unavailable
  // (a future candidate / not available). Reject a bare positive claim.
  const idx = allDocs.indexOf(fabricated);
  if (idx !== -1) {
    const context = allDocs.slice(Math.max(0, idx - 120), idx + 160).toLowerCase();
    const framedAsFuture = /(not|no |future|deferred|candidate|unavailable|does not|isn't|no such)/.test(
      context,
    );
    check(`fabricated HTTP API is framed as unavailable: ${fabricated}`, framedAsFuture);
  } else {
    check(`no fabricated HTTP API claim: ${fabricated}`, true);
  }
}

// `docs` references the registry as the single source of truth.
check(
  "stdlib reference cites the Rust registry",
  /src\/stdlib\/signatures\.rs/.test(stdlibDoc),
);

console.log(`\nstdlib-consistency: ${passed} passed, ${failed} failed`);
process.exit(failed === 0 ? 0 : 1);
