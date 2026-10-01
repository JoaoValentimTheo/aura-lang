// Central site configuration. Every generator module reads from here so the
// public identity, canonical origin, and navigation live in exactly one place.

export const site = {
  name: "Aura",
  tagline: "A small, expression-oriented scripting language.",
  description:
    "Aura is a small, dynamically-typed, expression-oriented scripting language with a conservative static checker, a native Rust runtime, and a versioned in-browser WebAssembly Playground.",
  // The public origin. GitHub Pages is the definitive hosting platform, so the
  // canonical origin is the project-site URL. It is used for canonical URLs,
  // Open Graph, and the sitemap. The abandoned custom domain is not referenced.
  origin: "https://joaovalentimtheo.github.io/aura-lang",
  // Human-readable form of `origin`, for display in prose.
  publicUrl: "joaovalentimtheo.github.io/aura-lang",
  repository: "https://github.com/JoaoValentimTheo/aura-lang",
  issues: "https://github.com/JoaoValentimTheo/aura-lang/issues",
  releases:
    "https://github.com/JoaoValentimTheo/aura-lang/releases",
  license: "MIT",
  // Distinct identities, deliberately not collapsed (ADR-0001). `0.2.0` is the
  // current published release; `0.2.1` is the development line, whose language
  // is a superset of `0.2.0` (builtin-name reservation, unified type nesting).
  //   * releaseVersion      — the current published release / runtime artifact
  //   * languageVersion     — the language semantics the release implements
  //   * runtimeVersion      — the current browser runtime artifact
  //   * currentRelease      — the published release shown in the header/hero
  //   * developmentVersion  — the unreleased development line (labeled as such)
  //   * previousRelease     — the prior published release, kept addressable
  releaseVersion: "0.2.0",
  languageVersion: "0.2.0",
  runtimeVersion: "0.2.0",
  previousRelease: "0.0.2",
  // Historical releases, kept addressable and clearly not current.
  earliestRelease: "0.0.1",
  // Current published release, shown in the header and hero.
  currentRelease: "0.2.0",
  // The development line: documented and shown, but always clearly labeled
  // "development" so it is never mistaken for the stable release.
  developmentVersion: "0.2.1",
};

// Primary navigation. `key` links a nav entry to page metadata.
export const nav = [
  { key: "home", label: "Home", href: "" },
  { key: "learn", label: "Learn", href: "learn/" },
  { key: "language", label: "Language", href: "language/" },
  { key: "docs", label: "Docs", href: "docs/" },
  { key: "playground", label: "Playground", href: "playground/" },
  { key: "examples", label: "Examples", href: "examples/" },
  { key: "stdlib", label: "Standard Library", href: "stdlib/" },
  { key: "runtime", label: "Runtime", href: "runtime/" },
  { key: "tools", label: "Tools", href: "tools/" },
  { key: "architecture", label: "Architecture", href: "architecture/" },
  { key: "roadmap", label: "Roadmap", href: "roadmap/" },
  { key: "releases", label: "Releases", href: "releases/" },
  { key: "about", label: "About", href: "about/" },
];

// Footer grouping for a denser set of links.
export const footerColumns = [
  {
    title: "Language",
    links: [
      { label: "Overview", href: "language/" },
      { label: "Learn Aura", href: "learn/" },
      { label: "Standard Library", href: "stdlib/" },
      { label: "Examples", href: "examples/" },
    ],
  },
  {
    title: "Platform",
    links: [
      { label: "Playground", href: "playground/" },
      { label: "Runtime", href: "runtime/" },
      { label: "Architecture", href: "architecture/" },
      { label: "Tools & CLI", href: "tools/" },
    ],
  },
  {
    title: "Project",
    links: [
      { label: "Documentation", href: "docs/" },
      { label: "Roadmap", href: "roadmap/" },
      { label: "Releases", href: "releases/" },
      { label: "About", href: "about/" },
    ],
  },
];
