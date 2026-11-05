use super::super::PdfDocument;
use super::pdf_fixtures::build_catalog_test_pdf;

const TEXT: &str = "CacheSentinel";
const PAGE_WIDTH: u32 = 600;
const WRAPPER_PAGE_HEIGHT: u32 = 120;
const FIGURE_PAGE_HEIGHT: u32 = 800;

fn shared_form_pdf(page_heights: [u32; 2]) -> Vec<u8> {
    let pages = page_heights.map(|height| {
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {PAGE_WIDTH} {height}] \
             /Contents 5 0 R /Resources << /XObject << /Shared 6 0 R >> >> >>"
        )
    });
    let paint = "BT /F1 12 Tf 410 60 Td (CacheSentinel) Tj ET";
    let form = format!(
        "<< /Type /XObject /Subtype /Form /BBox [0 0 400 120] \
         /Resources << /Font << /F1 7 0 R >> >> /Length {} >>\nstream\n{paint}\nendstream",
        paint.len()
    );
    build_catalog_test_pdf(
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>",
        &[
            (3, pages[0].as_bytes()),
            (4, pages[1].as_bytes()),
            (5, b"<< /Length 10 >>\nstream\n/Shared Do\nendstream"),
            (6, form.as_bytes()),
            (
                7,
                b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>",
            ),
        ],
    )
}

fn span_text(document: &PdfDocument, page: usize) -> String {
    document
        .extract_spans(page)
        .expect("extract actual shared-form page")
        .iter()
        .map(|span| span.text.as_str())
        .collect()
}

fn assert_page_context_survives_cache_warm(page_heights: [u32; 2]) {
    let pdf = shared_form_pdf(page_heights);
    let expected = page_heights.map(|height| if height == WRAPPER_PAGE_HEIGHT { TEXT } else { "" });
    for (page, height) in page_heights.into_iter().enumerate() {
        let fresh = PdfDocument::from_bytes(pdf.clone()).expect("open independent fresh-page control");
        assert_eq!(fresh.page_count().unwrap(), 2);
        assert_eq!(
            fresh.get_page_media_box(page).unwrap(),
            (0.0, 0.0, PAGE_WIDTH as f32, height as f32)
        );
        assert_eq!(span_text(&fresh, page), expected[page], "fresh page {page}");
    }

    for order in [[0, 1], [1, 0]] {
        let warmed = PdfDocument::from_bytes(pdf.clone()).expect("open mixed-page cache regression");
        for page in order {
            assert_eq!(span_text(&warmed, page), expected[page], "page {page}, order {order:?}");
        }
        assert_eq!(warmed.xobject_spans_cache.lock().unwrap().len(), 2);
        for page in order {
            // Bypass completed page spans so this read exercises the warm Form cache. ~keep
            warmed.page_spans_cache.lock().unwrap().clear();
            assert_eq!(
                span_text(&warmed, page),
                expected[page],
                "cached page {page}, order {order:?}"
            );
        }
        assert_eq!(warmed.xobject_spans_cache.lock().unwrap().len(), 2);
    }
}

#[test]
fn wrapper_first_form_spans_keep_each_destination_pages_clipping_policy() {
    assert_page_context_survives_cache_warm([WRAPPER_PAGE_HEIGHT, FIGURE_PAGE_HEIGHT]);
}

#[test]
fn figure_first_form_spans_keep_each_destination_pages_clipping_policy() {
    assert_page_context_survives_cache_warm([FIGURE_PAGE_HEIGHT, WRAPPER_PAGE_HEIGHT]);
}

#[test]
fn equal_page_contexts_reuse_cached_form_spans_without_losing_text() {
    let pdf = shared_form_pdf([WRAPPER_PAGE_HEIGHT; 2]);
    let document = PdfDocument::from_bytes(pdf).expect("open equal-page cache control");
    for page in [0, 1, 0, 1] {
        // Bypass completed page spans so repeated reads exercise the warm Form cache. ~keep
        document.page_spans_cache.lock().unwrap().clear();
        assert_eq!(span_text(&document, page), TEXT, "page {page}");
        assert_eq!(document.xobject_spans_cache.lock().unwrap().len(), 1);
    }
}

