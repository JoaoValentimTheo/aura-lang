"""
Smoke tests for the real-world example programs in examples/real/.

Each program must:
  1. pass `aura check` (parse + type + mutability checks), and
  2. run successfully with `aura run`, producing non-empty output.

These are the "dogfood" programs: they exercise the CLI, stdlib, and Python
interop the way a real user would.
"""

from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
REAL_DIR = ROOT / "examples" / "real"


def _aura(*args: str, env: dict | None = None) -> subprocess.CompletedProcess:
    full_env = os.environ.copy()
    if env:
        full_env.update(env)
    return subprocess.run(
        [sys.executable, "-m", "aura.cli", *args],
        capture_output=True,
        text=True,
        cwd=ROOT,
        env=full_env,
        timeout=120,
    )


PROGRAMS = {
    "wordcount": ["examples/real/sample.txt"],
    "todo_api": [],
    "async_fetch": [],
    "sales_pipeline": [],
}


@pytest.mark.parametrize("name,args", sorted(PROGRAMS.items()),
                         ids=sorted(PROGRAMS))
def test_real_example_checks(name: str, args: list[str]) -> None:
    """`aura check` must pass for every real example."""
    path = f"examples/real/{name}.aura"
    result = _aura("check", path)
    assert result.returncode == 0, (
        f"aura check failed for {path}\n"
        f"stdout: {result.stdout}\nstderr: {result.stderr}"
    )


@pytest.mark.parametrize("name,args", sorted(PROGRAMS.items()),
                         ids=sorted(PROGRAMS))
def test_real_example_runs(name: str, args: list[str]) -> None:
    """`aura run` must succeed and produce output for every real example."""
    path = f"examples/real/{name}.aura"
    env = {}
    if name == "async_fetch":
        # The example talks to its own loopback server; the HTTP client
        # blocks private addresses unless this is set.
        env["AURA_HTTP_ALLOW_PRIVATE"] = "1"
    result = _aura("run", path, *args, env=env)
    assert result.returncode == 0, (
        f"aura run failed for {path}\n"
        f"stdout: {result.stdout}\nstderr: {result.stderr}"
    )
    assert result.stdout.strip(), f"no output from {path}"


def test_real_example_readme_exists() -> None:
    """The real examples must be documented."""
    readme = REAL_DIR / "README.md"
    assert readme.exists(), "examples/real/README.md is missing"
    text = readme.read_text(encoding="utf-8")
    for name in PROGRAMS:
        assert f"{name}.aura" in text, f"{name}.aura not documented in README"
