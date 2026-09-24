import { url, icon } from "../lib/components.mjs";

export const notFoundPage = {
  title: "Page not found",
  async render(base) {
    return `<div class="container center" style="padding:var(--space-20) var(--space-6)">
  <span class="section__eyebrow">404</span>
  <h1 style="font-size:var(--type-display-small)">This page does not exist.</h1>
  <p class="muted">The link may be old, or the page may have moved.</p>
  <div class="hero__actions" style="justify-content:center;margin-top:var(--space-6)">
    <a class="btn btn--filled" href="${url("", base)}">${icon("arrow")} Back to home</a>
    <a class="btn btn--outlined" href="${url("docs/", base)}">Documentation</a>
    <a class="btn btn--text" href="${url("playground/", base)}">Playground</a>
  </div>
</div>`;
  },
};
