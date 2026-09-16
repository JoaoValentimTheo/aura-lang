"""Coverage tests for stdlib and tooling modules that lacked dedicated tests.

Focused on the Python-facing API of modules not exercised elsewhere:
``stdlib.itertools``, the ``stdlib.python`` interop bridge, the Aura formatter,
the ``migrate_mut`` rewrite helper, and the runtime module aliases.
"""
import io
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).parent.parent))


# ============================================================================
# stdlib.itertools
# ============================================================================

class TestItertools:
    def test_range_iter_forms(self):
        from stdlib.itertools import range_iter
        assert list(range_iter(3)) == [0, 1, 2]
        assert list(range_iter(1, 4)) == [1, 2, 3]
        assert list(range_iter(0, 6, 2)) == [0, 2, 4]

    def test_cycle_is_lazy_infinite(self):
        from stdlib.itertools import cycle
        import itertools as py_it
        assert list(py_it.islice(cycle('ab'), 5)) == ['a', 'b', 'a', 'b', 'a']

    def test_repeat(self):
        from stdlib.itertools import repeat
        assert list(repeat('x', 3)) == ['x', 'x', 'x']

    def test_chain(self):
        from stdlib.itertools import chain
        assert list(chain([1, 2], [3])) == [1, 2, 3]

    def test_combinations_and_permutations(self):
        from stdlib.itertools import combinations, permutations
        assert combinations([1, 2, 3], 2) == [(1, 2), (1, 3), (2, 3)]
        assert len(permutations([1, 2, 3])) == 6
        assert permutations([1, 2, 3], 2) == [(1, 2), (1, 3), (2, 1),
                                              (2, 3), (3, 1), (3, 2)]

    def test_product(self):
        from stdlib.itertools import product
        assert product([1, 2], ['a']) == [(1, 'a'), (2, 'a')]

    def test_count_is_infinite(self):
        from stdlib.itertools import count
        import itertools as py_it
        assert list(py_it.islice(count(5, 2), 3)) == [5, 7, 9]

    def test_enumerate_iter(self):
        from stdlib.itertools import enumerate_iter
        assert enumerate_iter(['a', 'b']) == [(0, 'a'), (1, 'b')]

    def test_islice(self):
        from stdlib.itertools import islice
        assert islice([1, 2, 3, 4, 5], 2) == [1, 2]
        assert islice([1, 2, 3, 4, 5], 1, 3) == [2, 3]

    def test_takewhile_dropwhile(self):
        from stdlib.itertools import takewhile, dropwhile
        assert takewhile(lambda x: x < 3, [1, 2, 3, 1]) == [1, 2]
        assert dropwhile(lambda x: x < 3, [1, 2, 3, 1]) == [3, 1]

    def test_groupby(self):
        from stdlib.itertools import groupby
        result = groupby(['a', 'a', 'b'], key=lambda x: x)
        assert result == {'a': ['a', 'a'], 'b': ['b']}

    def test_filterfalse(self):
        from stdlib.itertools import filterfalse
        assert filterfalse(lambda x: x % 2 == 0, [1, 2, 3, 4]) == [1, 3]

    def test_starmap(self):
        from stdlib.itertools import starmap
        assert starmap(lambda a, b: a + b, [(1, 2), (3, 4)]) == [3, 7]

    def test_tee(self):
        from stdlib.itertools import tee
        a, b = tee([1, 2, 3])
        assert list(a) == [1, 2, 3]
        assert list(b) == [1, 2, 3]

    def test_zip_longest(self):
        from stdlib.itertools import zip_longest
        assert zip_longest([1, 2], ['a'], fillvalue='-') == [(1, 'a'), (2, '-')]

    def test_pairwise(self):
        from stdlib.itertools import pairwise
        assert pairwise([1, 2, 3]) == [(1, 2), (2, 3)]


# ============================================================================
# stdlib.python bridge
# ============================================================================

