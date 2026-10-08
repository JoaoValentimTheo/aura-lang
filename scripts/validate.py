#!/usr/bin/env python3
"""Observable, sharded validation orchestrator for Aura.

Why this exists: `cargo test --locked --all-features` links and runs ~56 test
binaries in sequence. On macOS a freshly linked binary pays a large
first-execution cost (~59 s wall / ~0.03 s user CPU) for external linker
verification, so the monolithic run can appear to hang for tens of minutes with
no visible progress. This script makes every target observable, gives each an
explicit timeout, and never waits silently.

Guarantees:
  * target inventory is discovered from `cargo metadata`, never guessed;
  * each configuration is compiled once (`--no-run`) and then each test binary
    is executed separately with a wall-clock timeout;
  * every target prints an elapsed time and PASS / FAIL / TIMEOUT;
  * a resumable cache keyed by (HEAD, configuration, toolchain) skips targets
    already PASSed for an unchanged tree, and is invalidated per-target when
    that target's source is newer than the result;
  * exit status is real (non-zero when any target fails or times out).

Usage:
    python3 scripts/validate.py                 # all default configurations
    python3 scripts/validate.py --list          # print the target inventory
    python3 scripts/validate.py --config all-features --test keystone_json
    python3 scripts/validate.py --no-cache      # ignore the result cache

The watchdog envelope from AGENTS.md is reported: a target over 1.5x its
recorded baseline is flagged as an anomaly; over 2x it is flagged loudly. The
baseline is the last observed duration for that (config, target).
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
import sys
import time
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
CACHE_DIR = REPO / "target" / "validate-cache"

# Configuration name -> the exact cargo argument suffix after `test`.
CONFIGS: dict[str, list[str]] = {
    "all-features": ["--locked", "--all-features"],
    "bare": ["--locked", "--no-default-features"],
    "canonical": [
        "--locked",
        "--no-default-features",
        "--features",
        "cli,repl,json,regex,time",
    ],
}

BASELINE_PATH = CACHE_DIR / "baselines.json"
STATE_PATH = CACHE_DIR / "results.json"


def run(cmd: list[str], **kw) -> subprocess.CompletedProcess:
    return subprocess.run(cmd, cwd=REPO, text=True, capture_output=True, **kw)


def toolchain() -> str:
    for tool in ("rustc",):
        try:
            out = run([tool, "--version"]).stdout.strip()
            if out:
                return out
        except OSError:
            pass
    return "unknown"


def head_sha() -> str:
    return run(["git", "rev-parse", "HEAD"]).stdout.strip() or "no-git"


def discover_targets() -> list[str]:
    """Every `test`-kind target name, from cargo metadata (never guessed)."""
    out = run(["cargo", "metadata", "--no-deps", "--format-version", "1"])
    if out.returncode != 0:
        raise SystemExit(f"cargo metadata failed:\n{out.stderr}")
    meta = json.loads(out.stdout)
    names: set[str] = set()
    for pkg in meta["packages"]:
        for target in pkg["targets"]:
            if "test" in target["kind"] and target["name"] != pkg["name"]:
                names.add(target["name"])
    return sorted(names)


def load_json(path: Path) -> dict:
    try:
        return json.loads(path.read_text())
    except (OSError, ValueError):
        return {}


def save_json(path: Path, data: dict) -> None:
    CACHE_DIR.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(data, indent=2, sort_keys=True))


def target_fingerprint(config: str) -> str:
    """A key that identifies the tree state a result is valid for."""
    parts = [head_sha(), config, toolchain()]
    return hashlib.sha256("|".join(parts).encode()).hexdigest()[:16]


def cache_key(config: str, target: str) -> str:
    return f"{target_fingerprint(config)}/{config}/{target}"


def newest_source_mtime() -> float:
    newest = 0.0
    for base in ("src", "tests", "Cargo.toml", "Cargo.lock"):
        p = REPO / base
        if p.is_file():
            newest = max(newest, p.stat().st_mtime)
        elif p.is_dir():
            for root, _dirs, files in os.walk(p):
                for f in files:
                    if f.endswith(".rs"):
                        newest = max(newest, (Path(root) / f).stat().st_mtime)
    return newest


def compile_config(config: str, extra: list[str]) -> int:
    args = CONFIGS[config]
    print(f"[compile] {config}: cargo test {' '.join(args)} --no-run", flush=True)
    start = time.time()
    proc = run(["cargo", "test", *args, "--no-run", *extra])
    elapsed = time.time() - start
    if proc.returncode != 0:
        print(proc.stdout[-4000:])
        print(proc.stderr[-4000:], file=sys.stderr)
    print(f"[compile] {config}: exit={proc.returncode} in {elapsed:.1f}s", flush=True)
    return proc.returncode


def run_target(config: str, target: str, timeout: float) -> tuple[str, float, str]:
    args = CONFIGS[config]
    cmd = ["cargo", "test", *args, "--test", target, "--", "--test-threads=1"]
    start = time.time()
    try:
        proc = subprocess.run(
            cmd, cwd=REPO, text=True, capture_output=True, timeout=timeout
        )
    except subprocess.TimeoutExpired:
        return "TIMEOUT", time.time() - start, ""
    elapsed = time.time() - start
    tail = (proc.stdout or "").strip().splitlines()
    summary = next((l for l in reversed(tail) if "test result" in l), "")
    status = "PASS" if proc.returncode == 0 else "FAIL"
    return status, elapsed, summary


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--config", choices=[*CONFIGS, "all"], default="all")
    ap.add_argument("--test", action="append", default=[])
    ap.add_argument("--list", action="store_true")
    ap.add_argument("--no-cache", action="store_true")
    ap.add_argument("--timeout", type=float, default=600.0)
    args = ap.parse_args()

    targets = discover_targets()
    if args.test:
        wanted = set(args.test)
        missing = wanted - set(targets)
        if missing:
            print(f"unknown target(s): {sorted(missing)}", file=sys.stderr)
            return 2
        targets = [t for t in targets if t in wanted]

    if args.list:
        print(f"{len(targets)} test targets:")
        for t in targets:
            print(f"  {t}")
        return 0

    configs = list(CONFIGS) if args.config == "all" else [args.config]
    baselines = load_json(BASELINE_PATH)
    results = {} if args.no_cache else load_json(STATE_PATH)
    src_mtime = newest_source_mtime()

    rows: list[tuple[str, str, str, float, str]] = []
    failures = 0
    for config in configs:
        if compile_config(config, []) != 0:
            failures += 1
            continue
        fp = target_fingerprint(config)
        total = len(targets)
        for i, target in enumerate(targets, 1):
            key = cache_key(config, target)
            cached = results.get(key)
            if (
                not args.no_cache
                and cached
                and cached.get("status") == "PASS"
                and cached.get("mtime", 1e18) >= src_mtime
            ):
                print(f"[{i}/{total}] {config}/{target} ... CACHED PASS", flush=True)
                rows.append((config, target, "CACHED", 0.0, ""))
                continue
            status, elapsed, summary = run_target(config, target, args.timeout)
            base = baselines.get(f"{fingerprint_label()}/{config}/{target}")
            flag = ""
            if base:
                if elapsed > base * 2:
                    flag = "  <<< >2x baseline: STOP+DIAGNOSE"
                elif elapsed > base * 1.5:
                    flag = "  <1.5-2x baseline>"
            note = " [macOS first-exec]" if elapsed > 20 else ""
            print(
                f"[{i}/{total}] {config}/{target} ... {status} "
                f"{elapsed:.1f}s{note}{flag}  {summary}",
                flush=True,
            )
            rows.append((config, target, status, elapsed, summary))
            results[key] = {
                "status": status,
                "seconds": round(elapsed, 3),
                "mtime": src_mtime,
            }
            baselines[f"{fingerprint_label()}/{config}/{target}"] = round(elapsed, 3)
            # Persist incrementally so an interrupted run resumes rather than
            # restarting already-passed targets.
            if not args.no_cache:
                save_json(STATE_PATH, results)
                save_json(BASELINE_PATH, baselines)
            if status != "PASS":
                failures += 1

    if not args.no_cache:
        save_json(STATE_PATH, results)
        save_json(BASELINE_PATH, baselines)

    by_status: dict[str, int] = {}
    for _c, _t, status, _e, _s in rows:
        base = "PASS" if status.startswith("CACHED") else status
        by_status[base] = by_status.get(base, 0) + 1
    print("\n=== summary ===")
    print(f"configs: {', '.join(configs)}")
    print(f"targets: {len(targets)} per config")
    print(
        "PASS={PASS} FAIL={FAIL} TIMEOUT={TIMEOUT} CACHED={CACHED}".format(
            PASS=by_status.get("PASS", 0),
            FAIL=by_status.get("FAIL", 0),
            TIMEOUT=by_status.get("TIMEOUT", 0),
            CACHED=by_status.get("CACHED", 0),
        )
    )
    slow = [r for r in rows if r[3] > 20]
    if slow:
        print("targets >20s (macOS first-exec is expected on freshly linked binaries):")
        for c, t, s, e, _ in slow:
            print(f"  {c}/{t} {s} {e:.1f}s")
    return 1 if failures else 0


_FP_LABEL: str | None = None


def fingerprint_label() -> str:
    global _FP_LABEL
    if _FP_LABEL is None:
        _FP_LABEL = f"{head_sha()[:12]}-{toolchain().split()[-1] if toolchain() else 'x'}"
    return _FP_LABEL


if __name__ == "__main__":
    raise SystemExit(main())
