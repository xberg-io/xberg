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
    let repeated = check_accepted_ocr_tokens(accepted, plain, &document, &[])
        .unwrap()
        .repeated_for_page(&[])
        .unwrap();

    let filtered = without_repeated_words(plain, &repeated);
    assert_eq!(filtered, "Café Café Summary");
    assert!(
        check_accepted_ocr_tokens(accepted, &filtered, &document, &[])
            .unwrap()
            .repeated_for_page(&[])
            .unwrap()
            .is_empty()
    );

    let accepted = "Caf&#233;";
    let plain = "Café Summary Café";
    let repeated = check_accepted_ocr_tokens(accepted, plain, &document, &[])
        .unwrap()
        .repeated_for_page(&[])
        .unwrap();
    assert_eq!(without_repeated_words(plain, &repeated), "Café Summary");
}

fn render_mixed_plain_with_captions(captions: &[(&str, f64)]) -> String {
    render_mixed_plain_with_prose_and_table("", captions)
}

fn render_mixed_plain_with_prose_and_table(prose: &str, captions: &[(&str, f64)]) -> String {
    render_mixed_plain_with_table_bounds(prose, captions, true)
}

fn render_mixed_plain_with_table_bounds(prose: &str, captions: &[(&str, f64)], table_has_bounds: bool) -> String {
    use crate::types::internal::{ElementKind, InternalDocument, InternalElement};
    use crate::types::{BoundingBox, Table};

    let cells = vec![vec!["North".into(), "10".into()]];
    let table = crate::rendering::common::render_table_markdown(&cells);
    let accepted = format!("{prose}\n\n{table}");
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
        bounding_box: table_has_bounds.then_some(BoundingBox {
            x0: 20.0,
            y0: 30.0,
            x1: 120.0,
            y1: 40.0,
        }),
        ..Default::default()
    });
    document.push_element(InternalElement::text(ElementKind::Table { table_index }, "", 0).with_page(1));
    reconcile_mixed_plain_page(accepted.trim(), &document, 1).expect("reconcile the Plain page")
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
    use crate::types::internal::{ElementKind, InternalElement};
    let element = InternalElement::text(ElementKind::Table { table_index: 0 }, "", 0).with_page(1);
    let source = crate::types::Table {
        cells,
        ..Default::default()
    };
    let separate_elements = [
        PlainWriter {
            start: 0,
            end: first_row_end,
            element: &element,
        },
        PlainWriter {
            start: first_row_end,
            end: plain.len(),
            element: &element,
        },
    ];
    assert!(!PlainWords::new(plain, plain).retains_table(0, &source, &separate_elements));
    let one_element = [PlainWriter {
        start: 0,
        end: plain.len(),
        element: &element,
    }];
    assert!(PlainWords::new(plain, plain).retains_table(0, &source, &one_element));
}

#[test]
fn mixed_plain_keeps_distinct_caption_whole_when_accepted_prose_shares_a_word() {
    assert_eq!(
        render_mixed_plain_with_prose_and_table("North office", &[("North office", 150.0), ("North Summary", 100.0)]),
        "North office\n\nNorth Summary\n\nNorth 10"
    );
    assert_eq!(
        render_mixed_plain_with_prose_and_table("North office", &[("North Summary", 150.0), ("North office", 100.0)]),
        "North Summary\n\nNorth office\n\nNorth 10"
    );
}

#[test]
fn mixed_plain_keeps_typed_table_beside_equal_text_from_a_separate_source() {
    assert_eq!(
        render_mixed_plain_with_captions(&[("North 10", 100.0), ("Summary", 80.0)]),
        "North 10\n\nSummary\n\nNorth 10"
    );
    assert_eq!(
        render_mixed_plain_with_captions(&[("North 10", 30.0), ("Summary", 80.0)]),
        "North 10\n\nSummary"
    );
}

#[test]
fn mixed_plain_does_not_restore_duplicate_prose_inside_a_same_source_caption() {
    assert_eq!(
        render_mixed_plain_with_prose_and_table(
            "North office",
            &[("North office", 100.0), ("North office Summary", 100.0)]
        ),
        "North office\n\nSummary\n\nNorth 10"
    );
}

