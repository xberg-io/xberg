use std::collections::BTreeSet;

use sha2::{Digest, Sha256};

use super::*;

fn pins_of(manifest: &str) -> BTreeSet<String> {
    crate::model_download::parse_sha256_manifest(manifest)
        .unwrap()
        .into_iter()
        .map(|(path, _)| path)
        .collect()
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
        assert!(["Apache-2.0", "MIT"].contains(&model.license), "{}", model.id);
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
        assert!(model.validate_revisions().is_err(), "owned revision {floating:?}");
        let mut model = CANDIDATE_MODELS[0];
        model.upstream_revision = floating;
        assert!(model.validate_revisions().is_err(), "upstream revision {floating:?}");
    }
    assert_ne!(CLASSIFICATION_MODELS_REVISION, CAPTIONING_MODELS_REVISION);
}

#[test]
fn every_required_file_and_the_license_are_pinned_with_a_nonzero_size() {
    for model in &CANDIDATE_MODELS {
        let files = model.files().unwrap();
        let pinned: BTreeSet<&str> = files.iter().map(|f| f.path.as_str()).collect();
        for required in model.required_files.iter().chain(&["LICENSE", "UPSTREAM_README.md"]) {
            let path = format!("{}/{required}", model.id);
            assert!(pinned.contains(path.as_str()), "{path} is not pinned");
        }
        for file in &files {
            assert!(file.size_bytes > 0, "{} has no size", file.path);
            assert_eq!(file.sha256.len(), 64, "{}", file.path);
        }
    }
}

#[test]
fn size_and_checksum_manifests_describe_exactly_the_same_files() {
    let sums: BTreeSet<String> = pins_of(CLASSIFICATION_SHA256_MANIFEST)
        .into_iter()
        .chain(pins_of(CAPTIONING_SHA256_MANIFEST))
        .collect();
    let sizes: BTreeSet<String> = SIZE_MANIFEST
        .lines()
        .filter(|line| !line.starts_with('#') && !line.trim().is_empty())
        .map(|line| line.split_once("  ").unwrap().1.to_owned())
        .collect();
    assert_eq!(sums, sizes);
    let inventoried: usize = CANDIDATE_MODELS.iter().map(|m| m.files().unwrap().len()).sum();
    assert_eq!(inventoried, sums.len(), "a manifest path belongs to no candidate");
}

#[test]
fn each_manifest_serves_only_its_own_repository() {
    for model in &CANDIDATE_MODELS {
        let own = if model.repository.ends_with("classification-models") {
            pins_of(CLASSIFICATION_SHA256_MANIFEST)
        } else {
            pins_of(CAPTIONING_SHA256_MANIFEST)
        };
        let prefix = format!("{}/", model.id);
        assert!(own.iter().any(|p| p.starts_with(&prefix)), "{}", model.id);
    }
}

#[test]
fn truncated_or_malformed_checksum_lines_are_rejected_by_the_shared_parser() {
    let (line, _) = CLASSIFICATION_SHA256_MANIFEST
        .lines()
        .find(|line| !line.starts_with('#'))
        .unwrap()
        .split_once("  ")
        .unwrap();
    let truncated = format!("{}  nli-minilm2-l6-h768/model.onnx\n", &line[..63]);
    assert!(crate::model_download::parse_sha256_manifest(&truncated).is_err());
    let traversal = format!("{line}  ../nli-minilm2-l6-h768/model.onnx\n");
    assert!(crate::model_download::parse_sha256_manifest(&traversal).is_err());
}

fn small_pin(bytes: &[u8]) -> CandidateFile {
    CandidateFile {
        path: "m/file.bin".to_owned(),
        sha256: hex::encode(Sha256::digest(bytes)),
        size_bytes: bytes.len() as u64,
    }
}

#[test]
fn exact_bytes_pass_and_corrupted_truncated_or_other_revision_bytes_fail() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("file.bin");
    let pin = small_pin(b"upstream revision A");

    std::fs::write(&file, b"upstream revision A").unwrap();
    verify_against_pin(&pin, &file).unwrap();

    std::fs::write(&file, b"upstream revision B").unwrap();
    assert!(
        verify_against_pin(&pin, &file)
            .unwrap_err()
            .contains("Checksum mismatch"),
        "same-size corruption or a file from another revision"
    );

    std::fs::write(&file, b"upstream revision").unwrap();
    assert!(verify_against_pin(&pin, &file).unwrap_err().contains("Size mismatch"));

    std::fs::remove_file(&file).unwrap();
    assert!(verify_against_pin(&pin, &file).unwrap_err().contains("cannot stat"));
}

#[test]
fn a_real_pin_rejects_a_file_with_other_content_and_unpinned_names() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("tokenizer_config.json");
    std::fs::write(&file, b"{}").unwrap();
    let model = &CANDIDATE_MODELS[0];
    assert!(
        model
            .verify_file("tokenizer_config.json", &file)
            .unwrap_err()
            .contains("Size mismatch")
    );
    assert!(
        model
            .verify_file("not-pinned.bin", &file)
            .unwrap_err()
            .contains("is not pinned")
    );
}
