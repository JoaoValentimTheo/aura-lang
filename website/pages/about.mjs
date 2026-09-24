import { site } from "../site.config.mjs";
import { url, pageHead, icon, callout } from "../lib/components.mjs";

export const aboutPage = {
  key: "about",
  title: "About",
  path: "about/",
  activeKey: "about",
  description:
    "About the Aura project: its design philosophy, its repository, and how to contribute.",
  async render(base) {
    return `${pageHead({
      eyebrow: "About",
      title: "About Aura",
      lede: "Aura is an open-source, from-scratch programming language with a native Rust runtime and a browser WebAssembly runtime, under the MIT license.",
    })}

<section class="section section--tight">
  <div class="container">
    <div class="grid grid--2">
      <div class="prose">
        <h2>Philosophy</h2>
        <p>Aura is designed to be small enough to hold in your head. It favours
        a single obvious spelling for each construct, immutable bindings by
        default, and a runtime that is inspectable rather than clever.</p>
        <h2>Principles</h2>
        <ul>
          <li><strong>Small language.</strong> A focused set of constructs, each
          with one spelling.</li>
          <li><strong>Expression-oriented.</strong> Blocks, <code>if</code>, and
          <code>match</code> yield values.</li>
          <li><strong>Dynamic with conservative checking.</strong> Annotations are
          optional; the checker rejects only what it can prove.</li>
          <li><strong>Interpreted runtime.</strong> No bytecode, no optimizer —
          a tree-walking interpreter with bounded resources.</li>
          <li><strong>Explicit capabilities.</strong> All outside-world access
          goes through a host.</li>
          <li><strong>Portable execution.</strong> The same runtime runs
          natively and on WebAssembly.</li>
          <li><strong>Reproducibility.</strong> Stable diagnostic codes and
          versioned, hash-identified runtime artifacts.</li>
          <li><strong>Technical simplicity.</strong> Tiny dependencies;
          an unsafe-free core.</li>
        </ul>
      </div>
      <div>
        <div class="card card--elevated">
          <h3>The project</h3>
          <ul>
            <li><strong>License:</strong> ${site.license}</li>
            <li><strong>Language version:</strong> ${site.languageVersion}</li>
            <li><strong>Repository:</strong> <a href="${site.repository}" target="_blank" rel="noopener">github.com/JoaoValentimTheo/aura-lang</a></li>
            <li><strong>Issues:</strong> <a href="${site.issues}" target="_blank" rel="noopener">issue tracker</a></li>
            <li><strong>Releases:</strong> <a href="${site.releases}" target="_blank" rel="noopener">releases</a></li>
            <li><strong>Website:</strong> aura.lang.dev</li>
          </ul>
          <div class="hero__actions" style="margin-top:var(--space-4)">
            <a class="btn btn--filled" href="${site.repository}" target="_blank" rel="noopener">${icon("github")} Source</a>
            <a class="btn btn--outlined" href="${site.issues}" target="_blank" rel="noopener">Issues</a>
          </div>
        </div>
        ${callout(
          "note",
          "<p>GitHub is the project's source and development platform. The public website identity is <strong>aura.lang.dev</strong>.</p>",
        )}
      </div>
    </div>
  </div>
</section>

<section class="section section--alt">
  <div class="container">
    <div class="prose">
      <h2>Contributing</h2>
      <p>Contributions are welcome through issues and pull requests on GitHub.
      Two documents are the project's source of truth:</p>
      <ul>
        <li><a href="${site.repository}/blob/main/docs/LANGUAGE_SPEC.md" target="_blank" rel="noopener">The language specification</a> — normative semantics.</li>
        <li><a href="${site.repository}/blob/main/docs/contract.md" target="_blank" rel="noopener">The compatibility contract</a> — the surface a program may rely on.</li>
      </ul>
      <p>Diagnostic codes are part of the public contract; a code is never reused
      for a different meaning. New diagnostics get new numbers, and the test
      suite asserts that every documented code is reachable.</p>
      <h2>What Aura is not (yet)</h2>
      <p>To keep the project honest: Aura has no classes, inheritance,
      interfaces, traits, or generics; no modules beyond inert <code>use</code>
      and <code>pub</code>; no LSP, debugger, formatter, or package manager; and
      no data-science or AI ecosystem. Those are future directions, listed on the
      <a href="${url("roadmap/", base)}">roadmap</a>.</p>
    </div>
  </div>
</section>`;
  },
};
