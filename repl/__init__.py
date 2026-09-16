"""Compatibility shim: the implementation lives in ``aura.repl``."""
import sys as _sys
import aura.repl as _impl
_sys.modules[__name__] = _impl