#[test]
fn mixed_plain_requires_table_bounds_before_using_an_embedded_row() {
    use crate::types::internal::{ElementKind, InternalElement};
    use crate::types::{BoundingBox, Table};

    let prose = "Opening\nNorth 10";
    let plain = "Opening\nNorth 10\n\nNorth 10";
    let bounds = BoundingBox {
        x0: 0.0,
        y0: 10.0,
        x1: 40.0,
        y1: 30.0,
    };
    let mut paragraph = InternalElement::text(ElementKind::Paragraph, prose, 0);
    paragraph.bbox = Some(bounds);
    let table = InternalElement::text(ElementKind::Table { table_index: 0 }, "", 0);
    let writers = [
        PlainWriter {
            start: 0,
            end: prose.len() + 2,
            element: &paragraph,
        },
        PlainWriter {
            start: prose.len() + 2,
            end: plain.len(),
            element: &table,
        },
    ];
    for bounding_box in [None, Some(bounds)] {
        let source = Table {
            cells: vec![vec!["North".into(), "10".into()]],
            bounding_box,
            ..Default::default()
        };
        let mut retained = PlainWords::new(plain, "Opening");
        assert!(retained.retains("Opening", PROSE));
        assert!(retained.retains_table(0, &source, &writers));
        assert_eq!(retained.table_substitutions.contains(&0), bounding_box.is_some());
    }
}

#[test]
fn mixed_plain_exact_prose_lookup_does_not_cross_a_long_writer_run() {
    use crate::types::BoundingBox;
    use crate::types::internal::{ElementKind, InternalElement};

    let mut caption = InternalElement::text(ElementKind::Paragraph, "Caption", 0);
    caption.bbox = Some(BoundingBox {
        x0: 0.0,
        y0: 0.0,
        x1: 10.0,
        y1: 10.0,
    });
    let mut plain = "Caption\n".repeat(32);
    let end = plain.len();
    plain.push_str("Target");
    let mut writers: Vec<_> = (0..end)
        .step_by("Caption\n".len())
        .map(|start| PlainWriter {
            start,
            end: start + "Caption\n".len(),
            element: &caption,
        })
        .collect();
    writers.push(PlainWriter {
        start: end,
        end: plain.len(),
        element: &caption,
    });

    let mut retained = PlainWords::new(&plain, "Target");
    assert!(!retained.retains_exact_prose_writer("Target", &writers));
    assert_eq!(retained.words.next(), Some("Caption"));
}

#[test]
fn mixed_plain_preserves_a_distinct_caption_even_when_every_word_is_known() {
    assert_eq!(
        render_mixed_plain_with_prose_and_table(
            "North office\n\nSummary",
            &[
                ("North office", 150.0),
                ("North Summary", 120.0),
                ("Summary", 90.0),
                ("New label", 60.0)
            ]
        ),
        "North office\n\nNorth Summary\n\nSummary\n\nNew label\n\nNorth 10"
    );
}

#[test]
fn mixed_plain_preserves_a_prose_row_and_a_typed_table_without_source_proof() {
    assert_eq!(
        render_mixed_plain_with_table_bounds("Opening", &[("Opening\nNorth 10", 150.0), ("New label", 60.0)], false),
        "Opening\nNorth 10\n\nNew label\n\nNorth 10"
    );
}

#[test]
fn mixed_plain_keeps_real_prose_after_the_exact_writer_limit() {
    assert_mixed_plain_keeps_cross_writer_prose("North office");
}

#[test]
fn mixed_plain_keeps_cross_writer_prose_from_a_markdown_heading() {
    assert_mixed_plain_keeps_cross_writer_prose("# North office");
}

#[test]
fn mixed_plain_keeps_cross_writer_prose_from_a_markdown_list() {
    assert_mixed_plain_keeps_cross_writer_prose("- North office");
}

#[test]
fn mixed_plain_keeps_cross_writer_prose_from_a_markdown_quote() {
    assert_mixed_plain_keeps_cross_writer_prose("> North office");
}

fn assert_mixed_plain_keeps_cross_writer_prose(accepted: &str) {
    let writers = [
        ("North Summary", 160.0),
        ("First caption", 145.0),
        ("Second caption", 130.0),
        ("Third caption", 115.0),
        ("Fourth caption", 100.0),
        ("Fifth caption", 85.0),
        ("Sixth caption", 70.0),
        ("Seventh caption", 55.0),
        ("North office", 40.0),
    ];
    let expected = writers
        .iter()
        .map(|&(text, _)| text)
        .chain(["North 10"])
        .collect::<Vec<_>>()
        .join("\n\n");
    assert_eq!(render_mixed_plain_with_prose_and_table(accepted, &writers), expected);
}

#[test]
fn mixed_plain_keeps_a_typed_list_item_after_a_cross_writer_match() {
    use crate::types::Table;
    use crate::types::internal::{ElementKind, InternalDocument, InternalElement};

    let mut document = InternalDocument::new("pdf");
    document.push_element(InternalElement::text(ElementKind::Paragraph, "North Summary", 0).with_page(1));
    document
        .push_element(InternalElement::text(ElementKind::ListItem { ordered: false }, "North office", 0).with_page(1));
    let cells = vec![vec!["North".into(), "10".into()]];
    let accepted = format!(
        "- North office\n\n{}",
        crate::rendering::common::render_table_markdown(&cells)
    );
    let table_index = document.push_table(Table {
        cells,
        page_number: 1,
        ..Default::default()
    });
    document.push_element(InternalElement::text(ElementKind::Table { table_index }, "", 0).with_page(1));

    assert_eq!(
        reconcile_mixed_plain_page(accepted.trim(), &document, 1).expect("reconcile typed list and table"),
        "North Summary\n\nNorth office\nNorth 10"
    );
}

