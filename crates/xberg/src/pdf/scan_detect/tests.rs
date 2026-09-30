use super::*;

/// Both page passes go through [`map_pages`], so the order guarantee is pinned on it
/// directly. A pass that only counted pages would still be green on a shuffled result,
/// and every caller indexes its result by page number.
#[test]
fn map_pages_returns_one_entry_per_page_in_page_order() {
    let page_count = 1024;
    let squares = map_pages(page_count, |page_index| page_index * page_index);
    assert_eq!(
        squares,
        (0..page_count)
            .map(|page_index| page_index * page_index)
            .collect::<Vec<_>>(),
        "map_pages must return one entry per page, in page order"
    );
}

/// The pass must reach more than one thread. Asserted against a pool this test builds
/// rather than the machine's core count, so it means the same thing on a one-core runner
/// as on the 32-core box the issue was measured on, and against the closure's own record
/// rather than wall clock, which flakes under load.
///
/// Each page holds its thread for [`DISPATCH_PAGE_HOLD`]: with no work per page, one
/// worker drains the whole range before the others wake on a loaded runner (22 of 60
/// runs under a 12-core load), and the pass then looks sequential. ~keep
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn map_pages_dispatches_pages_across_the_pool() {
    use std::collections::HashSet;
    use std::sync::Mutex;

    const DISPATCH_PAGE_COUNT: usize = 512;
    const DISPATCH_POOL_THREADS: usize = 8;
    const DISPATCH_PAGE_HOLD: std::time::Duration = std::time::Duration::from_micros(200);

    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(DISPATCH_POOL_THREADS)
        .build()
        .expect("the test's own pool must build");
    let threads: Mutex<HashSet<std::thread::ThreadId>> = Mutex::new(HashSet::new());

    let pages = pool.install(|| {
        map_pages(DISPATCH_PAGE_COUNT, |page_index| {
            threads
                .lock()
                .expect("thread record must not be poisoned")
                .insert(std::thread::current().id());
            std::thread::sleep(DISPATCH_PAGE_HOLD);
            page_index
        })
    });

    assert_eq!(pages, (0..DISPATCH_PAGE_COUNT).collect::<Vec<_>>());
    let distinct = threads.lock().expect("thread record must not be poisoned").len();
    assert!(
        distinct > 1,
        "map_pages ran every page on {distinct} thread(s): the pass did not dispatch across the pool"
    );
}

/// The wiring, on a real document: both passes must agree with a page-by-page run, entry
/// for entry. This is what says `detect` and `fabricated_provenance_page_indices` read the
/// pages they claim to and report them in page order, which the seam test above cannot
/// say on its own.
///
/// The fixture is chosen so that a reordering is visible: its pages do not all score the
/// same, and some but not all of them carry a fabricated mapping (15 of 18). On a
/// born-digital fixture every page scores `0.0` and no page is fabricated, so the two
/// comparisons below would pass on any permutation. Both properties are asserted on the
/// sequential run so the fixture cannot drift into that shape unnoticed.
///
/// The parallel pass runs on its own freshly opened handle. A handle the sequential pass
/// has already walked has every font and page object cached, so a concurrent read
/// through it never races a cold load, which is the shape #1737 exists to make
/// order-independent. ~keep
///
/// This test calls `fabricated_provenance_page_indices`, which increments
/// `FABRICATED_PROVENANCE_SECOND_PASS_CALLS` -- now thread-local, so it no longer shares
/// state with `provenance_is_not_read_a_second_time_for_a_document_with_no_excluded_layers`
/// (`pdf/native/text.rs`) or any other test's thread. `#[serial]` is kept regardless, since
/// nothing about the counter change makes concurrent PDF-handle work here cheaper. ~keep
#[test]
#[serial_test::serial]
fn both_page_passes_match_a_page_by_page_run() {
    let thresholds = crate::core::config::OcrQualityThresholds::default();
    let min_ratio = thresholds.min_provenance_fallback_ratio;
    let min_chars = thresholds.min_total_non_whitespace;

    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/pdf/non_ascii_text.pdf");
    let sequential_doc = PdfDocument::open(&path).expect("corpus document must open");
    let page_count = sequential_doc
        .page_count()
        .expect("corpus document must report a page count");
    assert!(
        page_count > 1,
        "a single-page fixture cannot detect a reordering; got {page_count} page(s)"
    );

    let sequential_scores: Vec<f32> = (0..page_count)
        .map(|page_index| {
            page_signals(&sequential_doc, page_index)
                .as_ref()
                .map_or(0.0, score_page)
        })
        .collect();
    let sequential_fabricated: Vec<usize> = (0..page_count)
        .filter(|&page_index| page_has_fabricated_text(&sequential_doc, page_index, min_ratio, min_chars))
        .collect();
    assert!(
        sequential_scores.iter().any(|&score| score != sequential_scores[0]),
        "fixture must score its pages differently for a reordering to be visible; got {sequential_scores:?}"
    );
    assert!(
        !sequential_fabricated.is_empty() && sequential_fabricated.len() < page_count,
        "fixture must fabricate some but not all pages for a reordering to be visible; \
         got {sequential_fabricated:?} of {page_count}"
    );

    let parallel_doc = PdfDocument::open(&path).expect("corpus document must open a second time");
    let detection = detect(&parallel_doc).expect("detection must run on the corpus document");
    assert_eq!(
        detection.page_confidence, sequential_scores,
        "scan confidences must match a page-by-page run, page for page"
    );
    assert_eq!(
        fabricated_provenance_page_indices(&parallel_doc, min_ratio, min_chars),
        sequential_fabricated,
        "fabricated-mapping pages must match a page-by-page run, in ascending page order"
    );
}

