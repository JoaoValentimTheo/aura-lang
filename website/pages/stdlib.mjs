import { url, icon, codeBlock, pageHead, dataTable, callout } from "../lib/components.mjs";

export const stdlibPage = {
  key: "stdlib",
  title: "Standard Library",
  path: "stdlib/",
  activeKey: "stdlib",
  description:
    "Aura's standard library: core functions, collection functions, methods, scripting I/O, and the json, regex, and time modules.",
  async render(base) {
    const core = dataTable(
      ["Function", "Signature", "Returns"],
      [
        ["<code>print</code>", "<code>print(...)</code>", "writes a line to stdout"],
        ["<code>len</code>", "<code>len(x)</code>", "<code>int</code>"],
        ["<code>to_string</code>", "<code>to_string(x)</code>", "<code>string</code>"],
        ["<code>to_int</code>", "<code>to_int(x)</code>", "<code>int</code>"],
        ["<code>to_float</code>", "<code>to_float(x)</code>", "<code>float</code>"],
        ["<code>range</code>", "<code>range(n)</code> / <code>range(a, b)</code>", "<code>range</code>"],
        ["<code>abs</code>", "<code>abs(n)</code>", "number"],
        ["<code>min</code> / <code>max</code>", "<code>min(a, b)</code>", "the lesser / greater"],
        ["<code>assert</code>", "<code>assert(cond[, msg])</code>", "<code>none</code> / <code>E4028</code>"],
      ],
    );
    const collections = dataTable(
      ["Function", "Signature", "Returns"],
      [
        ["<code>push</code>", "<code>push(list, v)</code>", "<code>none</code>"],
        ["<code>keys</code> / <code>values</code>", "<code>keys(map)</code>", "<code>[string]</code> / <code>[T]</code>"],
        ["<code>sort</code> / <code>reverse</code>", "<code>sort(list)</code>", "list"],
        ["<code>map</code> / <code>filter</code>", "<code>map(list, f)</code>", "<code>[T]</code>"],
        ["<code>reduce</code>", "<code>reduce(list, f, init)</code>", "accumulated"],
        ["<code>sum</code>", "<code>sum(list)</code>", "number"],
        ["<code>enumerate</code> / <code>zip</code>", "<code>zip(a, b)</code>", "list of lists"],
      ],
    );
    const io = dataTable(
      ["Function", "Signature", "Returns"],
      [
        ["<code>read_line</code>", "<code>read_line()</code>", "<code>string \\| none</code>"],
        ["<code>read_file</code>", "<code>read_file(path)</code>", "<code>string \\| none</code>"],
        ["<code>write_file</code>", "<code>write_file(path, text)</code>", "<code>none</code>"],
        ["<code>args</code>", "<code>args()</code>", "<code>[string]</code>"],
      ],
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
      ${core}
      <h2>Collection functions</h2>
      ${collections}
      <h2>Scripting I/O</h2>
      ${io}
      ${callout(
        "info",
        "<p>A capability the host does not provide reports <code>E5002</code>. In the Playground, filesystem, clock, and sleep are unavailable; stdout, stdin, and args work. See <a href='" +
          url("docs/guide-io/", base) +
          "'>I/O and arguments</a>.</p>",
      )}

      <h2>Methods</h2>
      <p>Methods are called on a receiver; a method access without parentheses
      is a zero-argument call.</p>
      ${dataTable(
        ["Receiver", "Methods"],
        [
          ["string", "<code>len</code> <code>upper</code> <code>lower</code> <code>trim</code> <code>contains</code> <code>starts_with</code> <code>ends_with</code> <code>split</code> <code>replace</code> <code>chars</code>"],
          ["list", "<code>len</code> <code>push</code> <code>pop</code> <code>first</code> <code>last</code> <code>join</code> <code>contains</code> <code>sort</code> <code>reverse</code> <code>map</code> <code>filter</code> <code>reduce</code>"],
          ["map", "<code>len</code> <code>get</code> <code>has</code> <code>keys</code> <code>values</code> <code>remove</code>"],
          ["range", "<code>len</code>"],
        ],
      )}

      <h2>Feature modules</h2>
      ${dataTable(
        ["Module", "Functions"],
        [
          ["<code>json</code>", "<code>json_encode</code> <code>json_decode</code>"],
          ["<code>regex</code>", "<code>regex_match</code> <code>regex_find</code> <code>regex_find_all</code> <code>regex_replace</code>"],
          ["<code>time</code>", "<code>time_unix</code> <code>time_now</code> <code>sleep_ms</code>"],
        ],
      )}
    </div>

    <div style="margin-top:var(--space-8)">
      ${codeBlock({
        source: `fn main() {
    let scores = [88, 42, 95, 67, 71]
    let passing = scores.filter((s) -> s >= 70)
    print(passing.sort())
    print(f"average = {scores.reduce((a, b) -> a + b, 0) / len(scores)}")

    let m = {"name": "Aura", "kind": "language"}
    print(json_encode(m))
    print(regex_find_all("[0-9]+", "version 0.0.2"))
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
