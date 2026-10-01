#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]
//! Provider-neutral module graph foundation.

use std::sync::{Arc, Mutex};

use aura::error::{codes, Diag};
use aura::module_graph::{
    InMemorySourceProvider, ModuleGraph, ModuleGraphBuilder, ProviderChild, ProviderError,
    SourceDescriptor, SourceKey, SourceProvider,
};
use aura::source::SourceMap;
use aura::{CompileMode, SharedBuf};

fn key(value: &str) -> SourceKey {
    SourceKey::new(value)
}

fn provider(root_source: &str) -> (InMemorySourceProvider, SourceKey) {
    let root = key("entry");
    let mut provider = InMemorySourceProvider::new(root.clone());
    provider
        .insert_source(root.clone(), "main.aura", root_source)
        .unwrap();
    (provider, root)
}

fn add_source(
    provider: &mut InMemorySourceProvider,
    parent: &SourceKey,
    logical_name: &str,
    source_key: &str,
    display_name: &str,
    source: &str,
) -> SourceKey {
    let child = key(source_key);
    provider
        .insert_source(child.clone(), display_name, source)
        .unwrap();
    provider.add_child(parent, logical_name, &child).unwrap();
    child
}

fn build(provider: &InMemorySourceProvider) -> ModuleGraph {
    ModuleGraphBuilder::new()
        .build(provider)
        .expect("module graph should build")
}

fn run_graph(graph: ModuleGraph) -> Result<String, Diag> {
    let (module, _, _, _) = graph.into_parts();
    let module = aura::resolve::resolve(module)?;
    aura::check::Checker::module_in_mode(&module, CompileMode::Program)?;
    let out = Arc::new(Mutex::new(Vec::new()));
    aura::execute(module, Some(Box::new(SharedBuf(out.clone()))))?;
    let bytes = out.lock().unwrap().clone();
    Ok(String::from_utf8(bytes).unwrap())
}

fn graph_paths(graph: &ModuleGraph) -> Vec<String> {
    graph
        .nodes()
        .iter()
        .map(|node| node.path().segments().join("::"))
        .collect()
}

#[test]
fn existing_one_file_compilation_is_unchanged() {
    assert_eq!(
        aura::run_source("fn main() { print(7) }", "<single>").unwrap(),
        "7\n"
    );
}

#[test]
fn existing_in_source_modules_are_unchanged() {
    let source =
        "module child { pub fn value() -> int { return 8 } }\nfn main() { print(child::value()) }";
    assert_eq!(aura::run_source(source, "<in-source>").unwrap(), "8\n");
}

#[test]
fn one_external_logical_child_lowers_into_the_existing_pipeline() {
    let (mut provider, root) = provider("fn main() { print(child::value()) }");
    add_source(
        &mut provider,
        &root,
        "child",
        "child-key",
        "child.aura",
        "pub fn value() -> int { return 9 }",
    );

    let graph = build(&provider);
    assert_eq!(graph_paths(&graph), vec!["", "child"]);
    assert_eq!(run_graph(graph).unwrap(), "9\n");
}

#[test]
fn nested_external_modules_are_public_containers() {
    let (mut provider, root) = provider("fn main() { print(foo::bar::value()) }");
    let foo = add_source(&mut provider, &root, "foo", "foo-key", "foo.aura", "");
    add_source(
        &mut provider,
        &foo,
        "bar",
        "bar-key",
        "foo/bar.aura",
        "pub fn value() -> int { return 11 }",
    );

    let graph = build(&provider);
    assert_eq!(graph_paths(&graph), vec!["", "foo", "foo::bar"]);
    assert_eq!(run_graph(graph).unwrap(), "11\n");
}

#[test]
fn provider_backed_and_in_source_forms_are_semantically_equivalent() {
    let in_source = "module math { pub fn twice(x: int) -> int { return x * 2 } }\nfn main() { print(math::twice(6)) }";
    let expected = aura::run_source(in_source, "<equivalent>").unwrap();

    let (mut provider, root) = provider("fn main() { print(math::twice(6)) }");
    add_source(
        &mut provider,
        &root,
        "math",
        "math-key",
        "math.aura",
        "pub fn twice(x: int) -> int { return x * 2 }",
    );

    assert_eq!(run_graph(build(&provider)).unwrap(), expected);
}