#[test]
fn mixed_plain_keeps_both_north_occurrences_in_one_writer() {
    assert_eq!(
        render_mixed_plain_with_prose_and_table("North office", &[("North Summary North office", 150.0)]),
        "North Summary North office\n\nNorth 10"
    );
}

#[test]
fn mixed_plain_keeps_a_trailing_repeat_in_the_same_writer() {
    assert_eq!(
        render_mixed_plain_with_prose_and_table("North office", &[("North Summary office North", 150.0)]),
        "North Summary office North\n\nNorth 10"
    );
}

#[test]
fn mixed_plain_keeps_an_ambiguous_writer_and_drops_its_same_source_copy() {
    let prose = "North Summary office North";
    assert_eq!(
        render_mixed_plain_with_prose_and_table("North office", &[(prose, 150.0), (prose, 150.0)]),
        "North Summary office North\n\nNorth 10"
    );
}

#[test]
fn mixed_plain_keeps_a_typed_table_that_only_intersects_the_prose_bounds() {
    use crate::types::internal::{ElementKind, InternalDocument, InternalElement};
    use crate::types::{BoundingBox, Table};

    let mut document = InternalDocument::new("pdf");
    let mut prose = InternalElement::text(ElementKind::Paragraph, "Opening\nNorth 10", 0).with_page(1);
    prose.bbox = Some(BoundingBox {
        x0: 0.0,
        y0: 39.0,
        x1: 500.0,
        y1: 740.0,
    });
    document.push_element(prose);
    document.push_element(InternalElement::text(ElementKind::Paragraph, "New label", 0).with_page(1));
    let cells = vec![vec!["North".into(), "10".into()]];
    let accepted = format!("Opening\n\n{}", crate::rendering::common::render_table_markdown(&cells));
    let index = document.push_table(Table {
        cells,
        page_number: 1,
        bounding_box: Some(BoundingBox {
            x0: 20.0,
            y0: 30.0,
            x1: 120.0,
            y1: 40.0,
        }),
        ..Default::default()
    });
    document.push_element(InternalElement::text(ElementKind::Table { table_index: index }, "", 0).with_page(1));

    assert_eq!(
        reconcile_mixed_plain_page(accepted.trim(), &document, 1).expect("reconcile mixed Plain page"),
        "Opening\nNorth 10\n\nNew label\n\nNorth 10"
    );
}

#[test]
fn declined_mixed_plain_renders_transcript_table_rows_and_keeps_literal_prose() {
    use crate::core::config::OutputFormat;
    use crate::types::internal::{ElementKind, InternalDocument, InternalElement};
    use crate::types::{ExtractionMethod, PageBoundary, Table};

    let cells = vec![
        vec!["Region".into(), "Total".into()],
        vec!["North | West".into(), "10\n11".into()],
    ];
    let table = crate::rendering::common::render_table_markdown(&cells);
    let literal = "Café **literal** prose with a | pipe.";
    let accepted = format!("{literal}\n\n{table}\nA final accepted sentence.");
    let expected = format!("{literal}\n\nRegion Total\nNorth | West 10\n11\n\nA final accepted sentence.");
    let mut document = InternalDocument::new("pdf");
    let table_index = document.push_table(Table {
        cells,
        page_number: 2,
        ..Default::default()
    });
    document.push_element(InternalElement::text(ElementKind::Table { table_index }, "", 0).with_page(2));
    assert_eq!(reconcile_mixed_plain_page(&accepted, &document, 2), Err(PROSE));

    let boundaries = [
        PageBoundary {
            byte_start: 0,
            byte_end: 6,
            page_number: 1,
        },
        PageBoundary {
            byte_start: 8,
            byte_end: 8,
            page_number: 2,
        },
        PageBoundary {
            byte_start: 10,
            byte_end: 14,
            page_number: 3,
        },
    ];
    let documents = ahash::AHashMap::from([(2, document)]);
    for (format, page) in [
        (OutputFormat::Markdown, accepted.as_str()),
        (OutputFormat::Plain, expected.as_str()),
    ] {
        let mut text = format!("Native\n\n{accepted}\n\nTail");
        let mut replacements = ahash::AHashMap::from([(2, accepted.clone())]);
        render_mixed_plain_pages(
            ExtractionMethod::Mixed,
            &mut text,
            &boundaries,
            &mut replacements,
            &documents,
            &format,
        );
        assert_eq!(replacements[&2], page);
        assert_eq!(text, format!("Native\n\n{page}\n\nTail"));
    }
}