fn inherited_cmap_form_pdf(second_donor: u32) -> Vec<u8> {
    use super::pdf_fixtures::build_truetype_with_cmap_format4;

    let page = |font_id| {
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Contents 5 0 R \
             /Resources << /Font << /PageDonor {font_id} 0 R >> /XObject << /Shared 6 0 R >> >> >>"
        )
    };
    let font = |name, descriptor_id| {
        format!("<< /Type /Font /Subtype /TrueType /BaseFont /{name} /FontDescriptor {descriptor_id} 0 R >>")
    };
    let program = |mappings: &[(u16, u16)]| {
        let cmap = build_truetype_with_cmap_format4(mappings);
        let mut stream = format!("<< /Length {} >>\nstream\n", cmap.len()).into_bytes();
        stream.extend(cmap);
        stream.extend(b"\nendstream");
        stream
    };
    let paint = "BT /F2 12 Tf 20 100 Td <0003> Tj ET";
    let form = format!(
        "<< /Type /XObject /Subtype /Form /BBox [0 0 200 200] /Resources << /Font 8 0 R >> \
         /Length {} >>\nstream\n{paint}\nendstream",
        paint.len()
    );
    build_catalog_test_pdf(
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>",
        &[
            (3, page(7).as_bytes()),
            (4, page(second_donor).as_bytes()),
            (5, b"<< /Length 10 >>\nstream\n/Shared Do\nendstream"),
            (6, form.as_bytes()),
            (7, font("MNOPQR+ScopeDonor", 11).as_bytes()),
            (8, b"<< /F1 9 0 R /F2 10 0 R >>"),
            (9, font("ABCDEF+ScopeDonor", 13).as_bytes()),
            (10, b"<< /Type /Font /Subtype /Type0 /BaseFont /GHIJKL+ScopeDonor /Encoding /Identity-H /DescendantFonts [15 0 R] >>"),
            (11, b"<< /Type /FontDescriptor /FontName /MNOPQR+ScopeDonor /Flags 32 /FontFile2 12 0 R >>"),
            (12, &program(&[(0x42, 3), (0x43, 4)])),
            (13, b"<< /Type /FontDescriptor /FontName /ABCDEF+ScopeDonor /Flags 32 /FontFile2 14 0 R >>"),
            (14, &program(&[(0x41, 3)])),
            (15, b"<< /Type /Font /Subtype /CIDFontType2 /BaseFont /GHIJKL+ScopeDonor /CIDToGIDMap /Identity /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /DW 600 >>"),
            (16, font("UVWXYZ+ScopeDonor", 17).as_bytes()),
            (17, b"<< /Type /FontDescriptor /FontName /UVWXYZ+ScopeDonor /Flags 32 /FontFile2 18 0 R >>"),
            (18, &program(&[(0x43, 3), (0x44, 4)])),
        ],
    )
}

#[test]
fn cached_form_spans_keep_each_enclosing_cmap_donor() {
    let pdf = inherited_cmap_form_pdf(16);
    let expected = ["B", "C"];
    for (page, glyph) in expected.iter().enumerate() {
        let fresh = PdfDocument::from_bytes(pdf.clone()).expect("open fresh-page control");
        assert_eq!(fresh.page_count().expect("page count"), 2);
        assert_eq!(span_text(&fresh, page), *glyph, "fresh page {page}");
    }

    for order in [[0, 1], [1, 0]] {
        let doc = PdfDocument::from_bytes(pdf.clone()).expect("open shared form");
        for page in order.into_iter().chain(order) {
            doc.page_spans_cache.lock().unwrap().clear();
            assert_eq!(span_text(&doc, page), expected[page], "page {page}, order {order:?}");
        }
    }
}

