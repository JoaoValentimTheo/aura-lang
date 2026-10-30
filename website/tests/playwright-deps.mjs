// Playwright availability for the browser test suites.
//
// A browser suite has two independent environment requirements, and they fail
// for different reasons with different remedies:
//
//   1. the `playwright` package is not installed  -> run `npm ci`
//   2. the package is installed but the Chromium
//      binary was never downloaded                 -> run `npx playwright install chromium`
//
// Collapsing those two into one boolean is what let the suite silently pass:
// a broken install reported "dependencies unavailable, skipped" and exited 0.
// `detect()` therefore reports which layer is missing, and `load()` throws on
// anything that is neither layer - a genuine breakage must be loud.
//
// Usage:
//
//   import { detect, skipExitCode } from "../tests/playwright-deps.mjs";
//   const deps = detect(import.meta.url);
//   if (!deps.ok) process.exit(skipExitCode(deps, "my browser suite"));
//   const { chromium } = deps;
//
// `skipExitCode` owns both the message and the code, so a suite never prints a
// duplicate line and never decides severity on its own. It returns 0 when a
// skip is allowed and 1 under `AURA_REQUIRE_BROWSER`, which is the only way CI
// can distinguish "ran the browser" from "quietly skipped it".
//
// Resolution is done with `createRequire` rather than an `import()` of a
// resolved path: `playwright`'s entry point is CommonJS, so importing it by
// absolute path yields a default-only namespace with no named `chromium`
// export, and every caller would break on `chromium.launch`.

import { createRequire } from "node:module";
import { existsSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

function describe(err) {
  return String(err?.message ?? err).split("\n")[0];
}

/** Nearest `package.json` directory at or above `baseUrl`, for the remedy text. */
function packageDir(baseUrl) {
  let dir = baseUrl.startsWith("file:")
    ? dirname(fileURLToPath(baseUrl))
    : dirname(baseUrl);
  for (;;) {
    if (existsSync(join(dir, "package.json"))) return dir;
    const parent = dirname(dir);
    if (parent === dir) return null;
    dir = parent;
  }
}

function hints(baseUrl) {
  const dir = packageDir(baseUrl);
  const where = dir ? `cd ${dir}` : "cd website";
  return {
    install: `Install with: (${where} && npm ci && npx playwright install chromium)`,
    browser: `Install the browser with: (${where} && npx playwright install chromium)`,
  };
}

/**
 * Inspect Playwright availability from the given module URL.
 *
 * @param {string} baseUrl `import.meta.url` of the calling test file, so
 *   resolution starts from the package that declares the dependency.
 * @returns {{ok: true, playwright: object, chromium: object}
 *          | {ok: false, layer: "package"|"browser", reason: string}}
 */
export function detect(baseUrl) {
  const require = createRequire(baseUrl);
  const hint = hints(baseUrl);

  let playwright;
  try {
    playwright = require("playwright");
  } catch (err) {
    // Only a missing module means "not installed". A syntax error, a corrupt
    // install, or a native binding failure is a defect and must propagate.
    if (err?.code !== "MODULE_NOT_FOUND") throw err;
    return {
      ok: false,
      layer: "package",
      reason: `playwright is not installed. ${hint.install}`,
    };
  }

  const chromium = playwright?.chromium;
  if (typeof chromium?.launch !== "function") {
    throw new Error(
      "playwright loaded but exports no chromium.launch(); the install is corrupt",
    );
  }

  // `executablePath()` is inert - it reports the expected location without
  // launching anything, so this distinguishes "binary not downloaded" from a
  // launch failure without paying for a browser start.
  let executable;
  try {
    executable = chromium.executablePath();
  } catch (err) {
    throw new Error(`playwright could not report a chromium path: ${describe(err)}`);
  }
  if (!executable || !existsSync(executable)) {
    return {
      ok: false,
      layer: "browser",
      reason: `playwright is installed but the Chromium binary is missing (${executable || "unknown path"}). ${hint.browser}`,
    };
  }

  return { ok: true, playwright, chromium };
}

/**
 * `detect()`, but with an unresolvable optional dependency treated as absent.
 *
 * The accessibility suite also needs `axe-core`; when Playwright itself is
 * missing that second check is pointless, so callers get a single verdict.
 *
 * @param {string} baseUrl `import.meta.url` of the calling test file.
 * @param {string[]} [packages] Extra packages that must resolve.
 */
export function detectAll(baseUrl, packages = []) {
  const deps = detect(baseUrl);
  if (!deps.ok) return deps;

  const require = createRequire(baseUrl);
  const hint = hints(baseUrl);
  for (const name of packages) {
    try {
      require.resolve(name);
    } catch (err) {
      if (err?.code !== "MODULE_NOT_FOUND") throw err;
      return {
        ok: false,
        layer: "package",
        reason: `${name} is not installed. ${hint.install}`,
      };
    }
  }
  return deps;
}

/**
 * Whether a missing browser is allowed to be a skip at all.
 *
 * Locally, a developer without Playwright installed should still be able to run
 * the static suites, so absence is a skip. In CI it must never be: a runner that
 * is supposed to exercise the browser and quietly skips it is the exact failure
 * this helper exists to remove. Setting `AURA_REQUIRE_BROWSER=1` (which the
 * workflows do) turns every skip into a failure, so a broken `npm ci` or a
 * failed browser download is a red build rather than a green one.
 */
export function strictRequested() {
  const raw = (process.env.AURA_REQUIRE_BROWSER ?? "").trim().toLowerCase();
  return raw !== "" && raw !== "0" && raw !== "false";
}

/**
 * Print the outcome for an unavailable dependency and return the exit code the
 * caller should use.
 *
 * @param {{ok: false, layer: string, reason: string}} deps
 * @param {string} label Human-readable name of what is being skipped, e.g.
 *   `"browser/worker/cache"`.
 * @returns {0|1} `0` when a skip is allowed, `1` when it is not.
 */
export function skipExitCode(deps, label) {
  if (strictRequested()) {
    console.error(
      `\n=== ${label} ===\nFAILED: browser dependencies are required ` +
        `(AURA_REQUIRE_BROWSER=1), but ${deps.reason}`,
    );
    return 1;
  }
  console.log(`\n=== ${label} ===\nSKIPPED: ${deps.reason}`);
  return 0;
}
