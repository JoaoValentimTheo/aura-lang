// The single source of truth for the website's deployment base path.
//
// The base is never assumed ad hoc: it is resolved once, here, and every tool
// (the generator and the link checker) consumes the same value. A base change
// therefore moves the whole site coherently.
//
// Deployment targets:
//   * GitHub Pages project site — https://<user>.github.io/aura-lang/
//     base = /aura-lang/
//   * custom domain — https://aura.lang.dev/
//     base = /
//
// Precedence:
//   1. an explicit `--base=<path>` argument,
//   2. the `AURA_SITE_BASE` environment variable,
//   3. the built-in default.
//
// The built-in default is the **project-site** base (`/aura-lang/`) because
// that is the currently active GitHub Pages deployment; a custom-domain build
// must state its intent explicitly (`--base=/` or `AURA_SITE_BASE=/`). This
// keeps a bare `node website/build.mjs` correct for the deployment that
// actually exists.

export const PROJECT_BASE = "/aura-lang/";
export const CUSTOM_DOMAIN_BASE = "/";

/** Normalize a base so it always starts and ends with exactly one slash. */
export function normalizeBase(raw) {
  if (!raw || raw === "/" || raw === ".") return "/";
  let b = raw.trim();
  if (!b.startsWith("/")) b = `/${b}`;
  if (!b.endsWith("/")) b = `${b}/`;
  return b;
}

/**
 * Resolve the deployment base from an argument list and the environment.
 *
 * @param {string[]} argv  process arguments (may contain `--base=…`)
 * @param {Record<string, string|undefined>} env  environment (defaults to process.env)
 */
export function resolveBase(argv = process.argv, env = process.env) {
  const arg = argv.find((a) => a.startsWith("--base="));
  if (arg) return normalizeBase(arg.slice("--base=".length));
  if (env.AURA_SITE_BASE) return normalizeBase(env.AURA_SITE_BASE);
  return PROJECT_BASE;
}
