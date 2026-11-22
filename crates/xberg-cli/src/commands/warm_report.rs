//! Rendering for `xberg cache warm`: the download summary and the experimental candidate
//! catalog, in text/JSON/TOON. Kept separate from the command orchestration in [`super::cache`].

use std::path::Path;

use anyhow::{Context, Result};
use serde_json::json;

use crate::{WireFormat, style};

/// Print the warm command's summary in the requested wire format.
#[cfg(any(
    feature = "embeddings",
    feature = "layout-detection",
    feature = "paddle-ocr",
    feature = "tree-sitter",
    feature = "ner-onnx",
    feature = "classification-onnx",
    feature = "captioning-onnx"
))]
#[expect(
    clippy::print_stdout,
    reason = "cache warm download summary is the command's stdout result output"
)]
pub(super) fn print_warm_summary(
    format: WireFormat,
    cache_base: &Path,
    hf_cache_label: &Option<String>,
    downloaded: &[String],
    already_cached: &[String],
    #[cfg(any(feature = "classification-onnx", feature = "captioning-onnx"))]
    candidates: &[super::onnx_candidates::CandidateSummary],
) -> Result<()> {
    match format {
        WireFormat::Text => {
            if !downloaded.is_empty() {
                println!("{}", style::label("Downloaded:"));
                for d in downloaded {
                    println!("  {}", style::success(d));
                }
            }
            if !already_cached.is_empty() {
                println!("{}", style::label("Already cached:"));
                for c in already_cached {
                    println!("  {}", style::dim(c));
                }
            }
            println!(
                "Xberg-managed cache: {}",
                style::success(&cache_base.display().to_string())
            );
            if let Some(hf_cache) = hf_cache_label {
                println!("Hugging Face cache: {}", style::success(hf_cache));
            }
            #[cfg(any(feature = "classification-onnx", feature = "captioning-onnx"))]
            for candidate in candidates {
                println!("{}", style::label(&candidate.feature.to_uppercase()));
                println!("  {}", candidate.headline());
                for artifact in &candidate.artifacts {
                    println!(
                        "  {} {} size={} sha256={}",
                        style::label(artifact.state),
                        artifact.path,
                        artifact.size_bytes,
                        artifact.sha256
                    );
                }
            }
        }
        WireFormat::Json => {
            let output = warm_output(
                cache_base,
                hf_cache_label,
                downloaded,
                already_cached,
                #[cfg(any(feature = "classification-onnx", feature = "captioning-onnx"))]
                candidates,
            )?;
            println!(
                "{}",
                serde_json::to_string_pretty(&output).context("Failed to serialize warm results to JSON")?
            );
        }
        WireFormat::Toon => {
            let output = warm_output(
                cache_base,
                hf_cache_label,
                downloaded,
                already_cached,
                #[cfg(any(feature = "classification-onnx", feature = "captioning-onnx"))]
                candidates,
            )?;
            println!(
                "{}",
                serde_toon::to_string(&output).context("Failed to serialize warm results to TOON")?
            );
        }
    }

    Ok(())
}

/// The warm summary as JSON, including the experimental candidate catalog when compiled in.
#[cfg(any(
    feature = "embeddings",
    feature = "layout-detection",
    feature = "paddle-ocr",
    feature = "tree-sitter",
    feature = "ner-onnx",
    feature = "classification-onnx",
    feature = "captioning-onnx"
))]
fn warm_output(
    cache_base: &Path,
    hf_cache_label: &Option<String>,
    downloaded: &[String],
    already_cached: &[String],
    #[cfg(any(feature = "classification-onnx", feature = "captioning-onnx"))]
    candidates: &[super::onnx_candidates::CandidateSummary],
) -> Result<serde_json::Value> {
    #[cfg_attr(
        not(any(feature = "classification-onnx", feature = "captioning-onnx")),
        expect(unused_mut, reason = "the feature-gated candidates field is the only mutation")
    )]
    let mut output = json!({
        "cache_dir": cache_base.to_string_lossy(),
        "xberg_cache_dir": cache_base.to_string_lossy(),
        "hugging_face_cache_dir": hf_cache_label,
        "downloaded": downloaded,
        "already_cached": already_cached,
    });
    #[cfg(any(feature = "classification-onnx", feature = "captioning-onnx"))]
    if let Some(object) = output.as_object_mut() {
        object.insert(
            "candidates".to_owned(),
            serde_json::to_value(candidates).context("Failed to serialize candidate state")?,
        );
    }
    Ok(output)
}
