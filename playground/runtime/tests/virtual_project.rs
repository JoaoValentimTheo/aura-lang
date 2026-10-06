#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use aura::error::codes;
use aura_playground_runtime as rt;
use serde_json::{json, Value};

fn child(name: &str, key: &str) -> Value {
    json!({ "name": name, "key": key })
}

fn source(key: &str, name: &str, text: &str, children: Vec<Value>) -> Value {
    json!({
        "key": key,
        "name": name,
        "text": text,
        "children": children,
    })
}

fn project(entry: &str, sources: Vec<Value>) -> Vec<u8> {
    serde_json::to_vec(&json!({ "entry": entry, "sources": sources })).unwrap()
}

fn run(raw: &[u8]) -> Value {
    let (json, _status, _version) = rt::execute_project_bytes(raw, &[]);
    serde_json::from_str(&json).expect("valid runtime JSON")
}

fn code(result: &Value) -> u64 {
    result["diagnostics"][0]["code"].as_u64().unwrap()
}

fn source_name(result: &Value) -> Option<&str> {
    result["diagnostics"][0]["source"].as_str()
}

#[test]
fn one_virtual_source_runs() {
    let raw = project(
        "root",
        vec![source(
            "root",
            "main.aura",
            "fn main() { print(7) }",
            vec![],
        )],
    );
    let result = run(&raw);
    assert_eq!(result["status"], "ok");
    assert_eq!(result["stdout"], "7\n");
}

#[test]
fn nested_virtual_children_use_the_existing_module_graph() {
    let raw = project(
        "root",
        vec![
            source(
                "root",
                "main.aura",
                "fn main() { print(foo::bar::value()) }",
                vec![child("foo", "foo")],
            ),
            source("foo", "foo.aura", "", vec![child("bar", "bar")]),
            source(
                "bar",
                "foo/bar.aura",
                "pub fn value() -> int { return 11 }",
                vec![],
            ),
        ],
    );
    let result = run(&raw);
    assert_eq!(result["status"], "ok");
    assert_eq!(result["stdout"], "11\n");
}

#[test]
fn virtual_source_and_child_insertion_order_is_non_semantic() {
    fn make(reverse: bool) -> Vec<u8> {
        let mut root_children = vec![child("b", "b"), child("a", "a")];
        let mut sources = vec![
            source(
                "root",
                "main.aura",
                "fn main() { print(a::value() + b::value()) }",
                root_children.clone(),
            ),
            source("a", "a.aura", "pub fn value() -> int { return 1 }", vec![]),
            source("b", "b.aura", "pub fn value() -> int { return 2 }", vec![]),
        ];
        if reverse {
            root_children.reverse();
            sources[0]["children"] = Value::Array(root_children);
            sources.reverse();
        }
        project("root", sources)
    }

    let left = rt::execute_project_bytes(&make(false), &[]).0;
    let right = rt::execute_project_bytes(&make(true), &[]).0;
    assert_eq!(left, right);
}

#[test]
fn child_frontend_diagnostics_expose_source_name() {
    let cases = [
        ("@", codes::INVALID_CHAR),
        ("use missing\n", codes::UNKNOWN_MODULE),
        (
            "pub fn bad() -> int { return \"wrong\" }\n",
            codes::RETURN_MISMATCH,
        ),
        (
            "pub fn f(x: int) -> int { return 1 }\npub fn f(x: int) -> int { return 2 }\n",
            codes::REDECLARED,
        ),
    ];
    for (text, expected) in cases {
        let raw = project(
            "root",
            vec![
                source(
                    "root",
                    "main.aura",
                    "fn main() {}",
                    vec![child("child", "child")],
                ),
                source("child", "child.aura", text, vec![]),
            ],
        );
        let result = run(&raw);
        assert_eq!(code(&result), u64::from(expected), "{text}");
        assert_eq!(source_name(&result), Some("child.aura"), "{text}");
    }
}

#[test]
fn runtime_diagnostic_in_child_exposes_source_name() {
    let raw = project(
        "root",
        vec![
            source(
                "root",
                "main.aura",
                "fn main() { child::boom() }",
                vec![child("child", "child")],
            ),
            source(
                "child",
                "child.aura",
                "pub fn boom() { assert(false) }",
                vec![],
            ),
        ],
    );
    let result = run(&raw);
    assert_eq!(code(&result), u64::from(codes::ASSERT));
    assert_eq!(source_name(&result), Some("child.aura"));
}