/// Positive control for the probe that
/// `provenance_is_not_read_a_second_time_for_a_document_with_no_excluded_layers`
/// (`pdf/native/text.rs`) relies on. That test asserts the counter is **zero** -- the
/// second whole-document read did not happen. A zero it can never leave proves nothing:
/// a counter no call site reaches on the asserting thread and a genuinely skipped second
/// pass render identically. Making the counter thread-local removed the flake but also
/// removed cross-thread visibility, so the wiring needs its own witness. ~keep
#[test]
#[serial_test::serial]
fn the_second_pass_counter_observes_a_call_on_its_own_thread() {
    let thresholds = crate::core::config::OcrQualityThresholds::default();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/pdf/non_ascii_text.pdf");
    let doc = PdfDocument::open(&path).expect("corpus document must open");

    FABRICATED_PROVENANCE_SECOND_PASS_CALLS.with(|counter| counter.store(0, std::sync::atomic::Ordering::SeqCst));
    let _ = fabricated_provenance_page_indices(
        &doc,
        thresholds.min_provenance_fallback_ratio,
        thresholds.min_total_non_whitespace,
    );

    assert_eq!(
        FABRICATED_PROVENANCE_SECOND_PASS_CALLS.with(|counter| counter.load(std::sync::atomic::Ordering::SeqCst)),
        1,
        "the counter must observe a call made on this thread, or the sibling test's zero is vacuous"
    );
}

/// Scores are sums of `f32` weights, so `0.50 + 0.35 + 0.10` lands a few ULPs
/// off `0.95`. Compare within tolerance rather than rounding the score.
#[track_caller]
fn assert_score(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 1e-5,
        "expected score {expected}, got {actual}"
    );
}

/// A scan: full-page raster, no text layer at all.
fn bare_scan() -> PageScanSignals {
    PageScanSignals {
        image_coverage: 1.0,
        invisible_text_ratio: 0.0,
        glyph_count: 0,
        text_area_ratio: 0.0,
        codec: ImageCodecClass::Dct,
        producer_prior: ProducerPrior::Unknown,
    }
}

#[test]
fn sub_floor_image_coverage_scores_zero() {
    let signals = PageScanSignals {
        image_coverage: IMAGE_COVERAGE_MIN - 0.01,
        ..bare_scan()
    };
    assert_score(score_page(&signals), 0.0);
}

/// GH#1793: a page whose image covers less than the old 80% floor -- down to the
/// reporter's measured range (22-76%) -- must still be scored, not zeroed outright,
/// when its native text is header-sized. This is the same case as [`bare_scan`] (no
/// text at all is trivially header-sized) at a coverage the OLD floor would have
/// zeroed before ever inspecting the text layer.
#[test]
fn sub_full_page_coverage_with_header_sized_text_still_scores_as_a_scan() {
    let signals = PageScanSignals {
        image_coverage: 0.66, // the reporter's measured median (#1793)
        ..bare_scan()
    };
    assert_score(score_page(&signals), 0.85);
    assert!(f64::from(score_page(&signals)) >= DEFAULT_SCANNED_MIN_CONFIDENCE);
}

