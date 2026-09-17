"""Extra branch coverage for runtime aliases, crypto backend selection and
REPL edge paths.

These complement ``test_repl_deep.py`` and ``test_crypto.py`` by exercising
branches that only trigger on unusual inputs (bad specs, empty paths, backend
selection, non-identifier targets).
"""
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).parent.parent
sys.path.insert(0, str(ROOT))


# ============================================================================
# aura.runtime - alias installation and removal
# ============================================================================

class TestRuntimeAliases:
    def test_install_and_uninstall_roundtrip(self):
        from aura import runtime
        runtime.install_runtime_aliases()
        assert 'stdlib' in sys.modules
        assert sys.modules['stdlib'].__name__ == 'aura.stdlib'
        runtime.uninstall_runtime_aliases()
        assert 'stdlib' not in sys.modules

    def test_install_is_idempotent(self):
        from aura import runtime
        runtime.install_runtime_aliases()
        first = sys.modules['stdlib']
        runtime.install_runtime_aliases()
        assert sys.modules['stdlib'] is first

    def test_register_submodules_skips_non_package(self):
        import types

        from aura import runtime
        module = types.ModuleType('aura.noroot')
        # No __path__ -> returns immediately without touching sys.modules.
        runtime._register_submodules('noroot', module)
        assert 'noroot' not in sys.modules

    def test_register_submodules_skips_existing_and_import_errors(self):
        import importlib

        from aura import runtime
        module = importlib.import_module('aura.stdlib')
        runtime._register_submodules('stdlib', module)
        assert 'stdlib.math' in sys.modules

    def test_install_skips_existing_top_level_module(self, monkeypatch):
        from aura import runtime
        monkeypatch.setitem(sys.modules, 'stdlib', type(sys)('stdlib_fake'))
        monkeypatch.setattr(runtime, '_installed', False)
        runtime.install_runtime_aliases()
        assert sys.modules['stdlib'].__name__ == 'stdlib_fake'
        runtime.uninstall_runtime_aliases()
        assert sys.modules['stdlib'].__name__ == 'stdlib_fake'

    def test_python_submodule_alias(self):
        from aura import runtime
        runtime.uninstall_runtime_aliases()
        sys.modules.pop('python', None)
        runtime.install_runtime_aliases()
        assert 'python' in sys.modules
        assert sys.modules['python'].__name__ == 'aura.stdlib.python'
        runtime.uninstall_runtime_aliases()


# ============================================================================
# aura.stdlib.crypto_backend - reference backend and helpers
# ============================================================================

class TestCryptoBackend:
    def test_info_shape(self):
        from aura.stdlib import crypto_backend as cb
        info = cb.info()
        assert info['name'] in ('cryptography', 'reference')
        assert isinstance(info['production'], bool)
        assert 'ML-KEM-512' in info['algorithms']

    def test_reference_backend_kem_roundtrip(self):
        from aura.stdlib.crypto_backend import _ReferenceBackend
        backend = _ReferenceBackend()
        keys = backend.kem_keypair('ML-KEM-768')
        enc = backend.kem_encapsulate(keys['public_key'], 'ML-KEM-768')
        shared = backend.kem_decapsulate(
            keys['secret_key'], enc['ciphertext'], 'ML-KEM-768')
        assert shared == enc['shared_secret']

    def test_reference_backend_dsa_roundtrip(self):
        from aura.stdlib.crypto_backend import _ReferenceBackend
        backend = _ReferenceBackend()
        keys = backend.dsa_keypair('ML-DSA-44')
        signature = backend.dsa_sign(keys['secret_key'], 'msg', 'ML-DSA-44')
        assert backend.dsa_verify(
            keys['public_key'], 'msg', signature, 'ML-DSA-44') is True
        assert backend.dsa_verify(
            keys['public_key'], 'other', signature, 'ML-DSA-44') is False

    def test_reference_backend_rejects_unknown_algorithms(self):
        from aura.stdlib.crypto_backend import _ReferenceBackend
        backend = _ReferenceBackend()
        with pytest.raises(ValueError):
            backend.kem_keypair('NOPE')
        with pytest.raises(ValueError):
            backend.dsa_keypair('NOPE')

    def test_module_level_kem_functions(self):
        from aura.stdlib import crypto_backend as cb
        keys = cb.kem_keypair('ML-KEM-1024')
        enc = cb.kem_encapsulate(keys['public_key'], 'ML-KEM-1024')
        shared = cb.kem_decapsulate(
            keys['secret_key'], enc['ciphertext'], 'ML-KEM-1024')
        assert shared == enc['shared_secret']

    def test_module_level_dsa_functions(self):
        from aura.stdlib import crypto_backend as cb
        keys = cb.dsa_keypair('ML-DSA-65')
        sig = cb.dsa_sign(keys['secret_key'], b'data', 'ML-DSA-65')
        assert cb.dsa_verify(keys['public_key'], b'data', sig, 'ML-DSA-65') is True

    def test_to_bytes_conversions(self):
        from aura.stdlib.crypto_backend import _to_bytes
        assert _to_bytes(b'x') == b'x'
        assert _to_bytes(bytearray(b'xy')) == b'xy'
        assert _to_bytes('hi') == b'hi'
        with pytest.raises(TypeError):
            _to_bytes(123)

    def test_select_is_cached(self):
        from aura.stdlib import crypto_backend as cb
        cb._BACKEND = None
        first = cb._select()
        second = cb._select()
        assert first is second


