import { url, icon, codeBlock, pageHead, dataTable, callout } from "../lib/components.mjs";

export const toolsPage = {
  key: "tools",
  title: "Tools & CLI",
  path: "tools/",
  activeKey: "tools",
  description:
    "Aura's command-line tool, REPL, editor-agnostic diagnostics, and the browser Playground.",
  async render(base) {
    return `${pageHead({
      eyebrow: "Tools",
      title: "Tools and CLI",
      lede: "Aura ships a command-line interpreter, a persistent REPL, machine-readable diagnostics, and a browser Playground — all built on the same front end.",
    })}

<section class="section section--tight">
  <div class="container">
    <div class="grid grid--2">
      <div class="prose">
        <h2>The <code>aura</code> command</h2>
        ${dataTable(
          ["Command", "Purpose"],
          [
            ["<code>aura run &lt;file&gt;</code>", "parse, check, and execute (stdin with <code>-</code>)"],
            ["<code>aura check &lt;file&gt;</code>", "parse and check without running"],
            ["<code>aura eval &lt;code&gt;</code>", "run a one-liner"],
            ["<code>aura repl</code>", "an interactive session"],
            ["<code>aura version</code>", "print the version"],
          ],
        )}
        <p>Diagnostics print to standard error with a
        <code>file:line:column:</code> prefix and the stable code:</p>
        <pre tabindex="0" role="region" aria-label="Diagnostic example"><code>program.aura:3:12: E3001: type mismatch …</code></pre>
      </div>
      <div>
        ${codeBlock({
          source: `$ aura run examples/tour.aura
Aura: sum 1..5 = 15
fib(10) = 55
...

$ aura run greet.aura Ada
hello Ada

$ echo "world" | aura run upper.aura
WORLD

$ aura check my_program.aura
$ aura eval 'print(1 + 2)'
3`,
          title: "shell",
          base,
        })}
      </div>
    </div>
  </div>
</section>

<section class="section section--alt">
  <div class="container">
    <div class="section__head">
      <span class="section__eyebrow">Tooling</span>
      <h2>What exists, and what does not</h2>
      <p class="section__lede">Aura's tooling is honest about its current
      surface. Nothing below is advertised unless it exists.</p>
    </div>
    ${dataTable(
      ["Tool", "Status"],
      [
        ["<code>aura</code> CLI (run / check / eval / repl / version)", "Available"],
        ["Persistent REPL", "Available"],
        ["Stable <code>E####</code> diagnostics", "Available"],
        ["Scripting I/O and arguments", "Available"],
        ["Browser Playground (WebAssembly, multi-file projects)", "Available"],
        ["AIS/0.1 semantic interface (<code>aura ais</code>)", "Available"],
        ["MCP adapter over stdio (<code>aura mcp</code>)", "Available"],
        ["Language server / LSP", "Not yet"],
        ["Debugger / step execution", "Not yet"],
        ["Package manager", "Not yet"],
        ["Formatter", "Not yet"],
        ["Editor plugins", "Not yet"],
      ],
    )}
    <div class="hero__actions" style="margin-top:var(--au-space-6)">
      <a class="btn btn--filled" href="${url("playground/", base)}">${icon("play")} Open the Playground</a>
      <a class="btn btn--outlined" href="${url("docs/ais/", base)}">AIS documentation</a>
      <a class="btn btn--outlined" href="${url("docs/mcp/", base)}">MCP adapter</a>
      <a class="btn btn--text" href="${url("docs/cli/", base)}">CLI reference</a>
    </div>
    ${callout(
      "note",
      "<p><strong>Semantic tooling.</strong> AIS/0.1 exposes the compiler's proved semantic facts as stable machine-readable JSON; the MCP adapter transports that same data over JSON-RPC 2.0. Neither is a second compiler or a model provider — the compiler stays the semantic authority. Both require the <code>json</code> feature (default).</p>",
    )}
  </div>
</section>`;
  },
};