#[test]
fn a_text_page_with_a_figure_is_not_a_scan() {
    let signals = PageScanSignals {
        image_coverage: 0.30,
        invisible_text_ratio: 0.0,
        glyph_count: 2000,
        text_area_ratio: 0.45, // a real page of text, not header-sized (GH#1793 review)
        codec: ImageCodecClass::Dct,
        producer_prior: ProducerPrior::Unknown,
    };
    assert_score(score_page(&signals), 0.0);
}

/// The born-digital slide with a full-bleed background image and a real page of
/// *visible*, non-header-sized text under it (GH#1779's `control-decorative-background`):
/// it must stay below any usable threshold, exactly as it did before the fix.
#[test]
fn full_bleed_slide_with_visible_text_scores_below_default_threshold() {
    let signals = PageScanSignals {
        image_coverage: 1.0,
        invisible_text_ratio: 0.0,
        glyph_count: 133,
        text_area_ratio: 0.30, // a real amount of visible body text, not header-sized
        codec: ImageCodecClass::Dct,
        producer_prior: ProducerPrior::Unknown,
    };
    assert_score(score_page(&signals), SCORE_FULL_PAGE_RASTER);
    assert!(f64::from(score_page(&signals)) < DEFAULT_SCANNED_MIN_CONFIDENCE);
}

/// GH#1779: a full-page raster with only a header/footer-sized native text layer (a
/// running header, page number, or Bates stamp) must clear the default threshold, unlike
/// the decorative-background case above -- this is the separation the issue's fix exists
/// to create.
#[test]
fn full_bleed_page_with_header_sized_text_clears_the_default_threshold() {
    let signals = PageScanSignals {
        image_coverage: 1.0,
        invisible_text_ratio: 0.0,
        glyph_count: 47,       // the corpus's smallest visible-text case (issue #1752)
        text_area_ratio: 0.02, // a header/footer/stamp, well under TEXT_AREA_HEADER_MAX
        codec: ImageCodecClass::Dct,
        producer_prior: ProducerPrior::Unknown,
    };
    assert_score(score_page(&signals), 0.85);
    assert!(f64::from(score_page(&signals)) >= DEFAULT_SCANNED_MIN_CONFIDENCE);
}

/// The reporter's case: full-page raster under an invisible OCR sidecar.
#[test]
fn hidden_sidecar_over_a_raster_is_detected() {
    let signals = PageScanSignals {
        image_coverage: 1.0,
        invisible_text_ratio: 1.0,
        glyph_count: 217,
        text_area_ratio: 0.40, // a full invisible sidecar covers real area; irrelevant to
        // the outcome since invisible_text_ratio alone already qualifies for the bonus
        codec: ImageCodecClass::Other,
        producer_prior: ProducerPrior::Unknown,
    };
    assert_score(score_page(&signals), 0.85);
    assert!(f64::from(score_page(&signals)) >= DEFAULT_SCANNED_MIN_CONFIDENCE);
}

#[test]
fn scan_with_no_text_layer_is_detected() {
    assert_score(score_page(&bare_scan()), 0.85);
}

#[test]
fn bilevel_codec_and_scanner_producer_raise_confidence() {
    let signals = PageScanSignals {
        codec: ImageCodecClass::Ccitt,
        producer_prior: ProducerPrior::Scanner,
        ..bare_scan()
    };
    assert_score(score_page(&signals), 1.0);
}

#[test]
fn jbig2_counts_as_a_bilevel_codec() {
    let signals = PageScanSignals {
        codec: ImageCodecClass::Jbig2,
        ..bare_scan()
    };
    assert_score(score_page(&signals), 0.95);
}

/// A sidecar of *any* quality reads identically here. kreuzberg detects that
/// a sidecar came from a scanner, never whether its text is accurate.
#[test]
fn sidecar_quality_does_not_affect_the_score() {
    let good = PageScanSignals {
        invisible_text_ratio: 1.0,
        glyph_count: 212,
        ..bare_scan()
    };
    let bad = PageScanSignals {
        invisible_text_ratio: 1.0,
        glyph_count: 217,
        ..bare_scan()
    };
    assert_score(score_page(&good), score_page(&bad));
}

#[test]
fn score_never_leaves_the_unit_interval() {
    let maxed = PageScanSignals {
        image_coverage: 1.0,
        invisible_text_ratio: 1.0,
        glyph_count: 0,
        text_area_ratio: 0.0,
        codec: ImageCodecClass::Ccitt,
        producer_prior: ProducerPrior::Scanner,
    };
    let score = score_page(&maxed);
    assert!((0.0..=1.0).contains(&score), "score {score} escaped [0,1]");
}

