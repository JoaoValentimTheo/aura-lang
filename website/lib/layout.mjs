// The shared page shell: <head>, app bar, navigation, and footer.
//
// The generated site is fully static. This module emits the same shell for
// every page, with per-page SEO metadata and a correct canonical URL derived
// from the configured origin and base path.

import { site, nav, navPrimary, navGroups, footerColumns } from "../site.config.mjs";
import { icon, url, escapeHtml } from "./components.mjs";

function head({ title, description, path, base, pageType = "website", structuredData, extraHead = "" }) {
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
<meta name="theme-color" content="#ffffff" media="(prefers-color-scheme: light)" />
<meta name="theme-color" content="#05060a" media="(prefers-color-scheme: dark)" />

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

<link rel="stylesheet" href="${url("assets/aurea.css", base)}" />
<link rel="stylesheet" href="${url("assets/styles.css", base)}" />
${extraHead}
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

// Build one navigation surface from the single `nav` source of truth.
//
// `keys`, when given, restricts and orders the entries; otherwise every
// destination is rendered. The same generator feeds the wide inline bar, the
// grouped overflow menu, and the compact dropdown, so the three can never
// disagree about a route.
function navLink(item, activeKey, base) {
  const current = item.key === activeKey ? ' aria-current="page"' : "";
  return `<a href="${url(item.href, base)}"${current}>${escapeHtml(item.label)}</a>`;
}

function appBar(activeKey, base) {
  const currentKeys = new Set([activeKey]);
  // The restrained inline set (wide layouts).
  const inline = navPrimary
    .map((key) => nav.find((item) => item.key === key))
    .filter(Boolean)
    .map((item) => navLink(item, activeKey, base))
    .join("");
  // The grouped overflow menu ("More"): every destination not already shown
  // inline, so the two surfaces partition the routes with no duplication and
  // no gap.
  const inlineKeys = new Set(navPrimary);
  const overflowGroups = navGroups
    .map((group) => {
      const keys = group.keys.filter((key) => !inlineKeys.has(key));
      if (keys.length === 0) return "";
      const items = keys
        .map((key) => nav.find((item) => item.key === key))
        .filter(Boolean)
        .map((item) => `<li>${navLink(item, activeKey, base)}</li>`)
        .join("");
      const id = `more-group-${group.title.toLowerCase()}`;
      return `<div class="nav-menu__group">
        <div class="nav-menu__heading" id="${id}">${escapeHtml(group.title)}</div>
        <ul aria-labelledby="${id}">${items}</ul>
      </div>`;
    })
    .filter(Boolean)
    .join("");
  // The compact dropdown (narrow layouts): every destination, the same groups.
  const compactGroups = navGroups
    .map((group) => {
      const items = group.keys
        .map((key) => nav.find((item) => item.key === key))
        .filter(Boolean)
        .map((item) => `<li>${navLink(item, activeKey, base)}</li>`)
        .join("");
      return `<div class="nav-compact__group">
        <div class="nav-compact__heading">${escapeHtml(group.title)}</div>
        <ul>${items}</ul>
      </div>`;
    })
    .join("");
  // Whether the active page is only reachable through the overflow menu, so
  // the More control can carry a current indicator too.
  const overflowActive =
    activeKey !== "" && !navPrimary.includes(activeKey);
  const moreCurrent = overflowActive ? ' data-has-current="true"' : "";

  return `<header class="app-bar">
  <div class="container app-bar__inner">
    <a class="brand" href="${url("", base)}" aria-label="${site.name} home">
      <span class="brand__mark" aria-hidden="true">λ</span>
      <span class="brand__name">${site.name}</span>
      <span class="brand__tag">${escapeHtml(site.releaseVersion)}</span>
    </a>

    <nav class="nav" id="primary-nav" aria-label="Primary">
      <div class="nav__inline">
        ${inline}
      </div>
      <div class="nav-menu" data-nav-menu${moreCurrent}>
        <button class="nav-menu__trigger" type="button" aria-expanded="false" aria-controls="more-menu" aria-haspopup="true" data-nav-menu-trigger>
          More <span class="nav-menu__caret" aria-hidden="true">${icon("chevron-down")}</span>
        </button>
        <div class="nav-menu__popup" id="more-menu" hidden>
          ${overflowGroups}
        </div>
      </div>
    </nav>

    <div class="app-bar__utils">
      <a class="icon-btn" href="${site.repository}" target="_blank" rel="noopener" aria-label="Aura on GitHub" title="Aura on GitHub">${icon("github")}</a>
      <button class="icon-btn" type="button" data-theme-toggle aria-label="Switch color theme">
        <span data-theme-icon-light>${icon("sun")}</span>
        <span data-theme-icon-dark hidden>${icon("moon")}</span>
      </button>
      <button class="nav-toggle" type="button" aria-expanded="false" aria-controls="compact-nav" aria-label="Open navigation menu" data-nav-toggle>
        <span data-nav-icon-open>${icon("menu")}</span>
        <span data-nav-icon-close hidden>${icon("close")}</span>
      </button>
    </div>
  </div>

  <nav class="nav-compact" id="compact-nav" aria-label="Navigation" data-nav-compact hidden>
    <div class="container">
      ${compactGroups}
    </div>
  </nav>
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
  // A closed release line has no development version; render "Stable X" alone
  // rather than "development null" or a fabricated successor.
  const versionLabel = site.developmentVersion
    ? `Stable <strong>${escapeHtml(site.currentRelease)}</strong> · development <strong>${escapeHtml(site.developmentVersion)}</strong>`
    : `Stable <strong>${escapeHtml(site.currentRelease)}</strong>`;
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
      <span class="footer-version">${versionLabel}</span>
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
  const {
    title,
    description,
    path,
    body,
    activeKey,
    withContainer = true,
    structuredData,
    extraHead,
  } = page;
  const main = withContainer ? `<div class="container">${body}</div>` : body;
  return `<!doctype html>
<html lang="en">
<head>
${head({ title, description, path, base, structuredData, extraHead })}
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
