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
/// `before_each` runs ahead of every read.
fn page_reads(doc: &PdfDocument, page: usize, before_each: impl Fn(&PdfDocument)) -> Vec<String> {
    let mut reads = Vec::new();
    before_each(doc);
    let mut witnesses = Vec::new();
    let preserved = doc
        .extract_page_text_with_cmsy_capture(page, ReadingOrder::TopToBottom, Some(&mut witnesses))
        .expect("preserved script geometry");
    reads.push(format!("{preserved:?}"));
    reads.push(format!("{witnesses:?}"));
    before_each(doc);
    reads.push(format!("{:?}", doc.extract_spans_raw(page).expect("raw spans")));
    before_each(doc);
    reads.push(format!(
        "{:?}",
        doc.extract_spans_with_reading_order(page, ReadingOrder::ColumnAware)
            .expect("column-aware spans")
    ));
    before_each(doc);
    reads.push(format!(
        "{:?}",
        doc.extract_page_text_with_options(page, ReadingOrder::TopToBottom)
            .expect("top-to-bottom page text")
    ));
    before_each(doc);
    reads.push(format!("{:?}", doc.extract_spans(page).expect("postprocessed spans")));
    before_each(doc);
    reads.push(format!(
        "{:?}",
        doc.extract_page_text_with_script_geometry(page, ReadingOrder::TopToBottom)
            .expect("script geometry")
    ));
    before_each(doc);
    reads.push(format!("{:?}", doc.extract_text(page).expect("text")));
    reads.push(format!("{:?}", doc.mc_actualtext_mcids.lock().unwrap().get(&page)));
    reads
}

/// Read every page of `pdf` in `order` with the page passes running, then check
/// that a second round is served entirely from the cache and equals, read by
/// read, a cold document of the same bytes with every page cache emptied before
/// each read.
fn assert_cached_reads_equal_uncached(pdf: &[u8], order: &[usize]) {
    let warm = PdfDocument::from_bytes(pdf.to_vec()).expect("open warm document");
    warm.begin_page_passes();
    for &page in order {
        drop(page_reads(&warm, page, |_| {}));
    }
    for &page in order {
        let before = page_span_interpretations();
        let cached = page_reads(&warm, page, |_| {});
        assert_eq!(
            page_span_interpretations(),
            before,
            "page {page}, order {order:?}: the warm reads must all be served from the cache"
        );
        let cold = PdfDocument::from_bytes(pdf.to_vec()).expect("open cold document");
        let uncached = page_reads(&cold, page, PdfDocument::release_page_caches);
        assert_eq!(cached, uncached, "page {page}, order {order:?}");
    }
}

#[test]
fn cached_reads_equal_uncached_reads_for_every_option_set() {
    assert_cached_reads_equal_uncached(&option_set_pdf(), &[0, 1, 2, 3, 4]);
}

#[test]
fn cached_reads_equal_uncached_reads_on_a_tagged_layered_page() {
    assert_cached_reads_equal_uncached(&super::span_cache_actual_text::tagged_layered_pdf(), &[0]);
}

/// Two pages that paint one Form XObject under different font scopes, read in
/// both orders: the shared font and Form caches are where GH#2429 found
/// order-dependent decoding.
#[test]
fn cached_reads_equal_uncached_reads_across_a_shared_form_in_either_order() {
    let pdfs = [
        super::form_cache_page_context::inherited_cmap_form_pdf(16),
        super::form_cache_page_context::shadowed_selected_font_pdf("D", 13),
        super::form_cache_page_context::shadowed_selected_font_pdf("space", 12),
    ];
    for pdf in &pdfs {
        for order in [[0, 1], [1, 0]] {
            assert_cached_reads_equal_uncached(pdf, &order);
        }
    }
}

