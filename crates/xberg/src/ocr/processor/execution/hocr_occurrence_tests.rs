use super::*;
use crate::ocr::hocr_parser::parse_hocr_to_internal_document_with_page_offset_and_stats;

#[test]
fn hocr_receipts_use_selected_tags_and_exact_utf8_byte_ranges() {
    let hocr = r#"<div class="ocr_page" title="ppageno 0">
      <span class="ocrx_word" id="omitted" title="bbox 10 10 45 25">Same</span>
      <p class="ocr_par">
        <span class="ocr_line">
          <span class="ocrx_word" id="kept" title="bbox 70 50 100 65">Same</span>
          <span class="ocrx_word" id="accent" title="bbox 110 50 140 65">café</span>
        </span>
        <span class="ocr_line">
          <span class="ocrx_word" id="continuation" title="bbox 70 75 100 90">line</span>
        </span>
      </p>
      <p class="ocr_par">
        <span class="ocr_line">
          <span class="ocrx_word" id="footer" title="bbox 70 100 100 115">footer</span>
        </span>
      </p>
    </div>"#;
    let stock = "Same café\nline\n\nfooter";
    let default = parse_hocr_to_internal_document_with_page_offset_and_stats(hocr, None, 1);
    assert_eq!(flatten_hocr_elements_to_text(&default.document.elements), stock);
    let receipts = serialize_hocr_occurrences(hocr, None, 1, stock).unwrap();
    let ids: Vec<_> = receipts.iter().map(|word| word.id).collect();
    let tag = |name| hocr.find(name).unwrap() - "<span class=\"ocrx_word\" id=\"".len();
    assert_eq!(ids, [tag("kept"), tag("accent"), tag("continuation"), tag("footer")]);
    assert!(!ids.contains(&tag("omitted")));
    assert_eq!(receipts[0].bounds, [70, 50, 100, 65]);
    assert_eq!(
        receipts
            .iter()
            .map(|word| (word.bytes.clone(), &stock[word.bytes.clone()]))
            .collect::<Vec<_>>(),
        [(0..4, "Same"), (5..10, "café"), (11..15, "line"), (17..23, "footer")]
    );
    assert!(serialize_hocr_occurrences(hocr, None, 1, "Same café\nline\n\nother").is_none());
}

#[test]
fn hocr_receipts_omit_dictionary_filtered_words() {
    let hocr = r#"<p class="ocr_par">
      <span class="ocr_line">
        <span class="ocrx_word" title="bbox 10 10 40 20">readable</span>
        <span class="ocrx_word" title="bbox 50 10 80 20">heading</span>
      </span>
      <span class="ocr_line">
        <span class="ocrx_word" title="bbox 10 30 40 40">OWATS</span>
        <span class="ocrx_word" title="bbox 50 30 80 40">DNDEVET</span>
        <span class="ocrx_word" title="bbox 90 30 120 40">OPMENT</span>
      </span>
    </p>"#;
    let filter = DictionaryLineFilter {
        is_valid_word: &|word| Some(matches!(word, "readable" | "heading")),
        max_invalid_ratio: 0.6,
    };
    let stock = "readable heading";
    let receipts = serialize_hocr_occurrences(hocr, Some(&filter), 1, stock).unwrap();
    assert_eq!(receipts.len(), 2);
    assert_eq!(&stock[receipts[0].bytes.clone()], "readable");
    assert_eq!(&stock[receipts[1].bytes.clone()], "heading");
    assert!(receipts.iter().all(|word| word.bounds[1] == 10));
}

#[test]
fn retained_hocr_receipt_rejects_same_text_from_a_different_element_owner() {
    let html = r#"<p class="ocr_par"><span class="ocr_line">
        <span class="ocrx_word" title="bbox 10 10 45 25">Same</span>
    </span></p>"#;
    let (parsed, words) = crate::ocr::hocr_parser::parse_hocr_with_occurrences(html, None, 1);
    let dimensions = (100, 100);
    let read = capture_retained_hocr_read(&parsed.document.elements, words.unwrap(), dimensions).unwrap();
    assert!(read.matches("Same", dimensions, &parsed.document));
    assert!(!read.matches("Same", (101, 100), &parsed.document));
    let mut changed = parsed.document.clone();
    changed.elements[0] = changed.elements[0].clone().with_index(42);
    assert_eq!(flatten_hocr_elements_to_text(&changed.elements), "Same");
    assert!(!read.matches("Same", dimensions, &changed));
}

#[test]
fn inline_receipt_uses_unique_geometry_and_declines_ambiguous_tsv_owners() {
    let html = r#"<p class="ocr_par"><span class="ocr_line">
        <span class="ocrx_word" title="bbox 30 40 50 60">word</span>
    </span></p>"#;
    let (parsed, words) = crate::ocr::hocr_parser::parse_hocr_with_occurrences(html, None, 1);
    let read = capture_retained_hocr_read(&parsed.document.elements, words.unwrap(), (100, 100)).unwrap();
    let header = "level\tpage_num\tblock_num\tpar_num\tline_num\tword_num\tleft\ttop\twidth\theight\tconf\ttext\n";
    let matching = "5\t1\t1\t1\t1\t1\t30\t40\t20\t20\t90\tword\n";
    let other = "5\t1\t1\t1\t1\t2\t60\t40\t20\t20\t90\tword\n";
    let retained = inline_hocr_receipt(&format!("{header}{matching}{other}"), &[], 0.0, &read).unwrap();
    assert_eq!(retained.text, "word word");
    assert_eq!(retained.ranges, [(read.occurrences[0].id, 0..4)]);
    assert!(inline_hocr_receipt(&format!("{header}{matching}{matching}"), &[], 0.0, &read).is_none());
    assert!(inline_hocr_receipt(&format!("{header}{other}"), &[], 0.0, &read).is_none());
}
