#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use aura::error::codes;
use aura::module_graph::{
    InMemorySourceProvider, ModuleGraph, ModuleGraphBuilder, SourceKey, SourceProvider,
};
use aura::native_source::NativeFilesystemSourceProvider;
use aura::{CompileMode, SharedBuf};

static NEXT_TEMP: AtomicUsize = AtomicUsize::new(0);

struct TempTree(PathBuf);

impl TempTree {
    fn new() -> Self {
        let base = std::env::temp_dir();
        let id = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let path = base.join(format!("aura-native-source-{}-{}", std::process::id(), id));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }

    fn write(&self, name: &str, text: &str) -> PathBuf {
        let path = self.0.join(name);
        fs::write(&path, text).unwrap();
        path
    }

    fn mkdir(&self, name: &str) -> PathBuf {
        let path = self.0.join(name);
        fs::create_dir_all(&path).unwrap();
        path
    }
}

impl Drop for TempTree {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn provider(tree: &TempTree, entry: &str) -> NativeFilesystemSourceProvider {
    NativeFilesystemSourceProvider::new(tree.path().join(entry)).unwrap()
}

fn graph(tree: &TempTree, entry: &str) -> ModuleGraph {
    ModuleGraphBuilder::new()
        .build(&provider(tree, entry))
        .unwrap()
}

fn paths(graph: &ModuleGraph) -> Vec<String> {
    graph
        .nodes()
        .iter()
        .map(|node| node.path().segments().join("::"))
        .collect()
}

fn run_file(tree: &TempTree, entry: &str) -> Result<String, aura::error::DiagnosticReport> {
    let compilation = aura::compile_file_with_mode(tree.path().join(entry), CompileMode::Program)?;
    let output = Arc::new(Mutex::new(Vec::new()));
    compilation.execute_with(Some(Box::new(SharedBuf(output.clone()))), Vec::new(), None)?;
    let bytes = output.lock().unwrap().clone();
    Ok(String::from_utf8(bytes).unwrap())
}

#[test]
fn entry_root_and_source_names_are_stable_and_portable() {
    let tree = TempTree::new();
    tree.write("main.aura", "fn main() { }\n");
    let provider = provider(&tree, "main.aura");
    let entry = provider.entry().unwrap().unwrap();
    assert_eq!(entry.name(), "main.aura");
    assert_eq!(entry.key().as_str(), "<native-entry>");
    assert_eq!(provider.source_root(), tree.path());
    assert_eq!(
        provider.load(entry.key()).unwrap().as_deref(),
        Some("fn main() { }\n")
    );
    assert!(!entry
        .name()
        .contains(tree.path().to_string_lossy().as_ref()));
}

#[test]
fn file_and_mod_directory_forms_build_the_same_logical_tree() {
    let file_tree = TempTree::new();
    file_tree.write("main.aura", "fn main() { print(foo::value()) }\n");
    file_tree.write("foo.aura", "pub fn value() -> int { return 1 }\n");
    let dir_tree = TempTree::new();
    dir_tree.write("main.aura", "fn main() { print(foo::value()) }\n");
    let foo = dir_tree.mkdir("foo");
    fs::write(foo.join("mod.aura"), "pub fn value() -> int { return 1 }\n").unwrap();
    assert_eq!(paths(&graph(&file_tree, "main.aura")), vec!["", "foo"]);
    assert_eq!(paths(&graph(&dir_tree, "main.aura")), vec!["", "foo"]);
}

#[test]
fn nested_file_and_directory_modules_are_discovered() {
    let tree = TempTree::new();
    tree.write("main.aura", "fn main() { }\n");
    let foo = tree.mkdir("foo");
    fs::write(foo.join("mod.aura"), "\n").unwrap();
    fs::write(foo.join("bar.aura"), "\n").unwrap();
    let baz = foo.join("baz");
    fs::create_dir(&baz).unwrap();
    fs::write(baz.join("mod.aura"), "\n").unwrap();
    assert_eq!(
        paths(&graph(&tree, "main.aura")),
        vec!["", "foo", "foo::bar", "foo::baz"]
    );
}

#[test]
fn children_are_sorted_and_repeated_builds_are_identical() {
    let tree = TempTree::new();
    tree.write("main.aura", "fn main() { }\n");
    for name in ["zeta.aura", "alpha.aura", "middle.aura"] {
        tree.write(name, "\n");
    }
    let first = graph(&tree, "main.aura");
    let second = graph(&tree, "main.aura");
    assert_eq!(paths(&first), vec!["", "alpha", "middle", "zeta"]);
    let first_names: Vec<_> = first.sources().iter().map(|source| source.name()).collect();
    let second_names: Vec<_> = second
        .sources()
        .iter()
        .map(|source| source.name())
        .collect();
    assert_eq!(first_names, second_names);
}

#[test]
fn file_directory_collision_is_deterministic_and_portable() {
    let tree = TempTree::new();
    tree.write("main.aura", "\n");
    tree.write("foo.aura", "\n");
    let foo = tree.mkdir("foo");
    fs::write(foo.join("mod.aura"), "\n").unwrap();
    let error = ModuleGraphBuilder::new()
        .build(&provider(&tree, "main.aura"))
        .unwrap_err();
    assert_eq!(error.diagnostic().code, codes::MODULE_SOURCE_OWNERSHIP);
    assert!(!error
        .diagnostic()
        .message
        .contains(tree.path().to_string_lossy().as_ref()));
    assert!(error.diagnostic().message.contains("foo"));
}

#[test]
fn missing_child_is_reported_as_a_source_path_error() {
    let tree = TempTree::new();
    tree.write("main.aura", "\n");
    let provider = provider(&tree, "main.aura");
    let missing = SourceKey::new("missing.aura");
    assert_eq!(
        provider.load(&missing).unwrap_err().message(),
        "source missing.aura cannot be inspected: not found"
    );
}

#[test]
fn invalid_utf8_preserves_source_provenance_without_temp_prefix() {
    let tree = TempTree::new();
    tree.write("main.aura", "\n");
    fs::write(tree.path().join("bad.aura"), [b'f', b'n', b' ', 0xff]).unwrap();
    let provider = provider(&tree, "main.aura");
    let error = provider.load(&SourceKey::new("bad.aura")).unwrap_err();
    assert!(error.message().contains("valid UTF-8"));
    let report = ModuleGraphBuilder::new().build(&provider).unwrap_err();
    assert_eq!(report.diagnostic().code, codes::INVALID_CHAR);
    let location = report
        .location()
        .expect("invalid UTF-8 keeps source identity");
    assert_eq!(
        report.sources().get(location.source).unwrap().name(),
        "bad.aura"
    );
    assert!(!report
        .diagnostic()
        .message
        .contains(tree.path().to_string_lossy().as_ref()));
}

#[test]
fn source_key_traversal_and_absolute_keys_are_rejected() {
    let tree = TempTree::new();
    tree.write("main.aura", "\n");
    let provider = provider(&tree, "main.aura");
    let absolute = tree
        .path()
        .join("secret.aura")
        .to_string_lossy()
        .into_owned();
    for key in [
        "../secret.aura",
        "foo/../../secret.aura",
        ".",
        "./secret.aura",
        absolute.as_str(),
    ] {
        let error = provider.load(&SourceKey::new(key)).unwrap_err();
        assert!(
            error.message().contains("escapes or is not relative"),
            "{key}: {}",
            error.message()
        );
    }
}

#[cfg(unix)]
#[test]
fn symlinked_entry_ancestor_and_child_are_rejected() {
    use std::os::unix::fs::symlink;
    let tree = TempTree::new();
    tree.write("main.aura", "\n");
    tree.write("child.aura", "\n");
    symlink(
        tree.path().join("child.aura"),
        tree.path().join("link.aura"),
    )
    .unwrap();
    assert!(provider(&tree, "main.aura")
        .load(&SourceKey::new("link.aura"))
        .is_err());
    symlink(
        tree.path().join("main.aura"),
        tree.path().join("entry.aura"),
    )
    .unwrap();
    assert!(
        NativeFilesystemSourceProvider::new(tree.path().join("entry.aura"))
            .unwrap()
            .entry()
            .is_err()
    );
}

#[cfg(unix)]
#[test]
fn symlinked_mod_owner_is_rejected() {
    use std::os::unix::fs::symlink;
    let tree = TempTree::new();
    tree.write("main.aura", "\n");
    let foo = tree.mkdir("foo");
    let real = tree.write("real.aura", "\n");
    symlink(real, foo.join("mod.aura")).unwrap();
    assert!(provider(&tree, "main.aura")
        .children(&SourceKey::new("<native-entry>"))
        .is_err());
}

#[test]
fn case_only_module_names_are_rejected() {
    let tree = TempTree::new();
    tree.write("main.aura", "\n");
    tree.write("Foo.aura", "\n");
    tree.write("foo.aura", "\n\n");
    if fs::canonicalize(tree.path().join("Foo.aura")).unwrap()
        == fs::canonicalize(tree.path().join("foo.aura")).unwrap()
    {
        return;
    }
    let error = provider(&tree, "main.aura")
        .children(&SourceKey::new("<native-entry>"))
        .unwrap_err();
    assert!(error.message().contains("case-only module collision"));
}

#[test]
fn relative_entry_paths_normalize_without_changing_diagnostics() {
    let tree = TempTree::new();
    tree.write("main.aura", "\n");
    let provider = NativeFilesystemSourceProvider::new(tree.path().join("x/../main.aura")).unwrap();
    assert_eq!(provider.entry_path(), tree.path().join("main.aura"));
    assert_eq!(provider.entry().unwrap().unwrap().name(), "main.aura");
}

#[test]
fn selected_entry_is_not_rediscovered_as_a_logical_child() {
    let tree = TempTree::new();
    tree.write("app.aura", "fn main() { }\n");
    assert_eq!(paths(&graph(&tree, "app.aura")), vec![""]);
}

#[cfg(unix)]
#[test]
fn invalid_candidates_are_selected_in_stable_name_order() {
    use std::os::unix::fs::symlink;

    let tree = TempTree::new();
    tree.write("main.aura", "\n");
    let target = tree.write("target.txt", "x");
    symlink(&target, tree.path().join("z_bad.aura")).unwrap();
    symlink(&target, tree.path().join("a_bad.aura")).unwrap();

    for _ in 0..16 {
        let error = provider(&tree, "main.aura")
            .children(&SourceKey::new("<native-entry>"))
            .unwrap_err();
        assert!(
            error.message().contains("a_bad.aura"),
            "{}",
            error.message()
        );
    }
}

#[cfg(unix)]
#[test]
fn symlinked_source_root_ancestor_is_rejected() {
    use std::os::unix::fs::symlink;

    let container = TempTree::new();
    let real = container.mkdir("real");
    fs::write(real.join("main.aura"), "fn main() { }\n").unwrap();
    let link = container.path().join("link");
    symlink(&real, &link).unwrap();

    let provider = NativeFilesystemSourceProvider::new(link.join("main.aura")).unwrap();
    assert!(provider.entry().is_err());
}

#[cfg(unix)]
#[test]
fn permission_denied_optional_owner_is_not_treated_as_absence() {
    use std::os::unix::fs::PermissionsExt;

    let tree = TempTree::new();
    tree.write("main.aura", "\n");
    let locked = tree.mkdir("locked");
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();

    let result = ModuleGraphBuilder::new().build(&provider(&tree, "main.aura"));
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o700)).unwrap();

    let error = result.expect_err("permission denial must be a provider failure");
    assert_eq!(error.diagnostic().code, codes::IO);
    assert!(
        error.diagnostic().message.contains("permission denied"),
        "{}",
        error.diagnostic().message
    );
    assert!(!error
        .diagnostic()
        .message
        .contains(tree.path().to_string_lossy().as_ref()));
}

