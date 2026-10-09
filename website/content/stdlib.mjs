// Website-facing standard-library metadata.
//
// This is a *presentation* mirror of the authoritative Rust registry
// (`src/stdlib/signatures.rs`), which the checker and runtime both consult.
// It is deliberately not a second language specification: it records only the
// names, signatures, and return summaries the website documents, and a
// consistency test (`tests/stdlib-consistency.test.mjs`) verifies that every
// builtin and method named here exists in the Rust registry under the
// appropriate feature set. Drift therefore fails the suite rather than
// shipping.
//
// Both the landing standard-library page (`pages/stdlib.mjs`) and the
// documentation reference render from these arrays, so the two cannot
// contradict each other.

export const coreFunctions = [
  ["print", "print(...)", "`none` — write arguments separated by spaces, then a newline"],
  ["len", "len(x)", "`int` — for string, list, array, tuple, set, map, range"],
  ["to_string", "to_string(x)", "`string`"],
  ["to_int", "to_int(x)", "`int`"],
  ["to_float", "to_float(x)", "`float`"],
  ["range", "range(n)` / `range(a, b)", "`range` — start-inclusive, end-exclusive"],
  ["abs", "abs(n)", "number — absolute value"],
  ["min", "min(a, b)", "the lesser of two values"],
  ["max", "max(a, b)", "the greater of two values"],
  ["assert", "assert(cond)` / `assert(cond, msg)", "`none`; `E4028` on failure"],
];

export const collectionFunctions = [
  ["push", "push(list, v)", "`none` — append to the list"],
  ["keys", "keys(map)", "`[K]` — the map's key type"],
  ["values", "values(map)", "`[V]` — the map's value type"],
  ["sort", "sort(list)", "a sorted list"],
  ["reverse", "reverse(string | list)", "the reversed value"],
  ["map", "map(list, f)", "`[T]` — apply `f` to each element"],
  ["filter", "filter(list, f)", "`[T]` — keep elements whose predicate is truthy"],
  ["reduce", "reduce(list, f, init)", "the accumulated value"],
  ["sum", "sum(list)", "number"],
  ["enumerate", "enumerate(list)", "`[[index, value], ...]`"],
  ["zip", "zip(a, b)", "`[[a_i, b_i], ...]`"],
];

// Method inventory by receiver kind. Mirrors `methods()` in the Rust registry.
export const methods = [
  {
    receiver: "string",
    note: "Immutable UTF-8 text.",
    names: [
      "len()", "upper()", "lower()", "trim()", "contains(s)", "starts_with(s)",
      "ends_with(s)", "split(s) → [string]", "replace(a, b)", "chars() → [string]",
    ],
  },
  {
    receiver: "list",
    note: "Ordered, mutable, resizable.",
    names: [
      "len()", "push(v)", "pop()", "first()", "last()", "join(s)", "contains(v)",
      "sort()", "reverse()", "map(f)", "filter(f)", "reduce(f, init)",
    ],
  },
  {
    receiver: "array",
    note: "Fixed-length: the read-side sequence methods, but no resizing.",
    names: [
      "len()", "first()", "last()", "join(s)", "contains(v)", "map(f)",
      "filter(f)", "reduce(f, init)",
    ],
  },
  {
    receiver: "tuple",
    note: "Fixed-length, heterogeneous, immutable.",
    names: [
      "len()", "first()", "last()", "contains(v)", "map(f)", "filter(f)",
      "reduce(f, init)",
    ],
  },
  {
    receiver: "set",
    note: "Unordered membership of key-capable scalars; not indexable.",
    names: ["len()", "has(v)", "contains(v)", "add(v)", "remove(v)"],
  },
  {
    receiver: "map",
    note: "Ordered by key.",
    names: ["len()", "get(k)", "has(k)", "keys()", "values()", "items()", "remove(k)"],
  },
  {
    receiver: "range",
    note: "Iterable integer range.",
    names: ["len()"],
  },
];

export const scriptingIo = [
  ["read_line", "read_line()", "`string | none` — one line, or `none` at end of input"],
  ["read_file", "read_file(path)", "`string | none` — missing path is `none`"],
  ["write_file", "write_file(path, content)", "`none`"],
  ["args", "args()", "`[string]`"],
];

export const featureModules = [
  {
    name: "json",
    gated: false,
    functions: [
      ["json_encode", "json_encode(value) → `string`"],
      ["json_decode", "json_decode(text) → value"],
      ["json_decode_as", "json_decode_as(text, Type) → `Type` — the second argument is a **type** (`User`, `[User]`, `[int; 3]`, `(int, string)`, `{int}`, `{string: Pokemon}`); strict, `E4031` on mismatch"],
    ],
  },
  {
    name: "regex",
    gated: false,
    functions: [
      ["regex_match", "regex_match(pattern, text) → `bool`"],
      ["regex_find", "regex_find(pattern, text) → `string | none`"],
      ["regex_find_all", "regex_find_all(pattern, text) → `[string]`"],
      ["regex_replace", "regex_replace(pattern, text, replacement) → `string`"],
    ],
  },
  {
    name: "time",
    gated: false,
    functions: [
      ["time_unix", "time_unix() → `int`"],
      ["time_now", "time_now() → a local-time map"],
      ["sleep_ms", "sleep_ms(n) → `none`"],
    ],
  },
  {
    name: "http",
    gated: true,
    note: "Native-only and not default; network is host-owned and unavailable in the browser Playground (`E5002`).",
    functions: [
      ["http_get", "http_get(url) → `{status, headers, body}`"],
      ["http_request", "http_request(method, url) → `{status, headers, body}`"],
      ["http_request", "http_request(method, url, options) → `{status, headers, body}`"],
    ],
  },
];

/** Every builtin function name documented by the website. */
export function documentedBuiltinNames() {
  const names = new Set();
  for (const [name] of coreFunctions) names.add(name);
  for (const [name] of collectionFunctions) names.add(name);
  for (const [name] of scriptingIo) names.add(name);
  for (const mod of featureModules) {
    for (const [name] of mod.functions) names.add(name);
  }
  return [...names];
}
