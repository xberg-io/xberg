use super::super::*;
use super::common::*;
use super::pdf_fixtures::*;

/// A page that draws text directly AND paints an overlay Form XObject
/// invoked without a `cm` (dangling operands ahead of the XObject name)
/// must extract both the direct text and the XObject's text, matching
/// poppler's and pymupdf's behaviour on the same malformed content (both
/// tools resolve `Do`'s name from whatever immediately precedes it,
/// discarding the dangling numeric operands rather than misreading them
/// as the XObject name).
#[test]
fn test_direct_and_overlay_xobject_text_both_extracted_with_orphaned_do_operands() {
    let pdf_bytes = build_xobject_do_with_orphaned_operands_pdf(Some("base body text"));
    let doc = PdfDocument::from_bytes(pdf_bytes).expect("parse repro pdf");

    let chars: String = doc.extract_chars(0).unwrap().iter().map(|c| c.char).collect();
    assert!(chars.contains("base body text"), "missing direct text: {chars:?}");
    assert!(chars.contains("overlay text"), "missing XObject text: {chars:?}");

    let text = doc.extract_text(0).unwrap();
    assert!(text.contains("base body text"));
    assert!(text.contains("overlay text"));

    let plain = doc.extract_text(0).unwrap();
    assert!(plain.contains("base body text"));
    assert!(plain.contains("overlay text"));
}

/// A page with no direct content of its own, whose only text comes from
/// a Form XObject invoked without a `cm`, must not extract as empty.
#[test]
fn test_xobject_only_page_text_extracted_with_orphaned_do_operands() {
    let pdf_bytes = build_xobject_do_with_orphaned_operands_pdf(None);
    let doc = PdfDocument::from_bytes(pdf_bytes).expect("parse repro pdf");

    let chars: String = doc.extract_chars(0).unwrap().iter().map(|c| c.char).collect();
    assert_eq!(chars, "overlay text");

    let text = doc.extract_text(0).unwrap();
    assert!(text.contains("overlay text"), "extract_text came back empty: {text:?}");
}

/// A page whose only content is a Form XObject holding the whole body inside a
/// `/PlacedPDF` scope must extract the body, not drop it.
#[test]
fn test_placed_pdf_body_inside_form_xobject_is_extracted() {
    let lines: Vec<String> = (0..30)
        .map(|i| format!("Paragraph line number {i} of the placed body text"))
        .collect();
    let doc = PdfDocument::from_bytes(build_placed_pdf_inside_form_pdf(&lines)).expect("parse pdf");

    let text = doc.extract_text(0).unwrap();
    assert!(text.contains("Paragraph line number 0 "), "first line lost: {text:?}");
    assert!(text.contains("Paragraph line number 29 "), "last line lost: {text:?}");
    let chars: String = doc.extract_chars(0).unwrap().iter().map(|c| c.char).collect();
    assert!(
        chars.contains("Paragraph line number 29 "),
        "char layer lost the body: {chars:?}"
    );
}

/// A short `/PlacedPDF` scope inside a form (a figure label, not the page
/// body) stays suppressed.
#[test]
fn test_small_placed_pdf_inside_form_xobject_stays_suppressed() {
    let lines = vec!["Figure label".to_string()];
    let doc = PdfDocument::from_bytes(build_placed_pdf_inside_form_pdf(&lines)).expect("parse pdf");
    let text = doc.extract_text(0).unwrap();
    assert!(
        !text.contains("Figure label"),
        "decorative placed text leaked: {text:?}"
    );
}

/// A page paints its own body, then a form holding a shifted copy of that body
/// inside `/PlacedPDF`. The copy is a duplicate overlay: the form's verdict must
/// count the page's own text, so the copy stays suppressed.
#[test]
fn test_shifted_duplicate_placed_form_is_suppressed_when_page_has_body() {
    let lines: Vec<String> = (0..30)
        .map(|i| format!("Paragraph line number {i} of the page body text"))
        .collect();
    let page = format!("{}/Fm0 Do\n", text_lines_stream(&lines, 20, 780));
    let form = format!("/PlacedPDF /MC0 BDC\n{}EMC\n", text_lines_stream(&lines, 29, 771));
    let doc = PdfDocument::from_bytes(build_page_and_form_pdf(&page, &form)).expect("parse pdf");

    let text = doc.extract_text(0).unwrap();
    assert_eq!(
        text.matches("Paragraph line number 7 ").count(),
        1,
        "duplicate overlay copy was kept: {text:?}"
    );
    let chars: String = doc.extract_chars(0).unwrap().iter().map(|c| c.char).collect();
    assert_eq!(
        chars.matches("Paragraph line number 7 ").count(),
        1,
        "char layer kept the duplicate overlay copy: {chars:?}"
    );
}

/// Page body text plus a form with a placed body of different words: the placed
/// text is real content, so counting the page text must not suppress it.
#[test]
fn test_unique_placed_form_is_kept_when_page_has_body() {
    let body: Vec<String> = (0..30).map(|i| format!("Alpha entry {i} on the page itself")).collect();
    let placed: Vec<String> = (0..30)
        .map(|i| format!("Zulu sentence {i} inside the placed region"))
        .collect();
    let page = format!("{}/Fm0 Do\n", text_lines_stream(&body, 20, 780));
    let form = format!("/PlacedPDF /MC0 BDC\n{}EMC\n", text_lines_stream(&placed, 300, 780));
    let doc = PdfDocument::from_bytes(build_page_and_form_pdf(&page, &form)).expect("parse pdf");

    let text = doc.extract_text(0).unwrap();
    assert!(text.contains("Alpha entry 7 "), "page body lost: {text:?}");
    assert!(text.contains("Zulu sentence 7 "), "unique placed text lost: {text:?}");
}