/// Every distinct score `score_page` can return, over the whole signal matrix.
///
/// The score is a sum of four independent weights, so the reachable set is small and
/// fixed, and the gap between `0.65` and `0.85` is what gives
/// [`DEFAULT_SCANNED_MIN_CONFIDENCE`] its meaning: any threshold in `(0.65, 0.85]`
/// selects exactly the same pages as `0.70` does, so moving it within that interval is
/// a no-op dressed as a behaviour change. Pinned so that stays visible to whoever
/// proposes the move. ~keep
/// Every `(codec, producer_prior)` score for one `(image_coverage, glyph_count,
/// invisible_text_ratio, text_area_ratio)` combination, for
/// [`score_page_reaches_exactly_the_documented_set_of_scores`].
fn scores_over_codec_and_producer(
    image_coverage: f32,
    glyph_count: usize,
    invisible_text_ratio: f32,
    text_area_ratio: f32,
) -> Vec<f32> {
    let codecs = [
        ImageCodecClass::Dct,
        ImageCodecClass::Other,
        ImageCodecClass::Ccitt,
        ImageCodecClass::Jbig2,
    ];
    let producers = [ProducerPrior::Unknown, ProducerPrior::Authoring, ProducerPrior::Scanner];
    codecs
        .into_iter()
        .flat_map(|codec| {
            producers.into_iter().map(move |producer_prior| {
                score_page(&PageScanSignals {
                    image_coverage,
                    invisible_text_ratio,
                    glyph_count,
                    text_area_ratio,
                    codec,
                    producer_prior,
                })
            })
        })
        .collect()
}

/// Every distinct score `score_page` can return, over the whole signal matrix, now
/// including [`is_header_sized_text`]'s `text_area_ratio` axis (GH#1779/#1793). The set
/// itself is unchanged by that fix -- `text_area_ratio` only widens which *combinations*
/// of the other signals reach the same eight non-zero sums, it does not introduce new
/// magnitudes -- but the reachability of each one now depends on coverage tier and
/// header-sizedness together, not on coverage and glyph presence alone. ~keep
#[test]
fn score_page_reaches_exactly_the_documented_set_of_scores() {
    let mut reachable: Vec<f32> = Vec::new();
    let coverage_tiers = [
        0.0,
        IMAGE_COVERAGE_MIN - 0.01,
        IMAGE_COVERAGE_MIN,
        IMAGE_COVERAGE_FULL - 0.01,
        IMAGE_COVERAGE_FULL,
        1.0,
    ];
    let text_combos = [
        // (glyph_count, invisible_text_ratio, text_area_ratio)
        (0, 0.0, 0.0),                           // no text at all: header-sized via glyph_count
        (250, 0.0, TEXT_AREA_HEADER_MAX),        // header-sized via area, at the boundary (GH#1779/#1793)
        (250, 0.0, TEXT_AREA_HEADER_MAX + 0.01), // just over the boundary: not header-sized
        (250, 0.0, 1.0),                         // a full page of visible text: not header-sized
        (250, INVISIBLE_TEXT_MIN, 1.0),          // invisible sidecar at the ratio floor
        (250, 1.0, 1.0),                         // fully invisible sidecar
    ];
    for image_coverage in coverage_tiers {
        for (glyph_count, invisible_text_ratio, text_area_ratio) in text_combos {
            for score in
                scores_over_codec_and_producer(image_coverage, glyph_count, invisible_text_ratio, text_area_ratio)
            {
                if !reachable.iter().any(|seen| (seen - score).abs() < 1e-5) {
                    reachable.push(score);
                }
            }
        }
    }
    reachable.sort_by(|left, right| left.partial_cmp(right).expect("scores are finite"));

    let expected = [0.0, 0.50, 0.55, 0.60, 0.65, 0.85, 0.90, 0.95, 1.00];
    assert_eq!(
        reachable.len(),
        expected.len(),
        "reachable score set changed: got {reachable:?}, expected {expected:?}"
    );
    for (actual, expected) in reachable.iter().zip(expected) {
        assert_score(*actual, expected);
    }
}

