use super::*;
use std::collections::HashSet;

/// (cohort, total cells, required (non-optional) cells). The required count is what the raw
/// artifact / aggregate validators gate publication on; optional competitor cells are allowed
/// but never required for the per-format-family cohorts.
const CONTRACT_CELL_COUNTS: [(Cohort, usize, usize); 8] = [
    (Cohort::Native, 23, 22),
    (Cohort::Ocr, 41, 30),
    (Cohort::Office, 11, 4),
    (Cohort::Markup, 11, 4),
    (Cohort::Ebook, 6, 4),
    (Cohort::Email, 6, 4),
    (Cohort::Data, 11, 4),
    (Cohort::Images, 19, 12),
];

#[test]
fn contract_cell_counts_are_exact_and_uniquely_keyed() {
    for (cohort, total, required) in CONTRACT_CELL_COUNTS {
        let contract = cohort.contract();
        assert_eq!(contract.matrix.len(), total, "{} total cells", cohort.as_str());
        assert_eq!(
            contract.matrix.iter().filter(|entry| !entry.optional).count(),
            required,
            "{} required cells",
            cohort.as_str()
        );
        assert_eq!(
            contract
                .matrix
                .iter()
                .map(|entry| &entry.artifact)
                .collect::<HashSet<_>>()
                .len(),
            contract.matrix.len(),
            "{} artifact names must be unique",
            cohort.as_str()
        );
        assert_eq!(
            contract
                .matrix
                .iter()
                .map(MatrixEntry::aggregate_key)
                .collect::<HashSet<_>>()
                .len(),
            contract.matrix.len(),
            "{} aggregate keys must be unique",
            cohort.as_str()
        );
    }
}

#[test]
fn native_contract_includes_every_pdfium_workflow_cell() {
    let contract = Cohort::Native.contract();
    let pdfium_entries: Vec<_> = contract
        .matrix
        .iter()
        .filter(|entry| entry.artifact.starts_with("benchmarks-rust-pdfium-"))
        .collect();

    assert_eq!(pdfium_entries.len(), 4);
    assert!(pdfium_entries.iter().all(|entry| !entry.optional));
    assert_eq!(
        pdfium_entries
            .iter()
            .map(|entry| entry.artifact.as_str())
            .collect::<HashSet<_>>(),
        HashSet::from([
            "benchmarks-rust-pdfium-baseline-markdown-single-file-native-pdf-fast-b8",
            "benchmarks-rust-pdfium-baseline-markdown-batch-native-pdf-fast-b8",
            "benchmarks-rust-pdfium-baseline-plaintext-single-file-native-pdf-fast-b8",
            "benchmarks-rust-pdfium-baseline-plaintext-batch-native-pdf-fast-b8",
        ])
    );
}

#[test]
fn only_rendered_page_cohorts_run_the_layout_pipeline() {
    for cohort in Cohort::ALL {
        let contract = cohort.contract();
        let has_layout = contract
            .matrix
            .iter()
            .any(|entry| entry.framework.starts_with("xberg-") && entry.framework.ends_with("-layout"));
        assert_eq!(
            has_layout,
            cohort.includes_layout(),
            "{}: layout-pipeline presence must match includes_layout()",
            cohort.as_str()
        );
        let has_paddle = contract
            .matrix
            .iter()
            .any(|entry| entry.framework.starts_with("xberg-") && entry.framework.ends_with("-paddle"));
        assert_eq!(
            has_paddle,
            cohort.includes_paddle_ocr(),
            "{}: paddle-pipeline presence must match includes_paddle_ocr()",
            cohort.as_str()
        );
        let has_sceptre = contract
            .matrix
            .iter()
            .any(|entry| entry.framework.contains("-sceptre-ort"));
        assert_eq!(
            has_sceptre,
            cohort.includes_sceptre_ort(),
            "{}: Sceptre ORT presence must match includes_sceptre_ort()",
            cohort.as_str()
        );
        // The pdfium backend is a separate dimension from the OCR/layout pipeline grid this
        // formula models: native adds 4 `xberg-*-baseline-pdfium` cells via
        // `xberg_pdfium_entries`, which `native_contract_includes_every_pdfium_workflow_cell`
        // covers exactly. Counting them here made native read 12 against a predicted 8. ~keep
        let xberg_required = contract
            .matrix
            .iter()
            .filter(|entry| {
                entry.framework.starts_with("xberg-")
                    && !entry.framework.ends_with("-pdfium")
                    && !entry.framework.contains("-auto-ocr")
            })
            .count();
        // 4 cells (md/plain x single/batch) per enabled pipeline, 2 (markdown only) for a
        // layout pipeline: baseline, +layout, +baseline-paddle, +layout-paddle (the last only
        // when layout is also present), +3 Sceptre ORT variants (one of them layout).
        let mut full_pipelines: usize = 1;
        let mut layout_pipelines: usize = 0;
        if cohort.includes_layout() {
            layout_pipelines += 1;
        }
        if cohort.includes_paddle_ocr() {
            full_pipelines += 1;
            if cohort.includes_layout() {
                layout_pipelines += 1;
            }
        }
        if cohort.includes_sceptre_ort() {
            full_pipelines += 2;
            layout_pipelines += 1;
        }
        assert_eq!(
            xberg_required,
            full_pipelines * 4 + layout_pipelines * 2,
            "{}: xberg cell count",
            cohort.as_str()
        );
    }
}

