use super::*;

#[cfg(all(feature = "pdf", feature = "ocr"))]
#[tokio::test]
#[serial_test::serial]
async fn should_keep_ocr_alternatives_in_the_structured_page_of_a_replaced_page() {
    use super::super::pipeline::extract_mixed_ocr_native;
    use crate::core::config::{ExtractionConfig, OcrConfig};
    use std::sync::Arc;

    const BACKEND: &str = "layer-test-structured-copies-backend";
    let misread: Vec<String> = HARBOUR_LAYER.iter().map(|line| misread_longest_word(line)).collect();
    crate::plugins::register_ocr_backend(Arc::new(StructuredBackend {
        name: BACKEND,
        text: misread.join("\n"),
    }))
    .unwrap();
    let single_backend = ExtractionConfig {
        use_cache: false,
        ocr: Some(OcrConfig {
            backend: BACKEND.to_string(),
            ..Default::default()
        }),
        ..Default::default()
    };
    let pipeline = ExtractionConfig {
        use_cache: false,
        ocr: Some(OcrConfig {
            pipeline: Some(zero_quality_pipeline(BACKEND)),
            ..Default::default()
        }),
        ..Default::default()
    };
    let pdf = lines_pdf(&HARBOUR_LAYER);
    let layer = HARBOUR_LAYER.join("\n");
    let boundary = [crate::types::PageBoundary {
        byte_start: 0,
        byte_end: layer.len(),
        page_number: 1,
    }];
    let mut results = Vec::new();
    for (route, config) in [("single backend", &single_backend), ("pipeline", &pipeline)] {
        let output = extract_mixed_ocr_native(&layer, &boundary, &[1], &pdf, config, None)
            .await
            .unwrap();
        results.push((route, output.1, output.2));
    }
    crate::plugins::unregister_ocr_backend(BACKEND).unwrap();

    for (route, replaced, structured) in results {
        assert!(
            replaced.contains_key(&1),
            "{route}: the OCR text must replace the page, or this test proves nothing"
        );
        let page = structured
            .get(&1)
            .unwrap_or_else(|| panic!("{route}: the replaced page has a structured document"));
        assert!(
            !page.elements.is_empty(),
            "{route}: the structured page holds the OCR lines, or this test proves nothing"
        );
        let text: Vec<&str> = page.elements.iter().map(|element| element.text.as_str()).collect();
        let text = text.join("\n");
        for line in HARBOUR_LAYER {
            assert_eq!(
                text.matches(line).count(),
                1,
                "{route}: layer line {line:?} once: {text:?}"
            );
        }
        for copy in &misread {
            assert_eq!(
                text.matches(copy.as_str()).count(),
                1,
                "{route}: the unproved OCR alternative stays: {text:?}"
            );
        }
    }
}

#[cfg(all(feature = "pdf", feature = "ocr"))]
#[tokio::test]
#[serial_test::serial]
async fn should_keep_ocr_alternatives_in_the_structured_page_of_a_whole_document_run() {
    use super::super::pipeline::{PageOcrHints, extract_with_ocr_with_page_hints};
    use crate::core::config::{ExtractionConfig, OcrConfig};
    use std::sync::Arc;

    const BACKEND: &str = "layer-test-structured-copies-whole-document-backend";
    let misread: Vec<String> = HARBOUR_LAYER.iter().map(|line| misread_longest_word(line)).collect();
    crate::plugins::register_ocr_backend(Arc::new(StructuredBackend {
        name: BACKEND,
        text: misread.join("\n"),
    }))
    .unwrap();
    let config = ExtractionConfig {
        use_cache: false,
        ocr: Some(OcrConfig {
            backend: BACKEND.to_string(),
            ..Default::default()
        }),
        ..Default::default()
    };
    let pdf = lines_pdf(&HARBOUR_LAYER);
    let hints = PageOcrHints {
        source_dpi: None,
        known_full_page_scan: false,
        single_block_pages: None,
        native_page_layers: Some(Arc::new(ahash::AHashMap::from([(1, HARBOUR_LAYER.join("\n"))]))),
        ..Default::default()
    };
    let output = extract_with_ocr_with_page_hints(
        Some(&pdf),
        None,
        #[cfg(feature = "layout-detection")]
        None,
        &config,
        None,
        Some(hints),
    )
    .await;
    crate::plugins::unregister_ocr_backend(BACKEND).unwrap();
    let output = output.expect("the whole-document run must return a result");

    let document = output.4.expect("the whole-document run returns a structured document");
    assert!(
        !document.elements.is_empty(),
        "the structured document holds the OCR lines, or this test proves nothing"
    );
    let structured: Vec<&str> = document.elements.iter().map(|element| element.text.as_str()).collect();
    for (route, text) in [("structured document", structured.join("\n")), ("flat text", output.0)] {
        for line in HARBOUR_LAYER {
            assert_eq!(
                text.matches(line).count(),
                1,
                "{route}: layer line {line:?} once: {text:?}"
            );
        }
        for copy in &misread {
            assert_eq!(
                text.matches(copy.as_str()).count(),
                1,
                "{route}: the unproved OCR alternative stays: {text:?}"
            );
        }
    }
}

