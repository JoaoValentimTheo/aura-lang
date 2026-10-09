// AIS/0.1 documentation fixtures and consistency.
//
// The AIS and MCP documentation pages make concrete claims about the protocol:
// schema fields, capability defaults, version identity, delivery modes, and the
// MCP tool/resource inventory. This test verifies those claims against the
// *actual compiler* rather than trusting prose:
//
//   1. It runs the real `aura ais` CLI over committed fixtures and compares the
//      output byte-for-byte (structurally, by parsed JSON) with committed
//      golden JSON, so a documented example cannot silently drift from what the
//      compiler emits.
//   2. It checks the AIS/MCP pages' factual claims (protocol version, capability
//      advertisement, tool/resource names, version fields) against the same
//      real outputs and against `src/mcp.rs` / `src/ais.rs`.
//
// If no `aura` binary with AIS support is available, the executable half is
// skipped — explicitly, never silently passed. The static half always runs.
//
// Usage: node website/tests/ais-doc.test.mjs

import { execFileSync, execSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const repo = resolve(here, "../..");
const fixtures = join(here, "fixtures", "ais");
const golden = join(fixtures, "golden");

let passed = 0;
let skipped = 0;
let failed = 0;
function check(name, cond, detail) {
  if (cond) passed += 1;
  else {
    failed += 1;
    console.error(`FAIL ${name}${detail ? `: ${detail}` : ""}`);
  }
}
function skip(name, why) {
  skipped += 1;
  console.log(`skip ${name}: ${why}`);
}

/* ---------------------------------------------------- locate the binary */
function findAura() {
  for (const rel of ["target/debug/aura", "target/release/aura"]) {
    const p = join(repo, rel);
    if (!existsSync(p)) continue;
    try {
      const help = execFileSync(p, ["--help"], { encoding: "utf8" });
      if (/\bais\b/.test(help) && /\bmcp\b/.test(help)) return p;
    } catch {
      /* not usable */
    }
  }
  return null;
}
const aura = findAura();

/* ---------------------------------------------------- docs source text */
const aisDoc = readFileSync(join(repo, "website/content/ais.md"), "utf8");
const mcpDoc = readFileSync(join(repo, "website/content/mcp.md"), "utf8");
const cliDoc = readFileSync(join(repo, "website/content/cli.md"), "utf8");
const aisSrc = readFileSync(join(repo, "src/ais.rs"), "utf8");
const mcpSrc = readFileSync(join(repo, "src/mcp.rs"), "utf8");

/* ------------------------------------------------ static claims (always) */
// Protocol identity.
const aisVersion = aisSrc.match(/AIS_VERSION:\s*&str\s*=\s*"([^"]+)"/)?.[1];
check("AIS_VERSION parsed from source", Boolean(aisVersion), String(aisVersion));
check(
  "ais.md documents the implementation protocol version",
  typeof aisVersion === "string" && aisDoc.includes(`AIS/${aisVersion}`),
  `version=${aisVersion}`,
);
check(
  "the website does not advertise a different AIS protocol version",
  !/AIS\/0\.[2-9]/.test(aisDoc + mcpDoc),
);

// Capability advertisement must match the source default exactly.
const capDefaults = { symbols: true, types: true, diagnostics: true, flow: false, completion: false };
for (const [name, value] of Object.entries(capDefaults)) {
  check(
    `ais.md states capability ${name}: ${value}`,
    new RegExp(`"?${name}"?:\\s*${value}`).test(aisDoc),
    `${name}`,
  );
}
// The source's Default impl must agree (guards a source change from silently
// invalidating the docs).
for (const [name, value] of Object.entries(capDefaults)) {
  check(
    `src/ais.rs advertises ${name}: ${value}`,
    new RegExp(`${name}:\\s*${value}`).test(aisSrc),
    `${name}`,
  );
}

// MCP tool inventory must match src/mcp.rs.
const toolNames = [...mcpSrc.matchAll(/"name":\s*"(aura_[a-z_]+)"/g)].map((m) => m[1]);
const uniqueTools = [...new Set(toolNames)];
check("MCP tools parsed from source", uniqueTools.length >= 7, uniqueTools.join(","));
for (const tool of uniqueTools) {
  check(`mcp.md documents tool ${tool}`, mcpDoc.includes(tool), tool);
}

// MCP resource inventory.
for (const uri of ["aura://schema", "aura://versions", "aura://capabilities"]) {
  check(`mcp.md documents resource ${uri}`, mcpDoc.includes(uri), uri);
  check(`mcp.rs defines resource ${uri}`, mcpSrc.includes(uri), uri);
}

