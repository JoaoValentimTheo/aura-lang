"""Shared pytest configuration for the Aura test suite.

Installs the runtime aliases (``stdlib`` → ``aura.stdlib`` and friends) once,
before any test runs. Aura itself installs them lazily, but doing it up front
keeps module identity stable for the whole session: otherwise a test that
toggles the aliases can force a module to be re-imported under a different name,
which perturbs coverage attribution and ``isinstance`` checks.
"""
import sys
from pathlib import Path

ROOT = Path(__file__).parent.parent
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))


def pytest_configure(config):
    from aura.runtime import install_runtime_aliases

    install_runtime_aliases()