import { docGroups } from "../content/docs.mjs";
import { url, escapeHtml, pageHead } from "../lib/components.mjs";

export const docsIndexPage = {
  key: "docs",
  title: "Documentation",
  path: "docs/",
  activeKey: "docs",
  description:
    "Aura documentation: getting started, the language guide, reference material, and tooling.",
  async render(base) {
    // Documentation groups, laid out in a grid. "Semantic Tooling" (AIS and the
    // MCP adapter) is surfaced as a highlight card so it is discoverable to
    // tooling developers without adding another global navigation destination.
    const groups = docGroups
      .map((g) => {
        const highlight = g.title === "Semantic Tooling";
        return `<div class="card card--elevated${highlight ? " docs-index__highlight" : ""}">
  <h3>${escapeHtml(g.title)}</h3>
  ${highlight ? '<p class="muted">The compiler\'s semantic interface for developer and AI tooling.</p>' : ""}
  <ul>
    ${g.items
      .map(
        (it) =>
          `<li><a href="${url(`docs/${it.slug}/`, base)}">${escapeHtml(it.title)}</a></li>`,
      )
      .join("")}
  </ul>
</div>`;
      })
      .join("");
    return `${pageHead({
      eyebrow: "Documentation",
      title: "Aura documentation",
      lede: "A guided path from your first program to the language reference, the runtime architecture, and the semantic tooling interface.",
    })}
<section class="section">
  <div class="container">
    <div class="grid grid--3">${groups}</div>
  </div>
</section>`;
  },
};
