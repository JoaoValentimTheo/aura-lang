// Aura syntax highlighting for code presentation.
//
// This is a *presentational* tokenizer derived from the real lexical rules in
// docs/LANGUAGE_SPEC.md §3 and docs/grammar.md. It is not a parser and defines
// no semantics; if it ever disagrees with the language, the language wins. It
// exists only to colour code blocks and never executes anything.

const KEYWORDS = new Set([
  "fn", "let", "mut", "if", "else", "match", "while", "loop", "for", "in",
  "return", "throw", "break", "continue", "try", "catch", "finally", "struct",
  "enum", "type", "use", "pub", "and", "or", "not",
]);

const LITERALS = new Set(["true", "false", "none"]);

// Built-in functions and methods from the signature registry, so highlighting
// reflects the real standard library.
const BUILTINS = new Set([
  "print", "len", "to_string", "to_int", "to_float", "range", "abs", "min",
  "max", "push", "pop", "keys", "values", "sort", "reverse", "map", "filter",
  "reduce", "sum", "assert", "enumerate", "zip", "read_line", "read_file",
  "write_file", "args", "py_eval", "py_import", "py_call", "py_version",
  "json_encode", "json_decode", "regex_match", "regex_find", "regex_find_all",
  "regex_replace", "time_now", "time_unix", "sleep_ms",
  "upper", "lower", "trim", "contains", "starts_with", "ends_with", "split",
  "replace", "chars", "first", "last", "join", "get", "has", "remove",
]);

const TYPES = new Set(["int", "float", "bool", "string"]);

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
  if (BUILTINS.has(word)) return "tok-fn";
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
