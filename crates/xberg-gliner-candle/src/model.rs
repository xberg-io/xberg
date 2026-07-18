//! Public `Gliner2Candle` API: load, adapter lifecycle, entity extraction.
//! Adapted from `anno::backends::gliner2_fastino_candle::GLiNER2FastinoCandle`,
//! trimmed to `extract_ner`-only scope (see plan Global Constraints).

#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;
use std::path::PathBuf;

use candle_core::Device;

#[cfg(not(target_arch = "wasm32"))]
use crate::lora;
use crate::{GlinerCandleError, Result, decode, encoder, heads, pipeline};

/// Candle-based GLiNER2 backend with PEFT LoRA adapter merge-at-load support.
pub struct Gliner2Candle {
    pub(crate) tokenizer: xberg_gliner::V2Tokenizer,
    pub(crate) splitter: xberg_gliner::V2Splitter,
    pub(crate) device: Device,
    /// Directory containing the base model's `tokenizer.json`, `config.json`
    /// (or `encoder_config/config.json`), and `model.safetensors`. Used to
    /// re-merge from disk on `load_adapter`/`unload_adapter` — only read on
    /// non-wasm32 targets; empty (`PathBuf::new()`) on the `from_bytes` path.
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    base_model_dir: PathBuf,
    pub(crate) encoder: encoder::Encoder,
    pub(crate) heads: heads::AllHeads,
    active_adapter: Option<String>,
    model_id: String,
    /// Approximate resident size in bytes — the base `model.safetensors` file
    /// size, recorded at load. A merged adapter produces a same-shape model, so
    /// this stays valid across `load_adapter`/`unload_adapter`.
    approx_bytes: u64,
}

impl std::fmt::Debug for Gliner2Candle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Gliner2Candle")
            .field("model_id", &self.model_id)
            .field("active_adapter", &self.active_adapter)
            .finish()
    }
}

impl Gliner2Candle {
    /// Active adapter name, or `None` if running on pure base weights.
    pub fn active_adapter(&self) -> Option<&str> {
        self.active_adapter.as_deref()
    }

    /// Approximate resident size in bytes (the base safetensors file size).
    /// Used by the dispatch layer's weight-bounded model cache to bound RAM.
    pub fn approx_bytes(&self) -> u64 {
        self.approx_bytes
    }

    /// Load from in-memory model bytes (browser/OPFS, wasm — no filesystem).
    /// `encoder_config_json` is the `config.json` (or `encoder_config/config.json`)
    /// contents; `tokenizer_json` is the HF `tokenizer.json` contents;
    /// `safetensors` is the `model.safetensors` contents.
    pub fn from_bytes(safetensors: &[u8], tokenizer_json: &[u8], encoder_config_json: &[u8]) -> Result<Self> {
        let device = Device::Cpu;
        let tokenizer = xberg_gliner::V2Tokenizer::from_bytes(tokenizer_json)?;
        let splitter = xberg_gliner::V2Splitter::new()?;
        let config: candle_transformers::models::debertav2::Config = serde_json::from_slice(encoder_config_json)
            .map_err(|e| GlinerCandleError::Backend(format!("encoder config parse: {e}")))?;
        // wasm32: F16, halving resident memory after loading -- confirmed via
        // a live browser reproduction that F32 (previously hardcoded here)
        // exhausts wasm32 linear memory for a DeBERTa-v2-scale encoder, even
        // with the streaming per-tensor loader (see streaming_load.rs) that
        // eliminates the OTHER, larger source of peak memory (materializing
        // every tensor in the file as F32 before any conversion). Native
        // targets keep F32 -- plenty of RAM, and F32 avoids any precision
        // risk for callers relying on `load_adapter`'s LoRA merge (which
        // stays F32-only, non-wasm32-gated, see model.rs's other impl block).
        #[cfg(target_arch = "wasm32")]
        let dtype = candle_core::DType::F16;
        #[cfg(not(target_arch = "wasm32"))]
        let dtype = candle_core::DType::F32;
        let encoder = encoder::Encoder::from_buffered_safetensors(safetensors, &config, &device, dtype)?;
        let heads_loaded = heads::AllHeads::from_buffered_safetensors(safetensors, &device, dtype)?;

        Ok(Self {
            tokenizer,
            splitter,
            device,
            base_model_dir: PathBuf::new(),
            encoder,
            heads: heads_loaded,
            active_adapter: None,
            model_id: "gliner2_candle_bytes".to_string(),
            approx_bytes: safetensors.len() as u64,
        })
    }

    /// Extract entities for the given zero-shot `labels`.
    pub fn extract_ner(&self, text: &str, labels: &[&str], threshold: f32) -> Result<Vec<xberg_gliner::Span>> {
        if labels.is_empty() {
            return Ok(vec![]);
        }
        let owned_labels: Vec<String> = labels.iter().map(|s| s.to_string()).collect();
        let (scorer_out, pred_count, encoded) = pipeline::run_pipeline(
            &self.tokenizer,
            &self.splitter,
            &self.device,
            &self.encoder,
            &self.heads,
            text,
            &owned_labels,
        )?;
        if pred_count == 0 {
            return Ok(vec![]);
        }
        let output = decode::decode_span_scores(
            text,
            &encoded.words,
            &owned_labels,
            &scorer_out,
            pred_count,
            threshold,
            /* flat_ner = */ true,
            /* dup_label = */ false,
            /* multi_label = */ false,
        )?;
        Ok(output.spans.into_iter().next().unwrap_or_default())
    }
}

