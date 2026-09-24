#!/usr/bin/env node
// Build the Aura website into `website/dist/`.
//
// Usage:
//   node website/build.mjs [--base=/path/]
//
// The deployment base is explicit and selected by the environment, never
// assumed:
//
//   * GitHub Pages **project site** (https://<user>.github.io/aura-lang/):
//     base = /aura-lang/
//   * **custom domain** (https://aura.lang.dev/): base = /
//
// Precedence: `--base=<path>` on the command line, then the `AURA_SITE_BASE`
// environment variable, then the project-site default `/aura-lang/` so the
// live Pages deployment is correct without extra configuration. Every
// internal link, asset, Worker, and runtime URL is generated through one
// helper, so the whole site moves with the base.
//
// Prerequisites: the Playground runtime artifacts must exist. Run
// `node playground/build.mjs` first (or use the repo's combined build script).

import { buildSite } from "./lib/site.mjs";
import { resolveBase } from "./lib/base.mjs";

import { homePage } from "./pages/home.mjs";
import { learnPage } from "./pages/learn.mjs";
import { languagePage } from "./pages/language.mjs";
import { docsIndexPage } from "./pages/docs-index.mjs";
import { docPages } from "./pages/docs.mjs";
import { examplesPage } from "./pages/examples.mjs";
import { stdlibPage } from "./pages/stdlib.mjs";
import { runtimePage } from "./pages/runtime.mjs";
import { toolsPage } from "./pages/tools.mjs";
import { architecturePage } from "./pages/architecture.mjs";
import { roadmapPage } from "./pages/roadmap.mjs";
import { releasesPage } from "./pages/releases.mjs";
import { aboutPage } from "./pages/about.mjs";
import { playgroundPage } from "./pages/playground.mjs";
import { notFoundPage } from "./pages/not-found.mjs";

const base = resolveBase();

const pages = [
  homePage,
  learnPage,
  languagePage,
  docsIndexPage,
  ...docPages(),
  examplesPage,
  stdlibPage,
  runtimePage,
  toolsPage,
  architecturePage,
  roadmapPage,
  releasesPage,
  aboutPage,
  playgroundPage,
];

await buildSite({ base, pages });

// A 404 page for GitHub Pages (served for unknown paths at the site root).
const { distDir } = await import("./lib/site.mjs");
const { renderPage } = await import("./lib/layout.mjs");
const { writeFileSync } = await import("node:fs");
const { join } = await import("node:path");
writeFileSync(
  join(distDir, "404.html"),
  renderPage(
    {
      title: "Page not found",
      description: "The requested page was not found.",
      path: "404/",
      activeKey: "",
      body: await notFoundPage.render(base),
    },
    base,
  ),
);

console.log(`Website built: ${pages.length + 1} pages → website/dist/ (base ${base})`);
