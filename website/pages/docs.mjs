// Documentation page factory.
//
// Reads a Markdown file from `website/content/`, renders it, and wraps it in a
// docs layout (grouped sidebar, breadcrumbs, on-this-page table of contents).

import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { docGroups, allDocs, docsVersion, docsLanguageVersion } from "../content/docs.mjs";
import { renderMarkdown } from "../lib/markdown.mjs";
import { url, escapeHtml } from "../lib/components.mjs";
import { site } from "../site.config.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const contentDir = resolve(here, "..", "content");

function sidebar(base, activeSlug, activeGroup) {
  return `<aside class="docs-sidebar" aria-label="Documentation">
${docGroups
  .map(
    (group) => `<div class="docs-sidebar__group">
  <div class="docs-sidebar__title">${escapeHtml(group.title)}</div>
  <ul class="docs-sidebar__list">
    ${group.items
      .map((item) => {
        const current = item.slug === activeSlug ? ' aria-current="page"' : "";
        return `<li><a href="${url(`docs/${item.slug}/`, base)}"${current}>${escapeHtml(
          item.title,
        )}</a></li>`;
      })
      .join("")}
  </ul>
</div>`,
  )
  .join("")}
<div class="docs-sidebar__group">
  <div class="docs-sidebar__title">Normative documents</div>
  <ul class="docs-sidebar__list">
    <li><a href="${site.repository}/blob/main/docs/LANGUAGE_SPEC.md" target="_blank" rel="noopener">Language specification</a></li>
    <li><a href="${site.repository}/blob/main/docs/contract.md" target="_blank" rel="noopener">Compatibility contract</a></li>
    <li><a href="${site.repository}/blob/main/docs/grammar.md" target="_blank" rel="noopener">Grammar (source)</a></li>
    <li><a href="${site.repository}/blob/main/docs/errors.md" target="_blank" rel="noopener">Diagnostic codes (source)</a></li>
  </ul>
</div>
</aside>`;
}

function toc(headings) {
  const items = headings.filter((h) => h.level === 2 || h.level === 3);
  if (items.length < 2) return "";
  return `<nav class="toc" aria-label="On this page">
  <div class="toc__title">On this page</div>
  <ul>${items
    .map(
      (h) =>
        `<li><a href="#${h.id}" style="${h.level === 3 ? "padding-left:var(--space-4)" : ""}">${escapeHtml(
          h.text,
        )}</a></li>`,
    )
    .join("")}</ul>
</nav>`;
}

function buildDocPage(doc, markdown, base) {
  const headings = [];
  const bodyHtml = renderMarkdown(markdown, { headings });
  const groupSlug = doc.group.toLowerCase().replace(/\s+/g, "-");
  return {
    title: doc.title,
    description: `${doc.title} — Aura documentation (version ${docsVersion}).`,
    path: `docs/${doc.slug}/`,
    activeKey: "docs",
    withContainer: false,
    async render(b) {
      return `<div class="container">${breadcrumbs(b)}</div>
<div class="container docs-layout">
  ${sidebar(b, doc.slug, groupSlug)}
  <article class="prose">
    ${bodyHtml}
    <div class="divider"></div>
    <p class="muted">Documentation for Aura <strong>${docsVersion}</strong>,
    which implements the language version <strong>${docsLanguageVersion}</strong>.
    Aura is under active development; the language specification linked in the
    sidebar is normative.</p>
  </article>
  ${toc(headings)}
</div>`;
    },
  };

  function breadcrumbs(b) {
    return `<nav class="breadcrumbs" aria-label="Breadcrumb" style="padding-left:0">
  <a href="${url("", b)}">Home</a><span aria-hidden="true">/</span>
  <a href="${url("docs/", b)}">Docs</a><span aria-hidden="true">/</span>
  <span aria-current="page">${escapeHtml(doc.title)}</span>
</nav>`;
  }
}

/** Build every documentation page descriptor. */
export function docPages() {
  return allDocs().map((doc) => {
    const markdown = readFileSync(join(contentDir, doc.file), "utf8");
    return buildDocPage(doc, markdown, "/");
  });
}

export { buildDocPage };
