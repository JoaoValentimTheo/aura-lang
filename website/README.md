# The Aura website

The static website for Aura, deployed to GitHub Pages at
**https://joaovalentimtheo.github.io/aura-lang/**.

## Architecture

```text
website/
├── site.config.mjs     public identity, origin, navigation
├── build.mjs           entry point: builds every page into dist/
├── lib/                generator library (layout, components, markdown, highlight)
├── pages/              one module per route
├── content/            documentation and docs manifest (Markdown)
├── examples/           the validated example catalog
├── assets/             design tokens, styles, shared JS, icons
└── tests/              build validation, links, browser, accessibility
```

* **No framework.** A small Node generator emits static HTML. There is no
  client-side router, no server, and no runtime Markdown parsing.
* **Material 3.** `assets/tokens.css` defines the full Material You role
  palette; `assets/styles.css` consumes those roles. Light and dark themes are
  token-driven and respect `prefers-color-scheme`.
* **Static.** The build writes `robots.txt`, `sitemap.xml`, a `404.html`, and
  `.nojekyll`. It is plain static output for GitHub Pages.
* **Explicit deployment base.** `lib/base.mjs` is the single source of truth
  for the base path. The default is the **project-site** base `/aura-lang/`,
  the definitive GitHub Pages deployment. Every internal link, asset, the
  Worker, and runtime URL is generated through one helper, and in-content
  Markdown links are resolved through the same base, so the whole site moves
  coherently.
* **No `CNAME`.** A `CNAME` file makes GitHub Pages claim a custom domain and
  301-redirect the project URL to it. No custom domain is configured, so no
  `CNAME` is emitted.

## Deployment base

| Target | URL | Base |
|---|---|---|
| GitHub Pages project site | `https://joaovalentimtheo.github.io/aura-lang/` | `/aura-lang/` |

Resolution order: `--base=<path>`, then `AURA_SITE_BASE`, then the
project-site default. A bare `node website/build.mjs` builds correctly for the
GitHub Pages deployment; a root-domain build can still state its intent with
`--base=/`.

## Building

The Playground runtime artifacts must exist first:

```bash
node playground/build.mjs      # build + manifest the immutable wasm runtime
node website/build.mjs         # → website/dist/ (base /aura-lang/ by default)
node website/build.mjs --base=/          # root-domain variant (unused by Pages)
node website/tests/run-all.mjs           # examples, links, base, browser, a11y
```

`tests/check-base.mjs` fails the build if any generated internal link bypasses
the configured base — the class of defect that made the Pages project
deployment request CSS and routes from the domain root.

## Playground integration

The [`/playground/`](pages/playground.mjs) page reuses the validated Gaiola 7
Playground **unchanged**. The build copies `playground/web/` and the immutable
`playground/runtimes/` into `dist/playground/`, so the controller's relative
paths resolve exactly as they do for the standalone Playground. The website
page only provides the shell; it contains no execution logic.

## Hosting

Hosting is **GitHub Pages**, deployed by
[`.github/workflows/pages.yml`](../.github/workflows/pages.yml) at the
project-site URL `https://joaovalentimtheo.github.io/aura-lang/` (base
`/aura-lang/`). The canonical URLs and sitemap use that origin. No custom
domain, no `CNAME`, and no external host are involved.
