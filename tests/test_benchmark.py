"""Performance benchmark suite for the Aura transpiler.

Measures parsing, transpilation, and full pipeline times across
representative source files. Run with:
    python -m pytest tests/test_benchmark.py -v --benchmark-only
or standalone:
    python tests/test_benchmark.py
"""

from __future__ import annotations

import time
from pathlib import Path

import pytest

from aura.parser.to_ast import parse_file
from aura.transpiler.transformer import Transformer

ROOT = Path(__file__).parent.parent
EXAMPLES = ROOT / "examples"

# Representative files ordered by complexity
BENCHMARK_FILES = [
    EXAMPLES / "hello.aura",
    EXAMPLES / "aup" / "pipeline.aura",
    EXAMPLES / "aup" / "builder.aura",
    EXAMPLES / "aup" / "strategy.aura",
    EXAMPLES / "aup" / "hybrid_crypto.aura",
]

# Filter to files that actually exist
BENCHMARK_FILES = [f for f in BENCHMARK_FILES if f.exists()]


def _bench_parse(path: Path, iterations: int = 100) -> float:
    """Return median parse time in seconds over ``iterations`` runs."""
    times = []
    for _ in range(iterations):
        t0 = time.perf_counter()
        parse_file(str(path))
        times.append(time.perf_counter() - t0)
    times.sort()
    return times[len(times) // 2]


def _bench_transform(path: Path, iterations: int = 100) -> float:
    """Return median transpilation time (parse + transform) in seconds."""
    ast = parse_file(str(path))
    times = []
    for _ in range(iterations):
        t0 = time.perf_counter()
        t = Transformer()
        t.transform(ast)
        times.append(time.perf_counter() - t0)
    times.sort()
    return times[len(times) // 2]


def _bench_full(path: Path, iterations: int = 50) -> float:
    """Return median full pipeline time (parse + transform) in seconds."""
    times = []
    for _ in range(iterations):
        t0 = time.perf_counter()
        ast = parse_file(str(path))
        t = Transformer()
        t.transform(ast)
        times.append(time.perf_counter() - t0)
    times.sort()
    return times[len(times) // 2]


class TestBenchmark:
    """Performance benchmarks (not assertions — informational)."""

    @pytest.mark.parametrize("path", BENCHMARK_FILES, ids=lambda p: p.name)
    def test_parse_median(self, path):
        median = _bench_parse(path)
        print(f"\n  PARSE  {path.name:30s} {median*1000:8.2f} ms")

    @pytest.mark.parametrize("path", BENCHMARK_FILES, ids=lambda p: p.name)
    def test_transform_median(self, path):
        median = _bench_transform(path)
        print(f"\n  XFORM  {path.name:30s} {median*1000:8.2f} ms")

    @pytest.mark.parametrize("path", BENCHMARK_FILES, ids=lambda p: p.name)
    def test_full_pipeline_median(self, path):
        median = _bench_full(path)
        print(f"\n  FULL   {path.name:30s} {median*1000:8.2f} ms")

    def test_throughput(self):
        """Measure lines-per-second throughput on the largest file."""
        if not BENCHMARK_FILES:
            pytest.skip("No benchmark files available")
        largest = max(BENCHMARK_FILES, key=lambda f: f.stat().st_size)
        lines = len(largest.read_text().splitlines())
        iterations = 20
        times = []
        for _ in range(iterations):
            t0 = time.perf_counter()
            ast = parse_file(str(largest))
            t = Transformer()
            t.transform(ast)
            times.append(time.perf_counter() - t0)
        median = sorted(times)[len(times) // 2]
        lps = lines / median if median > 0 else 0
        print(f"\n  THROUGHPUT {largest.name}: {lps:.0f} lines/sec "
              f"({lines} lines in {median*1000:.1f}ms)")


if __name__ == "__main__":
    print("Aura Transpiler Benchmark Suite")
    print("=" * 60)
    for path in BENCHMARK_FILES:
        size = path.stat().st_size
        lines = len(path.read_text().splitlines())
        print(f"\n  {path.name} ({lines} lines, {size} bytes)")
        parse_t = _bench_parse(path, iterations=200)
        transform_t = _bench_transform(path, iterations=200)
        full_t = _bench_full(path, iterations=100)
        print(f"    Parse:     {parse_t*1000:8.2f} ms")
        print(f"    Transform: {transform_t*1000:8.2f} ms")
        print(f"    Full:      {full_t*1000:8.2f} ms")
    print("\n" + "=" * 60)