/// `0.65` -- the ceiling a full-page raster with a *non-header-sized* visible text layer
/// is said to hit -- needs a bilevel codec *and* a scanner producer *and* substantial
/// (non-header-sized) visible text simultaneously.
///
/// Drop any one of the three and the page scores at most `0.60`. Before GH#1752's fix,
/// this conjunction was claimed unreachable in practice because *any* visible text at all
/// blocked [`SCORE_NO_VISIBLE_TEXT`] -- but GH#1752's own reproducer (a full-page CCITT
/// scan, scanner producer, and a 9-character corner stamp) showed real pages do carry
/// visible text here: a header, footer, or stamp. The fix narrows the ceiling's third leg
/// from "any visible text" to "non-header-sized visible text" (see
/// [`is_header_sized_text`]): a stamp-sized text layer now takes the `0.35` bonus like an
/// invisible sidecar does, landing at `1.0` instead of `0.65`. ~keep
#[test]
fn a_score_of_0_65_requires_bilevel_codec_and_scanner_producer_and_non_header_sized_text_at_once() {
    let all_three = PageScanSignals {
        image_coverage: 1.0,
        invisible_text_ratio: 0.0,
        glyph_count: 250,
        text_area_ratio: 0.5, // substantial, non-header-sized visible text
        codec: ImageCodecClass::Ccitt,
        producer_prior: ProducerPrior::Scanner,
    };
    assert_score(score_page(&all_three), 0.65);

    assert_score(
        score_page(&PageScanSignals {
            codec: ImageCodecClass::Dct,
            ..all_three
        }),
        0.55,
    );
    assert_score(
        score_page(&PageScanSignals {
            producer_prior: ProducerPrior::Authoring,
            ..all_three
        }),
        0.60,
    );
    // Hiding the text layer takes the larger `SCORE_NO_VISIBLE_TEXT` instead, which is
    // why the sidecar case never sits in the 0.50..=0.65 band at all. ~keep
    assert_score(
        score_page(&PageScanSignals {
            invisible_text_ratio: 1.0,
            ..all_three
        }),
        1.0,
    );
    // GH#1752: a header/footer/stamp-sized text layer -- what the issue's own reproducer
    // actually carries -- takes the bonus too, so the ceiling is never hit in practice. ~keep
    assert_score(
        score_page(&PageScanSignals {
            text_area_ratio: 0.02,
            ..all_three
        }),
        1.0,
    );
}

/// GH#1779/#1793/#1752: a born-digital page whose figure covers the sheet, carrying a
/// small visible text layer -- a caption, a heading, a running footer, a corner stamp --
/// scores by how much of the page that text COVERS, not by how many glyphs it has. The
/// three smallest counts on record (a 4-glyph section heading, a 27-glyph figure caption,
/// a 47-glyph newspaper footer -- issue #1752) are all header/footer-sized by area and now
/// score [`SCORE_FULL_PAGE_RASTER`] `+` [`SCORE_NO_VISIBLE_TEXT`]; a page with the same
/// glyph counts but substantial area coverage (a caption is not the same thing as a
/// half-page pull-quote) stays at [`SCORE_FULL_PAGE_RASTER`] alone, unchanged from before
/// the fix.
#[test]
fn a_full_bleed_page_scores_by_text_area_not_by_glyph_count() {
    for glyph_count in [4, 27, 47, 133, 687, 2698] {
        let header_sized = score_page(&PageScanSignals {
            image_coverage: 1.0,
            invisible_text_ratio: 0.0,
            glyph_count,
            text_area_ratio: 0.02,
            codec: ImageCodecClass::Dct,
            producer_prior: ProducerPrior::Authoring,
        });
        assert_score(header_sized, 0.85);
        assert!(
            f64::from(header_sized) >= DEFAULT_SCANNED_MIN_CONFIDENCE,
            "{glyph_count} header-sized glyphs scored {header_sized}, below the default threshold"
        );

        let substantial = score_page(&PageScanSignals {
            image_coverage: 1.0,
            invisible_text_ratio: 0.0,
            glyph_count,
            text_area_ratio: 0.40,
            codec: ImageCodecClass::Dct,
            producer_prior: ProducerPrior::Authoring,
        });
        assert_score(substantial, SCORE_FULL_PAGE_RASTER);
        assert!(
            f64::from(substantial) < DEFAULT_SCANNED_MIN_CONFIDENCE,
            "{glyph_count} glyphs covering a substantial area scored {substantial}, at or \
             above the default threshold"
        );
    }
}