#[test]
fn graph_order_is_independent_of_provider_insertion_order() {
    fn make(reverse: bool) -> ModuleGraph {
        let (mut provider, root) = provider("");
        let a = key("a-key");
        let b = key("b-key");
        if reverse {
            provider.insert_source(b.clone(), "b.aura", "").unwrap();
            provider.insert_source(a.clone(), "a.aura", "").unwrap();
            provider.add_child(&root, "b", &b).unwrap();
            provider.add_child(&root, "a", &a).unwrap();
        } else {
            provider.insert_source(a.clone(), "a.aura", "").unwrap();
            provider.insert_source(b.clone(), "b.aura", "").unwrap();
            provider.add_child(&root, "a", &a).unwrap();
            provider.add_child(&root, "b", &b).unwrap();
        }
        build(&provider)
    }

    let left = make(false);
    let right = make(true);
    assert_eq!(graph_paths(&left), vec!["", "a", "b"]);
    assert_eq!(graph_paths(&left), graph_paths(&right));
    let left_names: Vec<_> = left.sources().iter().map(|source| source.name()).collect();
    let right_names: Vec<_> = right.sources().iter().map(|source| source.name()).collect();
    assert_eq!(left_names, vec!["main.aura", "a.aura", "b.aura"]);
    assert_eq!(left_names, right_names);
}

#[test]
fn duplicate_external_ownership_is_rejected_deterministically() {
    let (mut provider, root) = provider("");
    let z = key("z-key");
    let a = key("a-key");
    provider.insert_source(z.clone(), "z/foo.aura", "").unwrap();
    provider.insert_source(a.clone(), "a/foo.aura", "").unwrap();
    provider.add_child(&root, "foo", &z).unwrap();
    provider.add_child(&root, "foo", &a).unwrap();

    let error = ModuleGraphBuilder::new().build(&provider).unwrap_err();
    assert_eq!(error.diagnostic().code, codes::MODULE_SOURCE_OWNERSHIP);
    assert!(error.location().is_none());
    assert!(error.diagnostic().message.contains("a/foo.aura`"));
    assert!(error.diagnostic().message.contains("z/foo.aura`"));
}

#[test]
fn external_and_in_source_child_ownership_collision_uses_parent_provenance() {
    let (mut provider, root) = provider("module foo { }\n");
    add_source(&mut provider, &root, "foo", "foo-key", "foo.aura", "");

    let error = ModuleGraphBuilder::new().build(&provider).unwrap_err();
    assert_eq!(error.diagnostic().code, codes::MODULE_SOURCE_OWNERSHIP);
    let location = error
        .location()
        .expect("collision has a declaration location");
    assert_eq!(
        error.sources().get(location.source).unwrap().name(),
        "main.aura"
    );
    assert_eq!(location.span.start, 0);
}

#[test]
fn distinct_sources_keep_distinct_ids_even_with_identical_local_spans() {
    let (mut provider, root) = provider("");
    add_source(
        &mut provider,
        &root,
        "a",
        "a-key",
        "a.aura",
        "pub fn value() -> int { return 1 }",
    );
    add_source(
        &mut provider,
        &root,
        "b",
        "b-key",
        "b.aura",
        "pub fn value() -> int { return 1 }",
    );

    let graph = build(&provider);
    assert_ne!(graph.nodes()[1].source(), graph.nodes()[2].source());
    assert_eq!(
        graph
            .sources()
            .get(graph.nodes()[1].source())
            .unwrap()
            .text(),
        graph
            .sources()
            .get(graph.nodes()[2].source())
            .unwrap()
            .text()
    );
}

#[test]
fn direct_provider_ownership_cycle_terminates_as_duplicate_source_identity() {
    let (mut provider, root) = provider("");
    provider.add_child(&root, "again", &root).unwrap();
    let error = ModuleGraphBuilder::new().build(&provider).unwrap_err();
    assert_eq!(error.diagnostic().code, codes::DUPLICATE_LOGICAL_SOURCE);
}

#[test]
fn indirect_provider_ownership_cycle_terminates_as_duplicate_source_identity() {
    let (mut provider, root) = provider("");
    let a = add_source(&mut provider, &root, "a", "a-key", "a.aura", "");
    provider.add_child(&a, "root", &root).unwrap();
    let error = ModuleGraphBuilder::new().build(&provider).unwrap_err();
    assert_eq!(error.diagnostic().code, codes::DUPLICATE_LOGICAL_SOURCE);
}