/// The CMSY source-glyph fixture of the structure pass's inline-math evidence.
fn cmsy_witness_pdf() -> Vec<u8> {
    let content = "BT /F1 12 Tf 40 700 Td (V) Tj 0 -30 Td [(V)] TJ ET";
    let stream = format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len());
    build_catalog_test_pdf(
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        &[
            (
                3,
                &b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 600 800] /Contents 4 0 R \
                   /Resources << /Font << /F1 5 0 R >> >> >>"[..],
            ),
            (4, stream.as_bytes()),
            (5, &b"<< /Type /Font /Subtype /Type1 /BaseFont /ABCDEF+CMSY10 >>"[..]),
        ],
    )
}

#[test]
fn cached_reads_equal_uncached_reads_with_cmsy_witnesses() {
    let pdf = cmsy_witness_pdf();
    let doc = PdfDocument::from_bytes(pdf.clone()).expect("open document");
    let mut witnesses = Vec::new();
    drop(
        doc.extract_page_text_with_cmsy_capture(0, ReadingOrder::TopToBottom, Some(&mut witnesses))
            .unwrap(),
    );
    assert_eq!(witnesses.len(), 2, "the fixture must yield witnesses");
    assert_cached_reads_equal_uncached(&pdf, &[0]);
}

/// With preserved reads expected, read each page flattened and then preserved: one content run
/// serves both, the preserved read and the flattened script baselines equal a cold document's,
/// and every later read equals an uncached one.
fn assert_paired_reads_equal_uncached(pdf: &[u8], order: &[usize]) {
    let warm = PdfDocument::from_bytes(pdf.to_vec()).expect("open warm document");
    warm.begin_page_passes();
    warm.expect_preserved_script_geometry();
    for &page in order {
        let before = page_span_interpretations();
        drop(warm.extract_spans_raw(page).expect("flattened read"));
        let mut witnesses = Vec::new();
        let preserved = warm
            .extract_page_text_with_cmsy_capture(page, ReadingOrder::TopToBottom, Some(&mut witnesses))
            .expect("preserved read");
        assert_eq!(
            page_span_interpretations() - before,
            1,
            "page {page}, order {order:?}: one content run serves both geometries"
        );

        let cold = PdfDocument::from_bytes(pdf.to_vec()).expect("open cold document");
        drop(cold.extract_spans_raw(page).expect("cold flattened read"));
        let mut cold_witnesses = Vec::new();
        let cold_preserved = cold
            .extract_page_text_with_cmsy_capture(page, ReadingOrder::TopToBottom, Some(&mut cold_witnesses))
            .expect("cold preserved read");
        assert_eq!(format!("{preserved:?}"), format!("{cold_preserved:?}"), "page {page}");
        assert_eq!(format!("{witnesses:?}"), format!("{cold_witnesses:?}"), "page {page}");
        assert_eq!(
            warm.flattened_script_baselines(page),
            cold.flattened_script_baselines(page),
            "page {page}"
        );
    }
    for &page in order {
        let before = page_span_interpretations();
        let cached = page_reads(&warm, page, |_| {});
        assert_eq!(
            page_span_interpretations(),
            before,
            "page {page}, order {order:?}: the warm reads must all be served from the cache"
        );
        let cold = PdfDocument::from_bytes(pdf.to_vec()).expect("open cold document");
        let uncached = page_reads(&cold, page, PdfDocument::release_page_caches);
        assert_eq!(cached, uncached, "page {page}, order {order:?}");
    }
}

#[test]
fn paired_reads_equal_uncached_reads_for_every_fixture() {
    assert_paired_reads_equal_uncached(&option_set_pdf(), &[0, 1, 2, 3, 4]);
    assert_paired_reads_equal_uncached(&super::span_cache_actual_text::tagged_layered_pdf(), &[0]);
    assert_paired_reads_equal_uncached(&cmsy_witness_pdf(), &[0]);
    assert_paired_reads_equal_uncached(&warning_pdf(), &[0]);
    for pdf in [
        super::form_cache_page_context::inherited_cmap_form_pdf(16),
        super::form_cache_page_context::shadowed_selected_font_pdf("D", 13),
        super::form_cache_page_context::shadowed_selected_font_pdf("space", 12),
    ] {
        for order in [[0, 1], [1, 0]] {
            assert_paired_reads_equal_uncached(&pdf, &order);
        }
    }
}

