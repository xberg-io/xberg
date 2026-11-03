use super::super::*;
use super::common::*;

/// `n` lines of 6-point Helvetica from the top at `y0`, 8 points apart. Each line
/// holds `word` six times, with a different letter after it, so a line sizes a group
/// by a known count and no two lines share a word.
pub(super) fn six_word_lines(word: &str, n: usize, y0: i32) -> String {
    let mut out = String::from("BT /F1 6 Tf\n");
    for i in 0..n {
        let line: Vec<String> = (0..6)
            .map(|k| format!("{word}{}x", (b'a' + ((i + k) % 26) as u8) as char))
            .collect();
        out.push_str(&format!(
            "1 0 0 1 20 {} Tm ({}) Tj\n",
            y0 - 8 * i as i32,
            line.join(" ")
        ));
    }
    out + "ET\n"
}

pub(super) fn placed(body: &str) -> String {
    format!("/PlacedPDF /MC0 BDC\n{body}EMC\n")
}

fn span_text(spans: &[TextSpan]) -> String {
    spans.iter().map(|s| s.text.as_str()).collect()
}

fn char_text(chars: &[crate::layout::TextChar]) -> String {
    chars.iter().map(|c| c.char).collect()
}

/// A group of 8 kept lines plus `removed`, a body of 60 lines that a filter takes
/// out. Without the filter the group passes the 800 character gate, with it the
/// group stays under the gate. Spans and chars follow one verdict in each case.
fn assert_filter_decides_the_group(
    removed: &str,
    catalog_extra: &str,
    resources_extra: &str,
    layers: &[&str],
    inks: &[&str],
) {
    let body = placed(&format!("{}{removed}", six_word_lines("Zephyr", 8, 200)));
    let page = six_word_lines("Page", 2, 780) + "/Fo Do\n";
    let pages = [("[0 0 600 800]", page.as_str())];
    let pdf = build_pages_pdf(
        &pages,
        &[("Fo", "[0 0 600 800]", &body)],
        catalog_extra,
        resources_extra,
    );
    let doc = PdfDocument::from_bytes(pdf).expect("parse pdf");
    let unfiltered = (
        span_text(&doc.extract_spans(0).unwrap()),
        char_text(&doc.extract_chars(0).unwrap()),
    );
    assert!(unfiltered.0.contains("Zephyr"), "no filter, spans: {:?}", unfiltered.0);
    assert!(unfiltered.1.contains("Zephyr"), "no filter, chars: {:?}", unfiltered.1);

    let set = |names: &[&str]| names.iter().map(|n| n.to_string()).collect::<HashSet<String>>();
    let spans = doc.extract_spans_filtered(0, set(layers), set(inks)).unwrap();
    let chars = doc.extract_chars_filtered(0, set(layers), set(inks)).unwrap();
    let (spans, chars) = (span_text(&spans), char_text(&chars));
    assert!(spans.contains("Page"), "the page text is lost in spans: {spans:?}");
    assert!(chars.contains("Page"), "the page text is lost in chars: {chars:?}");
    assert!(!spans.contains("Zephyr"), "filtered spans kept the group: {spans:?}");
    assert!(!chars.contains("Zephyr"), "filtered chars kept the group: {chars:?}");
}

/// A layer filter takes a group under the gate, so the char verdict uses the filter.
#[test]
fn test_layer_filter_reaches_the_verdict_of_a_char_run() {
    let removed = format!("/OC /L1 BDC\n{}EMC\n", six_word_lines("Quokka", 60, 700));
    let catalog = "/OCProperties << /OCGs [5 0 R] /D << /ON [5 0 R] >> >>";
    assert_filter_decides_the_group(&removed, catalog, "/Properties << /L1 5 0 R >>", &["Hidden"], &[]);
}

