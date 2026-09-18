"""Import hook that lets Aura programs import other Aura modules.

When an Aura program runs, ``import mymodule`` normally resolves to a Python
module. This hook makes ``import mymodule`` / ``from mymodule import x`` also
work when ``mymodule.aura`` exists next to the running script (or in a
subdirectory for dotted imports such as ``import pkg.util``).

The imported Aura file is transpiled with the regular ``Transformer`` and then
executed in its own module namespace, so it behaves like a Python module.

Every imported file is checked with the same rules as the entry file, except
the entry-point rule: a library needs no ``main`` (and in fact must not declare
one, since a module's ``main`` would never run — see ``E312``).
"""

import sys
from pathlib import Path


def _load_aura_source(path):
    """Parse, check and transpile an Aura file; return the Python source.

    Raises ``SyntaxError`` with the offending file and messages when the file
    violates Aura's rules, so an invalid import fails loudly at the import
    site instead of misbehaving later.
    """
    from aura.parser.to_ast import parse_file
    from aura.transpiler.rules import RuleChecker
    from aura.transpiler.semantics import MutabilityChecker
    from aura.transpiler.transformer import Transformer

    ast = parse_file(str(path))

    checker = MutabilityChecker()
    if not checker.check_program(ast):
        message = "; ".join(str(e) for e in checker.errors)
        raise SyntaxError(f"{path}: semantic error: {message}")

    rules = RuleChecker()
    if not rules.check_program(ast, require_main=False):
        message = "; ".join(str(e) for e in rules.collector.errors)
        raise SyntaxError(f"{path}: rule error: {message}")

    return Transformer().transform(ast)

class AuraFinder:
    """Meta path finder for ``.aura`` modules."""

    def __init__(self, roots):
        # Resolve to absolute paths up front so later `chdir` calls do not
        # change resolution.
        self.roots = [Path(r).resolve() for r in roots if r]

    def _candidates(self, fullname):
        # A malformed name (empty, or with a component that has no stem) can
        # come from a direct `find_spec` call; refuse it rather than letting
        # `with_suffix` raise.
        parts = [part for part in str(fullname).split('.') if part]
        if not parts or any(part in ('.', '..') for part in parts):
            return
        rel = Path(*parts)
        for root in self.roots:
            yield root / rel.with_suffix('.aura')
            # Package form: `pkg/__init__.aura`
            yield root / rel / '__init__.aura'

    def _is_within(self, candidate: Path) -> bool:
        """True when ``candidate`` resolves inside one of the search roots.

        Module names come from the parser's IDENT-only grammar, so this is
        normally trivially true. It is enforced anyway so a direct, crafted
        ``find_spec`` call cannot resolve a path outside the project (for
        example through a symlink or a ``..`` component).
        """
        try:
            resolved = candidate.resolve()
        except OSError:
            return False
        for root in self.roots:
            try:
                if resolved == root or root in resolved.parents:
                    return True
            except OSError:
                continue
        return False

    def find_spec(self, fullname, path=None, target=None):
        # Reject a malformed or traversal-shaped name up front: only dot-
        # separated identifiers can name an Aura module.
        parts = [part for part in str(fullname).split('.') if part]
        if not parts or any(part in ('.', '..') for part in parts):
            return None
        # A dotted import first resolves its parent package. Treat any
        # directory that contains Aura sources as a package so that
        # `import pkg.util` can find `pkg/` and then `pkg/util.aura`.
        for root in self.roots:
            package_dir = root / Path(*parts)
            if package_dir.is_dir() and self._is_within(package_dir) and (
                (package_dir / '__init__.aura').is_file()
                or any(package_dir.glob('*.aura'))
            ):
                return self._package_spec(fullname, package_dir)

        for candidate in self._candidates(fullname):
            if candidate.is_file() and self._is_within(candidate):
                return self._spec_for(fullname, candidate)
        return None

    def _package_spec(self, fullname, package_dir):
        import importlib.util

        loader = AuraPackageLoader(fullname, package_dir)
        return importlib.util.spec_from_loader(
            fullname, loader, origin=str(package_dir),
            is_package=True,
        )

    def _spec_for(self, fullname, file_path):
        import importlib.util

        loader = AuraLoader(fullname, file_path)
        return importlib.util.spec_from_loader(fullname, loader, origin=str(file_path))


class AuraLoader:
    """Load an Aura source file as a Python module."""

    def __init__(self, fullname, file_path):
        self.fullname = fullname
        self.file_path = Path(file_path)

    def create_module(self, spec):
        return None  # use default module creation

    def exec_module(self, module):
        code = _load_aura_source(self.file_path)
        module.__file__ = str(self.file_path)
        compiled = compile(code, str(self.file_path), 'exec')
        exec(compiled, module.__dict__)


class AuraPackageLoader:
    """Load a directory containing Aura sources as a package.

    If the package has an ``__init__.aura`` it is executed; otherwise the
    package is created empty so that dotted submodules (``pkg.util``) can be
    imported normally.
    """

    def __init__(self, fullname, package_dir):
        self.fullname = fullname
        self.package_dir = Path(package_dir)
        self.is_package = True

    def create_module(self, spec):
        # Give the package a search location so submodules resolve through the
        # regular import machinery (and therefore through this finder).
        if spec is not None:
            spec.submodule_search_locations = [str(self.package_dir)]
        return None

    def exec_module(self, module):
        # A package's entry point is `__init__.aura`, or a file named after
        # the package itself (`App/App.aura`), which is how a facade module
        # lives next to its own source files.
        init = self.package_dir / '__init__.aura'
        if not init.is_file():
            entry = self.package_dir / (self.package_dir.name + '.aura')
            if not entry.is_file():
                return
            init = entry
        code = _load_aura_source(init)
        module.__file__ = str(init)
        exec(compile(code, str(init), 'exec'), module.__dict__)


def install_aura_import_hook(search_roots):
    """Install the Aura import finder for the given search roots.

    Safe to call multiple times: only one finder per root set is installed.
    """
    resolved = [str(Path(r).resolve()) for r in search_roots if r]
    for finder in sys.meta_path:
        if isinstance(finder, AuraFinder) and [str(r) for r in finder.roots] == resolved:
            return finder
    finder = AuraFinder(search_roots)
    sys.meta_path.insert(0, finder)
    return finder