/// The page keeps its own placed text, runs a form, then paints more placed
/// text. The form must give the saved verdict back, not reset it to false.
#[test]
fn test_page_placed_text_after_a_form_is_still_kept() {
    let lines: Vec<String> = (0..30)
        .map(|i| format!("Paragraph line number {i} of the placed body text"))
        .collect();
    let tail = vec!["Closing line after the form".to_string()];
    let page = format!(
        "/PlacedPDF /MC0 BDC\n{}EMC\n/Fm0 Do\n/PlacedPDF /MC1 BDC\n{}EMC\n",
        text_lines_stream(&lines, 20, 780),
        text_lines_stream(&tail, 20, 300)
    );
    let form = text_lines_stream(&["Form label".to_string()], 400, 100);
    let doc = PdfDocument::from_bytes(build_page_and_form_pdf(&page, &form)).expect("parse pdf");

    let text = doc.extract_text(0).unwrap();
    assert!(
        text.contains("Paragraph line number 5 "),
        "page placed body lost: {text:?}"
    );
    assert!(text.contains("Form label"), "form text lost: {text:?}");
    assert!(
        text.contains("Closing line after the form"),
        "page placed text after the form was dropped: {text:?}"
    );
}

/// Two pages paint the same form. Page 1 has no body of its own, so the form
/// body is kept there. Page 2 paints its own copy of that body, so the form is a
/// duplicate overlay there. The verdict of page 1 must not be reused on page 2.
#[test]
fn test_form_placed_verdict_is_not_reused_across_pages() {
    let lines: Vec<String> = (0..30)
        .map(|i| format!("Paragraph line number {i} of the shared body text"))
        .collect();
    let form = format!("/PlacedPDF /MC0 BDC\n{}EMC\n", text_lines_stream(&lines, 29, 771));
    let page_a = "/Fm0 Do\n".to_string();
    let page_b = format!("{}/Fm0 Do\n", text_lines_stream(&lines, 20, 780));
    let doc = PdfDocument::from_bytes(build_two_pages_sharing_form_pdf(&page_a, &page_b, &form)).expect("parse pdf");

    let first = doc.extract_text(0).unwrap();
    assert!(
        first.contains("Paragraph line number 7 "),
        "page 1 lost the form body: {first:?}"
    );
    let second = doc.extract_text(1).unwrap();
    assert_eq!(
        second.matches("Paragraph line number 7 ").count(),
        1,
        "page 2 reused the verdict of page 1: {second:?}"
    );
}

/// Page 1 keeps its own placed text, so it keeps the placed body of the shared form
/// without a verdict of its own. Page 2 paints a copy of that body outside any
/// scope, so the form is a duplicate overlay there. The spans that page 1 kept
/// must not be reused on page 2, in either page order.
#[test]
fn test_form_kept_under_a_page_verdict_is_not_reused_across_pages() {
    let shared: Vec<String> = (0..30)
        .map(|i| format!("Paragraph line number {i} of the shared body text"))
        .collect();
    let other: Vec<String> = (0..30)
        .map(|i| format!("Unrelated line number {i} of the page text"))
        .collect();
    let form = format!("/PlacedPDF /MC0 BDC\n{}EMC\n", text_lines_stream(&shared, 29, 771));
    let page_a = format!(
        "/PlacedPDF /MC9 BDC\n{}EMC\n/Fm0 Do\n",
        text_lines_stream(&other, 20, 780)
    );
    let page_b = format!("{}/Fm0 Do\n", text_lines_stream(&shared, 20, 780));
    let pdf = build_two_pages_sharing_form_pdf(&page_a, &page_b, &form);

    let fresh = PdfDocument::from_bytes(pdf.clone()).expect("parse pdf");
    let alone = fresh.extract_text(1).unwrap();
    assert_eq!(
        alone.matches("Paragraph line number 7 ").count(),
        1,
        "page 2 alone keeps a duplicate form body: {alone:?}"
    );

    let doc = PdfDocument::from_bytes(pdf).expect("parse pdf");
    let first = doc.extract_text(0).unwrap();
    assert!(
        first.contains("Paragraph line number 7 "),
        "page 1 lost the form body: {first:?}"
    );
    let second = doc.extract_text(1).unwrap();
    assert_eq!(
        second.matches("Paragraph line number 7 ").count(),
        1,
        "page 2 reused the form spans that page 1 kept: {second:?}"
    );
}

/// The page paints its normal body through a form, and then a shifted `/PlacedPDF`
/// copy of that body through a second form. The page stream holds no text, so the
/// text of the first form is the outside text that shows the second form is a copy.
#[test]
fn test_placed_form_copy_of_a_body_painted_by_a_sibling_form_is_suppressed() {
    let lines: Vec<String> = (0..30)
        .map(|i| format!("Sibling paragraph number {i} about river locks"))
        .collect();
    let body = text_lines_stream(&lines, 20, 780);
    let copy = format!("/PlacedPDF /MC0 BDC\n{}EMC\n", text_lines_stream(&lines, 29, 771));
    let page = "/Fm1 Do\n/Fm0 Do\n".to_string();
    let doc =
        PdfDocument::from_bytes(build_two_pages_with_forms_pdf(&page, "", &copy, Some(&body))).expect("parse pdf");

    let first = doc.extract_text(0).unwrap();
    assert_eq!(
        first.matches("Sibling paragraph number 7 ").count(),
        1,
        "the placed copy next to a sibling form body was kept: {first:?}"
    );
}

/// The normal body and the shifted `/PlacedPDF` copy sit in one enclosing form, and
/// the copy is a form that the enclosing form paints after its body.
#[test]
fn test_placed_form_copy_of_a_body_painted_by_an_enclosing_form_is_suppressed() {
    let lines: Vec<String> = (0..30)
        .map(|i| format!("Enclosing paragraph number {i} about canal gates"))
        .collect();
    let copy = format!("/PlacedPDF /MC0 BDC\n{}EMC\n", text_lines_stream(&lines, 29, 771));
    let enclosing = format!("{}/Fm1 Do\n", text_lines_stream(&lines, 20, 780));
    let page = "/Fm0 Do\n".to_string();
    let doc =
        PdfDocument::from_bytes(build_two_pages_with_forms_pdf(&page, "", &enclosing, Some(&copy))).expect("parse pdf");

    let first = doc.extract_text(0).unwrap();
    assert_eq!(
        first.matches("Enclosing paragraph number 7 ").count(),
        1,
        "the placed copy inside an enclosing form body was kept: {first:?}"
    );
}