#[test]
fn no_cohort_contract_contains_a_plaintext_layout_cell() {
    for cohort in Cohort::ALL {
        let offenders: Vec<_> = cohort
            .contract()
            .matrix
            .iter()
            .filter(|entry| entry.framework.starts_with("xberg-plaintext-") && entry.framework.contains("layout"))
            .map(|entry| entry.framework.clone())
            .collect();
        assert!(offenders.is_empty(), "{}: {offenders:?}", cohort.as_str());
    }
}

#[test]
fn auto_ocr_cells_are_optional_and_only_in_the_ocr_pdf_cohort() {
    for cohort in Cohort::ALL {
        let cells: Vec<_> = cohort
            .contract()
            .matrix
            .into_iter()
            .filter(|entry| entry.framework.starts_with("xberg-") && entry.framework.contains("-auto-ocr"))
            .collect();
        if cohort == Cohort::Ocr {
            assert_eq!(
                cells.len(),
                8,
                "{}: 2 variants x md/plain x single/batch",
                cohort.as_str()
            );
            assert!(cells.iter().all(|entry| entry.optional), "{}", cohort.as_str());
            let frameworks: HashSet<&str> = cells.iter().map(|entry| entry.framework.as_str()).collect();
            for name in [
                "xberg-markdown-auto-ocr",
                "xberg-plaintext-auto-ocr",
                "xberg-markdown-auto-ocr-paddle",
                "xberg-plaintext-auto-ocr-paddle",
            ] {
                assert!(frameworks.contains(name), "{}: {name}", cohort.as_str());
            }
        } else {
            assert!(cells.is_empty(), "{}", cohort.as_str());
        }
    }
}

#[test]
fn ocr_pdf_release_contract_includes_ort_variants_but_not_tract() {
    let contract = Cohort::Ocr.contract();
    let frameworks: HashSet<&str> = contract.matrix.iter().map(|entry| entry.framework.as_str()).collect();
    for variant in ["sceptre-ort", "sceptre-ort-layout", "sceptre-ort-autorotate"] {
        assert!(frameworks.iter().any(|framework| framework.ends_with(variant)));
    }
    assert!(!frameworks.iter().any(|framework| framework.contains("sceptre-tract")));
}

#[test]
fn image_release_contract_excludes_sceptre_diagnostic_variants() {
    let contract = Cohort::Images.contract();
    assert!(contract.matrix.iter().all(|entry| !entry.framework.contains("sceptre")));
}

#[test]
fn sceptre_ort_aggregate_keys_preserve_engine_and_variant_identity() {
    let contract = Cohort::Ocr.contract();
    let keys: HashSet<String> = contract.matrix.iter().map(MatrixEntry::aggregate_key).collect();
    for variant in ["sceptre-ort", "sceptre-ort-layout", "sceptre-ort-autorotate"] {
        assert!(keys.contains(&format!("xberg-markdown-{variant}:single")));
        assert_eq!(
            keys.contains(&format!("xberg-plaintext-{variant}:batch")),
            !variant.contains("layout"),
            "{variant}: plaintext layout cells are not part of the contract"
        );
    }
    assert!(keys.iter().all(|key| !key.contains("sceptre-tract")));
}

#[test]
fn mineru_is_single_file_only_wherever_present() {
    for cohort in Cohort::ALL {
        let contract = cohort.contract();
        let mineru_modes: Vec<ExecutionMode> = contract
            .matrix
            .iter()
            .filter(|entry| entry.framework == "mineru")
            .map(|entry| entry.mode)
            .collect();
        assert!(
            mineru_modes.iter().all(|mode| *mode == ExecutionMode::SingleFile),
            "{}: mineru must be single-file only",
            cohort.as_str()
        );
    }
}

#[test]
fn cohort_parse_round_trips_as_str() {
    for cohort in Cohort::ALL {
        assert_eq!(Cohort::parse(cohort.as_str()).unwrap(), cohort);
    }
    assert!(Cohort::parse("bogus").is_err());
}

#[test]
fn xberg_aggregate_keys_omit_output_format() {
    let contract = Cohort::Native.contract();
    let entry = contract
        .matrix
        .iter()
        .find(|entry| entry.framework == "xberg-markdown-baseline")
        .expect("xberg-markdown-baseline cell present");
    assert_eq!(entry.aggregate_key(), "xberg-markdown-baseline:single");
}
