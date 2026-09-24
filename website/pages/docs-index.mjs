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
    const groups = docGroups
      .map(
        (g) => `<div class="card card--elevated">
  <h3>${escapeHtml(g.title)}</h3>
  <ul>
    ${g.items
      .map(
        (it) =>
          `<li><a href="${url(`docs/${it.slug}/`, base)}">${escapeHtml(it.title)}</a></li>`,
      )
      .join("")}
  </ul>
</div>`,
      )
      .join("");
    return `${pageHead({
      eyebrow: "Documentation",
      title: "Aura documentation",
      lede: "A guided path from your first program to the language reference and the runtime architecture.",
    })}
<section class="section">
  <div class="container">
    <div class="grid grid--3">${groups}</div>
  </div>
</section>`;
  },
};
