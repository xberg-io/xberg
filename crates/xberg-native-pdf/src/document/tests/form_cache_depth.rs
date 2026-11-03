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
        assert_eq!(root_form_is_cached(&document, 0), false, "truncated root, mode {mode}");
    }
}
