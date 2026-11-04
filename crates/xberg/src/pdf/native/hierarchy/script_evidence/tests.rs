use super::*;
use xberg_native_pdf::geometry::Rect;

fn span(text: &str, x: f32, y: f32, width: f32, size: f32) -> TextSpan {
    TextSpan {
        text: text.to_owned(),
        bbox: Rect::new(x, y, width, size),
        font_size: size,
        ..Default::default()
    }
}

fn page(spans: Vec<TextSpan>) -> NativePageSegments {
    NativePageSegments::from_final_spans(spans, &HashMap::new(), ArtifactRetention::default())
}

#[test]
fn should_leave_measured_author_overlap_unsupported_regardless_of_final_offsets() {
    let mut base = span(
        "Author",
        94.29801177978516,
        697.1630249023438,
        91.20623016357422,
        11.9552001953125,
    );
    let script = span(
        "1",
        185.50401306152344,
        701.5010375976562,
        4.234514236450195,
        7.970099925994873,
    );
    assert!(script.bbox.x < base.bbox.right());
    assert_eq!(page(vec![base.clone(), script.clone()]).owners.len(), 0);
    base.char_widths = vec![base.bbox.width / 6.0; 6];
    assert_eq!(page(vec![base.clone(), script.clone()]).owners.len(), 0);
    base.char_x_offsets = vec![94.29801, 110.0, 125.0, 140.0, 155.0, 170.0];
    let captured = page(vec![base, script]);
    assert_eq!(
        captured.owners.len(),
        0,
        "final offsets do not certify their native provenance"
    );
    assert_eq!(captured.segments[1].baseline_y.to_bits(), 1143955473);
}

#[test]
fn should_leave_measured_ordinal_overlap_unsupported_regardless_of_final_offsets() {
    let mut base = span(
        "1",
        117.74600219726562,
        661.489990234375,
        5.479450225830078,
        10.958900451660156,
    );
    let suffix = span(
        "st",
        123.22500610351562,
        665.448974609375,
        5.116690635681152,
        7.671199798583984,
    );
    assert_eq!(page(vec![base.clone(), suffix.clone()]).owners.len(), 0);
    base.char_x_offsets = vec![base.bbox.x];
    assert_eq!(page(vec![base.clone(), suffix.clone()]).owners.len(), 0);
    base.text = "1 conference".to_owned();
    base.bbox.width = 80.0;
    base.char_x_offsets = (0..12).map(|i| base.bbox.x + i as f32 * 5.0).collect();
    assert_eq!(
        page(vec![base, suffix]).owners.len(),
        0,
        "interior insertion is unsupported"
    );
}

#[test]
fn should_leave_measured_affiliation_prefix_unowned() {
    let marker = span(
        "1",
        89.53799438476562,
        674.801025390625,
        4.234514236450195,
        7.970099925994873,
    );
    let base = span(
        "University",
        94.2699966430664,
        670.4630126953125,
        86.85453033447266,
        11.9552001953125,
    );
    let captured = page(vec![marker, base]);
    assert_eq!(captured.owners.len(), 0);
    assert_eq!(
        captured.segments.iter().map(|s| s.text.as_str()).collect::<Vec<_>>(),
        ["1", "University"]
    );
}

#[test]
fn should_reject_ambiguous_bases_and_unrelated_small_raised_text() {
    let base = span("Author", 100.0, 700.0, 40.0, 12.0);
    let suffix = span("1", 140.0, 704.5, 4.0, 8.0);
    assert_eq!(page(vec![base.clone(), base.clone(), suffix.clone()]).owners.len(), 0);
    let mut other_column = suffix.clone();
    other_column.bbox.x = 350.0;
    assert_eq!(page(vec![base.clone(), other_column]).owners.len(), 0);
    let mut unrelated = suffix.clone();
    unrelated.bbox.y = 720.0;
    assert_eq!(page(vec![base.clone(), unrelated]).owners.len(), 0);
    let mut rotated = suffix.clone();
    rotated.rotation_degrees = 90.0;
    assert_eq!(page(vec![base.clone(), rotated]).owners.len(), 0);
    let ordinary_subscript = span("2", 140.0, 697.0, 4.0, 8.0);
    assert_eq!(page(vec![base, ordinary_subscript]).owners.len(), 0);
}

