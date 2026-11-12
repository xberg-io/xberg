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
    let repeated = check_accepted_ocr_tokens(accepted, plain, &document).unwrap();

    let filtered = without_repeated_words(plain, &repeated);
    assert_eq!(filtered, "Café Café Summary");
    assert!(
        check_accepted_ocr_tokens(accepted, &filtered, &document)
            .unwrap()
            .is_empty()
    );

    let accepted = "Caf&#233;";
    let plain = "Café Summary Café";
    let repeated = check_accepted_ocr_tokens(accepted, plain, &document).unwrap();
    assert_eq!(without_repeated_words(plain, &repeated), "Café Summary");
}
