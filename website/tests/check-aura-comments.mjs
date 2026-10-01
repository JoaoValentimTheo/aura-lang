// Guard: Aura's line-comment token is `#`, never `//`. A fenced ```aura block
// that uses `//` would not be valid Aura, so this check rejects it in every
// website content page and every runnable example source.
//
// It only inspects Aura code (```aura fences and `source` fields), not the
// surrounding JavaScript/Markdown, where `//` and `https://` are legitimate.
//
// Usage: node website/tests/check-aura-comments.mjs

import { readFileSync, readdirSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const repo = resolve(here, "../..");
const contentDir = join(repo, "website/content");

let failures = 0;
function report(where, line, text) {
  failures += 1;
  console.error(`FAIL ${where}:${line}: Aura comment must use '#' not '//': ${text.trim()}`);
}

// 1. Fenced ```aura blocks in every content page.
for (const name of readdirSync(contentDir).filter((n) => n.endsWith(".md"))) {
  const lines = readFileSync(join(contentDir, name), "utf8").split("\n");
  let inAura = false;
  for (let i = 0; i < lines.length; i += 1) {
    const line = lines[i];
    if (line.trim().startsWith("```")) {
      if (inAura) inAura = false;
      else inAura = line.trim().slice(3).trim() === "aura";
      continue;
    }
    if (inAura && line.includes("//")) report(`website/content/${name}`, i + 1, line);
  }
}

// 2. Runnable example sources.
const { examples } = await import(join(repo, "website/examples/examples.mjs"));
for (const ex of examples) {
  (ex.source || "").split("\n").forEach((line, i) => {
    if (line.includes("//")) report(`example ${ex.id}`, i + 1, line);
  });
}

if (failures > 0) {
  console.error(`\naura-comments: ${failures} problem(s)`);
  process.exit(1);
}
console.log("aura-comments: all Aura examples use '#' comments");
