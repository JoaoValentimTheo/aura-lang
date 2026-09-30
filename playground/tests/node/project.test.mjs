// Project-state unit tests.
//
// These exercise the Playground's multi-file *state* model with no DOM, no
// Worker, and no wasm: `web/project.js` is a pure ES module. The model holds
// only files, an active key, and an entry key — Aura module resolution is not
// its business and is never asserted here.
//
// Usage: node playground/tests/node/project.test.mjs

import assert from "node:assert/strict";
import {
  KEY_PATTERN,
  MAX_PROJECT_SOURCES,
  Project,
  createKeyGenerator,
  highestKeyCounter,
  isValidSourceName,
  utf8Length,
} from "../../web/project.js";

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

// --- initial state --------------------------------------------------------

test("the default project is exactly the historical single source", () => {
  const p = Project.default();
  assert.equal(p.size, 1);
  assert.equal(p.sources[0].name, "main.aura");
  assert.match(p.sources[0].text, /fn main\(\)/);
  assert.equal(p.activeKey, p.sources[0].key);
  assert.equal(p.entryKey, p.sources[0].key);
  assert.ok(p.isSingleSource());
});

test("every generated key satisfies the runtime's virtual-key grammar", () => {
  const p = Project.default();
  for (let i = 0; i < 8; i += 1) {
    const r = p.createSource();
    assert.equal(r.ok, true);
    assert.ok(KEY_PATTERN.test(r.key), `key ${r.key} is not a valid provider key`);
  }
  for (const s of p.sources) assert.ok(KEY_PATTERN.test(s.key));
});

// --- create ---------------------------------------------------------------

test("creating a second file keeps the first and activates the new one", () => {
  const p = Project.default();
  const first = p.sources[0].key;
  const r = p.createSource({ name: "helper.aura", text: "pub fn v() -> int { return 1 }" });
  assert.equal(r.ok, true);
  assert.equal(p.size, 2);
  assert.equal(p.activeKey, r.key);
  assert.equal(p.entryKey, first, "the entry is unchanged by adding a file");
  assert.equal(p.get(first).text, Project.default().sources[0].text);
  assert.equal(p.isSingleSource(), false);
});

test("a default-named new file never collides with an existing name", () => {
  const p = Project.default();
  const a = p.createSource();
  const b = p.createSource();
  assert.equal(a.ok, true);
  assert.equal(b.ok, true);
  const names = p.sources.map((s) => s.name);
  assert.equal(new Set(names).size, names.length, `names not unique: ${names.join(", ")}`);
  assert.ok(names.includes("main-2.aura"));
  assert.ok(names.includes("main-3.aura"));
});

test("creating a duplicate name is rejected and changes nothing", () => {
  const p = Project.default();
  p.createSource({ name: "helper.aura" });
  const before = p.snapshot();
  const r = p.createSource({ name: "main.aura" });
  assert.equal(r.ok, false);
  assert.match(r.error, /already exists/);
  assert.deepEqual(p.sources, before.sources);
  assert.equal(p.activeKey, before.activeKey);
});

test("an invalid or oversized name is rejected", () => {
  const p = Project.default();
  assert.equal(p.createSource({ name: "" }).ok, false);
  assert.equal(p.createSource({ name: "a\nb" }).ok, false);
  assert.equal(p.createSource({ name: "x".repeat(2000) }).ok, false);
  assert.equal(p.size, 1);
});

// --- edit + switch --------------------------------------------------------

test("edits survive switching between files", () => {
  const p = Project.default();
  const first = p.activeKey;
  const second = p.createSource({ name: "helper.aura" }).key;
  p.setText(second, "pub fn helper() -> int { return 2 }");
  assert.equal(p.selectSource(first).ok, true);
  assert.equal(p.active(), p.get(first));
  assert.match(p.active().text, /hello, Aura/);
  assert.equal(p.selectSource(second).ok, true);
  assert.equal(p.active().text, "pub fn helper() -> int { return 2 }");
});

test("selecting an unknown file is rejected without changing the active file", () => {
  const p = Project.default();
  const before = p.activeKey;
  assert.equal(p.selectSource("nope").ok, false);
  assert.equal(p.activeKey, before);
});

test("setText on an unknown key is a no-op", () => {
  const p = Project.default();
  const before = p.snapshot();
  assert.equal(p.setText("nope", "x"), false);
  assert.deepEqual(p.sources, before.sources);
});

// --- rename ---------------------------------------------------------------

