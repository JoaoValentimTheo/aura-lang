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
    tag: "Delivered",
    kind: "success",
    title: "Traits · static behavioral contracts",
    body: "A <code>trait</code> names a set of method signatures a struct agrees to implement with <code>impl Trait for Struct</code>. Traits add no value type and no dynamic dispatch: calls resolve statically by nominal type. Available in the development runtime.",
  },
  {
    tag: "Delivered",
    kind: "success",
    title: "Language foundation · contract synchronized",
    body: "The core contract was synchronised end to end: <code>const</code> as the canonical module constant, one explicit scope/binding matrix, the mutation-capability model (<code>let mut</code> / <code>mut self</code>), the bitwise operator family with compound assignments, a small f-string format mini-language, and uniform trailing commas.",
  },
  {
    tag: "Delivered",
    kind: "success",
    title: "Shadowing · variable bindings",
    body: "<code>let</code> and <code>let mut</code> always create a new binding and may shadow an existing one, including in the same scope. The initializer resolves against the previous binding, and <code>mut</code> belongs to the new binding. <code>const</code> is not shadowable.",
  },
  {
    tag: "Delivered",
    kind: "success",
    title: "Foundation stability · CI and clean-room verified",
    body: "A full validation, Break-the-Aura destruction pass, semantic-consistency audit, and PC ↔ Web symmetry gate, with CI, a clean-room build, and native/WASM differential all green. The foundation is frozen for feature development.",
  },
  {
    tag: "Delivered",
    kind: "success",
    title: "Function & method overloading",
    body: "A function or method may have several definitions when their ordered input types differ. The return type does not distinguish overloads, and resolution is deterministic: the most specific match wins, and a tie is an error rather than a coin flip.",
  },
  {
    tag: "Next",
    kind: "planned",
    title: "Generics and trait bounds",
    body: "Parameterisation builds on the current OOP model of nominal structs, methods, traits, and overloads. Inheritance is not the direction: reuse stays composition-first.",
  },
  {
    tag: "Planned",
    kind: "planned",
    title: "Hardening and conformance",
    body: "Conformance suites, differential testing, and fuzzing. The goal is confidence that the language behaves identically across every substrate.",
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
    "The Aura roadmap: delivered infrastructure, the 0.0.2 website and release, the delivered language foundation (OOP V1/V2, shadowing, stability), and the next features — method overloading, then generics.",
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
      lede: "Aura is built in deliberate stages. Infrastructure and portability came first; the language foundation is now synchronized and frozen before the next feature is added.",
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
      <p>After 0.0.2, development moved through a deliberate maturation
      cycle — syntax refinement, semantic consistency, diagnostics, and
      conformance testing — which is now complete. OOP V1 (struct methods) and
      OOP V2 (traits) are delivered, and the language foundation (bindings,
      scopes, mutation, operators, f-strings) is synchronized, stabilised, and
      frozen. Function and method overloading is implemented; generics follow
      later.</p>
      <a class="eyebrow-link" href="${url("releases/", base)}">See releases →</a>
    </div>
  </div>
</section>`;
  },
};