#[test]
fn should_reject_changed_or_removed_endpoints_and_incomplete_runs() {
    let captured = page(vec![
        span("Author", 100.0, 700.0, 40.0, 12.0),
        span("1", 140.0, 704.5, 4.0, 8.0),
        span(",", 144.0, 704.5, 2.0, 8.0),
        span("2", 146.0, 704.5, 4.0, 8.0),
    ]);
    assert_eq!(captured.owners.len(), 3);
    assert_eq!(captured.valid_owners(&captured.segments).len(), 3);
    assert_eq!(captured.valid_owners(&captured.segments[..3]).len(), 0);
    let mut changed = captured.segments.clone();
    changed[0].text.push('!');
    assert_eq!(captured.valid_owners(&changed).len(), 0);
    let mut scoped = span("2", 140.0, 704.5, 4.0, 8.0);
    scoped.mcid_scope = Some(xberg_native_pdf::structure::McidScope::Page(1));
    assert_eq!(
        page(vec![span("Author", 100.0, 700.0, 40.0, 12.0), scoped])
            .owners
            .len(),
        0
    );
}

#[test]
fn should_reject_numeric_note_boundary_and_changed_scope_role_or_nonfinite_geometry() {
    let base = span("Parameter 1", 100.0, 700.0, 40.0, 12.0);
    let suffix = span("2", 140.0, 704.5, 4.0, 8.0);
    let note = span("2", 100.0, 300.0, 4.0, 8.0);
    let body = span("Independent footnote body", 108.0, 300.0, 100.0, 8.0);
    assert_eq!(page(vec![base.clone(), suffix.clone(), note, body]).owners.len(), 0);
    let mut nonfinite = suffix.clone();
    nonfinite.text_rise = f32::NAN;
    assert_eq!(page(vec![base.clone(), nonfinite]).owners.len(), 0);
    let mut role = suffix.clone();
    role.heading_level = Some(2);
    assert_eq!(page(vec![base.clone(), role]).owners.len(), 0);
    let mut mirrored = suffix;
    mirrored.mirrored = true;
    assert_eq!(page(vec![base, mirrored]).owners.len(), 0);
}

#[test]
fn should_reject_overlap_when_prior_script_rejoining_synthesizes_terminal_origins() {
    let mut original = span("AlphaBeta", 100.0, 700.0, 80.0, 12.0);
    original.char_widths = vec![80.0 / 9.0; 9];
    assert_eq!(
        original.char_x_offsets.len(),
        0,
        "the source has no native glyph origins"
    );
    let transformed = super::super::rejoin_inline_scripts_inner(
        vec![
            original,
            span("2", 140.0, 703.0, 4.0, 8.0),
            span("3", 179.9999, 704.5, 4.0, 8.0),
        ],
        ScriptGeometry::Preserve,
    );
    let fragment = transformed
        .iter()
        .find(|source| source.text == "Beta")
        .expect("earlier script splits the base");
    assert_eq!(
        fragment.char_x_offsets.len(),
        4,
        "split_span supplied width-scaled origins"
    );
    assert!(fragment.char_x_offsets.last().is_some_and(|origin| *origin < 179.9999));
    assert!(
        179.9999 < fragment.bbox.right(),
        "later marker overlaps the terminal fragment"
    );
    let captured = page(transformed);
    assert_eq!(
        captured.owners.len(),
        0,
        "transformed estimates cannot prove native terminal insertion"
    );
}

#[test]
fn should_reject_a_complete_run_when_later_member_has_a_different_unique_base() {
    let captured = page(vec![
        span("A", 100.0, 700.0, 40.0, 12.0),
        span("B", 110.0, 700.0, 34.0, 12.0),
        span("1", 140.0, 704.5, 4.0, 8.0),
        span("2", 144.0, 704.5, 4.0, 8.0),
    ]);
    let a_run: Vec<_> = captured
        .owners
        .iter()
        .filter(|owner| owner.base_ordinal == 0)
        .map(|owner| owner.script_ordinal)
        .collect();
    assert_eq!(
        a_run,
        Vec::<usize>::new(),
        "A cannot own a complete run whose second member belongs uniquely to B"
    );
    assert!(
        captured
            .owners
            .iter()
            .all(|owner| owner.base_ordinal == 1 && owner.script_ordinal == 3),
        "only an independently proved B suffix may survive conservative run rejection"
    );
}