/// A whole-document run on images renders no PDF page, so no side has line boxes: the layer lines
/// the OCR text lacks go first and every OCR line stays, misreads included (rule 3 of the merge:
/// a duplicate line is the accepted cost, a lost distinct line is not). ~keep
#[cfg(all(feature = "pdf", feature = "ocr"))]
#[tokio::test]
async fn should_keep_every_ocr_line_of_an_image_run_that_renders_no_pdf() {
    use super::super::pipeline::{PageOcrHints, extract_with_ocr_with_page_hints};
    use crate::core::config::{ExtractionConfig, OcrConfig};
    use std::sync::Arc;

    const BACKEND: &str = "layer-test-image-run-keeps-ocr-lines-backend";
    let misread: Vec<String> = HARBOUR_LAYER.iter().map(|line| misread_longest_word(line)).collect();
    crate::plugins::register_ocr_backend(Arc::new(StructuredBackend {
        name: BACKEND,
        text: misread.join("\n"),
    }))
    .unwrap();
    let config = ExtractionConfig {
        use_cache: false,
        ocr: Some(OcrConfig {
            backend: BACKEND.to_string(),
            ..Default::default()
        }),
        ..Default::default()
    };
    let images = [image::DynamicImage::new_rgb8(16, 16)];
    let hints = PageOcrHints {
        source_dpi: None,
        known_full_page_scan: false,
        single_block_pages: None,
        native_page_layers: Some(Arc::new(ahash::AHashMap::from([(1, HARBOUR_LAYER.join("\n"))]))),
        ..Default::default()
    };
    let output = extract_with_ocr_with_page_hints(
        None,
        Some(&images),
        #[cfg(feature = "layout-detection")]
        None,
        &config,
        None,
        Some(hints),
    )
    .await;
    crate::plugins::unregister_ocr_backend(BACKEND).unwrap();
    let output = output.expect("the image run must return a result");

    let document = output.4.expect("the image run returns a structured document");
    let structured: Vec<&str> = document.elements.iter().map(|element| element.text.as_str()).collect();
    for (route, text) in [("structured document", structured.join("\n")), ("flat text", output.0)] {
        for line in HARBOUR_LAYER {
            assert_eq!(
                text.matches(line).count(),
                1,
                "{route}: layer line {line:?} once: {text:?}"
            );
        }
        for copy in &misread {
            assert_eq!(
                text.matches(copy.as_str()).count(),
                1,
                "{route}: without boxes the OCR line {copy:?} stays: {text:?}"
            );
        }
    }
}