#[cfg(unix)]
#[test]
fn symlink_module_directory_diagnostic_never_leaks_absolute_root() {
    use std::os::unix::fs::symlink;

    let tree = TempTree::new();
    tree.write("main.aura", "\n");
    tree.write("foo.aura", "\n");
    let outside = tree.mkdir("outside");
    symlink(&outside, tree.path().join("foo")).unwrap();

    let error = ModuleGraphBuilder::new()
        .build(&provider(&tree, "main.aura"))
        .unwrap_err();
    assert_eq!(error.diagnostic().code, codes::MODULE_SOURCE_PATH);
    assert!(error.diagnostic().message.contains("foo"));
    assert!(
        !error
            .diagnostic()
            .message
            .contains(tree.path().to_string_lossy().as_ref()),
        "{}",
        error.diagnostic().message
    );
}

#[test]
fn direct_mod_aura_selection_owns_the_logical_root() {
    let tree = TempTree::new();
    tree.write("mod.aura", "fn main() { }\n");
    let graph = graph(&tree, "mod.aura");
    assert_eq!(paths(&graph), vec![""]);
    assert_eq!(graph.sources().iter().next().unwrap().name(), "mod.aura");
}

#[test]
fn sibling_root_mod_aura_conflicts_with_selected_entry() {
    let tree = TempTree::new();
    tree.write("main.aura", "fn main() { }\n");
    tree.write("mod.aura", "\n");
    let error = ModuleGraphBuilder::new()
        .build(&provider(&tree, "main.aura"))
        .unwrap_err();
    assert_eq!(error.diagnostic().code, codes::MODULE_SOURCE_OWNERSHIP);
    assert!(error.diagnostic().message.contains("<root>"));
}