#[test]
fn should_reject_complete_run_when_later_member_has_a_different_projected_role() {
    let mut later = span("2", 144.0, 704.5, 4.0, 8.0);
    later.mcid = Some(7);
    let roles = HashMap::from([(7, Some(2))]);
    let captured = NativePageSegments::from_final_spans(
        vec![
            span("A", 100.0, 700.0, 40.0, 12.0),
            span("1", 140.0, 704.5, 4.0, 8.0),
            later,
        ],
        &roles,
        ArtifactRetention::default(),
    );
    assert_eq!(captured.segments[0].assigned_role, None);
    assert_eq!(captured.segments[1].assigned_role, None);
    assert_eq!(captured.segments[2].assigned_role, Some(2));
    assert_eq!(
        captured.owners.len(),
        0,
        "each complete-run member must match the base's projected role"
    );
}

#[test]
fn should_accept_measured_nonoverlapping_ordinal_without_claiming_native_glyph_origins() {
    let base = span(
        "4",
        193.9890594482422,
        599.6990356445312,
        4.981300354003906,
        9.962599754333496,
    );
    let suffix = span(
        "th",
        198.97105407714844,
        603.3140258789062,
        5.425617218017578,
        6.973800182342529,
    );
    assert!(suffix.bbox.x > base.bbox.right());
    assert_eq!(base.char_x_offsets.len(), 0);
    let captured = page(vec![base, suffix]);
    assert_eq!(captured.owners.len(), 1);
    assert_eq!(
        (captured.owners[0].base_ordinal, captured.owners[0].script_ordinal),
        (0, 1)
    );
    assert_eq!(captured.segments[1].baseline_y.to_bits(), 1142346777);
}

#[test]
fn should_reject_complete_run_when_later_member_has_incompatible_native_context() {
    let ordinary = span("2", 144.0, 704.5, 4.0, 8.0);
    let mut scoped = ordinary.clone();
    scoped.mcid_scope = Some(xberg_native_pdf::structure::McidScope::Page(1));
    let mut heading = ordinary.clone();
    heading.heading_level = Some(2);
    let mut rotated = ordinary.clone();
    rotated.rotation_degrees = 90.0;
    let mut mirrored = ordinary;
    mirrored.mirrored = true;
    for later in [scoped, heading, rotated, mirrored] {
        let captured = page(vec![
            span("A", 100.0, 700.0, 40.0, 12.0),
            span("1", 140.0, 704.5, 4.0, 8.0),
            later,
        ]);
        assert_eq!(
            captured.owners.len(),
            0,
            "an incompatible member must invalidate the complete run"
        );
    }
}

#[test]
fn should_reject_a_raised_fraction_numerator_beside_a_baseline_delimiter() {
    if let Some(directory) = std::env::var_os("XBERG_SCRIPT_AMBIGUITY_RECEIPTS") {
        record_actual_ambiguous_native_owner_context(&std::path::PathBuf::from(directory));
    }
    let captured = page(vec![
        span("sin(", 100.0, 700.0, 40.0, 12.0),
        span("ω", 140.0, 704.5, 4.0, 8.0),
        span("2", 140.0, 695.5, 4.0, 8.0),
        span(")", 145.0, 700.0, 3.0, 12.0),
    ]);
    assert_eq!(captured.owners.len(), 0, "a stacked numerator is not a proved exponent");
}

#[test]
fn should_reject_a_decimal_cohort_with_a_nearby_unclaimed_point() {
    let captured = page(vec![
        span("10", 100.0, 700.0, 20.0, 12.0),
        span("2", 120.0, 704.5, 0.25, 8.0),
        span(".", 120.25, 704.5000610351562, 0.25, 8.0),
        span("58", 120.5, 704.5, 0.5, 8.0),
    ]);
    assert_eq!(
        captured.owners.len(),
        0,
        "a decimal point cannot be omitted from its cohort"
    );
}