#[test]
fn declined_mixed_plain_requires_unambiguous_page_owned_table_occurrences() {
    use crate::types::Table;
    use crate::types::internal::{ElementKind, InternalDocument, InternalElement};

    let mut document = InternalDocument::new("pdf");
    let table_index = document.push_table(Table {
        cells: vec![vec!["Name".into(), "Count".into()], vec!["North".into(), "2".into()]],
        page_number: 2,
        ..Default::default()
    });
    let table_element = InternalElement::text(ElementKind::Table { table_index }, "", 0).with_page(2);
    let table = "|Name |Count|\n| :--- | ---: |\n| North|2 |";
    let plain = "Name Count\nNorth 2";
    let duplicated = format!("{table}\n\nLiteral **middle**.\n\n{table}");
    assert_eq!(render_transcript_tables_as_plain(table, &document, 2), None);

    document.push_element(table_element.clone().with_page(1));
    assert_eq!(render_transcript_tables_as_plain(table, &document, 2), None);
    document.push_element(table_element.clone());
    assert_eq!(render_transcript_tables_as_plain(table, &document, 1), None);
    assert_eq!(
        render_transcript_tables_as_plain(table, &document, 2).as_deref(),
        Some(plain)
    );
    assert_eq!(render_transcript_tables_as_plain(&duplicated, &document, 2), None);

    document.push_element(table_element);
    assert_eq!(render_transcript_tables_as_plain(&duplicated, &document, 2), None);
    let table_index = document.push_table(document.tables[0].clone());
    document.push_element(InternalElement::text(ElementKind::Table { table_index }, "", 0).with_page(2));
    assert_eq!(render_transcript_tables_as_plain(table, &document, 2), None);
    assert_eq!(
        render_transcript_tables_as_plain(&duplicated, &document, 2),
        Some(format!("{plain}\n\nLiteral **middle**.\n\n{plain}"))
    );
}

#[test]
fn declined_mixed_plain_preserves_excess_cells_and_literal_table_examples() {
    use crate::types::Table;
    use crate::types::internal::{ElementKind, InternalDocument, InternalElement};

    let mut document = InternalDocument::new("pdf");
    let table_index = document.push_table(Table {
        cells: vec![vec!["Name".into(), "Count".into()], vec!["North".into(), "2".into()]],
        page_number: 1,
        ..Default::default()
    });
    document.push_element(InternalElement::text(ElementKind::Table { table_index }, "", 0).with_page(1));
    let valid = "| Name | Count |\n| :--- | ---: |\n| North | 2 |";
    let excess = format!("{valid} Extra accepted text |");
    let empty_excess = format!("{valid} |");
    let headerless = "|  |  |\n| :--- | ---: |\n| Name | Count |\n| North | 2 |";
    assert_eq!(
        render_transcript_tables_as_plain(headerless, &document, 1).as_deref(),
        Some("Name Count\nNorth 2")
    );
    assert_eq!(render_transcript_tables_as_plain(&excess, &document, 1), None);
    assert_eq!(render_transcript_tables_as_plain(&empty_excess, &document, 1), None);

    let literal = format!("```markdown\n{valid}\n```");
    let accepted = format!("{excess}\n\n{literal}\n\n{valid}");
    assert_eq!(
        render_transcript_tables_as_plain(&accepted, &document, 1),
        Some(format!("{excess}\n\n{literal}\n\nName Count\nNorth 2"))
    );
}

#[test]
fn declined_mixed_plain_groups_many_distinct_and_repeated_tables() {
    use crate::types::Table;
    use crate::types::internal::{ElementKind, InternalDocument, InternalElement};

    let mut document = InternalDocument::new("pdf");
    let mut source = Vec::new();
    let mut expected = Vec::new();
    for index in 0..2048 {
        let value = index / 2;
        let table_index = document.push_table(Table {
            cells: vec![
                vec!["Region".into(), "Total".into()],
                vec![format!("Region {value}"), value.to_string()],
            ],
            page_number: 1,
            ..Default::default()
        });
        document.push_element(InternalElement::text(ElementKind::Table { table_index }, "", 0).with_page(1));
        source.push(format!("|Region| Total |\n| :--- | ---: |\n| Region {value}|{value}|"));
        expected.push(format!("Region Total\nRegion {value} {value}"));
    }
    let unmatched = source[0].clone();
    source.push(unmatched.clone());
    expected[0] = unmatched.clone();
    expected[1] = unmatched.clone();
    expected.push(unmatched);
    assert_eq!(
        render_transcript_tables_as_plain(&source.join("\n\n"), &document, 1),
        Some(expected.join("\n\n"))
    );
}