#[test]
fn missing_main_is_attributed_to_virtual_entry() {
    let raw = project(
        "root",
        vec![
            source("root", "main.aura", "", vec![child("child", "child")]),
            source(
                "child",
                "child.aura",
                "pub fn helper() -> int { return 1 }",
                vec![],
            ),
        ],
    );
    let result = run(&raw);
    assert_eq!(code(&result), u64::from(codes::NO_MAIN));
    assert_eq!(source_name(&result), Some("main.aura"));
}

#[test]
fn one_source_virtual_project_matches_single_source_eval_fallback() {
    let raw = project("root", vec![source("root", "main.aura", "1 + 2", vec![])]);
    let result = run(&raw);
    assert_eq!(result["status"], "ok");
    assert_eq!(result["result"], "3");
    assert!(result["diagnostics"].as_array().unwrap().is_empty());
}

#[test]
fn equal_local_spans_in_distinct_virtual_sources_keep_distinct_names() {
    fn bad(name: &str, key: &str, logical: &str) -> Value {
        let raw = project(
            "root",
            vec![
                source(
                    "root",
                    "main.aura",
                    "fn main() {}",
                    vec![child(logical, key)],
                ),
                source(key, name, "@", vec![]),
            ],
        );
        run(&raw)
    }
    let a = bad("a.aura", "a", "a");
    let b = bad("b.aura", "b", "b");
    assert_eq!(a["diagnostics"][0]["line"], b["diagnostics"][0]["line"]);
    assert_eq!(a["diagnostics"][0]["column"], b["diagnostics"][0]["column"]);
    assert_eq!(source_name(&a), Some("a.aura"));
    assert_eq!(source_name(&b), Some("b.aura"));
}

