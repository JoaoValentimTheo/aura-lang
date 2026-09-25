// Reusable HTML component helpers for the Aura website.
//
// Everything is a pure function returning an HTML string. Content is escaped
// unless it is generated markup, so authoring mistakes cannot inject HTML.

import { highlight, escapeHtml } from "./highlight.mjs";

export { escapeHtml };

/** Build a root-absolute URL under the configured base path. */
export function url(path = "", base = "/") {
  const clean = path.replace(/^\/+/, "");
  const b = base.endsWith("/") ? base : `${base}/`;
  return `${b}${clean}`;
}

/** A few inline SVG icons (no icon-font dependency). */
export function icon(name) {
  const paths = {
    menu: '<path d="M3 6h18M3 12h18M3 18h18" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>',
    close:
      '<path d="M6 6l12 12M18 6L6 18" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>',
    sun: '<circle cx="12" cy="12" r="4" fill="currentColor"/><path d="M12 2v3M12 19v3M2 12h3M19 12h3M4.9 4.9l2.1 2.1M17 17l2.1 2.1M19.1 4.9L17 7M7 17l-2.1 2.1" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>',
    moon: '<path d="M20 14.5A8.5 8.5 0 019.5 4a7 7 0 108.5 10.5z" fill="currentColor"/>',
    github:
      '<path fill="currentColor" d="M12 .5A11.5 11.5 0 00.5 12a11.5 11.5 0 007.9 10.9c.6.1.8-.2.8-.5v-2c-3.2.7-3.9-1.4-3.9-1.4-.5-1.3-1.3-1.7-1.3-1.7-1-.7.1-.7.1-.7 1.1.1 1.7 1.2 1.7 1.2 1 1.7 2.6 1.2 3.3.9.1-.7.4-1.2.7-1.5-2.6-.3-5.3-1.3-5.3-5.7 0-1.3.5-2.3 1.2-3.1-.1-.3-.5-1.5.1-3.1 0 0 1-.3 3.3 1.2a11.5 11.5 0 016 0C17.3 4.6 18.3 5 18.3 5c.6 1.6.2 2.8.1 3.1.8.8 1.2 1.8 1.2 3.1 0 4.4-2.7 5.4-5.3 5.7.4.4.8 1.1.8 2.2v3.3c0 .3.2.6.9.5A11.5 11.5 0 0023.5 12 11.5 11.5 0 0012 .5z"/>',
    arrow: '<path d="M5 12h14M13 6l6 6-6 6" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" fill="none"/>',
    play: '<path d="M8 5v14l11-7z" fill="currentColor"/>',
    external:
      '<path d="M14 3h7v7M21 3l-9 9M19 14v5a2 2 0 01-2 2H5a2 2 0 01-2-2V7a2 2 0 012-2h5" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" fill="none"/>',
    copy: '<path d="M9 9h10v10H9zM5 15H4a1 1 0 01-1-1V4a1 1 0 011-1h10a1 1 0 011 1v1" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" fill="none"/>',
    check: '<path d="M5 13l4 4L19 7" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" fill="none"/>',
  };
  return `<svg viewBox="0 0 24 24" aria-hidden="true" width="20" height="20">${paths[name] || ""}</svg>`;
}

/**
 * The Playground URL that carries a runnable example with the navigation.
 *
 * The payload lives *in the link itself* (`?source=…&args=…&stdin=…`), never
 * in a per-tab side effect, so the selected example survives every navigation
 * mode: a plain click, a modifier/middle click that opens a new tab, or a
 * click that happens before the shared site script has attached its listener.
 * A link that loses its payload cannot silently fall back to the Playground's
 * default program, because the Playground reads this query on load.
 */
function playgroundHref({ source, args, stdin, base }) {
  const params = new URLSearchParams();
  params.set("source", source);
  if (args && args.length) params.set("args", JSON.stringify(args.map(String)));
  if (stdin) params.set("stdin", stdin);
  return `${url(`playground/?${params.toString()}`, base)}#code`;
}

/**
 * A highlighted Aura code block with a copy button and an optional
 * "Run in Playground" action that hands the source to the Playground page.
 *
 * When `runnable` is set, `args` (array) and `stdin` (string) travel with the
 * source inside the Run link, so a program that reads arguments or standard
 * input arrives in the Playground ready to run.
 */
