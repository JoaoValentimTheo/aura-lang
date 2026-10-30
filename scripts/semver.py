#!/usr/bin/env python3
"""SemVer 2.0.0 precedence, in exactly one place.

Release tooling compared Aura versions with `sort -V`, which is not SemVer:

  * it ranks a prerelease *above* the release it leads to
    (`1.0.0-rc.1` > `1.0.0`, the exact inverse of the specification),
  * it treats build metadata as significant (`1.0.0+a` vs `1.0.0+b`),
  * and it prints both inputs for equal versions, so `| head -n1` picks one
    arbitrarily.

Three separate release paths therefore disagreed about the same invariant
(ADR-0001: `LANGUAGE_VERSION <= RELEASE_VERSION`). They now all call this
module, so the precedence the preflight, the manifest generator, and the
release workflow enforce cannot drift apart again.

Usage:
  scripts/semver.py compare A B
      Print -1, 0, or 1 for A < B, A == B, A > B (build metadata ignored).

  scripts/semver.py check RELEASE LANGUAGE
      Exit 0 when both are valid and LANGUAGE <= RELEASE.

  Add `--quiet` to suppress this module's own diagnostics; a caller that
  reports its own message should use it and branch on the exit status.

Exit status:
  0  success (`compare` printed the comparison, or `check` passed)
  2  an argument is not valid SemVer
  3  `check` only: LANGUAGE is ahead of RELEASE
  64 usage error
"""

import re
import sys

# SemVer 2.0.0, section 9 (the normative grammar). Build metadata is matched
# but not captured: it carries no precedence.
_VERSION = re.compile(
    r"(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)"
    r"(?:-([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?"
    r"(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?",
    re.ASCII,
)

OK = 0
INVALID = 2
AHEAD = 3
USAGE = 64


def parse(value):
    """Return `((major, minor, patch), prerelease-or-None)` or None if invalid.

    `prerelease` is the raw dot-separated identifier list, kept as strings so
    that numeric identifiers can be compared numerically below.
    """
    match = _VERSION.fullmatch(value)
    if match is None:
        return None
    prerelease = match.group(4)
    if prerelease is None:
        return tuple(map(int, match.group(1, 2, 3))), None
    parts = prerelease.split(".")
    # Section 9: numeric identifiers must not include leading zeroes.
    if any(p.isdigit() and len(p) > 1 and p[0] == "0" for p in parts):
        return None
    return tuple(map(int, match.group(1, 2, 3))), parts


def compare(left, right):
    """Compare two parsed versions; return -1, 0, or 1."""
    if left[0] != right[0]:
        return (left[0] > right[0]) - (left[0] < right[0])
    a, b = left[1], right[1]
    # Section 11.3: a version with a prerelease has lower precedence.
    if a is None or b is None:
        return (a is None) - (b is None)
    for x, y in zip(a, b):
        xn, yn = x.isdigit(), y.isdigit()
        if xn and yn:
            xx, yy = int(x), int(y)
        elif xn != yn:
            # Section 11.4.3: numeric identifiers rank below alphanumeric ones.
            return -1 if xn else 1
        else:
            xx, yy = x, y
        if xx != yy:
            return (xx > yy) - (xx < yy)
    # Section 11.4.4: a larger set of fields has higher precedence.
    return (len(a) > len(b)) - (len(a) < len(b))


def usage(stream=sys.stderr):
    stream.write(__doc__)
    return USAGE


def main(argv):
    quiet = "--quiet" in argv
    args = [a for a in argv if a != "--quiet"]
    if not args:
        return usage()
    command, operands = args[0], args[1:]

    if command == "compare":
        if len(operands) != 2:
            return usage()
        left, right = (parse(v) for v in operands)
        if left is None or right is None:
            if not quiet:
                bad = [v for v, p in zip(operands, (left, right)) if p is None]
                print(f"semver: {' and '.join(map(repr, bad))} is not valid SemVer", file=sys.stderr)
            return INVALID
        print(compare(left, right))
        return OK

    if command == "check":
        if len(operands) != 2:
            return usage()
        release, language = (parse(v) for v in operands)
        if release is None or language is None:
            if not quiet:
                bad = [v for v, p in zip(operands, (release, language)) if p is None]
                print(f"semver: {' and '.join(map(repr, bad))} is not valid SemVer", file=sys.stderr)
            return INVALID
        if compare(language, release) > 0:
            if not quiet:
                print(
                    f"semver: LANGUAGE_VERSION ({operands[1]}) is ahead of release version ({operands[0]})",
                    file=sys.stderr,
                )
            return AHEAD
        return OK

    return usage()


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
