use std::collections::BTreeSet;

use serde_json::json;
use sha2::{Digest, Sha256};

use super::*;

fn pinned_file(bytes: &[u8]) -> CandidateFile {
    CandidateFile {
        path: "probe/file.bin".to_owned(),
        sha256: hex::encode(Sha256::digest(bytes)),
        size_bytes: bytes.len() as u64,
        repository: "xberg-io/probe".to_owned(),
        revision: "a".repeat(40),
        license: "MIT".to_owned(),
        source: "spdx:MIT".to_owned(),
    }
}

fn inventory_document() -> serde_json::Value {
    json!({
        "schema_version": INVENTORY_SCHEMA_VERSION,
        "models": [{
            "id": "probe",
            "task": "image_captioning",
            "status": "experimental_not_qualified",
            "repository": "xberg-io/probe",
            "revision": "a".repeat(40),
            "upstream_repository": "upstream/probe",
            "upstream_revision": "b".repeat(40),
            "license": "MIT",
            "provenance": "test export",
            "files": [{
                "path": "probe/file.bin",
                "sha256": "0".repeat(64),
                "size_bytes": 1,
                "source": "spdx:MIT"
            }]
        }]
    })
}

fn parse_document(document: &serde_json::Value) -> Result<(), String> {
    inventory_from(&document.to_string()).map(|_| ())
}

#[test]
fn inventory_has_the_expected_models_and_file_counts() {
    let counts: Vec<(&str, usize)> = CANDIDATE_MODELS
        .iter()
        .map(|model| (model.id, model.files().unwrap().len()))
        .collect();
    assert_eq!(
        counts,
        [
            ("nli-minilm2-l6-h768", 11),
            ("smolvlm-256m-instruct", 19),
            ("smolvlm-500m-instruct", 19),
            ("florence-2-base", 20),
        ]
    );
}

#[test]
fn every_candidate_is_experimental_permissive_and_owned_by_xberg() {
    let mut ids = BTreeSet::new();
    for model in &CANDIDATE_MODELS {
        assert!(ids.insert(model.id), "duplicate id {}", model.id);
        assert_eq!(model.status, CandidateStatus::ExperimentalNotQualified, "{}", model.id);
        assert!(model.repository.starts_with("xberg-io/"), "{}", model.id);
        assert!(matches!(model.license, "Apache-2.0" | "MIT"), "{}", model.id);
        assert!(!model.upstream_repository.starts_with("xberg-io/"), "{}", model.id);
        assert_eq!(CandidateModel::find(model.id), Some(model));
    }
    assert_eq!(CandidateModel::find("not-a-candidate"), None);
    assert_eq!(
        CANDIDATE_MODELS[0].license, "Apache-2.0",
        "the license field must stay SPDX"
    );
    assert!(
        CANDIDATE_MODELS[0]
            .license_caveat
            .unwrap()
            .contains("nreimers/MiniLMv2")
    );
    for model in &CANDIDATE_MODELS[1..] {
        assert_eq!(model.license_caveat, None, "{}", model.id);
    }
}

#[test]
fn revisions_are_immutable_commit_hashes_and_floating_refs_are_rejected() {
    for model in &CANDIDATE_MODELS {
        model.validate_revisions().unwrap();
    }
    let upper: &'static str = "A".repeat(40).leak();
    let short = &upper[..39];
    let non_hex = "g".repeat(40).leak();
    for floating in ["main", "refs/heads/main", "v1", "", upper, short, non_hex] {
        let mut model = CANDIDATE_MODELS[0];
        model.revision = floating;
        let error = model.validate_revisions().unwrap_err();
        assert!(
            error.contains("owned revision") && error.contains("not an immutable commit hash"),
            "{error}"
        );
        let mut model = CANDIDATE_MODELS[0];
        model.upstream_revision = floating;
        let error = model.validate_revisions().unwrap_err();
        assert!(
            error.contains("upstream revision") && error.contains("not an immutable commit hash"),
            "{error}"
        );
    }
    assert_ne!(CLASSIFICATION_MODELS_REVISION, CAPTIONING_MODELS_REVISION);
}