#[test]
fn should_preserve_unpaired_greek_and_numeric_exponents() {
    for (base, script) in [("x", "ω"), ("10", "9")] {
        let captured = page(vec![
            span(base, 100.0, 700.0, 20.0, 12.0),
            span(script, 120.0, 704.5, 4.0, 8.0),
            span("unrelated lower text", 200.0, 695.5, 30.0, 8.0),
        ]);
        assert_eq!(captured.owners.len(), 1, "an unpaired exponent remains supported");
        assert_eq!(captured.owners[0].base.text, base);
        assert_eq!(captured.owners[0].script.text, script);
    }
}

#[test]
fn should_keep_decimal_exponents_complete_or_leave_them_unassociated() {
    for scripts in [
        vec![span("2.58", 120.0, 704.5, 4.0, 8.0)],
        vec![
            span("2", 120.0, 704.5, 0.25, 8.0),
            span(".", 120.25, 704.5, 0.25, 8.0),
            span("58", 120.5, 704.5, 0.5, 8.0),
        ],
    ] {
        let original: String = scripts.iter().map(|script| script.text.as_str()).collect();
        let mut spans = vec![span("10", 100.0, 700.0, 20.0, 12.0)];
        spans.extend(scripts);
        let captured = page(spans);
        let associated: String = captured.owners.iter().map(|owner| owner.script.text.as_str()).collect();
        assert!(
            associated.is_empty() || associated == original,
            "accept the entire decimal or none"
        );
        assert_eq!(
            captured
                .segments
                .iter()
                .skip(1)
                .map(|s| s.text.as_str())
                .collect::<String>(),
            "2.58"
        );
    }
}

fn record_actual_ambiguous_native_owner_context(output: &std::path::Path) {
    use sha2::{Digest, Sha256};
    let root = std::env::var_os("XBERG_SCRIPT_RETENTION_FIXTURE_ROOT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents"));
    std::fs::create_dir_all(&output).expect("owner receipt directory");
    for (name, hash) in [
        (
            "arxiv-2405.00569v2-p9.pdf",
            "92339e619eb9397ffe4550f1b1342cafe7b2b70f90ba13e8b07b048d8216ae64",
        ),
        (
            "gutenberg-36670-p169.pdf",
            "70ae20eae453381732868eb65ef868bea7bbd3c36e2774401ccb52e2b1539c0e",
        ),
        (
            "gutenberg-38769-p500.pdf",
            "950d23bb5d8f1c47c6a8cca4e23d2230f883a12b4dbf5cf12793f1f9c1512a4b",
        ),
        (
            "ntrs-20040081236-p13.pdf",
            "71cd7420f93701108312f7b8bd1ba0b1c55665386e1e4051ae2a05782e639308",
        ),
    ] {
        let bytes = std::fs::read(root.join("pdf/regression_pages").join(name)).expect("mandatory ambiguity fixture");
        assert_eq!(hex::encode(Sha256::digest(&bytes)), hash);
        let mut doc = crate::pdf::native::NativeDocument::open_bytes(&bytes).expect("native source");
        let (pages, used_tree) =
            extract_all_segment_carriers(&mut doc, ArtifactRetention::default()).expect("actual native carriers");
        assert_eq!(pages.len(), 1);
        assert!(!pages[0].segments.is_empty(), "the actual final spans must be present");
        let neighbors =
            super::super::extract_final_spans_from_page(&mut doc, 0, ScriptGeometry::Preserve).expect("neighbor spans");
        let owners: Vec<_> = pages[0]
            .owners
            .iter()
            .map(|owner| {
                serde_json::json!({
                    "base_final_span_ordinal":owner.base_ordinal,
                    "script_final_span_ordinal":owner.script_ordinal,
                    "base_snapshot":owner.base_source.as_ref(),
                    "script_snapshot":owner.script_source.as_ref(),
                })
            })
            .collect();
        let record = serde_json::json!({
            "fixture":name,"sha256":hash,"used_structure_tree":used_tree,
            "capture_path":"extract_all_segment_carriers; exact final-span snapshots, not original native IDs",
            "owners":owners,"independently_reextracted_final_spans":neighbors,
            "neighbor_read_limitation":"A second final-span read supplies context; its ordinal is not asserted to be the first extraction's original native identity."
        });
        std::fs::write(
            output.join(format!("{name}.owner-context.json")),
            serde_json::to_vec_pretty(&record).expect("serialize owner context"),
        )
        .expect("write owner context");
    }
}
