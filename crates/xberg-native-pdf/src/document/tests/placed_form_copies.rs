use super::super::*;
use super::common::*;
use super::placed_form_verdicts::{placed, six_word_lines};
use crate::config::ExtractionProfile;
use crate::extractors::SpanMergingConfig;

const FULL_PAGE: &str = "[0 0 600 800]";
const HARBOUR: &str = "The harbour office opens at dawn and the pilots gather near the old crane to read \
    the tide tables before the first ferry leaves for the outer islands where the keepers still trade salted \
    fish for lamp oil and news from the mainland while gulls circle above the nets and the market stalls fill \
    slowly with baskets of mussels and bundles of dried seaweed that the older women carry down from the \
    terraces every morning in all weather and sell by weight to the cooks from the three inns along the quay \
    who argue about the price until the bell rings for the noon boat and everyone turns to watch the mail \
    sacks swing ashore from the deck of the little steamer that has served this coast for longer than anyone \
    can remember and that the children of the town still run down the hill to greet on every summer evening \
    when the sun sits low over the breakwater and the smell of tar and woodsmoke drifts across the water";
const LIBRARY: &str = "Our committee reviewed the proposed budget for the school library and found that the \
    cost of new shelving would exceed the amount set aside by the district last spring so the members agreed \
    to ask the parents association for a small grant and to delay the purchase of the reading chairs until \
    the autumn term when the treasurer expects the fundraising dinner to cover most of the remaining balance \
    and the head teacher has promised to report on how many pupils use the room during the lunch hour because \
    the survey that was taken in the winter suggested that younger children rarely visit without a teacher \
    and that the opening times do not suit the families who depend on the afternoon bus to get home after \
    the last lesson of the day which is why the governors will discuss a later closing time at their next \
    meeting and will write to every household once the new timetable has been approved by the whole board";

fn body_lines() -> Vec<String> {
    (0..30)
        .map(|i| format!("Kestrel{i:02} harbour lantern morning channel sailors weather"))
        .collect()
}

fn prose_lines(text: &str) -> Vec<String> {
    let words: Vec<&str> = text.split_whitespace().collect();
    words.chunks(9).map(|chunk| chunk.join(" ")).collect()
}

/// The inside of a `TJ`. Every word is cut after four letters by the offset -150,
/// past the threshold that makes an offset a word break.
fn cut_words(words: &[&str]) -> String {
    words
        .iter()
        .map(|word| format!("({}) -150 ({} ) ", &word[..4], &word[4..]))
        .collect()
}

/// One `TJ` of cut words for each line.
fn widening_tj(lines: &[String], x: i32, top: i32) -> String {
    let mut out = String::from("BT /F1 10 Tf\n");
    for (i, line) in lines.iter().enumerate() {
        let words: Vec<&str> = line.split(' ').collect();
        let cuts = cut_words(&words);
        out.push_str(&format!("1 0 0 1 {x} {} Tm [{cuts}] TJ\n", top - 12 * i as i32));
    }
    out + "ET\n"
}

/// The first four letters of each word in a `Tj`, the rest in a `TJ`.
fn tj_then_tj(lines: &[String], x: i32, top: i32) -> String {
    let mut out = String::from("BT /F1 10 Tf\n");
    for (i, line) in lines.iter().enumerate() {
        out.push_str(&format!("1 0 0 1 {x} {} Tm\n", top - 12 * i as i32));
        for word in line.split(' ') {
            out.push_str(&format!("({}) Tj [({} )] TJ\n", &word[..4], &word[4..]));
        }
    }
    out + "ET\n"
}

fn ink(text: &str) -> usize {
    text.chars().filter(|c| !c.is_whitespace()).count()
}