test("rename changes the display name and preserves the opaque key", () => {
  const p = Project.default();
  const key = p.activeKey;
  const r = p.renameSource(key, "entry.aura");
  assert.equal(r.ok, true);
  assert.equal(p.get(key).name, "entry.aura");
  assert.equal(p.activeKey, key, "the key is untouched by rename");
  assert.equal(p.entryKey, key);
  assert.equal(p.size, 1);
});

test("rename to an existing name is rejected", () => {
  const p = Project.default();
  p.createSource({ name: "helper.aura" });
  const r = p.renameSource(p.sources[1].key, "main.aura");
  assert.equal(r.ok, false);
  assert.match(r.error, /already exists/);
  assert.equal(p.sources[1].name, "helper.aura");
});

test("rename to the same name is a harmless success", () => {
  const p = Project.default();
  assert.equal(p.renameSource(p.activeKey, "main.aura").ok, true);
  assert.equal(p.size, 1);
});

test("a rename can never rewrite a source's text or its key", () => {
  const p = Project.default();
  p.createSource({ name: "helper.aura", text: "pub fn helper() -> int { return 2 }" });
  const child = p.sources[1];
  const beforeText = child.text;
  const beforeKey = child.key;
  assert.equal(p.renameSource(child.key, "renamed.aura").ok, true);
  assert.equal(p.get(beforeKey).text, beforeText);
  assert.equal(p.get(beforeKey).key, beforeKey);
  // Renaming is display-only: no source text was touched.
  assert.equal(p.get(beforeKey).text, "pub fn helper() -> int { return 2 }");
});

// --- delete ---------------------------------------------------------------

test("deleting a non-active source leaves the active file alone", () => {
  const p = Project.default();
  const first = p.activeKey;
  const second = p.createSource({ name: "helper.aura" }).key;
  assert.equal(p.selectSource(first).ok, true);
  const r = p.deleteSource(second);
  assert.equal(r.ok, true);
  assert.equal(p.size, 1);
  assert.equal(p.activeKey, first);
  assert.equal(p.entryKey, first);
});

test("deleting the active source activates a deterministic survivor", () => {
  const p = Project.default();
  const first = p.activeKey;
  const second = p.createSource({ name: "b.aura" }).key;
  const third = p.createSource({ name: "c.aura" }).key;
  assert.equal(p.activeKey, third);
  const r = p.deleteSource(third);
  assert.equal(r.ok, true);
  assert.equal(p.activeKey, second, "the previous file becomes active");
  assert.equal(p.entryKey, first, "the entry was untouched");
  assert.equal(p.deleteSource(second).ok, true);
  assert.equal(p.activeKey, first);
});

test("deleting the entry source reassigns the entry to the active source", () => {
  const p = Project.default();
  const first = p.entryKey;
  const second = p.createSource({ name: "helper.aura" }).key;
  assert.equal(p.activeKey, second);
  const r = p.deleteSource(first);
  assert.equal(r.ok, true);
  assert.equal(p.entryKey, second, "the entry follows the active survivor");
  assert.equal(p.activeKey, second);
  assert.equal(p.size, 1);
});

test("deleting the last remaining file is refused", () => {
  const p = Project.default();
  const r = p.deleteSource(p.activeKey);
  assert.equal(r.ok, false);
  assert.match(r.error, /at least one file/);
  assert.equal(p.size, 1);
});

test("deleting an unknown file is refused", () => {
  const p = Project.default();
  assert.equal(p.deleteSource("nope").ok, false);
  assert.equal(p.size, 1);
});

// --- entry ----------------------------------------------------------------

test("the entry can be moved to another file", () => {
  const p = Project.default();
  const first = p.entryKey;
  const second = p.createSource({ name: "helper.aura" }).key;
  assert.equal(p.setEntry(second).ok, true);
  assert.equal(p.entryKey, second);
  assert.equal(p.activeKey, second);
  assert.equal(p.setEntry("nope").ok, false);
  assert.equal(p.entryKey, second);
  assert.equal(p.setEntry(first).ok, true);
  assert.equal(p.entryKey, first);
});

// --- reset / load ---------------------------------------------------------

test("reset restores the canonical one-source project", () => {
  const p = Project.default();
  p.createSource({ name: "helper.aura", text: "x" });
  p.renameSource(p.sources[0].key, "renamed.aura");
  p.setEntry(p.sources[1].key);
  p.reset();
  assert.equal(p.size, 1);
  assert.equal(p.sources[0].name, "main.aura");
  assert.match(p.sources[0].text, /hello, Aura/);
  assert.equal(p.activeKey, p.sources[0].key);
  assert.equal(p.entryKey, p.sources[0].key);
});

