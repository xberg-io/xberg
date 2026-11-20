use std::collections::BTreeSet;

use sha2::{Digest, Sha256};

use super::*;

fn pins_of(manifest: &str) -> BTreeSet<(String, String)> {
    crate::model_download::parse_sha256_manifest(manifest)
        .unwrap()
        .into_iter()
        .collect()
}

fn pinned_file(bytes: &[u8]) -> CandidateFile {
    CandidateFile {
        path: "probe/file.bin".to_owned(),
        sha256: hex::encode(Sha256::digest(bytes)),
        size_bytes: bytes.len() as u64,
        repository: "xberg-io/probe".to_owned(),
        revision: "a".repeat(40),
        license: "Apache-2.0".to_owned(),
    }
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
        assert!(
            model.license.starts_with("Apache-2.0") || model.license == "MIT",
            "{}",
            model.id
        );
        assert!(!model.upstream_repository.starts_with("xberg-io/"), "{}", model.id);
        assert_eq!(CandidateModel::find(model.id), Some(model));
    }
    assert_eq!(CandidateModel::find("not-a-candidate"), None);
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
fn the_inventory_pins_sizes_licenses_and_owned_sources_per_file() {
    for model in &CANDIDATE_MODELS {
        for file in model.files().unwrap() {
            assert_eq!(file.repository, model.repository, "{}", file.path);
            assert_eq!(file.revision, model.revision, "{}", file.path);
            assert_eq!(file.license, model.license, "{}", file.path);
            assert_eq!(file.sha256.len(), 64, "{}", file.path);
            assert!(file.size_bytes > 0, "{} has no size", file.path);
            assert_eq!(
                file.label(),
                format!("{}@{}:{}", model.repository, model.revision, file.path)
            );
        }
    }
    let pinned = CANDIDATE_MODELS[0].files().unwrap();
    let tokenizer = pinned
        .iter()
        .find(|file| file.path.ends_with("/tokenizer.json"))
        .unwrap();
    assert_eq!(tokenizer.size_bytes, 1_356_048);
    for model in &CANDIDATE_MODELS {
        assert!(!model.provenance().unwrap().is_empty(), "{}", model.id);
    }
}

#[test]
fn the_inventory_and_the_sha256_manifests_agree_exactly() {
    for model in &CANDIDATE_MODELS {
        let recorded: BTreeSet<(String, String)> = model
            .files()
            .unwrap()
            .into_iter()
            .map(|file| (file.path, file.sha256))
            .collect();
        let manifest: BTreeSet<(String, String)> = pins_of(model.sha256_manifest)
            .into_iter()
            .filter(|(path, _)| path.starts_with(&format!("{}/", model.id)))
            .collect();
        assert_eq!(
            recorded, manifest,
            "{} inventory drifted from its sha256 manifest",
            model.id
        );
    }
    assert!(
        !INVENTORY_JSON.contains("\"main\""),
        "no floating revision may be pinned"
    );
}