#[test]
fn should_preserve_both_ocr_alternatives_for_one_native_row() {
    use super::super::document::{layer_lines_to_put_first, text_with_layer_first};

    let thresholds = OcrQualityThresholds::default();
    let layer = HARBOUR_LAYER.join("\n");
    let early = misread_longest_word(HARBOUR_LAYER[5]);
    let late = HARBOUR_LAYER[5].replacen("library", "librarv", 1);
    let ocr_text = format!(
        "{}\n{early}\n{}\n{late}",
        HARBOUR_LAYER[..3].join("\n"),
        HARBOUR_LAYER[3..5].join("\n")
    );
    // The early misread sits on the layer line's row; the late one on a row of its own. ~keep
    let boxes = page_boxes(&HARBOUR_LAYER, &lines_at_rows(&ocr_text, &[0, 1, 2, 5, 3, 4, 6]));
    let first = layer_lines_to_put_first(&layer, &ocr_text, false, &thresholds, Some(&boxes))
        .expect("the OCR text confirms the layer and lacks its last line");
    let text = text_with_layer_first(&ocr_text, &first);
    assert!(text.contains(early.as_str()), "the first alternative stays: {text:?}");
    assert!(text.contains(late.as_str()), "the second misread stays: {text:?}");
    assert_eq!(
        text.matches(HARBOUR_LAYER[5]).count(),
        1,
        "the layer line once: {text:?}"
    );
}

/// A layer with one word with a letter among numbers that the OCR text reads differently (#2439):
/// one shared word does not confirm it, because the OCR text must hold half of all its words. The
/// same layer is confirmed when the OCR text holds half of its numbers too. ~keep
#[test]
fn should_not_confirm_a_layer_of_numbers_on_its_one_lettered_word() {
    use super::super::document::ocr_confirms_layer;

    let thresholds = OcrQualityThresholds::default();
    let layer = format!("Total\n{}", LEDGER_LAYER.join("\n"));
    let other_numbers: Vec<String> = LEDGER_LAYER
        .iter()
        .map(|line| line.split(' ').map(|n| format!("{n}9")).collect::<Vec<_>>().join(" "))
        .collect();
    let misread = format!("Total\n{}", other_numbers.join("\n"));
    assert!(
        !ocr_confirms_layer(&layer, &misread, &thresholds),
        "one lettered word of a layer of numbers does not confirm it"
    );
    let half = format!("Total\n{}", LEDGER_LAYER[..3].join("\n"));
    assert!(
        ocr_confirms_layer(&layer, &half, &thresholds),
        "the OCR text holds the word and half the numbers: confirmed"
    );
}

#[test]
fn should_preserve_ambiguous_copies_in_titles_headings_images_tables_and_text() {
    use super::super::document::put_layer_first_in_document;
    use crate::types::internal::{ElementKind, InternalDocument, InternalElement};

    let copy = "Copy line";
    let rows = format!("Region Total\n{copy}\nNorth 1204\nSouth 3381");
    let mut doc = InternalDocument::new("pdf");
    doc.tables.push(crate::types::Table {
        cells: vec![
            vec!["Region".into(), "Total".into()],
            vec!["Copy".into(), "line".into()],
            vec!["North".into(), "1204".into()],
        ],
        ..Default::default()
    });
    let kinds = [
        (ElementKind::Title, copy.to_string()),
        (ElementKind::Heading { level: 2 }, copy.to_string()),
        (ElementKind::Image { image_index: 0 }, copy.to_string()),
        (ElementKind::Table { table_index: 0 }, rows.clone()),
    ];
    for (kind, text) in kinds.iter().cloned() {
        doc.push_element(InternalElement::text(kind, text, 0));
    }
    doc.push_element(InternalElement::text(
        ElementKind::OcrText {
            level: crate::types::ocr_elements::OcrElementLevel::Block,
        },
        format!("{copy}\nRead line"),
        0,
    ));
    let tables_before = format!("{:?}", doc.tables);
    let first = layer_first_fixture("Layer line");
    put_layer_first_in_document(&mut doc, None, &first);
    assert_eq!(
        doc.elements.len(),
        6,
        "the layer element and every element: {:?}",
        doc.elements
    );
    assert_eq!(doc.elements[0].text, "Layer line");
    for (element, (kind, text)) in doc.elements[1..5].iter().zip(kinds.iter()) {
        assert_eq!(&element.kind, kind);
        assert_eq!(&element.text, text, "a {kind:?} keeps its text");
    }
    assert_eq!(
        doc.elements[5].text, "Copy line\nRead line",
        "ambiguous text occurrence stays"
    );
    assert_eq!(format!("{:?}", doc.tables), tables_before, "the table data stays");
}

