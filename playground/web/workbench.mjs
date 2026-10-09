// The Playground workbench markup — the single source of truth for the
// Playground UI, shared by both entry points:
//
//   * the integrated website page (`/playground/`, generated at build time by
//     `website/pages/playground.mjs`, which imports `workbenchMarkup` and
//     embeds it as static HTML), and
//   * the standalone Playground (`playground/index.html`), whose `app.js`
//     injects this same markup into an empty `<main id="workbench">` before it
//     wires the controller.
//
// Because there is exactly one markup string, the two shells cannot drift in
// element ids, classes, ARIA wiring, or control set. `tests` assert that the
// static website markup equals `workbenchMarkup()`.
//
// This module is a pure string function: no DOM, no Node APIs. It is valid in
// Node (for the static build) and in the browser (for the standalone page).

/**
 * The workbench HTML.
 *
 * Element ids are the controller contract (`playground/web/app.js`); they are
 * identical to what the standalone Playground has always used, so nothing that
 * consumes the DOM has to change.
 *
 * @returns {string} the workbench markup (inside `<main class="workbench">`)
 */
export function workbenchMarkup() {
  return `<aside class="wb-explorer" aria-label="Explorer">
  <h2 class="wb-pane__title">Explorer</h2>
  <div class="wb-explorer__group" aria-label="Examples">
    <div class="wb-explorer__label">Examples</div>
    <ul id="examples" class="explorer__list"></ul>
  </div>
</aside>

<section class="wb-editor" aria-label="Editor">
  <div class="wb-editor__topbar">
    <div id="file-tabs" class="file-tabs" role="tablist" aria-label="Aura source files"></div>
    <div class="file-actions">
      <button id="file-add" class="wb-btn wb-btn--ghost" type="button" title="Add a source file">Add file</button>
      <button id="file-rename" class="wb-btn wb-btn--ghost" type="button" title="Rename the active file">Rename</button>
      <button id="file-entry" class="wb-btn wb-btn--ghost" type="button" title="Make the active file the entry point">Set entry</button>
      <button id="file-delete" class="wb-btn wb-btn--ghost" type="button" title="Delete the active file">Delete</button>
      <button id="project-reset" class="wb-btn wb-btn--ghost" type="button" title="Reset the project">Reset</button>
    </div>
  </div>
  <div id="project-note" class="project-note" hidden></div>

  <div class="wb-editor__topbar">
    <div class="wb-versions">
      <label for="version">Runtime</label>
      <select id="version" aria-label="Aura runtime version"></select>
    </div>
    <div class="wb-run">
      <button id="search-toggle" class="wb-btn wb-btn--ghost" type="button" title="Find (Ctrl/Cmd + F)">Find</button>
      <button id="run" class="wb-btn wb-btn--primary" type="button" title="Ctrl/Cmd + Enter">Run</button>
      <button id="stop" class="wb-btn wb-btn--ghost" type="button" disabled>Stop</button>
    </div>
  </div>

  <div id="search" class="search" role="search" hidden>
    <input id="search-input" class="search__input" type="search" placeholder="Find in buffer" aria-label="Find in buffer" />
    <span id="search-count" class="search__count" role="status" aria-live="polite"></span>
    <button id="search-prev" class="wb-btn wb-btn--ghost" type="button" aria-label="Previous match">Prev</button>
    <button id="search-next" class="wb-btn wb-btn--ghost" type="button" aria-label="Next match">Next</button>
    <label class="search__case"><input id="search-case" type="checkbox" /> Match case</label>
    <button id="search-close" class="wb-btn wb-btn--ghost" type="button" aria-label="Close search">Close</button>
  </div>

  <div id="editor" class="editor">
    <pre id="gutter" class="editor__gutter" aria-hidden="true"></pre>
    <div class="editor__scroll">
      <pre id="highlight" class="editor__highlight" aria-hidden="true"></pre>
      <textarea
        id="source"
        class="editor__input"
        spellcheck="false"
        autocomplete="off"
        autocapitalize="off"
        aria-label="Aura source"
        aria-controls="completion"
      ></textarea>
      <ul id="completion" class="completion" role="listbox" aria-label="Suggestions" hidden></ul>
    </div>
  </div>

  <div class="editor-inputs">
    <div class="field">
      <label for="args">Arguments (one per line)</label>
      <textarea id="args" spellcheck="false" aria-label="Program arguments"></textarea>
    </div>
    <div class="field">
      <label for="stdin">Standard input</label>
      <textarea id="stdin" spellcheck="false" aria-label="Standard input"></textarea>
    </div>
  </div>
</section>

<section class="wb-output" aria-label="Output">
  <div class="wb-output__tabs" role="tablist" aria-label="Output views">
    <button id="tab-output" class="tab" role="tab" type="button" aria-selected="true" aria-controls="panel-output">Output</button>
    <button id="tab-problems" class="tab" role="tab" type="button" aria-selected="false" aria-controls="panel-problems">Problems<span id="problems-count" class="tab__count"></span></button>
  </div>
  <div class="wb-output__bar">
    <span class="wb-pane__title">Output</span>
    <span id="status" class="status" role="status" aria-live="polite">Ready</span>
  </div>
  <div id="panel-output" class="panel" role="tabpanel" aria-labelledby="tab-output">
    <pre id="stdout" class="stdout" role="region" aria-label="Standard output"></pre>
    <p id="output-note" class="output-note" role="status" hidden></p>
  </div>
  <div id="panel-problems" class="panel" role="tabpanel" aria-labelledby="tab-problems" hidden>
    <ul id="diagnostics" class="diagnostics" aria-label="Problems"></ul>
  </div>
  <div id="runtime-note" class="runtime-note" hidden></div>
</section>`;
}