#[test]
fn a_task_selection_returns_only_its_own_models() {
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
fn task_files_exclude_the_other_task_and_optional_graphs() {
    let all: BTreeSet<String> = CANDIDATE_MODELS
        .iter()
        .flat_map(|model| model.files().unwrap())
        .map(|file| file.path)
        .collect();
    for model in &CANDIDATE_MODELS {
        let required = model.task_files().unwrap();
        let optional = model.optional_files().unwrap();
        let required_paths: BTreeSet<&str> = required.iter().map(|file| file.path.as_str()).collect();
        let optional_paths: BTreeSet<&str> = optional.iter().map(|file| file.path.as_str()).collect();
        assert!(required_paths.is_disjoint(&optional_paths), "{}", model.id);
        assert_eq!(
            required_paths.len() + optional_paths.len(),
            model.files().unwrap().len()
        );
        for path in required_paths.iter().chain(&optional_paths) {
            assert!(all.contains(*path), "{path}");
        }
        for file in required.iter().chain(&optional) {
            assert!(file.path.starts_with(&format!("{}/", model.id)), "{}", file.path);
        }
        assert!(required_paths.iter().any(|path| path.ends_with("/LICENSE")));
        assert!(required_paths.iter().any(|path| path.ends_with("/UPSTREAM_README.md")));
    }
    let nli = &CANDIDATE_MODELS[0];
    if let Some(graph) = nli_minilm2_l6_h768_graph() {
        let required: BTreeSet<String> = nli.task_files().unwrap().into_iter().map(|file| file.path).collect();
        assert!(required.contains(&format!("{}/{graph}", nli.id)));
        assert!(!required.contains(&format!("{}/model.onnx", nli.id)));
        assert!(
            !nli.optional_files()
                .unwrap()
                .iter()
                .any(|file| file.path.ends_with(&format!("/{graph}"))),
            "the selected graph must not be optional"
        );
    }
}

#[test]
fn the_nli_graph_alternatives_are_pinned_and_its_license_caveat_is_recorded() {
    let nli = &CANDIDATE_MODELS[0];
    let pinned: BTreeSet<String> = nli.files().unwrap().into_iter().map(|f| f.path).collect();
    for graph in ["model.onnx", "model_qint8_arm64.onnx", "model_quint8_avx2.onnx"] {
        assert!(pinned.contains(&format!("{}/{graph}", nli.id)), "{graph}");
    }
    assert!(
        !nli.required_files.contains(&"model.onnx"),
        "the fp32 graph is over the adapter cap"
    );
    assert!(nli.license.contains("unstated"), "{}", nli.license);
    let entry = nli.inventory_model().unwrap();
    assert!(entry.license_caveat.unwrap().contains("nreimers/MiniLMv2"));
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
fn every_checksum_path_belongs_to_exactly_one_candidate() {
    let sums = pins_of(CLASSIFICATION_SHA256_MANIFEST).len() + pins_of(CAPTIONING_SHA256_MANIFEST).len();
    let inventoried: usize = CANDIDATE_MODELS.iter().map(|m| m.files().unwrap().len()).sum();
    assert_eq!(inventoried, sums, "a manifest path belongs to no candidate");
}

#[test]
fn each_manifest_serves_only_its_own_repository() {
    for model in &CANDIDATE_MODELS {
        let own: BTreeSet<String> = if model.repository.ends_with("classification-models") {
            pins_of(CLASSIFICATION_SHA256_MANIFEST)
        } else {
            pins_of(CAPTIONING_SHA256_MANIFEST)
        }
        .into_iter()
        .map(|(path, _)| path)
        .collect();
        let prefix = format!("{}/", model.id);
        assert!(own.iter().any(|p| p.starts_with(&prefix)), "{}", model.id);
    }
}

#[test]
fn truncated_or_malformed_checksum_lines_are_rejected_by_the_shared_parser() {
    let line = CLASSIFICATION_SHA256_MANIFEST
        .lines()
        .find(|line| !line.starts_with('#'))
        .unwrap()
        .split_once("  ")
        .unwrap()
        .0;
    let truncated = format!("{}  nli-minilm2-l6-h768/model.onnx\n", &line[..63]);
    assert!(
        crate::model_download::parse_sha256_manifest(&truncated)
            .unwrap_err()
            .contains("checksum must be SHA256 hex")
    );
    let traversal = format!("{line}  ../nli-minilm2-l6-h768/model.onnx\n");
    assert!(
        crate::model_download::parse_sha256_manifest(&traversal)
            .unwrap_err()
            .contains("path must be a relative artifact file")
    );
}

#[test]
fn a_malformed_inventory_fails_closed() {
    for (document, expected) in [
        ("not json", "candidate inventory is malformed"),
        ("{\"schema_version\":2,\"models\":[]}", "schema 2 is unsupported"),
        (
            "{\"schema_version\":1,\"models\":[{\"id\":\"x\",\"task\":\"image_captioning\",\"status\":\"experimental_not_qualified\",\"repository\":\"o/r\",\"revision\":\"a\",\"upstream_repository\":\"u/r\",\"upstream_revision\":\"b\",\"license\":\"MIT\",\"files\":[]}]}",
            "has no files",
        ),
    ] {
        let result: Result<InventoryDocument, _> = serde_json::from_str(document);
        if let Ok(document) = result {
            let error = validate_inventory(&document).unwrap_err();
            assert!(error.contains(expected), "expected {expected:?} in {error:?}");
        } else {
            assert!(expected.contains("malformed"), "{document}");
        }
    }
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
    let error = CANDIDATE_MODELS[1].verify_downloaded_set(dir.path()).unwrap_err();
    assert!(
        error.contains("Missing pinned artifact xberg-io/captioning-models@")
            && error.contains("smolvlm-256m-instruct/LICENSE"),
        "{error}"
    );
}

#[test]
fn both_candidate_families_are_registered_in_the_vendored_manifest_registry() {
    let registry = crate::model_download::vendored_model_manifests();
    for (family, manifest) in [
        ("candidate_classification", CLASSIFICATION_SHA256_MANIFEST),
        ("candidate_captioning", CAPTIONING_SHA256_MANIFEST),
    ] {
        let registered: Vec<_> = registry.iter().filter(|(name, _)| *name == family).collect();
        assert_eq!(registered.len(), 1, "{family} must be registered exactly once");
        assert_eq!(registered[0].1, manifest, "{family}");
    }
}