#[test]
fn source_root_is_exactly_entry_parent_and_never_discovers_upward() {
    let tree = TempTree::new();
    tree.write("outside.aura", "@\n");
    let app = tree.mkdir("app");
    fs::write(app.join("main.aura"), "fn main() { }\n").unwrap();
    fs::write(
        app.join("child.aura"),
        "pub fn value() -> int { return 1 }\n",
    )
    .unwrap();

    let provider = NativeFilesystemSourceProvider::new(app.join("main.aura")).unwrap();
    assert_eq!(provider.source_root(), app);
    let graph = ModuleGraphBuilder::new().build(&provider).unwrap();
    assert_eq!(paths(&graph), vec!["", "child"]);
    assert!(graph
        .sources()
        .iter()
        .all(|source| source.name() != "outside.aura"));
}

#[test]
fn in_source_and_physical_child_ownership_collision_is_deterministic() {
    let tree = TempTree::new();
    tree.write("main.aura", "module bar { }\nfn main() { }\n");
    tree.write("bar.aura", "\n");
    for _ in 0..8 {
        let error = ModuleGraphBuilder::new()
            .build(&provider(&tree, "main.aura"))
            .unwrap_err();
        assert_eq!(error.diagnostic().code, codes::MODULE_SOURCE_OWNERSHIP);
        assert!(error.diagnostic().message.contains("bar"));
    }
}