# ============================================================================
# REPL edge branches
# ============================================================================

class TestReplEdgeBranches:
    def _repl(self):
        from repl.engine import AuraREPL
        outputs = []
        return AuraREPL(output_func=outputs.append), outputs

    def test_load_with_rule_violation(self, tmp_path):
        repl, out = self._repl()
        src = tmp_path / 'rule.aura'
        src.write_text('def f() {\n  return 1\n  print(2)\n}\n')
        result = repl.handle_command(f':load {src}')
        assert not result.ok

    def test_load_with_syntax_error(self, tmp_path):
        repl, out = self._repl()
        src = tmp_path / 'bad.aura'
        src.write_text('let = 5\n')
        result = repl.handle_command(f':load {src}')
        assert not result.ok

    def test_run_with_missing_file(self):
        repl, out = self._repl()
        result = repl.handle_command(':run /nonexistent/prog.aura')
        assert not result.ok

    def test_ast_command_syntax_error(self):
        repl, out = self._repl()
        result = repl.handle_command(':ast let = 5')
        assert not result.ok

    def test_python_command_non_expression_statement(self):
        repl, out = self._repl()
        result = repl.handle_command(':py x = 5')
        assert result.ok
        assert repl.locals['x'] == 5

    def test_first_error_with_code_and_message(self):
        from repl.engine import AuraREPL

        class _Code:
            value = 'E999'

        class _Err:
            code = _Code()
            message = 'boom'

        assert AuraREPL._first_error([_Err()]) == '[E999] boom'

    def test_first_error_without_code_uses_str(self):
        from repl.engine import AuraREPL
        assert AuraREPL._first_error(['plain error']) == 'plain error'

    def test_first_error_empty(self):
        from repl.engine import AuraREPL
        assert AuraREPL._first_error([]) == 'rule violation'

    def test_format_vars_empty(self):
        from repl.engine import AuraREPL
        repl = AuraREPL(output_func=lambda *_: None)
        repl.namespace.clear()
        assert repl._format_vars() == '(no bindings)'

    def test_assigned_names_various_forms(self):
        from repl.engine import AuraREPL

        from aura.transpiler import ast
        repl = AuraREPL(output_func=lambda *_: None)

        tuple_decl = ast.VarDecl('(a, b)', True, value=ast.Identifier('src'))
        assert repl._assigned_names(tuple_decl) == ['a', 'b']

        simple = ast.VarDecl('x', True, value=ast.Identifier('src'))
        assert repl._assigned_names(simple) == ['x']

        const = ast.ConstDecl('k', ast.Identifier('src'))
        assert repl._assigned_names(const) == ['k']

        assign_id = ast.ExprStmt(ast.BinaryOp(
            ast.Identifier('y'), '=', ast.IntLiteral(1)))
        assert repl._assigned_names(assign_id) == ['y']

        assign_tuple = ast.ExprStmt(ast.BinaryOp(
            ast.TupleLiteral([ast.Identifier('p'), ast.Identifier('q')]), '=',
            ast.TupleLiteral([ast.IntLiteral(1), ast.IntLiteral(2)])))
        assert repl._assigned_names(assign_tuple) == ['p', 'q']

    def test_history_and_reset(self):
        repl, out = self._repl()
        repl.process('let x = 1')
        repl.handle_command(':history')
        assert any('x' in line for line in out)
        repl.handle_command(':reset')
        assert 'x' not in repl.locals

    def test_vars_after_binding(self):
        repl, out = self._repl()
        repl.process('let y = 5')
        repl.handle_command(':vars')
        assert any('y = ' in line for line in out)

    def test_type_command_value(self):
        repl, out = self._repl()
        result = repl.handle_command(':type 1 + 1')
        assert result.ok
        assert any('2' in line for line in out)
