use xberg_native_pdf::geometry::Rect;
use xberg_native_pdf::layout::{TextSpan, text_block::FontWeight};
use xberg_native_pdf::structure::McidScope;

use super::super::collect_suffix_script_owners;
use super::*;

fn span(text: &str, x: f32, y: f32, width: f32, font_size: f32) -> TextSpan {
    TextSpan {
        text: text.to_owned(),
        bbox: Rect::new(x, y, width, font_size),
        font_name: "source-glyph-font".to_owned(),
        font_size,
        mcid_scope: Some(McidScope::Page(0)),
        ..Default::default()
    }
}

fn fingerprint(span: &TextSpan) -> (Vec<u8>, bool, i32) {
    (
        serde_json::to_vec(span).expect("serialize source witness"),
        span.mirrored,
        span.page_rotation_applied,
    )
}

fn projection_fingerprint(segment: &SegmentData) -> (&str, [u32; 7], [bool; 3], Option<u8>) {
    (
        &segment.text,
        [
            segment.x,
            segment.y,
            segment.width,
            segment.height,
            segment.font_size,
            segment.baseline_y,
            segment.rotation_degrees,
        ]
        .map(f32::to_bits),
        [segment.is_bold, segment.is_italic, segment.is_monospace],
        segment.assigned_role,
    )
}

fn carrier(spans: &[xberg_native_pdf::layout::TextSpan]) -> NativePageSegments {
    let script_owners = collect_suffix_script_owners(spans);
    let segments = spans
        .iter()
        .enumerate()
        .map(|(original_index, span)| {
            let baseline = span.bbox.y + span.text_rise * span.font_size;
            NativeSegment {
                original_index,
                segment: SegmentData {
                    text: span.text.clone(),
                    x: span.bbox.x,
                    y: baseline,
                    width: span.bbox.width,
                    height: span.bbox.height,
                    font_size: span.font_size,
                    is_bold: span.font_weight == FontWeight::Bold,
                    is_italic: span.is_italic,
                    is_monospace: span.is_monospace,
                    baseline_y: baseline,
                    rotation_degrees: span.rotation_degrees,
                    assigned_role: None,
                },
            }
        })
        .collect();
    NativePageSegments {
        segments,
        script_owners,
    }
}

fn pair() -> Vec<xberg_native_pdf::layout::TextSpan> {
    vec![span("q", 100.0, 700.0, 10.0, 12.0), span("2 ", 110.0, 704.0, 4.0, 8.0)]
}

fn retained(page: &NativePageSegments, indices: &[usize]) -> Vec<NativeSegment> {
    indices.iter().map(|&index| page.segments[index].clone()).collect()
}

fn assert_remapped(
    owner: &SegmentScriptOwner,
    retained: &[NativeSegment],
    original: &super::super::NativeScriptOwner,
    base_index: usize,
    script_index: usize,
) {
    assert_eq!(owner.base_index, base_index);
    assert_eq!(owner.script_index, script_index);
    assert_eq!(
        retained[base_index].original_index,
        original.base_witness.original_index
    );
    assert_eq!(
        retained[script_index].original_index,
        original.script_witness.original_index
    );
    assert_eq!(owner.source.logical_baseline, original.logical_baseline);
    assert_eq!(owner.source.kind, original.kind);
    assert_eq!(owner.source.script_byte_range, 0..1);
    assert_eq!(
        fingerprint(&owner.source.base_witness.snapshot),
        fingerprint(&original.base_witness.snapshot)
    );
    assert_eq!(
        fingerprint(&owner.source.script_witness.snapshot),
        fingerprint(&original.script_witness.snapshot)
    );
    assert_eq!(retained[script_index].segment.text, "2 ");
    assert_eq!(retained[script_index].segment.baseline_y, 704.0);
    assert_eq!(owner.source.logical_baseline, 700.0);
}

#[test]
fn should_carry_both_exact_endpoints_without_flattening_glyph_geometry_2275() {
    let page = carrier(&pair());
    assert_eq!(page.script_owners.len(), 1, "validated owner producer must fire");
    let survivors = retained(&page, &[0, 1]);
    let owners = page.remap_script_owners(&survivors);
    assert_eq!(owners.len(), 1);
    assert_remapped(&owners[0], &survivors, &page.script_owners[0], 0, 1);
}

#[test]
fn should_remap_reordered_endpoints_after_unrelated_table_segments_are_removed_2275() {
    let mut spans = pair();
    spans.insert(0, span("table cell", 300.0, 500.0, 50.0, 12.0));
    let page = carrier(&spans);
    assert_eq!(page.script_owners.len(), 1);
    assert_eq!(page.script_owners[0].base_witness.original_index, 1);
    let survivors = retained(&page, &[2, 1]);
    let owners = page.remap_script_owners(&survivors);
    assert_eq!(owners.len(), 1);
    assert_remapped(&owners[0], &survivors, &page.script_owners[0], 1, 0);
}

