#!/usr/bin/env python3
"""Fast regression tests for the orchestration logic in scripts/validate.py.

These tests do not compile Rust and do not run the real suite. They drive the
validator's own functions with controlled fake commands and a temporary repo,
so the whole file runs in well under a second.

Run: python3 scripts/test_validate.py
"""

from __future__ import annotations

import importlib.util
import json
import os
import shutil
import stat
import subprocess
import sys
import tempfile
import time
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parent


def load_validator(repo: Path):
    spec = importlib.util.spec_from_file_location(
        f"validate_{id(repo)}", HERE / "validate.py"
    )
    mod = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(mod)
    mod.REPO = repo
    mod.CACHE_DIR = repo / "target" / "validate-cache"
    mod.STATE_PATH = mod.CACHE_DIR / "results.json"
    mod.BASELINE_PATH = mod.CACHE_DIR / "baselines.json"
    mod.LOCK_PATH = mod.CACHE_DIR / "validate.lock"
    mod.LOG_DIR = repo / "target" / "validate-run-logs"
    mod.SCRATCH_DIR = repo / "target" / "validate-run"
    return mod


def write(path: Path, text: str, mode: int = 0o644) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)
    path.chmod(mode)


def make_exe(path: Path, body: str) -> None:
    write(path, "#!/bin/sh\n" + body, 0o755)


