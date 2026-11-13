use super::super::PdfDocument;
use super::pdf_fixtures::build_catalog_test_pdf;

const FORM_COUNT: usize = 11;
const ANCESTOR_TEXT: &str = "AncestorSentinel";
const PARTIAL_TEXT: &str = "PartialSentinel";

fn stream(dictionary: &str, content: &str) -> Vec<u8> {
    format!(
        "<< {dictionary} /Length {} >>\nstream\n{content}\nendstream",
        content.len()
    )
    .into_bytes()
}

fn leaf_content() -> String {
    (0..20)
        .map(|line| format!("BT /F1 12 Tf 40 {} Td (LeafSentinel{line}) Tj ET\n", 600 - 16 * line))
        .collect()
}

fn nested_form_pdf(roots: &[usize]) -> Vec<u8> {
    nested_form_pdf_with_leaf(roots, &leaf_content())
}

fn nested_form_pdf_with_leaf(roots: &[usize], leaf: &str) -> Vec<u8> {
    let font = 3 + roots.len() as u32;
    let content_start = font + 1;
    let form_start = content_start + roots.len() as u32;
    let kids: String = (3..font).map(|page| format!("{page} 0 R ")).collect();
    let pages = format!("<< /Type /Pages /Kids [{kids}] /Count {} >>", roots.len());
    let mut objects = vec![(font, b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec())];
    for (page, root) in roots.iter().enumerate() {
        let content = content_start + page as u32;
        let form = form_start + *root as u32;
        objects.push((
            3 + page as u32,
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 600 800] /Contents {content} 0 R \
             /Resources << /XObject << /Root {form} 0 R >> >> >>"
            )
            .into_bytes(),
        ));
        objects.push((content, stream("", "/Root Do")));
    }
    for index in 0..FORM_COUNT {
        let next = form_start + index as u32 + 1;
        let mut resources = format!("/Font << /F1 {font} 0 R >>");
        if index + 1 < FORM_COUNT {
            resources.push_str(&format!(" /XObject << /Next {next} 0 R >>"));
        }
        let dictionary = format!("/Type /XObject /Subtype /Form /BBox [0 0 600 800] /Resources << {resources} >>");
        let content = match index {
            0 => format!("BT /F1 12 Tf 40 760 Td ({ANCESTOR_TEXT}) Tj ET /Next Do"),
            9 => format!("BT /F1 12 Tf 40 700 Td ({PARTIAL_TEXT}) Tj ET /Next Do"),
            10 => leaf.to_string(),
            _ => "/Next Do".to_string(),
        };
        objects.push((form_start + index as u32, stream(&dictionary, &content)));
    }
    let borrowed: Vec<_> = objects
        .iter()
        .map(|(number, bytes)| (*number, bytes.as_slice()))
        .collect();
    build_catalog_test_pdf(b"<< /Type /Catalog /Pages 2 0 R >>", pages.as_bytes(), &borrowed)
}

fn raw_span_text(document: &PdfDocument, page: usize) -> String {
    document
        .extract_spans_raw(page)
        .expect("extract nested forms")
        .into_iter()
        .map(|span| span.text)
        .collect()
}

fn public_text(document: &PdfDocument, page: usize, mode: &str) -> String {
    document.page_spans_cache.lock().unwrap().clear();
    let text = match mode {
        "spans" => document
            .extract_spans(page)
            .expect("extract public spans")
            .into_iter()
            .map(|span| span.text)
            .collect(),
        "text" => document.extract_text(page).expect("extract public text"),
        "config" => document
            .extract_spans_with_config(page, Default::default())
            .expect("extract configured spans")
            .into_iter()
            .map(|span| span.text)
            .collect(),
        "words" => document
            .extract_words_with_thresholds(page, None, None)
            .expect("extract public words")
            .into_iter()
            .map(|word| word.text)
            .collect(),
        _ => unreachable!(),
    };
    text.split_whitespace().collect()
}

fn root_form_is_cached(document: &PdfDocument, page_index: usize) -> bool {
    let page = document.get_page(page_index).expect("load actual page");
    let root = page.as_dict().unwrap()["Resources"].as_dict().unwrap()["XObject"]
        .as_dict()
        .unwrap()["Root"]
        .as_reference()
        .unwrap();
    let (llx, lly, urx, ury) = document.get_page_media_box(page_index).unwrap();
    let area = ((urx - llx) * (ury - lly)).abs();
    let key = (root, [1000, 0, 0, 1000, 0, 0], Some(area.to_bits()));
    document.xobject_spans_cache.lock().unwrap().get(&key).is_some()
}

