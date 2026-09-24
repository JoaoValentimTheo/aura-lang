import { url, icon, codeBlock, pageHead, callout } from "../lib/components.mjs";

const STEPS = [
  {
    title: "Try Aura",
    body: "Open the Playground and run the default program. Nothing to install.",
    href: "playground/",
    cta: "Open Playground",
  },
  {
    title: "First program",
    body: "Declare fn main, print a line, and bind a value.",
    href: "docs/first-program/",
    cta: "Your first program",
  },
  {
    title: "Values and bindings",
    body: "The value universe, immutable and mutable bindings, and truthiness.",
    href: "docs/guide-basics/",
    cta: "Values and bindings",
  },
  {
    title: "Functions and closures",
    body: "Declaration, arguments, recursion, lambdas, and the pipeline.",
    href: "docs/guide-functions/",
    cta: "Functions and closures",
  },
  {
    title: "Control flow",
    body: "if, loops, ranges, break/continue, return and throw.",
    href: "docs/guide-control/",
    cta: "Control flow",
  },
  {
    title: "Collections",
    body: "Lists, string-keyed maps, ranges, and higher-order methods.",
    href: "docs/guide-collections/",
    cta: "Collections",
  },
  {
    title: "Structs and enums",
    body: "Nominal data types with typed fields and variant payloads.",
    href: "docs/guide-data/",
    cta: "Structs and enums",
  },
  {
    title: "Pattern matching",
    body: "Destructure values with match, guards, and let patterns.",
    href: "docs/guide-matching/",
    cta: "Pattern matching",
  },
  {
    title: "Errors",
    body: "Throw, catch, finally, and why runtime diagnostics are fatal.",
    href: "docs/guide-errors/",
    cta: "Errors",
  },
  {
    title: "Advanced usage",
    body: "I/O and arguments, JSON, regex, the CLI, and the REPL.",
    href: "docs/guide-io/",
    cta: "I/O and arguments",
  },
];

export const learnPage = {
  key: "learn",
  title: "Learn Aura",
  path: "learn/",
  activeKey: "learn",
  description:
    "A guided progression through Aura, from your first program to advanced usage.",
  async render(base) {
    const cards = STEPS.map(
      (s, i) => `<a class="card" href="${url(s.href, base)}" style="display:block;text-decoration:none">
  <div class="example-card__meta"><span class="tag">Step ${i + 1}</span></div>
  <h3>${s.title}</h3>
  <p>${s.body}</p>
  <span class="eyebrow-link">${s.cta} ${icon("arrow")}</span>
</a>`,
    ).join("");
    return `${pageHead({
      eyebrow: "Learn",
      title: "Learn Aura",
      lede: "A guided path that builds up the language a step at a time. Every example can be run in the Playground.",
    })}
<section class="section">
  <div class="container">
    ${codeBlock({
      source: `fn main() {
    print("hello, Aura")
}`,
      title: "start.aura",
      runnable: true,
      base,
    })}
    ${callout(
      "info",
      "<p>Prefer the fastest route? The <a href='" +
        url("playground/", base) +
        "'>Playground</a> needs no installation. Prefer your machine? See <a href='" +
        url("docs/install/", base) +
        "'>Installing Aura</a>.</p>",
    )}
    <div class="grid grid--3" style="margin-top:var(--space-8)">${cards}</div>
  </div>
</section>`;
  },
};
