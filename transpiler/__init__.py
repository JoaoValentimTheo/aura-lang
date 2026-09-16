"""Compatibility shim: the implementation lives in ``aura.transpiler``."""
import sys as _sys
import importlib as _importlib
import aura.transpiler as _impl

_sys.modules[__name__] = _impl

for _sub in ('ast', 'errors', 'macros', 'semantics', 'transformer',
             'importer', 'types'):
    _sys.modules[f'{__name__}.{_sub}'] = _importlib.import_module(f'aura.transpiler.{_sub}')
for _sub in ('transformers', 'transformers.expressions', 'transformers.statements'):
    _sys.modules[f'{__name__}.{_sub}'] = _importlib.import_module(f'aura.transpiler.{_sub}')