#[test]
fn physical_creation_order_does_not_change_graph_order() {
    fn make(order: &[&str]) -> TempTree {
        let tree = TempTree::new();
        tree.write("main.aura", "fn main() { }\n");
        for name in order {
            tree.write(name, "\n");
        }
        tree
    }

    let left = make(&["z.aura", "a.aura", "m.aura"]);
    let right = make(&["m.aura", "a.aura", "z.aura"]);
    let left_graph = graph(&left, "main.aura");
    let right_graph = graph(&right, "main.aura");
    assert_eq!(paths(&left_graph), paths(&right_graph));
    let left_names: Vec<_> = left_graph
        .sources()
        .iter()
        .map(|source| source.name())
        .collect();
    let right_names: Vec<_> = right_graph
        .sources()
        .iter()
        .map(|source| source.name())
        .collect();
    assert_eq!(left_names, right_names);
}

#[test]
fn directory_without_mod_aura_is_an_absent_optional_owner() {
    let tree = TempTree::new();
    tree.write("main.aura", "fn main() { }\n");
    tree.mkdir("empty");
    assert_eq!(paths(&graph(&tree, "main.aura")), vec![""]);
}

#[test]
fn nested_source_names_always_use_forward_slashes() {
    let tree = TempTree::new();
    tree.write("main.aura", "fn main() { }\n");
    let foo = tree.mkdir("foo");
    fs::write(foo.join("mod.aura"), "\n").unwrap();
    fs::write(foo.join("bar.aura"), "\n").unwrap();
    let graph = graph(&tree, "main.aura");
    assert!(graph
        .sources()
        .iter()
        .any(|source| source.name() == "foo/bar.aura"));
    assert!(graph
        .sources()
        .iter()
        .all(|source| !source.name().contains('\\')));
}

