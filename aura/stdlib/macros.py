"""Aura standard library — compile-time macros (AuraMacroFactory surface).

Aura has two kinds of macro:

* **Runtime decorators** (``@debug``, ``@memoize``, ...) — ordinary Python
  decorators injected as a prelude; they wrap a function and cost a call.
* **Compile-time macros** — expanded by the transpiler *before* code is
  emitted. They receive their operands as quoted AST, return replacement AST,
  and leave no trace at runtime unless they choose to emit it.

This module is the Aura-facing surface of the compile-time engine in
:mod:`aura.transpiler.macro_factory`. Importing it exposes the built-in macro
names and provides the registration API used by tools that extend Aura::

    import macros

    // Built-ins available at compile time:
    assert_eq(a, b)      // evaluate both once, assert equality with both shown
    assert_ne(a, b)      // evaluate both once, assert inequality
    identity(x)          // expands to x (useful in metaprogramming tests)
    discard(x)           // evaluate x and bind it to a hygienic ignored name
    debug_value(x)       // print "x = <value>" once, then yield the value
    stringify(x)         // str(literal) at compile time, str(x) otherwise
    swap(a, b)           // exchange two lvalues via a hygienic temporary
    static_assert(c)     // compile-time check; a non-literal condition is an error
    todo()               // raise "not implemented" at this point
    todo("message")      // ... with a custom message
    unreachable("why")   // a documented todo that always raises

Extending Aura with your own macro is a build-time (Python) action, because a
macro runs while the program is being compiled::

    from aura.transpiler.macro_factory import Quote, gensym

    def double(args, kwargs):
        return Quote.binary("*", args[0], Quote.int_literal(2))

    registry = ...  # the ExpressionTransformer's macro_registry
    registry.register("double", double, min_args=1, max_args=1)

Then ``double(x)`` in Aura expands to ``x * 2`` with no runtime overhead.

A macro receives its operands as raw AST, so it can *inspect* them as well as
paste them: :func:`literal_value`, :func:`name_of`, :func:`is_identifier_named`
and :func:`contains_identifier` drive compile-time decisions. :class:`Quote`
builds expressions, statements and whole functions, so an expansion may be more
than one expression.

Hygiene: any identifier a macro introduces must be built with
:func:`aura.transpiler.macro_factory.gensym`, so it can never capture (or be
captured by) a name at the call site.
"""

from aura.transpiler.macro_factory import (  # noqa: F401
    Macro,
    MacroError,
    MacroRegistry,
    Quote,
    contains_identifier,
    default_registry,
    gensym,
    is_identifier_named,
    literal_value,
    name_of,
)

__all__ = [
    'Macro',
    'MacroError',
    'MacroRegistry',
    'Quote',
    'contains_identifier',
    'default_registry',
    'gensym',
    'is_identifier_named',
    'literal_value',
    'name_of',
    'BUILTIN_COMPILE_TIME_MACROS',
    'builtin_names',
]

#: Names of the macros every Aura program can call without registering one.
BUILTIN_COMPILE_TIME_MACROS = tuple(default_registry().names())


def builtin_names():
    """Return the names of the built-in compile-time macros."""
    return BUILTIN_COMPILE_TIME_MACROS


def is_macro(name):
    """Return True when ``name`` is a built-in compile-time macro."""
    return name in BUILTIN_COMPILE_TIME_MACROS
