#!/usr/bin/env node
// AIS/AIX differential dogfood harness (Keystone §19).
//
// The empirical question is: does the AIS/AIX semantic context let an AI
// consumer work with Aura more accurately and more cheaply than raw source?
//
// This harness makes the comparison *reproducible and honest*:
//
//   * It defines a fixed task bank of small Aura programs and a set of
//     questions whose answers are facts the compiler already knows
//     (a symbol's type, family, value kind, capabilities, a diagnostic code,
//     whether a change is semantic).
//   * It computes the GROUND TRUTH with the compiler itself (`aura ais`), so
//     the answers are checkable, not judged.
//   * It produces two conditions for the same task:
//       - RAW:   the source text only.
//       - AIS:   the source text plus the `aura ais slice` context.
//     and, when an external evaluator command is configured, runs the SAME
//     question under both conditions and scores tool-call-free correctness.
//   * It always reports the structural comparison it can compute itself:
//     the context size of each condition in bytes/tokens and the number of
//     semantic facts each carries. This runs in CI with no model.
//
// If no evaluator is available it prints exactly what was NOT executed and
// the command required to execute it. It never fabricates a model run.
//
// Usage:
//   node tests/ais_differential.mjs                     # structural only
//   AURA_AIS_EVAL_CMD='my-llm --prompt-file' \
//     node tests/ais_differential.mjs                   # real differential
//
// The evaluator command receives the prompt on stdin and must print the
// answer on stdout. It is deliberately provider-agnostic: nothing here is
// bound to a specific model or vendor.