#[test]
fn should_distinguish_repeated_tokens_and_reused_stream_sequences_by_original_ordinal_2275() {
    let mut spans = pair();
    let mut second = pair();
    for span in &mut second {
        span.bbox.x += 200.0;
    }
    spans.extend(second);
    assert!(spans.iter().all(|span| span.sequence == 0));
    let page = carrier(&spans);
    assert_eq!(page.script_owners.len(), 2);
    let survivors = retained(&page, &[3, 0, 2, 1]);
    let owners = page.remap_script_owners(&survivors);
    assert_eq!(owners.len(), 2);
    for (source_index, base_index, script_index) in [(0, 1, 3), (1, 2, 0)] {
        let source = &page.script_owners[source_index];
        let owner = owners
            .iter()
            .find(|owner| owner.source.script_witness.original_index == source.script_witness.original_index)
            .expect("each distinct source edge survives");
        assert_remapped(owner, &survivors, source, base_index, script_index);
    }
}

#[test]
fn should_drop_edges_when_either_or_both_table_owned_endpoints_are_removed_2275() {
    let page = carrier(&pair());
    assert_eq!(page.script_owners.len(), 1);
    for indices in [&[0][..], &[1][..], &[][..]] {
        assert!(page.remap_script_owners(&retained(&page, indices)).is_empty());
    }
}

#[test]
fn should_not_retarget_a_removed_endpoint_to_same_token_at_different_geometry_2275() {
    let mut spans = pair();
    spans.push(span("q", 300.0, 700.0, 10.0, 12.0));
    spans.push(span("2 ", 310.0, 704.0, 4.0, 8.0));
    let mut page = carrier(&spans);
    assert_eq!(page.script_owners.len(), 2);
    page.script_owners.truncate(1);
    assert!(page.remap_script_owners(&retained(&page, &[2, 1])).is_empty());
    assert!(page.remap_script_owners(&retained(&page, &[0, 3])).is_empty());
}

#[test]
fn should_keep_original_projection_collisions_ambiguous_after_one_survivor_is_removed_2275() {
    for duplicate_index in [0, 1] {
        let mut spans = pair();
        let validated = carrier(&spans);
        assert_eq!(
            validated.script_owners.len(),
            1,
            "unambiguous native owner producer fires"
        );
        assert_eq!(validated.remap_script_owners(&validated.segments).len(), 1);
        let mut duplicate = spans[duplicate_index].clone();
        duplicate.mcid_scope = Some(McidScope::Page(1));
        spans.push(duplicate);
        let mut page = carrier(&spans);
        // Preserve the real edge while testing transport independently of conservative cohort admission. ~keep
        page.script_owners = validated.script_owners;
        assert_eq!(page.script_owners.len(), 1);
        assert_eq!(page.segments.len(), 3);
        assert_eq!(
            page.segments
                .iter()
                .map(|record| record.original_index)
                .collect::<Vec<_>>(),
            [0, 1, 2]
        );
        assert_ne!(spans[duplicate_index].mcid_scope, spans[2].mcid_scope);
        assert_eq!(
            projection_fingerprint(&page.segments[duplicate_index].segment),
            projection_fingerprint(&page.segments[2].segment)
        );
        let originals = page.unique_originals();
        assert_eq!(originals.len(), 3);
        assert!(originals.get(&duplicate_index).is_some_and(Option::is_none));
        assert!(originals.get(&2).is_some_and(Option::is_none));
        assert!(originals.get(&(1 - duplicate_index)).is_some_and(Option::is_some));
        assert!(page.remap_script_owners(&page.segments).is_empty());
        assert!(page.remap_script_owners(&retained(&page, &[0, 1])).is_empty());
        let wrong_survivors = if duplicate_index == 0 { [2, 1] } else { [0, 2] };
        assert!(page.remap_script_owners(&retained(&page, &wrong_survivors)).is_empty());
    }
}

#[test]
fn should_omit_edges_when_a_surviving_original_ordinal_is_duplicated_2275() {
    let page = carrier(&pair());
    assert_eq!(page.script_owners.len(), 1);
    for indices in [&[0, 1, 1][..], &[0, 0, 1][..]] {
        assert!(page.remap_script_owners(&retained(&page, indices)).is_empty());
    }
}

#[test]
fn should_reject_a_changed_projection_even_when_its_original_ordinal_is_reused_2275() {
    let page = carrier(&pair());
    assert_eq!(page.script_owners.len(), 1);
    for field in 0..4 {
        let mut survivors = retained(&page, &[0, 1]);
        match field {
            0 => survivors[1].segment.baseline_y = 700.0,
            1 => survivors[0].segment.is_italic = true,
            2 => survivors[1].segment.text = "2".to_owned(),
            3 => survivors[0].segment.assigned_role = Some(1),
            _ => unreachable!(),
        }
        assert!(page.remap_script_owners(&survivors).is_empty());
    }
}
