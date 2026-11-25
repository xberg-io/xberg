use super::super::extract_api::page_span_interpretations;
use super::super::*;
use super::common::*;
use super::pdf_fixtures::build_catalog_test_pdf;

const FULL_PAGE: &str = "[0 0 600 800]";
const ROTATED_PAGE: &str = "[0 0 600 800] /Rotate 90";
const LAYER_CATALOG: &str = "/OCProperties << /OCGs [5 0 R] /D << /ON [5 0 R] >> >>";
const LAYER_RESOURCES: &str = "/Properties << /L1 5 0 R >>";

const PROSE: &str = "BT /F1 12 Tf 72 700 Td (A first line of prose) Tj 0 -14 Td (and a second line) Tj ET\n";
const SCRIPTS: &str = "BT /F1 12 Tf 72 700 Td (E = mc) Tj ET\n\
     BT /F1 8 Tf 110 703 Td (2) Tj ET\n\
     BT /F1 12 Tf 116 700 Td (holds) Tj ET\n\
     BT 3 Tr /F1 12 Tf 72 600 Td (Ghost) Tj ET\n";
const ROTATED: &str = "BT /F1 12 Tf 72 700 Td (Upright words) Tj ET\n\
     BT /F1 12 Tf 0 1 -1 0 300 100 Tm (Sideways words) Tj ET\n";
const ACTUAL_TEXT: &str =
    "/Span << /MCID 5 /ActualText (Replacement) >> BDC BT /F1 12 Tf 72 700 Td (Rxplacxmxnt) Tj ET EMC\n";
const LAYERED: &str = "BT /F1 12 Tf 72 700 Td (Visible words) Tj ET\n\
     /OC /L1 BDC BT /F1 12 Tf 72 680 Td (Layered words) Tj ET EMC\n";

fn option_set_pdf() -> Vec<u8> {
    build_pages_pdf(
        &[
            (FULL_PAGE, PROSE),
            (FULL_PAGE, SCRIPTS),
            (ROTATED_PAGE, ROTATED),
            (FULL_PAGE, ACTUAL_TEXT),
            (FULL_PAGE, LAYERED),
        ],
        &[],
        LAYER_CATALOG,
        LAYER_RESOURCES,
    )
}

fn many_pages_pdf(count: usize) -> Vec<u8> {
    let streams: Vec<String> = (0..count)
        .map(|page| format!("BT /F1 12 Tf 72 700 Td (Page {page} body) Tj 0 -14 Td (second line {page}) Tj ET\n"))
        .collect();
    let pages: Vec<(&str, &str)> = streams.iter().map(|stream| (FULL_PAGE, stream.as_str())).collect();
    build_pages_pdf(&pages, &[], "", "")
}

/// Every read a default extraction makes of one page, under each option set the
/// raw span cache keys on, rendered with `Debug` so every field is compared.
fn page_reads(doc: &PdfDocument, page: usize) -> Vec<String> {
    let mut witnesses = Vec::new();
    let preserved = doc
        .extract_page_text_with_cmsy_capture(page, ReadingOrder::TopToBottom, Some(&mut witnesses))
        .expect("preserved script geometry");
    vec![
        format!("{:?}", doc.extract_spans_raw(page).expect("raw spans")),
        format!(
            "{:?}",
            doc.extract_spans_with_reading_order(page, ReadingOrder::ColumnAware)
                .expect("column-aware spans")
        ),
        format!(
            "{:?}",
            doc.extract_page_text_with_options(page, ReadingOrder::TopToBottom)
                .expect("top-to-bottom page text")
        ),
        format!("{:?}", doc.extract_spans(page).expect("postprocessed spans")),
        format!(
            "{:?}",
            doc.extract_page_text_with_script_geometry(page, ReadingOrder::TopToBottom)
                .expect("script geometry")
        ),
        format!("{preserved:?}"),
        format!("{witnesses:?}"),
        format!("{:?}", doc.mc_actualtext_mcids.lock().unwrap().get(&page)),
    ]
}

#[test]
fn cached_reads_equal_a_fresh_interpretation_for_every_option_set() {
    let pdf = option_set_pdf();
    let warm = PdfDocument::from_bytes(pdf.clone()).expect("open warm document");
    warm.begin_page_passes();
    let pages = warm.page_count().unwrap();
    assert_eq!(pages, 5);
    for page in 0..pages {
        drop(page_reads(&warm, page));
    }
    for page in 0..pages {
        let fresh = PdfDocument::from_bytes(pdf.clone()).expect("open fresh document");
        let before = page_span_interpretations();
        let cached = page_reads(&warm, page);
        assert_eq!(
            page_span_interpretations(),
            before,
            "page {page}: the warm reads must all be served from the cache"
        );
        assert_eq!(cached, page_reads(&fresh, page), "page {page}");
    }
}