// No unavailable capability is advertised as shipped.
for (const forbidden of [/\bLSP\b.*\b(shipped|available|supported)\b/i, /prompts?\s+(are\s+)?(available|implemented|supported)/i]) {
  check(
    `ais/mcp docs do not advertise an unavailable capability (${forbidden})`,
    !forbidden.test(aisDoc) && !forbidden.test(mcpDoc),
  );
}
check(
  "mcp.md states prompts are not provided",
  /[Nn]one\.\s*\*\*Prompt/.test(mcpDoc) || /no prompt|None/i.test(mcpDoc),
);

// CLI page surfaces both commands.
check("cli.md documents `aura ais`", /aura ais/.test(cliDoc));
check("cli.md documents `aura mcp`", /aura mcp/.test(cliDoc));

// Fixture files exist.
for (const f of [
  "identity.aura",
  "collections.aura",
  "diagnostic.aura",
  "before.aura",
  "after.aura",
]) {
  check(`fixture exists: ${f}`, existsSync(join(fixtures, f)), f);
}
for (const f of [
  "identity.snapshot.json",
  "collections.snapshot.json",
  "collections.slice.json",
  "delta.json",
  "diagnostic.snapshot.json",
]) {
  check(`golden exists: ${f}`, existsSync(join(golden, f)), f);
}

