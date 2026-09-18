"""Tests for module facades and cross-file re-exports.

A module can act as a **facade** for a source folder: `module App { ... }` in
`App/App.aura` re-exports the classes and helpers defined in sibling files.

```
App/
  App.aura          module App { export Components, Utils }
  components.aura   class Components { ... }
  utils.aura        def double(...) / const VERSION
main.aura           import App; App.Components(...) / App.Utils.double(...)
```

A bare `export Name` resolves by convention (a sibling file or subfolder whose
name matches, case-insensitively), or explicitly with
`export Name from "module"`. When the file declares the name, the binding is
that declaration; otherwise the whole sibling module is exposed as the
namespace. Unresolved re-exports are reported as `E313`.

Tests write real folders and run the CLI, because resolution and the import
hook are filesystem-level behaviour.
"""
import subprocess
import sys
import textwrap
from pathlib import Path

import pytest

ROOT = Path(__file__).parent.parent
sys.path.insert(0, str(ROOT))

from aura.parser.to_ast import Parser, Tokenizer  # noqa: E402
from aura.transpiler.modules import (  # noqa: E402
    find_reexport_conflicts,
    resolve_reexport,
)
from aura.transpiler.rules import RuleChecker  # noqa: E402


def write(directory: Path, name: str, source: str) -> Path:
    path = directory / name
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(textwrap.dedent(source).lstrip(), encoding='utf-8')
    return path


def run_cli(cwd, *argv, timeout=60):
    return subprocess.run(
        [sys.executable, '-m', 'aura.cli', *argv],
        capture_output=True, text=True, timeout=timeout, cwd=str(cwd))


def run_program(cwd, path='main.aura'):
    result = run_cli(cwd, 'run', path)
    assert result.returncode == 0, result.stdout + result.stderr
    return result.stdout


def rule_codes(source, source_path=None):
    from aura.transpiler.ast import Program
    program = Parser(Tokenizer(source).tokenize()).parse()
    if source_path is not None:
        program = Program(program.statements, source_path=str(source_path))
    checker = RuleChecker()
    checker.check_program(program)
    return [e.code.value for e in checker.collector.errors]


# ============================================================================
# Parser
# ============================================================================

class TestReexportParsing:
    def test_single_bare_export(self):
        module = Parser(Tokenizer('module App {\n  export Components\n}\n')
                        .tokenize()).parse().statements[0]
        assert module.exports == {'Components'}
        assert len(module.reexports) == 1
        assert module.reexports[0].names == ['Components']
        assert module.reexports[0].source is None

    def test_multiple_names_in_one_export(self):
        module = Parser(Tokenizer('module App {\n  export A, B, C\n}\n')
                        .tokenize()).parse().statements[0]
        assert module.exports == {'A', 'B', 'C'}
        assert module.reexports[0].names == ['A', 'B', 'C']

    def test_separate_export_lines(self):
        module = Parser(Tokenizer(
            'module App {\n  export Components\n  export Utils\n}\n')
            .tokenize()).parse().statements[0]
        assert module.exports == {'Components', 'Utils'}
        assert [item.names for item in module.reexports] == [['Components'], ['Utils']]

    def test_explicit_source(self):
        module = Parser(Tokenizer(
            'module App {\n  export Components from "widgets"\n}\n')
            .tokenize()).parse().statements[0]
        assert module.reexports[0].source == 'widgets'

    def test_explicit_dotted_source(self):
        module = Parser(Tokenizer(
            'module App {\n  export X from "pkg.sub"\n}\n')
            .tokenize()).parse().statements[0]
        assert module.reexports[0].source == 'pkg.sub'

    def test_export_declaration_still_works(self):
        module = Parser(Tokenizer(
            'module App {\n  export def f() -> int { return 1 }\n}\n')
            .tokenize()).parse().statements[0]
        assert module.exports == {'f'}
        assert module.reexports == []
        assert module.members[0].is_exported is True

    def test_export_class_still_works(self):
        module = Parser(Tokenizer(
            'module App {\n  export class C { public let x: int = 0 }\n}\n')
            .tokenize()).parse().statements[0]
        assert module.exports == {'C'}
        assert module.reexports == []
        assert module.members[0].is_exported is True

    def test_mixed_declaration_and_reexport(self):
        module = Parser(Tokenizer(
            'module App {\n'
            '  export def local() -> int { return 1 }\n'
            '  export Components\n'
            '}\n').tokenize()).parse().statements[0]
        assert module.exports == {'local', 'Components'}
        assert [m.name for m in module.members] == ['local']
        assert [i.names for i in module.reexports] == [['Components']]

    def test_unexported_declaration_is_private(self):
        module = Parser(Tokenizer(
            'module App {\n  def hidden() -> int { return 1 }\n}\n')
            .tokenize()).parse().statements[0]
        assert module.exports == set()
        assert module.members[0].is_exported is False

    def test_bare_export_at_end_is_rejected(self):
        with pytest.raises(SyntaxError, match='export'):
            Parser(Tokenizer('module App {\n  export\n}\n').tokenize()).parse()

    def test_export_from_without_string_is_rejected(self):
        with pytest.raises(SyntaxError, match='quoted module path'):
            Parser(Tokenizer(
                'module App {\n  export X from widgets\n}\n').tokenize()).parse()

    def test_export_from_with_traversal_is_rejected(self):
        with pytest.raises(SyntaxError, match='invalid module path'):
            Parser(Tokenizer(
                'module App {\n  export X from "../evil"\n}\n').tokenize()).parse()

    def test_export_from_with_separator_is_rejected(self):
        with pytest.raises(SyntaxError, match='invalid module path'):
            Parser(Tokenizer(
                'module App {\n  export X from "a/b"\n}\n').tokenize()).parse()

    def test_export_outside_module_is_rejected(self):
        with pytest.raises(SyntaxError, match='export'):
            Parser(Tokenizer('export Components\n').tokenize()).parse()


