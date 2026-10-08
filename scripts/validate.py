#!/usr/bin/env python3
"""Observable, resumable, sharded validation orchestrator for Aura.

Root cause this script fixes
----------------------------
On macOS, the *first* execution of a freshly linked Rust test binary costs
30-65 s of wall time while using ~0.00-0.03 s of CPU. The cost is macOS's
security assessment of the freshly produced (ad-hoc linker-signed) Mach-O
image. It is *not* test execution:

    fresh link -> first exec : 35-62 s wall / 0.00 s user CPU
    same binary, 2nd exec    : 0.01 s
    same fresh binary copied to a small scratch directory, first exec : 0.4 s

The last observation is the fix. `target/debug/deps` accumulates hundreds of
thousands of entries over a development history (758,385 here); placing the
freshly linked image in a small scratch directory removes the one-time
assessment delay almost entirely. Copying the image out of the deps directory
does not change the binary's identity, so test semantics are preserved;
tests find their fixtures through `env!("CARGO_MANIFEST_DIR")` (compile-time)
and through the child `aura` binary path (`env!("CARGO_BIN_EXE_aura")`), both
absolute and independent of where the test image itself lives.

What this script does
---------------------
* Discovers the *complete* Rust test surface from `cargo metadata` plus the
  real `cargo test --no-run` artifact stream: library unit tests, every
  integration test, the binary test target, and doctests. Unit tests are never
  silently dropped (the previous script skipped the lib target because its
  name equals the package name).
* Compiles each configuration once with live, bounded, streamed progress and
  a separate compile timeout; a compile failure is reported as
  COMPILATION FAILURE and stops the configuration.
* Runs each target as its *own* process group with a wall-clock deadline
  (graceful SIGTERM, then SIGKILL after a bounded grace period), so a timeout
  reaps the whole shard and never orphans test executables.
* Persists results atomically and incrementally, so an interrupted run
  resumes from the unfinished targets. Forced fresh runs still persist.
* Keys the cache on the compiled artifact digest plus a content digest of the
  runtime inputs, the toolchain, the target, and the execution arguments. A
  documentation-only commit does not invalidate Rust test binaries; a source
  or fixture change does. Different feature configurations can never share a
  cache entry. A corrupt cache is treated as absent, never as success.
* Emits `[CURRENT / TOTAL] config/target ... PASS|FAIL|TIMEOUT elapsed` and
  preserves failure logs. Exit status is real.

Usage
-----
    python3 scripts/validate.py --list
    python3 scripts/validate.py --config canonical
    python3 scripts/validate.py --config all
    python3 scripts/validate.py --config bare
    python3 scripts/validate.py --config canonical --test lexer
    python3 scripts/validate.py --config canonical --fresh     # ignore cache, persist
    python3 scripts/validate.py --config canonical --no-cache  # alias for --fresh
    python3 scripts/validate.py --config canonical --no-persist
    python3 scripts/validate.py --status
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import signal
import subprocess
import sys
import tempfile
import time
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
CACHE_DIR = REPO / "target" / "validate-cache"
STATE_PATH = CACHE_DIR / "results.json"
BASELINE_PATH = CACHE_DIR / "baselines.json"
LOCK_PATH = CACHE_DIR / "validate.lock"
LOG_DIR = CACHE_DIR / "logs"
SCRATCH_DIR = REPO / "target" / "validate-run"

# Configuration name -> exact cargo arguments after `test`.
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

# Files/dirs whose content can influence a test at *runtime* (read from disk
# rather than compiled in). Their content digest is part of the cache key.
RUNTIME_INPUT_ROOTS = [
    "src",
    "tests",
    "examples",
    "docs",
    "playground/web",
    "Cargo.toml",
    "Cargo.lock",
    "build.rs",
]

COMPILE_FAILURE = "COMPILE_FAILURE"


# --------------------------------------------------------------------------
# small helpers
# --------------------------------------------------------------------------
def run(cmd: list[str], **kw) -> subprocess.CompletedProcess:
    return subprocess.run(cmd, cwd=REPO, text=True, capture_output=True, **kw)


def toolchain() -> str:
    try:
        out = run(["rustc", "--version"]).stdout.strip()
        return out or "unknown"
    except OSError:
        return "unknown"


def host_triple() -> str:
    try:
        out = run(["rustc", "-vV"]).stdout
        for line in out.splitlines():
            if line.startswith("host:"):
                return line.split(":", 1)[1].strip()
    except OSError:
        pass
    return "unknown"


def git_sha() -> str:
    try:
        return run(["git", "rev-parse", "HEAD"]).stdout.strip() or "no-git"
    except OSError:
        return "no-git"


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def load_json(path: Path) -> dict:
    try:
        data = json.loads(path.read_text())
        return data if isinstance(data, dict) else {}
    except (OSError, ValueError):
        return {}


def atomic_write_json(path: Path, data: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, tmp = tempfile.mkstemp(dir=str(path.parent), prefix=path.name, suffix=".tmp")
    try:
        with os.fdopen(fd, "w") as f:
            json.dump(data, f, indent=2, sort_keys=True)
            f.flush()
            os.fsync(f.fileno())
        os.replace(tmp, path)
    finally:
        if os.path.exists(tmp):
            os.unlink(tmp)


# --------------------------------------------------------------------------
# cache key / input digest
# --------------------------------------------------------------------------
def runtime_inputs_digest() -> str:
    """Content digest over every file a test may read at runtime."""
    h = hashlib.sha256()
    for rel in RUNTIME_INPUT_ROOTS:
        p = REPO / rel
        if not p.exists():
            continue
        if p.is_file():
            h.update(rel.encode())
            h.update(p.read_bytes())
            continue
        for root, dirs, files in os.walk(p):
            dirs.sort()
            for name in sorted(files):
                fp = Path(root) / name
                try:
                    relp = fp.relative_to(REPO).as_posix()
                except ValueError:
                    relp = fp.as_posix()
                h.update(relp.encode())
                try:
                    h.update(fp.read_bytes())
                except OSError:
                    h.update(b"<unreadable>")
    return h.hexdigest()


def execution_args(config: str) -> list[str]:
    return ["cargo", "test", *CONFIGS[config]]


def cache_key(config: str, target: str, artifact_digest: str, inputs_digest: str) -> str:
    parts = [
        toolchain(),
        host_triple(),
        config,
        target,
        artifact_digest,
        inputs_digest,
        " ".join(execution_args(config)),
    ]
    return hashlib.sha256("|".join(parts).encode()).hexdigest()


# --------------------------------------------------------------------------
# target discovery
# --------------------------------------------------------------------------
def cargo_metadata() -> dict:
    out = run(["cargo", "metadata", "--no-deps", "--format-version", "1"])
    if out.returncode != 0:
        raise SystemExit(f"cargo metadata failed:\n{out.stderr}")
    return json.loads(out.stdout)


def declared_targets() -> list[dict]:
    """The full declared Rust test surface from cargo metadata.

    Each entry: {name, kind, exec_kind, required_features}. `exec_kind` is how
    the target is executed: 'lib', 'bin', 'test', or 'doc'. `required_features`
    is the target's cargo `required-features` list, so a target cargo will not
    build under the active configuration is reported as SKIP, not FAIL.
    """
    meta = cargo_metadata()
    rows: list[dict] = []
    for pkg in meta["packages"]:
        for t in pkg["targets"]:
            kinds = t["kind"]
            rf = list(t.get("required-features") or [])
            if t.get("test") and "lib" in kinds:
                rows.append(
                    {"name": t["name"], "kind": "lib", "exec_kind": "lib",
                     "required_features": rf}
                )
            if t.get("test") and "bin" in kinds:
                rows.append(
                    {"name": t["name"], "kind": "bin", "exec_kind": "bin",
                     "required_features": rf}
                )
            if t.get("test") and "test" in kinds:
                rows.append(
                    {"name": t["name"], "kind": "test", "exec_kind": "test",
                     "required_features": rf}
                )
            if t.get("doctest") and "lib" in kinds:
                rows.append(
                    {"name": f"{t['name']}:doc", "kind": "doc", "exec_kind": "doc",
                     "required_features": rf}
                )
    seen: set[tuple[str, str]] = set()
    out: list[dict] = []
    for r in rows:
        k = (r["name"], r["exec_kind"])
        if k not in seen:
            seen.add(k)
            out.append(r)
    return out


def compile_and_collect(config: str, compile_timeout: float) -> dict:
    """Compile the config once, streaming progress; return artifact map.

    Returns {'ok': bool, 'executables': {(name, kind): path}, 'seconds': float}.
    A non-zero cargo exit is a COMPILATION FAILURE. stdout is parsed for
    artifact JSON while stderr is drained to a log file every 5 s so the pipe
    can never deadlock and progress stays observable.
    """
    import threading

    args = CONFIGS[config]
    cmd = ["cargo", "test", *args, "--no-run", "--message-format=json"]
    print(
        f"[compile] {config}: cargo test {' '.join(args)} --no-run",
        flush=True,
    )
    start = time.time()
    executables: dict[tuple[str, str], str] = {}
    stdout_lines: list[str] = []
    stderr_tail: list[str] = []
    try:
        stderr_log = (CACHE_DIR / f"compile-{config}.stderr.log").open("w")
    except OSError:
        stderr_log = None

    try:
        proc = subprocess.Popen(
            cmd,
            cwd=REPO,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            start_new_session=True,
        )
    except OSError as e:
        print(f"[compile] {config}: cannot start cargo: {e}", file=sys.stderr)
        return {"ok": False, "executables": {}, "seconds": 0.0, "log": ""}

    def drain_stderr() -> None:
        assert proc.stderr is not None
        try:
            for line in proc.stderr:
                stderr_tail.append(line)
                if len(stderr_tail) > 400:
                    del stderr_tail[:100]
                if stderr_log is not None:
                    stderr_log.write(line)
                    stderr_log.flush()
        finally:
            try:
                proc.stderr.close()
            except OSError:
                pass

    t = threading.Thread(target=drain_stderr, daemon=True)
    t.start()

    deadline = start + compile_timeout
    next_report = start + 5.0
    timed_out = False
    seen_artifacts = 0
    assert proc.stdout is not None
    for line in proc.stdout:
        stdout_lines.append(line)
        if '"reason":"compiler-artifact"' in line:
            seen_artifacts += 1
        now = time.time()
        if now > deadline:
            timed_out = True
            _kill_group(proc, "compile")
            break
        if now >= next_report:
            next_report = now + 5.0
            print(
                f"[compile] {config}: ...{now - start:.0f}s "
                f"({seen_artifacts} artifacts so far)",
                flush=True,
            )
    try:
        proc.stdout.close()
    except OSError:
        pass
    try:
        proc.wait(timeout=15)
    except subprocess.TimeoutExpired:
        _kill_group(proc, "compile")
        timed_out = True
    t.join(timeout=5)
    if stderr_log is not None:
        stderr_log.close()

    if timed_out:
        print(
            f"[compile] {config}: COMPILATION FAILURE (timeout {compile_timeout:.0f}s)",
            file=sys.stderr,
        )
        for line in stderr_tail[-20:]:
            print("  " + line.rstrip(), file=sys.stderr)
        return {
            "ok": False,
            "executables": {},
            "seconds": time.time() - start,
            "log": "".join(stdout_lines[-200:]),
        }

    for line in stdout_lines:
        line = line.strip()
        if not line.startswith("{"):
            continue
        try:
            msg = json.loads(line)
        except ValueError:
            continue
        if msg.get("reason") != "compiler-artifact":
            continue
        exe = msg.get("executable")
        if not exe:
            continue
        target = msg.get("target", {})
        name = target.get("name")
        kinds = target.get("kind") or []
        is_test_harness = bool(msg.get("profile", {}).get("test"))
        if "lib" in kinds:
            # Only the test harness (profile.test) is executable test code; a
            # non-test lib artifact is not a runnable test binary.
            if is_test_harness:
                executables[(name, "lib")] = exe
        elif "bin" in kinds:
            # A bin target emits both its normal executable and (with
            # `cargo test`) a separate test harness; only the harness accepts
            # libtest arguments such as `--test-threads`.
            if is_test_harness:
                executables[(name, "bin")] = exe
        elif "test" in kinds:
            if is_test_harness:
                executables[(name, "test")] = exe

    elapsed = time.time() - start
    ok = proc.returncode == 0
    print(
        f"[compile] {config}: exit={proc.returncode} in {elapsed:.1f}s",
        flush=True,
    )
    if not ok:
        for line in stderr_tail[-40:]:
            print("  " + line.rstrip(), file=sys.stderr)
    return {
        "ok": ok,
        "executables": executables,
        "seconds": elapsed,
        "log": "".join(stdout_lines[-200:]),
    }


# --------------------------------------------------------------------------
# process-group execution
# --------------------------------------------------------------------------
def _kill_group(proc: subprocess.Popen, label: str, grace: float = 5.0) -> None:
    """Terminate a process group: SIGTERM, then SIGKILL after a grace period."""
    if proc.poll() is not None:
        return
    try:
        pgid = os.getpgid(proc.pid)
    except OSError:
        try:
            proc.kill()
        except OSError:
            pass
        return
    try:
        os.killpg(pgid, signal.SIGTERM)
    except OSError:
        pass
    deadline = time.time() + grace
    while time.time() < deadline:
        if proc.poll() is not None:
            return
        time.sleep(0.05)
    try:
        os.killpg(pgid, signal.SIGKILL)
    except OSError:
        pass


class TargetResult:
    def __init__(self, status: str, seconds: float, summary: str, log_path: Path):
        self.status = status
        self.seconds = seconds
        self.summary = summary
        self.log_path = log_path


def run_target(
    config: str, target: dict, exe: Path, timeout: float, scratch: Path,
    test_threads: int = 1,
) -> TargetResult:
    """Run one target as its own process group with a wall-clock deadline.

    Test/binary targets are copied into a small scratch directory before their
    first execution to avoid the macOS fresh-image assessment penalty of the
    huge deps directory (see module docstring).
    """
    label = f"{config}/{target['name']}"
    kind = target["exec_kind"]
    log_path = LOG_DIR / f"{config}__{target['name'].replace(':', '_')}.log"
    log_path.parent.mkdir(parents=True, exist_ok=True)

    if kind == "doc":
        cmd = ["cargo", "test", *CONFIGS[config], "--doc", "--",
               f"--test-threads={test_threads}"]
        run_cwd = REPO
        base = ["cargo", "test", "--doc"]
    else:
        run_cwd = REPO
        scratch.mkdir(parents=True, exist_ok=True)
        cp = scratch / f"{config}__{target['name'].replace(':', '_')}"
        try:
            shutil.copy2(exe, cp)
            cp.chmod(0o755)
        except OSError as e:
            return TargetResult(
                "FAIL", 0.0, f"cannot stage executable: {e}", log_path
            )
        cmd = [str(cp), f"--test-threads={test_threads}"]
        run_cwd = REPO

    start = time.time()
    logf = log_path.open("w")
    try:
        proc = subprocess.Popen(
            cmd,
            cwd=run_cwd,
            text=True,
            stdout=logf,
            stderr=subprocess.STDOUT,
            start_new_session=True,
        )
    except OSError as e:
        logf.close()
        return TargetResult("FAIL", time.time() - start, f"cannot start: {e}", log_path)

    timed_out = False
    try:
        proc.wait(timeout=timeout)
    except subprocess.TimeoutExpired:
        timed_out = True
        _kill_group(proc, label)
        try:
            proc.wait(timeout=10)
        except subprocess.TimeoutExpired:
            pass
    finally:
        logf.close()

    elapsed = time.time() - start
    text = log_path.read_text(errors="replace")
    summary = next(
        (l.strip() for l in reversed(text.splitlines()) if "test result" in l), ""
    )
    if timed_out:
        return TargetResult("TIMEOUT", elapsed, summary, log_path)
    status = "PASS" if proc.returncode == 0 else "FAIL"
    return TargetResult(status, elapsed, summary, log_path)


# --------------------------------------------------------------------------
# locking / state
# --------------------------------------------------------------------------
class LockError(RuntimeError):
    pass


def acquire_lock() -> None:
    CACHE_DIR.mkdir(parents=True, exist_ok=True)
    if LOCK_PATH.exists():
        try:
            pid = int(LOCK_PATH.read_text().strip())
        except (OSError, ValueError):
            pid = -1
        if pid > 0 and _pid_alive(pid):
            raise LockError(
                f"another validation run is active (pid {pid}); "
                f"lock: {LOCK_PATH}"
            )
        LOCK_PATH.unlink(missing_ok=True)
    LOCK_PATH.write_text(str(os.getpid()))


def release_lock() -> None:
    try:
        if LOCK_PATH.exists() and LOCK_PATH.read_text().strip() == str(os.getpid()):
            LOCK_PATH.unlink()
    except OSError:
        pass


def _pid_alive(pid: int) -> bool:
    try:
        os.kill(pid, 0)
    except OSError:
        return False
    return True


# --------------------------------------------------------------------------
# watchdog baseline
# --------------------------------------------------------------------------
def baseline_lookup(baselines: dict, config: str, target: str) -> float | None:
    key = f"{config}/{target}"
    entry = baselines.get(key)
    if isinstance(entry, dict):
        v = entry.get("seconds")
        return float(v) if isinstance(v, (int, float)) else None
    if isinstance(entry, (int, float)):
        return float(entry)
    return None


def baseline_update(baselines: dict, config: str, target: str, seconds: float) -> None:
    key = f"{config}/{target}"
    entry = baselines.get(key)
    samples: list[float] = []
    if isinstance(entry, dict):
        samples = [float(x) for x in entry.get("samples", []) if isinstance(x, (int, float))]
    elif isinstance(entry, (int, float)):
        samples = [float(entry)]
    samples.append(round(seconds, 3))
    samples = samples[-8:]
    baselines[key] = {"samples": samples, "seconds": round(sorted(samples)[len(samples) // 2], 3)}


# --------------------------------------------------------------------------
# main
# --------------------------------------------------------------------------
def main() -> int:
    ap = argparse.ArgumentParser(description="Aura validation orchestrator")
    ap.add_argument("--config", choices=[*CONFIGS, "all"], default="all")
    ap.add_argument("--test", action="append", default=[])
    ap.add_argument("--list", action="store_true")
    ap.add_argument("--status", action="store_true")
    ap.add_argument("--fresh", action="store_true", help="ignore cache; still persist")
    ap.add_argument("--no-cache", action="store_true", help="alias for --fresh")
    ap.add_argument("--no-persist", action="store_true", help="do not write results")
    ap.add_argument("--timeout", type=float, default=600.0, help="per-target seconds")
    ap.add_argument("--compile-timeout", type=float, default=1800.0)
    ap.add_argument("--test-threads", type=int, default=1)
    args = ap.parse_args()
    fresh = args.fresh or args.no_cache

    targets = declared_targets()
    if args.test:
        wanted = set(args.test)
        available = {t["name"] for t in targets}
        missing = wanted - available
        if missing:
            print(f"unknown target(s): {sorted(missing)}", file=sys.stderr)
            return 2
        targets = [t for t in targets if t["name"] in wanted]

    if args.list:
        print(f"{len(targets)} test targets:")
        for t in targets:
            print(f"  {t['exec_kind']:5} {t['name']}")
        return 0

    if args.status:
        results = load_json(STATE_PATH)
        by: dict[str, int] = {}
        for v in results.values():
            s = v.get("status", "?") if isinstance(v, dict) else "?"
            by[s] = by.get(s, 0) + 1
        print(f"cache entries: {len(results)}")
        for k in sorted(by):
            print(f"  {k}: {by[k]}")
        print(f"state: {STATE_PATH}")
        return 0

    configs = list(CONFIGS) if args.config == "all" else [args.config]

    try:
        acquire_lock()
    except LockError as e:
        print(str(e), file=sys.stderr)
        return 2

    try:
        return _run(args, configs, targets, fresh)
    finally:
        release_lock()


def config_enabled_features(config: str) -> set[str]:
    """The feature set the configuration enables (best effort, from CONFIGS)."""
    if config == "all-features":
        return {"<all>"}
    args = CONFIGS[config]
    enabled: set[str] = set()
    i = 0
    while i < len(args):
        if args[i] == "--features" and i + 1 < len(args):
            enabled.update(f for f in args[i + 1].split(",") if f)
            i += 2
            continue
        if args[i].startswith("--features="):
            enabled.update(f for f in args[i].split("=", 1)[1].split(",") if f)
        i += 1
    return enabled


def target_is_skipped(target: dict, enabled: set[str]) -> bool:
    if "<all>" in enabled:
        return False
    required = set(target.get("required_features") or [])
    return not required.issubset(enabled)


def _run(args, configs: list[str], targets: list[dict], fresh: bool) -> int:
    results = {} if fresh else load_json(STATE_PATH)
    baselines = load_json(BASELINE_PATH)
    inputs_digest = runtime_inputs_digest()

    SCRATCH_DIR.mkdir(parents=True, exist_ok=True)
    LOG_DIR.mkdir(parents=True, exist_ok=True)

    rows: list[tuple[str, str, str, float, str]] = []
    failures = 0
    any_compiled = False

    for config in configs:
        comp = compile_and_collect(config, args.compile_timeout)
        if not comp["ok"]:
            failures += 1
            print(
                f"[{config}] COMPILATION FAILURE — configuration aborted",
                file=sys.stderr,
            )
            rows.append((config, "<compile>", COMPILE_FAILURE, comp["seconds"], ""))
            continue
        any_compiled = True
        executables = comp["executables"]
        enabled = config_enabled_features(config)

        flat: list[tuple[dict, Path | None]] = []
        for t in targets:
            if target_is_skipped(t, enabled):
                flat.append((t, None))
                continue
            if t["exec_kind"] == "doc":
                flat.append((t, None))
                continue
            exe = executables.get((t["name"], t["exec_kind"]))
            if exe is None:
                flat.append((t, None))
            else:
                flat.append((t, Path(exe)))

        total = len(flat)
        for i, (t, exe) in enumerate(flat, 1):
            label = f"{config}/{t['name']}"
            if exe is None and t["exec_kind"] != "doc" and target_is_skipped(t, enabled):
                reason = "required-features=" + ",".join(t.get("required_features") or [])
                print(f"[{i}/{total}] {label} ... SKIP  {reason}", flush=True)
                rows.append((config, t["name"], "SKIP", 0.0, reason))
                continue
            artifact_digest = ""
            if exe is not None:
                try:
                    artifact_digest = sha256_file(exe)
                except OSError:
                    artifact_digest = ""
            key = cache_key(config, t["name"], artifact_digest, inputs_digest)
            cached = results.get(key)
            if (
                not fresh
                and isinstance(cached, dict)
                and cached.get("status") == "PASS"
                and cached.get("inputs") == inputs_digest
                and cached.get("artifact") == artifact_digest
            ):
                print(f"[{i}/{total}] {label} ... CACHED PASS", flush=True)
                rows.append((config, t["name"], "CACHED", 0.0, ""))
                continue

            if exe is None and t["exec_kind"] != "doc":
                print(
                    f"[{i}/{total}] {label} ... FAIL 0.0s  "
                    f"no compiled artifact for target",
                    flush=True,
                )
                rows.append((config, t["name"], "FAIL", 0.0, "no artifact"))
                failures += 1
                continue

            res = run_target(
                config, t, exe, args.timeout, SCRATCH_DIR, args.test_threads
            )

            base = baseline_lookup(baselines, config, t["name"])
            flag = ""
            if base:
                if res.seconds > base * 2:
                    flag = "  <<< >2x baseline: STOP+DIAGNOSE"
                elif res.seconds > base * 1.5:
                    flag = "  <1.5-2x baseline>"
            note = ""
            if res.status == "PASS" and res.seconds > 5:
                note = " [slow]"
            print(
                f"[{i}/{total}] {label} ... {res.status} "
                f"{res.seconds:.1f}s{note}{flag}  {res.summary}",
                flush=True,
            )
            rows.append((config, t["name"], res.status, res.seconds, res.summary))

            results[key] = {
                "status": res.status,
                "seconds": round(res.seconds, 3),
                "inputs": inputs_digest,
                "artifact": artifact_digest,
                "config": config,
                "target": t["name"],
                "summary": res.summary,
                "when": int(time.time()),
            }
            baseline_update(baselines, config, t["name"], res.seconds)

            # Persist incrementally regardless of --fresh: an interrupted run
            # must be resumable.
            if not args.no_persist:
                atomic_write_json(STATE_PATH, results)
                atomic_write_json(BASELINE_PATH, baselines)

            if res.status != "PASS":
                failures += 1
                if res.log_path.exists():
                    tail = res.log_path.read_text(errors="replace").splitlines()[-20:]
                    print("      ---- log tail ----", flush=True)
                    for line in tail:
                        print(f"      {line}", flush=True)

    if not args.no_persist:
        atomic_write_json(STATE_PATH, results)
        atomic_write_json(BASELINE_PATH, baselines)

    by_status: dict[str, int] = {}
    for _c, _t, status, _e, _s in rows:
        base = "CACHED" if status == "CACHED" else status
        by_status[base] = by_status.get(base, 0) + 1
    print("\n=== summary ===")
    print(f"configs: {', '.join(configs)}")
    print(f"targets: {len(targets)} declared per config")
    print(
        "PASS={PASS} FAIL={FAIL} TIMEOUT={TIMEOUT} CACHED={CACHED} "
        "SKIP={SKIP} COMPILE_FAILURE={CF}".format(
            PASS=by_status.get("PASS", 0),
            FAIL=by_status.get("FAIL", 0),
            TIMEOUT=by_status.get("TIMEOUT", 0),
            CACHED=by_status.get("CACHED", 0),
            SKIP=by_status.get("SKIP", 0),
            CF=by_status.get(COMPILE_FAILURE, 0),
        )
    )
    slow = [r for r in rows if r[3] > 5]
    if slow:
        print("targets >5s:")
        for c, t, s, e, _ in slow:
            print(f"  {c}/{t} {s} {e:.1f}s")
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())
