// Shared Aura language metadata for the Playground and website front ends.
//
// This is the *only* inventory of keywords, literal words, primitive types,
// builtins, and methods in the front end. It is presentational (highlighting and
// completion) and defines no semantics: the compiler's lexer and signature
// registry remain authoritative. `tests/language_metadata.rs` fails if this file
// drifts from them, so the two cannot silently disagree.
//
// Keep the arrays sorted. The drift test compares them as sets, but sorting
// keeps diffs reviewable when a word is added.

// HARD KEYWORDS — `aura::lex::KEYWORDS` minus the literal words, which are
// highlighted as literals rather than keywords.
export const KEYWORDS = [
  "and", "as", "break", "catch", "continue", "else", "enum", "finally", "fn",
  "for", "if", "in", "let", "loop", "match", "mut", "not", "or", "pub",
  "return", "struct", "throw", "try", "type", "use", "while",
];

// CONTEXTUAL LANGUAGE WORDS — meaningful in the grammar/spec but not hard
// keywords (they may be used as identifiers).
export const CONTEXTUAL = ["module", "impl", "trait", "const", "self"];

// LITERALS — spelled values that render as literals.
export const LITERALS = ["true", "false", "none"];

// PRIMITIVE TYPES.
export const TYPES = ["int", "float", "bool", "string"];

// BUILT-IN FUNCTIONS — `aura::stdlib::builtin_names()`.
export const BUILTINS = [
  "abs", "args", "assert", "enumerate", "filter", "json_decode",
  "json_decode_as",
  "json_encode", "keys", "len", "map", "max", "min", "print", "push",
  "py_call", "py_eval", "py_import", "py_version", "range", "read_file",
  "read_line", "reduce", "regex_find", "regex_find_all", "regex_match",
  "regex_replace", "reverse", "sleep_ms", "sort", "sum", "time_now",
  "time_unix", "to_float", "to_int", "to_string", "values", "write_file",
  "zip",
];

// STANDARD METHODS — `aura::stdlib::signatures::methods()`.
export const METHODS = [
  "add", "chars", "contains", "ends_with", "filter", "first", "get", "has",
  "items", "join", "keys", "last", "len", "lower", "map", "pop", "push",
  "reduce", "remove", "replace", "reverse", "sort", "split", "starts_with",
  "trim", "upper", "values",
];

export const KEYWORD_SET = new Set(KEYWORDS);
export const CONTEXTUAL_SET = new Set(CONTEXTUAL);
export const LITERAL_SET = new Set(LITERALS);
export const TYPE_SET = new Set(TYPES);
export const BUILTIN_SET = new Set(BUILTINS);
export const METHOD_SET = new Set(METHODS);
