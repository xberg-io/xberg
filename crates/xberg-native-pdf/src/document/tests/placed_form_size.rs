use super::super::*;
use super::common::*;
use super::placed_forms::{FULL_BBOX, paint_lines, placed_copy_counts, plain_line};

/// One line of the fixed-pitch size tests.
const CELL_LINE: &str = "Kestrel abcdefghij abcdefghij abcdefghij abcdefghij";

/// `CELL_LINE` 15 times, then its first `last_chars` characters. That is
/// `765 + last_chars` characters with the spaces.
fn cell_lines(last_chars: usize) -> Vec<String> {
    let mut lines = vec![CELL_LINE.to_string(); 15];
    lines.push(CELL_LINE[..last_chars].to_string());
    lines
}

/// One glyph per `Tm` and `Tj` in font `font`, 6 points apart. A space is an empty
/// cell with no glyph.
fn cell_line(font: &str, text: &str, x: i32, y: i32) -> String {
    let glyphs: String = text
        .char_indices()
        .filter(|(_, ch)| *ch != ' ')
        .map(|(col, ch)| format!("1 0 0 1 {} {y} Tm ({ch}) Tj\n", x + col as i32 * 6))
        .collect();
    format!("/{font} 10 Tf\n{glyphs}")
}

/// Needle counts `(spans, chars)` of a page whose only content is a form that paints
/// `form`, with or without a `/PlacedPDF` scope around it.
fn form_counts(form: &str, placed: bool) -> (usize, usize) {
    let form = if placed {
        format!("/PlacedPDF /MC0 BDC\n{form}EMC\n")
    } else {
        form.to_string()
    };
    let pdf = build_forms_pdf("/Fo Do\n", &[("Fo", FULL_BBOX, &form)]);
    let doc = PdfDocument::from_bytes(pdf).expect("parse pdf");
    let spans: String = doc.extract_spans(0).unwrap().iter().map(|s| s.text.as_str()).collect();
    let chars: String = doc.extract_chars(0).unwrap().iter().map(|c| c.char).collect();
    (spans.matches("Kestrel").count(), chars.matches("Kestrel").count())
}

/// 16 lines of 47 painted Courier glyphs and 4 empty cells. Span text holds a space
/// in each cell, 816 characters; the chars hold 752 glyphs. The span text decides a
/// placed form of that page, so all 16 lines stay in both views. The untagged page
/// and the page with literal spaces are the controls.
#[test]
fn test_empty_cells_count_toward_the_placed_size_in_both_views() {
    let lines = vec![CELL_LINE.to_string(); 16];
    let cells = paint_lines(&lines, 29, 771, |text, x, y| cell_line("F2", text, x, y));
    assert_eq!(form_counts(&cells, true), (16, 16), "tagged empty cells");
    assert_eq!(form_counts(&cells, false), (16, 16), "untagged empty cells");
    let literal = paint_lines(&lines, 29, 771, |text, x, y| {
        format!("/F2 10 Tf\n{}", plain_line(text, x, y))
    });
    assert_eq!(form_counts(&literal, true), (16, 16), "tagged literal spaces");
}

/// The size gate reads 799 and 800 the same way in both views, with 63 of those
/// characters in empty cells.
#[test]
fn test_empty_cells_reach_the_placed_size_at_the_same_character_in_both_views() {
    for (last_chars, kept) in [(34, 0), (35, 16)] {
        let form = paint_lines(&cell_lines(last_chars), 29, 771, |text, x, y| {
            cell_line("F2", text, x, y)
        });
        assert_eq!(form_counts(&form, true), (kept, kept), "last line of {last_chars}");
    }
}

/// A smaller page with the same cells, and a proportional font in the same cells,
/// fall under the size gate in both views. A proportional font has no empty cell,
/// so its gaps add nothing to either view.
#[test]
fn test_placed_page_under_the_size_stays_suppressed_in_both_views() {
    let lines = vec![CELL_LINE.to_string(); 15];
    let fewer = paint_lines(&lines, 29, 771, |text, x, y| cell_line("F2", text, x, y));
    assert_eq!(form_counts(&fewer, true), (0, 0), "one line less");
    let lines = vec![CELL_LINE.to_string(); 16];
    let proportional = paint_lines(&lines, 29, 771, |text, x, y| cell_line("F1", text, x, y));
    assert_eq!(form_counts(&proportional, true), (0, 0), "proportional font");
}