#[test]
fn unicode_entry_is_preserved_and_non_identifier_child_is_ignored() {
    let tree = TempTree::new();
    tree.write("máin.aura", "fn main() { }\n");
    tree.write("módulo.aura", "@\n");
    let graph = graph(&tree, "máin.aura");
    assert_eq!(paths(&graph), vec![""]);
    assert_eq!(graph.sources().iter().next().unwrap().name(), "máin.aura");
}

#[test]
fn mod_owner_spelling_is_exact_on_every_filesystem() {
    let tree = TempTree::new();
    tree.write("main.aura", "fn main() { }\n");
    let foo = tree.mkdir("foo");
    fs::write(foo.join("Mod.aura"), "@\n").unwrap();
    assert_eq!(paths(&graph(&tree, "main.aura")), vec![""]);
}

#[test]
fn selected_entry_spelling_is_exact_on_case_insensitive_filesystems() {
    let tree = TempTree::new();
    tree.write("main.aura", "fn main() { }\n");
    let mismatched = tree.path().join("MAIN.aura");
    if !mismatched.exists() {
        return;
    }

    let native = NativeFilesystemSourceProvider::new(&mismatched).unwrap();
    let error = ModuleGraphBuilder::new()
        .build(&native)
        .expect_err("entry spelling must be exact even when the host is case-insensitive");
    assert_eq!(error.diagnostic().code, codes::IO);
    assert!(error.diagnostic().message.contains("not found"));
}

#[cfg(unix)]
#[test]
fn non_utf8_candidate_name_is_ignored_without_affecting_determinism() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let tree = TempTree::new();
    tree.write("main.aura", "fn main() { }\n");
    let invalid = OsString::from_vec(vec![b'b', b'a', b'd', 0xff, b'.', b'a', b'u', b'r', b'a']);
    if fs::write(tree.path().join(invalid), b"@").is_err() {
        return;
    }
    assert_eq!(paths(&graph(&tree, "main.aura")), vec![""]);
}

#[test]
fn native_and_in_memory_providers_feed_the_same_graph_and_semantics() {
    let tree = TempTree::new();
    tree.write("main.aura", "fn main() { print(child::value()) }\n");
    tree.write("child.aura", "pub fn value() -> int { return 13 }\n");
    let native_graph = graph(&tree, "main.aura");

    let root = SourceKey::new("entry");
    let child = SourceKey::new("child");
    let mut memory = InMemorySourceProvider::new(root.clone());
    memory
        .insert_source(
            root.clone(),
            "main.aura",
            "fn main() { print(child::value()) }\n",
        )
        .unwrap();
    memory
        .insert_source(
            child.clone(),
            "child.aura",
            "pub fn value() -> int { return 13 }\n",
        )
        .unwrap();
    memory.add_child(&root, "child", &child).unwrap();
    let memory_graph = ModuleGraphBuilder::new().build(&memory).unwrap();
    assert_eq!(paths(&native_graph), paths(&memory_graph));

    assert_eq!(run_file(&tree, "main.aura").unwrap(), "13\n");
    let output = Arc::new(Mutex::new(Vec::new()));
    aura::compile_provider_with_mode(&memory, CompileMode::Program)
        .unwrap()
        .execute_with(Some(Box::new(SharedBuf(output.clone()))), Vec::new(), None)
        .unwrap();
    assert_eq!(
        String::from_utf8(output.lock().unwrap().clone()).unwrap(),
        "13\n"
    );
}

