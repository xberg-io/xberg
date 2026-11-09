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

/// A candidate whose pins describe `bytes`, so `verify_file` runs against real temp files.
fn model_pinning(bytes: &[u8]) -> CandidateModel {
    let sha = hex::encode(Sha256::digest(bytes));
    let mut model = CANDIDATE_MODELS[0];
    model.id = "probe";
    model.sha256_manifest = format!("{sha}  probe/file.bin\n").leak();
    model.size_manifest = format!("{}  probe/file.bin\n", bytes.len()).leak();
    model
}

#[test]
fn verify_file_accepts_exact_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("file.bin");
    std::fs::write(&file, b"pinned bytes").unwrap();
    model_pinning(b"pinned bytes").verify_file("file.bin", &file).unwrap();
}

#[test]
fn verify_file_rejects_same_size_corruption() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("file.bin");
    std::fs::write(&file, b"pinned bytez").unwrap();
    let error = model_pinning(b"pinned bytes")
        .verify_file("file.bin", &file)
        .unwrap_err();
    assert!(error.contains("Checksum mismatch for probe/file.bin"), "{error}");
}

#[test]
fn verify_file_rejects_truncation_and_growth() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("file.bin");
    let model = model_pinning(b"pinned bytes");
    for content in [&b"pinned byte"[..], b"pinned bytes!"] {
        std::fs::write(&file, content).unwrap();
        let error = model.verify_file("file.bin", &file).unwrap_err();
        assert!(
            error.contains("Size mismatch for probe/file.bin: expected 12 bytes"),
            "{error}"
        );
    }
}

#[test]
fn verify_file_rejects_missing_and_unpinned_files() {
    let dir = tempfile::tempdir().unwrap();
    let model = model_pinning(b"pinned bytes");
    let missing = dir.path().join("absent.bin");
    assert!(
        model
            .verify_file("file.bin", &missing)
            .unwrap_err()
            .contains("cannot stat probe/file.bin")
    );
    std::fs::write(&missing, b"pinned bytes").unwrap();
    assert!(
        model
            .verify_file("other.bin", &missing)
            .unwrap_err()
            .contains("probe/other.bin is not pinned for probe")
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
        error.contains("Size mismatch for nli-minilm2-l6-h768/tokenizer_config.json"),
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
