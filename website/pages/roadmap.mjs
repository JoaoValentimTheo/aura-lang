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
    body: "The WebAssembly runtime, the host boundary, the versioned Playground, the official website, GitHub Pages deployment, and cross-platform release infrastructure. The language semantics remained the frozen 0.0.1.",
  },
  {
    tag: "Released",
    kind: "success",
    title: "0.2.0 · Core completion",
    body: "The completed Aura Core: generic map keys and ordered keys, <code>map.items()</code>, list and map comprehensions, the Core separator/numeric/f-string rules, completed in-source module semantics (<code>pub</code>, <code>pub use</code>, aliases), collection type coherence, and CLI/REPL and Playground completion, with Native/WASM parity.",
  },
  {
    tag: "Released",
    kind: "success",
    title: "0.2.1 · Filesystem modules and multi-file Playground",
    body: "Filesystem-backed module acquisition for the CLI: a sibling <code>math.aura</code> or a directory <code>pkg/mod.aura</code> becomes a logical module through the same resolver as in-source modules (dual ownership is <code>E2020</code>). The Playground gains virtual multi-file projects over the additive Host ABI 1 <code>aura_project_*</code> transport. The language tightens too: builtin names are reserved as value bindings (<code>E1009</code>) and structural type nesting is bounded at 256 on every substrate (ADR-0004).",
  },
  {
    tag: "Delivered",
    kind: "success",
    title: "OOP V1 · Struct methods",
    body: "Behavior attached to structs with <code>impl</code> blocks and an explicit <code>self</code> receiver, plus composition. Structs remain data, methods remain functions, and there are no classes, inheritance, or dynamic dispatch.",
  },
  {
    tag: "Delivered",
    kind: "success",
    title: "Traits · static behavioral contracts",
    body: "A <code>trait</code> names a set of method signatures a struct agrees to implement with <code>impl Trait for Struct</code>. Traits add no value type and no dynamic dispatch: calls resolve statically by nominal type.",
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
    tag: "Delivered",
    kind: "success",
    title: "OOP completion · the four pillars",
    body: "The object model is finalized and mapped to the four classical concepts: <strong>encapsulation</strong> as struct data, deterministic member lookup, the mutation-capability rule, and <strong>in-source modules with real <code>pub</code>/private visibility</strong>, <strong>abstraction</strong> through traits, <strong>reuse</strong> by composition plus traits rather than inheritance, and <strong>polymorphism</strong> as overloading and static union resolution. No classes, no inheritance, no dynamic dispatch.",
  },
  {
    tag: "Delivered",
    kind: "success",
    title: "Generics and trait bounds",
    body: "Static, erased, nominal parametric polymorphism over the OOP model: generic functions, structs, methods, traits, and aliases, with trait bounds and inference composed into the single overload resolver. Collections stay structural (<code>[T]</code>, <code>{K: V}</code>, where a key must be key-capable); a type parameter is a compile-time placeholder erased before execution.",
  },
  {
    tag: "Delivered",
    kind: "success",
    title: "Hardening and conformance",
    body: "Conformance suites, differential testing, and fuzzing, plus the train-1 red-team campaign: every finding was fixed with regression coverage, and the language behaves identically across every substrate.",
  },
  {
    tag: "Pending human decision",
    kind: "planned",
    title: "Next major direction",
    body: "The filesystem-module program (FSM-P1…P6) is delivered and released. No successor engineering program has been selected; nothing is started by assumption. The explicitly deferred candidates include package management, browser persistence, an LSP, a formatter, async, and macros.",
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
    "The Aura roadmap: delivered infrastructure (0.0.1, 0.0.2), the completed Aura Core (0.2.0), the released 0.2.1 (filesystem modules and the multi-file Playground), and the next direction, which is not yet selected.",
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
      lede: "Aura is built in deliberate stages. Infrastructure and portability came first; the language core and the filesystem-module program are now delivered and released as 0.2.1.",
    })}
<section class="section">
  <div class="container">
    ${callout(
      "info",
      "<p>Items marked <strong>Pending human decision</strong> or <strong>Long-term</strong> are not available today and are not started. They are listed so the project's direction is honest and visible.</p>",
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
      conformance testing — which is complete. OOP V1 (struct methods) and
      OOP V2 (traits) are delivered, and the language foundation (bindings,
      scopes, mutation, operators, f-strings) is synchronized, stabilised, and
      frozen. Function and method overloading, generics, and the completed Core
      collections (generic maps, <code>items()</code>, comprehensions) are
      implemented and released as <strong>0.2.0</strong>. The filesystem-module
      program (FSM-P1…P6) shipped in <strong>0.2.1</strong>. The next major
      direction has not yet been selected.</p>
      <a class="eyebrow-link" href="${url("releases/", base)}">See releases →</a>
    </div>
  </div>
</section>`;
  },
};
