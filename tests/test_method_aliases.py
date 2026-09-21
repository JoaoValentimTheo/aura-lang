"""Regression tests for issue #10: METHOD_ALIASES rewrites stdlib module calls.

When calling a function on an imported stdlib module (e.g.
``import stdlib.string as strings`` then ``strings.trim(...)``), the
transpiler must NOT rewrite the member name through METHOD_ALIASES.  The
alias is only valid for method calls on string/collection *instances* (e.g.
``let s = "  hi  "`` then ``s.trim()``).

All tests compare ``aura run`` output, not just transpiled text.
"""
import sys
from pathlib import Path

ROOT = Path(__file__).parent.parent
sys.path.insert(0, str(ROOT))

from aura_test_helpers import run_aura, transpile  # noqa: E402

# ---- alias import: import stdlib.string as s ----

def test_alias_import_trim():
    out = run_aura('import stdlib.string as s\ndef main() { print(s.trim("  hi  ")) }\n')
    assert out.strip() == "hi"


def test_alias_import_starts_with():
    out = run_aura('import stdlib.string as s\ndef main() { print(s.starts_with("hello", "he")) }\n')
    assert out.strip() == "True"


def test_alias_import_ends_with():
    out = run_aura('import stdlib.string as s\ndef main() { print(s.ends_with("hello", "lo")) }\n')
    assert out.strip() == "True"


def test_alias_import_upper():
    out = run_aura('import stdlib.string as s\ndef main() { print(s.upper("hi")) }\n')
    assert out.strip() == "HI"


def test_alias_import_lower():
    out = run_aura('import stdlib.string as s\ndef main() { print(s.lower("HI")) }\n')
    assert out.strip() == "hi"


def test_alias_import_is_alpha():
    out = run_aura('import stdlib.string as s\ndef main() { print(s.is_alpha("abc")) }\n')
    assert out.strip() == "True"


# ---- dotted import: import stdlib.string ----

def test_dotted_import_trim():
    out = run_aura('import stdlib.string\ndef main() { print(stdlib.string.trim("  hi  ")) }\n')
    assert out.strip() == "hi"


def test_dotted_import_starts_with():
    out = run_aura('import stdlib.string\ndef main() { print(stdlib.string.starts_with("hello", "he")) }\n')
    assert out.strip() == "True"


def test_dotted_import_upper():
    out = run_aura('import stdlib.string\ndef main() { print(stdlib.string.upper("hi")) }\n')
    assert out.strip() == "HI"


# ---- from import: from stdlib.string import trim ----

def test_from_import_trim():
    out = run_aura('from stdlib.string import trim\ndef main() { print(trim("  hi  ")) }\n')
    assert out.strip() == "hi"


# ---- braces import: import stdlib.string { trim } ----

def test_braces_import_trim():
    out = run_aura('import stdlib.string { trim }\ndef main() { print(trim("  hi  ")) }\n')
    assert out.strip() == "hi"


# ---- shadowing: parameter shadows import ----

def test_parameter_shadows_import():
    src = 'import stdlib.string as s\ndef f(s) { return s.trim() }\ndef main() { print(f("  hi  ")) }\n'
    out = run_aura(src)
    assert out.strip() == "hi"


def test_let_shadows_import():
    src = ('import stdlib.string as s\n'
           'def main() { let s = "  hi  " print(s.trim()) }\n')
    out = run_aura(src)
    assert out.strip() == "hi"


def test_lambda_shadows_import():
    src = ('import stdlib.string as s\n'
           'def main() { let f = (s) => s.trim() print(f("  hi  ")) }\n')
    out = run_aura(src)
    assert out.strip() == "hi"


def test_nested_function_shadows_import():
    src = ('import stdlib.string as s\n'
           'def main() {\n'
           '  def inner(s) { return s.trim() }\n'
           '  print(inner("  hi  "))\n'
           '}\n')
    out = run_aura(src)
    assert out.strip() == "hi"


# ---- instance method aliases still work ----

def test_instance_trim_still_aliased():
    out = run_aura('def main() { let s = "  hi  " print(s.trim()) }\n')
    assert out.strip() == "hi"


def test_instance_starts_with_still_aliased():
    out = run_aura('def main() { let s = "hello" print(s.starts_with("he")) }\n')
    assert out.strip() == "True"


def test_instance_to_upper_still_aliased():
    out = run_aura('def main() { let s = "hi" print(s.to_upper()) }\n')
    assert out.strip() == "HI"


# ---- transpile text checks (supplementary) ----

def test_transpile_alias_import_preserves_name():
    py = transpile('import stdlib.string as s\ndef main() { s.trim("x") }\n')
    assert "s.trim(" in py
    assert "s.strip(" not in py


def test_transpile_dotted_import_preserves_name():
    py = transpile('import stdlib.string\ndef main() { stdlib.string.trim("x") }\n')
    assert "stdlib.string.trim(" in py
    assert "stdlib.string.strip(" not in py


def test_transpile_shadow_restores_alias():
    py = transpile('import stdlib.string as s\ndef f(s) { return s.trim() }\n')
    assert "s.strip(" in py