/// Filesystem-only API: base model loading from a local directory and PEFT
/// LoRA adapter merge-at-load. Not available on wasm32 (no filesystem).
#[cfg(not(target_arch = "wasm32"))]
impl Gliner2Candle {
    /// Load a PEFT-format LoRA adapter and merge it into the base weights.
    pub fn load_adapter(&mut self, name: &str, adapter_dir: &Path) -> Result<()> {
        let adapter = lora::LoraAdapter::load(adapter_dir, &self.device)?;

        if let Some(adapter_base) = adapter.config.base_model_name_or_path.as_deref()
            && !self.model_id.contains(adapter_base)
            && !adapter_base.contains(&self.model_id)
        {
            return Err(GlinerCandleError::Backend(format!(
                "load_adapter: adapter trained on '{adapter_base}', current model is \
                 '{}'. Refusing to merge — remove base_model_name_or_path from \
                 adapter_config.json to bypass.",
                self.model_id
            )));
        }

        let base_safetensors = self.base_model_dir.join("model.safetensors");
        let merged = lora::merge_into_base(&base_safetensors, &adapter, &self.device)?;

        let vb = candle_nn::VarBuilder::from_tensors(merged, candle_core::DType::F32, &self.device);
        let new_encoder = encoder::Encoder::from_var_builder(vb.pp("encoder"), &self.encoder.config)?;
        let new_heads = heads::AllHeads::from_var_builder(vb, &self.device)?;

        self.encoder = new_encoder;
        self.heads = new_heads;
        self.active_adapter = Some(name.to_string());
        Ok(())
    }

    /// Discard the active adapter and reload pure base weights from `base_model_dir`. Idempotent.
    pub fn unload_adapter(&mut self) -> Result<()> {
        if self.active_adapter.is_none() {
            return Ok(());
        }
        let weights_path = self.base_model_dir.join("model.safetensors");
        let config_path = resolve_encoder_config_path(&self.base_model_dir);
        self.encoder = encoder::Encoder::from_safetensors(&weights_path, &config_path, &self.device)?;
        self.heads = heads::AllHeads::from_safetensors(&weights_path, &self.device)?;
        self.active_adapter = None;
        Ok(())
    }

    /// Load from a local directory containing `tokenizer.json`, `config.json`
    /// (or `encoder_config/config.json`), and `model.safetensors`. CPU device.
    pub fn from_local(model_dir: &Path) -> Result<Self> {
        Self::from_local_with_device(model_dir, &Device::Cpu)
    }

    /// Load from a local directory with an explicit Candle device.
    pub fn from_local_with_device(model_dir: &Path, device: &Device) -> Result<Self> {
        let tokenizer_path = model_dir.join("tokenizer.json");
        let weights_path = model_dir.join("model.safetensors");
        let config_path = resolve_encoder_config_path(model_dir);

        if !weights_path.exists() {
            return Err(GlinerCandleError::Backend(format!(
                "model.safetensors not found in {} (PyTorch fastino/gliner2-* repo \
                 expected; an ONNX export is a different artifact)",
                model_dir.display()
            )));
        }

        let tokenizer = xberg_gliner::V2Tokenizer::from_file(&tokenizer_path)?;
        let splitter = xberg_gliner::V2Splitter::new()?;
        let encoder = encoder::Encoder::from_safetensors(&weights_path, &config_path, device)?;
        let heads_loaded = heads::AllHeads::from_safetensors(&weights_path, device)?;
        let model_id = model_dir
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "gliner2_candle_local".to_string());
        let approx_bytes = std::fs::metadata(&weights_path).map(|m| m.len()).unwrap_or(0);

        Ok(Self {
            tokenizer,
            splitter,
            device: device.clone(),
            base_model_dir: model_dir.to_path_buf(),
            encoder,
            heads: heads_loaded,
            active_adapter: None,
            model_id,
            approx_bytes,
        })
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn resolve_encoder_config_path(model_dir: &Path) -> PathBuf {
    let nested = model_dir.join("encoder_config").join("config.json");
    if nested.exists() {
        nested
    } else {
        model_dir.join("config.json")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn from_local_with_device_rejects_missing_weights() {
        let dir = tempfile::tempdir().expect("tempdir");
        let err = Gliner2Candle::from_local_with_device(dir.path(), &Device::Cpu).expect_err("empty dir must fail");
        assert!(err.to_string().contains("model.safetensors"));
    }

    #[test]
    fn from_bytes_rejects_empty_safetensors() {
        let err = Gliner2Candle::from_bytes(&[], b"{}", b"{}").expect_err("empty weights must fail");
        assert!(
            err.to_string().to_lowercase().contains("safetensors") || err.to_string().to_lowercase().contains("load")
        );
    }
}