/// Row A: a distinct OCR-only "Total 900" on its own row before the misread "Total 1OO" that sits
#[test]
fn should_keep_an_ocr_only_line_that_shares_a_label_with_a_misread_layer_line() {
    let mut ocr: Vec<&str> = MINUTES.to_vec();
    ocr[3] = "Total 1OO";
    ocr.insert(2, "Total 900");
    let page = merge_at_rows(&MINUTES, &ocr.join("\n"), &[0, 1, 9, 2, 3, 4, 5, 6, 7]);
    assert_eq!(
        (count_line(&page, "Total 900"), count_line(&page, "Total 1OO")),
        (1, 1),
        "{page:?}"
    );
    assert_eq!(count_line(&page, "Total 100"), 1, "{page:?}");
}

#[test]
fn should_keep_an_ocr_only_line_that_reorders_the_words_of_a_misread_layer_line() {
    let mut ocr: Vec<&str> = MINUTES.to_vec();
    ocr[2] = "Repairs to the north pler";
    ocr.insert(0, "the north pier to repairs");
    let page = merge_at_rows(&MINUTES, &ocr.join("\n"), &[9, 0, 1, 2, 3, 4, 5, 6, 7]);
    assert_eq!(count_line(&page, "the north pier to repairs"), 1, "{page:?}");
    assert_eq!(count_line(&page, "Repairs to the north pler"), 1, "{page:?}");
    assert_eq!(count_line(&page, "Repairs to the north pier"), 1, "{page:?}");
}

#[test]
fn should_restore_both_native_occurrences_and_keep_both_ocr_alternatives() {
    let mut layer: Vec<&str> = MINUTES.to_vec();
    layer.push("Total 100");
    let mut ocr = layer.clone();
    ocr[3] = "Total 1OO";
    ocr[8] = "Total 1O0";
    let page = merge_at_rows(&layer, &ocr.join("\n"), &[0, 1, 2, 3, 4, 5, 6, 7, 8]);
    let misreads = count_line(&page, "Total 1OO") + count_line(&page, "Total 1O0");
    assert_eq!((count_line(&page, "Total 100"), misreads), (2, 2), "{page:?}");
}

/// Row D: the layer holds "Total 100" and "Total 900"; OCR reads 900 exactly. Both totals once. ~keep
#[test]
fn should_keep_an_exactly_read_second_total_beside_a_misread_first() {
    let mut layer: Vec<&str> = MINUTES.to_vec();
    layer.insert(4, "Total 900");
    let mut ocr = layer.clone();
    ocr[3] = "Total 1OO";
    let page = merge_at_rows(&layer, &ocr.join("\n"), &[0, 1, 2, 3, 4, 5, 6, 7, 8]);
    let counts = (
        count_line(&page, "Total 100"),
        count_line(&page, "Total 900"),
        count_line(&page, "Total 1OO"),
    );
    assert_eq!(counts, (1, 1, 1), "{page:?}");
}

/// Row E: as row A with the OCR-only line after the misread. The rule does not depend on order. ~keep
#[test]
fn should_keep_an_ocr_only_line_after_the_misread_too() {
    let mut ocr: Vec<&str> = MINUTES.to_vec();
    ocr[3] = "Total 1OO";
    ocr.insert(6, "Total 900");
    let page = merge_at_rows(&MINUTES, &ocr.join("\n"), &[0, 1, 2, 3, 4, 5, 9, 6, 7]);
    assert_eq!(
        (count_line(&page, "Total 900"), count_line(&page, "Total 1OO")),
        (1, 1),
        "{page:?}"
    );
}