#[test]
fn the_fixture_exercises_script_geometry_rotation_and_actual_text() {
    let doc = PdfDocument::from_bytes(option_set_pdf()).expect("open document");
    let flattened = format!(
        "{:?}",
        doc.extract_page_text_with_options(1, ReadingOrder::TopToBottom)
            .unwrap()
            .spans
    );
    let preserved = format!(
        "{:?}",
        doc.extract_page_text_with_script_geometry(1, ReadingOrder::TopToBottom)
            .unwrap()
            .spans
    );
    assert_ne!(flattened, preserved, "the script page must differ by geometry");
    let painted: String = doc
        .extract_painting_text_and_spans(1)
        .unwrap()
        .1
        .iter()
        .map(|span| span.text.as_str())
        .collect();
    assert!(!painted.contains("Ghost"), "invisible text stays out: {painted:?}");
    assert!(
        doc.extract_spans_raw(1)
            .unwrap()
            .iter()
            .any(|span| span.text.contains("Ghost")),
        "the default read keeps invisible text"
    );
    assert_eq!(doc.get_page_rotation(2).unwrap(), 90);
    drop(doc.extract_spans_raw(3).unwrap());
    assert_eq!(
        doc.mc_actualtext_mcids.lock().unwrap().get(&3),
        Some(&HashSet::from([5]))
    );
}

/// A layer-filtered read is not served from, and does not enter, the
/// unfiltered cache.
#[test]
fn filtered_reads_bypass_the_unfiltered_cache() {
    let pdf = option_set_pdf();
    let hidden: HashSet<String> = HashSet::from(["Hidden".to_string()]);
    let filtered = |doc: &PdfDocument| -> String {
        doc.extract_spans_filtered_with_reading_order(4, ReadingOrder::ColumnAware, hidden.clone(), HashSet::new())
            .unwrap()
            .iter()
            .map(|span| span.text.as_str())
            .collect()
    };
    let unfiltered = |doc: &PdfDocument| -> String {
        doc.extract_spans_with_reading_order(4, ReadingOrder::ColumnAware)
            .unwrap()
            .iter()
            .map(|span| span.text.as_str())
            .collect()
    };
    let fresh = PdfDocument::from_bytes(pdf.clone()).unwrap();
    let expected_filtered = filtered(&fresh);
    assert!(!expected_filtered.contains("Layered"), "{expected_filtered:?}");

    let warm = PdfDocument::from_bytes(pdf).unwrap();
    warm.begin_page_passes();
    let expected_unfiltered = unfiltered(&warm);
    assert!(expected_unfiltered.contains("Layered"), "{expected_unfiltered:?}");
    assert_eq!(filtered(&warm), expected_filtered);
    assert_eq!(unfiltered(&warm), expected_unfiltered);
}

/// The text pass reads every page first; the table words, the heuristic
/// segments and the table script geometry then read the same pages again.
#[test]
fn later_passes_over_more_than_eight_pages_do_not_reinterpret() {
    const PAGES: usize = 12;
    let doc = PdfDocument::from_bytes(many_pages_pdf(PAGES)).expect("open document");
    doc.begin_page_passes();
    assert_eq!(doc.page_count().unwrap(), PAGES);
    let start = page_span_interpretations();
    for page in 0..PAGES {
        drop(
            doc.extract_spans_with_reading_order(page, ReadingOrder::ColumnAware)
                .unwrap(),
        );
    }
    assert_eq!(
        page_span_interpretations() - start,
        PAGES,
        "the text pass reads each page once"
    );
    for page in 0..PAGES {
        drop(doc.extract_words(page).unwrap());
    }
    for page in 0..PAGES {
        drop(
            doc.extract_page_text_with_options(page, ReadingOrder::TopToBottom)
                .unwrap(),
        );
    }
    assert_eq!(
        page_span_interpretations() - start,
        PAGES,
        "flattened passes reuse the text pass"
    );
    for _ in 0..2 {
        for page in 0..PAGES {
            drop(
                doc.extract_page_text_with_script_geometry(page, ReadingOrder::TopToBottom)
                    .unwrap(),
            );
        }
    }
    assert_eq!(
        page_span_interpretations() - start,
        2 * PAGES,
        "preserved script geometry is interpreted once per page"
    );
}