/// An ink filter takes a group under the gate, so the char verdict uses the filter.
#[test]
fn test_ink_filter_reaches_the_verdict_of_a_char_run() {
    let tint = "<< /FunctionType 2 /Domain [0 1] /C0 [0 0 0 0] /C1 [0 0 0 1] /N 1 >>";
    let spaces = format!("/ColorSpace << /Spot [/Separation /Varnish /DeviceCMYK {tint}] >>");
    let removed = format!("/Spot cs 1 scn\n{}0 g\n", six_word_lines("Quokka", 60, 700));
    assert_filter_decides_the_group(&removed, "", &spaces, &[], &["Varnish"]);
}

/// A figure form paints one line in its box and stray text outside it, on a full
/// page after a smaller first page. The wrapper test reads the page of the call in
/// both views, so the stray text goes from spans and chars. Tagged, the stray text
/// does not count for the group, so the line under the gate goes too.
#[test]
fn test_stray_form_text_is_clipped_by_the_size_of_its_own_page() {
    let stray = format!(
        "{}{}",
        six_word_lines("Zephyr", 40, 780),
        six_word_lines("Yarrow", 40, 90)
    );
    let inside = "BT /F1 6 Tf 1 0 0 1 10 96 Tm (Inside) Tj ET\n";
    let untagged = format!("{inside}{stray}");
    for (form, keeps_inside) in [(untagged.clone(), true), (placed(&untagged), false)] {
        let page = "BT /F1 6 Tf 300 400 Td (Notice) Tj ET\n/Small Do\n";
        let pages = [
            ("[0 0 120 120]", "BT /F1 6 Tf 10 10 Td (Front) Tj ET\n"),
            ("[0 0 600 800]", page),
        ];
        let pdf = build_pages_pdf(&pages, &[("Small", "[0 0 100 100]", &form)], "", "");
        let doc = PdfDocument::from_bytes(pdf).expect("parse pdf");
        let (spans, chars) = (
            span_text(&doc.extract_spans(1).unwrap()),
            char_text(&doc.extract_chars(1).unwrap()),
        );
        for text in [&spans, &chars] {
            assert!(text.contains("Notice"), "the page text is lost: {text:?}");
            assert!(!text.contains("Zephyr"), "stray text kept: {text:?}");
            assert_eq!(text.contains("Inside"), keeps_inside, "the line in the box: {text:?}");
        }
    }
}

/// A form covers most of a small second page and paints its body outside its box.
/// The wrapper test reads the small page, not the larger first page, so the body
/// stays in chars as it does in spans.
#[test]
fn test_wrapper_form_body_is_kept_by_the_size_of_its_own_page() {
    let form = six_word_lines("Zephyr", 12, 110).replace("1 0 0 1 20", "1 0 0 1 410");
    let pages = [
        ("[0 0 600 800]", "BT /F1 6 Tf 10 10 Td (Front) Tj ET\n"),
        ("[0 0 600 120]", "/Small Do\n"),
    ];
    let pdf = build_pages_pdf(&pages, &[("Small", "[0 0 400 120]", &form)], "", "");
    let doc = PdfDocument::from_bytes(pdf).expect("parse pdf");
    let (spans, chars) = (
        span_text(&doc.extract_spans(1).unwrap()),
        char_text(&doc.extract_chars(1).unwrap()),
    );
    assert!(spans.contains("Zephyr"), "wrapper body lost in spans: {spans:?}");
    assert!(chars.contains("Zephyr"), "wrapper body lost in chars: {chars:?}");
}

/// A form the size of a large first page paints outside its box on a small second
/// page. The merging entry point reads the size of the small page, as the other
/// views do, and leaves the span cache as a plain read would.
#[test]
fn test_merging_config_reads_the_size_of_its_own_page() {
    let form = six_word_lines("Zephyr", 12, 110).replace("1 0 0 1 20", "1 0 0 1 410");
    let pages = [
        ("[0 0 400 120]", "BT /F1 6 Tf 10 10 Td (Front) Tj ET\n"),
        ("[0 0 600 800]", "/Small Do\n"),
    ];
    let open = || {
        let pdf = build_pages_pdf(&pages, &[("Small", "[0 0 400 120]", &form)], "", "");
        PdfDocument::from_bytes(pdf).expect("parse pdf")
    };
    let spans = span_text(&open().extract_spans(1).unwrap());
    let doc = open();
    let merged = doc.extract_spans_with_config(1, crate::extractors::SpanMergingConfig::default());
    assert_eq!(span_text(&merged.unwrap()), spans);
    assert_eq!(span_text(&doc.extract_spans(1).unwrap()), spans);
}