/// A misread on a far row is not owned: it stays beside the layer line (a duplicate, never a lost
/// line). A page without boxes removes nothing either. ~keep
#[test]
fn should_remove_nothing_without_ownership_or_without_boxes() {
    use super::super::document::layer_lines_to_put_first;
    let mut ocr: Vec<&str> = MINUTES.to_vec();
    ocr[3] = "Total 1OO";
    let page = merge_at_rows(&MINUTES, &ocr.join("\n"), &[0, 1, 2, 9, 4, 5, 6, 7]);
    assert_eq!(
        (count_line(&page, "Total 100"), count_line(&page, "Total 1OO")),
        (1, 1),
        "{page:?}"
    );
    let thresholds = OcrQualityThresholds::default();
    let first = layer_lines_to_put_first(&MINUTES.join("\n"), &ocr.join("\n"), false, &thresholds, None)
        .expect("the OCR text confirms the layer and lacks one line");
    assert_eq!(first.lines, "Total 100");
}

/// Ownership needs half of the shorter height and half of the shorter width. ~keep
#[test]
fn should_own_a_box_on_half_of_its_height_and_width() {
    use super::super::document::LineBox;
    let owner = LineBox {
        left: 0.0,
        top: 0.0,
        right: 100.0,
        bottom: 10.0,
    };
    let at = |left: f32, top: f32| LineBox {
        left,
        top,
        right: left + 100.0,
        bottom: top + 10.0,
    };
    assert!(owner.owns(&at(0.0, 5.0)), "half the height shared");
    assert!(!owner.owns(&at(0.0, 5.5)), "less than half the height");
    assert!(owner.owns(&at(50.0, 0.0)), "half the width shared");
    assert!(!owner.owns(&at(51.0, 0.0)), "less than half the width: another column");
}

/// The row index bounds the matching work: with N layer lines and N OCR lines on N rows, each
/// layer line meets its band only, and the merge of 2000 lines takes a small part of a second
/// (67 s at the PR head). The band of a line on row 500 holds its row and its neighbours. ~keep
#[test]
fn should_bound_the_matching_work_to_the_band_of_each_layer_line() {
    use super::super::document::{OcrRowIndex, layer_lines_to_put_first};
    let thresholds = OcrQualityThresholds::default();
    let mut ms_by_n = Vec::new();
    for n in [250usize, 2000] {
        for accepted in ["accepted", "accepled"] {
            let layer: Vec<String> = (1..=n)
                .map(|k| format!("Harbour record {k} repairs accepted tonight"))
                .collect();
            let ocr: Vec<String> = (1..=n)
                .map(|k| format!("Harbour record {k} repalrs {accepted} tonight"))
                .collect();
            let rows: Vec<usize> = (0..n).collect();
            let layer_refs: Vec<&str> = layer.iter().map(String::as_str).collect();
            let ocr_text = ocr.join("\n");
            let boxes = page_boxes(&layer_refs, &lines_at_rows(&ocr_text, &rows));
            let index = OcrRowIndex::new(&boxes.ocr);
            let band: Vec<usize> = index.band(&row_box(n / 2)).collect();
            assert!(
                band.len() <= 3 && band.contains(&(n / 2)),
                "the band holds the row and its neighbours: {band:?}"
            );
            let start = std::time::Instant::now();
            let first = layer_lines_to_put_first(&layer.join("\n"), &ocr_text, false, &thresholds, Some(&boxes))
                .expect("every layer line is missing");
            let ms = start.elapsed().as_millis();
            println!("SCALING n={n} missing={} ms={ms}", first.lines.lines().count());
            assert_eq!(first.lines.lines().count(), n, "every native row remains recoverable");
            assert!(super::super::document::text_with_layer_first(&ocr_text, &first).ends_with(&ocr_text));
            ms_by_n.push(ms);
        }
    }
    assert_eq!(ms_by_n.len(), 4);
    assert!(
        ms_by_n.iter().all(|ms| *ms < 2000),
        "bounded row fixtures: {ms_by_n:?} ms"
    );
}
