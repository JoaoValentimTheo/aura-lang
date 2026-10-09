// Aurea design-system contract tests.
//
// Aurea is the Aura-owned, local, dependency-free CSS layer and the *only*
// design system of the active website. These checks ensure it stays that way:
// no external framework or CDN reference, tokens are defined with custom
// properties, the accessibility primitives exist, the components do not
// hard-code colours, and there is no Material You / Material 3 dependency
// anywhere in the active stylesheet chain.

import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, "..");
const read = (rel) => readFileSync(join(root, rel), "utf8");

const aurea = read("assets/aurea.css");
const styles = read("assets/styles.css");
const layout = read("lib/layout.mjs");
// The Playground workbench stylesheet is the second Aurea consumer; it must
// obey the same single-system rules (no external reference, no --md-*, no
// magic colours outside its token block).
const workbench = readFileSync(
  join(root, "..", "playground", "web", "workbench.css"),
  "utf8",
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

/* ---------------------------------------------- no external dependencies */
// Comments are stripped first, so a comment that *documents* the absence of a
// CDN is not mistaken for a reference to one.
const rules = aurea.replace(/\/\*[\s\S]*?\*\//g, "");
const styleRules = styles.replace(/\/\*[\s\S]*?\*\//g, "");
for (const forbidden of [
  "@import",
  "bootstrap",
  "tailwind",
  "cdn",
  "https://",
  "http://",
  "fonts.googleapis",
]) {
  check(
    `aurea: no external reference: ${forbidden}`,
    !rules.toLowerCase().includes(forbidden),
  );
  check(
    `styles: no external reference: ${forbidden}`,
    !styleRules.toLowerCase().includes(forbidden),
  );
}

/* ------------------------------------------- single design system (Aurea) */
// No Material You / Material 3 design-token reference may survive in the
// active stylesheet chain or the page shell.
const materialRefs = (text) => (text.match(/--md-[a-z0-9-]+/g) || []).length;
check("aurea.css has no --md-* tokens", materialRefs(aurea) === 0, `${materialRefs(aurea)} ref(s)`);
check(
  "styles.css has no --md-* tokens",
  materialRefs(styles) === 0,
  `${materialRefs(styles)} ref(s)`,
);
check(
  "all site styling consumes --au-* tokens",
  styleRules.includes("var(--au-") || styleRules.length === 0,
);
check(
  "layout loads aurea.css",
  layout.includes('assets/aurea.css'),
);
check(
  "layout does not load the retired Material tokens",
  !layout.includes("tokens.css"),
);

// The Playground workbench stylesheet is part of the same system.
const wbRules = workbench.replace(/\/\*[\s\S]*?\*\//g, "");
check("workbench.css has no --md-* tokens", materialRefs(workbench) === 0);
check(
  "workbench.css uses --au-* tokens",
  wbRules.includes("var(--au-"),
);
check(
  "workbench.css declares no external reference",
  !/@import|bootstrap|tailwind|cdn|https?:\/\/|fonts\.googleapis/i.test(wbRules),
);
check(
  "workbench.css has no magic colours outside tokens",
  hexOutsideTokens(workbench) === 0,
  `${hexOutsideTokens(workbench)} line(s)`,
);

/* -------------------------------------------------------- token presence */
// Tokens are CSS custom properties, not magic values.
check("defines design tokens", /--au-ink-0:/.test(aurea));
check("defines a black-first ramp", /--au-black:\s*#000000/.test(aurea));
check("defines a focus token", /--au-focus:/.test(aurea));
check("defines a spacing scale", /--au-space-4:/.test(aurea));
check("defines syntax roles", /--au-syntax-keyword:/.test(aurea));
check("defines status containers", /--au-error-bg:/.test(aurea));

// Accessibility primitives.
check("has a focus-visible contract", aurea.includes(":focus-visible"));
check("respects reduced motion", aurea.includes("prefers-reduced-motion"));
check("respects more contrast", aurea.includes("prefers-contrast: more"));
check("respects forced colors", aurea.includes("forced-colors: active"));
check("provides a wrap primitive", aurea.includes(".au-wrap"));
check("provides a local scroll primitive", aurea.includes(".au-scroll-x"));

// The light theme is a structural inversion, not a second system.
check("has a light theme override", aurea.includes('[data-theme="light"]'));

// Component rules consume tokens; they do not hard-code hex colours outside
// the token blocks. Count hex literals: they must all be inside `:root` or
// `[data-theme="light"]` token declarations.
function hexOutsideTokens(css) {
  const lines = css.split("\n");
  let inTokens = false;
  let offenders = 0;
  for (const line of lines) {
    // A token block opens on any `:root` or `data-theme` selector (including
    // the Playground's `:root:not([data-theme])` OS-preference inversion).
    if (/:root\b[^{]*\{/.test(line) || /\[data-theme[^{]*\{/.test(line)) {
      inTokens = true;
      // A one-line block closes on the same line.
      if (line.includes("}")) inTokens = false;
    } else if (line.trim() === "}") {
      inTokens = false;
    } else if (!inTokens && /#[0-9a-fA-F]{3,8}\b/.test(line)) {
      offenders += 1;
    }
  }
  return offenders;
}
const aOff = hexOutsideTokens(aurea);
check("no magic colours in aurea components", aOff === 0, `${aOff} line(s)`);
const sOff = hexOutsideTokens(styles);
check("no magic colours in site styling", sOff === 0, `${sOff} line(s)`);

/* --------------------------------- injected tokens use the Aurea namespace */
// Page modules and content embed a few inline styles. Every custom-prop
// reference they emit must be an `--au-*` token; a leftover from the retired
// Material/scaffolding vocabulary would render as an undefined value.
import { readdirSync } from "node:fs";
let injectedOffenders = [];
for (const dir of ["pages", "content", "lib"]) {
  for (const name of readdirSync(join(root, dir))) {
    if (!name.endsWith(".mjs")) continue;
    const text = readFileSync(join(root, dir, name), "utf8");
    for (const m of text.matchAll(/var\(--([a-z0-9-]+)\)/g)) {
      if (!m[1].startsWith("au-")) injectedOffenders.push(`${dir}/${name}: --${m[1]}`);
    }
  }
}
check(
  "injected styles use only --au-* tokens",
  injectedOffenders.length === 0,
  injectedOffenders.join("; "),
);

console.log(`\nAurea: ${passed} passed, ${failed} failed`);
process.exit(failed === 0 ? 0 : 1);