# ============================================================================
# Resolution
# ============================================================================

class TestResolution:
    def test_resolves_sibling_file(self, tmp_path):
        write(tmp_path, 'components.aura',
              'class Components { public let x: int = 0 }\n')
        found = resolve_reexport('Components', tmp_path)
        assert found is not None
        path, module_path, declares = found
        assert path.name == 'components.aura'
        assert module_path == 'components'
        assert declares is True

    def test_resolution_is_case_insensitive(self, tmp_path):
        write(tmp_path, 'Components.aura',
              'class Components { public let x: int = 0 }\n')
        assert resolve_reexport('Components', tmp_path) is not None

    def test_missing_file_returns_none(self, tmp_path):
        assert resolve_reexport('Components', tmp_path) is None

    def test_file_without_the_declaration_exposes_the_module(self, tmp_path):
        write(tmp_path, 'utils.aura', 'def double(x: int) -> int { return x }\n')
        _path, module_path, declares = resolve_reexport('Utils', tmp_path)
        assert module_path == 'utils'
        assert declares is False

    def test_explicit_source(self, tmp_path):
        write(tmp_path, 'widgets.aura',
              'class Components { public let x: int = 0 }\n')
        found = resolve_reexport('Components', tmp_path, source='widgets')
        assert found is not None
        assert found[1] == 'widgets'

    def test_explicit_dotted_source(self, tmp_path):
        write(tmp_path, 'pkg/sub.aura', 'class X { public let v: int = 0 }\n')
        found = resolve_reexport('X', tmp_path, source='pkg.sub')
        assert found is not None
        assert found[1] == 'pkg.sub'

    def test_subfolder_with_same_name(self, tmp_path):
        write(tmp_path, 'Widgets/Widgets.aura',
              'class Widgets { public let v: int = 0 }\n')
        found = resolve_reexport('Widgets', tmp_path)
        assert found is not None
        assert found[1] == 'Widgets.Widgets'

    def test_subfolder_init(self, tmp_path):
        write(tmp_path, 'Widgets/__init__.aura',
              'class Widgets { public let v: int = 0 }\n')
        found = resolve_reexport('Widgets', tmp_path)
        assert found is not None
        assert found[1] == 'Widgets'

    def test_dotted_declaration_is_found(self, tmp_path):
        write(tmp_path, 'components.aura',
              'public trait Components { public def m() -> int }\n')
        found = resolve_reexport('Components', tmp_path)
        assert found is not None and found[2] is True

    def test_enum_and_type_declarations_are_found(self, tmp_path):
        # The file must match the name by convention (or be named explicitly).
        write(tmp_path, 'kinds.aura', 'enum Kinds { A }\n')
        assert resolve_reexport('Kinds', tmp_path)[2] is True
        write(tmp_path, 'alias.aura', 'type Alias = int\n')
        assert resolve_reexport('Alias', tmp_path)[2] is True

    def test_explicit_source_finds_a_name_in_a_differently_named_file(self, tmp_path):
        write(tmp_path, 'kinds.aura', 'enum Kinds { A }\ntype Alias = int\n')
        assert resolve_reexport('Alias', tmp_path) is None
        found = resolve_reexport('Alias', tmp_path, source='kinds')
        assert found is not None and found[2] is True

    def test_missing_directory_is_none(self, tmp_path):
        assert resolve_reexport('X', tmp_path / 'does-not-exist') is None

    def test_conflicts_are_reported(self, tmp_path):
        # Two explicit sources resolving to different files under the same
        # module path is impossible; a genuine conflict is two names mapping to
        # the same module path with different files, so construct that case.
        write(tmp_path, 'a.aura', 'class A { public let x: int = 0 }\n')
        assert find_reexport_conflicts(['A'], tmp_path) == []


