import { url, codeBlock, pageHead, pipeline, dataTable, callout } from "../lib/components.mjs";

export const architecturePage = {
  key: "architecture",
  title: "Architecture",
  path: "architecture/",
  activeKey: "architecture",
  description:
    "How Aura is built: the front-end pipeline, the interpreter, the host boundary, and the way native and WebAssembly execution share one runtime.",
  async render(base) {
    return `${pageHead({
      eyebrow: "Architecture",
      title: "How Aura is built",
      lede: "A single pipeline shared by every entry point, a host boundary that isolates the outside world, and one runtime that runs natively or on WebAssembly.",
    })}

<section class="section section--tight">
  <div class="container">
    <div class="grid grid--2">
      <div>
        <h2>Source to execution</h2>
        ${pipeline([
          { title: "Aura source", body: "UTF-8 text." },
          { title: "Lexer", body: "Produces tokens; reports <code>E1xxx</code>." },
          { title: "Parser", body: "Builds the AST; enforces the nesting limit (<code>E1015</code>)." },
          { title: "Checker", body: "Names, mutability, declarations, provable types (<code>E2xxx</code>/<code>E3xxx</code>)." },
          { title: "Interpreter", body: "Walks the AST; runtime diagnostics are <code>E4xxx</code>." },
          { title: "Host", body: "The single boundary to the outside world." },
        ])}
      </div>
      <div class="prose">
        <h2>One front end</h2>
        <p>Every entry point — library, CLI, REPL, and the browser runtime —
        runs the same <code>lex → parse → check → execute</code> pipeline.
        <code>compile</code> takes a mode: <code>module</code> requires no entry
        point, <code>program</code> requires <code>fn main</code>. “What is a
        valid program” is defined in exactly one place.</p>
        <h2>One shared registry</h2>
        <p>The checker and the runtime consult the same signature registry for
        builtin arity, argument types, method existence, and return types, so the
        two layers cannot disagree about the standard library.</p>
      </div>
    </div>
  </div>
</section>

<section class="section section--alt">
  <div class="container">
    <div class="section__head">
      <span class="section__eyebrow">The host</span>
      <h2>One boundary, two implementations</h2>
      <p class="section__lede">Every language-visible interaction with the
      outside world goes through the host the interpreter owns.</p>
    </div>
    <div class="grid grid--2">
      <div class="card">
        <h3>Native <code>StdHost</code></h3>
        <p>Real process stdout, filesystem, clock, and sleep. Used by the CLI,
        the REPL, and the library. Native execution runs the interpreter on a
        dedicated 64 MiB stack.</p>
      </div>
      <div class="card">
        <h3>Browser <code>BrowserHost</code></h3>
        <p>In-memory stdout, supplied stdin, plain-data arguments. No
        filesystem, clock, sleep, DOM, network, storage, or JS handle. Used by
        the WebAssembly runtime inside a Web Worker.</p>
      </div>
    </div>
    ${callout(
      "note",
      "<p>A host carries only plain data — strings, integers, bytes. It never exposes JavaScript, the DOM, the network, process execution, environment variables, or native handles.</p>",
    )}
  </div>
</section>

<section class="section">
  <div class="container">
    <div class="grid grid--2">
      <div>
        ${codeBlock({
          source: `Aura source
    │
    ▼
  lexer ──▶ tokens
    │
    ▼
  parser ─▶ AST
    │
    ▼
  checker ─▶ checked AST
    │
    ▼
  interpreter
    │
    ▼            (host boundary)
  Host
    ├── Native  / StdHost
    └── Browser / BrowserHost`,
          title: "pipeline.txt",
          base,
        })}
      </div>
      <div class="prose">
        <h2>Execution substrates</h2>
        ${dataTable(
          ["Substrate", "Stack", "Threads"],
          [
            ["Native", "dedicated 64 MiB", "one execution thread"],
            ["WebAssembly", "engine stack", "none (<code>thread::spawn</code> unavailable)"],
          ],
        )}
        <h2>Why this shape</h2>
        <p>Putting every outside-world operation behind one trait means the
        language semantics are identical on every substrate; only capability
        availability differs. The browser runtime is not a separate
        implementation — it is the same interpreter with a different host.</p>
      </div>
    </div>
  </div>
</section>

<section class="section section--alt">
  <div class="container">
    <div class="section__head">
      <span class="section__eyebrow">Repository layout</span>
      <h2>Where things live</h2>
    </div>
    ${dataTable(
      ["Path", "Contents"],
      [
        ["<code>src/lex</code>, <code>src/parse</code>, <code>src/ast</code>", "front end"],
        ["<code>src/check</code>", "the conservative checker"],
        ["<code>src/run</code>", "the explicit-continuation evaluator and values"],
        ["<code>src/stdlib</code>", "builtins, methods, and the signature registry"],
        ["<code>src/host.rs</code>", "the host contract and native/limited hosts"],
        ["<code>playground/runtime</code>", "the WebAssembly runtime wrapper"],
        ["<code>playground/web</code>", "the Playground Worker and loader"],
        ["<code>website</code>", "this website's generator and tests"],
      ],
    )}
    <a class="eyebrow-link" href="${url("tools/", base)}">Tools &amp; CLI →</a>
  </div>
</section>

<section class="section">
  <div class="container">
    <div class="section__head">
      <span class="section__eyebrow">Semantic boundary</span>
      <h2>The compiler, AIS, and MCP</h2>
      <p class="section__lede">The compiler is the single semantic authority.
      AIS/0.1 is a stable projection of what it proved; the MCP adapter
      transports that projection. Neither defines meaning, and neither reaches
      past the host boundary.</p>
    </div>
    <div class="grid grid--2">
      <div>
        ${codeBlock({
          source: `Aura source
    │
    ▼
lexer / parser / checker / module system
    │
    ▼
compiler semantic model      ← authority
    │
    ├─ DX  (human)   → colored diagnostics, REPL, Playground
    │
    └─ AIX (machine) → AIS/0.1
                          ├── aura ais   (snapshot / slice / delta)
                          └── aura mcp   (JSON-RPC 2.0 transport)

Neither AIS nor MCP grants a Host capability.`,
          title: "semantics.txt",
          base,
        })}
      </div>
      <div class="prose">
        <h2>Two projections of one truth</h2>
        <p>The human <strong>DX</strong> renderer and the machine <strong>AIX</strong>
        payload read the same semantic facts. A diagnostic's colour and prose are
        a rendering; the structured code, severity, and range are the fact. A
        tooling consumer reads the fact, never the prose.</p>
        <h2>Transport, not authority</h2>
        <p><a href="${url("docs/ais/", base)}">AIS/0.1</a> is the machine
        projection the compiler emits. The
        <a href="${url("docs/mcp/", base)}">MCP adapter</a> is a transport over
        the same data: removing it would change no Aura semantics. The
        dependency is one-way — <code>src/ais.rs</code> does not depend on
        <code>src/mcp.rs</code>.</p>
        <h2>Outside the host boundary</h2>
        <p>Producing or consuming AIS grants no filesystem, network, Python,
        secret, environment, or Host authority. AIS describes a program the
        caller already submitted; it does not read the world on the caller's
        behalf.</p>
      </div>
    </div>
  </div>
</section>`;
  },
};
