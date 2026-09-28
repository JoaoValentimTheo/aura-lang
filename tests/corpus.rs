#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Permanent resource corpus.
//!
//! Every fixture under `tests/corpus/` is executed here and checked against an
//! explicit expected outcome. The table is **bidirectional**: a fixture on disk
//! with no entry fails, and an entry with no fixture fails, so a fixture can
//! never be silently skipped. See `tests/corpus/README.md`.
//!
//! Boundary values (`255/256/257`, `511/512/513`, `10,000,000+/-1`, display
//! depth 512) and every previously confirmed finding's minimal reproducer live
//! here, so each stays permanently regression-tested.
//!
//! TypeExpr nesting is intentionally **not** represented: Follow-up 3 /
//! AUDIT-3 remains DECISION-PENDING, so this corpus does not encode either
//! option.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use aura::run_source;

/// The expected outcome of running a fixture.
enum Expected {
    /// Runs and produces exactly this stdout.
    Ok(&'static str),
    /// Is rejected with this `E####` code (matched by code only, not prose).
    Err(&'static str),
}

/// The complete fixture inventory. Every `.aura` file under `tests/corpus`
/// must appear here, and nothing here may be missing on disk.
fn table() -> BTreeMap<&'static str, Expected> {
    let rows: Vec<(&'static str, Expected)> = vec![
        ("syntax/conf-lex-1-quote.aura", Expected::Err("E1004")),
        ("syntax/conf-lex-2-escape.aura", Expected::Err("E1003")),
        ("syntax/conf-lex-3-fspan.aura", Expected::Err("E2003")),
        ("syntax/conf-parse-1-pipe.aura", Expected::Err("E1006")),
        ("syntax/conf-parse-2-comma.aura", Expected::Ok("[3]\n")),
        ("syntax/conf-parse-3-braces.aura", Expected::Ok("{\n")),
        ("syntax/conf-parse-4-path.aura", Expected::Ok("4\n")),
        ("syntax/conf-parse-5-adjacency.aura", Expected::Ok("true\n")),
        ("syntax/conf-parse-6-return.aura", Expected::Ok("-1\n")),
        ("syntax/conf-parse-7-block.aura", Expected::Ok("1\n")),
        ("syntax/conf-parse-9-semi.aura", Expected::Ok("1\n")),
        ("aliases/parameterised.aura", Expected::Ok("2\n")),
        ("aliases/transparent_chain.aura", Expected::Ok("5\n")),
        ("ast/gen_00.aura", Expected::Ok("2\n0\n")),
        ("ast/gen_01.aura", Expected::Ok("3\n0\n1\n2\n3\n6\n45\n6\n13\n5\n9\n6\n0\n")),
        ("ast/gen_02.aura", Expected::Ok("7\n6\n2050\n2055\n0\n")),
        ("ast/gen_03.aura", Expected::Ok("1\n14\n0\n1\n2\n0\n1\n2\n3\n9\n0\n1\n2\n0\n")),
        ("ast/gen_04.aura", Expected::Ok("3\n0\n")),
        ("ast/gen_05.aura", Expected::Ok("0\n1\n2\n0\n1\n2\n1\n0\n")),
        ("ast/gen_06.aura", Expected::Ok("0\n1\n2\n0\n1\n2\n0\n")),
        ("ast/gen_07.aura", Expected::Ok("2\n1\n1\n42\n43\n5\n29\n42\n43\n5\n29\n0\n")),
        ("ast/gen_08.aura", Expected::Ok("0\n1\n2\n10\n0\n1\n2\n0\n1\n2\n0\n1\n2\n0\n1\n2\n0\n")),
        ("ast/gen_09.aura", Expected::Ok("0\n1\n2\n0\n1\n2\n0\n")),
        ("ast/gen_10.aura", Expected::Ok("0\n1\n2\n3\n0\n")),
        ("ast/gen_11.aura", Expected::Ok("3\n6\n3\n33\n12\n0\n")),
        // Curated shared-subgraph / fan-out seeds (AUDIT-4 surface).
        ("ast/gen_14.aura", Expected::Ok("7000004\n1\n0\n")),
        ("ast/gen_105.aura", Expected::Ok("2050\n2055\n0\n1\n2\n2\n0\n1\n2\n0\n1\n2\n0\n")),
        ("ast/gen_123.aura", Expected::Ok("1\n10\n2050\n2055\n5\n42\n27\n2\n11\n86\n3\n0\n")),
        ("ast/gen_161.aura", Expected::Ok("3\n0\n")),
        ("ast/gen_264.aura", Expected::Ok("2\n0\n1\n2\n2050\n2055\n2\n18\n26\n0\n")),
        ("ast/gen_265.aura", Expected::Ok("36\n45\n3\n-8\n7000004\n7999998\n11999999\n0\n")),
        ("ast/gen_398.aura", Expected::Ok("1\n7000004\n13\n2\n0\n")),
        ("ast/list_nest_200.aura", Expected::Ok("[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[1]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]\n")),
        ("ast/list_nest_252.aura", Expected::Ok("[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[1]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]\n")),
        ("ast/list_nest_253.aura", Expected::Ok("[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[1]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]\n")),
        ("ast/list_nest_254.aura", Expected::Err("E1015")),
        ("ast/list_nest_255.aura", Expected::Err("E1015")),
        ("ast/list_nest_256.aura", Expected::Err("E1015")),
        ("ast/list_nest_257.aura", Expected::Err("E1015")),
        ("call-frames/generic_509.aura", Expected::Ok("509\n")),
        ("call-frames/generic_510.aura", Expected::Ok("510\n")),
        ("call-frames/generic_511.aura", Expected::Err("E4011")),
        ("call-frames/generic_512.aura", Expected::Err("E4011")),
        ("call-frames/generic_513.aura", Expected::Err("E4011")),
        ("call-frames/mutual_509.aura", Expected::Ok("1\n")),
        ("call-frames/mutual_510.aura", Expected::Ok("0\n")),
        ("call-frames/mutual_511.aura", Expected::Err("E4011")),
        ("call-frames/mutual_512.aura", Expected::Err("E4011")),
        ("call-frames/mutual_513.aura", Expected::Err("E4011")),
        ("call-frames/simple_510.aura", Expected::Ok("510\n")),
        ("call-frames/simple_511.aura", Expected::Err("E4011")),
        ("call-frames/simple_512.aura", Expected::Err("E4011")),
        ("call-frames/simple_513.aura", Expected::Err("E4011")),
        ("cycles/dag_fanout.aura", Expected::Ok("2333422\n")),
        ("cycles/depth_511.aura", Expected::Ok("1025\n")),
        ("cycles/depth_512.aura", Expected::Ok("1025\n")),
        ("cycles/depth_513.aura", Expected::Ok("1025\n")),
        ("cycles/enum_payload.aura", Expected::Ok("2561\n")),
        ("cycles/list_map.aura", Expected::Ok("8182\n")),
        ("cycles/list_self_display.aura", Expected::Ok("1025\n")),
        ("cycles/list_self_eq.aura", Expected::Ok("true\n")),
        ("cycles/list_self_json.aura", Expected::Ok("1028\n")),
        ("cycles/list_self_twice.aura", Expected::Ok("5000001\n")),
        ("cycles/list_self_twice_json.aura", Expected::Ok("7000004\n")),
        ("cycles/map_struct.aura", Expected::Ok("15843\n")),
        ("cycles/mixed.aura", Expected::Ok("6159285\n")),
        ("cycles/nodes_511.aura", Expected::Ok("3500757\n")),
        ("cycles/nodes_512.aura", Expected::Ok("5000001\n")),
        ("cycles/nodes_513.aura", Expected::Ok("5000001\n")),
        ("cycles/struct_list.aura", Expected::Ok("2561\n")),
        ("cycles/struct_struct.aura", Expected::Ok("3841\n")),
        ("generics/alias_interaction.aura", Expected::Ok("2\n")),
        ("generics/bounds_erasure.aura", Expected::Ok("7\n")),
        ("generics/generic_enum.aura", Expected::Ok("3\n")),
        ("generics/generic_overload.aura", Expected::Ok("c\ng\n")),
        ("generics/generic_struct_field.aura", Expected::Ok("5\n")),
        ("generics/generic_union.aura", Expected::Ok("1\n0\n")),
        ("generics/generic_unknown.aura", Expected::Ok("5\n")),
        ("generics/inference_failure.aura", Expected::Ok("c\ns\n")),
        ("overload/alpha_duplicate.aura", Expected::Err("E2007")),
        ("overload/ambiguous_tie.aura", Expected::Err("E3001")),
        ("overload/concrete_beats_generic.aura", Expected::Ok("c\n")),
        ("overload/return_type_not_identity.aura", Expected::Err("E2007")),
        ("ranges/lazy_break_huge.aura", Expected::Ok("4\n")),
        ("ranges/lazy_iterate_small.aura", Expected::Ok("10\n")),
        ("ranges/len_10000000.aura", Expected::Ok("10000000\n")),
        ("ranges/len_10000001.aura", Expected::Ok("10000001\n")),
        ("ranges/len_9999999.aura", Expected::Ok("9999999\n")),
        ("traits/bound_violation.aura", Expected::Err("E3001")),
        ("traits/static_dispatch.aura", Expected::Ok("7\n")),
        ("traits/trait_over_generic_struct.aura", Expected::Ok("5\n")),
        ("unicode/bom.aura", Expected::Err("E1001")),
        ("unicode/combining.aura", Expected::Ok("2\n")),
        ("unicode/cr.aura", Expected::Ok("1\n")),
        ("unicode/crlf.aura", Expected::Ok("1\n")),
        ("unicode/emoji.aura", Expected::Ok("😀\n")),
        ("unicode/lf.aura", Expected::Ok("1\n")),
        ("unicode/multibyte_string.aura", Expected::Ok("9\n")),
        ("unions/mismatch.aura", Expected::Err("E3001")),
        ("unions/none_permits.aura", Expected::Ok("1\n2.5\n")),
        ("visibility/private_field.aura", Expected::Err("E2018")),
        ("visibility/private_function.aura", Expected::Err("E2018")),
        ("visibility/public_ok.aura", Expected::Ok("42\n")),
    ];
    rows.into_iter().collect()
}

/// Collect every `.aura` fixture on disk, relative to `tests/corpus`.
fn fixtures_on_disk() -> Vec<String> {
    fn walk(dir: &Path, out: &mut Vec<String>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|e| e == "aura") {
                let rel = path
                    .strip_prefix("tests/corpus")
                    .unwrap()
                    .to_string_lossy()
                    .into_owned();
                // Normalize to `/` so the inventory is identical on every
                // platform: Windows reports path components with `\`, while the
                // table uses `/`. Without this the inventory test would fail on
                // Windows with a false "not listed" mismatch.
                out.push(rel.replace('\\', "/"));
            }
        }
    }
    let mut out = Vec::new();
    walk(Path::new("tests/corpus"), &mut out);
    out.sort();
    out
}

