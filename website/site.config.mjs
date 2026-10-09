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
  // Distinct identities, deliberately not collapsed (ADR-0001). `0.3.1` is the
  // current published release (codename Keystone); the language it implements
  // is also `0.3.1`.
  //   * releaseVersion      — the current published release / runtime artifact
  //   * languageVersion     — the language semantics the release implements
  //   * runtimeVersion      — the current browser runtime artifact
  //   * currentRelease      — the published release shown in the header/hero
  //   * developmentVersion  — the unreleased development line, or null
  //   * previousRelease     — the prior published release, kept addressable
  releaseVersion: "0.3.1",
  languageVersion: "0.3.1",
  // The runtime crate's current build identity. While the 0.3.2 development
  // line is open this is a pre-release; the *published* default runtime
  // remains `releaseVersion` (`manifest.current` stays 0.3.1 until a human
  // authorizes promotion). See `docs/engineering/PLAYGROUND_032_CAMPAIGN.md`.
  runtimeVersion: "0.3.2-dev.5",
  previousRelease: "0.2.1",
  // Historical releases, kept addressable and clearly not current.
  earliestRelease: "0.0.1",
  // Current published release, shown in the header and hero.
  currentRelease: "0.3.1",
  // The `0.3.2` development line is open locally: a development runtime
  // (`0.3.2-dev.1`) exists in the manifest. It is never a published release:
  // the released `0.3.1` still ships to GitHub Pages as the default until a
  // human authorizes otherwise. Setting this here keeps the version-parity
  // guard honest about the open line.
  developmentVersion: "0.3.2",
};

// Primary navigation. `key` links a nav entry to page metadata.
//
// This array is the *single* source of truth for the destination set. The app
// bar derives every navigation surface from it — the wide inline bar, the
// "More" menu, and the compact dropdown — so no two navigation surfaces can
// drift apart. `navPrimary` selects the restrained set rendered inline on wide
// layouts; `navGroups` labels the remaining destinations in the overflow menu.
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

// The restricted set shown inline when there is room; everything else lives in
// the accessible "More" menu. Both derive from `nav` above.
export const navPrimary = ["learn", "language", "docs", "playground", "examples"];

// Grouping for the overflow/mobile menus. This must cover every `key` in
// `nav`, so that the compact menu lists every destination exactly once. The
// "More" menu is derived from the same groups minus the inline set, so no
// destination is duplicated across surfaces and none is dropped.
export const navGroups = [
  { title: "Overview", keys: ["home", "learn", "language"] },
  { title: "Language", keys: ["stdlib", "examples", "docs"] },
  { title: "Platform", keys: ["playground", "runtime", "architecture", "tools"] },
  { title: "Project", keys: ["roadmap", "releases", "about"] },
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
