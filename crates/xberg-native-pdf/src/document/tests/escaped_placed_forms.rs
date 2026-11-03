use super::super::PdfDocument;
use super::common::{build_forms_pdf, build_pages_pdf, text_lines_stream};
use crate::content::{Operator, parse_content_stream};
use crate::extractors::SpanMergingConfig;

const PAGE: &str = "[0 0 600 800]";
const NAMES: [&str; 6] = [
    "PlacedPDF",
    "Placed#50DF",
    "#50lacedPDF",
    "PlacedP#44#46",
    "P#6cacedPDF",
    "#50#6c#61#63#65#64#50#44#46",
];

fn marked(name: &str, operator: &str, body: &str) -> String {
    let properties = if operator == "BDC" { " /MC0" } else { "" };
    format!("/{name}{properties} {operator}\n{body}EMC\n")
}

fn body_lines() -> Vec<String> {
    (0..20)
        .map(|index| format!("Escapedbody line {index} holds unique river sediment samples"))
        .collect()
}

fn letters(text: &str) -> String {
    text.chars().filter(char::is_ascii_alphabetic).collect()
}

fn tagged_names(stream: &str) -> Vec<String> {
    parse_content_stream(stream.as_bytes())
        .expect("parse synthetic form stream")
        .into_iter()
        .filter_map(|operator| match operator {
            Operator::BeginMarkedContent { tag } | Operator::BeginMarkedContentDict { tag, .. } => Some(tag),
            _ => None,
        })
        .collect()
}

fn form_pdf(form: &str, page_zero: &str, nested: bool) -> Vec<u8> {
    let forms = if nested {
        vec![("Fo", PAGE, "/Fi Do\n"), ("Fi", PAGE, form)]
    } else {
        vec![("Fo", PAGE, form)]
    };
    build_pages_pdf(&[(PAGE, page_zero), (PAGE, "/Fo Do\n")], &forms, "", "")
}

type EarlierRead = fn(&PdfDocument);

fn earlier_reads() -> [EarlierRead; 5] {
    [
        |document| drop(document.extract_spans(0).unwrap()),
        |document| drop(document.extract_chars(0).unwrap()),
        |document| drop(document.extract_text(0).unwrap()),
        |document| {
            drop(
                document
                    .extract_spans_with_config(0, SpanMergingConfig::default())
                    .unwrap(),
            )
        },
        |document| drop(document.extract_words_with_thresholds(0, None, None).unwrap()),
    ]
}

fn form_views(pdf: &[u8], earlier: Option<EarlierRead>) -> [String; 2] {
    [false, true].map(|characters| {
        let document = PdfDocument::from_bytes(pdf.to_vec()).expect("open actual two-page PDF");
        assert_eq!(document.page_count().unwrap(), 2);
        assert_eq!(document.get_page_content_data(1).unwrap(), b"/Fo Do\n");
        if let Some(read) = earlier {
            read(&document);
        }
        if characters {
            document
                .extract_chars(1)
                .unwrap()
                .iter()
                .map(|character| character.char)
                .collect()
        } else {
            document
                .extract_spans(1)
                .unwrap()
                .iter()
                .map(|span| span.text.as_str())
                .collect()
        }
    })
}

#[test]
fn escaped_placed_form_body_matches_literal_in_fresh_spans_and_chars() {
    let lines = body_lines();
    let body = text_lines_stream(&lines, 20, 380);
    let expected = letters(&lines.join(""));
    assert!(expected.len() > 800);
    let mut checked = 0;
    for operator in ["BMC", "BDC"] {
        for name in NAMES {
            let form = marked(name, operator, &body);
            assert_eq!(tagged_names(&form), ["PlacedPDF"]);
            for actual in form_views(&form_pdf(&form, "/Fo Do\n", false), None) {
                assert_eq!(letters(&actual), expected, "{name} {operator}");
            }
            checked += 1;
        }
    }
    assert_eq!(checked, 12);
}

#[test]
fn escaped_placed_form_is_independent_of_earlier_page_reads_and_wrapper_cache() {
    let lines = body_lines();
    let body = text_lines_stream(&lines, 20, 380);
    let expected = letters(&lines.join(""));
    let page_lines: Vec<String> = (0..30)
        .map(|index| format!("Pagecontext line {index} describes glacier weather observations"))
        .collect();
    let page_zero = format!(
        "{}/Fo Do\n",
        marked("PlacedPDF", "BDC", &text_lines_stream(&page_lines, 20, 780))
    );
    let mut checked = 0;
    for name in NAMES {
        let form = marked(name, "BDC", &body);
        for nested in [false, true] {
            let pdf = form_pdf(&form, &page_zero, nested);
            for earlier in std::iter::once(None).chain(earlier_reads().map(Some)) {
                for actual in form_views(&pdf, earlier) {
                    assert_eq!(letters(&actual), expected, "{name}, nested={nested}");
                }
                checked += 1;
            }
        }
    }
    assert_eq!(checked, 72);
}