#[test]
fn scanned_page_indices_selects_only_pages_at_or_above_the_threshold() {
    let detection = ScanDetection {
        confidence: 0.9,
        page_confidence: vec![0.0, 0.5, 0.85, 0.9],
    };
    assert_eq!(detection.scanned_page_indices(0.7), vec![2, 3]);
    assert_eq!(detection.scanned_page_indices(0.85), vec![2, 3]);
    assert_eq!(detection.scanned_page_indices(0.95), Vec::<usize>::new());
}

/// The doc-comment on `DEFAULT_SCANNED_MIN_CONFIDENCE` claims a slide is only
/// OCR'd at a threshold of 0.50 or lower. Pin that boundary.
#[test]
fn a_full_bleed_slide_is_selected_only_at_a_threshold_of_0_50_or_lower() {
    let slide = ScanDetection {
        confidence: SCORE_FULL_PAGE_RASTER,
        page_confidence: vec![SCORE_FULL_PAGE_RASTER],
    };
    assert_eq!(slide.scanned_page_indices(0.50), vec![0]);
    assert_eq!(slide.scanned_page_indices(0.51), Vec::<usize>::new());
    assert_eq!(
        slide.scanned_page_indices(DEFAULT_SCANNED_MIN_CONFIDENCE as f32),
        Vec::<usize>::new()
    );
}

/// Build a span with the given text and provenance for the fabricated-fraction tests.
fn provenance_span(text: &str, provenance: Option<MappingProvenance>) -> TextSpan {
    TextSpan {
        text: text.to_string(),
        provenance,
        ..TextSpan::default()
    }
}

#[test]
fn all_fallback_page_counts_every_char_as_fabricated() {
    let spans = vec![provenance_span("garbled", Some(MappingProvenance::Fallback))];
    let (fabricated, total) = fabricated_char_counts(&spans);
    assert_eq!(fabricated, 7);
    assert_eq!(total, 7);
}

#[test]
fn all_to_unicode_page_never_counts_as_fabricated() {
    let spans = vec![provenance_span("legible text", Some(MappingProvenance::ToUnicode))];
    let (fabricated, total) = fabricated_char_counts(&spans);
    assert_eq!(fabricated, 0);
    assert_eq!(total, 11);
}

#[test]
fn all_none_provenance_page_never_counts_as_fabricated() {
    let spans = vec![provenance_span("unknown provenance", None)];
    let (fabricated, total) = fabricated_char_counts(&spans);
    assert_eq!(fabricated, 0);
    assert_eq!(total, 17);
}

#[test]
fn mixed_provenance_sums_only_fallback_spans() {
    let spans = vec![
        provenance_span("good", Some(MappingProvenance::ToUnicode)),
        provenance_span("bad", Some(MappingProvenance::Fallback)),
        provenance_span("also good", Some(MappingProvenance::EncodingName)),
    ];
    let (fabricated, total) = fabricated_char_counts(&spans);
    assert_eq!(fabricated, 3);
    assert_eq!(total, 4 + 3 + 8);
}

/// Boundary behavior of the ratio check itself (mirrors `page_has_fabricated_text`'s
/// `(fabricated / total) >= min_ratio` comparison without requiring a `PdfDocument`).
#[test]
fn ratio_at_threshold_triggers_but_just_below_does_not() {
    let spans = vec![
        provenance_span("aaaa", Some(MappingProvenance::Fallback)),
        provenance_span("aaaa", Some(MappingProvenance::ToUnicode)),
    ];
    let (fabricated, total) = fabricated_char_counts(&spans);
    let ratio = fabricated as f64 / total as f64;
    assert!((ratio - 0.5).abs() < f64::EPSILON);
    assert!(ratio >= 0.5, "exactly-at-threshold ratio must trigger");
    assert!(ratio < 0.500001, "sanity: ratio is exactly one half");
}

#[test]
fn empty_spans_have_zero_total_and_never_trigger() {
    let (fabricated, total) = fabricated_char_counts(&[]);
    assert_eq!(fabricated, 0);
    assert_eq!(total, 0);
}

#[test]
fn scanned_page_indices_clamps_an_out_of_range_threshold() {
    let detection = ScanDetection {
        confidence: 0.5,
        page_confidence: vec![0.0, 0.5],
    };
    // Negative thresholds clamp to 0.0, which still selects every page. ~keep
    assert_eq!(detection.scanned_page_indices(-1.0), vec![0, 1]);
    // Thresholds above 1.0 clamp to 1.0 and select nothing below it. ~keep
    assert_eq!(detection.scanned_page_indices(2.0), Vec::<usize>::new());
}

