// The documentation manifest.
//
// Each entry is a documentation page backed by a Markdown file under
// `website/content/`. The renderer turns the Markdown into a docs layout with
// a grouped sidebar, breadcrumbs, and an on-this-page table of contents.
//
// The `version` field records which Aura version a page documents. Versioned
// documentation is first-class: pages can be added per version without
// disturbing existing ones.
//
// `docsVersion` is the **release** the docs are published with. The language
// semantics it documents are the frozen `docsLanguageVersion`; the two are
// intentionally distinct.

export const docsVersion = "0.0.2";
export const docsLanguageVersion = "0.0.1";

export const docGroups = [
  {
    title: "Getting Started",
    items: [
      { slug: "getting-started", title: "Getting started", file: "getting-started.md" },
      { slug: "install", title: "Installing Aura", file: "install.md" },
      { slug: "first-program", title: "Your first program", file: "first-program.md" },
    ],
  },
  {
    title: "Language Guide",
    items: [
      { slug: "guide-basics", title: "Values and bindings", file: "guide-basics.md" },
      { slug: "guide-functions", title: "Functions and closures", file: "guide-functions.md" },
      { slug: "guide-control", title: "Control flow", file: "guide-control.md" },
      { slug: "guide-collections", title: "Collections", file: "guide-collections.md" },
      { slug: "guide-data", title: "Structs and enums", file: "guide-data.md" },
      { slug: "guide-matching", title: "Pattern matching", file: "guide-matching.md" },
      { slug: "guide-errors", title: "Errors", file: "guide-errors.md" },
      { slug: "guide-io", title: "I/O and arguments", file: "guide-io.md" },
    ],
  },
  {
    title: "Reference",
    items: [
      { slug: "reference-grammar", title: "Grammar", file: "reference-grammar.md" },
      { slug: "reference-operators", title: "Operators", file: "reference-operators.md" },
      { slug: "reference-types", title: "Types", file: "reference-types.md" },
      { slug: "reference-stdlib", title: "Standard library", file: "reference-stdlib.md" },
      { slug: "reference-errors", title: "Diagnostics", file: "reference-errors.md" },
      { slug: "reference-limits", title: "Resource limits", file: "reference-limits.md" },
    ],
  },
  {
    title: "Tooling",
    items: [
      { slug: "cli", title: "CLI", file: "cli.md" },
      { slug: "repl", title: "REPL", file: "repl.md" },
      { slug: "playground-doc", title: "Playground", file: "playground-doc.md" },
      { slug: "runtime-doc", title: "Runtime & host", file: "runtime-doc.md" },
    ],
  },
];

export function allDocs() {
  return docGroups.flatMap((g) =>
    g.items.map((it) => ({ ...it, group: g.title })),
  );
}

export function findDoc(slug) {
  return allDocs().find((d) => d.slug === slug) || null;
}
