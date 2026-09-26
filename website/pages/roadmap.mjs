import { url, pageHead, chip, callout } from "../lib/components.mjs";

const PHASES = [
  {
    tag: "Delivered",
    kind: "success",
    title: "Runtime, host, WASM and Playground",
    body: "A hardened runtime, cross-platform CI, a WebAssembly execution substrate, a host contract and execution boundary, WASM semantic validation, and a versioned browser Playground that runs the real runtime.",
  },
  {
    tag: "Released",
    kind: "success",
    title: "0.0.2 · WebAssembly, Playground, website",
    body: "The WebAssembly runtime, the host boundary, the versioned Playground, the official website, GitHub Pages deployment, and cross-platform release infrastructure. The language semantics remain the frozen 0.0.1.",
  },
  {
    tag: "Delivered",
    kind: "success",
    title: "OOP V1 · Struct methods",
    body: "Behavior attached to structs with <code>impl</code> blocks and an explicit <code>self</code> receiver, plus composition. Structs remain data, methods remain functions, and there are no classes, inheritance, or dynamic dispatch. Available in the development runtime.",
  },
  {
    tag: "Planned",
    kind: "planned",
    title: "Language maturation",
    body: "Syntax refinement, semantic consistency, standard-library and API refinement, diagnostics, and parser/checker/runtime hardening. This is a deliberately slow cycle.",
  },
  {
    tag: "Planned",
    kind: "planned",
    title: "Hardening and conformance",
    body: "Conformance suites, differential testing, and fuzzing. The goal is confidence that the language behaves identically across every substrate.",
  },
  {
    tag: "Planned",
    kind: "planned",
    title: "Syntax freeze",
    body: "Once the semantics are settled, the language surface is frozen so tools and documentation can stabilise.",
  },
  {
    tag: "Future",
    kind: "planned",
    title: "Traits and generics",
    body: "Abstraction and parameterisation build on the current OOP model of nominal structs and methods. Inheritance is not the direction: reuse stays composition-first.",
  },
  {
    tag: "Long-term",
    kind: "planned",
    title: "Python / PyO3 and the data ecosystem",
    body: "Deep Python interoperability through PyO3, and the data, scientific, and AI ecosystem that depends on it. A 1.0-era direction.",
  },
];

export const roadmapPage = {
  key: "roadmap",
  title: "Roadmap",
  path: "roadmap/",
  activeKey: "roadmap",
  description:
    "The Aura roadmap: delivered infrastructure, the 0.0.2 website and release, the language-maturation cycle, and long-term directions.",
  async render(base) {
    const items = PHASES.map(
      (p) => `<div class="card card--elevated">
  <div class="example-card__meta">${chip(p.tag, p.kind)}</div>
  <h3>${p.title}</h3>
  <p>${p.body}</p>
</div>`,
    ).join("");
    return `${pageHead({
      eyebrow: "Roadmap",
      title: "Where Aura is going",
      lede: "Aura is built in deliberate stages. Infrastructure and portability come first; the language surface settles before any new paradigm is added.",
    })}
<section class="section">
  <div class="container">
    ${callout(
      "info",
      "<p>Items marked <strong>Planned</strong> or <strong>Long-term</strong> are not available today. They are listed so the project's direction is honest and visible.</p>",
    )}
    <div class="grid grid--2" style="margin-top:var(--space-6)">${items}</div>
    <div class="divider"></div>
    <div class="prose">
      <h2>Why infrastructure first</h2>
      <p>A language's long-term health depends on its execution substrates and
      its reproducibility. Aura therefore prioritises a hardened runtime, a
      clean host boundary, verified WebAssembly execution, and versioned
      artifacts before it grows its syntax.</p>
      <p>After 0.0.2, development slows — on purpose — into a maturation cycle
      of syntax refinement, semantic consistency, diagnostics, and conformance
      testing. Only after a syntax freeze does object-oriented design begin.</p>
      <a class="eyebrow-link" href="${url("releases/", base)}">See releases →</a>
    </div>
  </div>
</section>`;
  },
};