test("loading files replaces the project atomically", () => {
  const p = Project.default();
  p.loadFiles([
    { name: "main.aura", text: "fn main() { print(app::greet()) }" },
    { name: "app.aura", text: 'pub fn greet() -> string { return "hi" }' },
  ]);
  assert.equal(p.size, 2);
  assert.deepEqual(p.sources.map((s) => s.name), ["main.aura", "app.aura"]);
  assert.equal(p.activeKey, p.sources[0].key);
  assert.equal(p.entryKey, p.sources[0].key);
});

test("loaded keys stay unique even when reloading repeatedly", () => {
  const p = Project.default();
  const files = [
    { name: "main.aura", text: "a" },
    { name: "b.aura", text: "b" },
  ];
  p.loadFiles(files);
  const keys1 = p.sources.map((s) => s.key);
  p.loadFiles(files);
  const keys2 = p.sources.map((s) => s.key);
  assert.deepEqual(keys1, keys2, "key generation is deterministic across loads");
  assert.equal(new Set(keys2).size, keys2.length);
});

// --- SourceKey strategy ---------------------------------------------------

test("keys are never derived from the display filename", () => {
  const p = Project.default();
  const r = p.createSource({ name: "s9.aura", text: "" });
  assert.equal(r.ok, true);
  assert.notEqual(r.key, "s9.aura");
  assert.ok(KEY_PATTERN.test(r.key));
  // A path-like or dot-containing filename must not leak into the key.
  const q = Project.default();
  const r2 = q.createSource({ name: "../weird\\name.aura", text: "" });
  assert.equal(r2.ok, true);
  assert.equal(r2.key.includes("/"), false);
  assert.equal(r2.key.includes("\\"), false);
  assert.equal(r2.key.includes("."), false);
});

test("keys stay stable across rename, edit, entry change, and delete of others", () => {
  const p = Project.default();
  const first = p.sources[0].key;
  const second = p.createSource({ name: "helper.aura" }).key;
  const third = p.createSource({ name: "extra.aura" }).key;
  p.renameSource(second, "renamed.aura");
  p.setText(second, "changed");
  p.setEntry(third);
  assert.equal(p.deleteSource(third).ok, true);
  assert.equal(p.get(first).key, first);
  assert.equal(p.get(second).key, second);
});

test("a restored key generator can never mint a duplicate key", () => {
  const p = Project.default();
  const second = p.createSource({ name: "helper.aura" }).key;
  // Simulate a snapshot/restore that hands back a stale counter.
  const restored = new Project({
    sources: p.sources.map((s) => ({ ...s })),
    activeKey: p.activeKey,
    entryKey: p.entryKey,
    keyCounter: 0,
  });
  const r = restored.createSource({ name: "third.aura" });
  assert.equal(r.ok, true);
  assert.notEqual(r.key, second);
  const keys = restored.sources.map((s) => s.key);
  assert.equal(new Set(keys).size, keys.length);
});

test("the key generator is deterministic and monotonic", () => {
  const g = createKeyGenerator(0);
  assert.equal(g.next(), "s1");
  assert.equal(g.next(), "s2");
  assert.equal(g.peek(), 2);
  assert.equal(highestKeyCounter([{ key: "s3" }, { key: "s11" }, { key: "zz" }]), 11);
  assert.equal(highestKeyCounter([]), 0);
});

// --- request shape --------------------------------------------------------