#[test]
fn semantic_reference_cycle_across_provider_sources_is_not_a_loader_error() {
    let (mut provider, root) = provider("fn main() { print(a::even(6)) }");
    add_source(
        &mut provider,
        &root,
        "a",
        "a-key",
        "a.aura",
        "pub fn even(n: int) -> bool { if n == 0 { return true }\n return b::odd(n - 1) }",
    );
    add_source(
        &mut provider,
        &root,
        "b",
        "b-key",
        "b.aura",
        "pub fn odd(n: int) -> bool { if n == 0 { return false }\n return a::even(n - 1) }",
    );

    assert_eq!(run_graph(build(&provider)).unwrap(), "true\n");
}

#[test]
fn indirect_semantic_reference_cycle_is_not_a_loader_error() {
    let (mut provider, root) = provider("fn main() { print(a::step(3)) }");
    add_source(
        &mut provider,
        &root,
        "a",
        "a-key",
        "a.aura",
        "pub fn step(n: int) -> int { if n == 0 { return 7 }\n return b::step(n - 1) }",
    );
    add_source(
        &mut provider,
        &root,
        "b",
        "b-key",
        "b.aura",
        "pub fn step(n: int) -> int { if n == 0 { return 7 }\n return c::step(n - 1) }",
    );
    add_source(
        &mut provider,
        &root,
        "c",
        "c-key",
        "c.aura",
        "pub fn step(n: int) -> int { if n == 0 { return 7 }\n return a::step(n - 1) }",
    );

    assert_eq!(run_graph(build(&provider)).unwrap(), "7\n");
}

#[test]
fn semantic_diamond_loads_shared_owner_once() {
    let (mut provider, root) = provider("fn main() { print(a::value() + b::value()) }");
    add_source(
        &mut provider,
        &root,
        "shared",
        "shared-key",
        "shared.aura",
        "pub fn value() -> int { return 4 }",
    );
    add_source(
        &mut provider,
        &root,
        "a",
        "a-key",
        "a.aura",
        "pub use shared::value",
    );
    add_source(
        &mut provider,
        &root,
        "b",
        "b-key",
        "b.aura",
        "pub use shared::value",
    );

    let graph = build(&provider);
    assert_eq!(graph.nodes().len(), 4);
    assert_eq!(run_graph(graph).unwrap(), "8\n");
}

#[test]
fn aliases_and_reexports_remain_resolver_owned() {
    let (mut provider, root) = provider("use a as m\nfn main() { print(m::value() + b::value()) }");
    add_source(
        &mut provider,
        &root,
        "a",
        "a-key",
        "a.aura",
        "pub fn value() -> int { return 5 }",
    );
    add_source(
        &mut provider,
        &root,
        "b",
        "b-key",
        "b.aura",
        "pub use a::value",
    );

    assert_eq!(run_graph(build(&provider)).unwrap(), "10\n");
}

#[test]
fn malformed_child_diagnostic_uses_the_child_source_id() {
    let (mut provider, root) = provider("");
    add_source(
        &mut provider,
        &root,
        "child",
        "child-key",
        "child.aura",
        "@",
    );

    let error = ModuleGraphBuilder::new().build(&provider).unwrap_err();
    assert_eq!(error.diagnostic().code, codes::INVALID_CHAR);
    let location = error.location().expect("parse diagnostic is source-aware");
    assert_eq!(
        error.sources().get(location.source).unwrap().name(),
        "child.aura"
    );
    assert_eq!(location.span.start, 0);
}

#[test]
fn resolver_error_in_child_keeps_child_provenance() {
    let (mut provider, root) = provider("fn main() { }\n");
    add_source(
        &mut provider,
        &root,
        "child",
        "child-key",
        "child.aura",
        "use missing\n",
    );

    let error = aura::compile_provider_with_mode(&provider, CompileMode::Program).unwrap_err();
    assert_eq!(error.diagnostic().code, codes::UNKNOWN_MODULE);
    let location = error
        .location()
        .expect("resolver diagnostic is source-aware");
    assert_eq!(
        error.sources().get(location.source).unwrap().name(),
        "child.aura"
    );
}