#[test]
fn a_witness_request_reinterprets_a_page_cached_without_witnesses() {
    let pdf = option_set_pdf();
    let doc = PdfDocument::from_bytes(pdf.clone()).unwrap();
    doc.begin_page_passes();
    drop(
        doc.extract_page_text_with_script_geometry(1, ReadingOrder::TopToBottom)
            .unwrap(),
    );
    let before = page_span_interpretations();
    let mut witnesses = vec![make_test_span("stale", 0.0, 0.0, 1.0, 1.0)];
    let captured = doc
        .extract_page_text_with_cmsy_capture(1, ReadingOrder::TopToBottom, Some(&mut witnesses))
        .unwrap();
    assert_eq!(page_span_interpretations() - before, 1);

    let fresh = PdfDocument::from_bytes(pdf).unwrap();
    let mut fresh_witnesses = vec![make_test_span("stale", 0.0, 0.0, 1.0, 1.0)];
    let fresh_captured = fresh
        .extract_page_text_with_cmsy_capture(1, ReadingOrder::TopToBottom, Some(&mut fresh_witnesses))
        .unwrap();
    assert_eq!(format!("{captured:?}"), format!("{fresh_captured:?}"));
    assert_eq!(format!("{witnesses:?}"), format!("{fresh_witnesses:?}"));

    let before = page_span_interpretations();
    drop(
        doc.extract_page_text_with_script_geometry(1, ReadingOrder::TopToBottom)
            .unwrap(),
    );
    let mut again = Vec::new();
    drop(
        doc.extract_page_text_with_cmsy_capture(1, ReadingOrder::TopToBottom, Some(&mut again))
            .unwrap(),
    );
    assert_eq!(
        page_span_interpretations(),
        before,
        "the witness entry serves both requests"
    );
    assert_eq!(format!("{again:?}"), format!("{fresh_witnesses:?}"));
}

/// A filtered read replaces the page's MC-scope /ActualText entry; a cached
/// unfiltered read must put back what its own interpretation recorded.
#[test]
fn a_cache_hit_restores_the_pages_actual_text_entry() {
    let doc = PdfDocument::from_bytes(option_set_pdf()).unwrap();
    doc.begin_page_passes();
    drop(doc.extract_spans_raw(3).unwrap());
    let recorded = doc.mc_actualtext_mcids.lock().unwrap().get(&3).cloned();
    assert_eq!(recorded, Some(HashSet::from([5])));
    doc.mc_actualtext_mcids.lock().unwrap().insert(3, HashSet::from([99]));
    let before = page_span_interpretations();
    drop(doc.extract_spans_raw(3).unwrap());
    assert_eq!(page_span_interpretations(), before);
    assert_eq!(doc.mc_actualtext_mcids.lock().unwrap().get(&3).cloned(), recorded);
}

#[test]
fn the_budget_admits_until_full_and_keeps_earlier_entries() {
    let mut cache = crate::cache::AdmissionBudgetCache::new(10);
    assert!(cache.insert(0, "a", 6));
    assert!(!cache.insert(1, "b", 5), "an entry past the budget is refused");
    assert_eq!(cache.get(&0), Some(&"a"));
    assert!(cache.insert(0, "c", 10), "a replacement may use the bytes it frees");
    assert_eq!(cache.get(&0), Some(&"c"));
    assert!(!cache.insert(0, "d", 11));
    assert_eq!(cache.get(&0), Some(&"c"), "a refused replacement keeps the entry");
    cache.clear();
    assert_eq!(cache.len(), 0);
    assert!(cache.insert(1, "b", 10));
}

#[test]
fn the_cache_admission_check_matches_insert() {
    let mut cache = crate::cache::AdmissionBudgetCache::new(10);
    assert!(cache.admits(&0, 10));
    assert!(!cache.admits(&0, 11));
    assert!(cache.insert(0, "a", 6));
    assert!(!cache.admits(&1, 5));
    assert!(cache.admits(&0, 10), "a replacement counts the bytes it frees");
}

/// Outside `begin_page_passes`/`release_page_caches` nothing is cached, and
/// releasing empties every per-page cache.
#[test]
fn the_page_pass_cache_lives_only_between_begin_and_release() {
    let doc = PdfDocument::from_bytes(many_pages_pdf(2)).expect("open document");
    let reads = |doc: &PdfDocument| {
        let before = page_span_interpretations();
        for _ in 0..2 {
            drop(doc.extract_spans_raw(0).unwrap());
        }
        page_span_interpretations() - before
    };
    assert_eq!(reads(&doc), 2, "no cache before the passes begin");
    doc.begin_page_passes();
    assert_eq!(reads(&doc), 1, "one interpretation while the passes run");
    drop(doc.extract_spans(1).unwrap());
    drop(doc.extract_chars(1).unwrap());
    doc.release_page_caches();
    assert!(doc.raw_page_spans_cache.lock().unwrap().is_none());
    assert_eq!(doc.page_spans_cache.lock().unwrap().len(), 0);
    assert_eq!(doc.page_chars_cache.lock().unwrap().len(), 0);
    assert_eq!(reads(&doc), 2, "no cache after release");
}

