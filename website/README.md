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
* **Static.** The build writes `robots.txt`, `sitemap.xml`, a `404.html`, and
  `.nojekyll`. It is host-agnostic static output, deployable to Vercel or
  GitHub Pages.
* **Explicit deployment base.** `lib/base.mjs` is the single source of truth
  for the base path. The default is the **project-site** base `/aura-lang/`
  (the GitHub Pages fallback); a root-domain build (Vercel / custom domain)
  states its intent with `--base=/` or `AURA_SITE_BASE=/`. Every internal
  link, asset, the Worker, and runtime URL is generated through one helper,
  and in-content Markdown links are resolved through the same base, so the
  whole site moves coherently.
* **No CNAME by default.** `CNAME` is a GitHub Pages-only mechanism that makes
  Pages claim a custom domain and 301-redirect its project URL to it. It is
  emitted only when `AURA_EMIT_CNAME=1` is set for a deliberate Pages
  custom-domain deployment. Vercel manages custom domains through its own
  project settings and does not use this file.

## Deployment base

| Target | URL | Base |
|---|---|---|
| Vercel / custom domain (primary) | `https://aura.lang.dev/` | `/` |
| GitHub Pages project site (fallback) | `https://<user>.github.io/aura-lang/` | `/aura-lang/` |

Resolution order: `--base=<path>`, then `AURA_SITE_BASE`, then the
project-site default. A bare `node website/build.mjs` builds correctly for the
GitHub Pages fallback that currently exists; the Vercel production build sets
`AURA_SITE_BASE=/`.

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

## Hosting

Primary hosting is **Vercel** at `https://aura.lang.dev/` (base `/`); GitHub
Pages remains a fallback at the project URL (`/aura-lang/`). See
[`../docs/vercel-migration.md`](../docs/vercel-migration.md) for the deployment
model, the Vercel project settings, and the DNS steps.

The intended public domain is `aura.lang.dev`. It is **not yet configured**:
DNS currently has no `aura.lang.dev` record and the Vercel project does not yet
exist (both require account-level actions outside the repository). The site's
canonical URLs already use `https://aura.lang.dev`, so once the domain is
attached to Vercel the only build change is the base:

```bash
AURA_SITE_BASE=/ node website/build.mjs
```

DNS, the Vercel project, and the custom domain are external, deliberate steps
and are not performed by the build.