fn complete_text() -> String {
    format!(
        "{PARTIAL_TEXT}{}",
        (0..20).map(|line| format!("LeafSentinel{line}")).collect::<String>()
    )
}

#[test]
fn depth_truncated_forms_preserve_partial_text_without_caching_ancestors() {
    let pdf = nested_form_pdf(&[0, 9, 8]);
    let document = PdfDocument::from_bytes(pdf.clone()).expect("open depth truncation regression");
    assert_eq!(document.page_count().unwrap(), 3);
    assert_eq!(raw_span_text(&document, 0), format!("{ANCESTOR_TEXT}{PARTIAL_TEXT}"));
    assert_eq!(document.xobject_spans_cache.lock().unwrap().len(), 0);
    for page in [1, 2] {
        let fresh = PdfDocument::from_bytes(pdf.clone()).expect("open fresh shallow control");
        assert_eq!(raw_span_text(&fresh, page), complete_text());
        assert_eq!(raw_span_text(&document, page), complete_text(), "shallow page {page}");
    }
    let chars: String = document
        .extract_chars(1)
        .expect("extract uncached character control")
        .into_iter()
        .map(|character| character.char)
        .collect();
    assert_eq!(chars, complete_text());
}

#[test]
fn complete_cached_forms_cannot_bypass_a_deeper_callers_depth_limit() {
    let pdf = nested_form_pdf(&[0, 9, 8]);
    for order in [[1, 0, 2], [1, 2, 0], [2, 1, 0], [0, 2, 1]] {
        let document = PdfDocument::from_bytes(pdf.clone()).expect("open cache order regression");
        for page in order.into_iter().chain(order) {
            let expected = if page == 0 {
                format!("{ANCESTOR_TEXT}{PARTIAL_TEXT}")
            } else {
                complete_text()
            };
            assert_eq!(raw_span_text(&document, page), expected, "page {page}, order {order:?}");
        }
        assert_eq!(document.xobject_spans_cache.lock().unwrap().len(), 3);
    }
}

#[test]
fn complete_nested_form_cache_reuses_spans_at_compatible_depths() {
    let document = PdfDocument::from_bytes(nested_form_pdf(&[9, 8])).expect("open nested reuse control");
    assert_eq!(raw_span_text(&document, 0), complete_text());
    assert_eq!(document.xobject_spans_cache.lock().unwrap().len(), 2);
    document.xobject_stream_cache.lock().unwrap().clear();
    assert_eq!(raw_span_text(&document, 1), complete_text());
    assert_eq!(document.xobject_spans_cache.lock().unwrap().len(), 3);
    assert_eq!(document.xobject_stream_cache.lock().unwrap().len(), 1);
    document.xobject_stream_cache.lock().unwrap().clear();
    for page in [0, 1] {
        assert_eq!(raw_span_text(&document, page), complete_text());
        assert_eq!(document.xobject_stream_cache.lock().unwrap().len(), 0);
    }
}

#[test]
fn complete_empty_nested_form_results_remain_cacheable() {
    let pdf = nested_form_pdf_with_leaf(&[10, 9], "BT /F1 12 Tf 40 600 Td () Tj ET");
    let document = PdfDocument::from_bytes(pdf).expect("open complete empty form control");
    assert_eq!(raw_span_text(&document, 0), "");
    assert_eq!(document.xobject_spans_cache.lock().unwrap().len(), 1);
    document.xobject_stream_cache.lock().unwrap().clear();
    assert_eq!(raw_span_text(&document, 1), PARTIAL_TEXT);
    assert_eq!(document.xobject_spans_cache.lock().unwrap().len(), 2);
    assert_eq!(document.xobject_stream_cache.lock().unwrap().len(), 1);
    document.xobject_stream_cache.lock().unwrap().clear();
    for (page, expected) in [(0, ""), (1, PARTIAL_TEXT)] {
        assert_eq!(raw_span_text(&document, page), expected);
        assert_eq!(document.xobject_stream_cache.lock().unwrap().len(), 0);
    }
}

#[test]
fn exact_depth_limit_forms_are_cacheable_but_one_extra_ancestor_still_truncates() {
    let document = PdfDocument::from_bytes(nested_form_pdf(&[1, 0])).expect("open depth boundary control");
    assert_eq!(raw_span_text(&document, 0), complete_text());
    assert_eq!(document.xobject_spans_cache.lock().unwrap().len(), 10);
    assert_eq!(raw_span_text(&document, 1), format!("{ANCESTOR_TEXT}{PARTIAL_TEXT}"));
    assert_eq!(document.xobject_spans_cache.lock().unwrap().len(), 10);
    document.xobject_stream_cache.lock().unwrap().clear();
    assert_eq!(raw_span_text(&document, 0), complete_text());
    assert_eq!(document.xobject_stream_cache.lock().unwrap().len(), 0);
}