#[test]
fn checker_error_in_child_keeps_child_provenance() {
    let (mut provider, root) = provider("fn main() { }\n");
    add_source(
        &mut provider,
        &root,
        "child",
        "child-key",
        "child.aura",
        "pub fn bad() -> int { return \"wrong\" }\n",
    );

    let error = aura::compile_provider_with_mode(&provider, CompileMode::Program).unwrap_err();
    assert_eq!(error.diagnostic().code, codes::RETURN_MISMATCH);
    let location = error
        .location()
        .expect("checker diagnostic is source-aware");
    assert_eq!(
        error.sources().get(location.source).unwrap().name(),
        "child.aura"
    );
}

#[test]
fn delayed_overload_checker_error_keeps_its_child_provenance() {
    let (mut provider, root) = provider("fn main() { }\n");
    add_source(
        &mut provider,
        &root,
        "a",
        "a-key",
        "a.aura",
        "pub fn f(x: int) -> int { return 1 }\npub fn f(x: int) -> int { return 2 }\n",
    );
    add_source(
        &mut provider,
        &root,
        "z",
        "z-key",
        "z.aura",
        "pub fn ok() -> int { return 3 }\n",
    );

    let error = aura::compile_provider_with_mode(&provider, CompileMode::Program).unwrap_err();
    assert_eq!(error.diagnostic().code, codes::REDECLARED);
    let location = error
        .location()
        .expect("overload diagnostic is source-aware");
    assert_eq!(
        error.sources().get(location.source).unwrap().name(),
        "a.aura"
    );
}

#[test]
fn multi_source_overload_diagnostic_order_is_deterministic() {
    for _ in 0..64 {
        let (mut provider, root) = provider("");
        add_source(
            &mut provider,
            &root,
            "a",
            "a-key",
            "a.aura",
            "pub fn f(x: int) -> int { return 1 }\npub fn f(x: int) -> int { return 2 }\n",
        );
        add_source(
            &mut provider,
            &root,
            "b",
            "b-key",
            "b.aura",
            "pub fn f(x: int) -> int { return 3 }\npub fn f(x: int) -> int { return 4 }\n",
        );
        let error = aura::compile_provider_with_mode(&provider, CompileMode::Module).unwrap_err();
        assert_eq!(error.diagnostic().code, codes::REDECLARED);
        let location = error.location().expect("duplicate overload has source");
        assert_eq!(
            error.sources().get(location.source).unwrap().name(),
            "a.aura"
        );
        assert!(error.diagnostic().message.contains("a::f"));
    }
}

#[test]
fn missing_main_is_attributed_to_the_entry_source() {
    let (mut provider, root) = provider("");
    add_source(
        &mut provider,
        &root,
        "child",
        "child-key",
        "child.aura",
        "pub fn helper() -> int { return 1 }\n",
    );
    let error = aura::compile_provider_with_mode(&provider, CompileMode::Program).unwrap_err();
    assert_eq!(error.diagnostic().code, codes::NO_MAIN);
    let location = error
        .location()
        .expect("missing main belongs to entry source");
    assert_eq!(
        error.sources().get(location.source).unwrap().name(),
        "main.aura"
    );
}

#[test]
fn runtime_error_in_child_keeps_child_provenance() {
    let (mut provider, root) = provider("fn main() { child::boom() }\n");
    add_source(
        &mut provider,
        &root,
        "child",
        "child-key",
        "child.aura",
        "pub fn boom() { assert(false) }\n",
    );

    let compilation =
        aura::compile_provider_with_mode(&provider, CompileMode::Program).expect("compile");
    let error = compilation
        .execute_with(None, Vec::new(), None)
        .expect_err("runtime must reject");
    assert_eq!(error.diagnostic().code, codes::ASSERT);
    let location = error
        .location()
        .expect("runtime diagnostic is source-aware");
    assert_eq!(
        error.sources().get(location.source).unwrap().name(),
        "child.aura"
    );
}

