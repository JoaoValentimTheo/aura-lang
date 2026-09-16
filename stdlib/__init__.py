"""Compatibility shim: the implementation lives in ``aura.stdlib``."""
import sys as _sys
import importlib as _importlib
import aura.stdlib as _impl

_sys.modules[__name__] = _impl

for _sub in ('collections', 'itertools', 'math', 'string', 'json', 'time', 'io',
             'regex', 'os', 'http', 'python', 'threading', 'asyncio', 'crypto',
             'crypto_backend'):
    _sys.modules[f'{__name__}.{_sub}'] = _importlib.import_module(f'aura.stdlib.{_sub}')