"""Compatibility shim: the implementation lives in ``aura.tools``."""
import sys as _sys
import aura.tools as _impl
_sys.modules[__name__] = _impl