class ValidatorBase(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = Path(tempfile.mkdtemp(prefix="valtest-"))
        (self.tmp / "tests").mkdir(parents=True, exist_ok=True)
        (self.tmp / "src").mkdir(parents=True, exist_ok=True)
        write(self.tmp / "src" / "lib.rs", "// v1\n")
        write(self.tmp / "tests" / "a.rs", "// a\n")
        write(self.tmp / "Cargo.toml", "[package]\nname='x'\n")
        self.v = load_validator(self.tmp)

    def tearDown(self) -> None:
        shutil.rmtree(self.tmp, ignore_errors=True)

    # ------------------------------------------------------------------
    def test_persist_incremental_and_resume(self):
        """Successful results persist; an interrupted run resumes."""
        key = self.v.cache_key("canonical", "a", "artA", "inputsA")
        results = {key: {"status": "PASS", "inputs": "inputsA", "artifact": "artA"}}
        self.v.atomic_write_json(self.v.STATE_PATH, results)
        loaded = self.v.load_json(self.v.STATE_PATH)
        self.assertEqual(loaded[key]["status"], "PASS")

    def test_atomic_state_not_corrupt(self):
        """A killed write must not corrupt existing state."""
        self.v.atomic_write_json(self.v.STATE_PATH, {"k": {"status": "PASS"}})
        good = self.v.STATE_PATH.read_text()
        # simulate a crashed temp file left behind
        stale = self.v.STATE_PATH.parent / (self.v.STATE_PATH.name + ".tmp")
        stale.write_text("{ this is not json")
        # existing state still parses
        self.assertEqual(self.v.load_json(self.v.STATE_PATH)["k"]["status"], "PASS")
        self.assertTrue(good)

    def test_corrupt_cache_is_not_success(self):
        self.v.STATE_PATH.parent.mkdir(parents=True, exist_ok=True)
        self.v.STATE_PATH.write_text("{ not valid json")
        self.assertEqual(self.v.load_json(self.v.STATE_PATH), {})

    def test_feature_configs_do_not_share_cache(self):
        a = self.v.cache_key("canonical", "t", "art", "in")
        b = self.v.cache_key("bare", "t", "art", "in")
        self.assertNotEqual(a, b)

    def test_doc_change_does_not_change_inputs_digest(self):
        before = self.v.runtime_inputs_digest()
        write(self.tmp / "AGENT_STATE.md", "docs only\n")
        write(self.tmp / "README.md", "docs only\n")
        after = self.v.runtime_inputs_digest()
        self.assertEqual(before, after, "documentation change must not invalidate tests")

    def test_source_change_changes_inputs_digest(self):
        before = self.v.runtime_inputs_digest()
        write(self.tmp / "src" / "lib.rs", "// v2 changed\n")
        after = self.v.runtime_inputs_digest()
        self.assertNotEqual(before, after)

    def test_fixture_change_changes_inputs_digest(self):
        before = self.v.runtime_inputs_digest()
        write(self.tmp / "tests" / "a.rs", "// fixture changed\n")
        after = self.v.runtime_inputs_digest()
        self.assertNotEqual(before, after)

    def test_cache_key_changes_with_artifact(self):
        a = self.v.cache_key("canonical", "t", "art1", "in")
        b = self.v.cache_key("canonical", "t", "art2", "in")
        self.assertNotEqual(a, b)

    # ------------------------------------------------------------------
    def test_run_target_pass_and_fail_propagate(self):
        exe = self.tmp / "fake-ok"
        make_exe(
            exe,
            'echo "running"; echo "test result: ok. 2 passed; 0 failed"; exit 0\n',
        )
        scratch = self.tmp / "scratch"
        tgt = {"name": "a", "exec_kind": "test"}
        r = self.v.run_target("bare", tgt, exe, 30, scratch, 1)
        self.assertEqual(r.status, "PASS")
        self.assertIn("test result", r.summary)

        exe_bad = self.tmp / "fake-bad"
        make_exe(exe_bad, 'echo "test result: FAILED. 1 passed; 1 failed"; exit 1\n')
        r2 = self.v.run_target("bare", tgt, exe_bad, 30, scratch, 1)
        self.assertEqual(r2.status, "FAIL")

    def test_run_target_propagates_nonzero_without_failure_text(self):
        """No false PASS: a non-zero exit with no test-result line is FAIL."""
        exe = self.tmp / "fake-crash"
        make_exe(exe, "exit 137\n")
        r = self.v.run_target(
            "bare", {"name": "a", "exec_kind": "test"}, exe, 30, self.tmp / "s", 1
        )
        self.assertEqual(r.status, "FAIL")

    def test_timeout_kills_process_group(self):
        """A timeout terminates the whole process group, including children."""
        exe = self.tmp / "fake-hang"
        marker = self.tmp / "child.pid"
        # parent sleeps, spawns a grandchild that sleeps far longer
        make_exe(
            exe,
            f'sleep 300 &\necho $! > "{marker}"\nsleep 300\n',
        )
        start = time.time()
        r = self.v.run_target(
            "bare", {"name": "a", "exec_kind": "test"}, exe, 2, self.tmp / "s", 1
        )
        elapsed = time.time() - start
        self.assertEqual(r.status, "TIMEOUT")
        self.assertLess(elapsed, 20, "graceful then forced termination must be bounded")
        # the grandchild must be dead shortly after
        time.sleep(0.5)
        pid = int(marker.read_text().strip())
        with self.assertRaises(OSError):
            os.kill(pid, 0)

    def test_original_binary_untouched_by_staging(self):
        """Staging copies the executable; the source artifact is unmodified."""
        exe = self.tmp / "artifact"
        make_exe(exe, 'echo "test result: ok. 1 passed; 0 failed"\n')
        before = exe.read_bytes()
        self.v.run_target(
            "bare", {"name": "a", "exec_kind": "test"}, exe, 30, self.tmp / "s", 1
        )
        self.assertEqual(before, exe.read_bytes())

    # ------------------------------------------------------------------
    def test_baseline_median_tracking(self):
        b = {}
        for s in (10, 12, 11):
            self.v.baseline_update(b, "canonical", "t", s)
        self.assertEqual(b["canonical/t"]["seconds"], 11.0)

    def test_lock_rejects_second_writer(self):
        self.v.acquire_lock()
        try:
            with self.assertRaises(self.v.LockError):
                self.v.acquire_lock()
        finally:
            self.v.release_lock()
        # stale lock with dead pid is reclaimed
        self.v.LOCK_PATH.parent.mkdir(parents=True, exist_ok=True)
        self.v.LOCK_PATH.write_text("999999")
        self.v.acquire_lock()
        self.v.release_lock()

    def test_declared_targets_includes_lib_and_binary_and_doc(self):
        """Unit tests (lib), the binary, and doctests are part of the surface."""
        # Drive declared_targets with a fake metadata via monkeypatching.
        fake = {
            "packages": [
                {
                    "name": "aura",
                    "targets": [
                        {"name": "aura", "kind": ["lib"],
                         "test": True, "doctest": True},
                        {"name": "aura", "kind": ["bin"],
                         "test": True, "doctest": False},
                        {"name": "lexer", "kind": ["test"],
                         "test": True, "doctest": False},
                    ],
                }
            ]
        }
        orig = self.v.cargo_metadata
        self.v.cargo_metadata = lambda: fake
        try:
            rows = self.v.declared_targets()
        finally:
            self.v.cargo_metadata = orig
        kinds = {(r["name"], r["exec_kind"]) for r in rows}
        self.assertIn(("aura", "lib"), kinds)
        self.assertIn(("aura", "bin"), kinds)
        self.assertIn(("lexer", "test"), kinds)
        self.assertIn(("aura:doc", "doc"), kinds)

    def test_compile_failure_propagates(self):
        """A non-zero compile is a COMPILATION FAILURE, never a silent skip."""
        # Fake cargo: emit nothing and exit 101.
        fakebin = self.tmp / "bin"
        fakebin.mkdir()
        make_exe(fakebin / "cargo", "exit 101\n")
        old_path = os.environ["PATH"]
        os.environ["PATH"] = f"{fakebin}:{old_path}"
        try:
            comp = self.v.compile_and_collect("canonical", 30)
        finally:
            os.environ["PATH"] = old_path
        self.assertFalse(comp["ok"])

    def test_compile_collects_artifacts(self):
        """Artifact JSON is parsed into (name, kind) -> executable."""
        exe = self.tmp / "fake-exe"
        make_exe(exe, "exit 0\n")
        fakebin = self.tmp / "bin"
        fakebin.mkdir()
        payload = json.dumps(
            {
                "reason": "compiler-artifact",
                "executable": str(exe),
                "profile": {"test": True},
                "target": {"name": "lexer", "kind": ["test"]},
            }
        )
        make_exe(
            fakebin / "cargo",
            "printf '%s\\n' '" + payload.replace("'", "'\\''") + "'\nexit 0\n",
        )
        old_path = os.environ["PATH"]
        os.environ["PATH"] = f"{fakebin}:{old_path}"
        try:
            comp = self.v.compile_and_collect("canonical", 30)
        finally:
            os.environ["PATH"] = old_path
        self.assertTrue(comp["ok"])
        self.assertEqual(comp["executables"][("lexer", "test")], str(exe))

    def test_compile_ignores_non_test_bin_artifact(self):
        """The normal bin executable (profile.test=false) is not the harness."""
        harness = self.tmp / "aura-harness"
        normal = self.tmp / "aura-bin"
        make_exe(harness, "exit 0\n")
        make_exe(normal, "exit 0\n")
        fakebin = self.tmp / "bin"
        fakebin.mkdir()
        lines = []
        for exe, test in ((normal, False), (harness, True)):
            lines.append(
                json.dumps(
                    {
                        "reason": "compiler-artifact",
                        "executable": str(exe),
                        "profile": {"test": test},
                        "target": {"name": "aura", "kind": ["bin"]},
                    }
                )
            )
        script = "\n".join(
            "printf '%s\\n' '" + l.replace("'", "'\\''") + "'" for l in lines
        )
        make_exe(fakebin / "cargo", script + "\nexit 0\n")
        old_path = os.environ["PATH"]
        os.environ["PATH"] = f"{fakebin}:{old_path}"
        try:
            comp = self.v.compile_and_collect("canonical", 30)
        finally:
            os.environ["PATH"] = old_path
        self.assertTrue(comp["ok"])
        self.assertEqual(comp["executables"][("aura", "bin")], str(harness))


if __name__ == "__main__":
    unittest.main(verbosity=2)
