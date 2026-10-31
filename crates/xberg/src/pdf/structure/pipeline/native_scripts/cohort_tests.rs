use xberg_native_pdf::geometry::Rect;
use xberg_native_pdf::layout::TextSpan;
use xberg_native_pdf::structure::McidScope;

use super::*;
use crate::pdf::native::hierarchy::{ArtifactRetention, ScriptGeometry};
use crate::pdf::structure::assembly::extract_text_and_annotations;
use crate::types::document_structure::AnnotationKind;

fn source_cohort(fraction: bool) -> Vec<TextSpan> {
    let (base, baseline, raised, glyphs) = if fraction {
        (
            "x",
            321.18585,
            325.14487,
            [
                ("1", 179.89998, 4.234514),
                ("/", 184.13397, 4.234514),
                ("3", 188.36797, 4.234514),
            ],
        )
    } else {
        (
            "n",
            117.48201,
            121.44101,
            [
                ("s", 265.321, 3.9157102),
                ("−", 269.237, 6.586491),
                ("t", 275.823, 3.0581276),
            ],
        )
    };
    let span = |text: &str, x, y, width, font_size| TextSpan {
        text: text.to_owned(),
        bbox: Rect::new(x, y, width, font_size),
        font_size,
        font_name: format!("native-font-{text}"),
        mcid_scope: Some(McidScope::Page(0)),
        ..Default::default()
    };
    let mut spans: Vec<_> = glyphs
        .into_iter()
        .map(|(text, x, width)| span(text, x, raised, width, 7.9701))
        .collect();
    let (x, width) = if fraction {
        (173.66498, 6.2345505)
    } else {
        (258.773, 6.5476418)
    };
    spans.push(span(base, x, baseline, width, 10.9091));
    spans
}

fn page(spans: Vec<TextSpan>) -> NativePageSegments {
    NativePageSegments::from_final_spans(
        spans,
        &Default::default(),
        ArtifactRetention::default(),
        ScriptGeometry::Preserve,
    )
}

fn assert_complete_power(fraction: bool, expected: &str) {
    let spans = source_cohort(fraction);
    let carrier = page(spans);
    let segments = carrier.segments().cloned().collect();
    let (segments, evidence) = prepare_segments(segments, carrier, &[]);
    let evidence = evidence.expect("native witnesses");
    assert_eq!(
        evidence.owners.len(),
        3,
        "every source member must belong to the complete run"
    );
    for (index, owner) in evidence.owners.iter().enumerate() {
        assert_eq!((owner.original_index(false), owner.original_index(true)), (3, index));
    }
    let mut before: Vec<_> = segments.iter().map(super::line_tests::fingerprint).collect();
    let paragraphs = super::super::segments_to_paragraphs_with_native_scripts(
        segments,
        &[],
        &[],
        &Default::default(),
        Some(evidence),
    );
    assert_eq!(paragraphs.len(), 1, "the entire power shares its base paragraph");
    let mut after: Vec<_> = paragraphs[0]
        .lines
        .iter()
        .flat_map(|line| &line.segments)
        .map(super::line_tests::fingerprint)
        .collect();
    before.sort();
    after.sort();
    assert_eq!(before, after, "no glyph projection is changed, dropped or duplicated");
    let (text, annotations) = extract_text_and_annotations(&paragraphs[0], &Default::default());
    assert_eq!(text, expected);
    assert_eq!(annotations.len(), 1);
    assert_eq!(annotations[0].kind, AnnotationKind::Superscript);
    assert_eq!((annotations[0].start, annotations[0].end), (1, expected.len() as u32));
    assert_eq!(&text[1..], if fraction { "1/3" } else { "s−t" });
}

#[test]
fn should_own_every_native_fractional_exponent_member_without_mutating_glyphs_2275() {
    assert_complete_power(true, "x1/3");
}

#[test]
fn should_own_every_native_subtracted_exponent_member_with_exact_utf8_range_2275() {
    assert_complete_power(false, "ns−t");
}

#[test]
fn should_drop_complete_native_run_when_any_member_is_filtered_projected_or_repaired_2275() {
    for fraction in [true, false] {
        let mut carrier = page(source_cohort(fraction));
        carrier.retain_segments(|segment| !matches!(segment.text.as_str(), "/" | "−"));
        let segments = carrier.segments().cloned().collect();
        let (_, evidence) = prepare_segments(segments, carrier, &[]);
        assert!(
            evidence.unwrap().owners.is_empty(),
            "filtering must not revive a prefix"
        );

        let carrier = page(source_cohort(fraction));
        let mut segments: Vec<_> = carrier.segments().cloned().collect();
        segments[1].assigned_role = Some(2);
        let (_, evidence) = prepare_segments(segments, carrier, &[]);
        assert!(
            evidence.unwrap().owners.is_empty(),
            "changed projection invalidates the whole run"
        );

        let carrier = page(source_cohort(fraction));
        let segments = carrier.segments().cloned().collect();
        let (mut segments, evidence) = prepare_segments(segments, carrier, &[]);
        let evidence = evidence.unwrap();
        assert_eq!(evidence.owners.len(), 3, "the control starts with a complete valid run");
        segments[1].text.push(' ');
        assert!(accepted_owners(&segments.iter().collect::<Vec<_>>(), &evidence).is_empty());
        let mut line = super::super::make_pdf_line(segments, 0.0);
        line.native_scripts = evidence.owners;
        assert!(
            line.valid_native_scripts().is_empty(),
            "assembly cannot keep surviving prefix proofs"
        );
    }
}

#[test]
fn should_not_absorb_an_ordinary_baseline_operator_into_a_native_power_2275() {
    for fraction in [true, false] {
        let mut spans = source_cohort(fraction);
        for index in [1, 2] {
            spans[index].bbox.y = spans[3].bbox.y;
        }
        let carrier = page(spans);
        let segments = carrier.segments().cloned().collect();
        let (_, evidence) = prepare_segments(segments, carrier, &[]);
        let owners = evidence.unwrap().owners;
        assert_eq!(owners.len(), 1);
        assert_eq!(
            (owners[0].original_index(false), owners[0].original_index(true)),
            (3, 0)
        );
    }
}
