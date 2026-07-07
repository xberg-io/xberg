//! Auto-download and checksum-verify local weights for the Candle VLM-OCR backends.
//!
//! Some VLM-OCR checkpoints are not fetchable from their original Hub location at
//! runtime: Tencent pulled `tencent/HunyuanOCR` from the Hugging Face Hub, so the
//! backend can no longer point `hf-hub` at it the way GLM-OCR and TrOCR do. We
//! re-host the same weights under `xberg-io/hunyuan-ocr` and fetch them through the
//! normal `hf-hub` path, verifying every file against a checked-in sha256 manifest
//! before use.
//!
//! Trust attaches to the manifest, not the host: a changed or tampered file fails
//! the staging step instead of silently feeding wrong weights into inference. The
//! shards are byte-identical to the original release, so the checked-in checksums are
//! unchanged. Caching is handled by `hf-hub` (the shared blob cache), so weights are
//! fetched once and reused across runs.

use std::path::{Path, PathBuf};

use crate::model_download::{hf_download, parse_sha256_manifest, verify_sha256};

/// A checksum-pinned VLM-OCR model hosted on the Hugging Face Hub.
struct HfModel {
    /// Hugging Face repo id, e.g. `xberg-io/hunyuan-ocr`.
    repo: &'static str,
    /// `sha256sum`-format manifest: `<sha256>  <filename>` per line, `#` comments.
    /// One entry per file the engine reads, checked in as the single source of truth.
    manifest: &'static str,
}

/// Hunyuan-OCR — re-hosted at `xberg-io/hunyuan-ocr` (the shards are byte-identical
/// to Tencent's original `tencent/HunyuanOCR`, which was pulled from the Hub). The
/// file list and checksums live in the checked-in `.sha256` manifest, which CI
/// (`sha256sum --check`) reads too — updating the model means editing one file.
const HUNYUAN_OCR: HfModel = HfModel {
    repo: "xberg-io/hunyuan-ocr",
    manifest: include_str!("hunyuan-ocr.sha256"),
};

/// Parse the model manifest into an ordered `(filename, sha256)` list, requiring at
/// least one entry. Format/validation live in [`parse_sha256_manifest`], shared with
/// the other checksum-manifest consumers.
fn manifest_files(content: &str) -> Result<Vec<(String, String)>, String> {
    let files = parse_sha256_manifest(content)?;
    if files.is_empty() {
        return Err("Manifest lists no files".to_string());
    }
    Ok(files)
}

/// Ensure the Hunyuan-OCR weights are present locally and return the model directory.
///
/// Every manifest file is fetched through `hf-hub` (warm cache hits skip the network)
/// and verified against its checked-in sha256 before use; a checksum mismatch fails
/// staging rather than feeding wrong weights into inference. The returned directory —
/// the `hf-hub` snapshot dir holding all the fetched files — is ready to hand to
/// `HunyuanOCREngine::init`.
pub(crate) fn ensure_hunyuan_ocr() -> Result<PathBuf, String> {
    ensure_model(&HUNYUAN_OCR)
}

fn ensure_model(model: &HfModel) -> Result<PathBuf, String> {
    let files = manifest_files(model.manifest)?;

    // `hf-hub` lands every file of a repo revision in the same snapshot directory, so
    // fetching all manifest files leaves the whole set side by side for the engine's
    // directory scan. Capture that directory from the first fetched file.
    let mut dir: Option<PathBuf> = None;
    for (name, sha256) in &files {
        let path = hf_download(model.repo, name)?;
        verify_sha256(&path, sha256, name)?;
        if dir.is_none() {
            dir = path.parent().map(Path::to_path_buf);
        }
    }

    dir.ok_or_else(|| format!("Fetched no files for {}", model.repo))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hunyuan_manifest_covers_every_file_the_engine_reads() {
        let files = manifest_files(HUNYUAN_OCR.manifest).expect("bundled manifest must parse");
        let names: Vec<&str> = files.iter().map(|(name, _)| name.as_str()).collect();
        // The engine reads config + generation_config + the four shards; the
        // processor reads preprocessor_config + tokenizer.
        for required in [
            "config.json",
            "generation_config.json",
            "preprocessor_config.json",
            "tokenizer.json",
            "model-00001-of-00004.safetensors",
            "model-00002-of-00004.safetensors",
            "model-00003-of-00004.safetensors",
            "model-00004-of-00004.safetensors",
        ] {
            assert!(names.contains(&required), "manifest missing {required}");
        }
        assert_eq!(
            files.len(),
            8,
            "manifest should list exactly the 8 files the engine reads"
        );
    }

    #[test]
    fn hunyuan_ocr_is_hosted_on_the_xberg_hf_repo() {
        assert_eq!(HUNYUAN_OCR.repo, "xberg-io/hunyuan-ocr");
    }

    #[test]
    fn manifest_files_requires_at_least_one_entry() {
        assert!(manifest_files("# only comments\n").is_err(), "no files");
        assert_eq!(
            manifest_files(&format!("{}  config.json\n", "a".repeat(64)))
                .unwrap()
                .len(),
            1
        );
    }

    /// End-to-end check of the real hf-hub → verify path against the re-hosted
    /// `xberg-io/hunyuan-ocr` release, using only the small config files (no multi-GB
    /// shards). Ignored by default (network); run with `--ignored`.
    #[test]
    #[ignore = "hits the HuggingFace Hub; run with --ignored"]
    fn stages_hunyuan_config_files_from_hf() {
        // A sub-manifest of just the small JSON files (the first four bundled entries).
        let bundled = manifest_files(HUNYUAN_OCR.manifest).unwrap();
        let small = &bundled[..4];
        let manifest: &'static str = Box::leak(
            small
                .iter()
                .map(|(name, sha256)| format!("{sha256}  {name}"))
                .collect::<Vec<_>>()
                .join("\n")
                .into_boxed_str(),
        );
        let model = HfModel {
            repo: HUNYUAN_OCR.repo,
            manifest,
        };

        let out = ensure_model(&model).expect("staging must succeed");
        for (name, sha256) in small {
            let path = out.join(name);
            assert!(path.exists(), "{name} should be staged in the snapshot dir");
            verify_sha256(&path, sha256, name).expect("staged file must match manifest checksum");
        }

        // A second call with a warm cache must still succeed (hf-hub serves the cached copy).
        ensure_model(&model).expect("warm cache must succeed");
    }
}
