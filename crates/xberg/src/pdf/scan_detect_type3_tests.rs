use super::*;

// GH#1782: unmapped Type 3 procedure codes are not Unicode. Extraction
// retains one '?' per code so the fabricated gate sees painted glyphs
// instead of silently dropping control-valued codes from its ratio. ~keep

fn type3_fixture(name: &str) -> PdfDocument {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/pdf/regressions/type3")
        .join(name);
    PdfDocument::open(&path).unwrap_or_else(|e| panic!("open fixture {name}: {e}"))
}

/// The achieved half of the fix: every span the Type 3 font paints is
/// now `MappingProvenance::Fallback`, not `EncodingName`. Before this
/// session's `best_mapping_provenance` fix, EVERY span from this font
/// (readable or not) read `EncodingName`, telling every consumer the
/// text was safely mappable when it was not.
#[test]
fn gh1782_type3_spans_carry_fallback_provenance() {
    let doc = type3_fixture("gh1782-2-readable-lines.pdf");
    let page_text = doc
        .extract_page_text_with_options(0, ReadingOrder::ColumnAware)
        .expect("extract GH#1782 fixture page text");

    let type3_spans: Vec<_> = page_text.spans.iter().filter(|s| s.font_name != "Helvetica").collect();
    assert!(
        !type3_spans.is_empty(),
        "fixture must contain at least one Type 3 span to test its provenance"
    );
    for span in &type3_spans {
        assert_eq!(
            span.provenance,
            Some(MappingProvenance::Fallback),
            "Type 3 span {:?} must carry Fallback provenance (no ToUnicode, procedural \
             /Differences glyph names) — got {:?}",
            span.text,
            span.provenance
        );
    }

    // The 2 readable Helvetica header lines are unaffected: still EncodingName.
    let helvetica_spans: Vec<_> = page_text.spans.iter().filter(|s| s.font_name == "Helvetica").collect();
    assert_eq!(
        helvetica_spans.len(),
        2,
        "fixture declares exactly 2 readable header lines"
    );
    for span in &helvetica_spans {
        assert_eq!(span.provenance, Some(MappingProvenance::EncodingName));
    }
}

/// At the default 0.5 threshold, two readable lines leave 130 painted
/// fallback glyphs against 126 readable non-whitespace chars; six lines
/// leave the same 130 fallback glyphs against 378 readable chars. ~keep
#[test]
fn gh1782_routes_only_when_unmapped_type3_glyphs_dominate() {
    let thresholds = crate::core::config::OcrQualityThresholds::default();
    for (name, expected, expected_counts) in [
        ("gh1782-2-readable-lines.pdf", vec![0], (130, 256)),
        ("gh1782-6-readable-lines.pdf", vec![], (130, 508)),
    ] {
        let doc = type3_fixture(name);
        let page = doc
            .extract_page_text_with_options(0, ReadingOrder::ColumnAware)
            .expect("extract fixture");
        assert_eq!(
            fabricated_char_counts(&page.spans),
            expected_counts,
            "{name}: absolute glyph counts"
        );
        let fabricated = fabricated_provenance_page_indices(
            &doc,
            thresholds.min_provenance_fallback_ratio,
            thresholds.min_total_non_whitespace,
        );
        assert_eq!(fabricated, expected, "{name}: route only when fallback glyphs dominate");
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/pdf/regressions/type3")
            .join(name);
        let bytes = std::fs::read(path).expect("read synthetic fixture");
        let mut native = crate::pdf::native::NativeDocument::open_bytes(&bytes).expect("open native fixture");
        let (_, _, _, metadata) = crate::pdf::native::text::extract_text_and_metadata(&mut native, None)
            .expect("extract native fixture with metadata");
        let expected_one_indexed = expected.iter().map(|index| *index as u32 + 1).collect::<Vec<_>>();
        assert_eq!(metadata.pdf_specific.fabricated_text_pages, Some(expected_one_indexed));
    }
}

