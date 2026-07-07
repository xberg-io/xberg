//! Auto-download and checksum-verify local weights for the Candle VLM-OCR backends.
//!
//! Some VLM-OCR checkpoints are not fetchable through `hf-hub` at runtime: Tencent
//! pulled `tencent/HunyuanOCR` from the Hugging Face Hub, so the backend can no
//! longer point `hf-hub` at it the way GLM-OCR and TrOCR do. This module stages the
//! weights from the model publisher's official ModelScope release instead and
//! verifies every file against a checked-in sha256 manifest before use.
//!
//! Trust attaches to the manifest, not the host: a changed or tampered upstream
//! file fails the staging step instead of silently feeding wrong weights into
//! inference. The staged copy lands in the shared xberg cache, so it is fetched
//! once and reused across runs.

use std::path::{Path, PathBuf};

use crate::model_download::verify_sha256;

/// A checksum-pinned VLM-OCR model hosted on ModelScope.
struct ModelScopeModel {
    /// ModelScope repo id, e.g. `Tencent-Hunyuan/HunyuanOCR`.
    repo: &'static str,
    /// Cache subdirectory under the xberg cache root.
    cache_key: &'static str,
    /// `sha256sum`-format manifest: `<sha256>  <filename>` per line, `#` comments.
    /// One entry per file the engine reads, checked in as the single source of truth.
    manifest: &'static str,
}

/// Hunyuan-OCR — Tencent's official ModelScope release (`Tencent-Hunyuan/HunyuanOCR`).
/// The file list and checksums live in the checked-in `.sha256` manifest, which CI
/// (`sha256sum --check`) reads too — updating the model means editing one file.
const HUNYUAN_OCR: ModelScopeModel = ModelScopeModel {
    repo: "Tencent-Hunyuan/HunyuanOCR",
    cache_key: "candle-ocr/hunyuan-ocr",
    manifest: include_str!("hunyuan-ocr.sha256"),
};

/// One file in a parsed manifest: the name (both cache filename and remote resolve
/// path) and the sha256 the downloaded bytes must match.
struct ManifestFile {
    name: String,
    sha256: String,
}

/// Parse a `sha256sum`-format manifest into an ordered file list.
///
/// Skips blank lines and `#` comments; each remaining line must be
/// `<64-hex-sha256>  <path>`. Mirrors the GLiNER checksum parser
/// ([`crate::text::ner`]) so the format is consistent across the codebase.
fn parse_manifest(content: &str) -> Result<Vec<ManifestFile>, String> {
    let mut files = Vec::new();
    for (index, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let mut parts = trimmed.split_whitespace();
        let sha256 = parts
            .next()
            .ok_or_else(|| format!("Invalid manifest line {}: missing checksum", index + 1))?;
        let name = parts
            .next()
            .ok_or_else(|| format!("Invalid manifest line {}: missing filename", index + 1))?;
        if sha256.len() != 64 || !sha256.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(format!(
                "Invalid manifest line {}: checksum must be SHA256 hex",
                index + 1
            ));
        }
        files.push(ManifestFile {
            name: name.trim_start_matches("./").to_string(),
            sha256: sha256.to_ascii_lowercase(),
        });
    }
    if files.is_empty() {
        return Err("Manifest lists no files".to_string());
    }
    Ok(files)
}

/// Monotonic counter giving each staged download a unique temp filename so
/// concurrent stagers of the same file never collide on the staging path.
static STAGE_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Ensure the Hunyuan-OCR weights are present locally and return the model directory.
///
/// Missing or checksum-invalid files are (re)downloaded from the official ModelScope
/// release and verified before use; already-valid files are left untouched, so a
/// warm cache skips the network entirely. The returned directory is ready to hand
/// to `HunyuanOCREngine::init`.
pub(crate) fn ensure_hunyuan_ocr() -> Result<PathBuf, String> {
    ensure_model(
        &HUNYUAN_OCR,
        &crate::cache_dir::resolve_cache_dir(HUNYUAN_OCR.cache_key),
    )
}

fn ensure_model(model: &ModelScopeModel, dir: &Path) -> Result<PathBuf, String> {
    let files = parse_manifest(model.manifest)?;
    std::fs::create_dir_all(dir).map_err(|e| format!("Failed to create model cache dir {}: {e}", dir.display()))?;

    for file in &files {
        let dst = dir.join(&file.name);

        // A file already present with the right bytes needs no network round-trip.
        if dst.exists() && verify_sha256(&dst, &file.sha256, &file.name).is_ok() {
            continue;
        }

        let url = modelscope_url(model.repo, &file.name);
        tracing::info!(file = %file.name, %url, "Staging Hunyuan-OCR weight");
        stage_file(&url, dir, &file.name, &file.sha256)?;
    }

    Ok(dir.to_path_buf())
}

/// ModelScope resolve URL for a file at repo `master`.
fn modelscope_url(repo: &str, name: &str) -> String {
    format!("https://modelscope.cn/models/{repo}/resolve/master/{name}")
}

/// Download `url` to a per-call temp file in `dir`, verify its sha256, then atomically
/// rename it into `dir/name`. The download + verify happens on a private temp path and
/// only a checksum-valid file is published, so a torn or tampered download can never be
/// observed as the real weight, and concurrent stagers each swap in their own copy.
fn stage_file(url: &str, dir: &Path, name: &str, sha256: &str) -> Result<(), String> {
    let tmp = dir.join(format!(
        ".{name}.{}.{}.tmp",
        std::process::id(),
        STAGE_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
    ));

    let result = download_to(url, &tmp).and_then(|()| verify_sha256(&tmp, sha256, name));
    if let Err(e) = result {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }

    std::fs::rename(&tmp, dir.join(name)).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        format!("Failed to publish {name} to {}: {e}", dir.display())
    })
}