#[test]
fn the_page_pass_budget_is_proportional_to_the_source_and_capped() {
    let doc = PdfDocument::from_bytes(many_pages_pdf(2)).expect("open document");
    doc.begin_page_passes();
    let budget = doc
        .raw_page_spans_cache
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .budget_bytes();
    assert_eq!(budget, 32 * doc.source_bytes.len());
    let mut large = PdfDocument::from_bytes(many_pages_pdf(2)).expect("open document");
    large.source_bytes.resize(1024 * 1024, b' ');
    large.begin_page_passes();
    let budget = large
        .raw_page_spans_cache
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .budget_bytes();
    assert_eq!(budget, 8 * 1024 * 1024);
}

/// A page whose interpretation does not fit the budget is read on every pass
/// and still equals an uncached read.
#[test]
fn pages_past_the_budget_are_interpreted_each_read_and_stay_equal() {
    let pdf = many_pages_pdf(3);
    let doc = PdfDocument::from_bytes(pdf.clone()).expect("open document");
    doc.begin_page_passes();
    *doc.raw_page_spans_cache.lock().unwrap() = Some(crate::cache::AdmissionBudgetCache::new(0));
    let before = page_span_interpretations();
    let first = format!("{:?}", doc.extract_spans_raw(2).unwrap());
    let second = format!("{:?}", doc.extract_spans_raw(2).unwrap());
    assert_eq!(page_span_interpretations() - before, 2);
    assert_eq!(doc.raw_page_spans_cache.lock().unwrap().as_ref().unwrap().len(), 0);
    let fresh = PdfDocument::from_bytes(pdf).unwrap();
    let reference = format!("{:?}", fresh.extract_spans_raw(2).unwrap());
    assert_eq!(first, reference);
    assert_eq!(second, reference);
}

/// A page drawn with a Type 3 font (a structured warning at font load) and with
/// a font name its resources lack (a missing-font recovery per decode).
fn warning_pdf() -> Vec<u8> {
    let content = "BT /F3 12 Tf 72 700 Td (A) Tj ET\nBT /F9 12 Tf 72 680 Td (Hello) Tj ET\n";
    let stream = format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len());
    build_catalog_test_pdf(
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        &[
            (
                3,
                &b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R \
                   /Resources << /Font << /F3 5 0 R >> >> >>"[..],
            ),
            (4, stream.as_bytes()),
            (
                5,
                &b"<< /Type /Font /Subtype /Type3 /FontBBox [0 0 1000 1000] \
                   /FontMatrix [0.001 0 0 0.001 0 0] /CharProcs << >> \
                   /Encoding << /Type /Encoding /Differences [65 /A] >> \
                   /FirstChar 65 /LastChar 65 /Widths [500] /Resources << >> >>"[..],
            ),
        ],
    )
}

#[test]
fn a_cache_hit_replays_the_warnings_its_interpretation_pushed() {
    let doc = PdfDocument::from_bytes(warning_pdf()).expect("open document");
    doc.begin_page_passes();
    drop(doc.take_structured_warnings());
    drop(doc.extract_spans_raw(0).unwrap());
    let first = doc.take_structured_warnings();
    assert!(
        first
            .iter()
            .any(|warning| warning.category == crate::extractors::warnings::WarningCategory::Type3Font),
        "{first:?}"
    );
    let before = page_span_interpretations();
    drop(doc.extract_spans_raw(0).unwrap());
    assert_eq!(page_span_interpretations(), before, "the second read is a hit");
    assert_eq!(format!("{:?}", doc.take_structured_warnings()), format!("{first:?}"));
}

#[test]
fn a_cache_hit_charges_the_recoveries_of_its_interpretation_again() {
    let doc = PdfDocument::from_bytes(warning_pdf()).expect("open document");
    doc.begin_page_passes();
    let start = doc.recovery_counts().snapshot();
    drop(doc.extract_spans_raw(0).unwrap());
    let first = doc.recovery_counts().snapshot().since(&start);
    assert!(first.missing_font_decodes > 0, "{first:?}");
    let middle = doc.recovery_counts().snapshot();
    drop(doc.extract_spans_raw(0).unwrap());
    assert_eq!(doc.recovery_counts().snapshot().since(&middle), first);
}
