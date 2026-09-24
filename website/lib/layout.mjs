// The shared page shell: <head>, app bar, navigation, and footer.
//
// The generated site is fully static. This module emits the same shell for
// every page, with per-page SEO metadata and a correct canonical URL derived
// from the configured origin and base path.

import { site, nav, footerColumns } from "../site.config.mjs";
import { icon, url, escapeHtml } from "./components.mjs";

function head({ title, description, path, base, pageType = "website", structuredData }) {
  const fullTitle = path === "" ? `${site.name} — ${site.tagline}` : `${title} — ${site.name}`;
  const canonical = `${site.origin}${url(path, "/")}`;
  const ogImage = `${site.origin}/assets/social-card.svg`;
  const data = structuredData
    ? `<script type="application/ld+json">${JSON.stringify(structuredData)}</script>`
    : "";
  return `<meta charset="utf-8" />
<meta name="viewport" content="width=device-width, initial-scale=1" />
<title>${escapeHtml(fullTitle)}</title>
<meta name="description" content="${escapeHtml(description)}" />
<link rel="canonical" href="${canonical}" />
<link rel="icon" href="${url("assets/favicon.svg", base)}" type="image/svg+xml" />
<meta name="theme-color" content="#4a57a9" media="(prefers-color-scheme: light)" />
<meta name="theme-color" content="#121318" media="(prefers-color-scheme: dark)" />

<meta property="og:type" content="${pageType}" />
<meta property="og:site_name" content="${site.name}" />
<meta property="og:title" content="${escapeHtml(fullTitle)}" />
<meta property="og:description" content="${escapeHtml(description)}" />
<meta property="og:url" content="${canonical}" />
<meta property="og:image" content="${ogImage}" />

<meta name="twitter:card" content="summary_large_image" />
<meta name="twitter:title" content="${escapeHtml(fullTitle)}" />
<meta name="twitter:description" content="${escapeHtml(description)}" />
<meta name="twitter:image" content="${ogImage}" />

<link rel="stylesheet" href="${url("assets/tokens.css", base)}" />
<link rel="stylesheet" href="${url("assets/styles.css", base)}" />
<script>
  // Apply the stored theme before first paint to avoid a flash. Kept inline
  // and tiny; the full controller loads deferred.
  (function () {
    try {
      var t = localStorage.getItem("aura-theme");
      if (t === "light" || t === "dark") document.documentElement.setAttribute("data-theme", t);
    } catch (e) {}
  })();
</script>
${data}`;
}

function appBar(activeKey, base) {
  const links = nav
    .map((item) => {
      const current = item.key === activeKey ? ' aria-current="page"' : "";
      return `<a href="${url(item.href, base)}"${current}>${escapeHtml(item.label)}</a>`;
    })
    .join("");
  return `<header class="app-bar">
  <div class="container app-bar__inner">
    <a class="brand" href="${url("", base)}" aria-label="${site.name} home">
      <span class="brand__mark" aria-hidden="true">λ</span>
      <span class="brand__name">${site.name}</span>
      <span class="brand__tag">${escapeHtml(site.languageVersion)}</span>
    </a>
    <button class="nav-toggle" type="button" aria-expanded="false" aria-controls="primary-nav" aria-label="Toggle navigation" data-nav-toggle>
      <span data-nav-icon-open>${icon("menu")}</span>
      <span data-nav-icon-close hidden>${icon("close")}</span>
    </button>
    <nav class="nav" id="primary-nav" aria-label="Primary">
      ${links}
      <a href="${site.repository}" target="_blank" rel="noopener" aria-label="Aura on GitHub">${icon("github")}</a>
      <button class="icon-btn" type="button" data-theme-toggle aria-label="Switch color theme">
        <span data-theme-icon-light>${icon("sun")}</span>
        <span data-theme-icon-dark hidden>${icon("moon")}</span>
      </button>
    </nav>
  </div>
</header>`;
}

function footer(base) {
  const columns = footerColumns
    .map(
      (col) => `<div class="footer-col">
  <h2>${escapeHtml(col.title)}</h2>
  <ul>${col.links
    .map((l) => `<li><a href="${url(l.href, base)}">${escapeHtml(l.label)}</a></li>`)
    .join("")}</ul>
</div>`,
    )
    .join("");
  return `<footer class="site-footer">
  <div class="container">
    <div class="footer-grid">
      <div class="footer-brand">
        <a class="brand" href="${url("", base)}">
          <span class="brand__mark" aria-hidden="true">λ</span>
          <span class="brand__name">${site.name}</span>
        </a>
        <p>${escapeHtml(site.description)}</p>
        <div class="pill-row">
          <a class="chip chip--assist" href="${site.repository}" target="_blank" rel="noopener">GitHub</a>
          <a class="chip chip--assist" href="${url("playground/", base)}">Playground</a>
        </div>
      </div>
      ${columns}
    </div>
    <div class="footer-bottom">
      <span>© ${new Date().getFullYear()} The Aura project · Licensed ${site.license}</span>
      <span>
        <a href="${site.issues}" target="_blank" rel="noopener">Issues</a> ·
        <a href="${site.releases}" target="_blank" rel="noopener">Releases</a> ·
        <a href="${url("about/", base)}">About</a>
      </span>
    </div>
  </div>
</footer>`;
}

/**
 * Render a full page.
 *
 * @param {object} page
 * @param {string} page.title       page title (without the site suffix)
 * @param {string} page.description meta description
 * @param {string} page.path        path relative to the site root ("" for home)
 * @param {string} page.body        main content HTML
 * @param {string} page.activeKey   nav key to mark current
 * @param {string} base             base URL prefix (e.g. "/")
 * @param {boolean} withContainer   wrap body in `.container` (default true)
 * @param {object} structuredData   optional JSON-LD object
 */
export function renderPage(page, base = "/") {
  const { title, description, path, body, activeKey, withContainer = true, structuredData } = page;
  const main = withContainer ? `<div class="container">${body}</div>` : body;
  return `<!doctype html>
<html lang="en">
<head>
${head({ title, description, path, base, structuredData })}
</head>
<body>
<a class="skip-link" href="#main">Skip to content</a>
${appBar(activeKey, base)}
<main id="main">
${main}
</main>
${footer(base)}
<script src="${url("assets/site.js", base)}" type="module"></script>
</body>
</html>`;
}