#[test]
fn escaped_placed_duplicate_form_leaves_one_body_in_both_paint_orders() {
    let lines = body_lines();
    let body = text_lines_stream(&lines, 20, 780);
    let copy = text_lines_stream(&lines, 29, 771);
    let expected = letters(&lines.join(""));
    for name in NAMES {
        let form = marked(name, "BDC", &copy);
        for page in [format!("{body}/Fo Do\n"), format!("/Fo Do\n{body}")] {
            let pdf = build_pages_pdf(&[(PAGE, &page)], &[("Fo", PAGE, &form)], "", "");
            let document = PdfDocument::from_bytes(pdf).expect("open duplicate form PDF");
            let spans: String = document
                .extract_spans(0)
                .unwrap()
                .iter()
                .map(|span| span.text.as_str())
                .collect();
            let chars: String = document
                .extract_chars(0)
                .unwrap()
                .iter()
                .map(|character| character.char)
                .collect();
            assert_eq!(letters(&spans), expected, "{name} spans");
            assert_eq!(letters(&chars), expected, "{name} chars");
        }
    }
}

#[test]
fn ordinary_form_names_are_not_decoded_twice_or_classified_by_substring() {
    let lines = ["Ordinary body remains visible".to_string()];
    let body = text_lines_stream(&lines, 20, 380);
    let expected = letters(&lines.join(""));
    for name in [
        "Placed#2350DF",
        "PlacedPDFExtra",
        "xPlacedPDF",
        "placedPDF",
        "Placed#50DF#20",
        "Placed#5",
        "Placed#GGDF",
    ] {
        let form = marked(name, "BDC", &body);
        assert_ne!(tagged_names(&form), ["PlacedPDF"]);
        let pdf = form_pdf(&form, "/Fo Do\n", false);
        for actual in form_views(&pdf, None) {
            assert_eq!(letters(&actual), expected, "{name}");
        }
        let document = PdfDocument::from_bytes(pdf).expect("open ordinary form PDF");
        drop(document.extract_spans(1).unwrap());
        assert_eq!(document.xobject_spans_cache.lock().unwrap().len(), 1, "{name}");
    }
}

#[test]
fn comments_strings_and_property_names_do_not_make_a_form_page_dependent() {
    let line = "Ordinary literal /Placed#50DF and /PlacedPDF remain complete".to_string();
    let body = text_lines_stream(std::slice::from_ref(&line), 20, 380);
    for form in [
        format!("% /PlacedPDF /MC0 BDC\n% /Placed#50DF BMC\n{body}"),
        body.clone(),
        marked("Span", "BDC", &body).replace("/MC0", "/Placed#50DF"),
        format!("/Span << /Unknown /PlacedPDF /Other /Placed#50DF >> BDC\n{body}EMC\n"),
    ] {
        let pdf = form_pdf(&form, "/Fo Do\n", false);
        for actual in form_views(&pdf, None) {
            assert_eq!(letters(&actual), letters(&line));
        }
        let document = PdfDocument::from_bytes(pdf).expect("open form with ordinary metadata");
        drop(document.extract_spans(1).unwrap());
        assert_eq!(document.xobject_spans_cache.lock().unwrap().len(), 1, "{form}");
    }
}

#[test]
fn small_exact_placed_form_stays_suppressed_and_out_of_the_span_cache() {
    let body = text_lines_stream(&["Decorative note".to_string()], 20, 380);
    for name in NAMES {
        let form = marked(name, "BDC", &body);
        let pdf = form_pdf(&form, "/Fo Do\n", false);
        assert_eq!(form_views(&pdf, None), ["", ""]);
        let document = PdfDocument::from_bytes(pdf).expect("open decorative form PDF");
        drop(document.extract_spans(1).unwrap());
        assert_eq!(document.xobject_spans_cache.lock().unwrap().len(), 0, "{name}");
    }
}

