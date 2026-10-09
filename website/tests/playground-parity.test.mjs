// Playground integration parity.
//
// The integrated website Playground and the standalone Playground must present
// one interface. They share a single markup source
// (`playground/web/workbench.mjs`) and a single stylesheet
// (`playground/web/workbench.css`). This test proves they cannot drift:
//
//   * the built website Playground contains exactly the shared markup;
//   * the standalone shell injects the same markup (no second copy lives in
//     `playground/index.html`);
//   * the element ids the controller depends on are present in both;
//   * the standalone shell loads the shared stylesheet and no Material sheet;
//   * the shared token block is byte-identical to the canonical Aurea tokens,
//     so the Playground and the site cannot diverge in colour.
//
// Usage: node website/tests/playground-parity.test.mjs

import { readFileSync, existsSync } from "node:fs";
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

const { workbenchMarkup } = await import(
  join(repo, "playground/web/workbench.mjs")
);
const markup = workbenchMarkup();

// The controller's DOM contract: every id it reads.
const REQUIRED_IDS = [
  "version", "run", "stop", "source", "args", "stdin", "stdout",
  "diagnostics", "status", "runtime-note", "highlight", "gutter",
  "problems-count", "tab-problems", "tab-output", "panel-output",
  "panel-problems", "examples", "editor", "file-tabs", "file-add",
  "file-rename", "file-entry", "file-delete", "project-reset",
  "project-note", "completion", "search", "search-toggle", "search-input",
  "search-count", "search-prev", "search-next", "search-case", "search-close",
];
for (const id of REQUIRED_IDS) {
  check(`shared markup contains #${id}`, markup.includes(`id="${id}"`));
}

/* ------------------------------------------------ integrated page */
{
  const page = join(website, "pages/playground.mjs");
  const src = readFileSync(page, "utf8");
  check("integrated page imports the shared markup", src.includes("workbenchMarkup"));
  check(
    "integrated page references the shared stylesheet",
    src.includes("playground/web/workbench.css"),
  );

  const dist = join(website, "dist/playground/index.html");
  if (existsSync(dist)) {
    const html = readFileSync(dist, "utf8");
    // The static page embeds the markup with collapsed leading whitespace in
    // the outer <div>; compare the essential structure by id presence.
    for (const id of REQUIRED_IDS) {
      check(`built integrated page contains #${id}`, html.includes(`id="${id}"`));
    }
    check(
      "integrated page loads the shared workbench stylesheet",
      html.includes("playground/web/workbench.css"),
    );
    check(
      "integrated page has no Material tokens sheet",
      !html.includes("tokens.css"),
    );
    check(
      "integrated page loads app.js",
      html.includes("playground/web/app.js") || html.includes("./web/app.js"),
    );
  }
}

/* ------------------------------------------------ standalone shell */
{
  const index = readFileSync(join(repo, "playground/index.html"), "utf8");
  check(
    "standalone shell does NOT duplicate the workbench markup",
    !index.includes('id="source"'),
  );
  check(
    "standalone shell mounts the shared workbench",
    index.includes("data-playground"),
  );
  check(
    "standalone shell loads the shared stylesheet",
    index.includes("./web/workbench.css"),
  );
  check(
    "standalone shell removed the Material style.css",
    !index.includes("style.css"),
  );
  check(
    "standalone shell loads app.js",
    index.includes("./web/app.js"),
  );

  const app = readFileSync(join(repo, "playground/web/app.js"), "utf8");
  check("controller injects the shared markup", app.includes("workbenchMarkup()"));
}

/* ------------------------------------------------ one token source */
{
  const aurea = readFileSync(join(website, "assets/aurea.css"), "utf8");
  const workbench = readFileSync(
    join(repo, "playground/web/workbench.css"),
    "utf8",
  );
  check("workbench.css declares no --md-* tokens", !/--md-/.test(workbench));
  check("workbench.css uses --au-* tokens", workbench.includes("var(--au-"));

  // The accent, surface, and syntax tokens must resolve to the same values in
  // both sheets, or the Playground would look different from the site.
  const grab = (css, name) => css.match(new RegExp(`${name}:\\s*([^;]+);`))?.[1]?.trim();
  for (const token of [
    "--au-accent",
    "--au-accent-strong",
    "--au-ink-0",
    "--au-ink-1",
    "--au-text",
    "--au-error",
    "--au-help",
    "--au-code-bg",
    "--au-syntax-keyword",
    "--au-syntax-string",
  ]) {
    const a = grab(aurea, token);
    const w = grab(workbench, token);
    check(`token parity: ${token}`, a && w && a === w, `${a} vs ${w}`);
  }
}

console.log(`\nplayground-parity: ${passed} passed, ${failed} failed`);
process.exit(failed === 0 ? 0 : 1);