#[test]
fn text_config_and_word_reads_cannot_cache_depth_truncated_form_spans() {
    let pdf = nested_form_pdf(&[0, 9]);
    for mode in ["spans", "text", "config", "words"] {
        let document = PdfDocument::from_bytes(pdf.clone()).expect("open alternate API regression");
        let expected = [format!("{ANCESTOR_TEXT}{PARTIAL_TEXT}"), complete_text()];
        for (page, text) in expected.iter().enumerate() {
            let fresh = PdfDocument::from_bytes(pdf.clone()).expect("open fresh public API control");
            assert_eq!(public_text(&fresh, page, mode), *text, "fresh page {page}, mode {mode}");
        }
        for page in [0, 1, 0, 1] {
            assert_eq!(
                public_text(&document, page, mode),
                expected[page],
                "warmed page {page}, mode {mode}"
            );
        }
        assert!(!root_form_is_cached(&document, 0), "truncated root, mode {mode}");
    }
}

fn placed_form_budget_pdf() -> Vec<u8> {
    let placements: String = (0..500)
        .map(|index| format!("q 1 0 0 1 {} {} cm /Root Do Q\n", 28 * (index % 20), 28 * (index / 20)))
        .collect();
    let probe = format!("BT /F1 12 Tf 20 760 Td (Prefix) Tj ET\n{placements}/Tail Do");
    let page = |content| {
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 600 800] /Contents {content} 0 R \
            /Resources << /Font << /F1 7 0 R >> /XObject << /Root 8 0 R /Tail 9 0 R >> >> >>"
        )
    };
    let form = "/Type /XObject /Subtype /Form /BBox [0 0 600 800] /Resources << /Font << /F1 7 0 R >> >>";
    build_catalog_test_pdf(
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>",
        &[
            (3, page(5).as_bytes()),
            (4, page(6).as_bytes()),
            (5, &stream("", &placements)),
            (6, &stream("", &probe)),
            (7, b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>"),
            (
                8,
                &stream(
                    form,
                    "BT /F1 12 Tf 4 16 Td [(A)] TJ /F1 8 Tf 8 4 Td [(B)] TJ ET \
                     BT /F1 12 Tf 4 4 Td [(C)] TJ ET",
                ),
            ),
            (9, &stream(form, "BT /F1 12 Tf 20 740 Td (BudgetTail) Tj ET")),
        ],
    )
}

#[test]
fn warm_form_spans_keep_script_sequence_gaps_without_spending_decode_budget() {
    let pdf = placed_form_budget_pdf();
    let extract = |document: &PdfDocument| {
        document
            .extract_page_text_with_script_geometry(1, super::super::ReadingOrder::TopToBottom)
            .expect("extract script geometry")
            .spans
    };
    let cold = PdfDocument::from_bytes(pdf.clone()).expect("open cold control");
    let cold_spans = extract(&cold);
    assert!(cold_spans.iter().any(|span| span.text == "C"));
    assert!(!cold_spans.iter().any(|span| span.text == "BudgetTail"));

    let warm = PdfDocument::from_bytes(pdf).expect("open cache replay regression");
    assert!(!raw_span_text(&warm, 0).is_empty());
    assert_eq!(warm.xobject_spans_cache.lock().unwrap().len(), 500);
    warm.xobject_stream_cache.lock().unwrap().clear();
    let mut spans = extract(&warm);
    assert!(spans.iter().any(|span| span.text == "BudgetTail"));
    assert_eq!(warm.xobject_stream_cache.lock().unwrap().len(), 1);

    spans.sort_unstable_by_key(|span| span.sequence);
    assert!(spans.windows(2).all(|pair| pair[0].sequence < pair[1].sequence));
    let [prefix, a, b, c, next_a, ..] = spans.as_slice() else {
        panic!("expected source spans from the caller and two Form placements");
    };
    assert_eq!(
        (
            prefix.text.as_str(),
            a.text.as_str(),
            b.text.as_str(),
            c.text.as_str(),
            next_a.text.as_str()
        ),
        ("Prefix", "A", "B", "C", "A")
    );
    assert!(a.sequence > prefix.sequence + 1);
    assert_eq!(b.sequence, a.sequence + 1);
    assert!(c.sequence > b.sequence + 1);
    assert!(next_a.sequence > c.sequence + 1);
}

