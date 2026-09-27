import { url, pageHead, callout } from "../lib/components.mjs";
import { site } from "../site.config.mjs";

// The Playground page.
//
// It reuses the validated Playground engine unchanged: the same
// `playground/web/app.js` controller, `playground/web/worker.js` Worker, and
// `playground/web/runtime.mjs` loader. The build copies those assets and the
// immutable `playground/runtimes` artifacts under `/playground/`, so the
// controller's relative paths (`./web/worker.js`, `./runtimes/manifest.json`)
// resolve exactly as they do for the standalone Playground.
//
// This page only supplies the *shell*: the site chrome, headings, and the
// same element ids the controller expects. It contains no execution logic.
export const playgroundPage = {
  key: "playground",
  title: "Playground",
  path: "playground/",
  activeKey: "playground",
  description:
    "The Aura Playground: run the real Aura WebAssembly runtime in your browser, isolated in a Web Worker, with versioned, immutable runtime artifacts.",
  withContainer: false,
  async render(base) {
    return `${pageHead({
      eyebrow: "Playground",
      title: "Aura Playground",
      lede: "Runs the real Aura runtime compiled to WebAssembly, isolated in a Web Worker. No account, no server execution, no code leaves your browser.",
    })}

<section class="section section--tight" style="padding-bottom:0">
  <div class="container">
    ${callout(
      "info",
      "<p>The <strong>Runtime</strong> selector chooses a real, immutable artifact with a recorded hash, not a label. The <strong>development runtime</strong> (<code>0.0.2-dev.11</code>) exercises the current language — struct methods, traits, <code>const</code>, bitwise operators, f-string formatting, and <code>a..b</code> ranges — while the published <code>0.0.2</code> release remains frozen and selectable. Filesystem, clock, and sleep are unavailable here and report <code>E5002</code>; standard input and arguments work.</p>",
    )}
  </div>
</section>

<div class="container pg-workbench" data-playground>
  <aside class="pg-explorer" aria-label="Explorer">
    <div class="pg-pane__head"><h2>Explorer</h2></div>
    <div class="pg-explorer__group" aria-label="Examples">
      <div class="pg-explorer__label">Examples</div>
      <ul id="examples" class="pg-explorer__list"></ul>
    </div>
  </aside>

  <section class="pg-pane pg-pane--editor" aria-label="Editor">
    <div class="pg-editor__tabs">
      <span class="tab tab--active">main.aura</span>
      <div class="pg-actions">
        <button id="run" class="btn btn--filled btn--small" type="button" title="Ctrl/Cmd + Enter">
          Run
        </button>
        <button id="stop" class="btn btn--outlined btn--small" type="button" disabled>Stop</button>
      </div>
    </div>
    <div class="pg-versions">
      <label for="version">Runtime</label>
      <select id="version" aria-label="Aura runtime version"></select>
    </div>
    <div id="editor" class="pg-editor-wrap">
      <pre id="gutter" class="pg-editor__gutter" aria-hidden="true"></pre>
      <div class="pg-editor__scroll">
        <pre id="highlight" class="pg-editor__highlight" aria-hidden="true"></pre>
        <textarea
          id="source"
          class="pg-editor__input"
          spellcheck="false"
          autocomplete="off"
          autocapitalize="off"
          aria-label="Aura source"
        ></textarea>
      </div>
    </div>
    <div class="pg-inputs">
      <div class="pg-field">
        <label for="args">Arguments (one per line)</label>
        <textarea id="args" spellcheck="false" aria-label="Program arguments"></textarea>
      </div>
      <div class="pg-field">
        <label for="stdin">Standard input</label>
        <textarea id="stdin" spellcheck="false" aria-label="Standard input"></textarea>
      </div>
    </div>
  </section>

  <section class="pg-pane pg-pane--output" aria-label="Output">
    <div class="pg-editor__tabs" role="tablist" aria-label="Output views">
      <button
        id="tab-output"
        class="tab"
        role="tab"
        type="button"
        aria-selected="true"
        aria-controls="panel-output"
      >
        Output
      </button>
      <button
        id="tab-problems"
        class="tab"
        role="tab"
        type="button"
        aria-selected="false"
        aria-controls="panel-problems"
      >
        Problems<span id="problems-count" class="tab__count"></span>
      </button>
    </div>
    <div class="pg-statusbar">
      <span id="status" class="status" role="status" aria-live="polite">idle</span>
    </div>
    <div id="panel-output" class="pg-panel" role="tabpanel" aria-labelledby="tab-output">
      <pre id="stdout" class="pg-stdout" role="region" aria-label="Standard output"></pre>
    </div>
    <div
      id="panel-problems"
      class="pg-panel"
      role="tabpanel"
      aria-labelledby="tab-problems"
      hidden
    >
      <ul id="diagnostics" class="pg-diagnostics" aria-label="Problems"></ul>
    </div>
    <div id="runtime-note" class="pg-note" hidden></div>
  </section>
</div>

<section class="section section--tight">
  <div class="container prose">
    <h2>About this Playground</h2>
    <p>This page uses the <strong>real Aura interpreter</strong> compiled to
    WebAssembly. There is no JavaScript reimplementation of Aura: the controller
    sends your source to a Worker, the Worker loads the selected immutable
    runtime artifact, and the runtime returns a structured result.</p>
    <p>Execution is isolated from the page in a Web Worker, so a program that
    loops forever cannot freeze the UI. <strong>Stop</strong> terminates the
    Worker — the primary hard-cancellation mechanism. The runtime module imports
    nothing and can reach no DOM, network, or storage.</p>
    <p>
      <a class="eyebrow-link" href="${url("docs/playground-doc/", base)}">Playground documentation →</a>
      <a class="eyebrow-link" href="${url("docs/runtime-doc/", base)}" style="margin-left:var(--space-4)">Runtime &amp; host →</a>
    </p>
  </div>
</section>

<script type="module" src="./web/app.js"></script>
`;
  },
};