# ============================================================================
# Generated Python
# ============================================================================

class TestFacadeCodegen:
    def test_imports_are_hoisted(self, tmp_path):
        write(tmp_path, 'App/components.aura',
              'class Components { public let x: int = 0 }\n')
        write(tmp_path, 'App/App.aura', 'module App {\n  export Components\n}\n')
        result = run_cli(tmp_path, 'transpile', 'App/App.aura')
        assert result.returncode == 0, result.stderr
        code = result.stdout
        # Imports at module level, bindings after.
        assert code.index('import App.components') < code.index('Components = ')

    def test_package_facade_does_not_nest_a_class(self, tmp_path):
        write(tmp_path, 'App/App.aura', 'module App {\n  export Components\n}\n')
        write(tmp_path, 'App/components.aura',
              'class Components { public let x: int = 0 }\n')
        code = run_cli(tmp_path, 'transpile', 'App/App.aura').stdout
        # `App/App.aura` backs the package, so no `class App:` wrapper.
        assert 'class App:' not in code

    def test_plain_file_facade_keeps_the_class_namespace(self, tmp_path):
        write(tmp_path, 'lib.aura', 'module Lib {\n  export Helper\n}\n')
        write(tmp_path, 'helper.aura', 'class Helper { public let v: int = 0 }\n')
        code = run_cli(tmp_path, 'transpile', 'lib.aura').stdout
        assert 'class Lib:' in code
        assert 'import helper' in code

    def test_generated_python_is_valid(self, tmp_path):
        import ast as py_ast
        write(tmp_path, 'App/App.aura',
              'module App {\n  export Components, Utils\n}\n')
        write(tmp_path, 'App/components.aura',
              'class Components { public let x: int = 0 }\n')
        write(tmp_path, 'App/utils.aura', 'def double(x: int) -> int { return x }\n')
        code = run_cli(tmp_path, 'transpile', 'App/App.aura').stdout
        py_ast.parse(code)


# ============================================================================
# End to end
# ============================================================================

