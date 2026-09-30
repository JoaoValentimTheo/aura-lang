// The Playground's project state model.
//
// This module is *state*, not semantics. It knows nothing about Aura modules,
// visibility, resolution, or canonical names. It only holds a flat set of
// editable sources and the two identities the runtime's virtual-project
// transport requires:
//
//   * an opaque provider `key` per source — the `SourceKey` the runtime's
//     `InMemorySourceProvider` uses as provider-local identity. Keys are
//     generated here, are path-free, satisfy the runtime's virtual-key
//     grammar (`[A-Za-z0-9_-]{1,128}`), and are NEVER derived from the
//     user's filename.
//   * a display `name` per source — the user-facing `SourceName`. It is
//     unique within a project so two sources can never collapse to the same
//     public diagnostic identity, and it is provenance only: the runtime
//     rejects a duplicate before provider construction.
//
// A displayed filename is therefore *not* module identity and *not* provider
// identity. Renaming a source changes its display name and nothing else: the
// key is stable, so no source ownership or reference can be rewritten by a
// rename. Aura module names continue to come exclusively from the source text
// (`module Name { … }`) and the project's declared child links, exactly as
// before.
//
// Pure ES module: no DOM, no worker, no wasm. It is unit-testable in Node.

/**
 * The runtime's virtual-key grammar, mirrored so the UI never generates a key
 * the runtime would reject with `E2022`. This is a *generation* rule, not a
 * validation authority: the runtime/provider remains authoritative for what it
 * accepts.
 */
export const KEY_PATTERN = /^[A-Za-z0-9_-]{1,128}$/;

/** Runtime policy: maximum length of one user-facing source name, in bytes. */
export const MAX_SOURCE_NAME_BYTES = 1024;

/** Runtime policy: maximum number of sources in one virtual project. */
export const MAX_PROJECT_SOURCES = 4096;

/** Runtime policy: maximum UTF-8 bytes of one source's text. */
export const MAX_SOURCE_BYTES = 256 * 1024;

/** Runtime policy: maximum encoded project request size, in bytes. */
export const MAX_PROJECT_BYTES = 2 * 1024 * 1024;

/**
 * The canonical one-source project: the single buffer the Playground has
 * always had. Its display name matches the historical static tab label.
 */
export const DEFAULT_SOURCE_NAME = "main.aura";

/** The starter program, preserved from the single-source Playground. */
export const DEFAULT_SOURCE_TEXT = `fn main() {
    print("hello, Aura")

    let xs = [1, 2, 3, 4, 5]
    let evens = xs.filter((x) -> x % 2 == 0)
    print(evens.map((x) -> x * x))
}
`;

/** UTF-8 byte length of a string, as the runtime measures it. */
export function utf8Length(text) {
  return new TextEncoder().encode(text).length;
}

/**
 * True when `name` is a usable display name: non-empty, within the runtime's
 * byte ceiling, and free of control characters (the runtime's own rule).
 */
export function isValidSourceName(name) {
  if (typeof name !== "string") return false;
  if (name.length === 0) return false;
  if (utf8Length(name) > MAX_SOURCE_NAME_BYTES) return false;
  for (const ch of name) {
    const code = ch.codePointAt(0);
    if (code < 0x20 || code === 0x7f) return false;
  }
  return true;
}

/**
 * A deterministic, path-free key generator.
 *
 * Keys are `s` followed by a monotonically increasing counter, so they match
 * the runtime's grammar, contain no separator or dot, and can never be
 * confused with a filename, a host path, or an Aura module path. The counter is
 * advanced past any key already present, so a generator restored alongside an
 * existing project can never mint a duplicate.
 */
export function createKeyGenerator(start = 0) {
  let next = start;
  return {
    next() {
      next += 1;
      return `s${next}`;
    },
    /** The counter value most recently handed out. */
    peek() {
      return next;
    },
  };
}

/** The largest `s<N>` counter already used by `sources`, or 0. */
export function highestKeyCounter(sources) {
  let max = 0;
  for (const source of sources) {
    const m = /^s(\d+)$/.exec(source.key);
    if (m) {
      const n = Number(m[1]);
      if (n > max) max = n;
    }
  }
  return max;
}

/**
 * A project: a flat, ordered collection of sources plus the active source and
 * the entry source.
 *
 * Invariants, enforced on every mutation:
 *
 *   1. `sources` is never empty.
 *   2. every `key` is unique and matches {@link KEY_PATTERN};
 *   3. every `name` is valid and unique within the project;
 *   4. `activeKey` names an existing source;
 *   5. `entryKey` names an existing source;
 *   6. `nextName()` never returns an existing name.
 *
 * Ordering is insertion order and is preserved across rename, delete, and
 * entry changes. The order is a UI convenience only — the runtime sorts sources
 * by key before building the provider, so graph order and diagnostic order
 * never depend on it.
 */