fn clipped_nested_form_pdf(probe: &str, child_invocation: &str) -> Vec<u8> {
    let page = |content| {
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 600 800] /Contents {content} 0 R \
             /Resources << /XObject << /ParentForm 8 0 R /ChildForm 9 0 R >> >> >>"
        )
    };
    let parent = stream(
        "/Type /XObject /Subtype /Form /BBox [0 0 100 100] \
         /Resources << /Font << /F1 7 0 R >> /XObject << /ChildForm 9 0 R >> >>",
        &format!("BT /F1 12 Tf 20 60 Td (ParentPaint) Tj ET {child_invocation}"),
    );
    let child = stream(
        "/Type /XObject /Subtype /Form /BBox [0 0 600 800] /Resources << /Font << /F1 7 0 R >> >>",
        "BT /F1 12 Tf 20 220 Td (ChildPaint) Tj ET",
    );
    build_catalog_test_pdf(
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>",
        &[
            (3, page(5).as_bytes()),
            (4, page(6).as_bytes()),
            (5, &stream("", "/ParentForm Do")),
            (6, &stream("", probe)),
            (7, b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>"),
            (8, &parent),
            (9, &child),
        ],
    )
}

#[test]
fn cached_parent_reserves_clipped_child_placement_without_suppressing_translations() {
    let pdf = clipped_nested_form_pdf(
        "/ParentForm Do /ChildForm Do q 1 0 0 1 160 0 cm /ChildForm Do Q",
        "/ChildForm Do",
    );
    let cold = PdfDocument::from_bytes(pdf.clone()).expect("open cold placement control");
    assert_eq!(raw_span_text(&cold, 1), "ChildPaintParentPaint");

    let warm = PdfDocument::from_bytes(pdf).expect("open warm placement control");
    assert_eq!(raw_span_text(&warm, 0), "ParentPaint");
    assert_eq!(warm.xobject_spans_cache.lock().unwrap().len(), 2);
    warm.xobject_stream_cache.lock().unwrap().clear();
    assert_eq!(raw_span_text(&warm, 1), "ChildPaintParentPaint");
    assert_eq!(warm.xobject_stream_cache.lock().unwrap().len(), 1);
}

#[test]
fn cached_parent_walks_again_when_direct_child_has_already_painted() {
    let pdf = clipped_nested_form_pdf("/ChildForm Do /ParentForm Do", "/ChildForm Do");
    let cold = PdfDocument::from_bytes(pdf.clone()).expect("open cold placement control");
    assert_eq!(raw_span_text(&cold, 1), "ChildPaintParentPaint");

    let warm = PdfDocument::from_bytes(pdf).expect("open warm placement control");
    assert_eq!(raw_span_text(&warm, 0), "ParentPaint");
    assert_eq!(warm.xobject_spans_cache.lock().unwrap().len(), 2);
    warm.xobject_stream_cache.lock().unwrap().clear();
    assert_eq!(raw_span_text(&warm, 1), "ChildPaintParentPaint");
    assert!(
        warm.xobject_stream_cache
            .lock()
            .unwrap()
            .get(&crate::object::ObjectRef::new(8, 0))
            .is_some(),
        "the parent must walk again when its child placement already exists"
    );
}

#[test]
fn cached_parent_uses_exact_caller_transform_for_rounded_child_placements() {
    let parent = "q 1 0 0 1 0.0004 0 cm /ParentForm Do Q";
    let nested_child = "q 1 0 0 1 0.0004 0 cm /ChildForm Do Q";
    let direct_child = "q 1 0 0 1 0.0008 0 cm /ChildForm Do Q";

    for (probe, expected) in [
        (format!("{parent} {direct_child}"), "ParentPaint"),
        (format!("{direct_child} {parent}"), "ChildPaintParentPaint"),
    ] {
        let pdf = clipped_nested_form_pdf(&probe, nested_child);
        let cold = PdfDocument::from_bytes(pdf.clone()).expect("open cold rounding control");
        assert_eq!(raw_span_text(&cold, 1), expected, "{probe}");

        let warm = PdfDocument::from_bytes(pdf).expect("open warm rounding control");
        assert_eq!(raw_span_text(&warm, 0), "ParentPaint");
        assert_eq!(warm.xobject_spans_cache.lock().unwrap().len(), 2);
        warm.xobject_stream_cache.lock().unwrap().clear();
        assert_eq!(raw_span_text(&warm, 1), expected, "{probe}");
        assert!(
            warm.xobject_stream_cache
                .lock()
                .unwrap()
                .get(&crate::object::ObjectRef::new(8, 0))
                .is_some(),
            "the parent must walk with the shifted caller transform: {probe}"
        );
    }
}
