# The Aura website

The static website for Aura, deployed to GitHub Pages at
**https://aura.lang.dev**.

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
* **Static, GitHub Pages-first.** The build writes `CNAME` (`aura.lang.dev`),
  `robots.txt`, `sitemap.xml`, a `404.html`, and `.nojekyll`.
* **Explicit deployment base.** `lib/base.mjs` is the single source of truth
  for the base path. The default is the **project-site** base `/aura-lang/`
  (the active GitHub Pages deployment); a custom-domain build states its intent
  with `--base=/` or `AURA_SITE_BASE=/`. Every internal link, asset, the
  Worker, and runtime URL is generated through one helper, and in-content
  Markdown links are resolved through the same base, so the whole site moves
  coherently.

## Deployment base

| Target | URL | Base |
|---|---|---|
| GitHub Pages project site | `https://<user>.github.io/aura-lang/` | `/aura-lang/` |
| Custom domain | `https://aura.lang.dev/` | `/` |

Resolution order: `--base=<path>`, then `AURA_SITE_BASE`, then the
project-site default. A bare `node website/build.mjs` therefore builds
correctly for the deployment that actually exists, and switching to the custom
domain is a one-variable change (`AURA_SITE_BASE=/`) with no source edit.

## Building

The Playground runtime artifacts must exist first:

```bash
node playground/build.mjs      # build + manifest the immutable wasm runtime
node website/build.mjs         # → website/dist/ (base /aura-lang/ by default)
node website/build.mjs --base=/          # custom-domain variant
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

## Custom domain

The intended public domain is `aura.lang.dev`. It is **not yet configured**:
the site currently serves from the GitHub Pages project URL, `/aura-lang/`.
The generator always writes the `CNAME` file (`aura.lang.dev`) and the site's
canonical URLs use it, so once the domain is configured in the repository's
Pages settings with a DNS record, only the build base changes:

```bash
AURA_SITE_BASE=/ node website/build.mjs
```

DNS and Pages settings are external, deliberate steps and are not performed by
the build.