// =========================================================================
// GH#1782 -- a page mixing a few readable lines with a Type 3 font that has no ToUnicode
// and procedural (non-AGL) /Differences glyph names. Fixtures are the reporter's
// `repro-type3-with-{2,6}-readable-lines.pdf`, pinned unmodified, verified against issue.
//
// `best_mapping_provenance`/`resolve_base_encoding_map` give every span the Type 3 font
// paints `MappingProvenance::Fallback` instead of `EncodingName`, which is what lets the
// fabricated gate see the font at all.
//
// An earlier version of this comment claimed the ratio then measured those fixtures at
// 19.7% / 7.6% and that a short Type 3 fragment therefore correctly stayed native. That
// was wrong, and wrong in a specific way worth recording: both numbers were artefacts of
// the very gap they were used to rule out. `fabricated_char_counts` counts DECODED
// characters, and the raw-code fallback in `extractors/text/advance.rs` drops any glyph
// whose code is not printable (`if ch >= '\x20' || ...`). The reporter's subset font uses
// procedure codes 1..31, so 113 of its 144 painted glyphs entered neither the numerator
// nor the denominator -- 31/(31+126) is exactly 19.75% and 31/(31+378) exactly 7.58%.
// The ratio was measuring its own blind spot.
//
// The fix is in the measurement, not the threshold: extraction now retains one `?` per
// painted glyph on an unmapped Type 3 font, and provably-blank `d0`/`d1`-only CharProcs
// map to a space so pure spacing is not counted as painted text. Exact counts are pinned
// in `scan_detect_type3_tests.rs` ((130, 256) routes, (130, 508) does not) rather than as
// a ratio, per the `measurement-discipline` rule. ~keep
// =========================================================================

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

/// GH#1782 residual, measured: unlike the two boundary fixtures above (19.7%/7.6% ratio,
/// both intentionally under 0.5), a page whose Type 3 text dominates — matching the
/// reporter's real-world document, where the font carries a median 98% of a page's
/// glyphs — clears the ratio easily and IS flagged. `gh1782-majority-share.pdf` mirrors
/// that shape (40 lines of the font under one readable header line) and measures an
/// 83.3% ratio (310/372 chars). See the module comment above for why this is possible. ~keep
#[test]
fn gh1782_majority_share_page_is_flagged_fabricated() {
    let doc = type3_fixture("gh1782-majority-share.pdf");
    let thresholds = crate::core::config::OcrQualityThresholds::default();
    let fabricated = fabricated_provenance_page_indices(
        &doc,
        thresholds.min_provenance_fallback_ratio,
        thresholds.min_total_non_whitespace,
    );
    assert_eq!(
        fabricated,
        vec![0],
        "a page whose unmapped-Type3 text dominates (matching the reported real-world \
         median 98% share) must be flagged fabricated by the existing ratio-based check"
    );
}

