//! Verify that the committed `parity-manifest.json` matches what the generator
//! would produce at runtime.  If this test fails, run:
//!
//!     task generate:manifest
//!
//! to regenerate the manifest and commit the result.

use assert_cmd::Command;
use std::path::Path;

/// Regenerate the manifest into a temp file and assert it is identical to the
/// committed version at `tools/e2e-generator/parity-manifest.json`.
#[test]
fn committed_manifest_is_fresh() {
    let tmp_dir = std::env::temp_dir().join("kreuzberg-manifest-freshness");
    let _ = std::fs::create_dir_all(&tmp_dir);
    let tmp_manifest = tmp_dir.join("parity-manifest.json");

    // Generate a fresh manifest to the temp path.
    Command::cargo_bin("kreuzberg-e2e-generator")
        .expect("binary exists")
        .args(["manifest", "--output", tmp_manifest.to_str().unwrap()])
        .assert()
        .success();

    // Find the committed manifest relative to the workspace root.
    // The binary runs from the workspace root, so the default path works,
    // but for this test we locate it via CARGO_MANIFEST_DIR.
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let committed = Path::new(manifest_dir).join("parity-manifest.json");

    assert!(
        committed.exists(),
        "committed manifest not found at {}",
        committed.display()
    );

    let committed_json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&committed).expect("read committed manifest"))
            .expect("parse committed manifest");

    let generated_json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&tmp_manifest).expect("read generated manifest"))
            .expect("parse generated manifest");

    assert_eq!(
        committed_json, generated_json,
        "Committed parity-manifest.json is stale. Run `task generate:manifest` to update it."
    );

    // Cleanup
    let _ = std::fs::remove_dir_all(&tmp_dir);
}
