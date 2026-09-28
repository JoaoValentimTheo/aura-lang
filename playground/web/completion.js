// Lightweight Aura completion for the Playground editor.
//
// This is an editor suggestion surface, not a language server and not a
// checker: it uses simple lexical context and the shared language metadata. It
// never executes Aura and never claims a suggestion is valid. When it disagrees
// with the compiler, the compiler wins.
//
// Sources: hard keywords, contextual words, primitive types, literals,
// builtins, methods, and names already present in the current buffer.

import {
  KEYWORDS,
  CONTEXTUAL,
  LITERALS,
  TYPES,
  BUILTINS,
  METHODS,
} from "./language.js";

// Small, well-known snippets keyed by their leading word.
const SNIPPETS = [
  { label: "fn main()", insert: "fn main() {\n    $0\n}" },
  { label: "for in", insert: "for x in xs {\n    $0\n}" },
  { label: "if else", insert: "if cond {\n    $0\n} else {\n}" },
  { label: "match", insert: "match value {\n    _ -> $0\n}" },
  { label: "type Map", insert: "type Map<K, V> = {K: V}" },
];

function kindOf(label) {
  if (KEYWORDS.includes(label)) return "keyword";
  if (CONTEXTUAL.includes(label)) return "keyword";
  if (LITERALS.includes(label)) return "literal";
  if (TYPES.includes(label)) return "type";
  if (BUILTINS.includes(label)) return "builtin";
  if (METHODS.includes(label)) return "method";
  return "name";
}

/** The identifier fragment immediately before `pos` in `text`, or "". */
export function wordAt(text, pos) {
  let i = pos;
  while (i > 0 && /[A-Za-z0-9_]/.test(text[i - 1])) i -= 1;
  return text.slice(i, pos);
}

/**
 * Compute completion candidates for `text` at caret `pos`.
 *
 * `receiver` is an optional hint from the caller (e.g. "map" when the caret
 * follows `m.` on a value the caller believes is a map). Method suggestions are
 * offered only when a `.` precedes the fragment, so unsupported APIs are not
 * shown in ordinary identifier position.
 */
export function candidates(text, pos, receiver = null) {
  const fragment = wordAt(text, pos);
  const before = text.slice(0, pos - fragment.length);
  const afterDot = before.endsWith(".");
  const lower = fragment.toLowerCase();

  // No popup on empty identifier position: it would be noise and would capture
  // Tab/Enter that the editor needs for indentation and focus movement. A dot
  // is an explicit request for members, so `m.` still opens the popup.
  if (fragment === "" && !afterDot) return [];

  // Names already written in the buffer (a safe, cheap "local" source).
  const seen = new Set();
  for (const m of text.matchAll(/[A-Za-z_][A-Za-z0-9_]*/g)) seen.add(m[0]);

  const pool = [];
  const add = (label, kind) => {
    if (lower && !label.toLowerCase().startsWith(lower)) return;
    if (label === fragment) return;
    pool.push({ label, kind });
  };

  if (afterDot) {
    // After a `.`, only methods are meaningful.
    for (const m of METHODS) add(m, "method");
  } else {
    for (const k of KEYWORDS) add(k, "keyword");
    for (const c of CONTEXTUAL) add(c, "keyword");
    for (const t of TYPES) add(t, "type");
    for (const l of LITERALS) add(l, "literal");
    for (const b of BUILTINS) add(b, "builtin");
    if (fragment) {
      for (const s of seen) {
        if (s !== fragment) add(s, "name");
      }
    }
  }

  // Stable, useful order: a prefix match on a builtin/method first, then the
  // rest alphabetically within kind.
  const order = { builtin: 0, method: 1, keyword: 2, type: 3, literal: 4, name: 5 };
  pool.sort((a, b) => {
    const ka = order[a.kind] ?? 9;
    const kb = order[b.kind] ?? 9;
    if (ka !== kb) return ka - kb;
    return a.label.localeCompare(b.label);
  });

  // Snippets only for a non-empty prefix at the start of a word.
  if (!afterDot && fragment) {
    for (const s of SNIPPETS) {
      if (s.label.toLowerCase().startsWith(lower)) {
        pool.unshift({ label: s.label, kind: "snippet", insert: s.insert });
      }
    }
  }

  // With an explicit prefix, cap the list so the popup stays bounded. With an
  // empty fragment after a dot, the full method set is useful and small.
  void receiver;
  return fragment ? pool.slice(0, 20) : pool.slice(0, 60);
}
