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
