"""One-off migration: add `mut` to `let` declarations whose bindings are
reassigned, so the corpus complies with Aura's immutability rules.

Run from the repo root:
    .venv/bin/python tools/migrate_mut.py [paths...]
Defaults to examples/ and tests/.
"""
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from aura.parser.to_ast import Parser, Tokenizer
from aura.transpiler.semantics import MutabilityChecker


def violations_for(ast):
    checker = MutabilityChecker()
    checker.check_program(ast)
    return set(checker.violations)


def rewrite(source, names):
    if not names:
        return source, 0
    # Strip comments for matching but keep line structure (same line count).
    changed = 0

    out_lines = []
    for line in source.split('\n'):
        stripped = line.lstrip()
        if stripped.startswith('let ') or stripped.startswith('\tlet ') or re.match(r'let\s', stripped):
            # Extract declared names up to ':' or '='.
            m = re.match(
                r'^(\s*)(.*?\blet)\b(\s+(?:public|private|protected|static|volatile)\b)*\s+',
                line,
            )
            if m and ' mut ' not in line and not re.search(r'\blet\s+mut\b', line):
                rest = line[m.end():]
                # names part: everything before ':' or '=' or ';' or end.
                names_part = re.split(r'[:=;]', rest, maxsplit=1)[0]
                declared = set(re.findall(r'[A-Za-z_]\w*', names_part))
                if declared & names:
                    # Insert `mut ` right after `let`.
                    insert_at = m.end()
                    line = line[:insert_at] + 'mut ' + line[insert_at:]
                    changed += 1
        out_lines.append(line)
    return '\n'.join(out_lines), changed


def process(path: Path):
    source = path.read_text()
    try:
        ast = Parser(Tokenizer(source).tokenize()).parse()
    except Exception as exc:
        return None, f"parse error: {exc}"
    try:
        names = violations_for(ast)
    except Exception as exc:
        return None, f"check error: {exc}"
    if not names:
        return 0, None
    new_source, changed = rewrite(source, names)
    if changed:
        path.write_text(new_source)
    return changed, None


def main(argv):
    targets = argv[1:] or ['examples', 'tests']
    files = []
    for target in targets:
        p = Path(target)
        if p.is_file():
            files.append(p)
        else:
            files.extend(sorted(p.rglob('*.aura')))
    total_files = 0
    total_changes = 0
    for f in files:
        changed, err = process(f)
        if err:
            print(f"SKIP {f}: {err}")
            continue
        if changed:
            total_files += 1
            total_changes += changed
    print(f"Migrated {total_files} files ({total_changes} declarations)")


if __name__ == '__main__':
    main(sys.argv)
