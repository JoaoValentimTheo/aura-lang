#!/usr/bin/env bash
# End-user artifact smoke test.
#
# Builds the shipped pure-Rust binary the way a user obtains it and exercises
# representative programs through the *installed executable* (not `cargo test`),
# so a packaging or CLI regression that unit tests miss is caught. This closes
# the "no end-user install test" gap on the V1 readiness board.
#
# Usage:
#   scripts/artifact-smoke.sh [--release|--debug]
#
# Exit is non-zero on the first failed check. Nothing is written outside a
# temporary directory.

set -euo pipefail

REPO_ROOT=$(git rev-parse --show-toplevel)
cd "$REPO_ROOT"

PROFILE="release"
case "${1:-}" in
    --debug) PROFILE="debug" ;;
    --release | "") PROFILE="release" ;;
    *)
        echo "usage: scripts/artifact-smoke.sh [--release|--debug]" >&2
        exit 2
        ;;
esac

echo "[artifact-smoke] building pure-Rust binary (no CPython), profile=$PROFILE"
cargo build --locked --no-default-features \
    --features cli,repl,json,regex,time ${PROFILE:+--release} >/dev/null

if [ "$PROFILE" = "release" ]; then
    BIN="$REPO_ROOT/target/release/aura"
else
    BIN="$REPO_ROOT/target/debug/aura"
fi

pass=0
fail=0

# check <label> <expected> <actual>
check() {
    local label="$1" expected="$2" actual="$3"
    if [ "$expected" = "$actual" ]; then
        pass=$((pass + 1))
        echo "  ok   $label"
    else
        fail=$((fail + 1))
        echo "  FAIL $label"
        echo "       expected: $(printf '%q' "$expected")"
        echo "       actual:   $(printf '%q' "$actual")"
    fi
}

WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT

echo "[artifact-smoke] binary: $BIN"

# version ---------------------------------------------------------------
expected_version=$(grep -m1 '^version = ' Cargo.toml | sed -E 's/^version = "(.*)"/\1/')
check "version reports the crate version" \
    "aura $expected_version" "$("$BIN" version)"

# hello world (file) ----------------------------------------------------
printf 'fn main() {\n    print("hello, world")\n}\n' > "$WORK/hello.aura"
check "run hello.aura" "hello, world" "$("$BIN" run "$WORK/hello.aura")"

# check subcommand ------------------------------------------------------
check "check accepts a valid program" "" "$("$BIN" check "$WORK/hello.aura")"

# eval one-liner --------------------------------------------------------
check "eval prints arithmetic" "3" "$("$BIN" eval 'print(1 + 2)')"

# stdin ----------------------------------------------------------------
check "run from stdin" "7" "$(printf 'fn main() { print(3 + 4) }\n' | "$BIN" run -)"

# functions + recursion ------------------------------------------------
printf 'fn fib(n) -> int {\n    if n < 2 { return n }\n    return fib(n - 1) + fib(n - 2)\n}\nfn main() { print(fib(10)) }\n' > "$WORK/fib.aura"
check "recursive function" "55" "$("$BIN" run "$WORK/fib.aura")"

# modules --------------------------------------------------------------
mkdir -p "$WORK/proj"
printf 'pub fn area(w: int, h: int) -> int {\n    return w * h\n}\n' > "$WORK/proj/geom.aura"
printf 'use geom::area\nfn main() { print(area(3, 4)) }\n' > "$WORK/proj/main.aura"
check "module import and call" "12" "$("$BIN" run "$WORK/proj/main.aura")"

# collections + json ---------------------------------------------------
printf 'fn main() {\n    let m = {"a": [1, 2, 3]}\n    print(len(m["a"]))\n    print(json_encode(m))\n}\n' > "$WORK/coll.aura"
check "collections + json" '3
{"a":[1,2,3]}' "$("$BIN" run "$WORK/coll.aura")"

# error diagnostic + non-zero exit ------------------------------------
printf 'fn main() { print(1 + "a") }\n' > "$WORK/bad.aura"
set +e
"$BIN" run "$WORK/bad.aura" >/dev/null 2>&1
bad_exit=$?
set -e
if [ "$bad_exit" -ne 0 ]; then
    pass=$((pass + 1))
    echo "  ok   type error exits non-zero"
else
    fail=$((fail + 1))
    echo "  FAIL type error exits non-zero (got 0)"
fi

# missing file ---------------------------------------------------------
set +e
"$BIN" run "$WORK/does-not-exist.aura" >/dev/null 2>&1
missing_exit=$?
set -e
if [ "$missing_exit" -ne 0 ]; then
    pass=$((pass + 1))
    echo "  ok   missing file exits non-zero"
else
    fail=$((fail + 1))
    echo "  FAIL missing file exits non-zero (got 0)"
fi

# REPL round-trip ------------------------------------------------------
repl_out=$(printf 'let x = 40\nx + 2\n:quit\n' | "$BIN" repl)
case "$repl_out" in
    *42*)
        pass=$((pass + 1))
        echo "  ok   REPL evaluates across submissions"
        ;;
    *)
        fail=$((fail + 1))
        echo "  FAIL REPL evaluates across submissions"
        echo "       got: $(printf '%q' "$repl_out")"
        ;;
esac

# py absent in the pure-Rust build ------------------------------------
printf 'fn main() { py_eval("1 + 1") }\n' > "$WORK/py.aura"
set +e
py_out=$("$BIN" run "$WORK/py.aura" 2>&1)
set -e
case "$py_out" in
    *E5002*)
        pass=$((pass + 1))
        echo "  ok   py_* reports E5002 in the pure-Rust build"
        ;;
    *)
        fail=$((fail + 1))
        echo "  FAIL py_* reports E5002 (got: $(printf '%q' "$py_out"))"
        ;;
esac

echo "[artifact-smoke] $pass passed, $fail failed"
[ "$fail" -eq 0 ]
