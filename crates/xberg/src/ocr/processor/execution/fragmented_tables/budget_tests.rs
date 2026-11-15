use super::super::super::{ImageOcrPolicy, publish_hocr_table_rows};
use super::*;

fn retained_leader_comparison_budget(hocr: &str, tsv: &str, table: &OcrTable) -> (usize, usize, usize) {
    let config = TesseractConfig::default();
    let limits = SecurityLimits {
        max_iterations: usize::MAX,
        ..Default::default()
    };
    let mut budget = FragmentWork(fragmented_hocr_budget(hocr, tsv, &limits));
    let boxes = fragmented_table_boxes(std::slice::from_ref(table), config.page_number, &limits, &mut budget).unwrap();
    let mut words = fragmented_tsv_source(tsv).unwrap();
    fragmented_hocr_source_matches(hocr, &words, &mut budget).unwrap();
    let legacy_copy_work = words.iter().map(|word| word.text.len() + 1).sum::<usize>();
    let (_, comparison) = prepare_fragmented_table_rows(
        &mut words,
        boxes[0],
        &table.cells,
        config.table_row_threshold_ratio,
        &mut budget,
    )
    .unwrap();
    (usize::MAX - budget.0.unwrap(), comparison, legacy_copy_work)
}

#[test]
fn psm3_publication_checks_ambiguity_with_budget_for_only_one_comparison() {
    let (hocr, tsv, mut document, table) = repeated_leader_publication_case(1, 65, [true, false, true]);
    let (first_pass, comparison, legacy_copy_work) = retained_leader_comparison_budget(&hocr, &tsv, &table);
    let mut tables = [table];
    let config = TesseractConfig::default();
    assert!(legacy_copy_work < comparison);
    let limits = SecurityLimits {
        max_iterations: first_pass + legacy_copy_work,
        ..Default::default()
    };
    assert!(matches!(
        reconcile_retained_table_leaders(&hocr, &document, &mut tables, &tsv, &config, &limits),
        Some(RetainedTableLeaders::Ambiguous)
    ));

    let source = document.elements.clone();
    publish_hocr_table_rows(
        &mut document,
        &mut tables,
        Some(&hocr),
        Some(&tsv),
        &config,
        ImageOcrPolicy::StandaloneAutoTableRows,
        &limits,
    );
    assert_eq!(document.elements.len(), 14, "one comparison can establish ambiguity");
    assert_eq!(document.elements, source);
    assert!(tables[0].markdown.is_empty());
}

#[test]
fn retained_leader_work_exhaustion_cannot_publish_a_partial_table() {
    let (hocr, tsv, document, table) = repeated_leader_publication_case(1, 40, [true; 3]);
    let (first_pass, comparison, _) = retained_leader_comparison_budget(&hocr, &tsv, &table);
    let config = TesseractConfig::default();
    let mut tables = [table];
    for max_iterations in [0, first_pass - 1, first_pass, first_pass + comparison - 1] {
        let limits = SecurityLimits {
            max_iterations,
            ..Default::default()
        };
        assert!(
            matches!(
                reconcile_retained_table_leaders(&hocr, &document, &mut tables, &tsv, &config, &limits),
                Some(RetainedTableLeaders::BudgetExhausted)
            ),
            "iteration limit {max_iterations}"
        );
        assert_eq!(tables[0].cells[0][0], "Signal timing");
        assert!(tables[0].markdown.is_empty());
    }
    let limits = SecurityLimits {
        max_iterations: first_pass + comparison,
        ..Default::default()
    };
    assert!(matches!(
        reconcile_retained_table_leaders(&hocr, &document, &mut tables, &tsv, &config, &limits),
        Some(RetainedTableLeaders::Reconciled(_))
    ));
    assert_eq!(tables[0].cells[0][0], "Signal timing ........");
}

fn leader_alignment_cap_case(rows: usize, grouped: bool) -> (String, String, InternalDocument, OcrTable) {
    let (_, tsv, _, mut table) = repeated_leader_publication_case(1, 40, [true; 3]);
    let mut tsv = format!("{}\n", tsv.lines().next().unwrap());
    let mut hocr = String::from(r#"<div class="ocr_page" title="ppageno 0">"#);
    if grouped {
        hocr.push_str(r#"<p class="ocr_par">"#);
    }
    for index in 0..rows {
        let (raw, row_tsv) = super::super::super::tests::word_table_markup(
            positioned_dot_leader_row()
                .into_iter()
                .filter_map(|word| (index != 1 || word.text != "........").then_some(word))
                .collect::<Vec<_>>()
                .iter()
                .map(|word| {
                    (
                        word.text.as_str(),
                        word.left,
                        word.top + index as u32 * 25,
                        word.width,
                        word.height,
                        index + 1,
                    )
                }),
        );
        if !grouped {
            hocr.push_str(r#"<p class="ocr_par">"#);
        }
        hocr.push_str(r#"<span class="ocr_line">"#);
        for (start, _) in raw.match_indices(r#"<span class="ocrx_word""#) {
            let end = start + raw[start..].find("</span>").unwrap() + "</span>".len();
            hocr.push_str(&raw[start..end]);
        }
        hocr.push_str("</span>");
        if !grouped {
            hocr.push_str("</p>");
        }
        tsv.extend(row_tsv.lines().skip(1).flat_map(|line| line.chars().chain(['\n'])));
    }
    if grouped {
        hocr.push_str("</p>");
    }
    hocr.push_str("</div>");
    table.bounding_box.as_mut().unwrap().bottom = 40 + (rows as u32 - 1) * 25;
    let document = crate::ocr::hocr_parser::parse_hocr_to_internal_document(&hocr);
    (hocr, tsv, document, table)
}

#[test]
fn psm3_publication_preserves_dotted_and_undotted_rows_at_the_alignment_cap() {
    for windows in [MAX_FRAGMENTED_HOCR_ALIGNMENTS, MAX_FRAGMENTED_HOCR_ALIGNMENTS + 1] {
        for grouped in [true, false] {
            let (hocr, tsv, mut document, table) = leader_alignment_cap_case(windows, grouped);
            let source = document.elements.clone();
            let source_lines = source.iter().map(|element| element.text.lines().count()).sum::<usize>();
            assert_eq!(source_lines, windows);
            assert_eq!(
                source
                    .iter()
                    .flat_map(|element| element.text.lines())
                    .filter(|line| *line == "Signal timing -64 143.92")
                    .count(),
                1
            );
            let mut tables = [table];
            let verdict = reconcile_retained_table_leaders(
                &hocr,
                &document,
                &mut tables,
                &tsv,
                &TesseractConfig::default(),
                &SecurityLimits::default(),
            );
            if windows == MAX_FRAGMENTED_HOCR_ALIGNMENTS {
                assert!(matches!(verdict, Some(RetainedTableLeaders::Ambiguous)));
            } else {
                assert!(matches!(verdict, Some(RetainedTableLeaders::BudgetExhausted)));
            }
            publish_hocr_table_rows(
                &mut document,
                &mut tables,
                Some(&hocr),
                Some(&tsv),
                &TesseractConfig::default(),
                ImageOcrPolicy::StandaloneAutoTableRows,
                &SecurityLimits::default(),
            );
            let remaining = document
                .elements
                .iter()
                .map(|element| element.text.lines().count())
                .sum::<usize>();
            assert_eq!(remaining, windows, "grouped={grouped}, candidate windows={windows}");
            assert_eq!(document.elements, source);
            assert_eq!(tables[0].cells[0][0], "Signal timing");
            assert!(tables[0].markdown.is_empty());
        }
    }
}
