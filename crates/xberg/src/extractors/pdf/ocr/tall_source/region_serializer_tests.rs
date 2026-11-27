use super::*;
use crate::ocr::processor::render_inline_ocr_table_content;
use crate::types::OcrTableBoundingBox;

fn bounds([left, top, right, bottom]: [u32; 4]) -> RegionBox {
    RegionBox {
        left,
        top,
        right,
        bottom,
    }
}

fn positioned(text: &str, b: [u32; 4]) -> HocrWord {
    let b = bounds(b);
    HocrWord {
        text: text.into(),
        left: b.left,
        top: b.top,
        width: b.right - b.left,
        height: b.bottom - b.top,
        confidence: 95.0,
    }
}

fn source(text: &str, b: [u32; 4]) -> SourceWord<'_> {
    SourceWord {
        text,
        bounds: bounds(b),
        confidence: 95.0,
    }
}

fn table(text: &str, b: [u32; 4]) -> OcrTable {
    let b = bounds(b);
    OcrTable {
        cells: vec![vec![text.into()]],
        markdown: text.into(),
        page_number: 1,
        bounding_box: Some(OcrTableBoundingBox {
            left: b.left,
            top: b.top,
            right: b.right,
            bottom: b.bottom,
        }),
    }
}

fn apply(transcript: &str, edits: &[OccurrenceEdit<'_>]) -> String {
    let mut revised = transcript.to_owned();
    for edit in edits.iter().rev() {
        assert_eq!(transcript.get(edit.bytes.clone()), Some(edit.expected));
        revised.replace_range(edit.bytes.clone(), edit.replacement);
    }
    revised
}

#[test]
fn serializer_joins_adjacent_cjk_words_and_reports_their_unspaced_byte_ranges() {
    let words = [
        positioned("停车", [100, 100, 160, 130]),
        positioned("区", [162, 100, 190, 130]),
        positioned("7.5", [200, 100, 240, 130]),
        positioned("指标", [300, 100, 360, 130]),
    ];
    let mut seen = Vec::new();

    let transcript =
        render_inline_ocr_table_content_with_occurrences(&words, &[], &mut |id, range| seen.push((id, range)));

    assert_eq!(transcript, "停车区 7.5 指标");
    assert_eq!(seen, [(0, 0..6), (1, 6..9), (2, 10..13), (3, 14..20)]);
    for (id, range) in seen {
        assert_eq!(transcript.get(range), Some(words[id].text.as_str()));
    }
}

#[test]
fn serializer_omits_table_owned_occurrences_instead_of_claiming_a_byte_valid_lookalike() {
    let boxes = [
        [205, 155, 251, 173],
        [262, 155, 354, 173],
        [205, 400, 251, 418],
        [262, 400, 354, 418],
    ];
    let words = [
        positioned("POP", boxes[0]),
        positioned("WAcES:", boxes[1]),
        positioned("POP", boxes[2]),
        positioned("WAcES:", boxes[3]),
    ];
    let tables = [table("table cells", [190, 150, 360, 180])];
    let source_bounds = boxes.map(bounds);
    let transcript = "table cells\n\nPOP WAcES:";
    let mut seen = Vec::new();
    assert_eq!(
        render_inline_ocr_table_content_with_occurrences(&words, &tables, &mut |id, range| seen.push((id, range))),
        transcript,
    );
    assert_eq!(seen, [(2, 13..16), (3, 17..23)]);
    let stock = InlineStock {
        words: &words,
        source_bounds: &source_bounds,
        tables: &tables,
        unowned_suffix: "",
        transcript,
    };
    let reads = [
        source("PDP", [206, 157, 251, 171]),
        source("IMAGES", [265, 157, 353, 171]),
    ];
    let target = bounds([203, 154, 355, 174]);
    assert!(stock.plan(target, target, &[0, 1], [&reads, &reads], &[]).is_none());
    assert!(stock.plan(target, target, &[2, 3], [&reads, &reads], &[]).is_none());

    let unfiltered = InlineStock {
        tables: &[],
        transcript: "POP WAcES:\n\nPOP WAcES:",
        ..stock
    };
    let edits = unfiltered.plan(target, target, &[0, 1], [&reads, &reads], &[]).unwrap();
    assert_eq!(apply(unfiltered.transcript, &edits), "PDP IMAGES\n\nPOP WAcES:");
}

#[test]
fn serializer_plan_keeps_the_other_column_three_tables_and_annotation_suffix_intact() {
    let boxes = [
        [320, 5455, 337, 5469],
        [368, 5457, 387, 5469],
        [402, 5457, 439, 5474],
        [906, 5455, 942, 5469],
        [943, 5455, 978, 5469],
    ];
    let words = [
        positioned("wi", boxes[0]),
        positioned("the", boxes[1]),
        positioned("blog", boxes[2]),
        positioned("FIRST", boxes[3]),
        positioned("NAME", boxes[4]),
    ];
    let source_bounds = boxes.map(bounds);
    let tables = [
        table("table one", [306, 358, 1229, 388]),
        table("table two", [306, 945, 1229, 975]),
        table("table three", [306, 1229, 1392, 1405]),
    ];
    let annotation = "\n\nAnnotations:\nwi";
    let body = render_inline_ocr_table_content(&words, &tables, None);
    assert_eq!(body, "table one\n\ntable two\n\ntable three\n\nwi the blog FIRST NAME");
    let transcript = format!("{body}{annotation}");
    let stock = InlineStock {
        words: &words,
        source_bounds: &source_bounds,
        tables: &tables,
        unowned_suffix: annotation,
        transcript: &transcript,
    };
    let reads = [
        source("View", [321, 5456, 364, 5469]),
        source("the", [369, 5457, 397, 5469]),
        source("blog", [402, 5457, 438, 5472]),
    ];
    let protected = [
        bounds([306, 358, 1229, 388]),
        bounds([306, 945, 1229, 975]),
        bounds([306, 1229, 1392, 1405]),
        source_bounds[3],
        source_bounds[4],
    ];
    let region = bounds([319, 5454, 440, 5475]);
    let column = bounds([0, 5287, 742, 5773]);
    let mut document = transcript.clone();
    let mut page = transcript.clone();
    assert!(stock.reconcile_plain(
        &mut document,
        &mut page,
        SelectedLine {
            region,
            column,
            occurrences: &[0, 1, 2],
            sources: [&reads, &reads],
            protected: &protected,
        }
    ));
    assert_eq!(
        document,
        "table one\n\ntable two\n\ntable three\n\nView the blog FIRST NAME\n\nAnnotations:\nwi"
    );
    assert_eq!(page, document);
    assert!(
        stock
            .plan(
                bounds([319, 5454, 906, 5475]),
                column,
                &[0, 1, 2],
                [&reads, &reads],
                &protected
            )
            .is_none()
    );
    let stale = InlineStock {
        transcript: &body,
        ..stock
    };
    assert!(
        stale
            .plan(region, column, &[0, 1, 2], [&reads, &reads], &protected)
            .is_none()
    );
}

#[test]
fn verified_plain_transaction_is_atomic_on_a_stale_page_and_preserves_the_suffix() {
    let boxes = [[205, 155, 251, 173], [262, 155, 354, 173], [360, 210, 390, 230]];
    let words = [
        positioned("POP", boxes[0]),
        positioned("WAcES:", boxes[1]),
        positioned("Hero", boxes[2]),
    ];
    let transcript = "POP WAcES:\n\nHero\n\nAnnotations:\nPOP WAcES:";
    let unowned_suffix = "\n\nAnnotations:\nPOP WAcES:";
    let regions = boxes.map(bounds);
    let stock = InlineStock {
        words: &words,
        source_bounds: &regions,
        tables: &[],
        unowned_suffix,
        transcript,
    };
    let reads = [
        source("PDP", [206, 157, 251, 171]),
        source("IMAGES", [265, 157, 353, 171]),
    ];
    let column = bounds([175, 132, 742, 283]);
    let protected = [regions[2]];
    let selected = || SelectedLine {
        region: bounds([203, 154, 355, 174]),
        column,
        occurrences: &[0, 1],
        sources: [&reads, &reads],
        protected: &protected,
    };
    let mut document = transcript.to_owned();
    let mut stale = "unrelated page".to_owned();
    assert!(!stock.reconcile_plain(&mut document, &mut stale, selected()));
    assert_eq!(document, transcript);
    assert_eq!(stale, "unrelated page");
    let mut page = transcript.to_owned();
    assert!(stock.reconcile_plain(&mut document, &mut page, selected()));
    assert_eq!(document, "PDP IMAGES\n\nHero\n\nAnnotations:\nPOP WAcES:");
    assert_eq!(page, document);
}

#[test]
fn hocr_and_inline_projections_publish_the_same_occurrence_with_distinct_separators() {
    let boxes = [[320, 5455, 337, 5469], [368, 5457, 387, 5469], [402, 5457, 439, 5474]];
    let words = [
        positioned("wi", boxes[0]),
        positioned("the", boxes[1]),
        positioned("blog", boxes[2]),
    ];
    let source_bounds = boxes.map(bounds);
    let html = r#"<p class="ocr_par"><span class="ocr_line">
        <span class="ocrx_word" title="bbox 320 5455 337 5469">wi</span>
    </span></p><p class="ocr_par"><span class="ocr_line">
        <span class="ocrx_word" title="bbox 368 5457 387 5469">the</span>
        <span class="ocrx_word" title="bbox 402 5457 439 5474">blog</span>
    </span></p>"#;
    let annotation = "\n\nAnnotations:\nwi";
    let stock_document = "wi\n\nthe blog\n\nAnnotations:\nwi";
    let stock_page = "table one\n\ntable two\n\ntable three\n\nwi the blog\n\nAnnotations:\nwi";
    let tables = [
        table("table one", [306, 358, 1229, 388]),
        table("table two", [306, 945, 1229, 975]),
        table("table three", [306, 1229, 1392, 1405]),
    ];
    let hocr = HocrStock {
        html,
        unowned_suffix: annotation,
        transcript: stock_document,
    };
    let inline = InlineStock {
        words: &words,
        source_bounds: &source_bounds,
        tables: &tables,
        unowned_suffix: annotation,
        transcript: stock_page,
    };
    let reads = [
        source("View", [321, 5456, 364, 5469]),
        source("the", [369, 5457, 397, 5469]),
        source("blog", [402, 5457, 438, 5472]),
    ];
    let selected = || SelectedLine {
        region: bounds([319, 5454, 440, 5475]),
        column: bounds([0, 5287, 742, 5773]),
        occurrences: &[0, 1, 2],
        sources: [&reads, &reads],
        protected: &[],
    };
    let mut doc = stock_document.to_owned();
    let mut page = stock_page.to_owned();
    let other = HocrStock {
        html: &html.replace("320 5455 337 5469", "321 5455 338 5469"),
        ..hocr
    };
    assert!(
        other
            .reconcile_with_inline(&inline, &mut doc, &mut page, selected())
            .is_none()
    );
    assert_eq!(doc, stock_document);
    assert_eq!(page, stock_page);
    hocr.reconcile_with_inline(&inline, &mut doc, &mut page, selected())
        .unwrap();
    assert_eq!(doc, "View\n\nthe blog\n\nAnnotations:\nwi");
    assert_eq!(
        page,
        "table one\n\ntable two\n\ntable three\n\nView the blog\n\nAnnotations:\nwi"
    );
}

#[test]
fn atomic_publication_declines_different_selected_owners_with_identical_text() {
    let document_stock = "wi\n\nwi the blog";
    let page_stock = "wi the blog\n\nwi";
    let document_edit = OccurrenceEdit {
        selection_index: 0,
        bytes: 0..2,
        expected: "wi",
        replacement: "View",
    };
    let page_edit = OccurrenceEdit {
        selection_index: 1,
        bytes: 0..2,
        expected: "wi",
        replacement: "View",
    };
    let mut document = document_stock.to_string();
    let mut page = page_stock.to_string();
    assert!(
        publish_plain_pair(
            &mut document,
            &mut page,
            (document_stock, &[document_edit]),
            (page_stock, &[page_edit]),
        )
        .is_none()
    );
    assert_eq!(document, document_stock);
    assert_eq!(page, page_stock);
}
