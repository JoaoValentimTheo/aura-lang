#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use aura::module_graph::{InMemorySourceProvider, SourceKey};
use aura::{Compilation, CompileMode, SharedBuf};
use aura_playground_runtime as rt;
use serde_json::{json, Value};

static NEXT_TEMP: AtomicU64 = AtomicU64::new(1);

struct TempProject(PathBuf);

impl TempProject {
    fn new() -> Self {
        let id = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("aura-vfs-parity-{}-{id}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        Self(root)
    }

    fn write(&self, relative: &str, text: &str) {
        let path = self.0.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, text).unwrap();
    }

    fn entry(&self) -> PathBuf {
        self.0.join("main.aura")
    }
}

impl Drop for TempProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn capture(compilation: Compilation) -> String {
    let out = Arc::new(Mutex::new(Vec::new()));
    compilation
        .execute_with(Some(Box::new(SharedBuf(out.clone()))), Vec::new(), None)
        .unwrap();
    let bytes = out.lock().unwrap().clone();
    String::from_utf8(bytes).unwrap()
}

fn memory_project(root: &str, children: &[(&str, &str, &str)]) -> InMemorySourceProvider {
    let root_key = SourceKey::new("root");
    let mut provider = InMemorySourceProvider::new(root_key.clone());
    provider.insert_source(root_key.clone(), "main.aura", root).unwrap();
    for (logical, key, text) in children {
        let child_key = SourceKey::new(*key);
        provider.insert_source(child_key.clone(), format!("{logical}.aura"), *text).unwrap();
        provider.add_child(&root_key, *logical, &child_key).unwrap();
    }
    provider
}

fn virtual_project(root: &str, children: &[(&str, &str, &str)]) -> Vec<u8> {
    let mut sources = vec![json!({
        "key": "root",
        "name": "main.aura",
        "text": root,
        "children": children.iter().map(|(logical, key, _)| json!({"name": logical, "key": key})).collect::<Vec<_>>(),
    })];
    for (logical, key, text) in children {
        sources.push(json!({
            "key": key,
            "name": format!("{logical}.aura"),
            "text": text,
            "children": [],
        }));
    }
    serde_json::to_vec(&json!({"entry": "root", "sources": sources})).unwrap()
}

#[test]
fn bounded_native_inmemory_virtual_differential() {
    let cases = [
        (
            "fn main() { print(child::value()) }",
            vec![("child", "child", "pub fn value() -> int { return 5 }")],
            "5\n",
        ),
        (
            "fn main() { print(a::value() + b::value()) }",
            vec![
                ("b", "b", "pub fn value() -> int { return 2 }"),
                ("a", "a", "pub fn value() -> int { return 1 }"),
            ],
            "3\n",
        ),
    ];

    for (root, children, expected) in cases {
        let tree = TempProject::new();
        tree.write("main.aura", root);
        for (logical, _, text) in &children {
            tree.write(&format!("{logical}.aura"), text);
        }
        let native = capture(aura::compile_file_with_mode(tree.entry(), CompileMode::Program).unwrap());
        let memory = capture(
            aura::compile_provider_with_mode(&memory_project(root, &children), CompileMode::Program).unwrap(),
        );
        let (virtual_json, virtual_status, _) = rt::execute_project_bytes(&virtual_project(root, &children), &[]);
        let virtual_result: Value = serde_json::from_str(&virtual_json).unwrap();
        assert_eq!(native, expected);
        assert_eq!(memory, native);
        assert_eq!(virtual_status, rt::status::OK);
        assert_eq!(virtual_result["stdout"], native);
    }
}

#[test]
fn child_checker_diagnostic_matches_across_all_three_providers() {
    let root = "fn main() {}";
    let children = vec![("child", "child", "pub fn bad() -> int { return \"wrong\" }")];
    let tree = TempProject::new();
    tree.write("main.aura", root);
    tree.write("child.aura", children[0].2);

    let native = aura::compile_file_with_mode(tree.entry(), CompileMode::Program).unwrap_err();
    let memory = aura::compile_provider_with_mode(&memory_project(root, &children), CompileMode::Program).unwrap_err();
    assert_eq!(native.diagnostic().code, memory.diagnostic().code);
    let native_name = native.location().and_then(|loc| native.sources().get(loc.source)).map(aura::source::Source::name);
    let memory_name = memory.location().and_then(|loc| memory.sources().get(loc.source)).map(aura::source::Source::name);
    assert_eq!(native_name, Some("child.aura"));
    assert_eq!(memory_name, native_name);

    let (virtual_json, _, _) = rt::execute_project_bytes(&virtual_project(root, &children), &[]);
    let virtual_result: Value = serde_json::from_str(&virtual_json).unwrap();
    assert_eq!(virtual_result["diagnostics"][0]["code"], u64::from(native.diagnostic().code));
    assert_eq!(virtual_result["diagnostics"][0]["source"], "child.aura");
}
