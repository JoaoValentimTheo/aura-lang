// Presentational Aura syntax highlighter for the Playground editor.
//
// This is a *presentational* tokenizer derived from the real lexical rules in
// `docs/LANGUAGE_SPEC.md` §3 and `docs/grammar.md`, mirroring the website's
// `website/lib/highlight.mjs`. It is not a parser and defines no semantics; if
// it ever disagrees with the language, the language wins. Output is escaped
// HTML, safe to assign to `innerHTML`.

const KEYWORDS = new Set([
  "fn", "let", "mut", "if", "else", "match", "while", "loop", "for", "in",
  "return", "throw", "break", "continue", "try", "catch", "finally", "struct",
  "enum", "type", "use", "pub", "and", "or", "not", "impl", "self",
]);

const LITERALS = new Set(["true", "false", "none"]);

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

function escapeHtml(s) {
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
 * inject as innerHTML. A trailing newline is preserved so the highlighted layer
 * keeps the same height as the textarea it sits behind.
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

    // Multiline comment: `<!--` … `--!>`.
    if (c === "<" && source.startsWith("<!--", i)) {
      const end = source.indexOf("--!>", i + 4);
      const j = end === -1 ? n : end + 4;
      push(source.slice(i, j), "tok-comment");
      i = j;
      continue;
    }

    // F-string or string.
    if (c === '"' || (c === "f" && source[i + 1] === '"')) {
      const isF = c === "f";
      let j = i + (isF ? 2 : 1);
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
      push(word, classFor(word));
      i = j;
      continue;
    }

    // Arrow / pipe.
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

    if (c === "\n") {
      out.push("\n");
      i += 1;
      continue;
    }

    push(c);
    i += 1;
  }

  // A trailing newline keeps the last line's height, matching the textarea.
  if (source.endsWith("\n")) out.push("\n");
  return out.join("");
}