#[test]
fn the_inventory_pins_sizes_licenses_sources_and_owned_revisions_per_file() {
    for model in &CANDIDATE_MODELS {
        for file in model.files().unwrap() {
            assert_eq!(file.repository, model.repository, "{}", file.path);
            assert_eq!(file.revision, model.revision, "{}", file.path);
            assert_eq!(file.license, model.license, "{}", file.path);
            assert_eq!(file.sha256.len(), 64, "{}", file.path);
            assert!(file.size_bytes > 0, "{} has no size", file.path);
            assert!(is_source(&file.source), "{} has a malformed source", file.path);
            assert_eq!(
                file.label(),
                format!("{}@{}:{}", model.repository, model.revision, file.path)
            );
        }
    }
    let files = CANDIDATE_MODELS[0].files().unwrap();
    let tokenizer = files
        .iter()
        .find(|file| file.path.ends_with("/tokenizer.json"))
        .unwrap();
    assert_eq!(tokenizer.size_bytes, 1_356_048);
    let graph = files
        .iter()
        .find(|file| file.path.ends_with("/model_qint8_arm64.onnx"))
        .unwrap();
    assert_eq!(
        graph.source,
        "cross-encoder/nli-MiniLM2-L6-H768@b95119ce93d3e065de6214e38cd4a97b0f2f2c6d:onnx/model_qint8_arm64.onnx"
    );
    let license = files.iter().find(|file| file.path.ends_with("/LICENSE")).unwrap();
    assert_eq!(license.source, "spdx:Apache-2.0");
    let card = files
        .iter()
        .find(|file| file.path.ends_with("/UPSTREAM_README.md"))
        .unwrap();
    assert!(card.source.ends_with(":README.md"), "{}", card.source);
    let florence = CANDIDATE_MODELS[3].files().unwrap();
    assert_eq!(
        florence
            .iter()
            .find(|file| file.path.ends_with("/LICENSE"))
            .unwrap()
            .source,
        "microsoft/Florence-2-base@5ca5edf5bd017b9919c05d08aebef5e4c7ac3bac:LICENSE"
    );
    for model in &CANDIDATE_MODELS {
        assert!(!model.provenance().unwrap().is_empty(), "{}", model.id);
    }
}

#[test]
fn inventory_paths_and_sources_are_unique_and_not_floating() {
    let document = inventory().unwrap();
    assert!(!document.models.is_empty());
    let mut paths = BTreeSet::new();
    for model in &document.models {
        assert!(is_commit_hash(&model.revision));
        assert!(is_commit_hash(&model.upstream_revision));
        for file in &model.files {
            assert!(paths.insert(file.path.as_str()), "duplicate {}", file.path);
            assert!(is_source(&file.source), "{}", file.path);
        }
    }
    assert!(
        !INVENTORY_JSON.contains("\"main\""),
        "no floating revision may be pinned"
    );
}

#[test]
fn a_task_selection_returns_only_its_own_evaluation_models() {
    let zero_shot = CandidateTask::ZeroShotClassification.models();
    let captioning = CandidateTask::ImageCaptioning.models();
    assert_eq!(
        zero_shot.iter().map(|model| model.id).collect::<Vec<_>>(),
        ["nli-minilm2-l6-h768"]
    );
    assert_eq!(
        captioning.iter().map(|model| model.id).collect::<Vec<_>>(),
        ["smolvlm-256m-instruct", "smolvlm-500m-instruct", "florence-2-base"]
    );
    let union: BTreeSet<&str> = zero_shot.iter().chain(&captioning).map(|model| model.id).collect();
    assert_eq!(union.len(), CANDIDATE_MODELS.len());
}