export class Project {
  /**
   * @param {object} [options]
   * @param {Array<{key: string, name: string, text: string, children?: Array}>} [options.sources]
   * @param {string} [options.activeKey]
   * @param {string} [options.entryKey]
   * @param {number} [options.keyCounter]
   */
  constructor({ sources, activeKey, entryKey, keyCounter } = {}) {
    const list = Array.isArray(sources) && sources.length > 0
      ? sources.map((s) => ({
          key: String(s.key),
          name: String(s.name),
          text: String(s.text ?? ""),
          // Declared ownership links are part of a source's state, so a
          // snapshot/restore round trip must not drop them.
          children: Array.isArray(s.children) ? s.children.map((c) => ({ ...c })) : [],
        }))
      : [{ key: "s1", name: DEFAULT_SOURCE_NAME, text: DEFAULT_SOURCE_TEXT, children: [] }];
    this.sources = list;
    this.activeKey = activeKey && list.some((s) => s.key === activeKey) ? activeKey : list[0].key;
    this.entryKey = entryKey && list.some((s) => s.key === entryKey) ? entryKey : list[0].key;
    this.keyCounter =
      typeof keyCounter === "number" ? keyCounter : highestKeyCounter(list);
  }

  /** The canonical single-source project. */
  static default() {
    return new Project();
  }

  /**
   * Build a project from a list of plain `{ name, text, children? }` files.
   *
   * A file may declare `children: [{ name, to }]`: `name` is the Aura logical
   * module name the parent refers to and `to` is the display name of the file
   * that provides it. Those declarations are carried through verbatim as the
   * runtime's provider child links, so the graph builder — never the UI —
   * decides ownership, collisions, and ordering. A declaration whose target is
   * missing is dropped here and would be refused by the runtime anyway.
   */
  static fromFiles(files) {
    const keys = createKeyGenerator(0);
    const sources = files.map((f) => ({
      key: keys.next(),
      name: f.name,
      text: f.text ?? "",
      children: Array.isArray(f.children) ? f.children.map((c) => ({ ...c })) : [],
    }));
    return new Project({
      sources,
      activeKey: sources[0].key,
      entryKey: sources[0].key,
      keyCounter: keys.peek(),
    });
  }

  /** A defensive copy, so callers can snapshot state without aliasing it. */
  snapshot() {
    return new Project({
      sources: this.sources.map((s) => ({ ...s })),
      activeKey: this.activeKey,
      entryKey: this.entryKey,
      keyCounter: this.keyCounter,
    });
  }

  /** Number of sources. */
  get size() {
    return this.sources.length;
  }

  /** The active source record. Always defined (invariant 1/4). */
  active() {
    return this.sources.find((s) => s.key === this.activeKey);
  }

  /** The entry source record. Always defined (invariant 1/5). */
  entry() {
    return this.sources.find((s) => s.key === this.entryKey);
  }

  /** Look one source up by key, or `undefined`. */
  get(key) {
    return this.sources.find((s) => s.key === key);
  }

  /** True when `name` is already used by a source other than `exceptKey`. */
  hasName(name, exceptKey = null) {
    return this.sources.some((s) => s.name === name && s.key !== exceptKey);
  }

  /**
   * A filename that is not yet used: `main.aura`, `main-2.aura`, `main-3.aura`, …
   */
  nextName(base = DEFAULT_SOURCE_NAME) {
    if (!this.hasName(base)) return base;
    const dot = base.lastIndexOf(".");
    const stem = dot > 0 ? base.slice(0, dot) : base;
    const ext = dot > 0 ? base.slice(dot) : "";
    for (let n = 2; ; n += 1) {
      const candidate = `${stem}-${n}${ext}`;
      if (!this.hasName(candidate)) return candidate;
    }
  }

  /** Mint a key that no source currently uses. */
  mintKey() {
    let key;
    do {
      this.keyCounter += 1;
      key = `s${this.keyCounter}`;
    } while (this.sources.some((s) => s.key === key));
    return key;
  }

  /** Store edited text for one source. */
  setText(key, text) {
    const source = this.get(key);
    if (!source) return false;
    source.text = String(text ?? "");
    return true;
  }

  /**
   * Create a new source.
   *
   * @param {object} [options]
   * @param {string} [options.name] display name; defaults to an unused
   *   `main-N.aura`. Must be unique and valid.
   * @param {string} [options.text]
   * @param {boolean} [options.activate] make it active (default true)
   * @returns {{ok: true, key: string} | {ok: false, error: string}}
   */
  createSource({ name, text = "", activate = true } = {}) {
    if (this.sources.length >= MAX_PROJECT_SOURCES) {
      return { ok: false, error: `a project may hold at most ${MAX_PROJECT_SOURCES} files` };
    }
    const resolved = name === undefined || name === null ? this.nextName() : String(name);
    if (!isValidSourceName(resolved)) {
      return { ok: false, error: "a file name must be non-empty and free of control characters" };
    }
    if (this.hasName(resolved)) {
      return { ok: false, error: `a file named "${resolved}" already exists` };
    }
    const key = this.mintKey();
    this.sources.push({ key, name: resolved, text: String(text ?? ""), children: [] });
    if (activate) this.activeKey = key;
    return { ok: true, key };
  }

