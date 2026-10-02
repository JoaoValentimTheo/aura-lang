import { site } from "../site.config.mjs";
import { url, icon, pageHead, chip, dataTable, callout } from "../lib/components.mjs";

export const releasesPage = {
  key: "releases",
  title: "Releases",
  path: "releases/",
  activeKey: "releases",
  description: `Aura releases: the current ${site.currentRelease} and the previous ${site.previousRelease}, plus the historical releases.`,
  async render(base) {
    return `${pageHead({
      eyebrow: "Releases",
      title: "Aura releases",
      lede: `Aura ${site.currentRelease} is the current public release: builtin-name reservation, a unified type-nesting limit, filesystem-backed module acquisition for the CLI, and the multi-file Playground.`,
    })}

<section class="section section--tight">
  <div class="container">
    <div class="card card--elevated">
      <div class="example-card__meta">
        ${chip("Published", "success")}
        ${chip("0.2.1")}
        ${chip("Latest")}
      </div>
      <h2 style="margin-top:var(--space-3)">Aura 0.2.1</h2>
      <p>The current release (published 2026-10-01). It builds on the completed
      Aura Core and adds module acquisition from the filesystem, multi-file
      projects in the Playground, and two language-contract tightenings.</p>
      <ul>
        <li>Filesystem-backed module acquisition: sibling <code>math.aura</code> and directory <code>pkg/mod.aura</code> sources become logical modules</li>
        <li>Multi-file Playground projects over the additive Host ABI 1 <code>aura_project_*</code> transport</li>
        <li>Builtin names reserved as user value bindings (<code>E1009</code>)</li>
        <li>Structural type nesting and alias chains bounded at 256 on every substrate (<code>E1015</code>, ADR-0004)</li>
        <li>O(n) string lexing, exact-type Python dict keys, and red-team robustness fixes</li>
        <li>Host ABI 1 and Playground API 1 unchanged; Native/WASM parity retained</li>
      </ul>
      <div class="hero__actions">
        <a class="btn btn--filled" href="${site.releases}" target="_blank" rel="noopener">${icon("external")} Release downloads</a>
        <a class="btn btn--outlined" href="${url("docs/migration-0-2-1/", base)}">Migration guide</a>
        <a class="btn btn--text" href="${site.repository}" target="_blank" rel="noopener">${icon("github")} Source</a>
      </div>
    </div>

    <div class="card" style="margin-top:var(--space-6)">
      <div class="example-card__meta">
        ${chip("Previous")}
        ${chip("0.2.0")}
      </div>
      <h2 style="margin-top:var(--space-3)">Aura 0.2.0</h2>
      <p>The Core-completion release. It ships the completed Aura Core language —
      generic maps keyed by <code>string</code>, <code>int</code>, or
      <code>bool</code>, <code>map.items()</code>, list and map comprehensions,
      the decided separator/numeric/f-string rules, and real in-source module
      visibility. It remains frozen and selectable.</p>
      <ul>
        <li>Generic map keys <code>{K: V}</code> with key-capability checking and ordered keys</li>
        <li><code>map.items()</code> and honest collection type checking</li>
        <li>List and map comprehensions with one generator and an optional filter</li>
        <li>Core syntax rules: real statement separators, numeric underscores, f-string braces</li>
        <li>Completed in-source modules: <code>pub</code>, <code>pub use</code>, aliases, per-module tags</li>
        <li>CLI/REPL and Playground completion; Native/WASM parity</li>
      </ul>
    </div>

    <div class="card" style="margin-top:var(--space-6)">
      <div class="example-card__meta">
        ${chip("Historical")}
        ${chip("0.0.2")}
      </div>
      <h2 style="margin-top:var(--space-3)">Aura 0.0.2</h2>
      <p>The infrastructure release. It shipped the frozen 0.0.1 language
      semantics (<strong>language version 0.0.1</strong>) and added portable
      execution, a versioned Playground, and the official website. Its runtime
      artifact is frozen and never overwritten.</p>
      <ul>
        <li>WebAssembly runtime built from the same interpreter</li>
        <li>An explicit host boundary; native and browser hosts behind one contract</li>
        <li>A versioned, immutable Playground with hashed runtime artifacts</li>
        <li>The official website on <a href="${site.origin}">GitHub Pages</a></li>
        <li>Cross-platform release infrastructure with checksummed artifacts</li>
      </ul>
    </div>

    <div class="card" style="margin-top:var(--space-6)">
      <div class="example-card__meta">
        ${chip("Historical")}
        ${chip("0.0.1")}
      </div>
      <h2 style="margin-top:var(--space-3)">Aura 0.0.1</h2>
      <p>The first usable public release: the language core, the runtime, the
      standard library (including scripting I/O), the CLI, the REPL, and the
      optional Python bridge.</p>
      <ul>
        <li>Lexer, parser, AST, conservative checker, tree-walking interpreter</li>
        <li>Core stdlib plus <code>json</code>, <code>regex</code>, and <code>time</code></li>
        <li>Scripting I/O and command-line arguments</li>
        <li>Stable <code>E####</code> diagnostics</li>
      </ul>
      <p class="muted">0.0.1 predates the WebAssembly substrate and has no
      browser runtime; the Playground lists it as unavailable rather than
      fabricating one.</p>
    </div>
  </div>
</section>

<section class="section section--alt">
  <div class="container">
    <div class="section__head">
      <span class="section__eyebrow">Runtime artifacts</span>
      <h2>Versioned, immutable runtimes</h2>
      <p class="section__lede">The Playground executes a specific artifact per
      release. Each entry records the release, the language version, the runtime
      version, the Host ABI version, the artifact, and a SHA-256 hash.</p>
    </div>
    ${dataTable(
      ["Release", "Language", "Browser runtime", "Status"],
      [
        ["0.0.1", "0.0.1", "none", chip("Historical (native only)")],
        ["0.0.2", "0.0.1", "WebAssembly, ABI 1", chip("Historical")],
        ["0.2.0", "0.2.0", "WebAssembly, ABI 1", chip("Previous")],
        ["0.2.1", "0.2.1", "WebAssembly, ABI 1", chip("Current", "success")],
      ],
    )}
    <p class="muted">Immutable historical artifacts are never silently
    overwritten; a change requires a new version. See the
    <a href="${url("docs/runtime-doc/", base)}">runtime reference</a>.</p>
    ${callout(
      "note",
      "<p>The <a href='" +
        url("playground/", base) +
        "'>Playground</a> executes the exact immutable artifact named by the manifest for the selected release.</p>",
    )}
  </div>
</section>

<section class="section">
  <div class="container">
    <div class="section__head">
      <span class="section__eyebrow">Install</span>
      <h2>Build from source today</h2>
    </div>
    <pre class="code-block" style="padding:var(--space-4)"><code>git clone ${site.repository}.git
cd aura-lang
git checkout v${site.releaseVersion}
cargo build --release --no-default-features --features cli,repl,json,regex,time
./target/release/aura version   # aura ${site.releaseVersion}</code></pre>
    <a class="eyebrow-link" href="${url("docs/install/", base)}">Installation guide ${icon("arrow")}</a>
  </div>
</section>`;
  },
};
