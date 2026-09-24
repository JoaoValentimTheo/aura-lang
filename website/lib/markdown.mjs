// A small, dependency-free Markdown renderer for the Aura documentation.
//
// It supports the subset the Aura docs actually use: headings, paragraphs,
// fenced code (with Aura highlighting), inline code, bold/italic, links,
// ordered/unordered lists, blockquotes, horizontal rules, and tables. Input is
// HTML-escaped except for generated markup, so it is injection-safe.

import { highlight, escapeHtml } from "./highlight.mjs";

function slugify(text) {
  return text
    .toLowerCase()
    .replace(/[^\w\s-]/g, "")
    .trim()
    .replace(/\s+/g, "-");
}

function inline(text) {
  let s = escapeHtml(text);
  // Inline code first, so its contents are not further processed.
  const codes = [];
  s = s.replace(/`([^`]+)`/g, (_, code) => {
    codes.push(code);
    return `\u0000CODEC${codes.length - 1}\u0000`;
  });
  // Links [text](href)
  s = s.replace(/\[([^\]]+)\]\(([^)\s]+)\)/g, (_, label, href) => {
    const safe = href.replace(/"/g, "%22");
    const external = /^https?:\/\//.test(safe);
    const attrs = external ? ' target="_blank" rel="noopener"' : "";
    return `<a href="${safe}"${attrs}>${label}</a>`;
  });
  // Bold then italic.
  s = s.replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>");
  s = s.replace(/(^|[^*])\*([^*]+)\*/g, "$1<em>$2</em>");
  // Restore inline code.
  s = s.replace(/\u0000CODEC(\d+)\u0000/g, (_, n) => `<code>${codes[Number(n)]}</code>`);
  return s;
}

/**
 * Render Markdown to HTML. `options.tocCollector`, if provided, receives
 * `{ level, id, text }` for every heading so a table of contents can be built.
 */
export function renderMarkdown(markdown, options = {}) {
  const lines = markdown.replace(/\r\n/g, "\n").split("\n");
  const out = [];
  let i = 0;
  const headings = options.headings || [];

  while (i < lines.length) {
    const line = lines[i];

    // Fenced code.
    const fence = line.match(/^```(\w*)\s*$/);
    if (fence) {
      const lang = fence[1];
      i += 1;
      const body = [];
      while (i < lines.length && !/^```\s*$/.test(lines[i])) {
        body.push(lines[i]);
        i += 1;
      }
      i += 1; // closing fence
      const code = body.join("\n");
      const rendered = lang === "aura" ? highlight(code) : escapeHtml(code);
      out.push(
        `<div class="code-block"><div class="code-block__head"><span class="code-block__title">${escapeHtml(
          lang || "text",
        )}</span><div class="code-block__actions"><button class="copy-btn" type="button" data-copy>Copy</button></div></div><pre tabindex="0" role="region" aria-label="${escapeHtml(
          lang || "text",
        )} code"><code class="language-${escapeHtml(lang)}">${rendered}</code></pre></div>`,
      );
      continue;
    }

    // Horizontal rule.
    if (/^---+\s*$/.test(line)) {
      out.push('<hr class="divider" />');
      i += 1;
      continue;
    }

    // Heading.
    const heading = line.match(/^(#{1,6})\s+(.*)$/);
    if (heading) {
      const level = heading[1].length;
      const text = heading[2].trim();
      const id = slugify(text);
      headings.push({ level, id, text });
      out.push(`<h${level} id="${id}">${inline(text)}</h${level}>`);
      i += 1;
      continue;
    }

    // Table (header line followed by separator).
    if (
      line.includes("|") &&
      i + 1 < lines.length &&
      /^\s*\|?[\s:|-]+\|[\s:|-]*$/.test(lines[i + 1])
    ) {
      const parseRow = (row) =>
        row
          .replace(/^\s*\|/, "")
          .replace(/\|\s*$/, "")
          .split("|")
          .map((c) => c.trim());
      const header = parseRow(line);
      i += 2;
      const rows = [];
      while (i < lines.length && lines[i].includes("|") && lines[i].trim() !== "") {
        rows.push(parseRow(lines[i]));
        i += 1;
      }
      const thead = `<thead><tr>${header
        .map((c) => `<th>${inline(c)}</th>`)
        .join("")}</tr></thead>`;
      const tbody = `<tbody>${rows
        .map(
          (r) =>
            `<tr>${r.map((c) => `<td>${inline(c)}</td>`).join("")}</tr>`,
        )
        .join("")}</tbody>`;
      out.push(
        `<div class="table-wrap" tabindex="0" role="region" aria-label="Table"><table class="data">${thead}${tbody}</table></div>`,
      );
      continue;
    }

    // Blockquote.
    if (/^>\s?/.test(line)) {
      const body = [];
      while (i < lines.length && /^>\s?/.test(lines[i])) {
        body.push(lines[i].replace(/^>\s?/, ""));
        i += 1;
      }
      out.push(
        `<blockquote><p>${inline(body.join(" "))}</p></blockquote>`,
      );
      continue;
    }

    // Unordered list.
    if (/^\s*[-*]\s+/.test(line)) {
      const items = [];
      while (i < lines.length && /^\s*[-*]\s+/.test(lines[i])) {
        items.push(lines[i].replace(/^\s*[-*]\s+/, ""));
        i += 1;
      }
      out.push(
        `<ul>${items.map((it) => `<li>${inline(it)}</li>`).join("")}</ul>`,
      );
      continue;
    }

    // Ordered list.
    if (/^\s*\d+\.\s+/.test(line)) {
      const items = [];
      while (i < lines.length && /^\s*\d+\.\s+/.test(lines[i])) {
        items.push(lines[i].replace(/^\s*\d+\.\s+/, ""));
        i += 1;
      }
      out.push(
        `<ol>${items.map((it) => `<li>${inline(it)}</li>`).join("")}</ol>`,
      );
      continue;
    }

    // Blank line.
    if (line.trim() === "") {
      i += 1;
      continue;
    }

    // Paragraph: gather until a blank line or a block opener.
    const para = [];
    while (
      i < lines.length &&
      lines[i].trim() !== "" &&
      !/^(#{1,6}\s|```|>\s?|\s*[-*]\s|\s*\d+\.\s)/.test(lines[i]) &&
      !/^---+$/.test(lines[i])
    ) {
      para.push(lines[i]);
      i += 1;
    }
    out.push(`<p>${inline(para.join(" "))}</p>`);
  }

  return out.join("\n");
}

export { slugify };