#[test]
fn equal_nonempty_font_scopes_reuse_the_same_cached_form_spans() {
    use super::super::ObjectRef;

    let doc = PdfDocument::from_bytes(inherited_cmap_form_pdf(7)).expect("open shared font scope");
    let key = (
        ObjectRef::new(6, 0),
        [1000, 0, 0, 1000, 0, 0],
        Some(40_000f32.to_bits()),
    );
    let cached_span_allocation = || {
        let mut cache = doc.xobject_spans_cache.lock().unwrap();
        let cached = cache.get(&key).expect("cached shared form");
        let spans = cached.spans.as_ref().expect("cached form text");
        assert_eq!(spans[0].text, "B");
        spans.as_ptr()
    };
    assert_eq!(span_text(&doc, 0), "B");
    let first = cached_span_allocation();
    assert_eq!(span_text(&doc, 1), "B");
    assert_eq!(cached_span_allocation(), first, "warm read must reuse the stored spans");
}

fn shadowed_selected_font_pdf(form_glyph: &str) -> Vec<u8> {
    let page = |font| {
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Contents 5 0 R \
         /Resources << /Font << /F1 {font} 0 R /F2 9 0 R >> /XObject << /Shared 6 0 R >> >> >>"
        )
    };
    let stream = |content: &str| format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len());
    let form_text = "BT /F1 13 Tf 20 110 Td (A) Tj /F2 12 Tf 40 0 Td (Own) Tj ET";
    let form = format!(
        "<< /Type /XObject /Subtype /Form /BBox [0 0 200 200] \
         /Resources << /Font << /F1 10 0 R /F2 9 0 R >> >> /Length {} >>\nstream\n{form_text}\nendstream",
        form_text.len()
    );
    let font = |glyph| {
        format!(
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica \
         /Encoding << /Type /Encoding /BaseEncoding /WinAnsiEncoding /Differences [65 /{glyph}] >> >>"
        )
    };
    build_catalog_test_pdf(
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>",
        &[
            (3, page(7).as_bytes()),
            (4, page(8).as_bytes()),
            (
                5,
                stream("BT /F1 12 Tf 20 160 Td (A) Tj ET /Shared Do BT 20 60 Td (A) Tj ET").as_bytes(),
            ),
            (6, form.as_bytes()),
            (7, font("B").as_bytes()),
            (8, font("C").as_bytes()),
            (
                9,
                b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>",
            ),
            (10, font(form_glyph).as_bytes()),
        ],
    )
}

fn assert_shadowed_selected_font_survives_form(form_glyph: &str, expected: [&str; 2]) {
    let pdf = shadowed_selected_font_pdf(form_glyph);
    for page in 0..2 {
        let doc = PdfDocument::from_bytes(pdf.clone()).expect("open cold caller context");
        assert_eq!(span_text(&doc, page), expected[page], "cold page {page}");
    }
    for order in [[0, 1], [1, 0]] {
        let doc = PdfDocument::from_bytes(pdf.clone()).expect("open shared Form contexts");
        for page in order.into_iter().chain(order) {
            doc.page_spans_cache.lock().unwrap().clear();
            assert_eq!(span_text(&doc, page), expected[page], "page {page}, order {order:?}");
        }
    }
}

#[test]
fn form_cache_misses_restore_the_callers_selected_shadowed_font() {
    assert_shadowed_selected_font_survives_form("D", ["BDOwnB", "CDOwnC"]);
}

#[test]
fn form_cache_misses_do_not_drop_caller_words_as_form_font_spaces() {
    assert_shadowed_selected_font_survives_form("space", ["B OwnB", "C OwnC"]);
}

#[test]
fn form_cache_misses_restore_the_callers_character_decoder() {
    for (glyph, expected) in [("D", ["BDOwnB", "CDOwnC"]), ("space", ["BOwnB", "COwnC"])] {
        let pdf = shadowed_selected_font_pdf(glyph);
        for order in [[0, 1], [1, 0]] {
            let doc = PdfDocument::from_bytes(pdf.clone()).expect("open shared character contexts");
            for page in order.into_iter().chain(order) {
                doc.page_chars_cache.lock().unwrap().clear();
                let actual: String = doc
                    .extract_chars(page)
                    .expect("extract independent character layer")
                    .iter()
                    .map(|character| character.char)
                    .filter(|character| !character.is_whitespace())
                    .collect();
                assert_eq!(actual, expected[page], "glyph {glyph}, page {page}, order {order:?}");
            }
        }
    }
}