  /**
   * Rename one source. The key is untouched, so no semantic reference and no
   * provider identity can be affected.
   *
   * @returns {{ok: true, name: string} | {ok: false, error: string}}
   */
  renameSource(key, name) {
    const source = this.get(key);
    if (!source) return { ok: false, error: "no such file" };
    const resolved = String(name ?? "");
    if (!isValidSourceName(resolved)) {
      return { ok: false, error: "a file name must be non-empty and free of control characters" };
    }
    if (resolved === source.name) return { ok: true, name: resolved };
    if (this.hasName(resolved, key)) {
      return { ok: false, error: `a file named "${resolved}" already exists` };
    }
    source.name = resolved;
    return { ok: true, name: resolved };
  }

  /**
   * Delete one source. Deleting the active or the entry source is allowed and
   * resolves deterministically:
   *
   *   * the survivor nearest the removed index becomes active (the previous
   *     file when one exists, otherwise the next);
   *   * the entry is reassigned to the (possibly new) active source, unless
   *     another source already held the entry, in which case the entry is
   *     left where it was.
   *
   * The last remaining source cannot be deleted: a project always has at
   * least one file.
   *
   * @returns {{ok: true, activeKey: string, entryKey: string} | {ok: false, error: string}}
   */
  deleteSource(key) {
    const index = this.sources.findIndex((s) => s.key === key);
    if (index === -1) return { ok: false, error: "no such file" };
    if (this.sources.length === 1) {
      return { ok: false, error: "a project must keep at least one file" };
    }
    const wasActive = this.activeKey === key;
    const wasEntry = this.entryKey === key;
    this.sources.splice(index, 1);
    if (wasActive) {
      const survivor = this.sources[Math.max(0, index - 1)];
      this.activeKey = survivor.key;
    }
    if (wasEntry) {
      this.entryKey = this.activeKey;
    }
    return { ok: true, activeKey: this.activeKey, entryKey: this.entryKey };
  }

  /** Select the active source. */
  selectSource(key) {
    if (!this.get(key)) return { ok: false, error: "no such file" };
    this.activeKey = key;
    return { ok: true, key };
  }

  /** Declare which source is the entry point. */
  setEntry(key) {
    if (!this.get(key)) return { ok: false, error: "no such file" };
    this.entryKey = key;
    return { ok: true, key };
  }

  /** Replace the whole project with the canonical one-source project. */
  reset() {
    const fresh = Project.default();
    this.sources = fresh.sources;
    this.activeKey = fresh.activeKey;
    this.entryKey = fresh.entryKey;
    this.keyCounter = fresh.keyCounter;
    return this;
  }

  /** Replace the whole project with the given `{ name, text }` files. */
  loadFiles(files) {
    const fresh = Project.fromFiles(files);
    this.sources = fresh.sources;
    this.activeKey = fresh.activeKey;
    this.entryKey = fresh.entryKey;
    this.keyCounter = fresh.keyCounter;
    return this;
  }

  /**
   * True when this project is exactly the historical single-source shape: one
   * source and no declared children. The runtime applies its eval fallback to
   * precisely this shape, so the UI uses the single-source `run()` path for it
   * and the project path otherwise.
   */
  isSingleSource() {
    return this.sources.length === 1;
  }

  /**
   * The virtual-project request the runtime's `runProject` consumes.
   *
   * `entry` and every `key` are opaque provider identities. `name` is display
   * provenance. No host path, module name, or Aura identity appears here.
   *
   * @returns {{entry: string, sources: Array<{key: string, name: string, text: string, children: Array}>}}
   */
  toRequest() {
    const byName = new Map(this.sources.map((s) => [s.name, s]));
    return {
      entry: this.entryKey,
      sources: this.sources.map((s) => ({
        key: s.key,
        name: s.name,
        text: s.text,
        // Declared ownership links, translated from the UI's file names to the
        // opaque provider keys the transport requires. A declaration whose
        // target no longer exists is dropped; the runtime would refuse a
        // dangling key anyway, and dropping keeps a stale link from turning a
        // recoverable edit into a transport error.
        children: (s.children || [])
          .map((c) => {
            const target = byName.get(c.to);
            return target ? { name: c.name, key: target.key } : null;
          })
          .filter(Boolean),
      })),
    };
  }

  /**
   * Whether the encoded request fits the runtime's transport ceilings.
   *
   * This is a usability guard. The runtime remains authoritative: it applies
   * the same limits and refuses an oversized request with its own diagnostic.
   *
   * @returns {{ok: true} | {ok: false, error: string}}
   */
  checkLimits() {
    if (this.sources.length > MAX_PROJECT_SOURCES) {
      return { ok: false, error: `a project may hold at most ${MAX_PROJECT_SOURCES} files` };
    }
    for (const source of this.sources) {
      if (utf8Length(source.text) > MAX_SOURCE_BYTES) {
        return {
          ok: false,
          error: `"${source.name}" exceeds the ${MAX_SOURCE_BYTES} byte per-file limit`,
        };
      }
    }
    const encoded = utf8Length(JSON.stringify(this.toRequest()));
    if (encoded > MAX_PROJECT_BYTES) {
      return {
        ok: false,
        error: `the project exceeds the ${MAX_PROJECT_BYTES} byte transport limit`,
      };
    }
    return { ok: true };
  }
}
