import { url, icon, callout } from "../lib/components.mjs";
import { site } from "../site.config.mjs";
import { workbenchMarkup } from "../../playground/web/workbench.mjs";

// The Playground page.
//
// It reuses the validated Playground engine unchanged: the same
// `playground/web/app.js` controller, `playground/web/worker.js` Worker, and
// `playground/web/runtime.mjs` loader, and the same `workbenchMarkup()` that
// the standalone Playground injects. Only one markup string exists, so the two
// entry points cannot drift.
//
// The build copies `playground/web` and the immutable `playground/runtimes`
// artifacts under `/playground/`, so the controller's relative paths
// (`./web/worker.js`, `./runtimes/manifest.json`) resolve exactly as they do
// for the standalone Playground.
export const playgroundPage = {
  key: "playground",
  title: "Playground",
  path: "playground/",
  activeKey: "playground",
  description:
    "The Aura Playground: run the real Aura WebAssembly runtime in your browser, isolated in a Web Worker, with versioned, immutable runtime artifacts.",
  withContainer: false,
  // The Playground workbench stylesheet (shared with the standalone Playground)
  // is layered on top of the site's Aurea tokens.
  extraHead: (base) => `<link rel="stylesheet" href="${url("playground/web/workbench.css", base)}" />`,
  async render(base) {
    return `<section class="section section--tight" style="padding-bottom:0">
  <div class="container">
    <div class="playground-head">
      <div>
        <span class="section__eyebrow">Playground</span>
        <h1>Aura Playground</h1>
        <p class="muted">The real Aura runtime, compiled to WebAssembly and isolated in a Web Worker.
        <strong>0.3.1</strong> Keystone is the default; earlier releases stay selectable. Nothing leaves your browser.</p>
      </div>
      <div class="playground-head__links">
        <a class="btn btn--outlined btn--small" href="${url("docs/playground-doc/", base)}">Playground docs</a>
        <a class="btn btn--text btn--small" href="${url("docs/runtime-doc/", base)}">Runtime &amp; host ${icon("arrow")}</a>
      </div>
    </div>
    ${callout(
      "note",
      "<p><strong>Capabilities:</strong> the browser host has no filesystem, clock, sleep, or network, so those report <code>E5002</code>. Standard input, arguments, and <strong>virtual multi-file projects</strong> work — files live in your browser session, not on disk. The browser runtime remains <strong>0.3.1</strong>.</p>",
    )}
  </div>
</section>

<div class="container">
  <div class="workbench" data-playground>
${workbenchMarkup()}
  </div>
</div>

<section class="section section--tight">
  <div class="container">
    <details class="about-disclosure">
      <summary>About this Playground</summary>
      <div class="prose">
        <p>This page uses the <strong>real Aura interpreter</strong> compiled to
        WebAssembly. There is no JavaScript reimplementation of Aura: the
        controller sends your source to a Worker, the Worker loads the selected
        immutable runtime artifact, verifies its SHA-256 against the manifest,
        and the runtime returns a structured result.</p>
        <p>Execution is isolated from the page in a Web Worker, so a program that
        loops forever cannot freeze the UI. <strong>Stop</strong> terminates the
        Worker — the primary hard-cancellation mechanism. The runtime module
        imports nothing and can reach no DOM, network, or storage.</p>
        <p>The <strong>Runtime</strong> selector chooses a real, immutable
        artifact with a recorded hash, not a label. The released
        <code>${site.releaseVersion}</code> (the default) exercises the current
        language; the previous <code>0.2.1</code>, <code>0.2.0</code>, and
        historical <code>0.0.2</code> releases remain frozen and selectable.</p>
        <p>
          <a class="eyebrow-link" href="${url("docs/playground-doc/", base)}">Playground documentation ${icon("arrow")}</a>
          <a class="eyebrow-link" href="${url("docs/runtime-doc/", base)}" style="margin-left:var(--au-space-4)">Runtime &amp; host ${icon("arrow")}</a>
        </p>
      </div>
    </details>
  </div>
</section>

<script type="module" src="./web/app.js"></script>
`;
  },
};
