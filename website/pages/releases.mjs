import { site } from "../site.config.mjs";
import { url, icon, pageHead, chip, dataTable, callout } from "../lib/components.mjs";

export const releasesPage = {
  key: "releases",
  title: "Releases",
  path: "releases/",
  activeKey: "releases",
  description:
    "Aura releases: the published 0.0.1 and the 0.0.2 development line.",
  async render(base) {
    return `${pageHead({
      eyebrow: "Releases",
      title: "Aura releases",
      lede: "Aura 0.0.1 is the first usable public release. 0.0.2 is in development and adds the website, WebAssembly runtime, and versioned Playground.",
    })}

<section class="section section--tight">
  <div class="container">
    <div class="card card--elevated">
      <div class="example-card__meta">
        ${chip("Published", "success")}
        ${chip("0.0.1")}
        <span class="muted">2026</span>
      </div>
      <h2 style="margin-top:var(--space-3)">Aura 0.0.1</h2>
      <p>The first usable public release. It includes the language core, the
      runtime, the standard library (including scripting I/O), the CLI, the REPL,
      and the optional Python bridge, covered by an executable test suite with CI
      on Linux, macOS, and Windows, with and without CPython.</p>
      <ul>
        <li>Lexer, parser, AST, conservative checker, tree-walking interpreter</li>
        <li>Core stdlib plus <code>json</code>, <code>regex</code>, and <code>time</code></li>
        <li>Scripting I/O and command-line arguments</li>
        <li>Stable <code>E####</code> diagnostics</li>
        <li>Optional Python bridge behind the <code>py</code> feature</li>
      </ul>
      <div class="hero__actions">
        <a class="btn btn--filled" href="${site.releases}" target="_blank" rel="noopener">${icon("external")} Release downloads</a>
        <a class="btn btn--outlined" href="${url("docs/", base)}">Documentation</a>
        <a class="btn btn--text" href="${site.repository}" target="_blank" rel="noopener">${icon("github")} Source</a>
      </div>
    </div>

    <div class="card" style="margin-top:var(--space-6)">
      <div class="example-card__meta">
        ${chip("In development", "planned")}
        ${chip("0.0.2")}
      </div>
      <h2 style="margin-top:var(--space-3)">Aura 0.0.2 <span class="muted">(unreleased)</span></h2>
      <p>0.0.2 is not released yet. It is the infrastructure release: the
      official website, GitHub Pages deployment at <code>aura.lang.dev</code>,
      the WebAssembly runtime, and the versioned Playground.</p>
      <ul>
        <li>WebAssembly runtime built from the same interpreter</li>
        <li>Versioned, immutable runtime artifacts with recorded hashes</li>
        <li>The official website and Playground</li>
      </ul>
      ${callout(
        "note",
        "<p>The Playground already offers an <strong>0.0.2 runtime</strong> artifact. That is a development artifact for experimentation; it does not make 0.0.2 a published release.</p>",
      )}
    </div>
  </div>
</section>

<section class="section section--alt">
  <div class="container">
    <div class="section__head">
      <span class="section__eyebrow">Runtime artifacts</span>
      <h2>Versioned, immutable runtimes</h2>
      <p class="section__lede">The Playground executes a specific artifact per
      version. Each entry records an artifact and a SHA-256 hash.</p>
    </div>
    ${dataTable(
      ["Version", "Language", "Browser runtime", "Status"],
      [
        ["0.0.1", "0.0.1", "none", chip("Published (native only)", "success")],
        ["0.0.2", "0.0.1", "WebAssembly, ABI 1", chip("In development", "planned")],
      ],
    )}
    <p class="muted">0.0.1 predates the WebAssembly substrate and therefore has
    no browser runtime; the Playground lists it as unavailable rather than
    fabricating one. See the <a href="${url("docs/runtime-doc/", base)}">runtime
    reference</a> for the artifact model.</p>
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
cargo build --release --no-default-features --features cli,repl,json,regex,time
./target/release/aura version</code></pre>
    <a class="eyebrow-link" href="${url("docs/install/", base)}">Installation guide ${icon("arrow")}</a>
  </div>
</section>`;
  },
};
