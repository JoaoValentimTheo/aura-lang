// Aura syntax highlighting for code presentation.
//
// This is a *presentational* tokenizer derived from the real lexical rules in
// docs/LANGUAGE_SPEC.md §3 and docs/grammar.md. It is not a parser and defines
// no semantics; if it ever disagrees with the language, the language wins. It
// exists only to colour code blocks and never executes anything.
//
// The inventories come from the shared `playground/web/language.js`, which
// `tests/language_metadata.rs` keeps in sync with the compiler, so the website
// and Playground cannot drift from the language or from each other.

import {
  KEYWORD_SET as KEYWORDS,
  CONTEXTUAL_SET,
  LITERAL_SET as LITERALS,
  TYPE_SET as TYPES,
  BUILTIN_SET as BUILTINS,
  METHOD_SET as METHODS,
} from "../../playground/web/language.js";

const ESCAPE = /[\\`*_{}[\]()#+\-.!|>]/g;
export function escapeHtml(s) {
  return s.replace(/[&<>"']/g, (c) => {
    switch (c) {
      case "&":
        return "&amp;";
      case "<":
        return "&lt;";
      case ">":
        return "&gt;";
      case '"':
        return "&quot;";
      default:
        return "&#39;";
    }
  });
}

function classFor(word) {
  if (KEYWORDS.has(word)) return "tok-keyword";
  if (LITERALS.has(word)) return "tok-number";
  if (TYPES.has(word)) return "tok-type";
  if (CONTEXTUAL_SET.has(word)) return "tok-keyword";
  if (BUILTINS.has(word) || METHODS.has(word)) return "tok-fn";
  return null;
}

/**
 * Highlight Aura source into HTML spans. Input is escaped; output is safe to
 * inject as innerHTML. Tokenizes strings, f-strings, comments, numbers, and
 * identifiers conservatively; anything unrecognised is emitted as punctuation.
 */
export function highlight(source) {
  const out = [];
  let i = 0;
  const n = source.length;

  const push = (text, cls) =>
    out.push(cls ? `<span class="${cls}">${escapeHtml(text)}</span>` : escapeHtml(text));

  while (i < n) {
    const c = source[i];

    // Comment: `#` to end of line.
    if (c === "#") {
      let j = i;
      while (j < n && source[j] !== "\n") j += 1;
      push(source.slice(i, j), "tok-comment");
      i = j;
      continue;
    }

    // F-string or string.
    if (c === '"' || (c === "f" && source[i + 1] === '"')) {
      const isF = c === "f";
      let j = i + (isF ? 2 : 1);
      // For f-strings, colour the whole literal as a string; interpolation
      // braces are left inside, which is a deliberate, safe simplification.
      while (j < n) {
        if (source[j] === "\\") {
          j += 2;
          continue;
        }
        if (source[j] === '"') {
          j += 1;
          break;
        }
        if (source[j] === "\n") break;
        j += 1;
      }
      push(source.slice(i, j), "tok-string");
      i = j;
      continue;
    }

    // Number.
    if (c >= "0" && c <= "9") {
      let j = i;
      while (j < n && /[0-9._a-fA-FxX]/.test(source[j])) j += 1;
      push(source.slice(i, j), "tok-number");
      i = j;
      continue;
    }

    // Identifier / keyword / literal.
    if (/[A-Za-z_]/.test(c)) {
      let j = i;
      while (j < n && /[A-Za-z0-9_]/.test(source[j])) j += 1;
      const word = source.slice(i, j);
      if (word === "f" && source[j] === '"') {
        // handled above; fall through as identifier
      }
      push(word, classFor(word));
      i = j;
      continue;
    }

    // Arrow / pipe / operators.
    if (c === "-" && source[i + 1] === ">") {
      push("->", "tok-operator");
      i += 2;
      continue;
    }
    if (c === "|" && source[i + 1] === ">") {
      push("|>", "tok-operator");
      i += 2;
      continue;
    }

    if ("+-*/%^=<>!".includes(c)) {
      let j = i;
      while (j < n && "+-*/%^=<>!".includes(source[j])) j += 1;
      push(source.slice(i, j), "tok-operator");
      i = j;
      continue;
    }

    if ("{}()[],:.".includes(c)) {
      push(c, "tok-punct");
      i += 1;
      continue;
    }

    push(c);
    i += 1;
  }

  return out.join("");
}

export { ESCAPE };