/// Letters of the page in spans, in chars and in the words of the Academic profile.
fn page_ink(doc: &PdfDocument, page: usize) -> [usize; 3] {
    let spans = doc.extract_spans(page).unwrap().iter().map(|s| ink(&s.text)).sum();
    let chars = doc
        .extract_chars(page)
        .unwrap()
        .iter()
        .filter(|c| !c.char.is_whitespace())
        .count();
    let words = doc
        .extract_words_with_thresholds(page, None, Some(ExtractionProfile::ACADEMIC))
        .unwrap()
        .iter()
        .map(|w| ink(&w.text))
        .sum();
    [spans, chars, words]
}

/// The placed form paints `form` beside the page text `body`, in both paint orders.
fn ink_of_both_orders(body: &str, form: &str) -> [[usize; 3]; 2] {
    let form = placed(form);
    [format!("{body}/Fo Do\n"), format!("/Fo Do\n{body}")].map(|page| {
        let forms = [("Fo", FULL_PAGE, form.as_str())];
        page_ink(
            &PdfDocument::from_bytes(build_forms_pdf(&page, &forms)).expect("parse pdf"),
            0,
        )
    })
}

/// `form` repeats `body`: the page reads as `body` alone in every view and order.
fn assert_copy_leaves(body: &str, form: &str) {
    let alone = PdfDocument::from_bytes(build_forms_pdf(body, &[])).expect("parse pdf");
    let want = page_ink(&alone, 0);
    assert!(want.iter().all(|&count| count > 800), "the body reads {want:?}");
    assert_eq!(ink_of_both_orders(body, form), [want; 2]);
}

/// `form` is its own text: the page keeps it in every view and order.
fn assert_form_stays(body: &str, form: &str, form_ink: usize) {
    let alone = PdfDocument::from_bytes(build_forms_pdf(body, &[])).expect("parse pdf");
    let want = page_ink(&alone, 0).map(|count| count + form_ink);
    assert_eq!(ink_of_both_orders(body, form), [want; 2]);
}

#[test]
fn test_copy_with_words_cut_by_a_tj_offset_leaves_every_view() {
    let lines = body_lines();
    assert_copy_leaves(&text_lines_stream(&lines, 20, 780), &widening_tj(&lines, 29, 771));
}

#[test]
fn test_copy_with_words_split_by_tj_then_tj_leaves_every_view() {
    let lines = body_lines();
    assert_copy_leaves(&text_lines_stream(&lines, 20, 780), &tj_then_tj(&lines, 29, 771));
}

/// The cuts are in the page text and the copy is plain.
#[test]
fn test_plain_copy_of_a_cut_body_leaves_every_view() {
    let lines = body_lines();
    assert_copy_leaves(&widening_tj(&lines, 20, 780), &text_lines_stream(&lines, 29, 771));
}

/// The words of each line come in the other order, so no line matches. The word
/// measure still sees the copy.
#[test]
fn test_copy_with_its_words_in_another_order_leaves_every_view() {
    let lines = body_lines();
    let turned: Vec<String> = lines
        .iter()
        .map(|line| line.split(' ').rev().collect::<Vec<_>>().join(" "))
        .collect();
    assert_copy_leaves(
        &text_lines_stream(&lines, 20, 780),
        &text_lines_stream(&turned, 29, 771),
    );
}