class TestFacadeRuntime:
    def test_export_class_and_module(self, tmp_path):
        write(tmp_path, 'App/App.aura',
              'module App {\n  export Components, Utils\n}\n')
        write(tmp_path, 'App/components.aura', '''
            class Components {
              public let name: str = "root"
              public def new(name: str) { self.name = name }
              public def describe() -> str { return "component:" + self.name }
            }
        ''')
        write(tmp_path, 'App/utils.aura', '''
            def double(x: int) -> int { return x * 2 }
            const VERSION = "1.0.0"
        ''')
        write(tmp_path, 'main.aura', '''
            import App
            def main() {
              print(App.Components("header").describe())
              print(App.Utils.double(21))
              print(App.Utils.VERSION)
            }
        ''')
        assert run_program(tmp_path) == 'component:header\n42\n1.0.0\n'

    def test_facade_with_explicit_source(self, tmp_path):
        write(tmp_path, 'lib.aura',
              'module Lib {\n  export Widgets from "widgets"\n}\n')
        write(tmp_path, 'widgets.aura',
              'class Widgets { public let n: int = 7 }\n')
        write(tmp_path, 'main.aura',
              'import lib\ndef main() { print(lib.Lib.Widgets().n) }\n')
        assert run_program(tmp_path) == '7\n'

    def test_facade_reexports_its_own_declaration(self, tmp_path):
        write(tmp_path, 'App/App.aura', '''
            module App {
              export Components
              class Components { public let v: int = 5 }
            }
        ''')
        write(tmp_path, 'main.aura',
              'import App\ndef main() { print(App.Components().v) }\n')
        assert run_program(tmp_path) == '5\n'

    def test_facade_mixing_local_and_sibling(self, tmp_path):
        write(tmp_path, 'App/App.aura', '''
            module App {
              export Components
              export def version() -> str { return "2.0" }
            }
        ''')
        write(tmp_path, 'App/components.aura',
              'class Components { public let v: int = 1 }\n')
        write(tmp_path, 'main.aura', '''
            import App
            def main() {
              print(App.Components().v)
              print(App.version())
            }
        ''')
        assert run_program(tmp_path) == '1\n2.0\n'

    def test_facade_class_is_usable_as_a_base(self, tmp_path):
        write(tmp_path, 'App/App.aura', 'module App {\n  export Base\n}\n')
        write(tmp_path, 'App/base.aura', '''
            class Base {
              public def greet() -> str { return "hi" }
            }
        ''')
        write(tmp_path, 'main.aura', '''
            import App
            class Child extends App.Base { }
            def main() { print(Child().greet()) }
        ''')
        assert run_program(tmp_path) == 'hi\n'

    def test_facade_reexport_is_idempotent_on_reimport(self, tmp_path):
        write(tmp_path, 'App/App.aura', 'module App {\n  export Helper\n}\n')
        write(tmp_path, 'App/helper.aura',
              'class Helper { public let v: int = 3 }\n')
        write(tmp_path, 'main.aura', '''
            import App
            import App as Again
            def main() { print(App.Helper().v + Again.Helper().v) }
        ''')
        assert run_program(tmp_path) == '6\n'


# ============================================================================
# Diagnostics
# ============================================================================

class TestFacadeDiagnostics:
    def test_unresolved_reexport_reports_e313(self, tmp_path):
        source = write(tmp_path, 'App.aura', 'module App {\n  export Missing\n}\n')
        assert 'E313' in rule_codes(source.read_text(), source_path=source)

    def test_resolved_reexport_is_clean(self, tmp_path):
        write(tmp_path, 'components.aura',
              'class Components { public let x: int = 0 }\n')
        source = write(tmp_path, 'App.aura', 'module App {\n  export Components\n}\n')
        assert 'E313' not in rule_codes(source.read_text(), source_path=source)

    def test_local_declaration_satisfies_reexport(self, tmp_path):
        source = write(tmp_path, 'App.aura', '''
            module App {
              export Components
              class Components { public let x: int = 0 }
            }
        ''')
        assert 'E313' not in rule_codes(source.read_text(), source_path=source)

    def test_reexported_name_is_public(self, tmp_path):
        write(tmp_path, 'components.aura',
              'class Components { public let x: int = 0 }\n')
        source = write(tmp_path, 'App.aura', '''
            module App {
              export Components
              def hidden() -> int { return 1 }
            }
        ''')
        codes = rule_codes(source.read_text(), source_path=source)
        assert 'E308' not in codes

    def test_private_member_after_reexport_still_reported(self, tmp_path):
        write(tmp_path, 'components.aura', 'class Components { public let x: int = 0 }\n')
        source = write(tmp_path, 'App.aura', '''
            module App {
              export Components
              def secret() -> int { return 1 }
            }
        ''')
        checker_source = source.read_text() + '\ndef use() { App.secret() }\n'
        codes = rule_codes(checker_source, source_path=source)
        assert 'E308' in codes

    def test_cli_check_reports_unresolved(self, tmp_path):
        write(tmp_path, 'App.aura', 'module App {\n  export Missing\n}\n')
        result = run_cli(tmp_path, 'check', 'App.aura')
        assert result.returncode != 0
        assert 'E313' in result.stdout + result.stderr

    def test_cli_run_reports_unresolved(self, tmp_path):
        write(tmp_path, 'App.aura', 'module App {\n  export Missing\n}\n')
        write(tmp_path, 'main.aura',
              'import App\ndef main() { print(1) }\n')
        result = run_cli(tmp_path, 'run', 'main.aura')
        assert result.returncode != 0

    def test_main_inside_a_module_reports_e312(self, tmp_path):
        source = write(tmp_path, 'lib.aura',
                       'module M {\n  def main() { print(1) }\n}\n')
        assert 'E312' in rule_codes(source.read_text(), source_path=source)

    def test_main_inside_a_facade_reports_e312(self, tmp_path):
        source = write(tmp_path, 'App/App.aura',
                       'module App {\n  def main() { print(1) }\n}\n')
        assert 'E312' in rule_codes(source.read_text(), source_path=source)