/// Every fixture is executed and matches its expected outcome.
#[test]
fn corpus_fixtures_match_expectations() {
    let table = table();
    let mut failures = Vec::new();
    for (rel, expected) in &table {
        let path = PathBuf::from("tests/corpus").join(rel);
        let src = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("corpus fixture {rel} is missing: {e}"));
        let result = run_source(&src, rel);
        match (expected, result) {
            (Expected::Ok(want), Ok(got)) => {
                if got != *want {
                    failures.push(format!("{rel}: expected stdout {want:?}, got {got:?}"));
                }
            }
            (Expected::Err(code), Err(d)) => {
                let got = format!("E{:04}", d.code);
                if got != *code {
                    failures.push(format!("{rel}: expected {code}, got {got}"));
                }
            }
            (Expected::Ok(want), Err(d)) => {
                failures.push(format!(
                    "{rel}: expected accept ({want:?}), got E{:04}",
                    d.code
                ));
            }
            (Expected::Err(code), Ok(got)) => {
                failures.push(format!("{rel}: expected {code}, got accept ({got:?})"));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "corpus failures:\n{}",
        failures.join("\n")
    );
}

/// The table and the directory agree in both directions: no silent skips.
#[test]
fn corpus_inventory_is_complete_and_sound() {
    let table = table();
    let disk = fixtures_on_disk();
    let listed: Vec<String> = table.keys().map(|s| (*s).to_string()).collect();

    for f in &disk {
        assert!(
            table.contains_key(f.as_str()),
            "fixture {f} exists on disk but is not listed in the corpus table"
        );
    }
    for f in &listed {
        assert!(
            disk.contains(f),
            "corpus table lists {f} but no fixture exists on disk"
        );
    }
    assert_eq!(disk.len(), table.len(), "corpus inventory size mismatch");
    assert!(
        disk.len() >= 60,
        "expected a substantial corpus, found {}",
        disk.len()
    );
}
