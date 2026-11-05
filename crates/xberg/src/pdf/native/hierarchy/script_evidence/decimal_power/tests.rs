use super::*;
use crate::pdf::native::hierarchy::ScriptGeometry;

fn actual_source() -> (crate::pdf::native::NativeDocument, Vec<TextSpan>) {
    use sha2::{Digest, Sha256};
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../test_documents/pdf/regression_pages/ntrs-20040081236-p13.pdf");
    let bytes = std::fs::read(path).expect("mandatory actual decimal power source");
    assert_eq!(
        hex::encode(Sha256::digest(&bytes)),
        "71cd7420f93701108312f7b8bd1ba0b1c55665386e1e4051ae2a05782e639308"
    );
    let mut doc = crate::pdf::native::NativeDocument::open_bytes(&bytes).unwrap();
    let spans =
        crate::pdf::native::hierarchy::extract_final_spans_from_page(&mut doc, 0, ScriptGeometry::Preserve).unwrap();
    assert!(!spans.is_empty());
    (doc, spans)
}

fn prepare_test(spans: &mut Vec<TextSpan>, source: impl FnOnce() -> Option<Authority>) -> Vec<DecimalRun> {
    let mut original = crate::pdf::native::hierarchy::script_evidence::NativePageSegments::from_final_spans(
        spans.clone(),
        &HashMap::new(),
        crate::pdf::native::hierarchy::ArtifactRetention::default(),
    );
    prepare_with_authority(spans, &mut original.owners, source)
}

fn glyphs(spans: &[TextSpan]) -> Vec<char> {
    let mut values = spans.iter().flat_map(|span| span.text.chars()).collect::<Vec<_>>();
    values.sort_unstable();
    values
}

fn rebuild_index(authority: &mut Authority) {
    authority.by_y = (0..authority.chars.len()).collect();
    authority
        .by_y
        .sort_by(|&a, &b| authority.chars[a].origin_y.total_cmp(&authority.chars[b].origin_y));
}

#[test]
fn actual_powers_use_one_authority_read_and_preserve_every_source_glyph() {
    let (doc, mut spans) = actual_source();
    let original = glyphs(&spans);
    let mut calls = 0;
    let runs = prepare_test(&mut spans, || {
        calls += 1;
        Authority::load(&doc.doc, 0)
    });
    assert_eq!(calls, 1);
    let powers: Vec<String> = runs
        .iter()
        .map(|run| run.members.iter().map(|&i| spans[i].text.as_str()).collect())
        .collect();
    assert_eq!(powers, ["−0.15", "2.58"]);
    assert_eq!(glyphs(&spans), original);
    assert!(
        runs.iter()
            .all(|run| spans[run.base].text.ends_with("Re") || spans[run.base].text.ends_with("Re)"))
    );
}

#[test]
fn a_page_without_split_points_does_not_read_character_authority() {
    let (_, mut spans) = actual_source();
    spans.retain(|span| span.text != ".");
    let before = serde_json::to_value(&spans).unwrap();
    let runs = prepare_test(&mut spans, || panic!("unnecessary character authority read"));
    assert!(runs.is_empty());
    assert_eq!(serde_json::to_value(spans).unwrap(), before);
}

#[test]
fn an_unclaimed_source_glyph_in_the_decimal_gap_prevents_ownership() {
    let (doc, mut spans) = actual_source();
    let point = spans
        .iter()
        .find(|span| span.text == "." && span.bbox.y < 350.0 && span.bbox.x > 470.0)
        .unwrap();
    let mut authority = Authority::load(&doc.doc, 0).unwrap();
    let mut extra = authority
        .chars
        .iter()
        .find(|glyph| {
            glyph.char == '.' && close(glyph.origin_x, point.bbox.x) && close(glyph.origin_y, baseline(point))
        })
        .unwrap()
        .clone();
    extra.char = 'x';
    extra.advance_width = authority.fonts[&point.font_name]
        .as_ref()
        .unwrap()
        .get_glyph_width(u16::from(b'x'))
        * extra.font_size
        / 1000.0;
    extra.rendered_advance = extra.advance_width;
    extra.bbox.width = extra.advance_width;
    authority.chars.push(extra);
    rebuild_index(&mut authority);
    let runs = prepare_test(&mut spans, || Some(authority));
    assert_eq!(
        runs.len(),
        1,
        "unretained overprinting glyphs cannot be borrowed or hidden"
    );
    assert!(spans.iter().any(|span| span.text == "258"));
}

fn unsigned_point(spans: &[TextSpan]) -> usize {
    spans
        .iter()
        .position(|span| span.text == "." && span.bbox.y < 350.0 && span.bbox.x > 470.0)
        .unwrap()
}

