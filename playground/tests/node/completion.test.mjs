// Completion and language-metadata unit tests.
//
// These exercise the presentational completion surface and the shared language
// inventory. No Aura execution and no DOM: the completion module is pure.

import assert from "node:assert/strict";
import { candidates, wordAt } from "../../web/completion.js";
import * as meta from "../../web/language.js";

let passed = 0;
function test(name, fn) {
  try {
    fn();
    passed += 1;
  } catch (err) {
    console.error(`FAIL: ${name}`);
    throw err;
  }
}

test("wordAt reads the fragment before the caret", () => {
  assert.equal(wordAt("pr", 2), "pr");
  assert.equal(wordAt("let pri", 7), "pri");
  assert.equal(wordAt("print(", 6), "");
});

test("typing a prefix offers the matching builtin", () => {
  const items = candidates("pri", 3).map((c) => c.label);
  assert.ok(items.includes("print"), `expected print, got ${items.join(", ")}`);
});

test("map methods are offered after a dot", () => {
  const items = candidates("m.", 2).map((c) => c.label);
  for (const m of ["get", "has", "keys", "values", "items", "remove", "len"]) {
    assert.ok(items.includes(m), `expected map method ${m}`);
  }
  // A dot position offers no keywords.
  assert.ok(!items.includes("fn"));
});

test("identifier position offers keywords and types", () => {
  const items = candidates("f", 1).map((c) => c.label);
  assert.ok(items.includes("fn"));
  assert.ok(items.includes("float"));
});

test("buffer names are suggested for a non-empty fragment", () => {
  const items = candidates("myCounter myC", 12).map((c) => c.label);
  assert.ok(items.includes("myCounter"));
});

test("the shared metadata has no duplicates", () => {
  for (const key of ["KEYWORDS", "LITERALS", "TYPES", "BUILTINS", "METHODS", "CONTEXTUAL"]) {
    const list = meta[key];
    assert.equal(new Set(list).size, list.length, `${key} has duplicates`);
  }
});

test("new Core words are present in the metadata", () => {
  // `items` (map method) and the module words must be known to the front end.
  assert.ok(meta.METHODS.includes("items"));
  assert.ok(meta.CONTEXTUAL.includes("module"));
  assert.ok(meta.CONTEXTUAL.includes("impl"));
  assert.ok(meta.CONTEXTUAL.includes("trait"));
  assert.ok(meta.CONTEXTUAL.includes("const"));
  assert.ok(meta.CONTEXTUAL.includes("self"));
});

console.log(`completion: ${passed} passed, 0 failed`);