#[test]
fn test_child_outside_parent_scope_keeps_its_group_inside_a_page_placed_scope() {
    let lines: Vec<String> = (0..30)
        .map(|index| format!("Tundra paragraph number {index} about ferry cables"))
        .collect();
    let unique: Vec<String> = (0..30)
        .map(|i| format!("Zephyr kite {i} drifts across salt flats"))
        .collect();
    let copy = format!("/PlacedPDF /MC0 BDC\n{}EMC\n", text_lines_stream(&lines, 29, 771));
    let child = format!("/PlacedPDF /MC1 BDC\n{}EMC\n", text_lines_stream(&unique, 20, 380));
    let body = text_lines_stream(&lines, 20, 780);
    for parent in [
        format!("{copy}/Fc Do\n"),
        format!("/Fc Do\n{copy}"),
        format!("{copy}/Span BMC\nq /Fc Do Q\nEMC\n"),
    ] {
        let wrapped = "/PlacedPDF BMC\nq /Fo Do Q\nEMC\n";
        for page in [format!("{body}{wrapped}"), format!("{wrapped}{body}")] {
            let pdf = build_forms_pdf(&page, &[("Fo", PAGE, &parent), ("Fc", PAGE, &child)]);
            let doc = PdfDocument::from_bytes(pdf).expect("parse pdf");
            let spans: String = doc
                .extract_spans(0)
                .unwrap()
                .iter()
                .map(|span| span.text.as_str())
                .collect();
            let chars: String = doc
                .extract_chars(0)
                .unwrap()
                .iter()
                .map(|character| character.char)
                .collect();
            let text = doc.extract_text(0).unwrap();
            for actual in [&spans, &chars, &text] {
                assert_eq!(
                    actual.matches("Zephyr kite 7 drifts").count(),
                    1,
                    "{page:?}: {actual:?}"
                );
                assert_eq!(
                    actual.matches("Tundra paragraph number 7 ").count(),
                    1,
                    "{page:?}: {actual:?}"
                );
            }
        }
    }
}

fn late_placed_form(name: &str) -> String {
    const GRAPHICS_OPERATORS: usize = 1_000_001;
    let mut form = "0 0 m\n".repeat(GRAPHICS_OPERATORS);
    let body = text_lines_stream(&body_lines(), 20, 380);
    form.push_str(&marked(name, "BMC", &body));
    assert!(form.len() > GRAPHICS_OPERATORS * 6);
    form
}

#[test]
fn placed_form_after_a_long_graphics_prefix_keeps_fresh_body_and_disallows_cache() {
    let expected = letters(&body_lines().join(""));
    assert!(expected.len() > 800);
    for name in ["PlacedPDF", "Placed#50DF"] {
        let form = late_placed_form(name);
        let pdf = form_pdf(&form, "/Fo Do\n", false);
        for actual in form_views(&pdf, None) {
            assert_eq!(letters(&actual), expected, "{name}");
        }
        let document = PdfDocument::from_bytes(pdf).expect("open form after long graphics prefix");
        drop(document.extract_spans(1).unwrap());
        assert_eq!(document.xobject_spans_cache.lock().unwrap().len(), 0, "{name}");
    }
}

#[test]
fn placed_form_after_a_long_graphics_prefix_is_independent_of_earlier_page_reads() {
    let page_lines: Vec<String> = (0..30)
        .map(|index| format!("Pagecontext line {index} describes glacier weather observations"))
        .collect();
    let page_zero = format!(
        "{}/Fo Do\n",
        marked("PlacedPDF", "BDC", &text_lines_stream(&page_lines, 20, 780))
    );
    let expected = letters(&body_lines().join(""));
    for name in ["PlacedPDF", "Placed#50DF"] {
        let pdf = form_pdf(&late_placed_form(name), &page_zero, false);
        for earlier in earlier_reads() {
            for actual in form_views(&pdf, Some(earlier)) {
                assert_eq!(letters(&actual), expected, "{name}");
            }
        }
        let document = PdfDocument::from_bytes(pdf).expect("open shared late-tagged form");
        drop(document.extract_spans(0).unwrap());
        assert_eq!(
            document.xobject_spans_cache.lock().unwrap().len(),
            0,
            "{name}, after page zero"
        );
        let spans = document.extract_spans(1).unwrap();
        let actual: String = spans.iter().map(|span| span.text.as_str()).collect();
        assert_eq!(letters(&actual), expected, "{name}, after page zero");
        assert_eq!(
            document.xobject_spans_cache.lock().unwrap().len(),
            0,
            "{name}, after page one"
        );
    }
}

#[test]
fn placed_tag_bytes_inside_an_inline_image_do_not_classify_an_ordinary_form() {
    let pixels = "/PlacedPDF BMC /Placed#50DF /MC0 BDC";
    let line = "Ordinary body after an inline image".to_string();
    let body = text_lines_stream(std::slice::from_ref(&line), 20, 380);
    let form = format!("BI /W {} /H 1 /BPC 8 /CS /G ID\n{pixels}\nEI\n{body}", pixels.len());
    let pdf = form_pdf(&form, "/Fo Do\n", false);
    for actual in form_views(&pdf, None) {
        assert_eq!(letters(&actual), letters(&line));
    }
    let document = PdfDocument::from_bytes(pdf).expect("open ordinary form with inline image");
    drop(document.extract_spans(1).unwrap());
    assert_eq!(document.xobject_spans_cache.lock().unwrap().len(), 1);
}
