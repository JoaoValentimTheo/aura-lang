// Documentation consistency mechanism.
//
// A lightweight guard against the documentation drifting from the shipped
// repository, in the spirit of §14 of the redesign brief. It verifies, without
// a documentation compiler:
//
//   * every documentation page declared in `content/docs.mjs` has a Markdown
//     file that exists and is reachable in the built site;
//   * every documented route resolves to a real `index.html` in the build;
//   * the website release identity agrees with the published metadata
//     (`site.config.mjs` vs `content/docs.mjs` vs the runtime manifest);
//   * the normative sidebar links are release-pinned (`blob/v<version>/…`),
//     not floating `blob/main/…` links;
//   * each pinned normative target genuinely exists at the released tag;
//   * historical pages keep their version context.
//
// Usage: node website/tests/docs-consistency.test.mjs

import { readFileSync, existsSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const repo = resolve(here, "../..");
const website = join(repo, "website");

let passed = 0;
let failed = 0;
function check(name, cond, detail) {
  if (cond) passed += 1;
  else {
    failed += 1;
    console.error(`FAIL ${name}${detail ? `: ${detail}` : ""}`);
  }
}

const { site } = await import(join(website, "site.config.mjs"));
const docs = await import(join(website, "content/docs.mjs"));
const manifest = JSON.parse(
  readFileSync(join(repo, "playground/runtimes/manifest.json"), "utf8"),
);

/* ------------------------------------------------ version identity parity */
check("docs version agrees with the site release", docs.docsVersion === site.releaseVersion);
check(
  "docs language version agrees with the site language version",
  docs.docsLanguageVersion === site.languageVersion,
);
check(
  "runtime manifest current matches the site runtime version",
  manifest.current === site.runtimeVersion,
  `manifest=${manifest.current} site=${site.runtimeVersion}`,
);

/* ------------------------------------------------ declared pages exist */
const all = docs.allDocs();
check("documentation manifest is non-empty", all.length > 0, `${all.length}`);
for (const doc of all) {
  const file = join(website, "content", doc.file);
  check(`doc file exists: ${doc.file}`, existsSync(file), file);
  const dist = join(website, "dist", "docs", doc.slug, "index.html");
  check(`doc route built: docs/${doc.slug}/`, existsSync(dist), dist);
}

/* ------------------------------------------------ duplicates */
const slugs = all.map((d) => d.slug);
check("no duplicate documentation slugs", new Set(slugs).size === slugs.length);

/* ------------------------------------------------ release-pinned links */
const docsPage = readFileSync(join(website, "pages", "docs.mjs"), "utf8");
check(
  "normative links are release-pinned",
  !/\/blob\/main\//.test(docsPage),
  "a blob/main/ link survives",
);
check(
  "normative links use the released tag",
  /blob\/v\$\{docsVersion\}\//.test(docsPage),
);

// Each pinned normative target must exist at the released tag.
const pinned = [...docsPage.matchAll(/docs\/([A-Za-z0-9_.\-]+\.md)/g)].map((m) => m[1]);
const uniquePinned = [...new Set(pinned)];
check("some normative targets are pinned", uniquePinned.length >= 4, uniquePinned.join(","));
for (const target of uniquePinned) {
  let exists = false;
  try {
    const out = execFileSync(
      "git",
      ["ls-tree", "-r", "--name-only", `v${site.releaseVersion}`, "--", `docs/${target}`],
      { cwd: repo, encoding: "utf8" },
    );
    exists = out.trim().length > 0;
  } catch {
    exists = false;
  }
  check(
    `pinned normative target exists at v${site.releaseVersion}: docs/${target}`,
    exists,
  );
}

/* ------------------------------------- no floating developer-branch links */
// Every repository documentation link in content/pages must be release-pinned
// (or point at an intentionally stable resource), never at a mutable branch.
import { readdirSync } from "node:fs";
const sourceDirs = [join(website, "content"), join(website, "pages")];
const floating = [];
for (const dir of sourceDirs) {
  for (const name of readdirSync(dir)) {
    if (!/\.(md|mjs)$/.test(name)) continue;
    const text = readFileSync(join(dir, name), "utf8");
    const re = /github\.com\/JoaoValentimTheo\/aura-lang\/blob\/([^/\s")]+)/g;
    for (const m of text.matchAll(re)) {
      if (m[1] === "main" || m[1] === "master" || m[1].startsWith("rewrite/")) {
        floating.push(`${name}: branch ${m[1]}`);
      }
    }
  }
}
check("no floating branch links in docs sources", floating.length === 0, floating.join("; "));

/* ------------------------------------------------ generated build */
const distDocs = join(website, "dist", "docs", "getting-started", "index.html");
if (existsSync(distDocs)) {
  const html = readFileSync(distDocs, "utf8");
  check(
    "generated docs shell has no floating blob/main normative link",
    !/\/blob\/main\/docs\//.test(html),
  );
  check(
    "generated docs shell pins normative links to the release",
    html.includes(`/blob/v${site.releaseVersion}/docs/`),
  );
}

/* ------------------------------------------------ AIS / MCP routes */
// The semantic-tooling pages must exist, live in the intended manifest group,
// and have a built route, so they cannot be dropped from the documentation.
const SEMANTIC = ["ais", "mcp"];
const semanticGroup = docs.docGroups.find((g) => g.title === "Semantic Tooling");
check("a 'Semantic Tooling' group exists", Boolean(semanticGroup));
check(
  "the Semantic Tooling group holds AIS and MCP",
  Boolean(semanticGroup) &&
    SEMANTIC.every((slug) => semanticGroup.items.some((it) => it.slug === slug)),
  semanticGroup ? semanticGroup.items.map((i) => i.slug).join(",") : "no group",
);
for (const slug of SEMANTIC) {
  const entry = all.find((d) => d.slug === slug);
  check(`AIS/MCP manifest entry exists: ${slug}`, Boolean(entry));
  check(`AIS/MCP file exists: ${slug}.md`, existsSync(join(website, "content", `${slug}.md`)));
  check(
    `AIS/MCP route built: docs/${slug}/`,
    existsSync(join(website, "dist", "docs", slug, "index.html")),
  );
}

/* --------------------------- CLI dispatch agrees with documented commands */
// The documented CLI verbs must match the real command inventory, so the docs
// cannot advertise a command the binary does not dispatch (or omit one it does).
const mainRs = readFileSync(join(repo, "src", "main.rs"), "utf8");
const cliDocForCommands = readFileSync(join(website, "content", "cli.md"), "utf8");
for (const cmd of ["run", "check", "eval", "repl", "version", "ais", "mcp"]) {
  check(
    `cli.md documents the \`aura ${cmd}\` command`,
    new RegExp(`\`aura ${cmd}\``).test(cliDocForCommands) ||
      new RegExp(`aura ${cmd}\\b`).test(cliDocForCommands),
    cmd,
  );
  // `ais`/`mcp` are dispatched behind `#[cfg(feature = "json")]`.
  check(
    `main.rs dispatches \`${cmd}\``,
    new RegExp(`Some\\("${cmd}"\\)`).test(mainRs) ||
      new RegExp(`\\("${cmd}",`).test(mainRs),
    cmd,
  );
}

/* --------------------- AIS protocol version agrees with the implementation */
const aisRs = readFileSync(join(repo, "src", "ais.rs"), "utf8");
const aisProtocol = aisRs.match(/AIS_VERSION:\s*&str\s*=\s*"([^"]+)"/)?.[1];
check("AIS protocol version parsed from src/ais.rs", Boolean(aisProtocol), String(aisProtocol));
const aisPage = readFileSync(join(website, "content", "ais.md"), "utf8");
check(
  "ais.md names the implementation AIS protocol version",
  typeof aisProtocol === "string" && aisPage.includes(`AIS/${aisProtocol}`),
  `version=${aisProtocol}`,
);
check(
  "ais.md reports the released language version",
  aisPage.includes(site.languageVersion),
);
check(
  "the docs do not advertise a nonexistent AIS version",
  !/AIS\/0\.[2-9]/.test(aisPage),
);

/* ------------------------- MCP tools agree with the implementation inventory */
const mcpRs = readFileSync(join(repo, "src", "mcp.rs"), "utf8");
const documentedMcpTools = [
  "aura_snapshot",
  "aura_slice",
  "aura_symbol",
  "aura_diagnostics",
  "aura_delta",
  "aura_revision",
  "aura_capabilities",
];
const mcpPage = readFileSync(join(website, "content", "mcp.md"), "utf8");
for (const tool of documentedMcpTools) {
  check(`mcp.rs defines tool ${tool}`, mcpRs.includes(`"${tool}"`), tool);
  check(`mcp.md documents tool ${tool}`, mcpPage.includes(tool), tool);
}
for (const uri of ["aura://schema", "aura://versions", "aura://capabilities"]) {
  check(`mcp.rs defines resource ${uri}`, mcpRs.includes(uri), uri);
  check(`mcp.md documents resource ${uri}`, mcpPage.includes(uri), uri);
}

/* ----------------- available vs unavailable tooling claims stay accurate */
const toolsPageSrc = readFileSync(join(website, "pages", "tools.mjs"), "utf8");
check(
  "tools page lists AIS as available",
  /AIS\/0\.1 semantic interface/.test(toolsPageSrc) && /Available/.test(toolsPageSrc),
);
check("tools page lists the MCP adapter as available", /MCP adapter/.test(toolsPageSrc));
for (const unavailable of ["Language server / LSP", "Debugger / step execution", "Formatter"]) {
  check(
    `tools page keeps "${unavailable}" labelled unavailable`,
    toolsPageSrc.includes(unavailable),
  );
}
check(
  "tools page does not claim LSP is available",
  !/LSP[^"]*"?,\s*"Available"/.test(toolsPageSrc),
);
// The MCP page must state prompts are not provided.
check(
  "mcp.md states prompts are not provided",
  /[Nn]one\.\s*(\*\*Prompt|\*\*prompts)/.test(mcpPage) || /no prompt|None\./i.test(mcpPage),
);
// AIS must not be described as an executable/authority.
check(
  "ais.md states AIS is not a runtime, model provider, or authority",
  /it never executes one|never executes one/i.test(aisPage) &&
    /compiler is the (only|single) semantic authority|semantic authority/i.test(aisPage),
);

/* ------------------------------------------------ historical context */
const known = readFileSync(join(website, "content", "known-limitations.md"), "utf8");
check(
  "known-limitations frames 0.3.2 as planned/not started",
  /0\.3\.2/.test(known) && /(planned|not started)/i.test(known),
);
const migration = readFileSync(join(website, "content", "migration-0-3-1.md"), "utf8");
check(
  "migration-0-3-1 documents the released transition",
  /0\.3\.1/.test(migration) && /0\.2\.1/.test(migration),
);

console.log(`\ndocs-consistency: ${passed} passed, ${failed} failed`);
process.exit(failed === 0 ? 0 : 1);