class TestPythonBridge:
    def test_import_module_and_load(self):
        from stdlib import python as bridge
        mod = bridge.import_module('math')
        assert mod.pi > 3.0
        proxy = bridge.load('os')
        assert bridge.is_module(proxy)

    def test_is_available(self):
        from stdlib import python as bridge
        assert bridge.is_available('math') is True
        assert bridge.is_available('nonexistent_module_xyz') is False

    def test_eval(self):
        from stdlib import python as bridge
        assert bridge.eval('2 ** 8') == 256
        with pytest.raises(TypeError):
            bridge.eval(123)

    def test_exec_code_returns_namespace(self):
        from stdlib import python as bridge
        ns = bridge.exec_code('x = 40 + 2')
        assert ns['x'] == 42
        with pytest.raises(TypeError):
            bridge.exec_code(None)

    def test_compile_source(self):
        from stdlib import python as bridge
        code = bridge.compile_source('1 + 1', mode='eval')
        assert eval(code) == 2

    def test_call(self):
        from stdlib import python as bridge
        assert bridge.call(lambda a, b: a * b, 6, 7) == 42
        with pytest.raises(TypeError):
            bridge.call(123)

    def test_getattr_setattr_hasattr(self):
        from stdlib import python as bridge

        class Obj:
            pass

        o = Obj()
        assert bridge.hasattr(o, 'x') is False
        bridge.setattr(o, 'x', 5)
        assert bridge.getattr(o, 'x') == 5
        assert bridge.hasattr(o, 'x') is True
        assert bridge.getattr(o, 'missing', 'default') == 'default'

    def test_type_name(self):
        from stdlib import python as bridge
        assert bridge.type_name(1).endswith('int')
        assert bridge.type_name('s').endswith('str')

    def test_predicates(self):
        from stdlib import python as bridge
        assert bridge.is_callable(len) is True
        assert bridge.is_class(int) is True
        assert bridge.is_instance(1, int) is True
        assert bridge.is_module(bridge) is True

    def test_to_aura_converts_nested_modules(self):
        from stdlib import python as bridge
        converted = bridge.to_aura({'m': __import__('math'), 'v': [1, 2]})
        assert bridge.is_module(converted['m'])
        assert converted['v'] == [1, 2]

    def test_module_proxy_attribute_access(self):
        from stdlib import python as bridge
        math = bridge.load('math')
        assert abs(math.sqrt(9) - 3.0) < 1e-9

    def test_interpreter_version(self):
        from stdlib import python as bridge
        version = bridge.interpreter_version()
        # Returns sys.version_info (a named tuple), not a string.
        assert version.major >= 3


# ============================================================================
# Aura formatter
# ============================================================================

class TestFormatter:
    def test_indents_block(self):
        from aura.tools.formatter import format_aura
        out = format_aura("def f() {\nreturn 1\n}")
        assert "    return 1" in out or "  return 1" in out

    def test_normalizes_operator_spacing(self):
        from aura.tools.formatter import format_aura
        out = format_aura("let x=1+2")
        assert "let x = 1 + 2" in out

    def test_preserves_string_contents(self):
        from aura.tools.formatter import format_aura
        out = format_aura('let s = "a  b  c"')
        assert '"a  b  c"' in out

    def test_collapses_repeated_spaces(self):
        from aura.tools.formatter import format_aura
        out = format_aura("let    x    =    1")
        assert "let x = 1" in out

    def test_preserves_comments(self):
        from aura.tools.formatter import format_aura
        out = format_aura("let x = 1  // keep me")
        assert "// keep me" in out

    def test_multi_char_operator_not_split(self):
        from aura.tools.formatter import format_aura
        out = format_aura("def f() -> int { return 1 }")
        assert "->" in out
        assert "- >" not in out

    def test_blank_lines_preserved(self):
        from aura.tools.formatter import format_aura
        out = format_aura("let a = 1\n\nlet b = 2")
        assert "let a = 1\n\nlet b = 2" in out


# ============================================================================
# migrate_mut rewrite helper
# ============================================================================

class TestMigrateMut:
    def test_rewrite_adds_mut_to_violating_declaration(self):
        from aura.tools.migrate_mut import rewrite
        new_source, changed = rewrite("let x = 1\nx = 2", {'x'})
        assert changed == 1
        assert "let mut x" in new_source

    def test_rewrite_leaves_non_violating(self):
        from aura.tools.migrate_mut import rewrite
        new_source, changed = rewrite("let y = 1", {'x'})
        assert changed == 0
        assert new_source == "let y = 1"

    def test_rewrite_no_names_is_noop(self):
        from aura.tools.migrate_mut import rewrite
        new_source, changed = rewrite("let x = 1", set())
        assert changed == 0
        assert new_source == "let x = 1"


# ============================================================================
# runtime aliases
# ============================================================================

class TestRuntimeAliases:
    def test_install_is_idempotent(self):
        from aura.runtime import install_runtime_aliases
        install_runtime_aliases()
        install_runtime_aliases()
        import sys as _sys
        assert 'stdlib' in _sys.modules

    def test_python_alias_available(self):
        from aura.runtime import install_runtime_aliases
        install_runtime_aliases()
        import importlib
        mod = importlib.import_module('python')
        assert hasattr(mod, 'eval')