/// The span text gives the size of the page's own text too. Its 816 characters with
/// the cells send a copy of 2295 to the duplicate measure, so the copy drops in both.
/// At 752 the copy would pass as the dominant text.
#[test]
fn test_empty_cells_count_toward_the_page_text_beside_a_placed_copy() {
    let lines = vec![CELL_LINE.to_string(); 16];
    let body = paint_lines(&lines, 20, 780, |text, x, y| cell_line("F2", text, x, y));
    let copy = paint_lines(&vec![CELL_LINE.to_string(); 45], 29, 771, plain_line);
    assert_eq!(placed_copy_counts(&body, &copy), [(16, 16); 2], "body first, body last");
}

/// A `TJ` offset after a literal space adds no space to the span text, so it adds
/// nothing to the size: 799 characters stay under the gate in both views.
#[test]
fn test_tj_offset_beside_a_literal_space_adds_nothing_to_the_placed_size() {
    let lines = cell_lines(34);
    let after = paint_lines(&lines, 29, 771, |text, x, y| {
        format!("1 0 0 1 {x} {y} Tm [({})] TJ\n", text.replace(' ', " ) -200 ("))
    });
    assert_eq!(form_counts(&after, true), (0, 0), "offset after the space");
    let before = paint_lines(&lines, 29, 771, |text, x, y| {
        format!("1 0 0 1 {x} {y} Tm [({})] TJ\n", text.replace(' ', ") -200 ( "))
    });
    assert_eq!(form_counts(&before, true), (0, 0), "offset before the space");
}

/// An offset that stands for a space counts in the span text: each one replaces the
/// space of its line, so 799 stays under the gate and 800 reaches it.
#[test]
fn test_tj_offset_that_stands_for_a_space_counts_in_both_views() {
    for (last_chars, kept) in [(34, 0), (35, 16)] {
        let form = paint_lines(&cell_lines(last_chars), 29, 771, |text, x, y| {
            format!("1 0 0 1 {x} {y} Tm [({})] TJ\n", text.replace(' ', ") -200 ("))
        });
        assert_eq!(form_counts(&form, true), (kept, kept), "last line of {last_chars}");
    }
}

/// After a literal space, a second offset in a row opens a new span, so it stands
/// for a space in the span text. Each gap holds 2 characters: 14 lines of 55, then a
/// last line of 29 or 30.
#[test]
fn test_second_tj_offset_after_a_literal_space_counts_in_both_views() {
    for (last_chars, kept) in [(27, 0), (28, 15)] {
        let mut lines = vec![CELL_LINE.to_string(); 14];
        lines.push(CELL_LINE[..last_chars].to_string());
        let form = paint_lines(&lines, 29, 771, |text, x, y| {
            let line = text.replace(' ', " ) -200 -200 (");
            format!("1 0 0 1 {x} {y} Tm [({line})] TJ\n")
        });
        assert_eq!(form_counts(&form, true), (kept, kept), "last line of {last_chars}");
    }
}

/// A `Tj` that ends with a space, then a `TJ` that starts with an offset. The array
/// opens a new span, so its offset stands for a space in the span text: each line
/// holds 52 characters, and the last one 20, so the page reaches 800.
#[test]
fn test_tj_offset_after_a_space_in_an_earlier_tj_counts_in_both_views() {
    let rest = "abcdefghij abcdefghij abcdefghij abcdefghij";
    let mut lines = vec![rest.to_string(); 15];
    lines.push(rest[..11].to_string());
    let form = paint_lines(&lines, 29, 771, |text, x, y| {
        format!("1 0 0 1 {x} {y} Tm (Kestrel ) Tj [-200 ({text})] TJ\n")
    });
    assert_eq!(form_counts(&form, true), (16, 16));
}

