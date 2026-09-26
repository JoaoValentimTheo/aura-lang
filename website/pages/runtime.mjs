import * as C from "../lib/components.mjs";
const { url, icon, codeBlock, pageHead, pipeline, dataTable, callout } = C;

export const runtimePage = {
  key: "runtime",
  title: "Runtime",
  path: "runtime/",
  activeKey: "runtime",
  description:
    "How the Aura runtime works: the host boundary, native and browser hosts, execution substrates, and versioned WebAssembly artifacts.",
  async render(base) {
    return `${pageHead({
      eyebrow: "Runtime",
      title: "One interpreter, many substrates",
      lede: "The Aura runtime is a tree-walking interpreter that talks to the outside world only through a host. That single boundary lets the same code run from the CLI and in a browser.",
    })}

<section class="section section--tight">
  <div class="container">
    <div class="grid grid--2">
      <div>
        <h2>The pipeline</h2>
        ${pipeline([
          { title: "Lex", body: "Source (UTF-8) is tokenized. Lexical errors are <code>E1xxx</code>." },
          { title: "Parse", body: "Tokens become an AST. Syntax and nesting errors are <code>E1xxx</code>." },
          { title: "Check", body: "Names, mutability, declarations, and provable types are validated (<code>E2xxx</code>/<code>E3xxx</code>)." },
          { title: "Execute", body: "The AST is walked. Runtime failures are <code>E4xxx</code>; missing capabilities are <code>E5002</code>." },
        ])}
      </div>
      <div class="prose">
        <h2>The host boundary</h2>
        <p>The interpreter owns a <code>Host</code>. Standard output, input,
        arguments, filesystem, clock, and sleep all route through it. The
        standard library never calls the operating system directly.</p>
        <ul>
          <li>A capability the host lacks is <code>E5002</code>.</li>
          <li>A genuine failure of a provided capability is <code>E4020</code>.</li>
          <li>A missing file where the filesystem exists is <code>none</code>.</li>
        </ul>
        <a class="eyebrow-link" href="${url("docs/runtime-doc/", base)}">Runtime reference ${icon("arrow")}</a>
      </div>
    </div>
  </div>
</section>

<section class="section section--alt">
  <div class="container">
    <div class="section__head">
      <span class="section__eyebrow">Hosts</span>
      <h2>Native and browser</h2>
    </div>
    ${dataTable(
      ["Capability", "Native (StdHost)", "Browser (BrowserHost)"],
      [
        ["stdout", "process stdout", "in-memory buffer"],
        ["stdin", "process stdin", "supplied string"],
        ["args", "process arguments", "plain data"],
        ["filesystem", "real files", "<code>E5002</code>"],
        ["clock", "real clock", "<code>E5002</code>"],
        ["sleep", "real sleep", "<code>E5002</code>"],
      ],
    )}
    ${callout(
      "note",
      "<p>The browser host exposes no DOM, network, storage, or JavaScript handle. Sleep is never busy-waited.</p>",
    )}
  </div>
</section>

<section class="section">
  <div class="container">
    <div class="grid grid--2">
      <div class="prose">
        <h2>Execution substrates</h2>
        <p>On native targets the interpreter runs on a dedicated 64 MiB stack,
        so the language's own recursion limit (<code>E4011</code>) is reached
        before any host-stack limit. In the browser runtime it runs inline on
        the engine stack; the parser's grouping backstop is calibrated per
        substrate so over-deep input is always <code>E1015</code>, never a host
        failure.</p>
        <h2>Versioned artifacts</h2>
        <p>Each runtime build is an immutable artifact identified by version and
        SHA-256. A version entry records the language version, artifact, hash,
        Playground API version, and Host ABI version. Selecting a version in the
        Playground selects exactly the artifact that executes; historical
        artifacts are never silently replaced.</p>
      </div>
      <div>
        ${codeBlock({
          source: `# The browser runtime is loaded from an immutable,
# versioned artifact. Its manifest records, per entry:
#
#   id                 0.0.2            (published release)
#   channel            release
#   language_version   0.0.1
#   release_version    0.0.2
#   host_abi_version   1
#   artifact           0.0.2/aura_playground_runtime.wasm
#   sha256             5a4ad3f7…34ed
#
#   id                 0.0.2-dev.4      (development runtime)
#   channel            development
#   release_version    0.0.2
#   host_abi_version   1
#
# Published release artifacts are frozen: their hashes are
# pinned and never rewritten. A newer runtime is added as a
# new entry, never substituted into an existing one. The
# loader refuses any artifact that declares imports, so the
# runtime can never gain host authority.`,
          title: "manifest.txt",
          base,
        })}
      </div>
    </div>
  </div>
</section>

<section class="section section--alt">
  <div class="container">
    <div class="section__head">
      <span class="section__eyebrow">WASM</span>
      <h2>How the browser runs Aura</h2>
    </div>
    <div class="grid grid--3">
      <div class="card"><h3>Zero imports</h3><p>The WebAssembly module imports nothing — no WASI, no JS. It can reach only its own linear memory.</p></div>
      <div class="card"><h3>Worker isolation</h3><p>Every run happens in a fresh Web Worker, isolated from the UI thread and terminated on Stop.</p></div>
      <div class="card"><h3>Structured results</h3><p>Diagnostics cross the boundary as structured data with stable codes and source positions.</p></div>
    </div>
    <div class="hero__actions" style="margin-top:var(--space-8)">
      <a class="btn btn--filled" href="${url("playground/", base)}">${icon("play")} Open the Playground</a>
      <a class="btn btn--outlined" href="${url("architecture/", base)}">Architecture</a>
    </div>
  </div>
</section>`;
  },
};
