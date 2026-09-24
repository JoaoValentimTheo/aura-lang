import { site } from "../site.config.mjs";
import { url, icon, pageHead, chip, dataTable, callout } from "../lib/components.mjs";

export const releasesPage = {
  key: "releases",
  title: "Releases",
  path: "releases/",
  activeKey: "releases",
  description:
    "Aura releases: the published 0.0.2 and the historical 0.0.1.",
  async render(base) {
    return `${pageHead({
      eyebrow: "Releases",
      title: "Aura releases",
      lede: "Aura 0.0.2 is the current public release: it adds the WebAssembly runtime, the host boundary, the versioned Playground, and this website, while keeping the frozen 0.0.1 language semantics.",
    })}

<section class="section section--tight">
  <div class="container">
    <div class="card card--elevated">
      <div class="example-card__meta">
        ${chip("Published", "success")}
        ${chip("0.0.2")}
        ${chip("Latest")}
      </div>
      <h2 style="margin-top:var(--space-3)">Aura 0.0.2</h2>
      <p>The infrastructure release. It ships the same frozen language semantics
      as 0.0.1 (<strong>language version 0.0.1</strong>) and adds portable
      execution, a versioned Playground, and the official website.</p>
      <ul>
        <li>WebAssembly runtime built from the same interpreter (<code>wasm32-unknown-unknown</code>)</li>
        <li>An explicit host boundary; native and browser hosts behind one contract</li>
        <li>A versioned, immutable Playground with hashed runtime artifacts</li>
        <li>The official website at <a href="${site.origin}">aura.lang.dev</a></li>
        <li>Cross-platform release infrastructure with checksummed artifacts</li>
      </ul>
      <div class="hero__actions">
        <a class="btn btn--filled" href="${site.releases}" target="_blank" rel="noopener">${icon("external")} Release downloads</a>
        <a class="btn btn--outlined" href="${url("docs/", base)}">Documentation</a>
        <a class="btn btn--text" href="${site.repository}" target="_blank" rel="noopener">${icon("github")} Source</a>
      </div>
    </div>

    <div class="card" style="margin-top:var(--space-6)">
      <div class="example-card__meta">
        ${chip("Published", "success")}
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
        ["0.0.1", "0.0.1", "none", chip("Published (native only)", "success")],
        ["0.0.2", "0.0.1", "WebAssembly, ABI 1", chip("Published", "success")],
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
