use std::fs;
use std::path::PathBuf;

use super::{COMPILED_PRODUCER_SOURCES, bytes_sha256, validate_producer_source_inputs};

fn compiled_source_tree() -> (tempfile::TempDir, PathBuf) {
    let temporary = tempfile::tempdir().expect("owned source tree");
    let root = temporary.path().join("workspace/crates/xberg");
    for (path, bytes) in COMPILED_PRODUCER_SOURCES {
        let destination = root.join(path);
        fs::create_dir_all(destination.parent().expect("source parent")).expect("source directories");
        fs::write(&destination, bytes).expect("exact compiled source fixture");
    }
    assert_eq!(COMPILED_PRODUCER_SOURCES.len(), 20);
    for (path, bytes) in COMPILED_PRODUCER_SOURCES {
        assert_eq!(fs::read(root.join(path)).unwrap(), *bytes);
    }
    assert!(
        root.canonicalize()
            .unwrap()
            .starts_with(temporary.path().canonicalize().unwrap())
    );
    (temporary, root)
}

fn assert_changed_source_rejected(path: &str) {
    let (_temporary, root) = compiled_source_tree();
    let compiled = COMPILED_PRODUCER_SOURCES
        .iter()
        .find(|(name, _)| *name == path)
        .expect("actual producer source")
        .1;
    assert!(!compiled.is_empty());
    let changed = [compiled, b"\nchanged producer bytes\n"].concat();
    fs::write(root.join(path), &changed).expect("mutate only owned source copy");
    assert_ne!(bytes_sha256(&changed), bytes_sha256(compiled));
    let error = validate_producer_source_inputs(&root)
        .expect_err("changed live source must fail before producer receipts or model construction");
    assert!(error.contains(path), "source mismatch must identify {path}: {error}");
    assert!(error.contains("compiled source mismatch"), "{error}");
    assert!(!root.join("producer").exists());
}

#[test]
fn should_accept_all_twenty_compiled_sources_before_capture() {
    let (_temporary, root) = compiled_source_tree();
    validate_producer_source_inputs(&root).expect("all actual compiled source bytes match");
    assert!(!root.join("producer").exists());
}

#[test]
fn should_reject_changed_output_validator_before_capture() {
    assert_changed_source_rejected("src/layout/models/pp_doclayout_v3/output.rs");
}

#[test]
fn should_reject_changed_preprocessing_before_capture() {
    assert_changed_source_rejected("src/layout/models/pp_doclayout_v3/preprocessing.rs");
}

#[test]
fn should_reject_changed_image_decoder_or_budget_before_capture() {
    for path in ["src/extraction/image_decode.rs", "src/extractors/security.rs"] {
        assert_changed_source_rejected(path);
    }
}
