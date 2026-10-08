#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! AIS/0.1 and MCP adapter property and metamorphic tests (Keystone §19-20).
//!
//! Authority: `docs/engineering/AIS.md`, `docs/engineering/MCP.md`.
//!
//! These are *properties* over generated inputs, not isolated examples:
//!
//! 1. **Bounded output** — no generated source makes the snapshot, slice, or
//!    delta exceed its documented bounds.
//! 2. **Three-level separation** — a symbol's `type_name`, `families`, and
//!    `value_kind` are always distinct concepts; no family name is ever
//!    emitted as a value kind and vice versa.
//! 3. **Revision identity** — the same bytes always produce the same revision,
//!    and a changed byte always produces a different one.
//! 4. **Metamorphic: whitespace/comment invariance** — edits that cannot
//!    change semantics (trailing whitespace, comments) leave the symbol
//!    *declarations* intact.
//! 5. **MCP ↔ AIS parity** — the adapter's tools emit exactly the same values
//!    as the AIS primitives they project; the adapter defines no semantics.
//! 6. **Malformed requests never panic** — the adapter's handler is total.
#![cfg(feature = "json")]

use proptest::prelude::*;

/// A generator for a *legal* Aura value-naming identifier.
///
/// `[a-z][a-z0-9]{1,5}` can still produce a reserved word (`fn`, `if`, `for`,
/// …) or a builtin name (reserved as a value binding since `E1009`), which
/// would make the generated source unparseable or unredeclarable and turn a
/// property into a generator bug. The filter is derived from the lexer's own
/// keyword list and the stdlib's builtin registry, so it can never drift from
/// the language.
fn legal_ident() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9]{1,5}".prop_filter("not a reserved word or builtin name", |s| {
        !aura::lex::KEYWORDS.contains(&s.as_str())
            && !aura::stdlib::builtin_names().contains(&s.as_str())
    })
}