#[test]
fn real_files_preserve_frontend_and_runtime_provenance() {
    for (label, child_source, expected) in [
        ("parser", "@\n", codes::INVALID_CHAR),
        ("resolver", "use missing::value\n", codes::UNKNOWN_MODULE),
        (
            "checker",
            "pub fn bad() -> int { return \"wrong\" }\n",
            codes::RETURN_MISMATCH,
        ),
        (
            "delayed-checker",
            "pub fn f(x: int) -> int { return 1 }\npub fn f(x: int) -> int { return 2 }\n",
            codes::REDECLARED,
        ),
    ] {
        let tree = TempTree::new();
        tree.write("main.aura", "fn main() { }\n");
        tree.write("child.aura", child_source);
        let report =
            aura::compile_file_with_mode(tree.path().join("main.aura"), CompileMode::Program)
                .expect_err(label);
        assert_eq!(report.diagnostic().code, expected, "{label}");
        let location = report.location().expect("real child diagnostic has source");
        assert_eq!(
            report.sources().get(location.source).unwrap().name(),
            "child.aura"
        );
    }

    let tree = TempTree::new();
    tree.write("main.aura", "fn main() { child::boom() }\n");
    tree.write("child.aura", "pub fn boom() { assert(false) }\n");
    let compilation =
        aura::compile_file_with_mode(tree.path().join("main.aura"), CompileMode::Program).unwrap();
    let report = compilation
        .execute_with(None, Vec::new(), None)
        .expect_err("runtime error");
    assert_eq!(report.diagnostic().code, codes::ASSERT);
    let location = report.location().unwrap();
    assert_eq!(
        report.sources().get(location.source).unwrap().name(),
        "child.aura"
    );
}

#[test]
fn real_files_with_identical_local_spans_keep_distinct_source_ids() {
    let tree = TempTree::new();
    tree.write("main.aura", "fn main() { }\n");
    tree.write("a.aura", "pub fn value() -> int { return 1 }\n");
    tree.write("b.aura", "pub fn value() -> int { return 1 }\n");
    let graph = graph(&tree, "main.aura");
    let a = graph
        .sources()
        .iter()
        .find(|source| source.name() == "a.aura")
        .unwrap();
    let b = graph
        .sources()
        .iter()
        .find(|source| source.name() == "b.aura")
        .unwrap();
    assert_ne!(a.id(), b.id());
    assert_eq!(a.text(), b.text());
}

#[test]
fn missing_main_is_attributed_to_real_entry_file() {
    let tree = TempTree::new();
    tree.write("entry.aura", "\n");
    tree.write("child.aura", "pub fn helper() -> int { return 1 }\n");
    let report = aura::compile_file_with_mode(tree.path().join("entry.aura"), CompileMode::Program)
        .expect_err("missing main");
    assert_eq!(report.diagnostic().code, codes::NO_MAIN);
    let location = report.location().unwrap();
    assert_eq!(
        report.sources().get(location.source).unwrap().name(),
        "entry.aura"
    );
}

#[test]
fn real_file_wrapper_depth_cannot_bypass_e1015() {
    let budget = aura::parse::parse_recursion_budget();
    let mut nested = String::new();
    for _ in 0..budget {
        nested.push_str("module nested {\n");
    }
    for _ in 0..budget {
        nested.push_str("}\n");
    }
    assert!(aura::parse::parse(&nested).is_ok());

    let mut equivalent = String::from("module child {\n");
    equivalent.push_str(&nested);
    equivalent.push_str("}\n");
    let in_source =
        aura::parse::parse(&equivalent).expect_err("one extra wrapper must exceed depth");
    assert_eq!(in_source.code, codes::NESTING);

    let tree = TempTree::new();
    tree.write("main.aura", "\n");
    tree.write("child.aura", &nested);
    let report = ModuleGraphBuilder::new()
        .build(&provider(&tree, "main.aura"))
        .expect_err("physical wrapper must consume the same depth budget");
    assert_eq!(report.diagnostic().code, codes::NESTING);
    let location = report.location().unwrap();
    assert_eq!(
        report.sources().get(location.source).unwrap().name(),
        "child.aura"
    );
}
