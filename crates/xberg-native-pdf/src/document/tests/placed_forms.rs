use super::super::*;
use super::common::*;

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

pub(super) const FULL_BBOX: &str = "[0 0 600 800]";

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
    let looping = format!(
        "{}/Fs Do\n/Fs Do\n",
        text_lines_stream(&["Looping side note".to_string()], 400, 60)
    );
    let pdf = build_forms_pdf(
        "/Fs Do\n/Fb Do\n/Fp Do\n",
        &[
            ("Fs", FULL_BBOX, &looping),
            ("Fb", FULL_BBOX, &body),
            ("Fp", FULL_BBOX, &copy),
        ],
    );
    let doc = PdfDocument::from_bytes(pdf.clone()).expect("parse pdf");
    let text = doc.extract_text(0).unwrap();
    assert_eq!(
        text.matches("Looping side note").count(),
        1,
        "self-painting form text: {text:?}"
    );
    assert_eq!(
        body_line_counts(pdf, "Meadow"),
        (1, 1),
        "body copies (text, chars): {text:?}"
    );
}

/// Two forms that paint each other twice, then a sibling form with the body, then
/// a shifted `/PlacedPDF` copy of the body. Each form of the cycle runs once, and
/// the copy is still judged a duplicate of the later body.
#[test]
fn test_mutually_painting_forms_do_not_hide_a_later_sibling_body() {
    let lines = body_lines("Orchard");
    let body = text_lines_stream(&lines, 20, 780);
    let copy = format!("/PlacedPDF /MC0 BDC\n{}EMC\n", text_lines_stream(&lines, 29, 771));
    let first = format!(
        "{}/Ft Do\n/Ft Do\n",
        text_lines_stream(&["First cycle note".to_string()], 400, 60)
    );
    let second = format!(
        "{}/Fs Do\n/Fs Do\n",
        text_lines_stream(&["Second cycle note".to_string()], 400, 40)
    );
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
    assert_eq!(
        body_line_counts(pdf, "Orchard"),
        (1, 1),
        "body copies (text, chars): {text:?}"
    );
}