#[test]
fn file_selection_is_explicit_and_precision_scoped() {
    let nli = &CANDIDATE_MODELS[0];
    let error = nli.files_for(CandidatePrecision::Fp32).unwrap_err();
    assert!(error.contains("exceeds the adapter's size cap"), "{error}");
    let quantized: BTreeSet<String> = nli
        .files_for(CandidatePrecision::Quantized)
        .unwrap()
        .into_iter()
        .map(|file| file.path)
        .collect();
    let host_graph = nli_minilm2_l6_h768_graph().unwrap();
    assert!(quantized.contains(&format!("{}/{host_graph}", nli.id)));
    assert!(!quantized.contains(&format!("{}/model.onnx", nli.id)));
    assert!(quantized.iter().any(|path| path.ends_with("/LICENSE")));
    assert!(quantized.iter().any(|path| path.ends_with("/UPSTREAM_README.md")));

    let captioning = &CANDIDATE_MODELS[1];
    let fp32: BTreeSet<String> = captioning
        .files_for(CandidatePrecision::Fp32)
        .unwrap()
        .into_iter()
        .map(|file| file.path)
        .collect();
    let int8: BTreeSet<String> = captioning
        .files_for(CandidatePrecision::Quantized)
        .unwrap()
        .into_iter()
        .map(|file| file.path)
        .collect();
    assert!(fp32.contains(&format!("{}/onnx/vision_encoder.onnx", captioning.id)));
    assert!(
        !fp32
            .iter()
            .any(|path| path.contains("/onnx/") && path.ends_with("_int8.onnx"))
    );
    assert!(int8.contains(&format!("{}/onnx/vision_encoder_int8.onnx", captioning.id)));
    assert!(!int8.contains(&format!("{}/onnx/vision_encoder.onnx", captioning.id)));
    let fp32_graphs: BTreeSet<_> = fp32.iter().filter(|path| path.contains("/onnx/")).collect();
    let int8_graphs: BTreeSet<_> = int8.iter().filter(|path| path.contains("/onnx/")).collect();
    assert!(fp32_graphs.is_disjoint(&int8_graphs));
    assert_eq!(fp32_graphs.len(), 3);
    assert_eq!(int8_graphs.len(), 3);
}

#[test]
fn optional_files_are_the_unselected_quantizations() {
    for model in &[&CANDIDATE_MODELS[0], &CANDIDATE_MODELS[1]] {
        for precision in [CandidatePrecision::Fp32, CandidatePrecision::Quantized] {
            let selected: BTreeSet<String> = match model.files_for(precision) {
                Ok(files) => files.into_iter().map(|file| file.path).collect(),
                Err(_) => continue,
            };
            let optional: BTreeSet<String> = model
                .optional_files(precision)
                .unwrap()
                .into_iter()
                .map(|file| file.path)
                .collect();
            assert!(selected.is_disjoint(&optional), "{}", model.id);
            assert_eq!(selected.len() + optional.len(), model.files().unwrap().len());
        }
    }
}

#[test]
fn every_required_file_and_the_license_are_pinned_with_a_sha256() {
    for model in &CANDIDATE_MODELS {
        let files = model.files().unwrap();
        let pinned: BTreeSet<&str> = files.iter().map(|f| f.path.as_str()).collect();
        for required in model.required_files.iter().chain(&["LICENSE", "UPSTREAM_README.md"]) {
            let path = format!("{}/{required}", model.id);
            assert!(pinned.contains(path.as_str()), "{path} is not pinned");
        }
        for file in &files {
            assert_eq!(file.sha256.len(), 64, "{}", file.path);
        }
    }
}

#[test]
fn a_malformed_inventory_is_rejected_by_the_real_parser() {
    assert!(
        inventory_from("not json")
            .unwrap_err()
            .contains("candidate inventory is malformed")
    );

    let mut schema = inventory_document();
    schema["schema_version"] = json!(3);
    assert!(parse_document(&schema).unwrap_err().contains("schema 3 is unsupported"));

    let mut empty = inventory_document();
    empty["models"] = json!([]);
    assert!(parse_document(&empty).unwrap_err().contains("declares no models"));

    let mut bad_sha = inventory_document();
    bad_sha["models"][0]["files"][0]["sha256"] = json!("xyz");
    assert!(parse_document(&bad_sha).unwrap_err().contains("no lowercase SHA-256"));

    let mut zero_size = inventory_document();
    zero_size["models"][0]["files"][0]["size_bytes"] = json!(0);
    assert!(parse_document(&zero_size).unwrap_err().contains("has no size"));

    let mut traversal = inventory_document();
    traversal["models"][0]["files"][0]["path"] = json!("probe/../escape.bin");
    assert!(parse_document(&traversal).unwrap_err().contains("does not belong"));

    let mut directory = inventory_document();
    directory["models"][0]["files"][0]["path"] = json!("probe/");
    assert!(parse_document(&directory).unwrap_err().contains("does not belong"));

    let mut duplicate = inventory_document();
    let file = duplicate["models"][0]["files"][0].clone();
    duplicate["models"][0]["files"] = json!([file.clone(), file]);
    assert!(parse_document(&duplicate).unwrap_err().contains("duplicate file path"));

    let mut empty_license = inventory_document();
    empty_license["models"][0]["license"] = json!("");
    assert!(parse_document(&empty_license).unwrap_err().contains("empty license"));

    let mut floating = inventory_document();
    floating["models"][0]["revision"] = json!("main");
    assert!(
        parse_document(&floating)
            .unwrap_err()
            .contains("not an immutable commit hash")
    );

    let mut bad_source = inventory_document();
    bad_source["models"][0]["files"][0]["source"] = json!("main:file.bin");
    assert!(parse_document(&bad_source).unwrap_err().contains("malformed source"));
}