#[test]
fn duplicate_source_digits_or_points_cannot_supply_a_decimal_power() {
    for duplicate in ['2', '.'] {
        let (doc, mut spans) = actual_source();
        let point = &spans[unsigned_point(&spans)];
        let mut authority = Authority::load(&doc.doc, 0).unwrap();
        let glyph = authority
            .chars
            .iter()
            .find(|glyph| glyph.char == duplicate && close(glyph.origin_y, baseline(point)) && glyph.origin_x > 470.0)
            .unwrap()
            .clone();
        authority.chars.push(glyph);
        rebuild_index(&mut authority);
        assert_eq!(prepare_test(&mut spans, || Some(authority)).len(), 1, "{duplicate}");
        assert!(spans.iter().any(|span| span.text == "258"));
    }
}

#[test]
fn a_point_that_does_not_fill_the_actual_digit_gap_is_not_borrowed() {
    let (doc, mut spans) = actual_source();
    let index = unsigned_point(&spans);
    let mut authority = Authority::load(&doc.doc, 0).unwrap();
    let point = unique_point(&authority, &spans[index]).unwrap() as *const TextChar;
    let glyph_index = authority
        .chars
        .iter()
        .position(|glyph| std::ptr::eq(glyph, point))
        .unwrap();
    spans[index].bbox.x += 0.002;
    authority.chars[glyph_index].origin_x += 0.002;
    authority.chars[glyph_index].bbox.x += 0.002;
    assert!(
        unique_point(&authority, &spans[index]).is_some(),
        "source mapping still agrees"
    );
    let digits = number_glyphs(&authority, spans.iter().find(|span| span.text == "258").unwrap()).unwrap();
    assert!(split_at_point(&spans[index], unique_point(&authority, &spans[index]).unwrap(), &digits).is_none());
    assert_eq!(prepare_test(&mut spans, || Some(authority)).len(), 1);
    assert!(spans.iter().any(|span| span.text == "258"));
}

#[test]
fn ambiguous_source_operands_are_not_resolved_to_the_nearest_copy() {
    let (doc, mut spans) = actual_source();
    let base = spans
        .iter()
        .find(|span| span.text.ends_with("Re") && span.bbox.y > 390.0)
        .unwrap();
    let mut authority = Authority::load(&doc.doc, 0).unwrap();
    let copies: Vec<_> = authority
        .row(baseline(base))
        .unwrap()
        .into_iter()
        .filter(|glyph| matches!(glyph.char, 'R' | 'e') && glyph.origin_x > 460.0 && glyph.origin_x < 474.0)
        .cloned()
        .map(|mut glyph| {
            glyph.origin_x += 20.0;
            glyph.bbox.x += 20.0;
            glyph
        })
        .collect();
    assert_eq!(copies.len(), 2);
    authority.chars.extend(copies);
    rebuild_index(&mut authority);
    assert!(terminal_edge(base, &authority).is_none());
    assert_eq!(prepare_test(&mut spans, || Some(authority)).len(), 1);
    assert!(spans.iter().any(|span| span.text == "015"));
}

#[test]
fn uncertain_font_or_source_context_preserves_the_unassociated_original() {
    for uncertain_font in [true, false] {
        let (doc, mut spans) = actual_source();
        let index = unsigned_point(&spans);
        let mut authority = Authority::load(&doc.doc, 0).unwrap();
        if uncertain_font {
            authority.fonts.insert(spans[index].font_name.clone(), None);
        } else {
            spans[index].mcid = Some(1);
        }
        let original = glyphs(&spans);
        let runs = prepare_test(&mut spans, || Some(authority));
        assert!(runs.len() <= 1);
        assert!(spans.iter().any(|span| span.text == "258"));
        assert_eq!(glyphs(&spans), original);
    }
}

#[test]
fn tagged_source_characters_cannot_certify_an_untagged_digit_span() {
    let (doc, mut spans) = actual_source();
    let mut authority = Authority::load(&doc.doc, 0).unwrap();
    let number = spans.iter().find(|span| span.text == "258").unwrap();
    let raw = number_glyphs(&authority, number).unwrap()[0] as *const TextChar;
    authority
        .chars
        .iter_mut()
        .find(|glyph| std::ptr::eq(*glyph, raw))
        .unwrap()
        .mcid = Some(42);
    assert_eq!(prepare_test(&mut spans, || Some(authority)).len(), 1);
    assert!(spans.iter().any(|span| span.text == "258"));
}

