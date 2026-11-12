use super::*;

fn group(source: &str, visible: &str, is_marker: bool, plain: &str) -> (bool, Vec<String>) {
    let mut retained = PlainWords::new(plain, source);
    let accepted = retains_inline_group(source, visible, is_marker, &mut retained);
    (accepted, retained.words.map(str::to_owned).collect())
}

#[test]
fn a_code_span_group_is_accepted_spelled_or_projected() {
    assert_eq!(group("`a b`", "a b", false, "`a b` tail"), (true, vec!["tail".into()]));
    assert_eq!(group("`a b`", "a b", false, "a b tail"), (true, vec!["tail".into()]));
}

#[test]
fn a_word_inside_a_spelled_group_that_the_source_lacks_fails_the_group() {
    // `x` ends both alternatives at once, so the group fails there.
    assert!(!group("`a b`", "a b", false, "`a x b` tail").0);
}

#[test]
fn an_alternative_that_a_word_killed_stays_dead_when_a_later_word_matches_it() {
    // `a` kills the spelled alternative and "`a" kills the projected one; "`a b`" would
    // otherwise revive and complete the spelled one.
    assert!(!group("`a b`", "a b", false, "a `a b` tail").0);
}

#[test]
fn a_projected_alternative_that_a_word_killed_stays_dead_when_later_words_match_it() {
    assert!(!group("a` a`", "a a", false, "a` a a").0);
}

#[test]
fn a_word_inside_a_projected_group_that_the_source_lacks_fails_the_group() {
    assert!(!group("`a b`", "a b", false, "a x b tail").0);
}

#[test]
fn a_failed_group_stops_reading_at_the_word_that_killed_both_spellings() {
    // `x` ends the spelled and the projected alternative at once, so the words after it are
    // left for the caller instead of being scanned for a late match.
    let (accepted, rest) = group("`a b`", "a b", false, "`a x b` tail");
    assert!(!accepted);
    assert_eq!(rest, ["b`", "tail"]);
}

#[test]
fn an_extra_word_before_a_group_starts_is_skipped_and_the_group_interior_stays_strict() {
    assert_eq!(
        group("**Budget**", "Budget", false, "Summary Budget 2030"),
        (true, vec!["2030".into()])
    );
    assert!(!group("`a b`", "a b", false, "x a y b").0);
}

#[test]
fn a_group_that_the_plain_words_end_inside_is_not_accepted() {
    assert!(!group("`a b`", "a b", false, "`a").0);
}

#[test]
fn a_block_marker_that_projects_to_nothing_is_consumed_only_when_the_element_spells_it() {
    assert_eq!(group("#", "", true, "# Status"), (true, vec!["Status".into()]));
    assert_eq!(group("#", "", true, "Status"), (true, vec!["Status".into()]));
    assert_eq!(
        group("#", "", true, "## Status"),
        (true, vec!["##".into(), "Status".into()])
    );
}

#[test]
fn mixed_plain_drops_a_repeat_of_projected_prose() {
    let accepted = "Caf&#233; Café";
    let plain = "Café Café Café Summary";
    let document = crate::types::internal::InternalDocument::new("pdf");
    let repeated = check_accepted_ocr_tokens(accepted, plain, &document, None).unwrap();

    let filtered = without_repeated_words(plain, &repeated);
    assert_eq!(filtered, "Café Café Summary");
    assert!(
        check_accepted_ocr_tokens(accepted, &filtered, &document, None)
            .unwrap()
            .is_empty()
    );

    let accepted = "Caf&#233;";
    let plain = "Café Summary Café";
    let repeated = check_accepted_ocr_tokens(accepted, plain, &document, None).unwrap();
    assert_eq!(without_repeated_words(plain, &repeated), "Café Summary");
}

fn render_mixed_plain_with_captions(captions: &[(&str, f64)]) -> String {
    use crate::types::internal::{ElementKind, InternalDocument, InternalElement};
    use crate::types::{BoundingBox, Table};

    let cells = vec![vec!["North".into(), "10".into()]];
    let accepted = crate::rendering::common::render_table_markdown(&cells);
    let mut document = InternalDocument::new("pdf");
    for &(caption, y0) in captions {
        let mut element = InternalElement::text(ElementKind::Paragraph, caption, 0).with_page(1);
        element.bbox = Some(BoundingBox {
            x0: 20.0,
            y0,
            x1: 120.0,
            y1: y0 + 10.0,
        });
        document.push_element(element);
    }
    let table_index = document.push_table(Table {
        cells,
        page_number: 1,
        ..Default::default()
    });
    document.push_element(InternalElement::text(ElementKind::Table { table_index }, "", 0).with_page(1));
    reconcile_mixed_plain_page(accepted.trim_end(), &document, 1).expect("reconcile the Plain page")
}

#[test]
fn mixed_plain_keeps_the_caption_and_its_shared_word_in_the_complete_table_row() {
    assert_eq!(
        render_mixed_plain_with_captions(&[("North Summary", 100.0)]),
        "North Summary\n\nNorth 10"
    );
}

#[test]
fn mixed_plain_drops_only_the_caption_with_the_same_source_bounds() {
    assert_eq!(
        render_mixed_plain_with_captions(&[("Summary", 100.0), ("Summary", 100.0), ("Summary", 80.0)]),
        "Summary\n\nSummary\n\nNorth 10"
    );
}

#[test]
fn mixed_plain_table_rows_must_share_a_renderer_element() {
    let plain = "North 10\nSouth 20";
    let cells = vec![vec!["North".into(), "10".into()], vec!["South".into(), "20".into()]];
    let first_row_end = "North 10\n".len();
    let separate_elements = [(0, first_row_end), (first_row_end, plain.len())];
    assert!(!PlainWords::new(plain, plain).retains_table(&cells, Some(&separate_elements)));
    assert!(PlainWords::new(plain, plain).retains_table(&cells, Some(&[(0, plain.len())])));
}
