"""Tests for the CLI's unified diagnostic reporting.

Every entry point must print ``file:line:column: SEVERITY [code]`` for type,
mutability and rule errors, and ``aura lint`` must use ``W`` codes.

The CLI is invoked as a subprocess so its process-global wiring (import hooks,
runtime aliases) cannot leak into the rest of the suite.
"""
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).parent.parent
sys.path.insert(0, str(ROOT))

MAIN = str(ROOT / "main.py")


def write(tmp_path, name, source):
    path = tmp_path / name
    path.write_text(source)
    return str(path)


def run(*args):
    return subprocess.run(
        [sys.executable, MAIN, *args],
        capture_output=True, text=True, timeout=30,
    )


# ---------------------------------------------------------------------------
# check
# ---------------------------------------------------------------------------

def test_check_reports_type_error_with_code(tmp_path):
    path = write(tmp_path, "t.aura", 'def main() { let x: int = "s" }')
    result = run("check", path)
    assert result.returncode == 1
    assert "[E101]" in result.stderr
    assert "t.aura:1:" in result.stderr


def test_check_reports_rule_error_with_code(tmp_path):
    path = write(tmp_path, "t.aura", "def main() {\n  return 1\n  print(2)\n}")
    result = run("check", path)
    assert result.returncode == 1
    assert "[E302]" in result.stderr
    assert "t.aura:3:" in result.stderr


def test_check_passes_clean_program(tmp_path):
    path = write(tmp_path, "t.aura", "def main() {\n  let x = 1\n  print(x)\n}")
    result = run("check", path)
    assert result.returncode == 0
    assert "passed" in result.stdout


def test_check_missing_file_returns_2(tmp_path):
    result = run("check", str(tmp_path / "nope.aura"))
    assert result.returncode == 2


# ---------------------------------------------------------------------------
# run (requires main)
# ---------------------------------------------------------------------------

def test_run_requires_main(tmp_path):
    path = write(tmp_path, "t.aura", "print(1)")
    result = run("run", path)
    assert result.returncode == 2
    assert "[E310]" in result.stderr


def test_run_invokes_main(tmp_path):
    path = write(tmp_path, "t.aura", 'def main() { print("hi") }')
    result = run("run", path)
    assert result.returncode == 0
    assert result.stdout == "hi\n"


def test_run_reports_mutability_with_code(tmp_path):
    path = write(tmp_path, "t.aura", "def main() {\n  let x = 1\n  x = 2\n}")
    result = run("run", path)
    assert result.returncode == 2
    assert "[E303]" in result.stderr


def test_run_forwards_arguments(tmp_path):
    path = write(tmp_path, "t.aura",
                 "def main(args: [string]) { print(args) }")
    result = run("run", path, "alpha", "beta")
    assert result.returncode == 0
    assert result.stdout.strip() == "['alpha', 'beta']"


# ---------------------------------------------------------------------------
# transpile
# ---------------------------------------------------------------------------

def test_transpile_reports_rules_with_code(tmp_path):
    path = write(tmp_path, "t.aura", "def main() {\n  break\n}")
    result = run("transpile", path)
    assert result.returncode == 2
    assert "[E004]" in result.stderr


# ---------------------------------------------------------------------------
# lint (W codes)
# ---------------------------------------------------------------------------

def test_lint_trailing_whitespace_is_w002(tmp_path):
    path = write(tmp_path, "t.aura", "def main() {\n    let x = 1   \n}\n")
    result = run("lint", path)
    assert result.returncode == 1
    assert "[W002]" in result.stderr
    assert "t.aura:2:" in result.stderr


def test_lint_long_line_is_w001(tmp_path):
    long_value = "1" * 120
    path = write(tmp_path, "t.aura",
                 f"def main() {{\n    let x = {long_value}\n}}\n")
    result = run("lint", path)
    assert result.returncode == 1
    assert "[W001]" in result.stderr


def test_lint_clean_file(tmp_path):
    path = write(tmp_path, "t.aura",
                 "def main() {\n    let x = 1\n    print(x)\n}\n")
    result = run("lint", path)
    assert result.returncode == 0
    assert "no style issues" in result.stdout


def test_lint_never_uses_e_codes(tmp_path):
    path = write(tmp_path, "t.aura", "def main() {\n    let x = 1 \n}\n")
    result = run("lint", path)
    assert "[E004]" not in result.stderr
    assert "[W" in result.stderr