/// The preserved read a paired flattened read cached pushes the warnings and charges the
/// recoveries of a preserved read of its own.
#[test]
fn a_paired_preserved_hit_replays_the_effects_of_its_own_read() {
    let effects = |doc: &PdfDocument| {
        drop(doc.take_structured_warnings());
        let start = doc.recovery_counts().snapshot();
        drop(
            doc.extract_page_text_with_script_geometry(0, ReadingOrder::TopToBottom)
                .unwrap(),
        );
        (
            format!("{:?}", doc.take_structured_warnings()),
            doc.recovery_counts().snapshot().since(&start),
        )
    };
    let warm = PdfDocument::from_bytes(warning_pdf()).expect("open document");
    warm.begin_page_passes();
    warm.expect_preserved_script_geometry();
    drop(warm.extract_spans_raw(0).unwrap());
    let before = page_span_interpretations();
    let (warnings, recovered) = effects(&warm);
    assert_eq!(page_span_interpretations(), before, "the preserved read is a hit");
    let cold = PdfDocument::from_bytes(warning_pdf()).expect("open document");
    let (cold_warnings, cold_recovered) = effects(&cold);
    assert!(cold_warnings.contains("Type3Font"), "{cold_warnings}");
    assert!(cold_recovered.missing_font_decodes > 0, "{cold_recovered:?}");
    assert_eq!(warnings, cold_warnings);
    assert_eq!(recovered, cold_recovered);
}

/// Without the announcement, or after the passes end, a flattened read interprets only its own
/// geometry.
#[test]
fn preserved_geometry_is_paired_only_when_announced_while_the_passes_run() {
    let preserved_runs = |announce: bool| {
        let doc = PdfDocument::from_bytes(option_set_pdf()).expect("open document");
        doc.begin_page_passes();
        if announce {
            doc.expect_preserved_script_geometry();
        }
        drop(doc.extract_spans_raw(1).unwrap());
        let before = page_span_interpretations();
        drop(
            doc.extract_page_text_with_script_geometry(1, ReadingOrder::TopToBottom)
                .unwrap(),
        );
        page_span_interpretations() - before
    };
    assert_eq!(preserved_runs(false), 1);
    assert_eq!(preserved_runs(true), 0);

    let doc = PdfDocument::from_bytes(option_set_pdf()).expect("open document");
    doc.begin_page_passes();
    doc.expect_preserved_script_geometry();
    doc.release_page_caches();
    doc.begin_page_passes();
    drop(doc.extract_spans_raw(1).unwrap());
    let before = page_span_interpretations();
    drop(
        doc.extract_page_text_with_script_geometry(1, ReadingOrder::TopToBottom)
            .unwrap(),
    );
    assert_eq!(page_span_interpretations() - before, 1, "release ends the pairing");
}

/// A budget that holds every page's flattened entry and nothing more still holds them all with
/// preserved reads paired: a preserved entry only takes its page's share of the budget.
#[test]
fn paired_preserved_entries_leave_the_flattened_entries_their_room() {
    const PAGES: usize = 4;
    let pdf = many_pages_pdf(PAGES);
    let unpaired = PdfDocument::from_bytes(pdf.clone()).unwrap();
    unpaired.begin_page_passes();
    for page in 0..PAGES {
        drop(unpaired.extract_spans_raw(page).unwrap());
    }
    let flattened_bytes = unpaired
        .raw_page_spans_cache
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .used_bytes();

    let doc = PdfDocument::from_bytes(pdf).unwrap();
    doc.begin_page_passes();
    *doc.raw_page_spans_cache.lock().unwrap() = Some(crate::cache::AdmissionBudgetCache::new(flattened_bytes));
    doc.expect_preserved_script_geometry();
    for page in 0..PAGES {
        drop(doc.extract_spans_raw(page).unwrap());
    }
    let before = page_span_interpretations();
    for page in 0..PAGES {
        drop(doc.extract_spans_raw(page).unwrap());
    }
    assert_eq!(
        page_span_interpretations(),
        before,
        "every flattened entry stayed cached"
    );
}

