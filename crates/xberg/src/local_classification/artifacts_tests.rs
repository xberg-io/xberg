use super::artifacts::*;
use super::tests::configuration;
use sha2::{Digest, Sha256};
use std::fs::{File, OpenOptions};
use std::io::Write;

#[test]
fn refuses_asset_before_loading_when_size_exceeds_remaining_budget() {
    let root = tempfile::tempdir().unwrap();
    let asset = root.path().join("model.onnx");
    File::create(&asset).unwrap().set_len(4096).unwrap();
    let mut budget = ArtifactBudget::new(4);
    let error = read_verified(&asset, "model.onnx", &"0".repeat(64), root.path(), 4096, &mut budget).unwrap_err();
    assert!(error.to_string().contains("remaining artifact byte limit"));
    assert_eq!(budget.remaining(), 4);
}

#[test]
fn accepts_exact_pinned_bytes_and_rejects_same_size_mutation() {
    let root = tempfile::tempdir().unwrap();
    let asset = root.path().join("tokenizer.json");
    let checksum = hex::encode(Sha256::digest(b"test"));
    std::fs::write(&asset, b"test").unwrap();
    let mut budget = ArtifactBudget::new(8);
    assert_eq!(
        read_verified(&asset, "tokenizer.json", &checksum, root.path(), 4, &mut budget).unwrap(),
        b"test"
    );
    assert_eq!(budget.remaining(), 4);
    std::fs::write(&asset, b"fake").unwrap();
    let error = read_verified(&asset, "tokenizer.json", &checksum, root.path(), 4, &mut budget).unwrap_err();
    assert!(error.to_string().contains("checksum mismatch"));
}

#[test]
fn rejects_growth_and_truncation_after_descriptor_size_snapshot() {
    let root = tempfile::tempdir().unwrap();
    let asset = root.path().join("asset");
    std::fs::write(&asset, b"test").unwrap();
    let mut reader = File::open(&asset).unwrap();
    let mut writer = OpenOptions::new().append(true).open(&asset).unwrap();
    writer.write_all(b"+").unwrap();
    assert!(
        read_exact_snapshot(&mut reader, 4, "asset")
            .unwrap_err()
            .to_string()
            .contains("changed")
    );

    let mut reader = File::open(&asset).unwrap();
    writer.set_len(3).unwrap();
    assert!(
        read_exact_snapshot(&mut reader, 5, "asset")
            .unwrap_err()
            .to_string()
            .contains("changed")
    );
}

#[cfg(unix)]
#[test]
fn accepts_hub_cache_symlinks_inside_root_and_rejects_escape() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let inside = root.path().join("inside");
    let external = outside.path().join("outside");
    let link = root.path().join("link");
    let checksum = hex::encode(Sha256::digest(b"test"));
    std::fs::write(&inside, b"test").unwrap();
    std::fs::write(&external, b"test").unwrap();
    std::os::unix::fs::symlink(inside, &link).unwrap();
    let mut budget = ArtifactBudget::new(8);
    assert_eq!(
        read_verified(&link, "model.onnx", &checksum, root.path(), 4, &mut budget).unwrap(),
        b"test"
    );
    std::fs::remove_file(&link).unwrap();
    std::os::unix::fs::symlink(external, &link).unwrap();
    let error = read_verified(&link, "model.onnx", &checksum, root.path(), 4, &mut budget).unwrap_err();
    assert!(error.to_string().contains("escapes the configured cache root"));
}

#[test]
fn rejects_manifest_extras_and_ambiguous_companion_pins() {
    let mut config = configuration();
    let line = format!("{}  unrelated.json\n", "0".repeat(64));
    config.model.sha256_manifest.push_str(&line);
    assert!(
        ArtifactInventory::new(&config.model)
            .unwrap_err()
            .to_string()
            .contains("unexpected")
    );

    config = configuration();
    config.model.model_file = "onnx/model.onnx".to_owned();
    config.model.sha256_manifest = config
        .model
        .sha256_manifest
        .replace("  model.onnx", "  onnx/model.onnx");
    let line = format!("{}  onnx/tokenizer.json\n", "0".repeat(64));
    config.model.sha256_manifest.push_str(&line);
    assert!(
        ArtifactInventory::new(&config.model)
            .unwrap_err()
            .to_string()
            .contains("ambiguous")
    );
}
