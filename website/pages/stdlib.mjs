import { url, icon, codeBlock, pageHead, dataTable, callout } from "../lib/components.mjs";
import {
  coreFunctions,
  collectionFunctions,
  scriptingIo,
  methods,
  featureModules,
} from "../content/stdlib.mjs";

// Render the shared metadata into a signature/returns table.
function fnTable(rows) {
  return dataTable(
    ["Function", "Signature", "Returns"],
    rows.map(([name, sig, returns]) => [`<code>${name}</code>`, `<code>${sig}</code>`, returns]),
  );
}

export const stdlibPage = {
  key: "stdlib",
  title: "Standard Library",
  path: "stdlib/",
  activeKey: "stdlib",
  description:
    "Aura's standard library: core functions, collection functions, methods, scripting I/O, and the json, regex, time, and HTTP capabilities.",
  async render(base) {
    const methodTable = dataTable(
      ["Receiver", "Methods", "Notes"],
      methods.map((m) => [
        `<code>${m.receiver}</code>`,
        m.names.map((n) => `<code>${n}</code>`).join(" "),
        m.note,
      ]),
    );
    const moduleTable = dataTable(
      ["Module", "Gating", "Functions"],
      featureModules.map((mod) => [
        `<code>${mod.name}</code>`,
        mod.gated ? "feature-gated" : "always available",
        mod.functions.map(([n]) => `<code>${n}</code>`).join(" "),
      ]),
    );

    return `${pageHead({
      eyebrow: "Standard Library",
      title: "The Aura standard library",
      lede: "A small, coherent set of builtins, methods, and feature-gated modules. Arity and argument types live in one shared registry that the checker and runtime both consult.",
    })}

<section class="section section--tight">
  <div class="container">
    <div class="prose">
      <h2>Core functions</h2>
      ${fnTable(coreFunctions)}
      <h2>Collection functions</h2>
      ${fnTable(collectionFunctions)}
      <h2>Scripting I/O</h2>
      ${fnTable(scriptingIo)}
      ${callout(
        "note",
        "<p>A capability the host does not provide reports <code>E5002</code>. In the Playground, filesystem, clock, sleep, and HTTP are unavailable; stdout, stdin, and args work. See <a href='" +
          url("docs/guide-io/", base) +
          "'>I/O and arguments</a>.</p>",
      )}

      <h2>Methods</h2>
      <p>Methods are called on a receiver; a method access without parentheses
      is a zero-argument call.</p>
      ${methodTable}

      <h2>Modules and capabilities</h2>
      <p><code>json</code>, <code>regex</code>, and <code>time</code> are
      enabled in the default feature set. <code>http</code> is
      <strong>native-only and not default</strong>; network access is
      host-owned, and the browser Playground has no HTTP authority.</p>
      ${moduleTable}
      <p>The typed JSON API is <code>json_decode_as(text, Type)</code> — the
      second argument is a canonical <strong>type</strong>, not a string. It is
      strict: a missing, wrong, or unknown struct field, or an array-length
      mismatch, is <code>E4031</code>.</p>
    </div>

    <div style="margin-top:var(--au-space-8)">
      ${codeBlock({
        source: `fn main() {
    let scores = [88, 42, 95, 67, 71]
    let passing = scores.filter((s) -> s >= 70)
    print(passing.sort())
    print(f"average = {scores.reduce((a, b) -> a + b, 0) / len(scores)}")

    let m = {"name": "Aura", "kind": "language"}
    print(json_encode(m))
    print(regex_find_all("[0-9]+", "version 0.3.1"))
}`,
        title: "stdlib.aura",
        runnable: true,
        base,
      })}
    </div>

    <a class="eyebrow-link" href="${url("docs/reference-stdlib/", base)}">Full reference ${icon("arrow")}</a>
  </div>
</section>`;
  },
};
