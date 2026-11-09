//! Unit tests for [`super`].
//!
//! Split out of `auto.rs` purely for file size, mirroring the split used for
//! `document.rs`/`document/tests.rs` and `extractors/text/mod.rs`/`extractors/text/tests.rs`. ~keep

use super::*;

#[test]
fn extract_mode_default_is_auto() {
    // README locked decision 2. ~keep
    assert_eq!(ExtractMode::default(), ExtractMode::Auto);
    assert_eq!(AutoExtractOptions::default().mode, ExtractMode::Auto);
}

#[test]
fn presets_have_expected_shape() {
    assert_eq!(AutoExtractOptions::fast().mode, ExtractMode::TextOnly);
    assert!(!AutoExtractOptions::fast().reconstruct_image_tables);
    assert_eq!(AutoExtractOptions::balanced().mode, ExtractMode::Auto);
    assert!(AutoExtractOptions::balanced().reconstruct_image_tables);
    assert_eq!(AutoExtractOptions::high_fidelity().mode, ExtractMode::Auto);
    assert!(AutoExtractOptions::high_fidelity().min_text_confidence.is_some());
    assert_eq!(AutoExtractOptions::default(), AutoExtractOptions::balanced());
}

#[test]
fn builder_mirrors_ocrconfigbuilder_shape() {
    let o = AutoExtractOptions::builder()
        .mode(ExtractMode::ForceOcr)
        .reconstruct_image_tables(false)
        .min_text_confidence(2.0)
        .force_ocr_pages([0, 2])
        .build();
    assert_eq!(o.mode, ExtractMode::ForceOcr);
    assert!(!o.reconstruct_image_tables);
    assert_eq!(o.min_text_confidence, Some(1.0));
    assert_eq!(o.force_ocr_pages, vec![0, 2]);
}