#[test]
fn verify_accepts_exact_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("file.bin");
    std::fs::write(&file, b"pinned bytes").unwrap();
    pinned_file(b"pinned bytes").verify(&file).unwrap();
}

#[test]
fn verify_rejects_same_size_corruption() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("file.bin");
    std::fs::write(&file, b"pinned bytez").unwrap();
    let error = pinned_file(b"pinned bytes").verify(&file).unwrap_err();
    assert!(
        error.contains("Checksum mismatch for xberg-io/probe@") && error.contains(":probe/file.bin"),
        "{error}"
    );
}

#[test]
fn verify_rejects_truncation_and_growth_by_size() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("file.bin");
    let pin = pinned_file(b"pinned bytes");
    for (content, expected) in [(&b"pinned byte"[..], 11_u64), (b"pinned bytes!", 13_u64)] {
        std::fs::write(&file, content).unwrap();
        let error = pin.verify(&file).unwrap_err();
        assert!(
            error.contains("Size mismatch") && error.contains(&format!("expected 12 bytes, got {expected}")),
            "{error}"
        );
    }
}

#[test]
fn verify_rejects_missing_and_non_files() {
    let dir = tempfile::tempdir().unwrap();
    let pin = pinned_file(b"pinned bytes");
    let missing = dir.path().join("absent.bin");
    assert!(
        pin.verify(&missing).unwrap_err().contains("Missing pinned artifact"),
        "a missing file must be named as missing"
    );
    std::fs::create_dir(&missing).unwrap();
    assert!(
        pin.verify(&missing).unwrap_err().contains("is not a regular file"),
        "a directory must not verify as an artifact"
    );
}

#[test]
fn verify_file_checks_the_real_pins_of_the_inventory() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("tokenizer_config.json");
    std::fs::write(&file, b"{}").unwrap();
    let error = CANDIDATE_MODELS[0]
        .verify_file("tokenizer_config.json", &file)
        .unwrap_err();
    assert!(
        error.contains("Size mismatch for xberg-io/classification-models@a29f2096c723fd39c45b7c21690ea0318866bd0b:")
            && error.contains("nli-minilm2-l6-h768/tokenizer_config.json"),
        "{error}"
    );
    std::fs::write(&file, vec![0_u8; 330]).unwrap();
    let error = CANDIDATE_MODELS[0]
        .verify_file("tokenizer_config.json", &file)
        .unwrap_err();
    assert!(
        error.contains("Checksum mismatch for xberg-io/classification-models@"),
        "{error}"
    );
    assert!(
        CANDIDATE_MODELS[0]
            .verify_file("other.bin", &file)
            .unwrap_err()
            .contains("is not pinned for")
    );
}