/// Stream an HTTP(S) GET body to `dst`, retrying transient failures.
fn download_to(url: &str, dst: &Path) -> Result<(), String> {
    let mut last_err = String::new();
    for attempt in 0..MAX_DOWNLOAD_ATTEMPTS {
        match try_download_to(url, dst) {
            Ok(()) => return Ok(()),
            Err(e) => {
                last_err = e;
                tracing::warn!(attempt = attempt + 1, url, error = %last_err, "Weight download failed, retrying");
            }
        }
    }
    Err(format!(
        "Failed to download {url} after {MAX_DOWNLOAD_ATTEMPTS} attempts: {last_err}"
    ))
}

const MAX_DOWNLOAD_ATTEMPTS: u32 = 4;

fn try_download_to(url: &str, dst: &Path) -> Result<(), String> {
    let response = ureq::get(url).call().map_err(|e| format!("GET {url} failed: {e}"))?;
    let mut reader = response.into_body().into_reader();
    let mut file = std::fs::File::create(dst).map_err(|e| format!("Failed to create {}: {e}", dst.display()))?;
    std::io::copy(&mut reader, &mut file).map_err(|e| format!("Failed to write {}: {e}", dst.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sha256_hex(bytes: &[u8]) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        hex::encode(hasher.finalize())
    }

    #[test]
    fn hunyuan_manifest_covers_every_file_the_engine_reads() {
        let files = parse_manifest(HUNYUAN_OCR.manifest).expect("bundled manifest must parse");
        let names: Vec<&str> = files.iter().map(|f| f.name.as_str()).collect();
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
    fn parse_manifest_reads_entries_and_normalizes() {
        let files = parse_manifest(
            "# comment\n\
             AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA  ./config.json\n\
             bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb  tokenizer.json\n",
        )
        .expect("valid manifest");
        assert_eq!(files.len(), 2);
        // `./` stripped, order preserved, checksum lowercased.
        assert_eq!(files[0].name, "config.json");
        assert_eq!(files[0].sha256, "a".repeat(64));
        assert_eq!(files[1].name, "tokenizer.json");
    }

    #[test]
    fn parse_manifest_rejects_malformed_lines() {
        assert!(
            parse_manifest("not-a-sha256  config.json").is_err(),
            "short/invalid hash"
        );
        assert!(parse_manifest(&"a".repeat(64)).is_err(), "missing filename");
        assert!(parse_manifest("# only comments\n").is_err(), "no files");
    }

    #[test]
    fn modelscope_url_targets_the_official_release() {
        assert_eq!(
            modelscope_url("Tencent-Hunyuan/HunyuanOCR", "config.json"),
            "https://modelscope.cn/models/Tencent-Hunyuan/HunyuanOCR/resolve/master/config.json"
        );
    }

    #[test]
    fn ensure_model_skips_download_when_cache_is_valid() {
        // A model whose one manifest file already exists with the right bytes must
        // not attempt any network access (the repo below is unreachable in tests).
        let dir = tempfile::tempdir().unwrap();
        let payload = b"cached weight bytes";
        std::fs::write(dir.path().join("config.json"), payload).unwrap();

        let manifest: &'static str = Box::leak(format!("{}  config.json\n", sha256_hex(payload)).into_boxed_str());
        let model = ModelScopeModel {
            repo: "unreachable/repo",
            cache_key: "unused",
            manifest,
        };

        let out = ensure_model(&model, dir.path()).expect("valid cache must short-circuit");
        assert_eq!(out, dir.path());
    }

    /// End-to-end check of the real ureq → verify → publish path against the live
    /// ModelScope release, using only the small config files (no multi-GB shards).
    /// Ignored by default (network); run with `--ignored`.
    #[test]
    #[ignore = "hits modelscope.cn; run with --ignored"]
    fn stages_hunyuan_config_files_from_modelscope() {
        let dir = tempfile::tempdir().unwrap();
        // A sub-manifest of just the small JSON files (the first four bundled entries).
        let bundled = parse_manifest(HUNYUAN_OCR.manifest).unwrap();
        let small = &bundled[..4];
        let manifest: &'static str = Box::leak(
            small
                .iter()
                .map(|f| format!("{}  {}", f.sha256, f.name))
                .collect::<Vec<_>>()
                .join("\n")
                .into_boxed_str(),
        );
        let model = ModelScopeModel {
            repo: HUNYUAN_OCR.repo,
            cache_key: "unused",
            manifest,
        };

        let out = ensure_model(&model, dir.path()).expect("staging must succeed");
        for f in small {
            let path = out.join(&f.name);
            assert!(path.exists(), "{} should be staged", f.name);
            verify_sha256(&path, &f.sha256, &f.name).expect("staged file must match manifest checksum");
        }

        // A second call with a warm cache must not re-download (and still succeed).
        ensure_model(&model, dir.path()).expect("warm cache must succeed");
    }

    #[test]
    fn stage_file_rejects_checksum_mismatch_and_leaves_no_partial() {
        let dir = tempfile::tempdir().unwrap();
        // Point at a real file:// style unreachable host so the download itself fails;
        // whichever fails first, the published file must not exist and no temp leaks.
        let err = stage_file(
            "https://modelscope.cn/models/does-not/exist/resolve/master/config.json",
            dir.path(),
            "config.json",
            &"0".repeat(64),
        )
        .expect_err("bad download/checksum must fail");
        assert!(!err.is_empty());
        assert!(
            !dir.path().join("config.json").exists(),
            "no file may be published on failure"
        );

        let leftovers: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| e.file_name().to_string_lossy().ends_with(".tmp"))
            .collect();
        assert!(
            leftovers.is_empty(),
            "staging temp must be cleaned up, found {leftovers:?}"
        );
    }
}