/// Word tokens are ASCII, so only the line measure sees a copy in other alphabets.
#[test]
fn test_copy_in_letters_outside_ascii_leaves_every_view() {
    let lines: Vec<String> = (0..30)
        .map(|i| {
            (0..7)
                .map(|w| format!("\\3{}{}\\35{}\\34{}\\33{}", i / 8, i % 8, w, (i + w) % 8, w))
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect();
    assert_copy_leaves(&text_lines_stream(&lines, 20, 780), &text_lines_stream(&lines, 29, 771));
}

/// Two texts share only common words, and none of their lines.
#[test]
fn test_own_text_stays_beside_a_longer_text_in_the_same_language() {
    let body = text_lines_stream(&prose_lines(HARBOUR), 20, 780);
    assert_form_stays(&body, &text_lines_stream(&prose_lines(LIBRARY), 29, 771), ink(LIBRARY));
}

/// The lines come in the other order and the offsets cut every word, so only whole
/// lines show the copy.
#[test]
fn test_cut_copy_with_its_lines_in_another_order_leaves_every_view() {
    let lines = body_lines();
    let turned: Vec<String> = lines.iter().rev().cloned().collect();
    assert_copy_leaves(&text_lines_stream(&lines, 20, 780), &widening_tj(&turned, 29, 771));
}

#[test]
fn test_cut_copy_in_capitals_leaves_every_view() {
    let lines = body_lines();
    let capitals: Vec<String> = lines.iter().map(|line| line.to_uppercase()).collect();
    assert_copy_leaves(&text_lines_stream(&lines, 20, 780), &widening_tj(&capitals, 29, 771));
}

/// The second half of each word sits one point higher. The raised text stays on the
/// line of the word.
#[test]
fn test_cut_copy_with_raised_text_leaves_every_view() {
    let lines = body_lines();
    let mut raised = String::from("BT /F1 10 Tf\n");
    for (i, line) in lines.iter().enumerate() {
        let y = 771 - 12 * i as i32;
        let words: Vec<&str> = line.split(' ').collect();
        let (first, rest) = words.split_at(words.len() / 2);
        raised.push_str(&format!("1 0 0 1 29 {y} Tm [{}] TJ\n", cut_words(first)));
        raised.push_str(&format!("1 0 0 1 200 {} Tm [{}] TJ\n", y + 1, cut_words(rest)));
    }
    assert_copy_leaves(&text_lines_stream(&lines, 20, 780), &(raised + "ET\n"));
}

/// Lines of exactly the shortest length that counts. Two columns fit them on the page.
#[test]
fn test_cut_copy_with_lines_of_eight_letters_leaves_every_view() {
    let lines: Vec<String> = (0..110).map(|i| format!("Harb{i:04}")).collect();
    let (left, right) = lines.split_at(55);
    let body = text_lines_stream(left, 20, 780) + &text_lines_stream(right, 220, 780);
    let copy = widening_tj(left, 29, 771) + &widening_tj(right, 229, 771);
    assert_copy_leaves(&body, &copy);
}

/// Rows of the form and of the page, with `cells` values in each row.
fn grid_lines(count: usize, cells: usize, pattern: impl Fn(usize, usize) -> bool) -> Vec<String> {
    (0..count)
        .map(|row| {
            (0..cells)
                .map(|cell| if pattern(row, cell) { "Y" } else { "N" })
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect()
}

/// Rows of seven letters take few values, so most rows of the form also stand on the
/// page, and the form shares no table with it.
#[test]
fn test_grid_with_rows_of_seven_letters_stays() {
    let lines = |shift: usize| grid_lines(62, 7, |row, cell| ((row + shift) / (cell + 1)).is_multiple_of(2));
    let form = lines(5);
    let body = text_lines_stream(&lines(0), 20, 780);
    let form_ink = form.iter().map(|line| ink(line)).sum();
    assert_form_stays(&body, &text_lines_stream(&form, 300, 780), form_ink);
}

/// The form repeats four rows of eight letters more often than the page shows them.
#[test]
fn test_form_with_a_line_repeated_more_often_than_on_the_page_stays() {
    let rows = |count| grid_lines(count, 8, |row, cell| (row % 4) >> (cell % 2) & 1 == 0);
    let form = rows(60);
    let body = text_lines_stream(&rows(20), 20, 780);
    let form_ink = form.iter().map(|line| ink(line)).sum();
    assert_form_stays(&body, &text_lines_stream(&form, 300, 780), form_ink);
}

/// Each row of the form repeats a long label from the page header, with its own codes.
#[test]
fn test_table_rows_with_a_long_label_from_the_page_stay() {
    let label = "Regional Transportation Commission";
    let header: Vec<String> = (0..30).map(|i| format!("{label} annual statement part {i}")).collect();
    let rows: Vec<String> = (0..30)
        .map(|i| format!("{label} a{i} b{} c{} d{}", i * 3, i * 7, i * 11))
        .collect();
    let form_ink = rows.iter().map(|line| ink(line)).sum();
    assert_form_stays(
        &text_lines_stream(&header, 20, 780),
        &text_lines_stream(&rows, 29, 771),
        form_ink,
    );
}

/// Serial numbers share a long prefix with other serial numbers on the page.
#[test]
fn test_serial_numbers_with_a_long_prefix_stay() {
    let ids = |start: usize, rows: usize| -> Vec<String> {
        (0..rows)
            .map(|row| {
                (0..3)
                    .map(|i| format!("MUNICIPALPERMIT{:04}", start + row * 3 + i))
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect()
    };
    let form = ids(5000, 30);
    let form_ink = form.iter().map(|line| ink(line)).sum();
    assert_form_stays(
        &text_lines_stream(&ids(1000, 30), 20, 780),
        &text_lines_stream(&form, 29, 771),
        form_ink,
    );
}

/// Two grids of 30 symbols a row share every short run and no row.
#[test]
fn test_grid_of_two_symbols_stays() {
    let hash = |row: usize, cell: usize| {
        let seed = (row * 31 + cell + 1) as u64;
        seed.wrapping_mul(6364136223846793005).wrapping_mul(6364136223846793005) >> 40 & 1 == 0
    };
    let body = grid_lines(30, 30, hash);
    let form = grid_lines(30, 30, |row, cell| hash(row + 1000, cell));
    let form_ink = form.iter().map(|line| ink(line)).sum();
    assert_form_stays(
        &text_lines_stream(&body, 20, 780),
        &text_lines_stream(&form, 29, 771),
        form_ink,
    );
}

fn child_pages() -> [(&'static str, [String; 2], usize); 3] {
    let scope = placed("/C Do\n");
    let plain = "/C Do\n".to_string();
    [
        ("tagged parent, then a scope", ["/P Do\n".to_string(), scope.clone()], 0),
        ("scope, then no scope", [scope.clone(), plain.clone()], 240),
        ("no scope, then a scope", [plain, scope], 0),
    ]
}

type Route = fn(&PdfDocument);

/// Each call that reads page 0 first.
fn earlier_calls() -> [(&'static str, Route); 5] {
    [
        ("spans", |doc| drop(doc.extract_spans(0).unwrap())),
        ("chars", |doc| drop(doc.extract_chars(0).unwrap())),
        ("text", |doc| drop(doc.extract_text(0).unwrap())),
        ("config", |doc| {
            drop(doc.extract_spans_with_config(0, SpanMergingConfig::default()).unwrap())
        }),
        ("words", |doc| {
            drop(
                doc.extract_words_with_thresholds(0, None, Some(ExtractionProfile::ACADEMIC))
                    .unwrap(),
            )
        }),
    ]
}

/// Page 0 paints the child and then a wrapper that paints it again, so the processed
/// guard skips the child inside the wrapper. Page 1 paints the wrapper alone and
/// reads the child, tagged or not, one wrapper level or two.
#[test]
fn test_wrapper_that_skipped_its_child_reads_the_child_on_the_next_page() {
    let lines = six_word_lines("Childw", 40, 780);
    let tagged = placed(&lines);
    for child in [lines.as_str(), tagged.as_str()] {
        let two_levels = [
            ("W", FULL_PAGE, "/V Do\n"),
            ("V", FULL_PAGE, "/C Do\n"),
            ("C", FULL_PAGE, child),
        ];
        for forms in [&two_levels[..], &[("W", FULL_PAGE, "/C Do\n"), two_levels[2]][..]] {
            let pages = [(FULL_PAGE, "/C Do\n/W Do\n"), (FULL_PAGE, "/W Do\n")];
            let open = || PdfDocument::from_bytes(build_pages_pdf(&pages, forms, "", "")).expect("parse pdf");
            let count = |doc: &PdfDocument| {
                let spans: String = doc.extract_spans(1).unwrap().iter().map(|s| s.text.as_str()).collect();
                let chars: String = doc.extract_chars(1).unwrap().iter().map(|c| c.char).collect();
                [spans.matches("Childw").count(), chars.matches("Childw").count()]
            };
            assert_eq!(count(&open()), [240, 240], "fresh");
            for (name, call) in earlier_calls() {
                let doc = open();
                call(&doc);
                assert_eq!(count(&doc), [240, 240], "after {name}");
            }
        }
    }
}

/// A child form reads on page 1 as on a fresh document, whatever read page 0 first.
#[test]
fn test_an_earlier_call_does_not_change_a_child_in_a_placed_scope() {
    let routes = earlier_calls();
    let child = six_word_lines("Childw", 40, 780);
    let parent = placed("/C Do\n");
    let forms = [("P", FULL_PAGE, parent.as_str()), ("C", FULL_PAGE, child.as_str())];
    for (input, streams, want) in child_pages() {
        let pages = [(FULL_PAGE, streams[0].as_str()), (FULL_PAGE, streams[1].as_str())];
        let open = || PdfDocument::from_bytes(build_pages_pdf(&pages, &forms, "", "")).expect("parse pdf");
        let count = |doc: &PdfDocument| {
            let spans: String = doc.extract_spans(1).unwrap().iter().map(|s| s.text.as_str()).collect();
            let chars: String = doc.extract_chars(1).unwrap().iter().map(|c| c.char).collect();
            [spans.matches("Childw").count(), chars.matches("Childw").count()]
        };
        assert_eq!(count(&open()), [want, want], "{input}, fresh");
        for (name, route) in routes {
            let doc = open();
            route(&doc);
            assert_eq!(count(&doc), [want, want], "{input}, after {name}");
        }
    }
}

/// A layer filter or an ink filter on page 1 removes text from a plain form. The
/// next unfiltered read of a page with the same form keeps that text.
#[test]
fn test_a_filtered_read_does_not_enter_the_span_cache() {
    let tint = "<< /FunctionType 2 /Domain [0 1] /C0 [0 0 0 0] /C1 [0 0 0 1] /N 1 >>";
    let spot = format!("[/Separation /Varnish /DeviceCMYK {tint}]");
    let resources = format!("/Properties << /L1 5 0 R >> /ColorSpace << /Spot {spot} >>");
    let catalog = "/OCProperties << /OCGs [5 0 R] /D << /ON [5 0 R] >> >>";
    let form = format!(
        "/OC /L1 BDC\n{}EMC\n/Spot cs 1 scn\n{}0 g\n",
        six_word_lines("Layerw", 4, 780),
        six_word_lines("Inkwrd", 4, 700)
    );
    let page = "/Fo Do\n";
    let pages = [(FULL_PAGE, page), (FULL_PAGE, page)];
    let set = |name: &str| std::iter::once(name.to_string()).collect::<HashSet<String>>();
    let filters = [(set("Hidden"), HashSet::new()), (HashSet::new(), set("Varnish"))];
    for (layers, inks) in filters {
        let pdf = build_pages_pdf(&pages, &[("Fo", FULL_PAGE, &form)], catalog, &resources);
        let doc = PdfDocument::from_bytes(pdf).expect("parse pdf");
        drop(doc.extract_spans_filtered(0, layers, inks).unwrap());
        let spans: String = doc.extract_spans(1).unwrap().iter().map(|s| s.text.as_str()).collect();
        assert_eq!(spans.matches("Layerw").count(), 24, "layer text: {spans:?}");
        assert_eq!(spans.matches("Inkwrd").count(), 24, "ink text: {spans:?}");
    }
}
