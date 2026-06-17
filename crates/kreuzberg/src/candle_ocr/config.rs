use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use kreuzberg_candle_ocr::DevicePreference;

/// Identifier for which candle model is selected by [`CandleOcrConfig`].
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CandleModelId {
    #[default]
    Trocr,
    PaddleocrVl,
}

/// Configuration passed to candle OCR backends through `OcrConfig::backend_options`.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct CandleOcrConfig {
    pub model: CandleModelId,
    pub device: DevicePreference,
    pub cache_dir: Option<PathBuf>,
    pub hf_revision: Option<String>,
    pub max_new_tokens: u32,
    pub temperature: f32,
}

impl Default for CandleOcrConfig {
    fn default() -> Self {
        Self {
            model: CandleModelId::default(),
            device: DevicePreference::Auto,
            cache_dir: None,
            hf_revision: None,
            max_new_tokens: 4096,
            temperature: 0.0,
        }
    }
}