/// The page paints the shifted `/PlacedPDF` copy through a form first, and then the
/// normal body through a sibling form. The copy is judged against the body that the
/// page paints after it, so only one copy of the body comes back.
#[test]
fn test_placed_form_copy_painted_before_a_sibling_form_body_is_suppressed() {
    let lines: Vec<String> = (0..30)
        .map(|i| format!("Sibling paragraph number {i} about river locks"))
        .collect();
    let body = text_lines_stream(&lines, 20, 780);
    let copy = format!("/PlacedPDF /MC0 BDC\n{}EMC\n", text_lines_stream(&lines, 29, 771));
    let page = "/Fm0 Do\n/Fm1 Do\n".to_string();
    let doc =
        PdfDocument::from_bytes(build_two_pages_with_forms_pdf(&page, "", &copy, Some(&body))).expect("parse pdf");

    let first = doc.extract_text(0).unwrap();
    assert_eq!(
        first.matches("Sibling paragraph number 7 ").count(),
        1,
        "the placed copy painted before a sibling form body was kept: {first:?}"
    );
    let chars: String = doc.extract_chars(0).unwrap().iter().map(|c| c.char).collect();
    assert_eq!(
        chars.matches("Sibling paragraph number 7 ").count(),
        1,
        "char layer kept the placed copy painted before a sibling form body: {chars:?}"
    );
}

/// The enclosing form paints the shifted `/PlacedPDF` copy first, and then its
/// normal body. The copy is judged against the body that follows it.
#[test]
fn test_placed_form_copy_painted_before_the_enclosing_form_body_is_suppressed() {
    let lines: Vec<String> = (0..30)
        .map(|i| format!("Enclosing paragraph number {i} about canal gates"))
        .collect();
    let copy = format!("/PlacedPDF /MC0 BDC\n{}EMC\n", text_lines_stream(&lines, 29, 771));
    let enclosing = format!("/Fm1 Do\n{}", text_lines_stream(&lines, 20, 780));
    let page = "/Fm0 Do\n".to_string();
    let doc =
        PdfDocument::from_bytes(build_two_pages_with_forms_pdf(&page, "", &enclosing, Some(&copy))).expect("parse pdf");

    let first = doc.extract_text(0).unwrap();
    assert_eq!(
        first.matches("Enclosing paragraph number 7 ").count(),
        1,
        "the placed copy painted before the enclosing form body was kept: {first:?}"
    );
    let chars: String = doc.extract_chars(0).unwrap().iter().map(|c| c.char).collect();
    assert_eq!(
        chars.matches("Enclosing paragraph number 7 ").count(),
        1,
        "char layer kept the placed copy painted before the enclosing form body: {chars:?}"
    );
}

/// A form keeps its whole body inside one `/PlacedPDF` scope, and paints a second
/// form inside that scope with a copy of the body. The second form's text is placed
/// text too, not outside text, so the body is not judged a duplicate and dropped.
#[test]
fn test_form_painted_inside_a_placed_scope_is_not_outside_text() {
    let lines: Vec<String> = (0..30)
        .map(|i| format!("Placed paragraph number {i} about meadow birds"))
        .collect();
    let outer = format!(
        "/PlacedPDF /MC0 BDC\n{}/Fm1 Do\nEMC\n",
        text_lines_stream(&lines, 20, 780)
    );
    let inner = text_lines_stream(&lines, 29, 771);
    let page = "/Fm0 Do\n".to_string();
    let doc =
        PdfDocument::from_bytes(build_two_pages_with_forms_pdf(&page, "", &outer, Some(&inner))).expect("parse pdf");

    let first = doc.extract_text(0).unwrap();
    assert!(
        first.contains("Placed paragraph number 7 "),
        "the placed body was judged against its own nested copy: {first:?}"
    );
}

/// The placed copy sits in a form that then paints the normal body through a form
/// named only in its own resources. That body is text outside the placed scope,
/// so the copy is judged a duplicate.
#[test]
fn test_body_in_a_form_named_only_by_the_placed_form_counts_as_outside_text() {
    let lines: Vec<String> = (0..30)
        .map(|i| format!("Nested paragraph number {i} about valley farms"))
        .collect();
    let outer = format!(
        "/PlacedPDF /MC0 BDC\n{}EMC\n/Fm1 Do\n",
        text_lines_stream(&lines, 29, 771)
    );
    let inner = text_lines_stream(&lines, 20, 780);
    let page = "/Fm0 Do\n".to_string();
    let pdf = build_two_pages_with_forms_pdf(&page, "", &outer, Some(&inner));
    // Rename the page's entry for the inner form; the outer form keeps its own. ~keep
    let text = String::from_utf8_lossy(&pdf).replace("/Fm0 6 0 R /Fm1 9 0 R", "/Fm0 6 0 R /Fx1 9 0 R");
    let doc = PdfDocument::from_bytes(text.into_bytes()).expect("parse pdf");

    let first = doc.extract_text(0).unwrap();
    assert_eq!(
        first.matches("Nested paragraph number 7 ").count(),
        1,
        "the placed copy was not judged against the body of its own nested form: {first:?}"
    );
}

/// Page 1 holds a copy of the body of a form with a placed scope, so that form is a
/// duplicate overlay there and its placed scope drops the text of the plain form it
/// paints. Page 2 paints the plain form directly, so its text must stay. The empty
/// result of page 1 must not be reused on page 2.
#[test]
fn test_form_nested_in_a_placed_form_is_not_reused_across_pages() {
    let outer_lines: Vec<String> = (0..30)
        .map(|i| format!("Outer body sentence {i} about harbour tides"))
        .collect();
    let inner_lines: Vec<String> = (0..30)
        .map(|i| format!("Inner note number {i} covering orchard frost"))
        .collect();
    let outer = format!(
        "/PlacedPDF /MC0 BDC\n{}/Fm1 Do\nEMC\n",
        text_lines_stream(&outer_lines, 29, 771)
    );
    let inner = text_lines_stream(&inner_lines, 300, 771);
    let page_a = format!("{}/Fm0 Do\n", text_lines_stream(&outer_lines, 20, 780));
    let page_b = "/Fm1 Do\n".to_string();
    let doc = PdfDocument::from_bytes(build_two_pages_with_forms_pdf(&page_a, &page_b, &outer, Some(&inner)))
        .expect("parse pdf");

    let first = doc.extract_text(0).unwrap();
    assert_eq!(
        first.matches("Outer body sentence 7 ").count(),
        1,
        "page 1 kept the duplicate placed copy: {first:?}"
    );
    let second = doc.extract_text(1).unwrap();
    assert!(
        second.contains("Inner note number 7 "),
        "page 2 reused the empty result of the nested form from page 1: {second:?}"
    );
}