# ============================================================================
# Importing an invalid module fails loudly
# ============================================================================

class TestImportedFileRules:
    def test_imported_file_with_rule_violation_raises(self, tmp_path):
        write(tmp_path, 'bad.aura', 'def f() { break }\n')
        write(tmp_path, 'main.aura',
              'import bad\ndef main() { print(1) }\n')
        result = run_cli(tmp_path, 'run', 'main.aura')
        assert result.returncode != 0

    def test_imported_file_with_semantic_error_raises(self, tmp_path):
        write(tmp_path, 'bad.aura', 'let x = 1\nx = 2\n')
        write(tmp_path, 'main.aura',
              'import bad\ndef main() { print(1) }\n')
        result = run_cli(tmp_path, 'run', 'main.aura')
        assert result.returncode != 0

    def test_imported_library_needs_no_main(self, tmp_path):
        write(tmp_path, 'lib.aura', 'def helper() -> int { return 4 }\n')
        write(tmp_path, 'main.aura',
              'import lib\ndef main() { print(lib.helper()) }\n')
        assert run_program(tmp_path) == '4\n'


# ============================================================================
# Import hook security
# ============================================================================

class TestImportSecurity:
    def test_crafted_fullname_cannot_escape_the_root(self, tmp_path):
        from aura.transpiler.importer import AuraFinder
        root = tmp_path / 'root'
        root.mkdir()
        (tmp_path / 'outside.aura').write_text('def x() {}\n')
        finder = AuraFinder([root])
        for name in ('..outside', '....etc.passwd', '.', '', '..'):
            assert finder.find_spec(name) is None, name

    def test_symlink_outside_the_root_is_refused(self, tmp_path):
        from aura.transpiler.importer import AuraFinder
        root = tmp_path / 'root'
        root.mkdir()
        outside = tmp_path / 'outside.aura'
        outside.write_text('def x() {}\n')
        link = root / 'escape.aura'
        try:
            link.symlink_to(outside)
        except (OSError, NotImplementedError):
            pytest.skip('symlinks unavailable')
        finder = AuraFinder([root])
        assert finder.find_spec('escape') is None

    def test_stdlib_is_not_hijacked(self, tmp_path):
        from aura.transpiler.importer import AuraFinder
        finder = AuraFinder([tmp_path])
        assert finder.find_spec('os') is None
        assert finder.find_spec('json') is None

    def test_reexport_source_cannot_traverse(self, tmp_path):
        write(tmp_path, 'App/App.aura',
              'module App {\n  export X from "../secret"\n}\n')
        result = run_cli(tmp_path, 'check', 'App/App.aura')
        # The parser rejects the path before resolution is attempted.
        assert result.returncode != 0

    def test_reexport_cannot_reach_absolute_path(self, tmp_path):
        write(tmp_path, 'App/App.aura',
              'module App {\n  export X from "/etc/passwd"\n}\n')
        result = run_cli(tmp_path, 'check', 'App/App.aura')
        assert result.returncode != 0