#[test]
fn external_depth_failure_is_attributed_to_the_child_source() {
    // Module nesting counts toward the semantic AST limit on every substrate
    // (ADR-0004): 256 levels parse, 257 is `E1015`. This test's intent is that
    // a depth failure inside an external child is reported against *that*
    // child source, so it uses a nesting depth just past the semantic limit.
    let semantic_limit = 256;
    let mut child_source = String::new();
    for _ in 0..=semantic_limit {
        child_source.push_str("module nested {\n");
    }
    for _ in 0..=semantic_limit {
        child_source.push_str("}\n");
    }
    assert!(
        aura::parse::parse(&child_source).is_err(),
        "257 nested modules must be rejected by the parser"
    );

    let (mut provider, root) = provider("");
    add_source(
        &mut provider,
        &root,
        "child",
        "child-key",
        "child.aura",
        &child_source,
    );
    let error = ModuleGraphBuilder::new().build(&provider).unwrap_err();
    assert_eq!(error.diagnostic().code, codes::NESTING);
    let location = error
        .location()
        .expect("depth failure belongs to child source");
    assert_eq!(
        error.sources().get(location.source).unwrap().name(),
        "child.aura"
    );
}

#[test]
fn repeated_build_has_identical_order_and_collision_diagnostic() {
    fn make(reverse: bool) -> InMemorySourceProvider {
        let (mut provider, root) = provider("");
        let left = key("left");
        let right = key("right");
        provider
            .insert_source(left.clone(), "z-owner.aura", "")
            .unwrap();
        provider
            .insert_source(right.clone(), "a-owner.aura", "")
            .unwrap();
        if reverse {
            provider.add_child(&root, "dup", &right).unwrap();
            provider.add_child(&root, "dup", &left).unwrap();
        } else {
            provider.add_child(&root, "dup", &left).unwrap();
            provider.add_child(&root, "dup", &right).unwrap();
        }
        provider
    }

    let first = ModuleGraphBuilder::new().build(&make(false)).unwrap_err();
    let second = ModuleGraphBuilder::new().build(&make(true)).unwrap_err();
    assert_eq!(first.diagnostic().code, second.diagnostic().code);
    assert_eq!(first.diagnostic().message, second.diagnostic().message);
}

#[test]
fn source_keys_are_opaque_and_have_no_filesystem_semantics() {
    let root = key("vfs://root/../opaque");
    let mut provider = InMemorySourceProvider::new(root.clone());
    provider
        .insert_source(
            root.clone(),
            "<vfs-main>",
            "fn main() { print(foo::value()) }",
        )
        .unwrap();
    let child = key("C:\\not\\a\\host\\path/../child");
    provider
        .insert_source(
            child.clone(),
            "<vfs-child>",
            "pub fn value() -> int { return 13 }",
        )
        .unwrap();
    provider.add_child(&root, "foo", &child).unwrap();

    assert_eq!(run_graph(build(&provider)).unwrap(), "13\n");
}

#[test]
fn foreign_source_ids_still_cannot_resolve_in_another_map() {
    let mut left = SourceMap::new();
    let mut right = SourceMap::new();
    let left_id = left.add("left", "x");
    let right_id = right.add("right", "x");
    assert!(left.get(right_id).is_none());
    assert!(right.get(left_id).is_none());
}

#[test]
fn duplicate_same_source_for_one_logical_child_is_rejected() {
    let (mut provider, root) = provider("");
    let child = key("child");
    provider
        .insert_source(child.clone(), "child.aura", "")
        .unwrap();
    provider.add_child(&root, "child", &child).unwrap();
    provider.add_child(&root, "child", &child).unwrap();

    let error = ModuleGraphBuilder::new().build(&provider).unwrap_err();
    assert_eq!(error.diagnostic().code, codes::DUPLICATE_LOGICAL_SOURCE);
}

#[test]
fn one_provider_source_cannot_own_two_logical_modules() {
    let (mut provider, root) = provider("");
    let child = key("child");
    provider
        .insert_source(child.clone(), "child.aura", "")
        .unwrap();
    provider.add_child(&root, "a", &child).unwrap();
    provider.add_child(&root, "b", &child).unwrap();

    let error = ModuleGraphBuilder::new().build(&provider).unwrap_err();
    assert_eq!(error.diagnostic().code, codes::DUPLICATE_LOGICAL_SOURCE);
    assert!(error.diagnostic().message.contains("`a`"));
    assert!(error.diagnostic().message.contains("`b`"));
}

#[test]
fn invalid_logical_child_name_is_a_module_source_path_error() {
    let (mut provider, root) = provider("");
    add_source(&mut provider, &root, "../foo", "child", "child.aura", "");
    let error = ModuleGraphBuilder::new().build(&provider).unwrap_err();
    assert_eq!(error.diagnostic().code, codes::MODULE_SOURCE_PATH);
    assert!(error.location().is_none());
}