/// The page keeps its own placed body. A form with a small placed scope follows the
/// page and is kept; it does not get a verdict of its own, which would drop it.
#[test]
fn test_small_placed_form_follows_a_page_that_keeps_its_placed_body() {
    let lines: Vec<String> = (0..30)
        .map(|i| format!("Paragraph line number {i} of the placed body text"))
        .collect();
    let page = format!(
        "/PlacedPDF /MC0 BDC\n{}EMC\n/Fm0 Do\n",
        text_lines_stream(&lines, 20, 780)
    );
    let form = format!(
        "/PlacedPDF /MC1 BDC\n{}EMC\n",
        text_lines_stream(&["Small form note".to_string()], 400, 100)
    );
    let doc = PdfDocument::from_bytes(build_page_and_form_pdf(&page, &form)).expect("parse pdf");

    let text = doc.extract_text(0).unwrap();
    assert!(text.contains("Paragraph line number 5 "), "page body lost: {text:?}");
    assert!(text.contains("Small form note"), "form text dropped: {text:?}");
}

/// A form that keeps its own placed body does not change the verdict of the page
/// stream after the form: a small placed note the page paints later stays dropped.
#[test]
fn test_form_verdict_does_not_leak_to_the_page_text_after_the_form() {
    let lines: Vec<String> = (0..30)
        .map(|i| format!("Paragraph line number {i} of the form body text"))
        .collect();
    let form = format!("/PlacedPDF /MC0 BDC\n{}EMC\n", text_lines_stream(&lines, 29, 771));
    let page = format!(
        "/Fm0 Do\n/PlacedPDF /MC1 BDC\n{}EMC\n",
        text_lines_stream(&["Late placed note".to_string()], 400, 100)
    );
    let doc = PdfDocument::from_bytes(build_page_and_form_pdf(&page, &form)).expect("parse pdf");

    let text = doc.extract_text(0).unwrap();
    assert!(text.contains("Paragraph line number 7 "), "form body lost: {text:?}");
    assert!(
        !text.contains("Late placed note"),
        "the verdict of the form leaked to the page text: {text:?}"
    );
}

/// Page 1 paints the plain body form, so its spans enter the cache. Page 2 paints it
/// again from the cache and then a shifted `/PlacedPDF` copy of it. The cached
/// spans are still outside text, so the copy is dropped.
#[test]
fn test_placed_form_copy_of_a_cached_sibling_form_body_is_suppressed() {
    let lines: Vec<String> = (0..30)
        .map(|i| format!("Cached paragraph number {i} about lock keepers"))
        .collect();
    let body = text_lines_stream(&lines, 20, 780);
    let copy = format!("/PlacedPDF /MC0 BDC\n{}EMC\n", text_lines_stream(&lines, 29, 771));
    let page_a = "/Fm1 Do\n".to_string();
    let page_b = "/Fm1 Do\n/Fm0 Do\n".to_string();
    let doc = PdfDocument::from_bytes(build_two_pages_with_forms_pdf(&page_a, &page_b, &copy, Some(&body)))
        .expect("parse pdf");

    doc.extract_text(0).unwrap();
    let second = doc.extract_text(1).unwrap();
    assert_eq!(
        second.matches("Cached paragraph number 7 ").count(),
        1,
        "the placed copy next to a cached sibling form body was kept: {second:?}"
    );
}

/// Thirty lines of a body about `topic`, and the number of times line 7 of it
/// appears in the text and in the characters of page 1 of `pdf`.
fn body_line_counts(pdf: Vec<u8>, topic: &str) -> (usize, usize) {
    let doc = PdfDocument::from_bytes(pdf).expect("parse pdf");
    let needle = format!("{topic} paragraph number 7 ");
    let text = doc.extract_text(0).unwrap();
    let chars: String = doc.extract_chars(0).unwrap().iter().map(|c| c.char).collect();
    (text.matches(&needle).count(), chars.matches(&needle).count())
}

fn body_lines(topic: &str) -> Vec<String> {
    (0..30)
        .map(|i| format!("{topic} paragraph number {i} about ferry cables"))
        .collect()
}

const FULL_BBOX: &str = "[0 0 600 800]";

/// The page paints a visible `/PlacedPDF` body and an ordinary sibling form that
/// holds the same body outside its small `/BBox`. The sibling's copy is clipped,
/// so the placed body is the only visible copy and comes back once, in both paint
/// orders.
#[test]
fn test_placed_body_survives_a_clipped_copy_in_a_sibling_form() {
    let lines = body_lines("Harbor");
    let placed = format!("/PlacedPDF /MC0 BDC\n{}EMC\n", text_lines_stream(&lines, 20, 780));
    let clipped = text_lines_stream(&lines, 20, 780);
    for page in ["/Fp Do\n/Fc Do\n", "/Fc Do\n/Fp Do\n"] {
        let pdf = build_forms_pdf(page, &[("Fp", FULL_BBOX, &placed), ("Fc", "[0 0 100 100]", &clipped)]);
        assert_eq!(
            body_line_counts(pdf, "Harbor"),
            (1, 1),
            "the visible placed body is not kept once (text, chars) for page {page:?}"
        );
    }
}

/// An enclosing form with a small `/BBox` paints its own copy of the body outside
/// the box, and a `/PlacedPDF` form whose body sits inside it. The enclosing copy
/// is clipped, so the placed body comes back once, in both paint orders.
#[test]
fn test_placed_body_survives_a_clipped_copy_in_the_enclosing_form() {
    let lines = body_lines("Quarry");
    let placed = format!("/PlacedPDF /MC0 BDC\n{}EMC\n", text_lines_stream(&lines, 20, 380));
    let copy = text_lines_stream(&lines, 20, 780);
    for enclosing in [format!("/Fp Do\n{copy}"), format!("{copy}/Fp Do\n")] {
        let pdf = build_forms_pdf(
            "/Fe Do\n",
            &[("Fe", "[0 0 400 400]", &enclosing), ("Fp", FULL_BBOX, &placed)],
        );
        assert_eq!(
            body_line_counts(pdf, "Quarry"),
            (1, 1),
            "the visible placed body is not kept once (text, chars) for enclosing stream {enclosing:?}"
        );
    }
}