/// Page 0 fills the span cache for a form, and page 1 paints that form before a
/// second group. The verdict run opens the same group numbers as the char run: the
/// repeated group is dropped and the unique group stays.
#[test]
fn test_span_cache_does_not_shift_the_groups_of_a_char_run() {
    let copy = placed(&six_word_lines("Copyw", 40, 440));
    let unique = placed(&six_word_lines("Kestrel", 40, 780));
    let forms = [
        ("F", "[0 0 600 800]", "/G Do\n"),
        ("G", "[0 0 600 800]", copy.as_str()),
        ("H", "[0 0 600 800]", unique.as_str()),
    ];
    let second = six_word_lines("Copyw", 40, 440).replace("1 0 0 1 20", "1 0 0 1 300") + "/F Do\n/H Do\n";
    let pages = [("[0 0 600 800]", "/G Do\n/F Do\n"), ("[0 0 600 800]", second.as_str())];
    let doc = PdfDocument::from_bytes(build_pages_pdf(&pages, &forms, "", "")).expect("parse pdf");
    doc.extract_spans(0).unwrap();
    let chars = char_text(&doc.extract_chars(1).unwrap());
    assert_eq!(
        chars.matches("Copyw").count(),
        240,
        "the page copy alone stays: {chars:?}"
    );
    assert_eq!(
        chars.matches("Kestrel").count(),
        240,
        "the unique group stays: {chars:?}"
    );
}

/// Two groups on one page, a repeated copy and a unique body, in both paint orders
/// and both call orders. Each group keeps its own verdict in both views.
#[test]
fn test_two_groups_keep_their_own_verdicts_in_any_order() {
    let copy = six_word_lines("Copyw", 40, 780);
    let dup = placed(&six_word_lines("Copyw", 40, 776));
    let uniq = placed(&six_word_lines("Kestrel", 40, 780).replace("1 0 0 1 20", "1 0 0 1 300"));
    let forms = [
        ("Dup", "[0 0 600 800]", dup.as_str()),
        ("Uniq", "[0 0 600 800]", uniq.as_str()),
    ];
    for order in ["/Dup Do\n/Uniq Do\n", "/Uniq Do\n/Dup Do\n"] {
        for spans_first in [true, false] {
            let page = format!("{copy}{order}");
            let doc = PdfDocument::from_bytes(build_forms_pdf(&page, &forms)).expect("parse pdf");
            let chars_early = (!spans_first).then(|| char_text(&doc.extract_chars(0).unwrap()));
            let spans = span_text(&doc.extract_spans(0).unwrap());
            let chars = chars_early.unwrap_or_else(|| char_text(&doc.extract_chars(0).unwrap()));
            for text in [&spans, &chars] {
                assert_eq!(
                    text.matches("Kestrel").count(),
                    240,
                    "{order:?} {spans_first}: {text:?}"
                );
                assert_eq!(text.matches("Copyw").count(), 240, "{order:?} {spans_first}: {text:?}");
            }
        }
    }
}

