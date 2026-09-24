import { examples } from "../examples/examples.mjs";
import { url, icon, escapeHtml, codeBlock, pageHead } from "../lib/components.mjs";

const LEVELS = [
  { id: "basics", label: "Basics" },
  { id: "intermediate", label: "Intermediate" },
  { id: "advanced", label: "Advanced" },
];

export const examplesPage = {
  key: "examples",
  title: "Examples",
  path: "examples/",
  activeKey: "examples",
  description:
    "Runnable Aura examples, each validated against the real WebAssembly runtime.",
  async render(base) {
    const grouped = LEVELS.map((level) => {
      const items = examples.filter((e) => e.level === level.id);
      const cards = items
        .map(
          (e) => `<a class="card card--elevated" href="#${e.id}" style="display:block;text-decoration:none">
  <div class="example-card__meta">${e.tags
    .map((t) => `<span class="tag">${escapeHtml(t)}</span>`)
    .join("")}</div>
  <h3>${escapeHtml(e.title)}</h3>
  <p>${escapeHtml(e.summary)}</p>
  <span class="eyebrow-link">View ${icon("arrow")}</span>
</a>`,
        )
        .join("");
      return `<section class="section section--tight">
  <div class="section__head">
    <span class="section__eyebrow">${escapeHtml(level.label)}</span>
    <h2>${escapeHtml(level.label)} examples</h2>
  </div>
  <div class="grid grid--3">${cards}</div>
</section>`;
    }).join("");

    const details = examples
      .map(
        (e) => `<section class="section section--tight" id="${e.id}">
  <div class="section__head">
    <span class="section__eyebrow">${escapeHtml(e.level)}</span>
    <h2>${escapeHtml(e.title)}</h2>
    <p class="section__lede">${escapeHtml(e.summary)}</p>
  </div>
  ${codeBlock({ source: e.source, title: `${e.id}.aura`, runnable: true, base })}
  ${e.note ? `<p>${e.note}</p>` : ""}
  ${e.output ? `<p class="muted">Expected output: <code>${escapeHtml(e.output.trimEnd())}</code></p>` : ""}
</section>`,
      )
      .join("");

    return `${pageHead({
      eyebrow: "Examples",
      title: "Aura by example",
      lede: "Every example below is validated against the real Aura WebAssembly runtime during the website build. Copy one, or run it in the Playground.",
    })}
<section class="section section--tight">
  <div class="container">
    <div class="hero__actions">
      <a class="btn btn--filled" href="${url("playground/", base)}">${icon("play")} Open the Playground</a>
      <a class="btn btn--outlined" href="${url("learn/", base)}">Learn Aura</a>
    </div>
  </div>
</section>
<div class="container">${grouped}</div>
<div class="container" style="margin-top:var(--space-8)">
  <div class="divider"></div>
  <h2>All examples in detail</h2>
  ${details}
</div>`;
  },
};
