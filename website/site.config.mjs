// Central site configuration. Every generator module reads from here so the
// public identity, canonical origin, and navigation live in exactly one place.

export const site = {
  name: "Aura",
  tagline: "A small, expression-oriented scripting language.",
  description:
    "Aura is a small, dynamically-typed, expression-oriented scripting language with a conservative static checker, a native Rust runtime, and a versioned in-browser WebAssembly Playground.",
  // The intended public origin. Used for canonical URLs, Open Graph, sitemap,
  // and the CNAME file written into the build. The GitHub repository path is
  // deliberately not part of the public identity.
  origin: "https://aura.lang.dev",
  repository: "https://github.com/JoaoValentimTheo/aura-lang",
  issues: "https://github.com/JoaoValentimTheo/aura-lang/issues",
  releases:
    "https://github.com/JoaoValentimTheo/aura-lang/releases",
  license: "MIT",
  // Three distinct identities, deliberately not collapsed:
  //   * releaseVersion  — the published release / runtime artifact (0.0.2)
  //   * languageVersion — the frozen language semantics (0.0.1)
  //   * previousRelease — the prior published release, kept addressable
  releaseVersion: "0.0.2",
  languageVersion: "0.0.1",
  runtimeVersion: "0.0.2",
  previousRelease: "0.0.1",
  // Current published release, shown in the header and hero.
  currentRelease: "0.0.2",
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