/// Four Courier lines of 100 characters from `x`, in a form with a 400 by 300 box,
/// and `filler` more characters in the same group. Returns whether the filler stays
/// in spans and in chars.
fn clipped_group_kept(x: i32, filler: usize) -> (bool, bool) {
    let line = "Kestrel abcdefghij ".repeat(6);
    let lines: String = (0..4)
        .map(|i| format!("1 0 0 1 {x} {} Tm ({}) Tj\n", 280 - 12 * i, &line[..100]))
        .collect();
    let fill: String = (0..filler.div_ceil(40))
        .map(|i| {
            format!(
                "1 0 0 1 5 {} Tm ({}) Tj\n",
                8 + 5 * i,
                "Z".repeat((filler - i * 40).min(40))
            )
        })
        .collect();
    let form = placed(&format!("BT /F2 10 Tf\n{lines}ET\nBT /F1 4 Tf\n{fill}ET\n"));
    let pdf = build_forms_pdf("/Fo Do\n", &[("Fo", "[0 0 400 300]", &form)]);
    let doc = PdfDocument::from_bytes(pdf).expect("parse pdf");
    let spans = span_text(&doc.extract_spans(0).unwrap());
    let chars = char_text(&doc.extract_chars(0).unwrap());
    (spans.contains('Z'), chars.contains('Z'))
}

/// Spans keep a line whose centre is in the box whole, and chars cut it at the box.
/// The two views still reach one verdict, in both directions of that difference.
#[test]
fn test_clipped_group_gets_one_verdict_in_spans_and_chars() {
    assert_eq!(clipped_group_kept(20, 400), (true, true), "centre in, spans count more");
    assert_eq!(
        clipped_group_kept(200, 664),
        (false, false),
        "centre out, chars count more"
    );
    assert_eq!(
        clipped_group_kept(200, 800),
        (true, true),
        "the filler alone passes the gate"
    );
}

/// The char run of `extractor` on page 0 of `doc`, as a caller of the extractor writes it.
fn direct_chars<'doc>(doc: &'doc PdfDocument, mut extractor: crate::extractors::TextExtractor<'doc>) -> String {
    let resources = doc.get_page_resources(0).unwrap();
    extractor.set_resources(resources.clone());
    extractor.set_document(doc);
    doc.load_fonts_public(&resources, &mut extractor).unwrap();
    char_text(&extractor.extract(&doc.get_page_content_data(0).unwrap()).unwrap())
}

/// One page whose form scope holds 16 Courier lines of 47 glyphs and 4 gaps from
/// `line`, so the group reaches the gate only if the 64 gaps count as spaces.
fn sixteen_line_doc(line: impl Fn(i32) -> String) -> PdfDocument {
    let lines: String = (0..16).map(|i| line(771 - 12 * i)).collect();
    let form = placed(&format!("BT /F2 10 Tf\n{lines}ET\n"));
    let pdf = build_forms_pdf("/Fo Do\n", &[("Fo", "[0 0 600 800]", &form)]);
    PdfDocument::from_bytes(pdf).expect("parse pdf")
}

/// One page whose form scope holds one short line with word gaps in a `TJ` array.
/// The group stays under the size gate, so the page drops it.
fn small_tj_group_doc() -> PdfDocument {
    let body = "BT /F1 10 Tf\n1 0 0 1 72 700 Tm [(Alpha) -600 (Bravo) -600 (Charlie)] TJ\nET\n";
    let form = placed(body);
    let pdf = build_forms_pdf("/Fo Do\n", &[("Fo", "[0 0 600 800]", &form)]);
    PdfDocument::from_bytes(pdf).expect("parse pdf")
}

/// The space a char run adds for a `TJ` offset belongs to the group of its words.
#[test]
fn test_space_from_a_tj_offset_leaves_with_its_dropped_group() {
    let doc = small_tj_group_doc();
    let chars = doc.extract_chars(0).unwrap();
    assert_eq!(char_text(&chars), "", "chars of the dropped group");
}

