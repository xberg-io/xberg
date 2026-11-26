use super::*;
use std::collections::HashSet;

/// (cohort, total cells, required (non-optional) cells). The required count is what the raw
/// artifact / aggregate validators gate publication on; optional competitor cells are allowed
/// but never required for the per-format-family cohorts.
const CONTRACT_CELL_COUNTS: [(Cohort, usize, usize); 11] = [
    (Cohort::Native, 23, 22),
    (Cohort::Ocr, 41, 30),
    (Cohort::Office, 15, 4),
    (Cohort::Markup, 11, 4),
    (Cohort::Ebook, 6, 4),
    (Cohort::Email, 6, 4),
    (Cohort::Data, 15, 4),
    (Cohort::Images, 23, 12),
    (Cohort::NativeStructured, 23, 22),
    (Cohort::OcrStructured, 41, 30),
    (Cohort::ImagesStructured, 23, 12),
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
        if matches!(cohort, Cohort::Ocr | Cohort::OcrStructured) {
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

#[test]
fn every_cohort_manifest_resolves_to_its_pinned_contract() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    for cohort in Cohort::ALL {
        let contract = cohort.contract();
        let path = crate_root.join(format!("cohorts/{}.json", cohort.manifest_file_stem()));
        let manifest = crate::cohort::CohortManifest::from_file(&path)
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        let name = cohort.as_str();

        assert_eq!(manifest.name, contract.manifest_name, "{name}: manifest name");
        assert_eq!(manifest.batch_size, contract.batch_size, "{name}: batch size");
        let fixtures: Vec<&str> = manifest.fixtures.iter().map(|f| f.to_str().unwrap()).collect();
        assert_eq!(fixtures, contract.fixtures, "{name}: fixtures");
        assert_eq!(contract.document_stems.len(), contract.fixtures.len(), "{name}: stems");
        assert_eq!(
            contract.document_extensions.len(),
            contract.fixtures.len(),
            "{name}: exts"
        );
        let digest = blake3::hash(&std::fs::read(&path).unwrap()).to_hex().to_string();
        assert_eq!(digest, contract.manifest_blake3, "{name}: manifest BLAKE3");
        assert_eq!(
            manifest.ocr_enabled,
            cohort.expects_ocr(),
            "{name}: manifest ocr_enabled vs expects_ocr"
        );

        let aggregate_cohort = Cohort::ALL
            .into_iter()
            .find(|candidate| candidate.contract().manifest_name == manifest.name);
        assert_eq!(aggregate_cohort, Some(cohort), "{name}: lookup by manifest name");
    }
}

#[test]
fn cohort_contracts_derive_artifact_names_from_their_own_manifest() {
    for cohort in Cohort::ALL {
        let stem = cohort.manifest_file_stem();
        assert!(
            cohort
                .contract()
                .matrix
                .iter()
                .all(|entry| entry.artifact.ends_with(&format!("-{stem}"))),
            "{}: artifact names must end with the manifest stem {stem}",
            cohort.as_str()
        );
    }
}

#[test]
fn workflow_cohort_manifests_exist_and_pinned_ones_resolve() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let workflow = crate_root.join("../../.github/workflows/benchmarks.yaml");
    let text = std::fs::read_to_string(&workflow).unwrap_or_else(|error| panic!("{}: {error}", workflow.display()));
    let mut stems = HashSet::new();
    for line in text.lines() {
        let Some((_, rest)) = line.split_once("cohort: ") else {
            continue;
        };
        let names: Vec<&str> = if let Some(list) = rest.trim_start().strip_prefix('[') {
            list.split(']').next().unwrap_or("").split(',').collect()
        } else if line.contains("{ pipeline:") {
            vec![rest.split(['}', ',']).next().unwrap_or("")]
        } else {
            continue;
        };
        stems.extend(
            names
                .into_iter()
                .map(|name| name.trim().to_string())
                .filter(|name| !name.is_empty()),
        );
    }
    assert!(stems.len() >= 8, "workflow cohort scan found only {stems:?}");
    for stem in &stems {
        let path = crate_root.join(format!("cohorts/{stem}.json"));
        let manifest = crate::cohort::CohortManifest::from_file(&path)
            .unwrap_or_else(|error| panic!("workflow cohort {stem}: {error}"));
        if let Some(cohort) = Cohort::ALL.into_iter().find(|c| c.manifest_file_stem() == stem) {
            assert_eq!(manifest.name, cohort.contract().manifest_name, "{stem}");
        }
    }
}