/* ------------------------------------------ executable checks (skippable) */
if (!aura) {
  skip("AIS CLI execution", "no `aura` binary with AIS support (build with the json feature)");
} else {
  // A rejected snapshot exits non-zero *and* prints the document, so capture
  // stdout from both the success and failure paths rather than throwing away a
  // valid payload.
  const runAis = (args, stdinFile) => {
    try {
      return execFileSync(aura, ["ais", ...args], {
        encoding: "utf8",
        input: stdinFile ? readFileSync(join(fixtures, stdinFile)) : undefined,
      });
    } catch (e) {
      if (typeof e.stdout === "string" && e.stdout.trim().startsWith("{")) {
        return e.stdout;
      }
      throw e;
    }
  };
  const readJson = (p) => JSON.parse(readFileSync(p, "utf8"));

  // Snapshot — identity levels.
  const identity = JSON.parse(runAis(["-"], "identity.aura"));
  check("snapshot ais_version is 0.1", identity.ais_version === "0.1", identity.ais_version);
  check("snapshot aura_version is 0.3.1", identity.aura_version === "0.3.1", identity.aura_version);
  check("snapshot language_version is 0.3.1", identity.language_version === "0.3.1");
  check(
    "snapshot capabilities match the documented default",
    JSON.stringify(identity.capabilities) === JSON.stringify(capDefaults),
    JSON.stringify(identity.capabilities),
  );
  check("snapshot carries a rev: identity", /^rev:[0-9a-f]{16}$/.test(identity.revision || ""), identity.revision);
  check(
    "identity snapshot matches golden",
    JSON.stringify(identity) === JSON.stringify(readJson(join(golden, "identity.snapshot.json"))),
  );

  // The list symbol keeps its three distinct identity levels.
  const lists = identity.symbols.find((s) => s.name === "lists");
  check("lists.type_name is [[int]]", lists?.type_name === "[[int]]", lists?.type_name);
  check("lists.families is [sequence]", JSON.stringify(lists?.families) === '["sequence"]');
  check("lists.value_kind is list", lists?.value_kind === "list");

  // Collections snapshot — the five identities never collapse.
  const collections = JSON.parse(runAis(["snapshot", "-"], "collections.aura"));
  check(
    "collections snapshot matches golden",
    JSON.stringify(collections) === JSON.stringify(readJson(join(golden, "collections.snapshot.json"))),
  );
  const kinds = Object.fromEntries(collections.symbols.map((s) => [s.name, s.value_kind]));
  check("list value kind", kinds.lists === "list", kinds.lists);
  check("array value kind", kinds.fixed === "array", kinds.fixed);
  check("tuple value kind", kinds.pair === "tuple", kinds.pair);
  check("set value kind", kinds.members === "set", kinds.members);
  check("map value kind", kinds.mapping === "map", kinds.mapping);
  check(
    "list and array share the sequence family but keep distinct value kinds",
    JSON.stringify(collections.symbols.find((s) => s.name === "lists").families) ===
      JSON.stringify(collections.symbols.find((s) => s.name === "fixed").families) &&
      collections.symbols.find((s) => s.name === "lists").value_kind !==
        collections.symbols.find((s) => s.name === "fixed").value_kind,
  );

  // Slice.
  const slice = JSON.parse(runAis(["slice", "-", "mapping", "2", "32"], "collections.aura"));
  check("slice matches golden", JSON.stringify(slice) === JSON.stringify(readJson(join(golden, "collections.slice.json"))));
  check("slice target is the requested symbol", slice.target === "mapping");
  check("slice carries the target symbol", slice.symbol?.name === "mapping");
  check(
    "slice lists the target's dependency",
    Array.isArray(slice.dependencies) && slice.dependencies.some((d) => d.name === "Point"),
  );
  check("slice reports omitted count", typeof slice.omitted === "number");

  // A budget of the target alone still carries the target.
  const tight = JSON.parse(runAis(["slice", "-", "mapping", "5", "1"], "collections.aura"));
  check("an exhausted slice still carries the target", tight.symbol?.name === "mapping");
  check("an exhausted slice reports omitted dependencies", tight.omitted >= 1, String(tight.omitted));

  // Delta.
  const delta = JSON.parse(
    runAis(["delta", join(fixtures, "before.aura"), join(fixtures, "after.aura")]),
  );
  check("delta matches golden", JSON.stringify(delta) === JSON.stringify(readJson(join(golden, "delta.json"))));
  const changes = Object.fromEntries(delta.symbols.map((c) => [c.name, c.change]));
  check("delta reports the added struct", changes.Size === "added", JSON.stringify(changes));
  check("delta reports the shifted declaration as moved", changes.area === "moved" || changes.area === "changed");
  check("delta references both revisions", /^rev:/.test(delta.from_revision) && /^rev:/.test(delta.to_revision));

  // Diagnostic snapshot.
  const diag = JSON.parse(runAis(["-"], "diagnostic.aura"));
  check(
    "diagnostic snapshot matches golden",
    JSON.stringify(diag) === JSON.stringify(readJson(join(golden, "diagnostic.snapshot.json"))),
  );
  const d0 = diag.diagnostics[0];
  check("diagnostic carries the E3001 code text", d0?.code_text === "E3001", d0?.code_text);
  check("diagnostic severity is error", d0?.severity === "error");
  check("diagnostic range has line/column/offset", {
    line: typeof d0?.range?.start?.line,
    column: typeof d0?.range?.start?.column,
    offset: typeof d0?.range?.start?.offset,
  }.line === "number" && typeof d0?.range?.start?.offset === "number");
  check("diagnostic contains no ANSI escapes", !/\u001b\[/.test(JSON.stringify(diag)));

  // Exit code: a rejected snapshot exits non-zero.
  let rejectedCode = 0;
  try {
    execFileSync(aura, ["ais", "-"], {
      input: readFileSync(join(fixtures, "diagnostic.aura")),
      stdio: ["pipe", "ignore", "ignore"],
    });
  } catch (e) {
    rejectedCode = e.status ?? 1;
  }
  check("a rejected snapshot exits non-zero", rejectedCode !== 0, String(rejectedCode));

  // MCP: handshake, tool inventory, resources.
  const mcpInput =
    [
      { jsonrpc: "2.0", id: 1, method: "initialize", params: {} },
      { jsonrpc: "2.0", id: 2, method: "tools/list", params: {} },
      { jsonrpc: "2.0", id: 3, method: "resources/list", params: {} },
      { jsonrpc: "2.0", id: 4, method: "resources/read", params: { uri: "aura://versions" } },
    ]
      .map((r) => JSON.stringify(r))
      .join("\n") + "\n";
  const out = execFileSync(aura, ["mcp"], { encoding: "utf8", input: mcpInput });
  const replies = out
    .trim()
    .split("\n")
    .map((l) => JSON.parse(l));
  const byId = Object.fromEntries(replies.map((r) => [r.id, r.result]));
  check("MCP initialize reports the AIS version", byId[1]?.serverInfo?.ais_version === "0.1", JSON.stringify(byId[1]?.serverInfo));
  const liveTools = byId[2]?.tools?.map((t) => t.name) || [];
  for (const t of uniqueTools) {
    check(`MCP tools/list advertises ${t}`, liveTools.includes(t), liveTools.join(","));
  }
  const liveResources = byId[3]?.resources?.map((r) => r.uri) || [];
  for (const uri of ["aura://schema", "aura://versions", "aura://capabilities"]) {
    check(`MCP resources/list advertises ${uri}`, liveResources.includes(uri), liveResources.join(","));
  }
  const versions = JSON.parse(byId[4]?.contents?.[0]?.text || "{}");
  check("MCP aura://versions reports ais_version 0.1", versions.ais_version === "0.1");
  check("MCP aura://versions reports language_version 0.3.1", versions.language_version === "0.3.1");
}

/* ------------------------------------ documented versions agree with release */
const siteConfig = await import(join(repo, "website/site.config.mjs"));
check(
  "ais.md shows the released language version",
  aisDoc.includes(siteConfig.site.languageVersion),
  siteConfig.site.languageVersion,
);

console.log(`\nais-doc: ${passed} passed, ${skipped} skipped, ${failed} failed`);
process.exit(failed === 0 ? 0 : 1);