/// Stray text in empty cells above a small form's `/BBox` is clipped with its size:
/// 799 characters inside the box stay under the gate and 800 reach it.
#[test]
fn test_clipped_empty_cells_leave_the_placed_size_in_both_views() {
    for (last_chars, kept) in [(34, 0), (35, 16)] {
        let stray = paint_lines(&vec![CELL_LINE.replace('K', "M"); 4], 29, 771, |text, x, y| {
            cell_line("F2", text, x, y)
        });
        let inside = paint_lines(&cell_lines(last_chars), 29, 290, |text, x, y| {
            format!("/F2 10 Tf\n{}", plain_line(text, x, y))
        });
        let form = format!("/PlacedPDF /MC0 BDC\n{stray}{inside}EMC\n");
        let pdf = build_forms_pdf("/Fo Do\n", &[("Fo", "[0 0 400 300]", &form)]);
        let doc = PdfDocument::from_bytes(pdf).expect("parse pdf");
        let spans: String = doc.extract_spans(0).unwrap().iter().map(|s| s.text.as_str()).collect();
        let chars: String = doc.extract_chars(0).unwrap().iter().map(|c| c.char).collect();
        let counts = (spans.matches("Kestrel").count(), chars.matches("Kestrel").count());
        assert_eq!(counts, (kept, kept), "last line of {last_chars}");
    }
}

/// A small `/PlacedPDF` scope on the page stays suppressed, so its empty cells add
/// nothing to the text before it: the form of 799 characters stays under the gate.
#[test]
fn test_empty_cells_in_suppressed_text_add_nothing_to_the_placed_size() {
    let form = paint_lines(&cell_lines(34), 29, 771, plain_line);
    let form = format!("/PlacedPDF /MC0 BDC\n{form}EMC\n");
    let figure = paint_lines(&vec![CELL_LINE.replace('K', "M"); 4], 29, 600, |text, x, y| {
        cell_line("F2", text, x, y)
    });
    let page = format!("/Fo Do\n/PlacedPDF /MC1 BDC\n{figure}EMC\n");
    let pdf = build_forms_pdf(&page, &[("Fo", FULL_BBOX, &form)]);
    let doc = PdfDocument::from_bytes(pdf).expect("parse pdf");
    let spans: String = doc.extract_spans(0).unwrap().iter().map(|s| s.text.as_str()).collect();
    let chars: String = doc.extract_chars(0).unwrap().iter().map(|c| c.char).collect();
    assert_eq!(
        (spans.matches("Kestrel").count(), chars.matches("Kestrel").count()),
        (0, 0)
    );
}

/// One extractor runs a page of empty cells, then a page of 799 characters. The
/// sizes of the first run do not reach the second, which stays under the gate.
#[test]
fn test_second_run_of_an_extractor_does_not_read_the_sizes_of_the_first() {
    let lines = vec![CELL_LINE.to_string(); 16];
    let cells = paint_lines(&lines, 29, 771, |text, x, y| cell_line("F2", text, x, y));
    let plain = paint_lines(&cell_lines(34), 29, 771, plain_line);
    let (cells, plain) = (
        format!("/PlacedPDF /MC0 BDC\n{cells}EMC\n"),
        format!("/PlacedPDF /MC1 BDC\n{plain}EMC\n"),
    );
    let forms = [("Fc", FULL_BBOX, cells.as_str()), ("Fp", FULL_BBOX, plain.as_str())];
    let doc = PdfDocument::from_bytes(build_forms_pdf(" ", &forms)).expect("parse pdf");
    let page = doc.get_page(0).unwrap();
    let resources = page.as_dict().unwrap().get("Resources").unwrap().clone();
    let mut extractor = crate::extractors::TextExtractor::new();
    extractor.set_resources(resources.clone());
    extractor.set_document(&doc);
    doc.load_fonts(&resources, &mut extractor).unwrap();
    assert_eq!(extractor.extract(b"/Fc Do\n").unwrap().len(), 752);
    assert_eq!(extractor.extract(b"/Fp Do\n").unwrap().len(), 0);
}
