// SBOM generator tests (V1 supply-chain policy, section 53).
//
// `scripts/sbom.sh` emits a CycloneDX-style SBOM from `cargo metadata`. These
// tests verify it is parseable, complete (every component carries a name,
// version, purl, and license), tagged with local-vs-third-party, and includes
// both shipped workspaces.
//
// Usage: node playground/tests/node/sbom.test.mjs

import { execFileSync } from "node:child_process";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const repo = resolve(here, "../../..");

let passed = 0;
let failed = 0;
function check(name, cond, detail) {
  if (cond) passed += 1;
  else {
    failed += 1;
    console.error(`FAIL ${name}${detail ? `: ${detail}` : ""}`);
  }
}

const out = execFileSync("bash", ["scripts/sbom.sh"], { cwd: repo, encoding: "utf8" });
const sbom = JSON.parse(out);

check("bomFormat is CycloneDX", sbom.bomFormat === "CycloneDX");
check("has a metadata component", sbom.metadata?.component?.name === "aura-lang");
check("has components", Array.isArray(sbom.components) && sbom.components.length > 0);
check(
  "every component has name, version, and purl",
  sbom.components.every((c) => c.name && c.version && c.purl?.startsWith("pkg:cargo/")),
);
check(
  "every component is tagged local or third-party",
  sbom.components.every((c) =>
    c.properties?.some((p) => p.name === "aura:local" && (p.value === "true" || p.value === "false")),
  ),
);
check(
  "every non-local component records a license",
  sbom.components
    .filter((c) => c.properties.some((p) => p.name === "aura:local" && p.value === "false"))
    .every((c) => Array.isArray(c.licenses) && c.licenses.length > 0),
);
check(
  "includes the core and playground runtime local crates",
  sbom.components.some((c) => c.name === "aura-lang") &&
    sbom.components.some((c) => c.name === "aura-playground-runtime"),
);
check(
  "componentCount property matches the component list",
  sbom.metadata.properties.some(
    (p) => p.name === "aura:componentCount" && Number(p.value) === sbom.components.length,
  ),
);
check(
  "output is deterministic",
  execFileSync("bash", ["scripts/sbom.sh"], { cwd: repo, encoding: "utf8" }) === out,
);

console.log(`sbom: ${passed} passed, ${failed} failed`);
if (failed > 0) process.exit(1);