/// The span-only page reads are the page-text reads with an empty character list.
#[test]
fn span_only_page_reads_equal_page_text_reads_without_chars() {
    let pdf = option_set_pdf();
    for page in 0..5 {
        for order in [ReadingOrder::TopToBottom, ReadingOrder::ColumnAware] {
            let doc = PdfDocument::from_bytes(pdf.clone()).unwrap();
            let mut text = doc.extract_page_text_with_options(page, order).unwrap();
            assert!(!text.chars.is_empty(), "page {page} has characters");
            let fresh = PdfDocument::from_bytes(pdf.clone()).unwrap();
            let spans = fresh.extract_page_spans_with_options(page, order).unwrap();
            assert!(spans.chars.is_empty());
            text.chars.clear();
            assert_eq!(format!("{spans:?}"), format!("{text:?}"), "page {page}");
        }
    }
    for (pdf, page) in [(cmsy_witness_pdf(), 0), (option_set_pdf(), 1)] {
        let doc = PdfDocument::from_bytes(pdf.clone()).unwrap();
        let mut witnesses = Vec::new();
        let mut text = doc
            .extract_page_text_with_cmsy_capture(page, ReadingOrder::TopToBottom, Some(&mut witnesses))
            .unwrap();
        let fresh = PdfDocument::from_bytes(pdf).unwrap();
        let mut span_witnesses = Vec::new();
        let spans = fresh
            .extract_page_spans_with_cmsy_capture(page, ReadingOrder::TopToBottom, Some(&mut span_witnesses))
            .unwrap();
        assert!(spans.chars.is_empty());
        text.chars.clear();
        assert_eq!(format!("{spans:?}"), format!("{text:?}"), "page {page}");
        assert_eq!(format!("{span_witnesses:?}"), format!("{witnesses:?}"), "page {page}");
    }
}

