use super::super::extract_api::page_span_interpretations;
use super::super::*;
use super::common::*;

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
        format!(
            "{:?}",
            doc.extract_painting_text_and_spans(page).expect("painting spans")
        ),
        format!("{:?}", doc.mc_actualtext_mcids.lock().unwrap().get(&page)),
    ]
}

#[test]
fn cached_reads_equal_a_fresh_interpretation_for_every_option_set() {
    let pdf = option_set_pdf();
    let warm = PdfDocument::from_bytes(pdf.clone()).expect("open warm document");
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