test("the runtime request uses opaque keys, display names, and no paths", () => {
  const p = Project.default();
  p.createSource({ name: "helper.aura", text: "pub fn v() -> int { return 1 }" });
  const req = p.toRequest();
  assert.equal(req.entry, p.entryKey);
  assert.equal(req.sources.length, 2);
  for (const s of req.sources) {
    assert.ok(KEY_PATTERN.test(s.key));
    assert.deepEqual(Object.keys(s).sort(), ["children", "key", "name", "text"]);
    assert.deepEqual(s.children, []);
    assert.equal(s.key.includes("/"), false);
    assert.equal(s.key.includes("."), false);
  }
  const serialized = JSON.stringify(req);
  assert.equal(serialized.includes("main.aura\""), true, "display names are present");
  assert.equal(/children\\?":\s*\[\s*\{/.test(serialized), false, "no child links are invented");
});

test("declared ownership links translate display names to opaque keys", () => {
  const p = Project.fromFiles([
    { name: "a.aura", text: "x", children: [{ name: "b", to: "b.aura" }] },
    { name: "b.aura", text: "y", children: [] },
  ]);
  const req = p.toRequest();
  assert.deepEqual(req.sources[0].children, [{ name: "b", key: req.sources[1].key }]);
  assert.deepEqual(req.sources[1].children, []);
  // The link carries the Aura module name and the opaque key only: no file
  // name and no path ever reaches the transport as ownership.
  assert.equal(JSON.stringify(req.sources[0].children).includes("b.aura"), false);
  // A link whose target no longer exists is dropped rather than sent dangling.
  const q = Project.fromFiles([
    { name: "a.aura", text: "x", children: [{ name: "b", to: "missing.aura" }] },
    { name: "b.aura", text: "y", children: [] },
  ]);
  assert.deepEqual(q.toRequest().sources[0].children, []);
});

test("ownership links survive a snapshot round trip", () => {
  const p = Project.fromFiles([
    { name: "a.aura", text: "x", children: [{ name: "b", to: "b.aura" }] },
    { name: "b.aura", text: "y", children: [] },
  ]);
  const copy = p.snapshot();
  assert.deepEqual(copy.toRequest(), p.toRequest());
});

test("the request entry always names a registered source", () => {
  const p = Project.default();
  p.createSource({ name: "helper.aura" });
  p.setEntry(p.sources[1].key);
  p.deleteSource(p.sources[1].key);
  const req = p.toRequest();
  assert.ok(req.sources.some((s) => s.key === req.entry));
});

// --- limits ---------------------------------------------------------------

test("the single-source project is within every transport limit", () => {
  assert.deepEqual(Project.default().checkLimits(), { ok: true });
});

test("an oversized file is reported before transport", () => {
  const p = Project.default();
  p.createSource({ name: "big.aura", text: "x".repeat(300 * 1024) });
  const r = p.checkLimits();
  assert.equal(r.ok, false);
  assert.match(r.error, /per-file limit/);
});

test("the source-count ceiling is enforced on create", () => {
  const p = Project.default();
  const many = [];
  for (let i = 0; i < MAX_PROJECT_SOURCES; i += 1) {
    many.push({ key: `s${i + 1}`, name: `f${i}.aura`, text: "" });
  }
  const full = new Project({ sources: many });
  assert.equal(full.size, MAX_PROJECT_SOURCES);
  const r = full.createSource({ name: "overflow.aura" });
  assert.equal(r.ok, false);
  assert.match(r.error, /at most/);
});

// --- name validation ------------------------------------------------------

test("name validation mirrors the runtime's display-name rule", () => {
  assert.equal(isValidSourceName("main.aura"), true);
  assert.equal(isValidSourceName("ünïcode.aura"), true);
  assert.equal(isValidSourceName(""), false);
  assert.equal(isValidSourceName("a\u0000b"), false);
  assert.equal(isValidSourceName("a\nb"), false);
  assert.equal(isValidSourceName("x".repeat(1025)), false);
  assert.equal(utf8Length("é"), 2);
});

// --- invariants under a mutation storm ------------------------------------

test("active and entry always name an existing source under random edits", () => {
  let seed = 12345;
  const rand = (n) => {
    seed = (seed * 1103515245 + 12345) & 0x7fffffff;
    return seed % n;
  };
  const p = Project.default();
  for (let i = 0; i < 400; i += 1) {
    const keys = p.sources.map((s) => s.key);
    switch (rand(6)) {
      case 0:
        p.createSource({ name: `f${i}.aura`, text: `t${i}` });
        break;
      case 1:
        p.deleteSource(keys[rand(keys.length)]);
        break;
      case 2:
        p.selectSource(keys[rand(keys.length)]);
        break;
      case 3:
        p.renameSource(keys[rand(keys.length)], `r${i}.aura`);
        break;
      case 4:
        p.setEntry(keys[rand(keys.length)]);
        break;
      default:
        p.setText(keys[rand(keys.length)], `body ${i}`);
        break;
    }
    assert.ok(p.size >= 1, `project emptied at step ${i}`);
    assert.ok(p.active(), `active key dangling at step ${i}`);
    assert.ok(p.entry(), `entry key dangling at step ${i}`);
    const names = p.sources.map((s) => s.name);
    assert.equal(new Set(names).size, names.length, `duplicate name at step ${i}: ${names}`);
    const ks = p.sources.map((s) => s.key);
    assert.equal(new Set(ks).size, ks.length, `duplicate key at step ${i}`);
    for (const k of ks) assert.ok(KEY_PATTERN.test(k), `bad key ${k} at step ${i}`);
  }
});

test("a snapshot is independent of the project it came from", () => {
  const p = Project.default();
  p.createSource({ name: "helper.aura" });
  const snap = p.snapshot();
  p.setText(p.sources[0].key, "mutated");
  p.renameSource(p.sources[1].key, "other.aura");
  assert.notEqual(snap.get(snap.sources[0].key).text, "mutated");
  assert.equal(snap.get(snap.sources[1].key).name, "helper.aura");
});

console.log(`project: ${passed} passed, 0 failed`);
