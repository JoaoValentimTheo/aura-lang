"""Import hook that lets Aura programs import other Aura modules.

When an Aura program runs, ``import mymodule`` normally resolves to a Python
module. This hook makes ``import mymodule`` / ``from mymodule import x`` also
work when ``mymodule.aura`` exists next to the running script (or in a
subdirectory for dotted imports such as ``import pkg.util``).

The imported Aura file is transpiled with the regular ``Transformer`` and then
executed in its own module namespace, so it behaves like a Python module.
"""

import sys
from pathlib import Path


class AuraFinder:
    """Meta path finder for ``.aura`` modules."""

    def __init__(self, roots):
        # Resolve to absolute paths up front so later `chdir` calls do not
        # change resolution.
        self.roots = [Path(r).resolve() for r in roots if r]

    def _candidates(self, fullname):
        parts = fullname.split('.')
        rel = Path(*parts)
        for root in self.roots:
            yield root / rel.with_suffix('.aura')
            # Package form: `pkg/__init__.aura`
            yield root / rel / '__init__.aura'

    def find_spec(self, fullname, path=None, target=None):
        # A dotted import first resolves its parent package. Treat any
        # directory that contains Aura sources as a package so that
        # `import pkg.util` can find `pkg/` and then `pkg/util.aura`.
        for root in self.roots:
            package_dir = root / Path(*fullname.split('.'))
            if package_dir.is_dir() and (
                (package_dir / '__init__.aura').is_file()
                or any(package_dir.glob('*.aura'))
            ):
                return self._package_spec(fullname, package_dir)

        for candidate in self._candidates(fullname):
            if candidate.is_file():
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
        from aura.parser.to_ast import parse_file
        from aura.transpiler.semantics import MutabilityChecker
        from aura.transpiler.transformer import Transformer

        ast = parse_file(str(self.file_path))

        checker = MutabilityChecker()
        if not checker.check_program(ast):
            message = "; ".join(checker.errors)
            raise SyntaxError(
                f"{self.file_path}: semantic error: {message}"
            )

        code = Transformer().transform(ast)
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
        init = self.package_dir / '__init__.aura'
        if not init.is_file():
            return
        from aura.parser.to_ast import parse_file
        from aura.transpiler.semantics import MutabilityChecker
        from aura.transpiler.transformer import Transformer

        ast = parse_file(str(init))
        checker = MutabilityChecker()
        if not checker.check_program(ast):
            message = "; ".join(checker.errors)
            raise SyntaxError(f"{init}: semantic error: {message}")
        code = Transformer().transform(ast)
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