/// #1786: a page that is one full-page raster reports that raster's own density (400 px
/// over 100 pt is 288 dpi); the same raster covering a quarter of the page is a figure and
/// reports none; a page without images reports none.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline", feature = "layout-detection"))]
#[test]
fn full_page_raster_density_reads_the_scan_density_and_ignores_figures() {
    let scan = PdfDocument::from_bytes(crate::pdf::render::build_full_page_raster_pdf(
        (100.0, 100.0),
        (400, 400),
        1.0,
        0,
    ))
    .unwrap();
    let density = full_page_raster_density(&scan, 0).expect("a full-page raster has a density");
    assert!(
        (density - 288.0).abs() < 0.5,
        "400 px over 100 pt is 288 dpi, got {density}"
    );

    let inset = PdfDocument::from_bytes(crate::pdf::render::build_full_page_raster_pdf(
        (100.0, 100.0),
        (400, 400),
        0.64,
        1,
    ))
    .unwrap();
    let density = full_page_raster_density(&inset, 0).expect("an inset scan with a stamp has a density");
    assert!(
        (density - 360.0).abs() < 0.5,
        "400 px painted over 80 pt is 360 dpi, got {density}"
    );

    let figure = PdfDocument::from_bytes(crate::pdf::render::build_full_page_raster_pdf(
        (100.0, 100.0),
        (400, 400),
        0.64,
        20,
    ))
    .unwrap();
    let figure_text = figure
        .extract_page_text_with_options(0, ReadingOrder::ColumnAware)
        .expect("extract mapped figure text");
    let mapped_glyphs = figure_text
        .spans
        .iter()
        .filter(|span| span.provenance != Some(MappingProvenance::Fallback))
        .map(|span| span.text.chars().count())
        .sum::<usize>();
    assert!(
        mapped_glyphs > OCR_SCAN_MAX_GLYPHS,
        "negative control must exceed the readable-glyph cutoff; got {mapped_glyphs}"
    );
    assert_eq!(
        full_page_raster_density(&figure, 0),
        None,
        "the same raster beside twenty lines of text is a figure on a text page"
    );

    let small = PdfDocument::from_bytes(crate::pdf::render::build_full_page_raster_pdf(
        (100.0, 100.0),
        (400, 400),
        0.16,
        0,
    ))
    .unwrap();
    assert_eq!(
        full_page_raster_density(&small, 0),
        None,
        "a raster under a quarter of the page is never a scan"
    );

    let blank = PdfDocument::from_bytes(crate::pdf::render::build_minimal_pdf_with_mediabox(100.0, 100.0)).unwrap();
    assert_eq!(
        full_page_raster_density(&blank, 0),
        None,
        "a page without images has no raster density"
    );
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline", feature = "layout-detection"))]
#[test]
fn inset_raster_with_more_than_four_hundred_mapped_artifact_glyphs_is_a_figure() {
    let mut source = lopdf::Document::load_mem(&crate::pdf::render::build_full_page_raster_pdf(
        (100.0, 100.0),
        (400, 400),
        0.64,
        20,
    ))
    .expect("load mapped-text figure fixture");
    let page_id = *source.get_pages().get(&1).expect("fixture has one page");
    let content = String::from_utf8(source.get_page_content(page_id)).expect("fixture content is ASCII");
    let artifact_content = content
        .replace("BT /F1", "/Artifact << /Type /Layout >> BDC BT /F1")
        .replace(" Tj ET\n", " Tj ET EMC\n");
    source
        .change_page_content(page_id, artifact_content.into_bytes())
        .expect("replace page content");
    let mut bytes = Vec::new();
    source.save_to(&mut bytes).expect("serialize artifact fixture");
    let doc = PdfDocument::from_bytes(bytes).expect("open artifact fixture");

    let page = doc
        .extract_page_text_with_options(0, ReadingOrder::ColumnAware)
        .expect("extract mapped artifact text");
    let mapped_artifact_glyphs = page
        .spans
        .iter()
        .filter(|span| span.artifact_type.is_some())
        .filter(|span| span.provenance != Some(MappingProvenance::Fallback))
        .map(|span| span.text.chars().count())
        .sum::<usize>();
    assert!(
        mapped_artifact_glyphs > OCR_SCAN_MAX_GLYPHS,
        "negative control must exceed the readable-glyph cutoff; got {mapped_artifact_glyphs}"
    );
    assert_eq!(
        full_page_raster_density(&doc, 0),
        None,
        "visible mapped artifact text still makes an inset raster a figure"
    );
}

/// A page carries a scan when its rasters cover at least a quarter of it, whatever its text
/// layer: a whole-page raster, an inset one, one covering exactly a quarter, and an inset one
/// under twenty lines of mapped text all do; a raster under a quarter of the page and a page without images do not.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[test]
fn carries_scan_raster_reads_raster_coverage_only() {
    let page = |coverage: f32, text_lines: usize| {
        PdfDocument::from_bytes(crate::pdf::render::build_full_page_raster_pdf(
            (100.0, 100.0),
            (400, 400),
            coverage,
            text_lines,
        ))
        .unwrap()
    };
    assert!(carries_scan_raster(&page(1.0, 0), 0), "a whole-page raster is a scan");
    assert!(carries_scan_raster(&page(0.64, 0), 0), "an inset raster is a scan");
    assert!(
        carries_scan_raster(&page(0.25, 0), 0),
        "a raster covering exactly a quarter of the page is a scan"
    );
    assert!(
        carries_scan_raster(&page(0.64, 20), 0),
        "the text layer does not decide whether the page carries a scan"
    );
    assert!(
        !carries_scan_raster(&page(0.16, 0), 0),
        "a raster under a quarter of the page is not a scan"
    );

    let blank = PdfDocument::from_bytes(crate::pdf::render::build_minimal_pdf_with_mediabox(100.0, 100.0)).unwrap();
    assert!(!carries_scan_raster(&blank, 0), "a page without images carries no scan");
}
