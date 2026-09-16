"""Compatibility shim: the implementation lives in ``aura.tools``."""
import sys as _sys
import importlib as _importlib
import aura.tools as _impl

_sys.modules[__name__] = _impl

for _sub in ('analyze_tests', 'debugger', 'deps', 'formatter',
             'generate_aura_scenarios', 'generate_diverse_tests',
             'generate_enhanced_tests', 'generate_massive_aura_v3',
             'generate_stress_scenarios', 'migrate_mut', 'release',
             'stochastic_aura_gen'):
    _sys.modules[f'{__name__}.{_sub}'] = _importlib.import_module(f'aura.tools.{_sub}')