/// A form that paints itself twice, then a sibling form with the body, then a
/// shifted `/PlacedPDF` copy of the body. The self-painting form runs once, so
/// its note appears once and the later sibling's body still shows that the copy
/// is a duplicate.
#[test]
fn test_self_painting_form_does_not_hide_a_later_sibling_body() {
    let lines = body_lines("Meadow");
    let body = text_lines_stream(&lines, 20, 780);
    let copy = format!("/PlacedPDF /MC0 BDC\n{}EMC\n", text_lines_stream(&lines, 29, 771));
    let looping = format!("{}/Fs Do\n/Fs Do\n", text_lines_stream(&["Looping side note".to_string()], 400, 60));
    let pdf = build_forms_pdf(
        "/Fs Do\n/Fb Do\n/Fp Do\n",
        &[("Fs", FULL_BBOX, &looping), ("Fb", FULL_BBOX, &body), ("Fp", FULL_BBOX, &copy)],
    );
    let doc = PdfDocument::from_bytes(pdf.clone()).expect("parse pdf");
    let text = doc.extract_text(0).unwrap();
    assert_eq!(text.matches("Looping side note").count(), 1, "self-painting form text: {text:?}");
    assert_eq!(body_line_counts(pdf, "Meadow"), (1, 1), "body copies (text, chars): {text:?}");
}

/// Two forms that paint each other twice, then a sibling form with the body, then
/// a shifted `/PlacedPDF` copy of the body. Each form of the cycle runs once, and
/// the copy is still judged a duplicate of the later body.
#[test]
fn test_mutually_painting_forms_do_not_hide_a_later_sibling_body() {
    let lines = body_lines("Orchard");
    let body = text_lines_stream(&lines, 20, 780);
    let copy = format!("/PlacedPDF /MC0 BDC\n{}EMC\n", text_lines_stream(&lines, 29, 771));
    let first = format!("{}/Ft Do\n/Ft Do\n", text_lines_stream(&["First cycle note".to_string()], 400, 60));
    let second = format!("{}/Fs Do\n/Fs Do\n", text_lines_stream(&["Second cycle note".to_string()], 400, 40));
    let pdf = build_forms_pdf(
        "/Fs Do\n/Fb Do\n/Fp Do\n",
        &[
            ("Fs", FULL_BBOX, &first),
            ("Ft", FULL_BBOX, &second),
            ("Fb", FULL_BBOX, &body),
            ("Fp", FULL_BBOX, &copy),
        ],
    );
    let doc = PdfDocument::from_bytes(pdf.clone()).expect("parse pdf");
    let text = doc.extract_text(0).unwrap();
    assert_eq!(text.matches("First cycle note").count(), 1, "cycle text: {text:?}");
    assert_eq!(text.matches("Second cycle note").count(), 1, "cycle text: {text:?}");
    assert_eq!(body_line_counts(pdf, "Orchard"), (1, 1), "body copies (text, chars): {text:?}");
}

/// The page paints its body inside the box of a form. The form paints text outside
/// its small `/BBox` first, which is clipped, and then a shifted `/PlacedPDF` copy
/// of the body. The clip must not move the copy out of its hold group, so the copy
/// is still dropped.
#[test]
fn test_placed_copy_after_clipped_text_in_the_same_form_is_dropped() {
    let lines = body_lines("Canyon");
    let stray: Vec<String> = (0..10).map(|i| format!("Stray galley line {i} beyond the box")).collect();
    let page = format!("{}/Fe Do\n", text_lines_stream(&lines, 20, 380));
    let enclosing = format!("{}/Fp Do\n", text_lines_stream(&stray, 20, 780));
    let copy = format!("/PlacedPDF /MC0 BDC\n{}EMC\n", text_lines_stream(&lines, 29, 371));
    let pdf = build_forms_pdf(&page, &[("Fe", "[0 0 400 400]", &enclosing), ("Fp", FULL_BBOX, &copy)]);
    assert_eq!(body_line_counts(pdf, "Canyon"), (1, 1), "body copies (text, chars)");
}

/// A shifted `/PlacedPDF` copy paints the lines of the body in the opposite order.
/// Each line is one word, so the text and the characters both judge the copy a
/// duplicate only when they split the copy into its lines.
#[test]
fn test_placed_copy_in_another_line_order_is_dropped_in_text_and_chars() {
    let words: Vec<String> = (0..80).map(|i| format!("Wordform{i:03}")).collect();
    let (left, right) = words.split_at(40);
    let body = format!("{}{}", text_lines_stream(left, 20, 780), text_lines_stream(right, 300, 780));
    let mut reversed_left = left.to_vec();
    reversed_left.reverse();
    let mut reversed_right = right.to_vec();
    reversed_right.reverse();
    let copy = format!(
        "/PlacedPDF /MC0 BDC\n{}{}EMC\n",
        text_lines_stream(&reversed_left, 29, 771),
        text_lines_stream(&reversed_right, 309, 771)
    );
    let page = format!("{body}/Fp Do\n");
    let doc = PdfDocument::from_bytes(build_forms_pdf(&page, &[("Fp", FULL_BBOX, &copy)])).expect("parse pdf");
    let text = doc.extract_text(0).unwrap();
    let chars: String = doc.extract_chars(0).unwrap().iter().map(|c| c.char).collect();
    assert_eq!(text.matches("Wordform007").count(), 1, "text kept the copy: {text:?}");
    assert_eq!(chars.matches("Wordform007").count(), 1, "chars kept the copy: {chars:?}");
}

