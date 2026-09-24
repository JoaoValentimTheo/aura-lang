import {
  url,
  icon,
  codeBlock,
  pageHead,
  dataTable,
  statusList,
} from "../lib/components.mjs";

export const languagePage = {
  key: "language",
  title: "Language",
  path: "language/",
  activeKey: "language",
  description:
    "An overview of the Aura language: values, functions, data, pattern matching, errors, and design choices.",
  async render(base) {
    return `${pageHead({
      eyebrow: "Language",
      title: "The Aura language",
      lede: "A small, dynamically-typed, expression-oriented language with optional annotations and a conservative checker. One spelling per construct.",
    })}

<section class="section section--tight">
  <div class="container">
    <div class="grid grid--2">
      <div class="prose">
        <h2>Design choices</h2>
        <p>Aura makes a small set of deliberate choices and keeps them
        consistent from the lexer to the runtime.</p>
        <ul>
          <li><strong>One spelling per construct.</strong> <code>fn</code> for
          functions; <code>and</code>/<code>or</code>/<code>not</code> for logic;
          <code>none</code> for absence.</li>
          <li><strong>Immutable by default.</strong> <code>let</code> binds
          immutably; <code>let mut</code> opts into reassignment.</li>
          <li><strong>Expression-oriented.</strong> <code>if</code>,
          <code>match</code>, and blocks yield values.</li>
          <li><strong>Conservative checking.</strong> Reject only what can be
          proven wrong; unknown types fall to the runtime.</li>
          <li><strong>Stable diagnostics.</strong> Every rejection is a stable
          <code>E####</code> code.</li>
          <li><strong>Explicit capabilities.</strong> All outside-world access
          goes through the host.</li>
        </ul>
      </div>
      <div>
        ${codeBlock({
          source: `fn classify(n) {
    if n < 0 {
        return "negative"
    } else if n == 0 {
        return "zero"
    } else {
        return "positive"
    }
}

fn main() {
    print(classify(-3))
    print(classify(0))
    print(classify(7))
}`,
          title: "classify.aura",
          runnable: true,
          base,
        })}
      </div>
    </div>
  </div>
</section>

<section class="section section--alt">
  <div class="container">
    <div class="section__head">
      <span class="section__eyebrow">Values</span>
      <h2>The value universe</h2>
      <p class="section__lede">Eleven value kinds, all first-class within their rules.</p>
    </div>
    ${dataTable(
      ["Kind", "Description", "Mutable"],
      [
        ["<code>int</code>", "signed 64-bit integer", "value"],
        ["<code>float</code>", "IEEE-754 binary64", "value"],
        ["<code>bool</code>", "<code>true</code> / <code>false</code>", "value"],
        ["<code>string</code>", "immutable UTF-8 text", "value"],
        ["<code>none</code>", "absence", "value"],
        ["<code>list</code>", "ordered, reference semantics", "shared"],
        ["<code>map</code>", "string-keyed, ordered by key", "shared"],
        ["<code>struct</code>", "named fields in declaration order", "shared fields"],
        ["<code>enum</code>", "tag + positional payload", "value"],
        ["<code>fn</code>", "closure or native", "value"],
        ["<code>range</code>", "start..end, step 1", "value"],
      ],
    )}
  </div>
</section>

<section class="section">
  <div class="container">
    <div class="section__head">
      <span class="section__eyebrow">Data</span>
      <h2>Structs, enums, and matching</h2>
    </div>
    <div class="grid grid--2">
      <div>
        ${codeBlock({
          source: `struct Point { x: int, y: int }

enum Shape {
    Circle(int),
    Rectangle(int, int),
}

fn area(s) -> int {
    return match s {
        Circle(r) -> 3 * r * r
        Rectangle(w, h) -> w * h
    }
}

fn main() {
    let p = Point { x: 3, y: 4 }
    print(p.x * p.x + p.y * p.y)
    print(area(Circle(2)))
    print(area(Rectangle(3, 4)))
}`,
          title: "shapes.aura",
          runnable: true,
          base,
        })}
      </div>
      <div class="prose">
        <h3>Records with types</h3>
        <p>Struct fields carry annotations that are validated and enforced where
        provable.</p>
        <h3>Tagged unions</h3>
        <p>Enum variants carry positional payloads, constructed by name and
        destructured by <code>match</code>.</p>
        <h3>Patterns</h3>
        <p>Patterns cover literals, bindings, list arity, and variant payloads,
        with optional guards.</p>
        <a class="eyebrow-link" href="${url("docs/guide-data/", base)}">Data guide ${icon("arrow")}</a>
      </div>
    </div>
  </div>
</section>

<section class="section section--alt">
  <div class="container">
    <div class="grid grid--2">
      <div class="prose">
        <h2>Functional core</h2>
        <p>Functions are first-class, closures capture by reference, and the
        pipeline operator threads a value through calls. Methods transform
        collections without mutating the original.</p>
        <a class="eyebrow-link" href="${url("docs/guide-functions/", base)}">Functions guide ${icon("arrow")}</a>
        <h2>Errors</h2>
        <p>Only an explicit <code>throw</code> is catchable. Runtime diagnostics
        such as division by zero are fatal and not catchable, so ordinary
        “not found” cases stay values rather than exceptions.</p>
        <a class="eyebrow-link" href="${url("docs/guide-errors/", base)}">Errors guide ${icon("arrow")}</a>
      </div>
      <div>
        ${codeBlock({
          source: `fn main() {
    let xs = [1, 2, 3, 4, 5, 6]
    let result = xs |> filter((x) -> x % 2 == 0) |> map((x) -> x * x)
    print(result)

    try {
        throw "boom"
    } catch e -> {
        print(f"caught {e}")
    } finally {
        print("done")
    }
}`,
          title: "pipeline.aura",
          runnable: true,
          base,
        })}
      </div>
    </div>
  </div>
</section>

<section class="section">
  <div class="container">
    <div class="section__head">
      <span class="section__eyebrow">Status</span>
      <h2>What the language includes today</h2>
    </div>
    ${statusList([
      ["Functions, closures, recursion", "Implemented", "success"],
      ["Structs, enums, type aliases", "Implemented", "success"],
      ["Pattern matching and destructuring", "Implemented", "success"],
      ["Lists, maps, ranges, pipeline, methods", "Implemented", "success"],
      ["Errors: throw / try / catch / finally", "Implemented", "success"],
      ["Optional type annotations + conservative checker", "Implemented", "success"],
      ["Scripting I/O and arguments", "Implemented", "success"],
      ["Classes, inheritance, traits, generics", "Planned", "planned"],
      ["Modules and visibility", "Reserved (inert syntax)", "planned"],
      ["Python / PyO3 interop", "Long-term", "planned"],
    ])}
    <div style="margin-top:var(--space-6)">
      <a class="btn btn--filled" href="${url("docs/reference-grammar/", base)}">Grammar reference</a>
      <a class="btn btn--outlined" href="${url("examples/", base)}">See examples</a>
    </div>
  </div>
</section>`;
  },
};
