#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::mem::size_of;

use aura::error::{render_with_sources, Diag, SourceDiagnostic, Span};
use aura::source::{Location, SourceId, SourceMap};
use aura::CompileMode;

#[test]
fn span_layout_and_offsets_remain_source_local() {
    assert_eq!(size_of::<Span>(), size_of::<usize>() * 2);
    assert_eq!(size_of::<SourceId>(), size_of::<u64>());
    assert_eq!(size_of::<Option<SourceId>>(), size_of::<u64>());
    assert_eq!(Span::new(3, 7), Span { start: 3, end: 7 });
}

#[test]
fn legacy_diag_struct_literal_remains_source_compatible() {
    let diag = Diag {
        code: 2003,
        message: "x".into(),
        span: Span::default(),
    };
    assert_eq!(diag.code, 2003);
}

#[test]
fn source_ids_and_locations_distinguish_identical_ranges() {
    let mut sources = SourceMap::new();
    let a = sources.add("a.aura", "x");
    let b = sources.add("b.aura", "y");
    let span = Span::new(0, 1);

    assert_ne!(a, b);
    assert_ne!(Location::new(a, span), Location::new(b, span));
    assert_eq!(sources.order(a), Some(0));
    assert_eq!(sources.order(b), Some(1));
}

#[test]
fn source_map_owns_the_correct_name_and_text() {
    let mut sources = SourceMap::new();
    let a = sources.add("<stdin>", "print(1)");
    let source = sources.get(a).expect("source must resolve in its map");

    assert_eq!(source.name(), "<stdin>");
    assert_eq!(source.text(), "print(1)");
}

#[test]
fn source_id_from_another_map_never_aliases() {
    let mut a_map = SourceMap::new();
    let a = a_map.add("a", "x");
    let mut b_map = SourceMap::new();
    let b = b_map.add("b", "y");

    assert_ne!(a, b);
    assert!(a_map.get(b).is_none());
    assert!(b_map.get(a).is_none());
}

#[test]
fn source_order_is_local_and_stable_even_when_maps_interleave() {
    let mut a_map = SourceMap::new();
    let mut b_map = SourceMap::new();

    let a0 = a_map.add("a0", "0");
    let b0 = b_map.add("b0", "0");
    let a1 = a_map.add("a1", "1");
    let b1 = b_map.add("b1", "1");

    assert_eq!(a_map.order(a0), Some(0));
    assert_eq!(a_map.order(a1), Some(1));
    assert_eq!(b_map.order(b0), Some(0));
    assert_eq!(b_map.order(b1), Some(1));
    assert!(a_map.get(b0).is_none());
    assert!(b_map.get(a0).is_none());
}

#[test]
fn identical_spans_render_against_their_own_sources() {
    let mut sources = SourceMap::new();
    let a = sources.add("a.aura", "αx\n");
    let b = sources.add("b.aura", "βy\n");
    let span = Span::new(2, 3);
    let da = SourceDiagnostic::new(Diag::new(2003, "a", span), a);
    let db = SourceDiagnostic::new(Diag::new(2003, "b", span), b);

    assert_eq!(
        render_with_sources(&sources, &da).as_deref(),
        Some("a.aura:1:2: E2003: a")
    );
    assert_eq!(
        render_with_sources(&sources, &db).as_deref(),
        Some("b.aura:1:2: E2003: b")
    );
}

#[test]
fn source_aware_rendering_is_deterministic_and_rejects_foreign_maps() {
    let mut sources = SourceMap::new();
    let id = sources.add("unit", "x\ny");
    let d = SourceDiagnostic::new(Diag::new(1006, "expected", Span::new(2, 3)), id);
    let first = render_with_sources(&sources, &d);
    let second = render_with_sources(&sources, &d);
    assert_eq!(first, second);

    let mut other = SourceMap::new();
    other.add("other", "x\ny");
    assert!(render_with_sources(&other, &d).is_none());
}

#[test]
fn locationless_diagnostic_does_not_invent_a_source() {
    let d = SourceDiagnostic::locationless(Diag::locationless(4999, "internal"));
    assert!(d.location().is_none());
}

#[test]
fn source_aware_rendering_rejects_out_of_bounds_or_non_boundary_spans() {
    let mut sources = SourceMap::new();
    let id = sources.add("unicode.aura", "α");
    let past_end = SourceDiagnostic::new(Diag::new(1006, "bad", Span::new(0, 3)), id);
    let mid_codepoint = SourceDiagnostic::new(Diag::new(1006, "bad", Span::new(1, 2)), id);

    assert!(render_with_sources(&sources, &past_end).is_none());
    assert!(render_with_sources(&sources, &mid_codepoint).is_none());
}

#[test]
fn source_boundary_preserves_parser_and_checker_offsets() {
    for (src, mode) in [
        ("fn main( {", CompileMode::Module),
        ("fn main() { nope }", CompileMode::Program),
    ] {
        let legacy = aura::compile_with_mode(src, mode).expect_err("must fail");
        let report = aura::compile_named_with_mode(src, "unit.aura", mode).expect_err("must fail");
        let sourced = report.diagnostic();

        assert_eq!(sourced.code, legacy.code);
        assert_eq!(sourced.message, legacy.message);
        assert_eq!(sourced.span, legacy.span);
        let location = report.location().expect("named source must be attached");
        assert_eq!(location.span, legacy.span);
        assert_eq!(
            report.sources().get(location.source).unwrap().name(),
            "unit.aura"
        );
    }
}

#[test]
fn two_source_frontend_diagnostics_keep_identical_spans_distinct() {
    let mut sources = SourceMap::new();
    let a = sources.add("a.aura", "@");
    let b = sources.add("b.aura", "$");

    let da = aura::compile_source_in_map(&sources, a, CompileMode::Module)
        .expect_err("invalid character must fail");
    let db = aura::compile_source_in_map(&sources, b, CompileMode::Module)
        .expect_err("invalid character must fail");

    assert_eq!(da.diagnostic().span, Span::new(0, 1));
    assert_eq!(db.diagnostic().span, Span::new(0, 1));
    assert_ne!(da.location(), db.location());
    assert!(render_with_sources(&sources, &da)
        .unwrap()
        .starts_with("a.aura:1:1:"));
    assert!(render_with_sources(&sources, &db)
        .unwrap()
        .starts_with("b.aura:1:1:"));
}

#[test]
fn runtime_diagnostic_is_lifted_to_the_entry_source() {
    let src = "fn main() { let xs = [1]\n print(xs[9]) }";
    let legacy = aura::run_program_with(src, "ignored", Vec::new(), None)
        .expect_err("index must fail at runtime");
    let report = aura::run_named_program_with(src, "runtime.aura", Vec::new(), None)
        .expect_err("index must fail at runtime");
    let sourced = report.diagnostic();

    assert_eq!(sourced.code, legacy.code);
    assert_eq!(sourced.message, legacy.message);
    assert_eq!(sourced.span, legacy.span);
    let location = report
        .location()
        .expect("runtime diagnostic must be sourced");
    assert_eq!(location.span, legacy.span);
    assert_eq!(
        report.sources().get(location.source).unwrap().name(),
        "runtime.aura"
    );
}

#[test]
fn stdin_is_just_a_display_identity_not_a_filesystem_path() {
    let report = aura::compile_named_with_mode("@", "<stdin>", CompileMode::Module)
        .expect_err("invalid character must fail");
    assert!(report.render().starts_with("<stdin>:1:1:"));

    let mut sources = SourceMap::new();
    let virtual_id = sources.add("virtual:cell", "x");
    assert_eq!(sources.get(virtual_id).unwrap().name(), "virtual:cell");
}