#[test]
fn invalid_logical_child_diagnostic_is_independent_of_provider_order() {
    fn diagnostic(order: [&str; 2]) -> String {
        let (mut provider, root) = provider("");
        for (index, logical_name) in order.into_iter().enumerate() {
            let child = key(&format!("child-{index}"));
            provider
                .insert_source(child.clone(), format!("child-{index}.aura"), "")
                .unwrap();
            provider.add_child(&root, logical_name, &child).unwrap();
        }
        ModuleGraphBuilder::new()
            .build(&provider)
            .unwrap_err()
            .diagnostic()
            .to_string()
    }

    let left = diagnostic(["../z", "./a"]);
    let right = diagnostic(["./a", "../z"]);
    assert_eq!(left, right);
    assert!(left.contains("../z"));
}

#[test]
fn case_only_virtual_children_are_rejected_portably() {
    let (mut provider, root) = provider("");
    for (logical_name, source_key) in [("Foo", "upper"), ("foo", "lower")] {
        let child = key(source_key);
        provider
            .insert_source(child.clone(), format!("{logical_name}.aura"), "")
            .unwrap();
        provider.add_child(&root, logical_name, &child).unwrap();
    }

    let error = ModuleGraphBuilder::new().build(&provider).unwrap_err();
    assert_eq!(error.diagnostic().code, codes::MODULE_SOURCE_PATH);
    assert!(error
        .diagnostic()
        .message
        .contains("case-only module collision"));
    assert!(error.diagnostic().message.contains("Foo"));
    assert!(error.diagnostic().message.contains("foo"));
}

#[test]
fn case_only_collision_order_matches_sorted_pairwise_precedence() {
    fn diagnostic(reverse: bool) -> String {
        let (mut provider, root) = provider("");
        let claims = [
            ("A", "upper-a"),
            ("B", "upper-b"),
            ("a", "lower-a"),
            ("b", "lower-b"),
        ];
        for (_, source_key) in claims {
            let child = key(source_key);
            provider
                .insert_source(child.clone(), format!("{source_key}.aura"), "")
                .unwrap();
        }
        let iter: Box<dyn Iterator<Item = (&str, &str)>> = if reverse {
            Box::new(claims.into_iter().rev())
        } else {
            Box::new(claims.into_iter())
        };
        for (logical_name, source_key) in iter {
            provider
                .add_child(&root, logical_name, &key(source_key))
                .unwrap();
        }
        ModuleGraphBuilder::new()
            .build(&provider)
            .unwrap_err()
            .diagnostic()
            .message
            .clone()
    }

    let forward = diagnostic(false);
    let reversed = diagnostic(true);
    assert_eq!(forward, reversed);
    assert!(forward.contains("A and a"), "{forward}");
    assert!(!forward.contains("B and b"), "{forward}");
}

#[test]
fn large_child_set_reaches_duplicate_source_diagnostic_without_pairwise_case_scan() {
    const CLAIMS: usize = 64_000;

    let (mut provider, root) = provider("");
    let shared = key("shared");
    provider
        .insert_source(shared.clone(), "shared.aura", "")
        .unwrap();
    for index in 0..CLAIMS {
        provider
            .add_child(&root, format!("m{index:x}"), &shared)
            .unwrap();
    }

    let error = ModuleGraphBuilder::new().build(&provider).unwrap_err();
    assert_eq!(error.diagnostic().code, codes::DUPLICATE_LOGICAL_SOURCE);
}

struct MissingChildProvider {
    entry: SourceDescriptor,
    child: SourceDescriptor,
}

struct ReadFailureProvider {
    entry: SourceDescriptor,
    child: SourceDescriptor,
}

impl SourceProvider for ReadFailureProvider {
    fn entry(&self) -> Result<Option<SourceDescriptor>, ProviderError> {
        Ok(Some(self.entry.clone()))
    }

    fn load(&self, key: &SourceKey) -> Result<Option<Arc<str>>, ProviderError> {
        if key == self.entry.key() {
            Ok(Some(Arc::<str>::from("")))
        } else {
            Err(ProviderError::new("simulated read failure"))
        }
    }