#[test]
fn existing_script_owners_retain_their_exact_source_keys_after_decimal_splits() {
    let (doc, spans) = actual_source();
    let roles = HashMap::new();
    let artifacts = crate::pdf::native::hierarchy::ArtifactRetention::default();
    let before = super::super::NativePageSegments::from_final_spans(spans.clone(), &roles, artifacts);
    assert!(
        before
            .owners
            .iter()
            .any(|owner| owner.script.text == "6" && owner.base.text.ends_with("10"))
    );
    let authority = Authority::load(&doc.doc, 0).unwrap();
    let primes = super::super::capture_literal_primes(&spans);
    let after = super::super::NativePageSegments::from_final_spans_with_primes(
        spans,
        &roles,
        artifacts,
        primes,
        Some(authority),
    );
    assert!(after.owners.len() > before.owners.len());
    for old in before.owners {
        let preserved: Vec<_> = after
            .owners
            .iter()
            .filter(|owner| {
                SegmentKey::from(owner.base.as_ref()) == SegmentKey::from(old.base.as_ref())
                    && SegmentKey::from(owner.script.as_ref()) == SegmentKey::from(old.script.as_ref())
                    && owner.run == old.run
            })
            .collect();
        assert_eq!(preserved.len(), 1, "preserve each legacy owner without re-inferring it");
        assert_eq!(
            serde_json::to_value(&preserved[0].base_source).unwrap(),
            serde_json::to_value(&old.base_source).unwrap()
        );
        assert_eq!(
            serde_json::to_value(&preserved[0].script_source).unwrap(),
            serde_json::to_value(&old.script_source).unwrap()
        );
        assert!(
            after
                .segments
                .iter()
                .any(|segment| SegmentKey::from(segment) == SegmentKey::from(old.base.as_ref()))
        );
    }
}

#[test]
fn a_decimal_without_a_raised_base_does_not_read_character_authority() {
    let (_, mut spans) = actual_source();
    spans.retain(|span| span.font_size == 7.0);
    assert!(!possible_pairs(&spans).is_empty());
    let before = serde_json::to_value(&spans).unwrap();
    let runs = prepare_test(&mut spans, || panic!("ordinary decimals need no character authority"));
    assert!(runs.is_empty());
    assert_eq!(serde_json::to_value(spans).unwrap(), before);
}

#[test]
fn a_native_record_after_the_signed_tail_prevents_partial_cohort_ownership() {
    let (doc, mut spans) = actual_source();
    let mut extra = spans.iter().find(|span| span.text == "015").unwrap().clone();
    extra.text = "a".into();
    extra.bbox.x += extra.bbox.width;
    extra.bbox.width = 3.5;
    extra.char_widths = vec![3.5];
    spans.push(extra);
    assert_eq!(prepare_test(&mut spans, || Authority::load(&doc.doc, 0)).len(), 1);
    assert!(spans.iter().any(|span| span.text == "015"));
}

#[test]
fn unclaimed_source_text_after_the_operand_cannot_be_hidden_by_its_span_width() {
    let (doc, mut spans) = actual_source();
    let base = spans
        .iter()
        .find(|span| span.text.ends_with("Re") && span.bbox.y > 390.0)
        .unwrap();
    let mut authority = Authority::load(&doc.doc, 0).unwrap();
    let mut glyph = authority
        .row(baseline(base))
        .unwrap()
        .into_iter()
        .find(|glyph| glyph.char == 'e' && glyph.origin_x > 460.0 && glyph.origin_x < 474.0)
        .unwrap()
        .clone();
    glyph.char = 'x';
    glyph.origin_x += 10.0;
    glyph.bbox.x += 10.0;
    glyph.advance_width = authority.fonts[&base.font_name]
        .as_ref()
        .unwrap()
        .get_glyph_width(u16::from(b'x'))
        * glyph.font_size
        / 1000.0;
    authority.chars.push(glyph);
    rebuild_index(&mut authority);
    assert_eq!(prepare_test(&mut spans, || Some(authority)).len(), 1);
    assert!(spans.iter().any(|span| span.text == "015"));
}

#[test]
fn an_unclaimed_source_glyph_left_of_the_unsigned_digits_prevents_ownership() {
    let (doc, mut spans) = actual_source();
    let number = spans.iter().find(|span| span.text == "258").unwrap();
    let mut authority = Authority::load(&doc.doc, 0).unwrap();
    let mut glyph = number_glyphs(&authority, number).unwrap()[0].clone();
    glyph.origin_x -= 1.0;
    glyph.bbox.x -= 1.0;
    authority.chars.push(glyph);
    rebuild_index(&mut authority);
    assert_eq!(
        number_glyphs(&authority, number).unwrap().len(),
        3,
        "digit mapping remains complete"
    );
    assert_eq!(prepare_test(&mut spans, || Some(authority)).len(), 1);
    assert!(spans.iter().any(|span| span.text == "258"));
}

#[test]
fn an_over_budget_candidate_page_does_not_read_character_authority() {
    let (_, original) = actual_source();
    let number = original.iter().find(|span| span.text == "258").unwrap();
    let point = &original[unsigned_point(&original)];
    let mut spans = Vec::new();
    for index in 0..=super::super::MAX_CANDIDATE_EDGES {
        let shift = index as f32 * 1000.0;
        for source in [number, point] {
            let mut copy = source.clone();
            copy.bbox.x += shift;
            spans.push(copy);
        }
    }
    assert_eq!(possible_pairs(&spans).len(), super::super::MAX_CANDIDATE_EDGES + 1);
    let before = serde_json::to_value(&spans).unwrap();
    assert!(prepare_test(&mut spans, || panic!("over-budget source authority read")).is_empty());
    assert_eq!(serde_json::to_value(spans).unwrap(), before);
}