export function codeBlock({
  source,
  lang = "aura",
  title,
  runnable = false,
  args = null,
  stdin = null,
  base = "/",
}) {
  const code = lang === "aura" ? highlight(source) : escapeHtml(source);
  const head = title || lang;
  const run = runnable
    ? `<a class="btn btn--text btn--small" href="${escapeHtml(
        playgroundHref({ source, args, stdin, base }),
      )}" data-run-example>${icon("play")}<span>Run</span></a>`
    : "";
  // The source is embedded in a data attribute for copy/run without a second
  // fetch. It is attribute-escaped. Arguments and input travel alongside so the
  // Playground receives the exact example, not just its source.
  const data = escapeHtml(source);
  const dataArgs = args && args.length ? escapeHtml(JSON.stringify(args)) : "";
  const dataStdin = stdin ? escapeHtml(stdin) : "";
  return `<div class="code-block">
  <div class="code-block__head">
    <span class="code-block__title">${escapeHtml(head)}</span>
    <div class="code-block__actions">
      ${run}
      <button class="copy-btn" type="button" data-copy data-code="${data}">${icon(
        "copy",
      )}<span>Copy</span></button>
    </div>
  </div>
  <pre tabindex="0" role="region" aria-label="${escapeHtml(
    lang,
  )} code"${dataArgs ? ` data-args="${dataArgs}"` : ""}${
    dataStdin ? ` data-stdin="${dataStdin}"` : ""
  }>${code ? `<code class="language-${escapeHtml(lang)}">${code}</code>` : ""}</pre>
</div>`;
}

/** A callout box. `kind` is "note" | "info" | "warn". */
export function callout(kind, html) {
  const glyph = kind === "warn" ? "!" : kind === "note" ? "i" : "i";
  const cls =
    kind === "warn" ? "callout callout--warn" : kind === "note" ? "callout callout--note" : "callout";
  return `<div class="${cls}"><span class="callout__icon" aria-hidden="true">${glyph}</span><div>${html}</div></div>`;
}

/** A feature row used on landing pages. */
export function feature(iconGlyph, title, body) {
  return `<div class="feature">
  <div class="feature__icon" aria-hidden="true">${iconGlyph}</div>
  <div>
    <h3>${escapeHtml(title)}</h3>
    <p>${body}</p>
  </div>
</div>`;
}

/** A status chip. */
export function chip(text, kind = "") {
  const cls = kind ? `chip chip--${kind}` : "chip";
  const dot = kind === "success" || kind === "planned" ? '<span class="chip__dot"></span>' : "";
  return `<span class="${cls}">${dot}${escapeHtml(text)}</span>`;
}

/** The interior-page header block. */
export function pageHead({ eyebrow, title, lede }) {
  return `<header class="page-head"><div class="container">
  ${eyebrow ? `<span class="page-head__eyebrow">${escapeHtml(eyebrow)}</span>` : ""}
  <h1>${escapeHtml(title)}</h1>
  ${lede ? `<p>${lede}</p>` : ""}
</div></header>`;
}

/** Breadcrumbs. `items` is an array of `{ label, href? }`. */
export function breadcrumbs(items, base = "/") {
  const parts = items.map((it, i) => {
    const isLast = i === items.length - 1;
    if (isLast || !it.href) {
      return `<span aria-current="page">${escapeHtml(it.label)}</span>`;
    }
    return `<a href="${url(it.href, base)}">${escapeHtml(it.label)}</a><span aria-hidden="true">/</span>`;
  });
  return `<nav class="breadcrumbs container" aria-label="Breadcrumb">${parts.join("")}</nav>`;
}

/** A numbered vertical pipeline diagram. `stages` is `{ title, body }[]`. */
export function pipeline(stages) {
  const rows = [];
  stages.forEach((s, i) => {
    rows.push(`<div class="pipeline__stage">
  <div class="pipeline__num" aria-hidden="true">${i + 1}</div>
  <div class="pipeline__body"><h3>${escapeHtml(s.title)}</h3><p>${s.body}</p></div>
</div>`);
    if (i < stages.length - 1) {
      rows.push('<div class="arrow-down" aria-hidden="true">↓</div>');
    }
  });
  return `<div class="pipeline">${rows.join("")}</div>`;
}

/** A status table from `[label, state, kind]` rows. */
export function statusList(rows) {
  const items = rows
    .map(
      ([label, state, kind]) =>
        `<li><span class="status-list__label">${escapeHtml(label)}</span>${chip(state, kind)}</li>`,
    )
    .join("");
  return `<ul class="status-list">${items}</ul>`;
}

/** A data table. `head` is a string array, `rows` an array of string arrays. */
export function dataTable(head, rows) {
  const thead = `<thead><tr>${head.map((h) => `<th>${escapeHtml(h)}</th>`).join("")}</tr></thead>`;
  const tbody = `<tbody>${rows
    .map(
      (r) =>
        `<tr>${r.map((c) => `<td>${c /* already-escaped or safe markup */}</td>`).join("")}</tr>`,
    )
    .join("")}</tbody>`;
  // tabindex/role make the horizontally scrollable wrapper keyboard-accessible
  // without changing its visual behaviour.
  return `<div class="table-wrap" tabindex="0" role="region" aria-label="Table"><table class="data">${thead}${tbody}</table></div>`;
}

/** A link card. */
export function linkCard({ title, body, href, base = "/", cta = "Open" }) {
  return `<a class="card card--elevated" href="${url(href, base)}" style="display:block;text-decoration:none">
  <h3>${escapeHtml(title)}</h3>
  <p>${body}</p>
  <span class="eyebrow-link">${escapeHtml(cta)} ${icon("arrow")}</span>
</a>`;
}