/// The page paints its body inside the box of a form. The form paints text outside
/// its small `/BBox` first, which is clipped, and then a shifted `/PlacedPDF` copy
/// of the body. The clip must not move the copy out of its hold group, so the copy
/// is still dropped.
#[test]
fn test_placed_copy_after_clipped_text_in_the_same_form_is_dropped() {
    let lines = body_lines("Canyon");
    let stray: Vec<String> = (0..10)
        .map(|i| format!("Stray galley line {i} beyond the box"))
        .collect();
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
    let body = format!(
        "{}{}",
        text_lines_stream(left, 20, 780),
        text_lines_stream(right, 300, 780)
    );
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
    assert_eq!(
        chars.matches("Wordform007").count(),
        1,
        "chars kept the copy: {chars:?}"
    );
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

/// A form keeps its whole body inside one `/PlacedPDF` scope and, inside that
/// scope, paints a second form with its own small `/PlacedPDF` note. The second
/// form joins the group of the first, so the note is kept with the body although
/// it is too small to be kept alone, in text and chars.
#[test]
fn test_form_painted_inside_the_parent_placed_scope_joins_its_group() {
    let lines = body_lines("Fjord");
    let outer = format!(
        "/PlacedPDF /MC0 BDC\n{}/Fi Do\nEMC\n",
        text_lines_stream(&lines, 20, 780)
    );
    let inner = format!(
        "/PlacedPDF /MC1 BDC\n{}EMC\n",
        text_lines_stream(&["Margin note on lichen".to_string()], 400, 60)
    );
    let pdf = build_forms_pdf("/Fo Do\n", &[("Fo", FULL_BBOX, &outer), ("Fi", FULL_BBOX, &inner)]);
    let doc = PdfDocument::from_bytes(pdf.clone()).expect("parse pdf");
    let text = doc.extract_text(0).unwrap();
    let chars: String = doc.extract_chars(0).unwrap().iter().map(|c| c.char).collect();
    assert_eq!(
        (
            text.matches("Margin note on lichen").count(),
            chars.matches("Margin note on lichen").count()
        ),
        (1, 1),
        "nested note (text, chars): {text:?}"
    );
    assert_eq!(body_line_counts(pdf, "Fjord"), (1, 1), "body copies (text, chars)");
}

/// The page paints its body and a form whose `/PlacedPDF` scope holds a shifted
/// copy of the body. Outside that scope, after its EMC or before its BDC, the form
/// paints a second form with its own `/PlacedPDF` scope of unique text, also inside
/// a plain `/Span` scope after the EMC. The second form opens its own hold group, so
/// its text is kept and the copy is dropped, with the page body painted before or
/// after the form, in text and chars.
#[test]
fn test_form_painted_outside_the_parent_placed_scope_opens_its_own_group() {
    let lines = body_lines("Tundra");
    let unique: Vec<String> = (0..30)
        .map(|i| format!("Zephyr kite {i} drifts across salt flats"))
        .collect();
    let copy = format!("/PlacedPDF /MC0 BDC\n{}EMC\n", text_lines_stream(&lines, 29, 771));
    let child = format!("/PlacedPDF /MC1 BDC\n{}EMC\n", text_lines_stream(&unique, 20, 380));
    let body = text_lines_stream(&lines, 20, 780);
    for parent in [
        format!("{copy}/Fc Do\n"),
        format!("/Fc Do\n{copy}"),
        format!("{copy}/Span BMC\n/Fc Do\nEMC\n"),
    ] {
        for page in [format!("{body}/Fo Do\n"), format!("/Fo Do\n{body}")] {
            let pdf = build_forms_pdf(&page, &[("Fo", FULL_BBOX, &parent), ("Fc", FULL_BBOX, &child)]);
            let doc = PdfDocument::from_bytes(pdf.clone()).expect("parse pdf");
            let text = doc.extract_text(0).unwrap();
            let chars: String = doc.extract_chars(0).unwrap().iter().map(|c| c.char).collect();
            let needle = "Zephyr kite 7 drifts";
            assert_eq!(
                (text.matches(needle).count(), chars.matches(needle).count()),
                (1, 1),
                "unique child text (text, chars) for parent {parent:?} and page {page:?}"
            );
            assert_eq!(
                body_line_counts(pdf, "Tundra"),
                (1, 1),
                "body copies (text, chars) for parent {parent:?} and page {page:?}"
            );
        }
    }
}

/// The page paints its body and a form that first paints a second form with a
/// shifted `/PlacedPDF` copy of the body, and then opens its own `/PlacedPDF` scope
/// of unique text. The second form's group is dropped, and the first form's group
/// comes back after it, so the unique text is kept, in text and chars.
#[test]
fn test_parent_placed_scope_after_a_child_form_keeps_its_own_group() {
    let lines = body_lines("Steppe");
    let unique: Vec<String> = (0..30)
        .map(|i| format!("Zephyr kite {i} drifts across salt flats"))
        .collect();
    let copy = format!("/PlacedPDF /MC0 BDC\n{}EMC\n", text_lines_stream(&lines, 29, 771));
    let parent = format!(
        "/Fc Do\n/PlacedPDF /MC1 BDC\n{}EMC\n",
        text_lines_stream(&unique, 20, 380)
    );
    let body = text_lines_stream(&lines, 20, 780);
    for page in [format!("{body}/Fo Do\n"), format!("/Fo Do\n{body}")] {
        let pdf = build_forms_pdf(&page, &[("Fo", FULL_BBOX, &parent), ("Fc", FULL_BBOX, &copy)]);
        let doc = PdfDocument::from_bytes(pdf.clone()).expect("parse pdf");
        let text = doc.extract_text(0).unwrap();
        let chars: String = doc.extract_chars(0).unwrap().iter().map(|c| c.char).collect();
        let needle = "Zephyr kite 7 drifts";
        assert_eq!(
            (text.matches(needle).count(), chars.matches(needle).count()),
            (1, 1),
            "unique parent text (text, chars) for page {page:?}"
        );
        assert_eq!(
            body_line_counts(pdf, "Steppe"),
            (1, 1),
            "body copies (text, chars) for page {page:?}"
        );
    }
}

/// The 30 body lines of the placed-copy tests: each holds the needle `Kestrel` once.
fn kestrel_lines() -> Vec<String> {
    (0..30)
        .map(|i| format!("Kestrel paragraph number {i} about ferry cables"))
        .collect()
}

/// Paint `lines` from the top at `x`, 12 points apart, one `BT` block, with
/// `line` writing the operators of one line at baseline `y`.
pub(super) fn paint_lines(lines: &[String], x: i32, top: i32, line: impl Fn(&str, i32, i32) -> String) -> String {
    let mut out = String::from("BT /F1 10 Tf\n");
    for (i, text) in lines.iter().enumerate() {
        out.push_str(&line(text, x, top - 12 * i as i32));
    }
    out + "ET\n"
}

/// Lines with literal spaces, one `Tm` and `Tj` each.
pub(super) fn plain_line(text: &str, x: i32, y: i32) -> String {
    format!("1 0 0 1 {x} {y} Tm ({text}) Tj\n")
}

/// `paint` applied to each word of `text` that has 4 or more letters and is not
/// the needle, with the word's two halves; other words stay as they are.
fn cut_words(text: &str, paint: impl Fn(&str, &str) -> String) -> Vec<String> {
    text.split(' ')
        .map(|w| {
            if w.len() >= 4 && w != "Kestrel" {
                paint(&w[..w.len() / 2], &w[w.len() / 2..])
            } else {
                w.to_string()
            }
        })
        .collect()
}

/// Needle counts `(spans, chars)` for a page that paints `body` and a form whose
/// `/PlacedPDF` scope paints `copy`: first with the body before the form, then
/// with the body after it.
pub(super) fn placed_copy_counts(body: &str, copy: &str) -> Vec<(usize, usize)> {
    let copy = format!("/PlacedPDF /MC0 BDC\n{copy}EMC\n");
    [format!("{body}/Fo Do\n"), format!("/Fo Do\n{body}")]
        .iter()
        .map(|page| {
            let pdf = build_forms_pdf(page, &[("Fo", "[0 0 600 800]", &copy)]);
            let doc = PdfDocument::from_bytes(pdf).expect("parse pdf");
            let spans: String = doc.extract_spans(0).unwrap().iter().map(|s| s.text.as_str()).collect();
            let chars: String = doc.extract_chars(0).unwrap().iter().map(|c| c.char).collect();
            (spans.matches("Kestrel").count(), chars.matches("Kestrel").count())
        })
        .collect()
}

/// The page paints each body line as separate word runs with horizontal gaps and
/// no space characters, and a form whose `/PlacedPDF` scope holds a copy of the
/// body as lines with literal spaces, shifted 9 points. The copy is dropped in
/// spans and in chars, with the page body painted before or after the form.
#[test]
fn test_placed_copy_of_a_body_painted_as_word_runs_is_dropped_in_chars() {
    let lines = kestrel_lines();
    let body = paint_lines(&lines, 20, 780, |text, x, y| {
        let words: String = text.split(' ').map(|w| format!("({w}) Tj 60 0 Td\n")).collect();
        format!("1 0 0 1 {x} {y} Tm\n{words}")
    });
    let copy = paint_lines(&lines, 29, 771, plain_line);
    assert_eq!(placed_copy_counts(&body, &copy), [(30, 30); 2], "body first, body last");
}

/// The copy cuts its words with a tightening `TJ` offset, which spans keep as one
/// word. The copy is dropped in chars as it is in spans.
#[test]
fn test_placed_copy_with_tightening_kerns_is_dropped_in_chars() {
    let lines = kestrel_lines();
    let body = paint_lines(&lines, 20, 780, plain_line);
    for offset in [150, 250] {
        let copy = paint_lines(&lines, 29, 771, |text, x, y| {
            let words = cut_words(text, |a, b| format!("{a}) {offset} ({b}"));
            format!("1 0 0 1 {x} {y} Tm [({})] TJ\n", words.join(" "))
        });
        assert_eq!(
            placed_copy_counts(&body, &copy),
            [(30, 30); 2],
            "kern {offset}: body first, body last"
        );
    }
}

/// The copy paints one glyph per `Tm` and `Tj`, and the second half of each cut
/// word sits 1 point higher. Spans join those glyphs into one run, so the copy is
/// dropped in chars as it is in spans.
#[test]
fn test_placed_copy_painted_per_glyph_with_a_raised_half_is_dropped_in_chars() {
    let lines = kestrel_lines();
    let body = paint_lines(&lines, 20, 780, plain_line);
    let copy = paint_lines(&lines, 29, 771, |text, x, y| {
        let marks = cut_words(text, |a, b| format!("{}{}", "_".repeat(a.len()), "^".repeat(b.len())));
        let raised: Vec<bool> = marks.join(" ").bytes().map(|m| m == b'^').collect();
        text.chars()
            .enumerate()
            .map(|(col, ch)| {
                let gy = y as f32 + if raised[col] { 1.0 } else { 0.0 };
                format!("1 0 0 1 {:.1} {gy:.1} Tm ({ch}) Tj\n", x as f32 + col as f32 * 6.0)
            })
            .collect()
    });
    assert_eq!(placed_copy_counts(&body, &copy), [(30, 30); 2], "body first, body last");
}

/// The copy paints one Courier glyph per `Tm` and `Tj` and marks each word gap
/// with an empty cell instead of a space glyph. Spans put a space in that cell,
/// so chars end a word there too, and the copy is dropped in both.
#[test]
fn test_placed_copy_in_fixed_pitch_cells_is_dropped_in_chars() {
    let lines = kestrel_lines();
    let body = paint_lines(&lines, 20, 780, plain_line);
    let copy = paint_lines(&lines, 29, 771, |text, x, y| {
        let glyphs: String = text
            .chars()
            .enumerate()
            .filter(|(_, ch)| *ch != ' ')
            .map(|(col, ch)| format!("1 0 0 1 {:.1} {y} Tm ({ch}) Tj\n", x as f32 + col as f32 * 6.0))
            .collect();
        format!("/F2 10 Tf\n{glyphs}/F1 10 Tf\n")
    });
    assert_eq!(placed_copy_counts(&body, &copy), [(30, 30); 2], "body first, body last");
}

/// The copy paints the second half of each cut word with a `TJ` after a `Tj`.
/// Spans start a new span at the `TJ`. The copy is dropped in spans and in chars.
#[test]
fn test_word_cut_by_a_tj_array_gives_one_count_in_both_views() {
    let lines = kestrel_lines();
    let body = paint_lines(&lines, 20, 780, plain_line);
    let copy = paint_lines(&lines, 29, 771, |text, x, y| {
        let words = cut_words(text, |a, b| format!("{a}) Tj [({b})] TJ ("));
        format!("1 0 0 1 {x} {y} Tm ({}) Tj\n", words.join(" "))
    });
    assert_eq!(placed_copy_counts(&body, &copy), [(30, 30); 2], "body first, body last");
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

/// A form clips stray text and then holds a copy with no spaces, so only the run
/// numbers separate the words. The clip must keep them aligned with the chars.
#[test]
fn test_placed_copy_as_word_runs_after_clipped_text_is_dropped_in_chars() {
    let lines = kestrel_lines();
    let stray: Vec<String> = (0..10)
        .map(|i| format!("Stray galley line {i} beyond the box"))
        .collect();
    let page = format!("{}/Fe Do\n", paint_lines(&lines, 20, 380, plain_line));
    let enclosing = format!("{}/Fp Do\n", paint_lines(&stray, 20, 780, plain_line));
    let inner = paint_lines(&lines, 29, 371, |text, x, y| {
        let words: String = text.split(' ').map(|w| format!("({w}) Tj 60 0 Td\n")).collect();
        format!("1 0 0 1 {x} {y} Tm\n{words}")
    });
    let copy = format!("/PlacedPDF /MC0 BDC\n{inner}EMC\n");
    let pdf = build_forms_pdf(
        &page,
        &[("Fe", "[0 0 500 400]", &enclosing), ("Fp", "[0 0 600 800]", &copy)],
    );
    let doc = PdfDocument::from_bytes(pdf).expect("parse pdf");
    let chars: String = doc.extract_chars(0).unwrap().iter().map(|c| c.char).collect();
    assert_eq!(chars.matches("Kestrel").count(), 30);
}

/// The body sits in fixed-pitch cells, so the run width after each gap decides
/// every later word of the line.
#[test]
fn test_placed_copy_of_a_body_in_fixed_pitch_cells_is_dropped_in_chars() {
    let lines = kestrel_lines();
    let body = paint_lines(&lines, 20, 780, |text, x, y| {
        let glyphs: String = text
            .chars()
            .enumerate()
            .filter(|(_, ch)| *ch != ' ')
            .map(|(col, ch)| format!("1 0 0 1 {:.1} {y} Tm ({ch}) Tj\n", x as f32 + col as f32 * 6.0))
            .collect();
        format!("/F2 10 Tf\n{glyphs}/F1 10 Tf\n")
    });
    let copy = paint_lines(&lines, 29, 771, plain_line);
    assert_eq!(placed_copy_counts(&body, &copy), [(30, 30); 2], "body first, body last");
}

/// The copy paints a one-glyph `TJ`, then one word per `Tm` from right to left.
/// A `TJ` leaves no open run, so each `Tm` after it ends a run in chars as in spans.
#[test]
fn test_placed_copy_painted_right_to_left_after_a_tj_is_dropped_in_chars() {
    let lines = kestrel_lines();
    let body = paint_lines(&lines, 20, 780, plain_line);
    let copy = paint_lines(&lines, 29, 771, |text, x, y| {
        let words: Vec<&str> = text.split(' ').collect();
        let mut starts = Vec::new();
        let mut at = x;
        for w in &words {
            starts.push(at);
            at += (w.len() as i32 + 1) * 6;
        }
        let mut out = format!("1 0 0 1 {} {y} Tm [(-)] TJ\n", x - 6);
        for i in (0..words.len()).rev() {
            out.push_str(&format!("1 0 0 1 {} {y} Tm ({}) Tj\n", starts[i], words[i]));
        }
        out
    });
    assert_eq!(placed_copy_counts(&body, &copy), [(30, 30); 2], "body first, body last");
}

#[test]
fn caller_only_placed_tag_retains_form_body() {
    let lines = body_lines("Harbor");
    let form = text_lines_stream(&lines, 20, 760);
    let header = text_lines_stream(&["Quarterly Review".into()], 20, 790);
    for scope in ["/PlacedPDF BMC", "/Placed#50DF /MC0 BDC"] {
        let page = format!("{header}{scope}\n/Fb Do\nEMC\n");
        let doc = PdfDocument::from_bytes(build_forms_pdf(&page, &[("Fb", FULL_BBOX, &form)])).expect("parse PDF");
        let text = doc.extract_text(0).unwrap();
        let spans: String = doc
            .extract_spans(0)
            .unwrap()
            .iter()
            .map(|span| span.text.as_str())
            .collect();
        let chars: String = doc.extract_chars(0).unwrap().iter().map(|ch| ch.char).collect();
        for (path, actual) in [("text", text), ("spans", spans), ("chars", chars)] {
            assert_eq!(actual.matches("Quarterly Review").count(), 1, "{scope}, {path}");
            for line in &lines {
                assert_eq!(
                    actual.matches(line).count(),
                    1,
                    "{scope}, {path}: lost or copied {line:?}"
                );
            }
        }
    }
}

#[test]
fn caller_only_placed_tag_drops_short_and_duplicate_forms() {
    let lines = body_lines("Harbor");
    let short = text_lines_stream(&lines[..8], 20, 760);
    let header = text_lines_stream(&["Quarterly Review".into()], 20, 790);
    let short_page = format!("{header}/PlacedPDF BMC\n/Fb Do\nEMC\n");
    assert_eq!(
        body_line_counts(build_forms_pdf(&short_page, &[("Fb", FULL_BBOX, &short)]), "Harbor"),
        (0, 0)
    );

    let body = text_lines_stream(&lines, 20, 760);
    let copy = text_lines_stream(&lines, 29, 751);
    let wrapped = "/PlacedPDF BMC\n/Fb Do\nEMC\n";
    for page in [format!("{body}{wrapped}"), format!("{wrapped}{body}")] {
        assert_eq!(
            body_line_counts(build_forms_pdf(&page, &[("Fb", FULL_BBOX, &copy)]), "Harbor"),
            (1, 1)
        );
    }
}

#[test]
fn caller_only_hold_keeps_an_own_tagged_child_in_its_separate_group() {
    let lines = body_lines("Harbor");
    let unique: Vec<_> = (0..25)
        .map(|i| format!("Zephyr kite {i} drifts across salt flats with lanterns"))
        .collect();
    let body = text_lines_stream(&lines, 20, 760);
    let copy = text_lines_stream(&lines, 29, 751);
    let child = format!("/PlacedPDF BMC\n{}EMC\n", text_lines_stream(&unique, 20, 260));
    let page = format!("{body}/PlacedPDF BMC\n/Fb Do\nEMC\n");
    for parent in [format!("/Fc Do\n{copy}"), format!("{copy}/Fc Do\n")] {
        let pdf = build_forms_pdf(&page, &[("Fb", FULL_BBOX, &parent), ("Fc", FULL_BBOX, &child)]);
        let doc = PdfDocument::from_bytes(pdf).expect("parse PDF");
        let text = doc.extract_text(0).unwrap();
        let chars: String = doc.extract_chars(0).unwrap().iter().map(|ch| ch.char).collect();
        for actual in [text, chars] {
            assert_eq!(actual.matches("Harbor paragraph number 7 ").count(), 1);
            assert_eq!(actual.matches("Zephyr kite 7 drifts").count(), 1);
        }
    }
}

#[test]
fn caller_tagged_siblings_follow_ordinary_form_placement() {
    let lines = body_lines("Harbor");
    let body = text_lines_stream(&lines, 20, 760);
    let ordinary = "/Fa Do\n/Fb Do\n";
    let tagged = "/PlacedPDF BMC\n/Fa Do\nEMC\n/PlacedPDF BMC\n/Fb Do\nEMC\n";
    for (x, y, expected) in [
        (20, 760, (1, 1)),
        (22, 759, (1, 2)),
        (29, 751, (2, 2)),
        (315, 760, (2, 2)),
    ] {
        let second = text_lines_stream(&lines, x, y);
        let forms = [("Fa", FULL_BBOX, body.as_str()), ("Fb", FULL_BBOX, second.as_str())];
        assert_eq!(body_line_counts(build_forms_pdf(ordinary, &forms), "Harbor"), expected);
        assert_eq!(
            body_line_counts(build_forms_pdf(tagged, &forms), "Harbor"),
            expected,
            "caller-marked sibling placement at ({x}, {y})"
        );
    }
}

#[test]
fn caller_tagged_overlapping_siblings_keep_unique_tail() {
    let lines = body_lines("Harbor");
    let body = text_lines_stream(&lines, 20, 760);
    let mut second = lines[..29].to_vec();
    second.push("Unique tail remains visible".into());
    let second = text_lines_stream(&second, 22, 759);
    let forms = [("Fa", FULL_BBOX, body.as_str()), ("Fb", FULL_BBOX, second.as_str())];
    for page in [
        "/Fa Do\n/Fb Do\n",
        "/PlacedPDF BMC\n/Fa Do\nEMC\n/PlacedPDF BMC\n/Fb Do\nEMC\n",
    ] {
        let doc = PdfDocument::from_bytes(build_forms_pdf(page, &forms)).expect("parse PDF");
        let text = doc.extract_text(0).unwrap();
        let chars: String = doc.extract_chars(0).unwrap().iter().map(|ch| ch.char).collect();
        for actual in [text, chars] {
            assert_eq!(actual.matches("Unique tail remains visible").count(), 1);
            assert_eq!(actual.matches(&lines[29]).count(), 1);
        }
    }
}