#[test]
fn options_json_roundtrip_is_stable() {
    // The JSON wire is the C-ABI boundary (matches split-by-bookmarks). ~keep
    let o = AutoExtractOptions::high_fidelity();
    let js = serde_json::to_string(&o).expect("serialize");
    assert!(js.contains("\"mode\":\"auto\""));
    let back: AutoExtractOptions = serde_json::from_str(&js).expect("deserialize");
    assert_eq!(o, back);
    // Partial JSON fills via #[serde(default)] (forward-compat). ~keep
    let partial: AutoExtractOptions = serde_json::from_str(r#"{"mode":"force_ocr"}"#).expect("partial");
    assert_eq!(partial.mode, ExtractMode::ForceOcr);
    assert!(partial.reconstruct_image_tables); // from Default(=balanced) ~keep
}

#[test]
fn reason_and_enum_wire_tokens_are_snake_case_frozen() {
    // Frozen append-only wire tokens (PadesLevel lesson). ~keep
    assert_eq!(
        serde_json::to_string(&ReasonCode::OcrRequestedButUnavailable).unwrap(),
        "\"ocr_requested_but_unavailable\""
    );
    assert_eq!(
        serde_json::to_string(&ExtractSource::ImageTableRecovery).unwrap(),
        "\"image_table_recovery\""
    );
    assert_eq!(serde_json::to_string(&PageKind::ImageText).unwrap(), "\"image_text\"");
}

#[test]
fn quad_from_xywh_is_tl_tr_br_bl() {
    let q = Quad::from_xywh(10.0, 20.0, 30.0, 40.0);
    assert_eq!(q.points[0], [10.0, 60.0]);
    assert_eq!(q.points[2], [40.0, 20.0]);
}

fn sig() -> PageSignals {
    PageSignals {
        text_glyph_count: 0,
        text_area_ratio: 0.0,
        image_area_ratio: 0.0,
        codec: ImageCodecClass::None,
        invisible_text_ratio: 0.0,
        garbled_ratio: 0.0,
        fragmented_word_ratio: 0.0,
        consecutive_repeat_ratio: 0.0,
        vector_path_density: 0.0,
        has_reliable_structure: false,
        producer_prior: ProducerPrior::Unknown,
        page_is_empty: false,
    }
}

/// A symbol font's lost glyphs are written as their own marker. The gate counts them as garbled,
/// as it counts U+FFFD: words that are mostly lost glyphs of either kind fail the gate. ~keep
#[test]
fn quality_gate_flags_lost_glyphs_of_either_marker() {
    for marker in ['\u{FFFD}', crate::LOST_SYMBOL_GLYPH_MARKER] {
        let text = format!("Total{m}{m}{m} amount{m}{m}{m} received{m}{m}{m}{m}", m = marker);
        assert_eq!(
            text_quality_gate(&text),
            Some(ReasonCode::GlyphMappingMissing),
            "{text:?}"
        );
    }
}

#[test]
fn quality_gate_flags_cid_garbage_and_passes_clean() {
    let garbage: String = "\u{FFFD}".repeat(40);
    assert_eq!(text_quality_gate(&garbage), Some(ReasonCode::GlyphMappingMissing));
    assert_eq!(
        text_quality_gate("The quick brown fox jumps over the lazy dog repeatedly."),
        None
    );
}

#[test]
fn quality_gate_catches_column_scramble_and_fragmentation() {
    // Critical fragmentation hard-trigger (every glyph split). ~keep
    let frag = "a b c d e f g h i j k l m n o p q r s t";
    assert_eq!(text_quality_gate(frag), Some(ReasonCode::GlyphMappingMissing));
    // Consecutive-repeat / 2-column scramble. ~keep
    let scramble = "alpha alpha beta beta gamma gamma delta delta epsilon epsilon zeta zeta";
    assert_eq!(text_quality_gate(scramble), Some(ReasonCode::TextLayerBelowThreshold));
}

#[test]
fn quality_gate_does_not_flag_dense_cjk_prose_as_fragmented() {
    // Real Japanese sentence about cats (no inter-word spaces — this
    // script never has them). Naturally clusters into short 1-3
    // character "words" once split at whatever boundary a caller
    // uses; that must not read as glyph-per-span CMap breakage the
    // way it legitimately would for Latin text. ~keep
    let ja = "ネコ 猫 は 狭義 に は 食肉目 ネコ科 ネコ属 に 分類 される \
              リビア ヤマネコ が 家畜 化 された イエネコ に 対する 通称 である";
    assert_eq!(
        text_quality_gate(ja),
        None,
        "dense CJK prose must not trigger the text-quality gate"
    );
    assert!(is_cjk_dominant_text(ja));
    assert!(!is_cjk_dominant_text("The quick brown fox jumps over the lazy dog."));
}

#[test]
fn cascade_empty_scanned_sparse_over_scan_hybrid_textlayer() {
    let mut s = sig();
    s.page_is_empty = true;
    assert_eq!(
        classify_from_signals(&s, &AutoExtractOptions::balanced()).0,
        PageKind::Empty
    );

    // Pure scan (CCITT) → high-confidence Scanned. ~keep
    let mut s = sig();
    s.image_area_ratio = 0.97;
    s.codec = ImageCodecClass::Ccitt;
    let (k, c, _) = classify_from_signals(&s, &AutoExtractOptions::balanced());
    assert_eq!(k, PageKind::Scanned);
    assert!(c >= 0.95);

    // Sparse text over a scan (case G — the headline fix): a tiny
    // header must NOT classify as TextLayer. ~keep
    let mut s = sig();
    s.image_area_ratio = 0.95;
    s.text_glyph_count = 60; // a Bates/header line ~keep
    s.text_area_ratio = 0.02;
    assert_eq!(
        classify_from_signals(&s, &AutoExtractOptions::balanced()).0,
        PageKind::Scanned
    );

    // Hybrid: real text + sub-page image (cases D/S). ~keep
    let mut s = sig();
    s.text_glyph_count = 800;
    s.text_area_ratio = 0.5;
    s.image_area_ratio = 0.25;
    assert_eq!(
        classify_from_signals(&s, &AutoExtractOptions::balanced()).0,
        PageKind::ImageText
    );

    let mut s = sig();
    s.text_glyph_count = 1200;
    s.text_area_ratio = 0.6;
    s.has_reliable_structure = true;
    let (k, c, r) = classify_from_signals(&s, &AutoExtractOptions::balanced());
    assert_eq!(k, PageKind::TextLayer);
    assert_eq!(r, ReasonCode::NativeTextHighConfidence);
    assert!(c >= 0.90);
}

#[test]
fn cascade_keeps_good_ocr_sidecar_over_scan() {
    // Case C/C2: scan + usable invisible OCR text → keep the text. ~keep
    let mut s = sig();
    s.image_area_ratio = 0.96;
    s.text_glyph_count = 1500;
    s.text_area_ratio = 0.55;
    s.invisible_text_ratio = 0.95;
    assert_eq!(
        classify_from_signals(&s, &AutoExtractOptions::balanced()).0,
        PageKind::TextLayer
    );
}

#[test]
fn summary_is_aggregate_only_never_forced_mode() {
    use PageKind::*;
    assert_eq!(summarise(&[]), DocumentSummary::Empty);
    assert_eq!(summarise(&[Empty, Empty]), DocumentSummary::Empty);
    assert_eq!(
        summarise(&[TextLayer, TextLayer, TextLayer, TextLayer, Empty]),
        DocumentSummary::MostlyText
    );
    assert_eq!(
        summarise(&[Scanned, Scanned, Scanned, Scanned]),
        DocumentSummary::MostlyScanned
    );
    // Heterogeneous doc (case Q) stays Mixed — not forced. ~keep
    assert_eq!(
        summarise(&[TextLayer, Scanned, ImageText, Scanned]),
        DocumentSummary::Mixed
    );
}

#[test]
fn auto_extractor_construction_is_cheap_and_infallible() {
    assert_eq!(AutoExtractor::new().options().mode, ExtractMode::Auto);
    assert_eq!(AutoExtractor::default().options().mode, ExtractMode::Auto);
    assert_eq!(AutoExtractor::text_only().options().mode, ExtractMode::TextOnly);
    let ae = AutoExtractor::with(AutoExtractOptions::high_fidelity());
    assert!(ae.options().min_text_confidence.is_some());
}
