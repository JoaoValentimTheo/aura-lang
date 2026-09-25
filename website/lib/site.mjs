// The static site generator.
//
// It writes a fully static build into `website/dist/`: one directory per
// route with an `index.html`, the shared assets, the versioned Playground
// runtime artifacts, robots.txt, a sitemap, and a 404 page.
//
// The generator supports a configurable base path so the same output can be
// served from `https://aura.lang.dev/` (base `/`) or from a project subpath
// (`/aura-lang/`). The base is resolved by `lib/base.mjs`.
//
// The `CNAME` file is a **GitHub Pages-only** mechanism: when it is present,
// GitHub Pages claims the named custom domain and 301-redirects the project
// URL to it. It is therefore emitted only when explicitly requested
// (`AURA_EMIT_CNAME=1`), never by default. Emitting it for a Vercel or
// project-site deployment would hijack that deployment's host, which is
// exactly what broke the GitHub Pages project-site fallback. Vercel manages
// custom domains through its own project settings and ignores this file.

import { mkdirSync, writeFileSync, rmSync, cpSync, existsSync, readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { site } from "../site.config.mjs";
import { renderPage } from "./layout.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const websiteRoot = resolve(here, "..");
const repoRoot = resolve(websiteRoot, "..");
const distDir = join(websiteRoot, "dist");

/** Write `contents` to `<dist>/<route>/index.html` (or a file if `file`). */
function writeRoute(route, html, file = "index.html") {
  const dir = route === "" ? distDir : join(distDir, route);
  mkdirSync(dir, { recursive: true });
  writeFileSync(join(dir, file), html);
}

/**
 * Build the site.
 *
 * @param {object} options
 * @param {string} options.base   base URL prefix (default "/")
 * @param {Array}  options.pages  page descriptors from `pages.mjs`
 */
export async function buildSite({ base = "/", pages } = {}) {
  rmSync(distDir, { recursive: true, force: true });
  mkdirSync(distDir, { recursive: true });

  // 1. HTML pages.
  const written = [];
  for (const page of pages) {
    const html = renderPage(
      {
        title: page.title,
        description: page.description,
        path: page.path,
        body: await page.render(base),
        activeKey: page.activeKey,
        withContainer: page.withContainer !== false,
        structuredData: page.structuredData
          ? page.structuredData(base)
          : undefined,
      },
      base,
    );
    writeRoute(page.path, html);
    written.push(page);
  }

  // 2. Static assets.
  cpSync(join(websiteRoot, "assets"), join(distDir, "assets"), {
    recursive: true,
  });

  // 2b. Documentation markdown is rendered to HTML at build time; no runtime
  //     markdown dependency ships to the browser.

  // 3. The Playground runtime artifacts + manifest (immutable, versioned).
  const runtimesSrc = join(repoRoot, "playground", "runtimes");
  if (!existsSync(runtimesSrc)) {
    throw new Error(
      "website build: playground/runtimes is missing; run `node playground/build.mjs` first",
    );
  }
  cpSync(runtimesSrc, join(distDir, "playground", "runtimes"), {
    recursive: true,
  });

  // 4. Playground client assets (worker, runtime loader, editor styles).
  cpSync(join(repoRoot, "playground", "web"), join(distDir, "playground", "web"), {
    recursive: true,
  });

  // 5. robots.txt, sitemap, and the Jekyll bypass.
  writeFileSync(join(distDir, "robots.txt"), `User-agent: *\nAllow: /\nSitemap: ${site.origin}/sitemap.xml\n`);
  writeFileSync(join(distDir, "sitemap.xml"), buildSitemap(written));
  writeFileSync(join(distDir, ".nojekyll"), "");

  // 5b. The GitHub Pages CNAME file is opt-in only (see the module header).
  // It must be present only for a deliberate GitHub Pages custom-domain
  // deployment, because it makes Pages claim the domain and redirect the
  // project URL to it.
  if (process.env.AURA_EMIT_CNAME === "1") {
    const host = new URL(site.origin).host;
    writeFileSync(join(distDir, "CNAME"), `${host}\n`);
  }

  return written;
}

function buildSitemap(pages) {
  const urls = pages
    .map((p) => {
      const loc = `${site.origin}/${p.path}${p.path ? "" : ""}`.replace(/\/+$/, "/");
      const priority = p.path === "" ? "1.0" : "0.7";
      return `  <url><loc>${loc}</loc><priority>${priority}</priority></url>`;
    })
    .join("\n");
  return `<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
${urls}
</urlset>
`;
}

export { distDir, websiteRoot, repoRoot };
