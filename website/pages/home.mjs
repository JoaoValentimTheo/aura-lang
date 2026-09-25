import { site } from "../site.config.mjs";
import {
  codeBlock,
  feature,
  chip,
  url,
  icon,
  dataTable,
  statusList,
} from "../lib/components.mjs";

const HERO_SOURCE = `fn fib(n) -> int {
    if n < 2 { return n }
    return fib(n - 1) + fib(n - 2)
}

fn main() {
    let name = "Aura"
    let mut total = 0
    for i in range(1, 6) { total = total + i }
    print(f"{name}: 1..5 = {total}, fib(10) = {fib(10)}")
}`;

export const homePage = {
  key: "home",
  title: "Home",
  path: "",
  activeKey: "home",
  description: site.description,
  structuredData: () => ({
    "@context": "https://schema.org",
    "@type": "SoftwareSourceCode",
    name: "Aura",
    description: site.description,
    url: site.origin,
    codeRepository: site.repository,
    license: "https://opensource.org/licenses/MIT",
    programmingLanguage: "Aura",
    version: site.releaseVersion,
  }),
  async render(base) {
    return `
<section class="hero">
  <div class="container hero__grid">
    <div class="hero__copy">
      <span class="hero__eyebrow">Aura ${site.releaseVersion} · released</span>
      <h1>A small language, built to be understood.</h1>
      <p class="hero__lede">
        Aura is a dynamically-typed, expression-oriented scripting language with a
        conservative static checker, a native Rust runtime, and a versioned
        WebAssembly Playground that runs in the browser.
      </p>
      <div class="hero__meta">
        ${chip("One spelling per construct")}
        ${chip("Immutable by default")}
        ${chip("No null — only none")}
        ${chip("Stable E#### diagnostics")}
      </div>
      <div class="hero__actions">
        <a class="btn btn--filled" href="${url("playground/", base)}">${icon("play")} Try in the Playground</a>
        <a class="btn btn--outlined" href="${url("docs/", base)}">Read the docs</a>
        <a class="btn btn--text" href="${site.repository}" target="_blank" rel="noopener">${icon("github")} Source</a>
      </div>
    </div>
    <div class="hero__code">
      ${codeBlock({ source: HERO_SOURCE, title: "fibonacci.aura", runnable: true, base })}
    </div>
  </div>
</section>

<section class="section">
  <div class="container">
    <div class="section__head">
      <span class="section__eyebrow">What is Aura?</span>
      <h2>A complete, small language — not a sketch of one.</h2>
      <p class="section__lede">
        Aura is implemented end to end: lexer, parser, checker, interpreter,
        standard library, CLI, REPL, and a browser runtime that runs the same
        interpreter through WebAssembly. Its design favours a single obvious
        spelling for each construct over permissiveness.
      </p>
    </div>
    <div class="grid grid--3">
      <div class="card card--elevated">
        <h3>Expression-oriented</h3>
        <p><code>if</code>, <code>match</code>, and blocks all yield values. A function
        returns the value of its last statement. There is no statement/expression
        divide to memorise.</p>
      </div>
      <div class="card card--elevated">
        <h3>Conservative checking</h3>
        <p>Type annotations are optional. The checker rejects only what it can
        <em>prove</em> is wrong; unknown types are left to the runtime, so it never
        over-rejects.</p>
      </div>
      <div class="card card--elevated">
        <h3>Explicit capabilities</h3>
        <p>Every interaction with the outside world goes through a host boundary:
        standard output, input, arguments, filesystem, clock, and sleep. The same
        program runs natively or in the browser.</p>
      </div>
    </div>
  </div>
</section>

<section class="section section--alt">
  <div class="container">
    <div class="section__head">
      <span class="section__eyebrow">Why Aura</span>
      <h2>Predictable, inspectable, and portable.</h2>
      <p class="section__lede">The language makes a small set of firm choices and
      keeps them consistent across every entry point.</p>
    </div>
    <div class="grid grid--2">
      ${feature("◆", "One spelling per construct", "Functions are <code>fn</code>; logic is <code>and</code>, <code>or</code>, <code>not</code>; absence is <code>none</code>. There is no <code>&&</code>, <code>null</code>, or <code>def</code> to trip over.")}
      ${feature("◇", "Immutable by default", "<code>let</code> binds immutably; <code>let mut</code> opts into reassignment. Mutation is visible and intentional.")}
      ${feature("▲", "Structured diagnostics", "Every rejection carries a stable <code>E####</code> code, grouped by phase: lexical, checking, type, runtime, capability. Codes are never reused.")}
      ${feature("▼", "One runtime, every substrate", "The same pipeline — <code>lex → parse → check → execute</code> — runs from the CLI, the REPL, the library, and the browser WASM runtime.")}
    </div>
  </div>
</section>

<section class="section">
  <div class="container">
    <div class="section__head">
      <span class="section__eyebrow">How Aura works</span>
      <h2>From source to result.</h2>
      <p class="section__lede">Aura is interpreted. Source is tokenized, parsed,
      checked, then walked. The same interpreter powers every surface.</p>
    </div>
    <div class="grid grid--2">
      <div>
        ${codeBlock({
          source: `struct Point { x: int, y: int }

enum Shape { Circle(int), Rectangle(int, int) }

fn area(s) -> int {
    return match s {
        Circle(r) -> 3 * r * r
        Rectangle(w, h) -> w * h
    }
}

fn main() {
    print(area(Circle(2)))
    print(area(Rectangle(3, 4)))
}`,
          title: "data.aura",
          runnable: true,
          base,
        })}
      </div>
      <div class="stack">
        <h3>Values</h3>
        <p><code>int</code>, <code>float</code>, <code>bool</code>, <code>string</code>,
        <code>none</code>, lists, string-keyed maps, struct instances, enum variants,
        functions, and ranges.</p>
        <h3>Patterns and matching</h3>
        <p><code>match</code> destructures list and variant patterns, with optional
        guards. A destructuring <code>let</code> binds several names at once.</p>
        <h3>Functions and closures</h3>
        <p>Functions are first-class and hoisted; lambdas capture their defining
        environment by reference. Named arguments are available for directly
        resolved functions.</p>
        <div class="pill-row">
          <a class="chip chip--assist" href="${url("language/", base)}">Language overview</a>
          <a class="chip chip--assist" href="${url("architecture/", base)}">Architecture</a>
        </div>
      </div>
    </div>
  </div>
</section>

<section class="section section--alt">
  <div class="container">
    <div class="section__head">
      <span class="section__eyebrow">Runtime &amp; WebAssembly</span>
      <h2>The same language, native or in the browser.</h2>
      <p class="section__lede">
        Aura's runtime talks to the outside world only through a host. Natively
        that host is the operating system; in the browser it is a
        capability-limited host running inside a Web Worker.
      </p>
    </div>
    <div class="grid grid--3">
      <div class="card">
        <h3>Native runtime</h3>
        <p>The CLI and REPL run the interpreter on a dedicated execution stack,
        with real filesystem, clock, and sleep through the native host.</p>
      </div>
      <div class="card">
        <h3>WASM runtime</h3>
        <p>The Playground loads a zero-import WebAssembly module built from
        this repository's runtime. It reaches no DOM, network, or storage.</p>
      </div>
      <div class="card">
        <h3>Versioned artifacts</h3>
        <p>Each runtime is an immutable artifact with a recorded hash. Selecting a
        version selects the artifact that executes — never a moving target.</p>
      </div>
    </div>
    <div class="hero__actions" style="margin-top:var(--space-8)">
      <a class="btn btn--tonal" href="${url("playground/", base)}">${icon("play")} Open the Playground</a>
      <a class="btn btn--text" href="${url("runtime/", base)}">Runtime details ${icon("arrow")}</a>
    </div>
  </div>
</section>

<section class="section">
  <div class="container">
    <div class="grid grid--2" style="align-items:start">
      <div>
        <div class="section__head">
          <span class="section__eyebrow">Current state</span>
          <h2>What exists today.</h2>
          <p class="section__lede">Aura ${site.releaseVersion} is the current public
          release, implementing the frozen <strong>${site.languageVersion}</strong>
          language. It adds the WebAssembly runtime, the host boundary, the
          versioned Playground, and this website on top of the
          ${site.previousRelease} language core.</p>
        </div>
        ${statusList([
          ["Lexer, parser, AST", "Implemented", "success"],
          ["Static checks (names, mutability, types)", "Implemented", "success"],
          ["Tree-walking interpreter", "Implemented", "success"],
          ["Core stdlib + json, regex, time", "Implemented", "success"],
          ["CLI, REPL, scripting I/O", "Implemented", "success"],
          ["Cross-platform CI", "Implemented", "success"],
          ["WebAssembly runtime + Playground", "Released", "success"],
          ["OOP, Python/PyO3, data ecosystem", "Planned", "planned"],
        ])}
      </div>
      <div>
        <div class="section__head">
          <span class="section__eyebrow">Learn Aura</span>
          <h2>Start where you like.</h2>
        </div>
        <div class="stack">
          <a class="card" href="${url("learn/", base)}" style="display:block;text-decoration:none">
            <h3>Learn Aura</h3>
            <p>A guided progression from your first program to closures, data types,
            and error handling.</p>
            <span class="eyebrow-link">Start learning ${icon("arrow")}</span>
          </a>
          <a class="card" href="${url("examples/", base)}" style="display:block;text-decoration:none">
            <h3>Examples</h3>
            <p>${16} runnable programs — each validated against the real runtime — with
            copy and Run-in-Playground actions.</p>
            <span class="eyebrow-link">Browse examples ${icon("arrow")}</span>
          </a>
          <a class="card" href="${url("stdlib/", base)}" style="display:block;text-decoration:none">
            <h3>Standard Library</h3>
            <p>Built-in functions, methods, and the feature-gated json, regex, and
            time modules.</p>
            <span class="eyebrow-link">Browse the stdlib ${icon("arrow")}</span>
          </a>
        </div>
      </div>
    </div>
  </div>
</section>

<section class="section section--alt">
  <div class="container">
    <div class="section__head">
      <span class="section__eyebrow">Roadmap</span>
      <h2>Where Aura is going.</h2>
      <p class="section__lede">With ${site.releaseVersion} released, development
      slows to a deliberate language-maturity cycle before any new paradigm is
      added.</p>
    </div>
    ${dataTable(
      ["Stage", "Focus", "Status"],
      [
        ["Gaiola 1–7", "Runtime, CI, host boundary, WASM, Playground", chip("Complete", "success")],
        ["0.0.1", "Scripting I/O, language core, native runtime", chip("Released", "success")],
        ["0.0.2", "WebAssembly, host boundary, Playground, website", chip("Released", "success")],
        ["Maturation", "Syntax refinement, semantics, stdlib, diagnostics", chip("Planned", "planned")],
        ["Hardening", "Conformance, differential testing, fuzzing", chip("Planned", "planned")],
        ["Syntax freeze", "Lock the language surface", chip("Planned", "planned")],
        ["OOP", "Architecture and implementation", chip("Planned", "planned")],
        ["Python / data", "PyO3 interop and the scientific ecosystem", chip("Long-term", "planned")],
      ],
    )}
    <a class="eyebrow-link" href="${url("roadmap/", base)}">Full roadmap ${icon("arrow")}</a>
  </div>
</section>

<section class="section">
  <div class="container center">
    <div class="section__head" style="margin-inline:auto">
      <span class="section__eyebrow">Get involved</span>
      <h2>Try it, read it, or build it.</h2>
      <p class="section__lede">Aura is open source under the ${site.license} license.</p>
    </div>
    <div class="hero__actions" style="justify-content:center">
      <a class="btn btn--filled" href="${url("playground/", base)}">${icon("play")} Open the Playground</a>
      <a class="btn btn--outlined" href="${url("docs/", base)}">Documentation</a>
      <a class="btn btn--text" href="${site.repository}" target="_blank" rel="noopener">${icon("github")} GitHub</a>
      <a class="btn btn--text" href="${url("releases/", base)}">Releases</a>
    </div>
  </div>
</section>
`;
  },
};