/// A form keeps its whole body inside one `/PlacedPDF` scope and first paints a
/// second form with its own small `/PlacedPDF` label. The second form joins the
/// hold group of the first, so the body after it is still held and kept.
#[test]
fn test_placed_form_keeps_its_body_after_a_nested_placed_form() {
    let lines = body_lines("Glacier");
    let outer = format!(
        "/PlacedPDF /MC0 BDC\n/Fi Do\n{}EMC\n",
        text_lines_stream(&lines, 20, 780)
    );
    let inner = format!(
        "/PlacedPDF /MC1 BDC\n{}EMC\n",
        text_lines_stream(&["Nested label".to_string()], 400, 60)
    );
    let pdf = build_forms_pdf("/Fo Do\n", &[("Fo", FULL_BBOX, &outer), ("Fi", FULL_BBOX, &inner)]);
    assert_eq!(body_line_counts(pdf, "Glacier"), (1, 1), "body copies (text, chars)");
}

/// Page 1 paints a plain form, so its spans enter the cache. Page 2 paints its own
/// body and a `/PlacedPDF` form whose placed scope paints that plain form, which
/// holds a shifted copy of the body. The copy is held and dropped, not taken from
/// the cache as kept text.
#[test]
fn test_cached_form_painted_in_a_placed_scope_is_still_held() {
    let lines = body_lines("Lagoon");
    let copy = text_lines_stream(&lines, 29, 771);
    let placed = "/PlacedPDF /MC0 BDC\n/Fm1 Do\nEMC\n".to_string();
    let page_a = "/Fm1 Do\n".to_string();
    let page_b = format!("{}/Fm0 Do\n", text_lines_stream(&lines, 20, 780));
    let doc = PdfDocument::from_bytes(build_two_pages_with_forms_pdf(&page_a, &page_b, &placed, Some(&copy)))
        .expect("parse pdf");
    doc.extract_text(0).unwrap();
    let second = doc.extract_text(1).unwrap();
    assert_eq!(
        second.matches("Lagoon paragraph number 7 ").count(),
        1,
        "page 2 kept the cached copy: {second:?}"
    );
}

#[test]
fn test_get_page_content_data_empty_content() {
    let pdf = build_minimal_pdf(b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let data = doc.get_page_content_data(0).unwrap();
    // Empty content stream still returns data (may be empty or have a newline) ~keep
    assert!(data.len() <= 2);
}

#[test]
fn test_get_page_content_data_with_content() {
    let content = b"BT /F1 12 Tf (Hello) Tj ET";
    let pdf = build_minimal_pdf(content);
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let data = doc.get_page_content_data(0).unwrap();
    assert!(!data.is_empty());
    let text = String::from_utf8_lossy(&data);
    assert!(text.contains("Hello"));
}

#[test]
fn test_get_page_content_data_blank_page() {
    let pdf = build_multi_page_pdf(1);
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let data = doc.get_page_content_data(0).unwrap();
    assert!(data.is_empty());
}

#[test]
fn test_extract_chars_blank_page() {
    let pdf = build_minimal_pdf(b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let chars = doc.extract_chars(0).unwrap();
    assert!(chars.is_empty());
}

#[test]
fn test_apply_intelligent_text_processing_empty() {
    let pdf = build_minimal_pdf(b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let spans: Vec<TextSpan> = vec![];
    let result = doc.apply_intelligent_text_processing(spans);
    assert!(result.is_empty());
}

#[test]
fn test_apply_intelligent_text_processing_ligature_preserved() {
    // The pipeline preserves Unicode ligature characters that come
    // from the font's ToUnicode map (U+FB01 = ﬁ). Expanding them to plain "fi"
    // caused Jaccard mismatches against ground-truth corpora that keep ligatures. ~keep
    let pdf = build_minimal_pdf(b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let spans = vec![make_test_span("\u{FB01}nd", 0.0, 0.0, 50.0, 12.0)];
    let result = doc.apply_intelligent_text_processing(spans);
    assert_eq!(result.len(), 1);
    assert!(
        result[0].text.contains('\u{FB01}'),
        "ﬁ must be preserved, got: {:?}",
        result[0].text
    );
}

#[test]
fn test_page_with_array_contents() {
    let mut pdf = b"%PDF-1.4\n".to_vec();

    let off1 = pdf.len();
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    let off2 = pdf.len();
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

    let off3 = pdf.len();
    pdf.extend_from_slice(
            b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents [4 0 R 5 0 R] /Resources << >> >>\nendobj\n",
        );

    let content1 = b"q";
    let off4 = pdf.len();
    pdf.extend_from_slice(format!("4 0 obj\n<< /Length {} >>\nstream\n", content1.len()).as_bytes());
    pdf.extend_from_slice(content1);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");

    let content2 = b"Q";
    let off5 = pdf.len();
    pdf.extend_from_slice(format!("5 0 obj\n<< /Length {} >>\nstream\n", content2.len()).as_bytes());
    pdf.extend_from_slice(content2);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");

    let xref_off = pdf.len();
    pdf.extend_from_slice(b"xref\n0 6\n");
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off1).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off2).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off3).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off4).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off5).as_bytes());
    pdf.extend_from_slice(format!("trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n", xref_off).as_bytes());

    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let data = doc.get_page_content_data(0).unwrap();
    let text = String::from_utf8_lossy(&data);
    assert!(text.contains("q"));
    assert!(text.contains("Q"));
}