/// The word-boundary detector writes its spans through the same group as a plain
/// run, so a dropped group leaves no span with that setting. The second `TJ` makes
/// the detector split text at the literal space.
#[test]
fn test_primary_boundary_spans_leave_with_their_dropped_group() {
    let body = "BT /F1 10 Tf\n1 0 0 1 72 700 Tm [(Alpha Bravo)] TJ\n[(Charlie Delta)] TJ\nET\n";
    let form = placed(body);
    let pdf = build_forms_pdf("/Fo Do\n", &[("Fo", "[0 0 600 800]", &form)]);
    let doc = PdfDocument::from_bytes(pdf).expect("parse pdf");
    let config = crate::extractors::TextExtractionConfig {
        word_boundary_mode: crate::pipeline::config::WordBoundaryMode::Primary,
        ..Default::default()
    };
    let mut extractor = crate::extractors::TextExtractor::with_config(config);
    let resources = doc.get_page_resources(0).unwrap();
    extractor.set_resources(resources.clone());
    extractor.set_document(&doc);
    doc.load_fonts_public(&resources, &mut extractor).unwrap();
    let spans = extractor
        .extract_text_spans(&doc.get_page_content_data(0).unwrap())
        .unwrap();
    assert_eq!(span_text(&spans), "", "spans of the dropped group");
}

/// With no run merging the empty cells add no space, so the group stays under the
/// gate. The verdict run of a char run uses the merging setting of that run.
#[test]
fn test_merging_setting_reaches_the_verdict_of_a_char_run() {
    let text = "Kestrel abcdefghij abcdefghij abcdefghij abcdefghij";
    let doc = sixteen_line_doc(|y| {
        text.char_indices()
            .filter(|(_, c)| *c != ' ')
            .map(|(col, c)| format!("1 0 0 1 {} {y} Tm ({c}) Tj\n", 29 + 6 * col))
            .collect()
    });
    let chars = direct_chars(&doc, crate::extractors::TextExtractor::new());
    assert!(chars.contains("Kestrel"), "default setting: {chars:?}");
    let off = crate::extractors::SpanMergingConfig {
        merge_tm_tj_runs: false,
        ..Default::default()
    };
    let extractor = crate::extractors::TextExtractor::new().with_merging_config(off);
    let chars = direct_chars(&doc, extractor);
    assert!(!chars.contains("Kestrel"), "no run merging: {chars:?}");
}

/// A threshold that no offset passes means that the `TJ` gaps add no space, so the
/// group stays under the gate. The verdict run of a char run uses that threshold.
#[test]
fn test_extraction_setting_reaches_the_verdict_of_a_char_run() {
    let doc = sixteen_line_doc(|y| {
        let gap = ") -600 (";
        let line = ["Kestrel", "abcdefghij", "abcdefghij", "abcdefghij", "abcdefghij"].join(gap);
        format!("1 0 0 1 29 {y} Tm [({line})] TJ\n")
    });
    let chars = direct_chars(&doc, crate::extractors::TextExtractor::new());
    assert!(chars.contains("Kestrel"), "default setting: {chars:?}");
    let high = crate::extractors::TextExtractionConfig {
        space_insertion_threshold: -100000.0,
        ..Default::default()
    };
    let chars = direct_chars(&doc, crate::extractors::TextExtractor::with_config(high));
    assert!(!chars.contains("Kestrel"), "no offset passes the threshold: {chars:?}");
}

/// The page ends without `ET`, so its last span is still open when the stream ends.
/// The verdict run counts that span, so the copy in the form is dropped in chars as
/// it is in spans.
#[test]
fn test_verdict_run_counts_the_span_open_at_the_end_of_the_page() {
    let line = "Kestrel paragraph about ferry cables";
    let copy = placed(&text_lines_stream(&vec![line.to_string(); 30], 20, 680));
    let page = format!(
        "/Fo Do\nBT /F1 4 Tf 1 0 0 1 5 700 Tm ({}) Tj\n",
        format!("{line} ").repeat(30)
    );
    let pdf = build_forms_pdf(&page, &[("Fo", "[0 0 600 800]", &copy)]);
    let doc = PdfDocument::from_bytes(pdf).expect("parse pdf");
    let spans = span_text(&doc.extract_spans(0).unwrap());
    let chars = char_text(&doc.extract_chars(0).unwrap());
    assert_eq!(spans.matches("Kestrel").count(), 30, "spans: {spans:?}");
    assert_eq!(chars.matches("Kestrel").count(), 30, "chars: {chars:?}");
}
