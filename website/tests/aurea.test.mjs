// Aurea design-system contract tests (Keystone §29).
//
// Aurea is the Aura-owned, local, dependency-free CSS layer. These checks
// ensure it stays that way: no external framework or CDN reference, tokens
// are defined with custom properties, the accessibility primitives exist,
// and the components do not hard-code colours.

import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, "..");
const css = readFileSync(join(root, "assets", "aurea.css"), "utf8");

let passed = 0;
let failed = 0;
function check(name, cond, detail) {
  if (cond) passed += 1;
  else {
    failed += 1;
    console.error(`FAIL ${name}${detail ? `: ${detail}` : ""}`);
  }
}

// No external framework, no CDN, no runtime design API. Comments are
// stripped first, so a comment that *documents* the absence of a CDN is not
// mistaken for a reference to one.
const rules = css.replace(/\/\*[\s\S]*?\*\//g, "");
for (const forbidden of [
  "@import",
  "bootstrap",
  "tailwind",
  "cdn",
  "https://",
  "http://",
  "fonts.googleapis",
]) {
  check(`no external reference: ${forbidden}`, !rules.toLowerCase().includes(forbidden));
}

// Tokens are CSS custom properties, not magic values.
check("defines design tokens", /--au-ink-0:/.test(css));
check("defines a black-first ramp", /--au-black:\s*#000000/.test(css));
check("defines a focus token", /--au-focus:/.test(css));
check("defines a spacing scale", /--au-space-4:/.test(css));

// Accessibility primitives.
check("has a focus-visible contract", css.includes(":focus-visible"));
check("respects reduced motion", css.includes("prefers-reduced-motion"));
check("respects more contrast", css.includes("prefers-contrast: more"));
check("respects forced colors", css.includes("forced-colors: active"));
check("provides a wrap primitive", css.includes(".au-wrap"));
check("provides a local scroll primitive", css.includes(".au-scroll-x"));

// The light theme is a structural inversion, not a second system.
check("has a light theme override", css.includes('[data-theme="light"]'));

// Component rules consume tokens; they do not hard-code hex colours outside
// the token blocks. Count hex literals: they must all be inside `:root` or
// `[data-theme="light"]` token declarations.
const hexOutsideTokens = (() => {
  const lines = css.split("\n");
  let inTokens = false;
  let offenders = 0;
  for (const line of lines) {
    if (line.includes(":root {") || line.includes('[data-theme="light"] {')) inTokens = true;
    else if (line.trim() === "}") inTokens = false;
    else if (!inTokens && /#[0-9a-fA-F]{3,8}\b/.test(line)) offenders += 1;
  }
  return offenders;
})();
check("no magic colours in components", hexOutsideTokens === 0, `${hexOutsideTokens} line(s)`);

console.log(`\nAurea: ${passed} passed, ${failed} failed`);
process.exit(failed === 0 ? 0 : 1);