import { spawnSync } from "node:child_process";
import { readFileSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const AURA = process.env.AURA_BIN || "target/debug/aura";

// ---------------------------------------------------------------------------
// Task bank: small programs plus fact questions with compiler-checkable truth
// ---------------------------------------------------------------------------

const TASKS = [
  {
    id: "list-family",
    source: `struct Row { label: string }
fn rows() -> [[int]] { return [] }
fn main() { print(rows()) }
`,
    questions: [
      {
        key: "family",
        ask: "What capability family does the return type of `rows` belong to? Answer with one lowercase word.",
        options: ["scalar", "sequence", "mapping", "object", "sum", "callable", "nullish"],
        truth: "sequence",
      },
      {
        key: "value_kind",
        ask: "What runtime value kind does the return type of `rows` produce? Answer with one lowercase word.",
        options: ["int", "float", "bool", "string", "list", "map", "struct", "enum", "none"],
        truth: "list",
      },
      {
        key: "iterable",
        ask: "Is the return type of `rows` iterable? Answer yes or no.",
        options: ["yes", "no"],
        truth: "yes",
      },
    ],
  },
  {
    id: "optional-field",
    source: `struct User { name: string, email: string | none }
fn find(_ok: bool) -> User | none { return none }
fn main() {
  let u = find(true)
  if u != none { print(u.name) }
}
`,
    questions: [
      {
        key: "email_type",
        ask: "What is the declared type of the `email` field? Answer with the exact type spelling.",
        options: ["string", "string | none", "none", "never"],
        truth: "string | none",
      },
      {
        key: "find_family",
        ask: "The return type of `find` is a union of a struct and none. Which family does the checker assign to it? Answer with one lowercase word.",
        options: ["scalar", "sequence", "mapping", "object", "sum", "callable", "nullish", "unknown"],
        truth: "sum",
      },
    ],
  },
  {
    id: "diagnostic",
    source: `fn main() { let x: int = "s" }
`,
    questions: [
      {
        key: "code",
        ask: "What diagnostic code does the checker report for this program? Answer with the code only, e.g. E1234.",
        options: ["E1006", "E2003", "E3001", "E4031"],
        truth: "E3001",
      },
      {
        key: "severity",
        ask: "What severity does the diagnostic carry? Answer with one lowercase word.",
        options: ["error", "warning", "hint"],
        truth: "error",
      },
    ],
  },
  {
    id: "delta",
    source: `fn a(x: int) -> int { return x }
`,
    questions: [
      {
        key: "change",
        ask: "A later revision of this file adds a comment line above `fn a` and changes nothing else. Is this a semantic change to the declaration `a`? Answer yes or no.",
        options: ["yes", "no"],
        truth: "no",
      },
    ],
  },
];

// ---------------------------------------------------------------------------
// Ground truth is the compiler, not this harness
// ---------------------------------------------------------------------------

function aisSnapshot(file) {
  // `aura ais` exits non-zero when the source has a diagnostic, but it still
  // prints the document on stdout. A diagnostic is a normal answer for this
  // harness, so capture stdout regardless of the exit code.
  const res = spawnSync(AURA, ["ais", file], { encoding: "utf8" });
  return JSON.parse(res.stdout);
}

function aisSlice(file, target) {
  const res = spawnSync(AURA, ["ais", "slice", file, target, "1", "32"], {
    encoding: "utf8",
  });
  return JSON.parse(res.stdout);
}

function aisDelta(oldFile, newFile) {
  const res = spawnSync(AURA, ["ais", "delta", oldFile, newFile], {
    encoding: "utf8",
  });
  return JSON.parse(res.stdout);
}

// Each task's ground truth is verified against the compiler snapshot; a task
// whose expectations no longer match the compiler is a harness bug and fails.
function verifyTruth(dir, task) {
  const file = join(dir, `${task.id}.aura`);
  writeFileSync(file, task.source);
  const doc = aisSnapshot(file);
  const problems = [];
  for (const q of task.questions) {
    if (task.id === "list-family") {
      const rows = doc.symbols.find((s) => s.name === "rows");
      if (q.key === "family" && !(rows.families || []).includes("sequence")) {
        problems.push(`${task.id}/${q.key}: compiler says families=${JSON.stringify(rows.families)}`);
      }
      if (q.key === "value_kind" && rows.value_kind !== "list") {
        problems.push(`${task.id}/${q.key}: compiler says value_kind=${rows.value_kind}`);
      }
      if (q.key === "iterable" && !(rows.capabilities || []).includes("iterable")) {
        problems.push(`${task.id}/${q.key}: compiler says capabilities=${JSON.stringify(rows.capabilities)}`);
      }
    }
    if (task.id === "optional-field") {
      const user = doc.symbols.find((s) => s.name === "User");
      const email = (user.fields || []).find((f) => f.name === "email");
      if (q.key === "email_type" && !email.type_name.includes("none")) {
        problems.push(`${task.id}/${q.key}: compiler says type_name=${email.type_name}`);
      }
      if (q.key === "find_family") {
        const find = doc.symbols.find((s) => s.name === "find");
        if (!(find.families || []).includes("sum")) {
          problems.push(`${task.id}/${q.key}: compiler says families=${JSON.stringify(find.families)}`);
        }
      }
    }
    if (task.id === "diagnostic" && q.key === "code") {
      const code = doc.diagnostics?.[0]?.code_text;
      if (code !== "E3001") problems.push(`${task.id}/${q.key}: compiler says ${code}`);
    }
    if (task.id === "diagnostic" && q.key === "severity") {
      const sev = doc.diagnostics?.[0]?.severity;
      if (sev !== "error") problems.push(`${task.id}/${q.key}: compiler says ${sev}`);
    }
    if (task.id === "delta" && q.key === "change") {
      // Compiler-checkable: a comment insertion must be `moved`, not `changed`.
      const revised = join(dir, `${task.id}.rev.aura`);
      writeFileSync(revised, `# note\n${task.source}`);
      const d = aisDelta(file, revised);
      const a = (d.symbols || []).find((c) => c.name === "a");
      if (a && a.change !== "moved") {
        problems.push(`${task.id}/${q.key}: compiler says change=${a.change}`);
      }
    }
  }
  return problems;
}

// ---------------------------------------------------------------------------
// Conditions
// ---------------------------------------------------------------------------

function rawContext(task) {
  return `Source:\n${task.source}`;
}

function aisContext(dir, task) {
  const file = join(dir, `${task.id}.aura`);
  const doc = aisSnapshot(file);
  // One slice per declared function keeps the context task-focused.
  const slices = doc.symbols
    .filter((s) => s.kind === "function")
    .map((s) => {
      try {
        return aisSlice(file, s.name);
      } catch {
        return null;
      }
    })
    .filter(Boolean);
  return `Source:\n${task.source}\n\nAIS semantic context:\n${JSON.stringify(
    { snapshot: { revision: doc.revision, symbols: doc.symbols, diagnostics: doc.diagnostics }, slices },
    null,
    1,
  )}`;
}

function promptFor(task, question, context) {
  return `${context}\n\nQuestion: ${question.ask}\nAllowed answers: ${question.options.join(
    ", ",
  )}\nAnswer with the answer alone.\n`;
}

// A deliberately simple token estimate (chars / 4), stated as an estimate.
const estimateTokens = (text) => Math.ceil(text.length / 4);

// ---------------------------------------------------------------------------
// Run
// ---------------------------------------------------------------------------

const dir = mkdtempSync(join(tmpdir(), "aura-ais-diff-"));
let harnessFailures = 0;

console.log("AIS/AIX differential dogfood (structural layer)\n");
for (const task of TASKS) {
  const problems = verifyTruth(dir, task);
  if (problems.length) {
    harnessFailures += problems.length;
    console.error(`FAIL ${task.id}: ground truth disagrees with the compiler`);
    for (const p of problems) console.error(`  - ${p}`);
  } else {
    console.log(`ok   ${task.id}: ${task.questions.length} questions verified against the compiler`);
  }
}

let rawTotal = 0;
let aisTotal = 0;
let factsRaw = 0;
let factsAis = 0;
for (const task of TASKS) {
  const raw = rawContext(task);
  const ais = aisContext(dir, task);
  rawTotal += estimateTokens(raw);
  aisTotal += estimateTokens(ais);
  factsRaw += 1; // the source is the only fact carrier
  factsAis += 1 + (JSON.parse(ais.split("AIS semantic context:\n")[1]).snapshot.symbols || []).length;
}
console.log(`\nContext cost (estimate, chars/4): raw=${rawTotal} tok, ais=${aisTotal} tok`);
console.log(`Fact carriers: raw=${factsRaw}, ais=${factsAis}`);
console.log(
  `Note: AIS context is larger than the raw source for tiny programs; its value is that every fact is explicit and machine-addressable, and it does not grow with the number of questions asked of the same revision.`,
);

// The real differential, when an evaluator is configured.
const evalCmd = process.env.AURA_AIS_EVAL_CMD;
if (!evalCmd) {
  console.log(
    "\nEXTERNAL DIFFERENTIAL: NOT EXECUTED (no evaluator configured).\n" +
      "To run it, set AURA_AIS_EVAL_CMD to a command that reads a prompt on stdin\n" +
      "and prints the answer on stdout, then re-run this file. Example:\n" +
      "  AURA_AIS_EVAL_CMD='ollama run llama3' node tests/ais_differential.mjs\n",
  );
} else {
  const [cmd, ...args] = evalCmd.split(" ");
  let correctRaw = 0;
  let correctAis = 0;
  let total = 0;
  for (const task of TASKS) {
    for (const q of task.questions) {
      total += 1;
      for (const [condition, ctx] of [
        ["raw", rawContext(task)],
        ["ais", aisContext(dir, task)],
      ]) {
        const prompt = promptFor(task, q, ctx);
        const res = spawnSync(cmd, args, { input: prompt, encoding: "utf8" });
        const answer = (res.stdout || "").trim().toLowerCase();
        const ok = answer === q.truth;
        if (condition === "raw" && ok) correctRaw += 1;
        if (condition === "ais" && ok) correctAis += 1;
      }
    }
  }
  console.log(`\nEXTERNAL DIFFERENTIAL: ${total} questions x 2 conditions`);
  console.log(`raw correct: ${correctRaw}/${total}`);
  console.log(`ais correct: ${correctAis}/${total}`);
  console.log(
    `semantic utility per token (correct answers / 1000 estimated context tokens):\n` +
      `  raw: ${((correctRaw / rawTotal) * 1000).toFixed(3)}\n` +
      `  ais: ${((correctAis / aisTotal) * 1000).toFixed(3)}`,
  );
}

process.exit(harnessFailures === 0 ? 0 : 1);
