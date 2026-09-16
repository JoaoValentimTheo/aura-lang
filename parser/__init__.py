"""Compatibility shim: the implementation lives in ``aura.parser``."""
import sys as _sys
import importlib as _importlib
import aura.parser as _impl

_sys.modules[__name__] = _impl

# Register submodules so `from parser.to_ast import ...` resolves to aura.
for _sub in ('to_ast',):
    _sys.modules[f'{__name__}.{_sub}'] = _importlib.import_module(f'aura.parser.{_sub}')