#[test]
fn gh1782_unmapped_type3_body_with_one_readable_header_routes_to_ocr() {
    const BLANK_GLYPH_CODE: u8 = 2;
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/pdf/regressions/type3/gh1782-2-readable-lines.pdf");
    let mut source = lopdf::Document::load(path).expect("load synthetic Type 3 fixture");
    let page_id = *source.get_pages().get(&1).expect("fixture has one page");
    let operators = lopdf::content::Content::decode(&source.get_page_content(page_id))
        .expect("decode synthetic page operators")
        .operations;
    let body_strings: Vec<&Vec<u8>> = operators
        .iter()
        .filter(|operation| operation.operator == "Tj")
        .skip(2)
        .filter_map(|operation| match operation.operands.first() {
            Some(lopdf::Object::String(bytes, _)) => Some(bytes),
            _ => None,
        })
        .collect();
    assert_eq!(
        body_strings.len(),
        4,
        "four Type 3 text-show operators, not duplicated spans"
    );
    assert_eq!(body_strings.iter().map(|bytes| bytes.len()).sum::<usize>(), 144);
    assert_eq!(
        body_strings
            .iter()
            .flat_map(|bytes| bytes.iter())
            .filter(|&&code| code == BLANK_GLYPH_CODE)
            .count(),
        14,
        "the d1-only g02 spacing procedure must not count as painted text",
    );
    assert_eq!(
        body_strings
            .iter()
            .flat_map(|bytes| bytes.iter())
            .filter(|&&code| code > b' ')
            .count(),
        31,
        "only printable procedure codes survived the old raw-code fallback",
    );
    let content = String::from_utf8(source.get_page_content(page_id)).expect("synthetic content is ASCII");
    let second_header = content
        .match_indices("BT /F1")
        .nth(1)
        .expect("fixture has two headers")
        .0;
    let body_start = content.find("BT\n/T3_0").expect("fixture has Type 3 body");
    let mut one_header = content;
    one_header.replace_range(second_header..body_start, "");
    source
        .change_page_content(page_id, one_header.into_bytes())
        .expect("replace page content");
    let mut bytes = Vec::new();
    source.save_to(&mut bytes).expect("serialize synthetic fixture");
    let doc = PdfDocument::from_bytes(bytes).expect("open one-header fixture");

    let page = doc
        .extract_page_text_with_options(0, ReadingOrder::ColumnAware)
        .expect("extract one-header page");
    let readable = page.spans.iter().filter(|span| span.font_name == "Helvetica").count();
    assert_eq!(readable, 1, "the readable header must remain present");
    let readable_text = page
        .spans
        .iter()
        .filter(|span| span.font_name == "Helvetica")
        .map(|span| span.text.as_str())
        .collect::<String>();
    assert_eq!(
        readable_text, "Revenue summary, all funds, adopted budget, line 1 of the readable header.",
        "routing must preserve the complete readable header before OCR",
    );
    assert_eq!(readable_text.split_whitespace().count(), 12);

    let thresholds = crate::core::config::OcrQualityThresholds::default();
    assert_eq!(
        fabricated_provenance_page_indices(
            &doc,
            thresholds.min_provenance_fallback_ratio,
            thresholds.min_total_non_whitespace,
        ),
        vec![0],
        "the Type 3 body dominates one readable line and needs OCR",
    );
}

/// Negative control (task requirement): a page whose text is entirely
/// ordinary, mappable Type 1 text — no Type 3 font at all — must NOT be
/// flagged fabricated. This is the regression the GH#1782 fix could
/// easily introduce (over-flagging any page that merely mixes fonts).
#[test]
fn mappable_text_only_page_is_not_flagged_fabricated() {
    let doc = type3_fixture("control-decorative-background.pdf");
    let thresholds = crate::core::config::OcrQualityThresholds::default();
    let fabricated = fabricated_provenance_page_indices(
        &doc,
        thresholds.min_provenance_fallback_ratio,
        thresholds.min_total_non_whitespace,
    );
    assert_eq!(
        fabricated,
        Vec::<usize>::new(),
        "a page of plain Type 1 text with no Type 3 font must never be flagged \
         fabricated — false positives here would route ordinary native-text \
         documents to OCR needlessly"
    );
}