#[test]
fn a_wrong_revision_or_repository_fails_before_the_bytes_are_checked() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("tokenizer_config.json");
    std::fs::write(&file, vec![0_u8; 330]).unwrap();
    let model = &CANDIDATE_MODELS[0];
    let other_revision = "b".repeat(40);
    for revision in ["main", "refs/heads/main", other_revision.as_str()] {
        let error = model
            .verify_downloaded_at(model.repository, revision, "tokenizer_config.json", &file)
            .unwrap_err();
        assert!(
            error.contains("is not the pinned immutable revision") && error.contains(CLASSIFICATION_MODELS_REVISION),
            "{error}"
        );
        assert!(!error.contains("Checksum"), "the revision must fail first: {error}");
    }
    let error = model
        .verify_downloaded_at("xberg-io/other-models", model.revision, "tokenizer_config.json", &file)
        .unwrap_err();
    assert!(
        error.contains("was fetched from xberg-io/other-models, but the inventory pins xberg-io/classification-models"),
        "{error}"
    );
    let error = model
        .verify_downloaded_at(model.repository, model.revision, "tokenizer_config.json", &file)
        .unwrap_err();
    assert!(
        error.contains("Checksum mismatch"),
        "the pinned revision still checks bytes: {error}"
    );
}

#[test]
fn a_downloaded_set_names_the_first_missing_artifact() {
    let dir = tempfile::tempdir().unwrap();
    let error = CANDIDATE_MODELS[1]
        .verify_downloaded_set(dir.path(), CandidatePrecision::Quantized)
        .unwrap_err();
    assert!(
        error.contains("Missing pinned artifact xberg-io/captioning-models@")
            && error.contains("smolvlm-256m-instruct/LICENSE"),
        "{error}"
    );
}

#[cfg(feature = "classification-onnx")]
#[test]
fn the_classifier_manifests_are_derived_from_the_inventory_pins() {
    let model = nli_minilm2_l6_h768_classifier().unwrap();
    let files = CANDIDATE_MODELS[0].files().unwrap();
    let mut seen = 0;
    for line in model.sha256_manifest.lines() {
        let (sha, path) = line.split_once("  ").unwrap();
        let pin = files.iter().find(|file| file.path == path).unwrap();
        assert_eq!(sha, pin.sha256);
        seen += 1;
    }
    let mut sizes_seen = 0;
    for line in model.size_manifest.lines() {
        let (size, path) = line.split_once("  ").unwrap();
        let pin = files.iter().find(|file| file.path == path).unwrap();
        assert_eq!(size.parse::<u64>().unwrap(), pin.size_bytes);
        sizes_seen += 1;
    }
    assert_eq!(seen, sizes_seen);
    assert_eq!(seen, 1 + CANDIDATE_MODELS[0].required_files.len());
}

#[test]
fn an_unsupported_target_fails_instead_of_substituting_another_graph() {
    let nli = &CANDIDATE_MODELS[0];
    let error = nli
        .files_for_target(CandidateTarget::Unsupported, CandidatePrecision::Quantized)
        .unwrap_err();
    assert!(error.contains("has no quantized graph for"), "{error}");
    assert!(error.contains("exceeds the adapter's cap"), "{error}");

    let arm64: BTreeSet<String> = nli
        .files_for_target(CandidateTarget::Aarch64, CandidatePrecision::Quantized)
        .unwrap()
        .into_iter()
        .map(|file| file.path)
        .collect();
    assert!(arm64.contains(&format!("{}/model_qint8_arm64.onnx", nli.id)));
    let arm64_graphs: BTreeSet<&String> = arm64.iter().filter(|path| path.ends_with(".onnx")).collect();
    let avx2: BTreeSet<String> = nli
        .files_for_target(CandidateTarget::X86_64, CandidatePrecision::Quantized)
        .unwrap()
        .into_iter()
        .map(|file| file.path)
        .collect();
    assert!(avx2.contains(&format!("{}/model_quint8_avx2.onnx", nli.id)));
    let avx2_graphs: BTreeSet<&String> = avx2.iter().filter(|path| path.ends_with(".onnx")).collect();
    assert!(arm64_graphs.is_disjoint(&avx2_graphs));
}

#[test]
fn no_fixed_taxonomy_candidate_is_staged_and_its_task_asks_for_one_explicitly() {
    assert!(CandidateTask::FixedTaxonomyClassification.models().is_empty());
    let mut probe = CANDIDATE_MODELS[0];
    probe.task = CandidateTask::FixedTaxonomyClassification;
    let files = CANDIDATE_MODELS[0].files().unwrap();
    let error = probe
        .select_files(CandidateTarget::host(), &files, CandidatePrecision::Quantized)
        .unwrap_err();
    assert!(
        error.contains("no fixed-taxonomy classification candidate is staged"),
        "{error}"
    );
}
