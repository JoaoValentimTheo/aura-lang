"""Backward-compatible entry point for the Aura CLI.

The implementation lives in ``aura.cli``; this shim keeps
``python3 main.py <command>`` working from a source checkout while the
installed console script is simply ``aura``.
"""
import os
import sys

# When run from a source checkout, prefer the local `aura` package.
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from aura.cli import main  # noqa: E402

if __name__ == '__main__':
    raise SystemExit(main())