#[test]
fn the_fixture_exercises_script_geometry_and_actual_text() {
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

/// The text pass reads preserved script geometry without witnesses and the
/// markdown hierarchy then asks for them: while the passes run, the first read
/// captures them, so the witness request is a hit that equals a cold capture.
#[test]
fn a_witness_request_is_served_from_a_page_cached_by_a_plain_read() {
    for (pdf, page) in [(cmsy_witness_pdf(), 0), (option_set_pdf(), 1)] {
        let doc = PdfDocument::from_bytes(pdf.clone()).unwrap();
        doc.begin_page_passes();
        let plain = doc
            .extract_page_text_with_script_geometry(page, ReadingOrder::TopToBottom)
            .unwrap();
        let before = page_span_interpretations();
        let mut witnesses = vec![make_test_span("stale", 0.0, 0.0, 1.0, 1.0)];
        let captured = doc
            .extract_page_text_with_cmsy_capture(page, ReadingOrder::TopToBottom, Some(&mut witnesses))
            .unwrap();
        assert_eq!(
            page_span_interpretations(),
            before,
            "page {page}: the witness request is a hit"
        );
        assert_eq!(format!("{captured:?}"), format!("{plain:?}"));

        let fresh = PdfDocument::from_bytes(pdf).unwrap();
        let mut fresh_witnesses = vec![make_test_span("stale", 0.0, 0.0, 1.0, 1.0)];
        let fresh_captured = fresh
            .extract_page_text_with_cmsy_capture(page, ReadingOrder::TopToBottom, Some(&mut fresh_witnesses))
            .unwrap();
        assert_eq!(format!("{captured:?}"), format!("{fresh_captured:?}"));
        assert_eq!(format!("{witnesses:?}"), format!("{fresh_witnesses:?}"));
    }
}

/// Every option-set page and the CMSY fixture, each interpreted cold with and
/// without witness capture: the page text, the warnings and the page's
/// /ActualText entry are the same, which is what lets one cached entry answer
/// both requests.
#[test]
fn capturing_witnesses_does_not_change_the_interpretation() {
    let read = |pdf: &[u8], page: usize, capture: bool| {
        let doc = PdfDocument::from_bytes(pdf.to_vec()).unwrap();
        drop(doc.take_structured_warnings());
        let mut witnesses = Vec::new();
        let text = doc
            .extract_page_text_with_cmsy_capture(page, ReadingOrder::TopToBottom, capture.then_some(&mut witnesses))
            .unwrap();
        let effects = format!(
            "{:?} {:?}",
            doc.take_structured_warnings(),
            doc.mc_actualtext_mcids.lock().unwrap().get(&page)
        );
        (format!("{text:?}"), effects, witnesses.len())
    };
    let cases = [
        (option_set_pdf(), 0..5),
        (cmsy_witness_pdf(), 0..1),
        (warning_pdf(), 0..1),
    ];
    let mut witnessed = 0;
    for (pdf, pages) in cases {
        for page in pages {
            let (plain_text, plain_effects, _) = read(&pdf, page, false);
            let (captured_text, captured_effects, witnesses) = read(&pdf, page, true);
            assert_eq!(captured_text, plain_text, "page {page}");
            assert_eq!(captured_effects, plain_effects, "page {page}");
            witnessed += witnesses;
        }
    }
    assert_eq!(witnessed, 2, "the CMSY fixture must yield its two witnesses");
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
    large.source_bytes = bytes::Bytes::from(vec![b' '; 1024 * 1024]);
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

/// The ruled-table pass reads one page's characters three times in a row: span
/// postprocessing fills the entry, then the table decimal stamp and the
/// ruled-cell repair read it. Each later read is the entry postprocessing
/// filled, and equals the public, copied result.
#[test]
fn every_char_read_on_the_current_page_reuses_the_postprocessed_entry() {
    let doc = PdfDocument::from_bytes(many_pages_pdf(4)).expect("open document");
    for page in 0..4 {
        drop(doc.extract_spans(page).unwrap());
        let filled = std::sync::Arc::clone(
            doc.page_chars_cache
                .lock()
                .unwrap()
                .get(&page)
                .expect("span postprocessing fills the page's characters"),
        );
        assert!(!filled.is_empty(), "page {page} has characters");
        for _ in 0..2 {
            let read = doc.cached_page_chars(page).unwrap();
            assert!(std::sync::Arc::ptr_eq(&filled, &read), "page {page} read is a hit");
        }
        assert_eq!(
            format!("{:?}", doc.extract_chars(page).unwrap()),
            format!("{:?}", *filled),
            "page {page}"
        );
    }
}

/// The cache keeps the current page and one more, and evicts the oldest when a
/// third page is read.
#[test]
fn the_page_chars_cache_holds_two_pages() {
    assert_eq!(super::super::open::PAGE_CHARS_CACHE_ENTRIES, 2);
    let doc = PdfDocument::from_bytes(many_pages_pdf(3)).expect("open document");
    let first = doc.cached_page_chars(0).unwrap();
    let second = doc.cached_page_chars(1).unwrap();
    assert!(std::sync::Arc::ptr_eq(&first, &doc.cached_page_chars(0).unwrap()));
    assert!(std::sync::Arc::ptr_eq(&second, &doc.cached_page_chars(1).unwrap()));
    drop(doc.cached_page_chars(2).unwrap());
    assert_eq!(doc.page_chars_cache.lock().unwrap().len(), 2);
    let reread = doc.cached_page_chars(0).unwrap();
    assert!(!std::sync::Arc::ptr_eq(&first, &reread), "page 0 was evicted");
    assert_eq!(format!("{reread:?}"), format!("{first:?}"));
}

/// A cached page keeps no growth slack from extraction.
#[test]
fn a_cached_page_holds_no_spare_char_capacity() {
    let doc = PdfDocument::from_bytes(many_pages_pdf(1)).expect("open document");
    let chars = doc.cached_page_chars(0).unwrap();
    assert!(!chars.is_empty());
    assert_eq!(chars.capacity(), chars.len());
}
