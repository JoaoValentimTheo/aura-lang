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
* **Configurable base path.** `--base=/prefix/` builds for a project subpath;
  all internal links, assets, the Worker, and runtime URLs are generated through
  one helper, so the same output works at any base.

## Building

The Playground runtime artifacts must exist first:

```bash
node playground/build.mjs      # build + manifest the immutable wasm runtime
node website/build.mjs         # → website/dist/
node website/tests/run-all.mjs # examples, links, browser, a11y
```

## Playground integration

The [`/playground/`](pages/playground.mjs) page reuses the validated Gaiola 7
Playground **unchanged**. The build copies `playground/web/` and the immutable
`playground/runtimes/` into `dist/playground/`, so the controller's relative
paths resolve exactly as they do for the standalone Playground. The website
page only provides the shell; it contains no execution logic.

## Custom domain

The production domain is `aura.lang.dev`. Configure it in the repository's
GitHub Pages settings and add a DNS `CNAME` record for the subdomain pointing at
GitHub Pages. The build writes the `CNAME` file; DNS and Pages settings are
external, deliberate steps.