#[test]
fn test_extract_hierarchical_content_blank_page() {
    let pdf = build_minimal_pdf(b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let result = doc.extract_hierarchical_content(0);
    // Should not crash, may return Ok(Some) or Ok(None) ~keep
    assert!(result.is_ok());
}

#[test]
fn test_extract_spans_with_config_blank_page() {
    let pdf = build_minimal_pdf(b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let config = crate::extractors::SpanMergingConfig::default();
    let spans = doc.extract_spans_with_config(0, config).unwrap();
    assert!(spans.is_empty());
}

#[test]
fn test_extract_chars_out_of_bounds() {
    let pdf = build_minimal_pdf(b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    assert!(doc.extract_chars(999).is_err());
}

#[test]
fn test_get_page_content_data_out_of_bounds() {
    let pdf = build_minimal_pdf(b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    assert!(doc.get_page_content_data(999).is_err());
}

#[test]
fn test_page_content_indirect_array() {
    let mut pdf = b"%PDF-1.4\n".to_vec();
    let off1 = pdf.len();
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    let off2 = pdf.len();
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    let off3 = pdf.len();
    pdf.extend_from_slice(
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << >> >>\nendobj\n",
    );
    let off4 = pdf.len();
    pdf.extend_from_slice(b"4 0 obj\n[5 0 R 6 0 R]\nendobj\n");
    let c1 = b"q";
    let off5 = pdf.len();
    pdf.extend_from_slice(format!("5 0 obj\n<< /Length {} >>\nstream\n", c1.len()).as_bytes());
    pdf.extend_from_slice(c1);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");
    let c2 = b"Q";
    let off6 = pdf.len();
    pdf.extend_from_slice(format!("6 0 obj\n<< /Length {} >>\nstream\n", c2.len()).as_bytes());
    pdf.extend_from_slice(c2);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");
    let xref_off = pdf.len();
    pdf.extend_from_slice(b"xref\n0 7\n");
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off1).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off2).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off3).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off4).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off5).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off6).as_bytes());
    pdf.extend_from_slice(format!("trailer\n<< /Size 7 /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n", xref_off).as_bytes());
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let data = doc.get_page_content_data(0).unwrap();
    let text = String::from_utf8_lossy(&data);
    assert!(text.contains("q"));
    assert!(text.contains("Q"));
}

#[test]
fn test_get_page_content_data_null_contents() {
    let mut pdf = b"%PDF-1.4\n".to_vec();
    let off1 = pdf.len();
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    let off2 = pdf.len();
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    let off3 = pdf.len();
    pdf.extend_from_slice(
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents null /Resources << >> >>\nendobj\n",
    );
    let xref_off = pdf.len();
    pdf.extend_from_slice(b"xref\n0 4\n");
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off1).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off2).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off3).as_bytes());
    pdf.extend_from_slice(format!("trailer\n<< /Size 4 /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n", xref_off).as_bytes());
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    assert!(doc.get_page_content_data(0).unwrap().is_empty());
}

#[test]
fn test_apply_intelligent_text_processing_fl_ligature_preserved() {
    // Same as ﬁ: ﬂ (U+FB02) must be preserved, not expanded to "fl". ~keep
    let pdf = build_minimal_pdf(b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let spans = vec![make_test_span("\u{FB02}oor", 0.0, 0.0, 50.0, 12.0)];
    let result = doc.apply_intelligent_text_processing(spans);
    assert!(
        result[0].text.contains('\u{FB02}'),
        "ﬂ must be preserved, got: {:?}",
        result[0].text
    );
}

#[test]
fn test_apply_intelligent_text_processing_ocr_font() {
    let pdf = build_minimal_pdf(b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let mut span = make_test_span("Test  Text", 0.0, 0.0, 100.0, 12.0);
    span.font_name = "OCR".to_string();
    let result = doc.apply_intelligent_text_processing(vec![span]);
    assert!(!result[0].text.contains("  "));
}

#[test]
fn test_extract_spans_with_config_adaptive() {
    let pdf = build_minimal_pdf(b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    assert!(
        doc.extract_spans_with_config(0, crate::extractors::SpanMergingConfig::adaptive())
            .unwrap()
            .is_empty()
    );
}

#[test]
fn test_extract_spans_with_config_out_of_bounds() {
    let pdf = build_minimal_pdf(b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    assert!(
        doc.extract_spans_with_config(999, crate::extractors::SpanMergingConfig::default())
            .is_err()
    );
}

#[test]
fn test_extract_page_text_blank_page() {
    let pdf = build_minimal_pdf(b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let page_text = doc.extract_page_text(0).unwrap();
    assert!(page_text.spans.is_empty());
    assert!(page_text.chars.is_empty());
    // MediaBox is [0 0 612 792] in build_minimal_pdf ~keep
    assert!((page_text.page_width - 612.0).abs() < 0.1);
    assert!((page_text.page_height - 792.0).abs() < 0.1);
}

#[test]
fn test_extract_page_text_has_page_dimensions() {
    let content = b"BT /F1 12 Tf (Hello) Tj ET";
    let pdf = build_minimal_pdf(content);
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let page_text = doc.extract_page_text(0).unwrap();
    assert!((page_text.page_width - 612.0).abs() < 0.1);
    assert!((page_text.page_height - 792.0).abs() < 0.1);
}

#[test]
fn test_extract_page_text_chars_derived_from_spans() {
    let content = b"BT /F1 12 Tf (Hello) Tj ET";
    let pdf = build_minimal_pdf(content);
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let page_text = doc.extract_page_text(0).unwrap();
    let expected_char_count: usize = page_text.spans.iter().map(|s| s.text.chars().count()).sum();
    assert_eq!(page_text.chars.len(), expected_char_count);
}

#[test]
fn test_extract_page_text_with_column_aware() {
    let pdf = build_minimal_pdf(b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let page_text = doc
        .extract_page_text_with_options(0, ReadingOrder::ColumnAware)
        .unwrap();
    assert!(page_text.spans.is_empty());
    assert!((page_text.page_width - 612.0).abs() < 0.1);
}

#[test]
fn test_extract_page_text_out_of_bounds() {
    let pdf = build_minimal_pdf(b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let result = doc.extract_page_text(99);
    assert!(result.is_err());
}

/// Regression test: Tm-scale containment filter must not
/// drop distinct text lines whose bounding boxes overlap spatially.
///
/// Before the fix, the containment filter in extract_text() would skip any
/// span geometrically contained within the previous span, even if the text
/// was different. This caused the second line to silently disappear.
///
/// The fix adds a `span.text == prev.text` guard so that only true
/// duplicates are filtered.
#[test]
fn test_containment_filter_preserves_distinct_overlapping_lines() {
    // Build a minimal PDF with two Td-placed text strings at very close Y
    // positions (Y=700 and Y=699 — within the 2.0pt "same line" threshold)
    // but with different content. The first string is wider so the second
    // is geometrically contained within it. ~keep
    let content = b"BT /F1 12 Tf 50 700 Td (First line has longer text here) Tj 0 -1 Td (Second) Tj ET";

    let mut pdf = b"%PDF-1.4\n".to_vec();

    let off1 = pdf.len();
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    let off2 = pdf.len();
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

    let off3 = pdf.len();
    pdf.extend_from_slice(
            b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>\nendobj\n",
        );

    let off4 = pdf.len();
    let content_len = content.len();
    pdf.extend_from_slice(format!("4 0 obj\n<< /Length {} >>\nstream\n", content_len).as_bytes());
    pdf.extend_from_slice(content);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");

    let off5 = pdf.len();
    pdf.extend_from_slice(b"5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n");

    let xref_off = pdf.len();
    pdf.extend_from_slice(b"xref\n0 6\n");
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off1).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off2).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off3).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off4).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off5).as_bytes());
    pdf.extend_from_slice(format!("trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n", xref_off).as_bytes());

    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let text = doc.extract_text(0).unwrap();

    assert!(
        text.contains("First line has longer text here"),
        "First line should be present in extracted text, got: {:?}",
        text
    );
    assert!(
        text.contains("Second"),
        "Second line must NOT be dropped by containment filter, got: {:?}",
        text
    );
}

/// `extract_spans`/`extract_words`/`extract_text_lines` must report the
/// resolved `/BaseFont` name ("Helvetica"), not the page's
/// `/Resources/Font` dictionary alias ("F1") — matching what
/// `extract_chars` already did.
#[test]
fn test_span_word_line_font_name_is_resolved_not_alias() {
    let content = b"BT /F1 12 Tf 50 700 Td (Hello) Tj ET";

    let mut pdf = b"%PDF-1.4\n".to_vec();
    let off1 = pdf.len();
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    let off2 = pdf.len();
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    let off3 = pdf.len();
    pdf.extend_from_slice(
            b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>\nendobj\n",
        );
    let off4 = pdf.len();
    pdf.extend_from_slice(format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()).as_bytes());
    pdf.extend_from_slice(content);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");
    let off5 = pdf.len();
    pdf.extend_from_slice(b"5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n");
    let xref_off = pdf.len();
    pdf.extend_from_slice(b"xref\n0 6\n");
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off1).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off2).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off3).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off4).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off5).as_bytes());
    pdf.extend_from_slice(format!("trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n", xref_off).as_bytes());

    let doc = PdfDocument::from_bytes(pdf).unwrap();

    let chars = doc.extract_chars(0).unwrap();
    assert_eq!(chars[0].font_name, "Helvetica");

    let spans = doc.extract_spans(0).unwrap();
    assert_eq!(spans[0].font_name, "Helvetica");

    let words = doc.extract_words(0).unwrap();
    assert_eq!(words[0].dominant_font, "Helvetica");

    let lines = doc.extract_text_lines(0).unwrap();
    assert_eq!(lines[0].words[0].dominant_font, "Helvetica");
}