#[test]
fn malformed_project_json_is_a_transport_error() {
    let malformed = run(br#"{"entry":"root","sources":[}"#);
    assert_eq!(code(&malformed), u64::from(codes::IO));
    assert!(source_name(&malformed).is_none());
}

#[test]
fn invalid_virtual_provider_keys_are_module_source_path_errors() {
    for key in [
        "",
        ".",
        "..",
        "/abs",
        "C:\\drive",
        "C:drive",
        "\\\\server\\share",
        "a\\b",
        "a/b",
        "a..b",
    ] {
        let raw = project(key, vec![source(key, "main.aura", "fn main() {}", vec![])]);
        let result = run(&raw);
        assert_eq!(
            code(&result),
            u64::from(codes::MODULE_SOURCE_PATH),
            "{key:?}"
        );
        assert!(source_name(&result).is_none(), "{key:?}");
    }

    let invalid_source = project(
        "root",
        vec![
            source("root", "main.aura", "fn main() {}", vec![]),
            source("a/b", "child.aura", "", vec![]),
        ],
    );
    assert_eq!(
        code(&run(&invalid_source)),
        u64::from(codes::MODULE_SOURCE_PATH)
    );

    let invalid_child = project(
        "root",
        vec![source(
            "root",
            "main.aura",
            "fn main() {}",
            vec![child("child", "a/b")],
        )],
    );
    assert_eq!(
        code(&run(&invalid_child)),
        u64::from(codes::MODULE_SOURCE_PATH)
    );
}

#[test]
fn duplicate_missing_and_unknown_virtual_keys_are_rejected() {
    let duplicate = project(
        "root",
        vec![
            source("root", "main.aura", "fn main() {}", vec![]),
            source("root", "other.aura", "fn main() {}", vec![]),
        ],
    );
    assert_eq!(code(&run(&duplicate)), u64::from(codes::IO));

    let missing_entry = project(
        "missing",
        vec![source("root", "main.aura", "fn main() {}", vec![])],
    );
    assert_eq!(
        code(&run(&missing_entry)),
        u64::from(codes::MODULE_SOURCE_PATH)
    );

    let unknown_child = project(
        "root",
        vec![source(
            "root",
            "main.aura",
            "fn main() {}",
            vec![child("child", "missing")],
        )],
    );
    assert_eq!(
        code(&run(&unknown_child)),
        u64::from(codes::MODULE_SOURCE_PATH)
    );
}

#[test]
fn duplicate_source_names_are_rejected_before_they_can_erase_public_provenance() {
    let raw = project(
        "root",
        vec![
            source(
                "root",
                "same.aura",
                "fn main() {}",
                vec![child("child", "child")],
            ),
            source("child", "same.aura", "@", vec![]),
        ],
    );
    let result = run(&raw);
    assert_eq!(code(&result), u64::from(codes::IO));
    assert!(source_name(&result).is_none());
    assert!(result["diagnostics"][0]["message"]
        .as_str()
        .unwrap()
        .contains("source name `same.aura` is registered more than once"));
}

#[test]
fn virtual_ownership_collisions_use_module_graph_diagnostics() {
    let duplicate_owner = project(
        "root",
        vec![
            source(
                "root",
                "main.aura",
                "fn main() {}",
                vec![child("child", "a"), child("child", "b")],
            ),
            source("a", "a.aura", "", vec![]),
            source("b", "b.aura", "", vec![]),
        ],
    );
    assert_eq!(
        code(&run(&duplicate_owner)),
        u64::from(codes::MODULE_SOURCE_OWNERSHIP)
    );

    let reused_source = project(
        "root",
        vec![
            source(
                "root",
                "main.aura",
                "fn main() {}",
                vec![child("a", "shared"), child("b", "shared")],
            ),
            source("shared", "shared.aura", "", vec![]),
        ],
    );
    assert_eq!(
        code(&run(&reused_source)),
        u64::from(codes::DUPLICATE_LOGICAL_SOURCE)
    );

    let in_source = project(
        "root",
        vec![
            source(
                "root",
                "main.aura",
                "module child {}\nfn main() {}",
                vec![child("child", "child")],
            ),
            source("child", "child.aura", "", vec![]),
        ],
    );
    let result = run(&in_source);
    assert_eq!(code(&result), u64::from(codes::MODULE_SOURCE_OWNERSHIP));
    assert_eq!(source_name(&result), Some("main.aura"));
}

#[test]
fn case_collision_and_wrong_case_reference_are_portable() {
    let collision = project(
        "root",
        vec![
            source(
                "root",
                "main.aura",
                "fn main() {}",
                vec![child("Foo", "upper"), child("foo", "lower")],
            ),
            source("upper", "Foo.aura", "", vec![]),
            source("lower", "foo.aura", "", vec![]),
        ],
    );
    assert_eq!(code(&run(&collision)), u64::from(codes::MODULE_SOURCE_PATH));

    let wrong_case = project(
        "root",
        vec![
            source(
                "root",
                "main.aura",
                "use foo::value\nfn main() { value() }",
                vec![child("Foo", "upper")],
            ),
            source(
                "upper",
                "Foo.aura",
                "pub fn value() -> int { return 1 }",
                vec![],
            ),
        ],
    );
    assert_eq!(code(&run(&wrong_case)), u64::from(codes::UNKNOWN_MODULE));
}

#[test]
fn semantic_cycle_runs_and_provider_ownership_cycle_terminates() {
    let semantic_cycle = project(
        "root",
        vec![
            source(
                "root",
                "main.aura",
                "fn main() { print(a::even(6)) }",
                vec![child("a", "a"), child("b", "b")],
            ),
            source(
                "a",
                "a.aura",
                "pub fn even(n: int) -> bool { if n == 0 { return true }\n return b::odd(n - 1) }",
                vec![],
            ),
            source(
                "b",
                "b.aura",
                "pub fn odd(n: int) -> bool { if n == 0 { return false }\n return a::even(n - 1) }",
                vec![],
            ),
        ],
    );
    assert_eq!(run(&semantic_cycle)["stdout"], "true\n");

    let ownership_cycle = project(
        "root",
        vec![
            source("root", "main.aura", "fn main() {}", vec![child("a", "a")]),
            source("a", "a.aura", "", vec![child("root", "root")]),
        ],
    );
    assert_eq!(
        code(&run(&ownership_cycle)),
        u64::from(codes::DUPLICATE_LOGICAL_SOURCE)
    );
}

#[test]
fn diamond_project_loads_shared_owner_once() {
    let raw = project(
        "root",
        vec![
            source(
                "root",
                "main.aura",
                "fn main() { print(a::value() + b::value()) }",
                vec![child("shared", "shared"), child("a", "a"), child("b", "b")],
            ),
            source(
                "shared",
                "shared.aura",
                "pub fn value() -> int { return 4 }",
                vec![],
            ),
            source("a", "a.aura", "pub use shared::value", vec![]),
            source("b", "b.aura", "pub use shared::value", vec![]),
        ],
    );
    assert_eq!(run(&raw)["stdout"], "8\n");
}

#[test]
fn virtual_depth_consumes_the_existing_e1015_budget() {
    let budget = aura::parse::parse_recursion_budget();
    let mut sources = Vec::with_capacity(budget + 2);
    for depth in 0..=budget + 1 {
        let key = format!("s{depth}");
        let next = format!("s{}", depth + 1);
        let children = if depth <= budget {
            vec![child("nested", &next)]
        } else {
            Vec::new()
        };
        sources.push(source(
            &key,
            &format!("{key}.aura"),
            if depth == 0 { "fn main() {}" } else { "" },
            children,
        ));
    }
    let raw = project("s0", sources);
    let result = run(&raw);
    assert_eq!(code(&result), u64::from(codes::NESTING));
}

#[test]
fn virtual_transport_limits_are_host_policy_errors() {
    let huge = "x".repeat(rt::limits::MAX_SOURCE_BYTES + 1);
    let raw = project("root", vec![source("root", "main.aura", &huge, vec![])]);
    assert_eq!(code(&run(&raw)), u64::from(codes::IO));
}

/// More sources than the transport limit is a host policy rejection.
#[test]
fn too_many_virtual_sources_are_rejected() {
    let mut sources = Vec::with_capacity(rt::limits::MAX_PROJECT_SOURCES + 1);
    for i in 0..=rt::limits::MAX_PROJECT_SOURCES {
        sources.push(source(
            &format!("s{i}"),
            &format!("s{i}.aura"),
            "",
            vec![],
        ));
    }
    let raw = project("root", sources);
    assert_eq!(code(&run(&raw)), u64::from(codes::IO));
}

/// An encoded project over the byte limit is rejected before parsing.
#[test]
fn oversized_virtual_project_bytes_are_rejected() {
    // One source whose text alone pushes the encoded request past the bound.
    let huge = "x".repeat(rt::limits::MAX_PROJECT_BYTES + 1);
    let raw = project("root", vec![source("root", "main.aura", &huge, vec![])]);
    assert_eq!(code(&run(&raw)), u64::from(codes::IO));
}

/// An entry key outside the key grammar is a module-source path error
/// (`E2022`), not a host I/O error.
#[test]
fn invalid_virtual_entry_key_is_a_module_source_error() {
    let raw = project("bad/key", vec![source("bad/key", "main.aura", "", vec![])]);
    assert_eq!(code(&run(&raw)), u64::from(codes::MODULE_SOURCE_PATH));
}

/// An over-long source display name is a host transport rejection (`E4020`);
/// the name is host metadata, not an Aura module path.
#[test]
fn oversized_virtual_source_name_is_rejected() {
    let name = "x".repeat(rt::limits::MAX_SOURCE_NAME_BYTES + 1);
    let raw = project("root", vec![source("root", &name, "fn main() {}", vec![])]);
    assert_eq!(code(&run(&raw)), u64::from(codes::IO));
}

#[test]
fn virtual_initialization_uses_parent_then_sorted_children() {
    let ordered = project(
        "root",
        vec![
            source(
                "root",
                "main.aura",
                "fn main() { print(\"main\") }",
                vec![child("b", "b"), child("a", "a")],
            ),
            source("b", "b.aura", "print(\"b\")", vec![]),
            source("a", "a.aura", "print(\"a\")", vec![]),
        ],
    );
    assert_eq!(run(&ordered)["stdout"], "a\nb\nmain\n");

    let forward_read = project(
        "root",
        vec![
            source(
                "root",
                "main.aura",
                "const ROOT = child::VALUE\nfn main() { print(ROOT) }",
                vec![child("child", "child")],
            ),
            source("child", "child.aura", "pub const VALUE = 7", vec![]),
        ],
    );
    assert_eq!(code(&run(&forward_read)), u64::from(codes::UNDEFINED));
}

#[test]
fn duplicate_json_fields_are_rejected_without_source_identity() {
    let result = run(br#"{"entry":"root","entry":"other","sources":[]}"#);
    assert_eq!(code(&result), u64::from(codes::IO));
    assert!(source_name(&result).is_none());
}
