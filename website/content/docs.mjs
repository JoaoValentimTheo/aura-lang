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
// semantics it documents are `docsLanguageVersion`; the two are intentionally
// distinct (ADR-0001). These pages document the current release `0.3.1`
// (codename Keystone: distinct collection identities, fixed-length contextual
// Arrays, the canonical type-position `json_decode_as`, parser hardening, and
// the multi-file Playground); the migration guide covers the
// `0.2.1 → 0.3.1` step.
//
// The manifest is the authoritative inventory of documentation routes. Pages
// are grouped by *meaning*, not one page per bullet: closely related topics
// (e.g. the AIS protocol and its MCP adapter) stay distinct pages because they
// document distinct surfaces, while topics with no page of their own are
// covered inside the page that owns them (e.g. "basic examples" live in
// Getting started and the language guide). Every slug is a preserved public
// route; slugs are never renamed for aesthetics.

export const docsVersion = "0.3.1";
export const docsLanguageVersion = "0.3.1";

export const docGroups = [
  {
    title: "Getting Started",
    items: [
      { slug: "getting-started", title: "Introduction", file: "getting-started.md" },
      { slug: "install", title: "Installation", file: "install.md" },
      { slug: "first-program", title: "Your first program", file: "first-program.md" },
      { slug: "zen", title: "Design principles", file: "zen.md" },
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
      { slug: "guide-oop", title: "The object model", file: "guide-oop.md" },
      { slug: "guide-generics", title: "Generics", file: "guide-generics.md" },
      { slug: "guide-modules", title: "Modules and visibility", file: "guide-modules.md" },
      { slug: "guide-matching", title: "Pattern matching", file: "guide-matching.md" },
      { slug: "guide-errors", title: "Errors", file: "guide-errors.md" },
      { slug: "guide-io", title: "I/O and arguments", file: "guide-io.md" },
    ],
  },
  {
    title: "Language Reference",
    items: [
      { slug: "reference-grammar", title: "Grammar", file: "reference-grammar.md" },
      { slug: "reference-types", title: "Types", file: "reference-types.md" },
      { slug: "reference-operators", title: "Operators", file: "reference-operators.md" },
      { slug: "reference-stdlib", title: "Standard library", file: "reference-stdlib.md" },
      { slug: "reference-errors", title: "Diagnostics", file: "reference-errors.md" },
      { slug: "reference-limits", title: "Resource limits", file: "reference-limits.md" },
    ],
  },
  {
    title: "Runtime & Tooling",
    items: [
      { slug: "cli", title: "CLI", file: "cli.md" },
      { slug: "repl", title: "REPL", file: "repl.md" },
      { slug: "playground-doc", title: "Playground", file: "playground-doc.md" },
      { slug: "runtime-doc", title: "Runtime & host", file: "runtime-doc.md" },
      { slug: "python", title: "Python interoperability", file: "python.md" },
      { slug: "security", title: "Security", file: "security.md" },
    ],
  },
  {
    title: "Semantic Tooling",
    items: [
      { slug: "ais", title: "Aura Intelligence Schema (AIS)", file: "ais.md" },
      { slug: "mcp", title: "MCP adapter", file: "mcp.md" },
    ],
  },
  {
    title: "Releases & Migration",
    items: [
      { slug: "migration-0-3-1", title: "Migrating to 0.3.1", file: "migration-0-3-1.md" },
      { slug: "migration-0-2-1", title: "Migrating to 0.2.1", file: "migration-0-2-1.md" },
      { slug: "known-limitations", title: "Known limitations", file: "known-limitations.md" },
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