/// `Word.sequence` must reflect content-stream emission order (the
/// originating span's `sequence`), not just reading order, so
/// consumers can tell genuinely-consecutive draw calls apart from
/// spatially-close-but-stream-distant ones (e.g. table cells vs.
/// overlays).
#[test]
fn test_extract_words_sequence_reflects_stream_order() {
    let content = b"BT /F1 12 Tf 50 700 Td (First) Tj 0 -20 Td (Second) Tj ET";

    let mut pdf = b"%PDF-1.4\n".to_vec();
    let off1 = pdf.len();
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    let off2 = pdf.len();
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    let off3 = pdf.len();
    pdf.extend_from_slice(
            b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>\nendobj\n",
        );
    let off4 = pdf.len();
    pdf.extend_from_slice(format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()).as_bytes());
    pdf.extend_from_slice(content);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");
    let off5 = pdf.len();
    pdf.extend_from_slice(b"5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n");
    let xref_off = pdf.len();
    pdf.extend_from_slice(b"xref\n0 6\n");
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off1).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off2).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off3).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off4).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off5).as_bytes());
    pdf.extend_from_slice(format!("trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n", xref_off).as_bytes());

    let doc = PdfDocument::from_bytes(pdf).unwrap();

    let words = doc.extract_words(0).unwrap();
    let first = words.iter().find(|w| w.text == "First").unwrap();
    let second = words.iter().find(|w| w.text == "Second").unwrap();
    assert!(
        first.sequence < second.sequence,
        "word drawn first in the content stream must have the smaller sequence: \
             First={}, Second={}",
        first.sequence,
        second.sequence
    );

    let lines = doc.extract_text_lines(0).unwrap();
    let line_words: Vec<&crate::layout::Word> = lines.iter().flat_map(|l| l.words.iter()).collect();
    let first_line_word = line_words.iter().find(|w| w.text == "First").unwrap();
    let second_line_word = line_words.iter().find(|w| w.text == "Second").unwrap();
    assert!(
        first_line_word.sequence < second_line_word.sequence,
        "extract_text_lines words must also carry stream-order sequence"
    );
}

#[test]
fn test_extract_page_text_dimensions_use_extent_not_corner_for_nonzero_origin() {
    // GH#1653: `/MediaBox [10 -100 622 692]` describes a 612x792 page whose
    // lower-left corner sits at (10, -100), not the origin. `page_width` /
    // `page_height` must report the EXTENT (urx - llx, ury - lly) = 612x792,
    // not the raw upper-right corner (622, 692). ~keep
    let pdf = build_minimal_pdf_with_media_box("10 -100 622 692", b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let page_text = doc.extract_page_text(0).unwrap();
    assert!(
        (page_text.page_width - 612.0).abs() < 0.1,
        "expected extent width 612, got {}",
        page_text.page_width
    );
    assert!(
        (page_text.page_height - 792.0).abs() < 0.1,
        "expected extent height 792, got {}",
        page_text.page_height
    );
}

#[test]
fn test_extract_page_text_dimensions_unchanged_for_zero_origin() {
    // Companion to the non-zero-origin test above: when llx == lly == 0 (the
    // overwhelming majority of real PDFs), extent and corner coincide, so this
    // must report the exact same values as before GH#1653's fix. ~keep
    let pdf = build_minimal_pdf_with_media_box("0 0 612 792", b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let page_text = doc.extract_page_text(0).unwrap();
    assert!((page_text.page_width - 612.0).abs() < 0.1);
    assert!((page_text.page_height - 792.0).abs() < 0.1);
}