// ---------------------------------------------------------------------------
// 1. Bounded output
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    #[test]
    fn snapshot_is_bounded_and_total(body in "\\PC{0,200}") {
        // Arbitrary text: the front end either parses it or reports a
        // diagnostic, but a snapshot always exists and is finite.
        let src = format!("fn main() {{ {body} }}");
        let doc = aura::ais::document("m.aura", &src, &aura::parse::parse(&src).unwrap_or_else(|_| {
            aura::parse::parse("fn main() {}").expect("the fallback parses")
        }));
        let json = serde_json::to_string(&doc).expect("serializes");
        prop_assert!(json.len() < 4 * 1024 * 1024);
    }

    #[test]
    fn slice_never_exceeds_its_budget(
        body in "\\PC{0,80}",
        depth in 0usize..4,
        budget in 1usize..8,
    ) {
        let src = format!(
            "struct A {{ x: int }}\nstruct B {{ y: A }}\nfn f(b: B) -> int {{ return b.y.x }}\nfn main() {{ let _ = {body:?}\n print(f(B {{ y: A {{ x: 1 }} }})) }}"
        );
        let module = match aura::parse::parse(&src) {
            Ok(m) => m,
            Err(_) => return Ok(()),
        };
        let doc = aura::ais::document("m.aura", &src, &module);
        let s = aura::ais::slice(&doc, "f", depth, budget);
        // The target is always carried; the dependencies never exceed
        // `budget - 1`; and what the budget excluded is always counted.
        prop_assert!(s.symbol.is_some());
        prop_assert!(s.dependencies.len() < budget);
        let reachable = s.dependencies.len() + s.omitted;
        prop_assert!(reachable <= 2, "f reaches only B (and A through B): {reachable}");
        if budget > 2 {
            prop_assert_eq!(s.omitted, 0);
        }
    }

    #[test]
    fn delta_between_a_source_and_itself_is_empty(body in "\\PC{0,120}") {
        let src = format!("fn main() {{ {body} }}");
        if let Ok(module) = aura::parse::parse(&src) {
            let doc = aura::ais::document("m.aura", &src, &module);
            let d = aura::ais::delta(&doc, &doc);
            prop_assert!(d.is_empty());
            prop_assert_eq!(d.from_revision, d.to_revision);
        }
    }

    // -----------------------------------------------------------------------
    // 2. Three-level separation
    // -----------------------------------------------------------------------

    #[test]
    fn families_and_value_kinds_are_disjoint_vocabularies(name in legal_ident()) {
        let src = format!("fn {name}() -> [[int]] {{ return [] }}\n");
        let doc = aura::ais::document("m.aura", &src, &aura::parse::parse(&src).unwrap());
        let sym = doc.symbols.iter().find(|s| s.name == name).unwrap();
        // The family vocabulary and the value-kind vocabulary never overlap:
        // a consumer can never read a value kind out of `families`.
        for f in &sym.families {
            prop_assert!(!matches!(f.as_str(), "list" | "map" | "struct" | "enum" | "int" | "float" | "bool" | "string" | "none"));
        }
        prop_assert_eq!(sym.value_kind.as_deref(), Some("list"));
        prop_assert_eq!(&sym.families, &vec!["sequence".to_string()]);
    }

    // -----------------------------------------------------------------------
    // 3. Revision identity
    // -----------------------------------------------------------------------

    #[test]
    fn revision_is_a_function_of_the_exact_bytes(s in "\\PC{0,200}") {
        let a = aura::ais::revision_of(&s);
        let b = aura::ais::revision_of(&s);
        prop_assert_eq!(&a, &b);
        prop_assert!(a.starts_with("rev:"));
        // A changed byte changes the revision.
        let changed = format!("{s} ");
        prop_assert_ne!(a, aura::ais::revision_of(&changed));
    }

    // -----------------------------------------------------------------------
    // 4. Metamorphic: comment and trailing-whitespace invariance
    // -----------------------------------------------------------------------

    #[test]
    fn a_comment_insertion_does_not_change_declarations(
        name in legal_ident(),
        comment in "[A-Za-z0-9 ]{0,40}",
    ) {
        // Inserting a comment line before a declaration cannot change what is
        // declared: the symbol set is identical and no family/value_kind is
        // invented. (The revision DOES change: the bytes changed.)
        let base = format!("fn {name}() -> int {{ return 1 }}\n");
        let with_comment = format!("# {comment}\n{base}");
        let a = aura::ais::document("m.aura", &base, &aura::parse::parse(&base).unwrap());
        let b = aura::ais::document(
            "m.aura",
            &with_comment,
            &aura::parse::parse(&with_comment).unwrap(),
        );
        let names =
            |d: &aura::ais::Document| -> Vec<String> { d.symbols.iter().map(|s| s.name.clone()).collect() };
        prop_assert_eq!(names(&a), names(&b));
        let kind_a = a.symbols[0].value_kind.clone();
        let kind_b = b.symbols[0].value_kind.clone();
        prop_assert_eq!(kind_a, kind_b);
        prop_assert_ne!(a.revision.clone(), b.revision.clone());
        prop_assert!(aura::ais::delta(&a, &b).is_empty());
    }

    #[test]
    fn trailing_whitespace_does_not_change_declarations(name in legal_ident()) {
        let base = format!("fn {name}() -> int {{ return 1 }}\n");
        let padded = format!("fn {name}() -> int {{ return 1 }}\n   \t\n");
        let a = aura::ais::document("m.aura", &base, &aura::parse::parse(&base).unwrap());
        let b = aura::ais::document("m.aura", &padded, &aura::parse::parse(&padded).unwrap());
        prop_assert!(aura::ais::delta(&a, &b).is_empty());
    }

    // -----------------------------------------------------------------------
    // 5. MCP <-> AIS parity
    // -----------------------------------------------------------------------

    #[test]
    fn mcp_snapshot_matches_the_ais_snapshot(name in legal_ident()) {
        let src = format!("fn {name}() -> [[int]] {{ return [] }}\nfn main() {{ print({name}()) }}\n");
        let call = serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": { "name": "aura_snapshot", "arguments": { "source": src } }
        });
        let response = aura::mcp::handle_message(&call).expect("a response");
        prop_assert_eq!(&response["result"]["isError"], &serde_json::json!(false));
        // The adapter's structured content is exactly the AIS document modulo
        // the checker diagnostics it attaches (none here).
        let doc = aura::ais::document("<mcp>", &src, &aura::parse::parse(&src).unwrap());
        let ais_json = serde_json::to_value(&doc).unwrap();
        prop_assert_eq!(&response["result"]["structuredContent"], &ais_json);
    }

    #[test]
    fn mcp_typed_decode_snapshot_transports_ais_identity_facts(name in legal_ident()) {
        // The adapter defines no type logic of its own: a typed-decode result
        // arrives through `structuredContent` with exactly the three identity
        // levels the AIS snapshot computed. The decoded nominal type is the
        // generated one, so the resolved identity tracks the source exactly.
        let ty = name.to_uppercase();
        let src = format!(
            "struct {ty} {{ id: int, name: string }}\n\
             fn load() -> {ty} {{\n\
                 return json_decode_as('{{\"id\":25,\"name\":\"pikachu\"}}', {ty})\n\
             }}\n\
             fn main() {{ print(load().name) }}\n"
        );
        let call = serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": { "name": "aura_snapshot", "arguments": { "source": src } }
        });
        let response = aura::mcp::handle_message(&call).expect("a response");
        prop_assert_eq!(&response["result"]["isError"], &serde_json::json!(false));
        let doc = aura::ais::checked_document(
            "<mcp>",
            &src,
            &aura::parse::parse(&src).expect("parses"),
        );
        let ais_json = serde_json::to_value(&doc).unwrap();
        prop_assert_eq!(&response["result"]["structuredContent"], &ais_json);
        let load = response["result"]["structuredContent"]["symbols"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["name"] == "load")
            .expect("the adapter transports the load symbol");
        prop_assert_eq!(&load["type_name"], &serde_json::json!(ty.clone()));
        prop_assert_eq!(&load["families"], &serde_json::json!(["object"]));
        prop_assert_eq!(&load["value_kind"], &serde_json::json!("struct"));
    }

    #[test]
    fn mcp_revision_matches_ais_revision(s in "\\PC{0,200}") {
        let call = serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": { "name": "aura_revision", "arguments": { "source": s } }
        });
        let response = aura::mcp::handle_message(&call).expect("a response");
        prop_assert_eq!(
            response["result"]["structuredContent"]["revision"].as_str().unwrap(),
            aura::ais::revision_of(&s)
        );
    }

    // -----------------------------------------------------------------------
    // 6. Malformed requests never panic
    // -----------------------------------------------------------------------

    #[test]
    fn the_mcp_handler_is_total_on_arbitrary_json(value in prop::collection::vec(any::<i64>(), 0..8)) {
        // Feed the handler a variety of JSON shapes, including ones no client
        // should send; it must return `Some` or `None`, never panic.
        for v in value {
            let _ = aura::mcp::handle_message(&serde_json::json!(v));
            let _ = aura::mcp::handle_message(&serde_json::json!({
                "jsonrpc": "2.0", "id": v, "method": "tools/call",
                "params": { "name": "aura_snapshot", "arguments": { "source": v } }
            }));
        }
    }
}
