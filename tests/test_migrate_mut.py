"""Coverage for the `mut` migration tool (``aura.tools.migrate_mut``)."""
from pathlib import Path

from aura.parser.to_ast import Parser, Tokenizer
from aura.tools import migrate_mut


def parse(source):
    return Parser(Tokenizer(source).tokenize()).parse()


# ---------------------------------------------------------------------------
# violations_for
# ---------------------------------------------------------------------------

def test_violations_for_detects_reassignment():
    ast = parse('def main() {\n  let x = 1\n  x = 2\n  print(x)\n}')
    assert migrate_mut.violations_for(ast) == {'x'}


def test_violations_for_clean_program_is_empty():
    ast = parse('def main() {\n  let x = 1\n  print(x)\n}')
    assert migrate_mut.violations_for(ast) == set()


# ---------------------------------------------------------------------------
# rewrite
# ---------------------------------------------------------------------------

def test_rewrite_inserts_mut_before_declaration():
    source = 'def main() {\n  let x = 1\n  x = 2\n}'
    new_source, changed = migrate_mut.rewrite(source, {'x'})
    assert changed == 1
    assert 'let mut x = 1' in new_source


def test_rewrite_no_names_is_identity():
    source = 'let x = 1\n'
    assert migrate_mut.rewrite(source, set()) == (source, 0)


def test_rewrite_ignores_unrelated_declarations():
    source = 'let a = 1\nlet b = 2\n'
    new_source, changed = migrate_mut.rewrite(source, {'zzz'})
    assert changed == 0
    assert new_source == source


def test_rewrite_skips_already_mut():
    source = 'let mut x = 1\n'
    new_source, changed = migrate_mut.rewrite(source, {'x'})
    assert changed == 0
    assert 'mut mut' not in new_source


def test_rewrite_handles_visibilities():
    source = 'public let x = 1\n'
    new_source, changed = migrate_mut.rewrite(source, {'x'})
    assert changed == 1
    assert 'let mut x' in new_source


def test_rewrite_multiple_declarations_on_separate_lines():
    source = 'let a = 1\nlet b = 2\n'
    new_source, changed = migrate_mut.rewrite(source, {'a', 'b'})
    assert changed == 2
    assert 'let mut a' in new_source
    assert 'let mut b' in new_source


def test_rewrite_preserves_line_count():
    source = 'let a = 1\nlet b = 2\nlet c = 3\n'
    new_source, _ = migrate_mut.rewrite(source, {'a'})
    assert new_source.count('\n') == source.count('\n')


# ---------------------------------------------------------------------------
# process
# ---------------------------------------------------------------------------

def test_process_rewrites_a_real_file(tmp_path):
    path = tmp_path / 'prog.aura'
    path.write_text('def main() {\n  let x = 1\n  x = 2\n}')
    changed, err = migrate_mut.process(path)
    assert err is None
    assert changed == 1
    assert 'let mut x' in path.read_text()


def test_process_returns_zero_when_nothing_to_change(tmp_path):
    path = tmp_path / 'clean.aura'
    path.write_text('def main() {\n  let x = 1\n  print(x)\n}')
    changed, err = migrate_mut.process(path)
    assert (changed, err) == (0, None)


def test_process_reports_parse_error(tmp_path):
    path = tmp_path / 'bad.aura'
    path.write_text('let = =\n')
    changed, err = migrate_mut.process(path)
    assert changed is None
    assert err is not None and err.startswith('parse error')


def test_process_reports_check_error(tmp_path, monkeypatch):
    path = tmp_path / 'prog.aura'
    path.write_text('def main() {\n  let x = 1\n  x = 2\n}')

    def boom(_ast):
        raise RuntimeError('kaboom')

    monkeypatch.setattr(migrate_mut, 'violations_for', boom)
    changed, err = migrate_mut.process(path)
    assert changed is None
    assert err == 'check error: kaboom'


# ---------------------------------------------------------------------------
# main
# ---------------------------------------------------------------------------

def test_main_migrates_directory(tmp_path, monkeypatch, capsys):
    (tmp_path / 'a.aura').write_text('def main() {\n  let x = 1\n  x = 2\n}')
    (tmp_path / 'clean.aura').write_text('def main() {\n  let y = 1\n  print(y)\n}')

    monkeypatch.setattr(migrate_mut, 'process', migrate_mut.process)
    migrate_mut.main(['migrate_mut', str(tmp_path)])
    out = capsys.readouterr().out
    assert 'Migrated 1 files (1 declarations)' in out


def test_main_accepts_a_single_file(tmp_path, capsys):
    path = tmp_path / 'one.aura'
    path.write_text('def main() {\n  let x = 1\n  x = 2\n}')
    migrate_mut.main(['migrate_mut', str(path)])
    assert 'Migrated 1 files' in capsys.readouterr().out


def test_main_reports_skipped_files(tmp_path, capsys):
    path = tmp_path / 'bad.aura'
    path.write_text('let = =\n')
    migrate_mut.main(['migrate_mut', str(path)])
    out = capsys.readouterr().out
    assert 'SKIP' in out
    assert 'Migrated 0 files' in out


def test_main_defaults_to_no_paths(monkeypatch, capsys, tmp_path):
    captured = {}

    def fake_rglob(self, pattern):
        captured['pattern'] = pattern
        return iter(())

    monkeypatch.setattr(Path, 'rglob', fake_rglob)
    monkeypatch.chdir(tmp_path)
    migrate_mut.main(['migrate_mut'])
    assert captured['pattern'] == '*.aura'
    assert 'Migrated 0 files' in capsys.readouterr().out