    fn children(&self, parent: &SourceKey) -> Result<Vec<ProviderChild>, ProviderError> {
        if parent == self.entry.key() {
            Ok(vec![ProviderChild::new("child", self.child.clone())])
        } else {
            Ok(Vec::new())
        }
    }
}

impl SourceProvider for MissingChildProvider {
    fn entry(&self) -> Result<Option<SourceDescriptor>, ProviderError> {
        Ok(Some(self.entry.clone()))
    }

    fn load(&self, key: &SourceKey) -> Result<Option<Arc<str>>, ProviderError> {
        if key == self.entry.key() {
            Ok(Some(Arc::<str>::from("")))
        } else {
            Ok(None)
        }
    }

    fn children(&self, parent: &SourceKey) -> Result<Vec<ProviderChild>, ProviderError> {
        if parent == self.entry.key() {
            Ok(vec![ProviderChild::new("child", self.child.clone())])
        } else {
            Ok(Vec::new())
        }
    }
}

#[test]
fn advertised_but_missing_provider_source_is_distinct_from_unknown_aura_module() {
    let provider = MissingChildProvider {
        entry: SourceDescriptor::new(key("entry"), "main.aura"),
        child: SourceDescriptor::new(key("missing"), "child.aura"),
    };
    let error = ModuleGraphBuilder::new().build(&provider).unwrap_err();
    assert_eq!(error.diagnostic().code, codes::MODULE_SOURCE_PATH);
    assert_ne!(error.diagnostic().code, codes::UNKNOWN_MODULE);
}

#[test]
fn provider_read_failure_after_ownership_is_e4020() {
    let provider = ReadFailureProvider {
        entry: SourceDescriptor::new(key("entry"), "main.aura"),
        child: SourceDescriptor::new(key("child"), "child.aura"),
    };
    let error = ModuleGraphBuilder::new().build(&provider).unwrap_err();
    assert_eq!(error.diagnostic().code, codes::IO);
    assert!(error.diagnostic().message.contains("child.aura"));
}

#[test]
fn parent_items_precede_sorted_physical_child_wrappers() {
    let (mut provider, root) = provider("const ROOT = 1\n");
    let b = add_source(
        &mut provider,
        &root,
        "b",
        "b",
        "b.aura",
        "const VALUE = 2\n",
    );
    let _ = b;
    add_source(
        &mut provider,
        &root,
        "a",
        "a",
        "a.aura",
        "const VALUE = 3\n",
    );

    let graph = build(&provider);
    assert_eq!(graph.module().items.len(), 3);
    assert!(matches!(
        graph.module().items[0],
        aura::ast::Item::Const { .. }
    ));
    assert!(matches!(
        &graph.module().items[1],
        aura::ast::Item::Module { name, public: true, .. } if name == "a"
    ));
    assert!(matches!(
        &graph.module().items[2],
        aura::ast::Item::Module { name, public: true, .. } if name == "b"
    ));
}

/// Red-team re-verification (F4 residual): a *pure* physical directory chain
/// (one logical module per directory segment) must be bounded by the semantic
/// `MAX_AST_DEPTH`, exactly like an equivalent in-source `module` chain —
/// otherwise it bypasses the limit that in-source nesting respects. Logical
/// depth is exercised through the provider so the test is not limited by the
/// host's filesystem path length.
#[test]
fn deep_physical_wrapper_chain_is_bounded_by_the_semantic_limit() {
    use aura::parse::MAX_AST_DEPTH;

    // Build a logical chain of `depth` nested physical children.
    let build = |depth: usize| {
        let (mut provider, root) = provider("");
        let mut parent = root.clone();
        for i in 0..depth {
            let key = key(&format!("k{i}"));
            provider
                .insert_source(key.clone(), format!("m{i}.aura"), "\n")
                .unwrap();
            provider.add_child(&parent, format!("m{i}"), &key).unwrap();
            parent = key;
        }
        ModuleGraphBuilder::new().build(&provider)
    };

    // At the limit: accepted.
    assert!(
        build(MAX_AST_DEPTH).is_ok(),
        "{MAX_AST_DEPTH} nested physical modules must be accepted"
    );
    // One past: the same stable diagnostic an equivalent in-source chain gives.
    let error = build(MAX_AST_DEPTH + 1).unwrap_err();
    assert_eq!(error.diagnostic().code, codes::NESTING);
}
