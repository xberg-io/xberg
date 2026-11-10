//! Extraction-config construction: the timeout heuristics and per-pipeline
//! `xberg::ExtractionConfig` builders behind [`build_extraction_config`], plus the fixture-language
//! and timed-OCR-result-cache adjustments applied to the config after it is built.

use super::pipeline::Pipeline;
use crate::corpus::CorpusDocument;
use std::path::Path;
use xberg::core::config::layout::LayoutDetectionConfig;

/// Base timeout for extraction in seconds.
///
/// This is the minimum timeout applied to all documents, regardless of size.
pub(super) const EXTRACTION_TIMEOUT_BASE_SECS: u64 = 60;

/// Minimum timeout for forced OCR pipelines, whose inference cost is substantially higher.
/// Five minutes leaves CI variance headroom while the outer safety timeout still bounds hangs. ~keep
pub(super) const FORCED_OCR_TIMEOUT_BASE_SECS: u64 = 300;

/// Per-page timeout for PDF documents in milliseconds.
///
/// For layout-heavy PDFs (which require layout detection with ONNX inference),
/// allocate additional time proportional to page count. This prevents timeouts
/// on legitimately large documents like 214-page standards sheets.
///
/// Example: 214-page PDF = 60s base + (214 * 400ms) = 60s + 85.6s = 145.6s total
pub(super) const EXTRACTION_TIMEOUT_PER_PAGE_MS: u64 = 400;

pub(super) const PP_OCR_V5: &str = "pp-ocrv5";
pub(super) const PP_OCR_V6: &str = "pp-ocrv6";
/// Sceptre's ONNX Runtime inference engine selector, passed via `OcrConfig::backend_options` as
/// `{"model":{"backend":"ort"}}` — mirrors `SCEPTRE_ORT_OPTIONS_JSON` in `adapters/xberg.rs`,
/// which drives the same selection for the separate `XbergPipeline` (CLI-subprocess) enum. ~keep
pub(super) const SCEPTRE_MODEL_BACKEND_ORT: &str = "ort";
pub(super) const PADDLE_DET_SIDE_1024: u32 = 1024;
pub(super) const PADDLE_DET_SIDE_1536: u32 = 1536;
pub(super) const PADDLE_DET_SIDE_2048: u32 = 2048;
pub(super) const PADDLE_DET_DB_THRESH_020: f32 = 0.20;
pub(super) const PADDLE_DET_DB_THRESH_030: f32 = 0.30;
pub(super) const PADDLE_DET_DB_BOX_THRESH_035: f32 = 0.35;
pub(super) const PADDLE_DET_DB_BOX_THRESH_050: f32 = 0.50;
pub(super) const PADDLE_DROP_SCORE_030: f32 = 0.30;
pub(super) const PADDLE_DROP_SCORE_040: f32 = 0.40;
pub(super) const PADDLE_DROP_SCORE_050: f32 = 0.50;
pub(super) const TESSERACT_PSM_VERTICAL_BLOCK: i32 = 5;
pub(super) const TESSERACT_PSM_SINGLE_BLOCK: i32 = 6;
pub(super) const TESSERACT_PSM_SPARSE_TEXT: i32 = 11;

/// Estimate PDF page count from file size (~12KB/page heuristic).
///
/// Fallback for when the PDF cannot be opened for a real page count.
fn estimate_pdf_page_count_from_size(file_size: u64) -> u64 {
    (file_size / 12_000).max(1)
}

/// Compute extraction timeout scaled by document page count.
///
/// Returns timeout in seconds. For PDFs, applies per-page scaling to accommodate
/// layout detection inference on large documents. Uses the real page count when
/// the document parses; falls back to a size-based estimate otherwise.
pub(super) fn compute_extraction_timeout_secs(path: &Path, file_type: &str, force_ocr: bool) -> u64 {
    let base_timeout_secs = if force_ocr {
        FORCED_OCR_TIMEOUT_BASE_SECS
    } else {
        EXTRACTION_TIMEOUT_BASE_SECS
    };
    if file_type.to_lowercase() != "pdf" {
        return base_timeout_secs;
    }

    let page_count = std::fs::read(path)
        .ok()
        .and_then(|bytes| xberg::pdf_page_count(&bytes, None).ok())
        .map(|count| count as u64)
        .or_else(|| {
            std::fs::metadata(path)
                .ok()
                .map(|m| estimate_pdf_page_count_from_size(m.len()))
        });

    let Some(page_count) = page_count else {
        tracing::warn!("Could not size PDF file, using base timeout: {}", path.display());
        return base_timeout_secs;
    };

    let per_page_secs = EXTRACTION_TIMEOUT_PER_PAGE_MS as f64 / 1000.0;
    let scaled_timeout = base_timeout_secs as f64 + (page_count as f64 * per_page_secs);

    scaled_timeout.ceil() as u64
}

fn build_paddle_extraction_config(
    model_version: &str,
    model_tier: &str,
    layout: Option<xberg::core::config::layout::LayoutDetectionConfig>,
) -> xberg::ExtractionConfig {
    build_paddle_extraction_config_with_quality_profile(model_version, model_tier, layout, None)
}

#[derive(Debug, Clone, Copy)]
pub(super) struct PaddleQualityProfile {
    pub(super) det_limit_side_len: u32,
    pub(super) det_db_thresh: f32,
    pub(super) det_db_box_thresh: f32,
    pub(super) drop_score: f32,
}

pub(super) const PADDLE_DEFAULT_QUALITY_PROFILE: PaddleQualityProfile = PaddleQualityProfile {
    det_limit_side_len: PADDLE_DET_SIDE_1024,
    det_db_thresh: PADDLE_DET_DB_THRESH_030,
    det_db_box_thresh: PADDLE_DET_DB_BOX_THRESH_050,
    drop_score: PADDLE_DROP_SCORE_050,
};
pub(super) const PADDLE_DET_SIDE_1536_QUALITY_PROFILE: PaddleQualityProfile = PaddleQualityProfile {
    det_limit_side_len: PADDLE_DET_SIDE_1536,
    ..PADDLE_DEFAULT_QUALITY_PROFILE
};
pub(super) const PADDLE_DET_SIDE_2048_QUALITY_PROFILE: PaddleQualityProfile = PaddleQualityProfile {
    det_limit_side_len: PADDLE_DET_SIDE_2048,
    ..PADDLE_DEFAULT_QUALITY_PROFILE
};
pub(super) const PADDLE_DET_DB_THRESH_020_QUALITY_PROFILE: PaddleQualityProfile = PaddleQualityProfile {
    det_db_thresh: PADDLE_DET_DB_THRESH_020,
    ..PADDLE_DEFAULT_QUALITY_PROFILE
};
pub(super) const PADDLE_DET_DB_BOX_THRESH_035_QUALITY_PROFILE: PaddleQualityProfile = PaddleQualityProfile {
    det_db_box_thresh: PADDLE_DET_DB_BOX_THRESH_035,
    ..PADDLE_DEFAULT_QUALITY_PROFILE
};
pub(super) const PADDLE_DROP_SCORE_030_QUALITY_PROFILE: PaddleQualityProfile = PaddleQualityProfile {
    drop_score: PADDLE_DROP_SCORE_030,
    ..PADDLE_DEFAULT_QUALITY_PROFILE
};
pub(super) const PADDLE_DROP_SCORE_040_QUALITY_PROFILE: PaddleQualityProfile = PaddleQualityProfile {
    drop_score: PADDLE_DROP_SCORE_040,
    ..PADDLE_DEFAULT_QUALITY_PROFILE
};

fn build_paddle_extraction_config_with_quality_profile(
    model_version: &str,
    model_tier: &str,
    layout: Option<xberg::core::config::layout::LayoutDetectionConfig>,
    quality_profile: Option<PaddleQualityProfile>,
) -> xberg::ExtractionConfig {
    let mut paddle_ocr_settings = xberg::PaddleOcrConfig {
        model_version: model_version.to_string(),
        model_tier: model_tier.to_string(),
        ..Default::default()
    };
    if let Some(profile) = quality_profile {
        paddle_ocr_settings.det_limit_side_len = profile.det_limit_side_len;
        paddle_ocr_settings.det_db_thresh = profile.det_db_thresh;
        paddle_ocr_settings.det_db_box_thresh = profile.det_db_box_thresh;
        paddle_ocr_settings.drop_score = profile.drop_score;
    }

    xberg::ExtractionConfig {
        output_format: xberg::core::config::OutputFormat::Markdown,
        force_ocr: true,
        ocr: Some(xberg::core::config::OcrConfig {
            backend: "paddleocr".to_string(),
            language: vec!["eng".to_string()],
            auto_rotate: None,
            paddle_ocr_settings: Some(paddle_ocr_settings),
            ..Default::default()
        }),
        layout,
        ..Default::default()
    }
}

fn build_paddle_v6_small_layout_quality_config(profile: PaddleQualityProfile) -> xberg::ExtractionConfig {
    build_paddle_extraction_config_with_quality_profile(
        PP_OCR_V6,
        "small",
        Some(xberg::core::config::layout::LayoutDetectionConfig::default()),
        Some(profile),
    )
}

/// Build a Sceptre extraction config pinned to the ONNX Runtime inference engine.
///
/// Mirrors `push_sceptre_args`/`SCEPTRE_ORT_OPTIONS_JSON` in `adapters/xberg.rs`: that CLI-subprocess
/// adapter selects the ONNX Runtime engine by passing `--ocr-backend-options
/// {"model":{"backend":"ort"}}`; this in-process config forces the same selection through
/// `OcrConfig::backend_options`, which `crate::extract_xberg_file` reads unchanged. ~keep
fn build_sceptre_extraction_config(
    layout: Option<xberg::core::config::layout::LayoutDetectionConfig>,
) -> xberg::ExtractionConfig {
    xberg::ExtractionConfig {
        output_format: xberg::core::config::OutputFormat::Markdown,
        force_ocr: true,
        ocr: Some(xberg::core::config::OcrConfig {
            backend: "sceptre".to_string(),
            language: vec!["eng".to_string()],
            backend_options: Some(serde_json::json!({
                "model": { "backend": SCEPTRE_MODEL_BACKEND_ORT }
            })),
            ..Default::default()
        }),
        layout,
        ..Default::default()
    }
}

fn build_tesseract_extraction_config(psm: i32) -> xberg::ExtractionConfig {
    xberg::ExtractionConfig {
        output_format: xberg::core::config::OutputFormat::Markdown,
        force_ocr: true,
        ocr: Some(xberg::core::config::OcrConfig {
            backend: "tesseract".to_string(),
            language: vec!["eng".to_string()],
            tesseract_config: Some(xberg::TesseractConfig {
                language: vec!["eng".to_string()],
                psm: Some(psm),
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn disable_timed_extraction_caches(config: &mut xberg::ExtractionConfig) {
    config.use_cache = false;

    // Only flip `use_cache` on a `tesseract_config` that already exists — an explicit PSM preset
    // (`build_tesseract_extraction_config`). Never materialize one here: this runs before any
    // fixture-specific language is known (`build_extraction_config`'s own tail), so a pipeline
    // that leaves `tesseract_config` implicit must wait for `finalize_timed_ocr_result_cache`,
    // which runs after `apply_fixture_ocr_language` and can pick the PSM that matches the real
    // (final) language instead of a placeholder. ~keep
    flip_existing_tesseract_result_caches(config);
}

fn flip_existing_tesseract_result_caches(config: &mut xberg::ExtractionConfig) {
    let Some(ocr) = config.ocr.as_mut() else {
        return;
    };
    if let Some(pipeline) = ocr.pipeline.as_mut() {
        for stage in &mut pipeline.stages {
            if stage.backend == "tesseract"
                && let Some(tesseract_config) = stage.tesseract_config.as_mut()
            {
                tesseract_config.use_cache = false;
            }
        }
    } else if ocr.backend == "tesseract"
        && let Some(tesseract_config) = ocr.tesseract_config.as_mut()
    {
        tesseract_config.use_cache = false;
    }
}

/// Materializes xberg's own OCR fallback default (`OcrConfig::default()`, backend `"tesseract"`)
/// when a pipeline left `config.ocr` entirely `None` (e.g. `Pipeline::Baseline`/`Native`'s
/// native-with-OCR-fallback pipelines). Gives a fixture's OCR language somewhere to attach via
/// `apply_fixture_ocr_language`, without forcing `force_ocr`. Must run *before* that call. ~keep
pub(super) fn materialize_implicit_ocr_config(config: &mut xberg::ExtractionConfig) {
    if config.ocr.is_none() {
        config.ocr = Some(xberg::OcrConfig::default());
    }
}

/// Finishes disabling the timed Tesseract OCR result cache once the fixture's language (if any)
/// has already been applied via `apply_fixture_ocr_language`. Must run *after* that call, or it
/// would compute PSM from the wrong (pre-fixture-language) language.
///
/// Flips `use_cache` on a `tesseract_config` that already exists (an explicit PSM preset) without
/// touching its `psm`. For a pipeline that leaves `tesseract_config` implicit (auto-PSM),
/// materializes one with `use_cache = false` and the PSM
/// `apply_default_whole_image_tesseract_psm` in `crates/xberg/src/extractors/image.rs` would
/// itself have picked for the config's current (now-final) language — so the result cache is
/// genuinely disabled without silently regressing to `TesseractConfig::default()`'s PSM 3. ~keep
pub(super) fn finalize_timed_ocr_result_cache(config: &mut xberg::ExtractionConfig) {
    let Some(ocr) = config.ocr.as_mut() else {
        return;
    };
    let parent_language = ocr.language.clone();
    if let Some(pipeline) = ocr.pipeline.as_mut() {
        for stage in &mut pipeline.stages {
            if stage.backend != "tesseract" {
                continue;
            }
            // A present `stage.language` wins over the parent's, matching xberg's own stage
            // semantics ("None = use parent OcrConfig.language" — `OcrPipelineStage::language` in
            // `crates/xberg/src/core/config/ocr.rs`). This is only correct because
            // `apply_fixture_ocr_language` refreshes `stage.language` too, not just the parent's
            // — otherwise a stale per-stage override would compute PSM from the wrong language
            // here. ~keep
            let languages = stage.language.clone().unwrap_or_else(|| parent_language.clone());
            materialize_or_disable_tesseract_result_cache(&mut stage.tesseract_config, &languages);
        }
    } else if ocr.backend == "tesseract" {
        materialize_or_disable_tesseract_result_cache(&mut ocr.tesseract_config, &parent_language);
    }
}

fn materialize_or_disable_tesseract_result_cache(
    tesseract_config: &mut Option<xberg::TesseractConfig>,
    languages: &[String],
) {
    match tesseract_config.as_mut() {
        Some(existing) => existing.use_cache = false,
        None => {
            *tesseract_config = Some(xberg::TesseractConfig {
                language: languages.to_vec(),
                psm: Some(crate::adapter::xberg_default_tesseract_psm(languages)),
                use_cache: false,
                ..Default::default()
            });
        }
    }
}

/// Build the base config shared by every pipeline before its own overrides are applied.
fn base_extraction_config() -> xberg::ExtractionConfig {
    xberg::ExtractionConfig {
        output_format: xberg::core::config::OutputFormat::Markdown,
        ..Default::default()
    }
}

/// `Pipeline::Tesseract` / `TesseractLayout` / `TesseractAutoRotate`: force_ocr'd Tesseract,
/// differing only in whether layout detection and auto-rotate are enabled.
fn tesseract_inline_config(
    base: xberg::ExtractionConfig,
    layout: Option<xberg::core::config::layout::LayoutDetectionConfig>,
    auto_rotate: bool,
) -> xberg::ExtractionConfig {
    xberg::ExtractionConfig {
        force_ocr: true,
        ocr: Some(xberg::core::config::OcrConfig {
            backend: "tesseract".to_string(),
            language: vec!["eng".to_string()],
            auto_rotate: auto_rotate.then_some(true),
            ..Default::default()
        }),
        layout,
        ..base
    }
}

/// Every `Pipeline::Paddle*` variant: model version/tier, an optional layout config, an optional
/// pinned quality-control profile, or an auto-rotate override — all built through
/// `build_paddle_extraction_config`/`build_paddle_v6_small_layout_quality_config`.
fn paddle_pipeline_config(pipeline: Pipeline) -> xberg::ExtractionConfig {
    match pipeline {
        Pipeline::Paddle => build_paddle_extraction_config(PP_OCR_V6, "medium", None),
        Pipeline::PaddleLayout => {
            build_paddle_extraction_config(PP_OCR_V6, "medium", Some(LayoutDetectionConfig::default()))
        }
        Pipeline::PaddleV6Small => build_paddle_extraction_config(PP_OCR_V6, "small", None),
        Pipeline::PaddleV6SmallLayout => {
            build_paddle_extraction_config(PP_OCR_V6, "small", Some(LayoutDetectionConfig::default()))
        }
        Pipeline::PaddleV6SmallLayoutDetSide1024 => {
            build_paddle_v6_small_layout_quality_config(PADDLE_DEFAULT_QUALITY_PROFILE)
        }
        Pipeline::PaddleV6SmallLayoutDetSide1536 => {
            build_paddle_v6_small_layout_quality_config(PADDLE_DET_SIDE_1536_QUALITY_PROFILE)
        }
        Pipeline::PaddleV6SmallLayoutDetSide2048 => {
            build_paddle_v6_small_layout_quality_config(PADDLE_DET_SIDE_2048_QUALITY_PROFILE)
        }
        Pipeline::PaddleV6SmallLayoutDetDbThresh020 => {
            build_paddle_v6_small_layout_quality_config(PADDLE_DET_DB_THRESH_020_QUALITY_PROFILE)
        }
        Pipeline::PaddleV6SmallLayoutDetDbBoxThresh035 => {
            build_paddle_v6_small_layout_quality_config(PADDLE_DET_DB_BOX_THRESH_035_QUALITY_PROFILE)
        }
        Pipeline::PaddleV6SmallLayoutDropScore030 => {
            build_paddle_v6_small_layout_quality_config(PADDLE_DROP_SCORE_030_QUALITY_PROFILE)
        }
        Pipeline::PaddleV6SmallLayoutDropScore040 => {
            build_paddle_v6_small_layout_quality_config(PADDLE_DROP_SCORE_040_QUALITY_PROFILE)
        }
        Pipeline::PaddleV6Tiny => build_paddle_extraction_config(PP_OCR_V6, "tiny", None),
        Pipeline::PaddleV6TinyLayout => {
            build_paddle_extraction_config(PP_OCR_V6, "tiny", Some(LayoutDetectionConfig::default()))
        }
        Pipeline::PaddleServer => build_paddle_extraction_config(PP_OCR_V5, "server", None),
        Pipeline::PaddleServerLayout => {
            build_paddle_extraction_config(PP_OCR_V5, "server", Some(LayoutDetectionConfig::default()))
        }
        Pipeline::PaddleAutoRotate => {
            let mut config = build_paddle_extraction_config(PP_OCR_V6, "medium", None);
            config
                .ocr
                .as_mut()
                .expect("Paddle benchmark config must include OCR")
                .auto_rotate = Some(true);
            config
        }
        Pipeline::PaddleNoRotate => build_paddle_extraction_config(PP_OCR_V6, "medium", None),
        _ => unreachable!("paddle_pipeline_config called with non-paddle-family pipeline: {pipeline:?}"),
    }
}

/// Every `Pipeline::Sceptre*` variant.
fn sceptre_pipeline_config(pipeline: Pipeline) -> xberg::ExtractionConfig {
    match pipeline {
        Pipeline::Sceptre => build_sceptre_extraction_config(None),
        Pipeline::SceptreLayout => build_sceptre_extraction_config(Some(LayoutDetectionConfig::default())),
        Pipeline::SceptreAutoRotate => {
            let mut config = build_sceptre_extraction_config(None);
            config
                .ocr
                .as_mut()
                .expect("Sceptre benchmark config must include OCR")
                .auto_rotate = Some(true);
            config
        }
        _ => unreachable!("sceptre_pipeline_config called with non-sceptre-family pipeline: {pipeline:?}"),
    }
}

/// Every `Pipeline::LayoutSlanet*` variant: native PDF + layout detection with a specific
/// SLANeXT/SLANet table model, differing only in which model is selected.
fn layout_slanet_pipeline_config(pipeline: Pipeline, base: xberg::ExtractionConfig) -> xberg::ExtractionConfig {
    let table_model = match pipeline {
        Pipeline::LayoutSlanetAuto => xberg::core::config::layout::TableModel::SlanetAuto,
        Pipeline::LayoutSlanetWired => xberg::core::config::layout::TableModel::SlanetWired,
        Pipeline::LayoutSlanetWireless => xberg::core::config::layout::TableModel::SlanetWireless,
        Pipeline::LayoutSlanetPlus => xberg::core::config::layout::TableModel::SlanetPlus,
        _ => unreachable!("layout_slanet_pipeline_config called with non-slanet-family pipeline: {pipeline:?}"),
    };
    xberg::ExtractionConfig {
        layout: Some(LayoutDetectionConfig {
            table_model,
            ..Default::default()
        }),
        ocr: Some(xberg::core::config::OcrConfig {
            backend: "tesseract".to_string(),
            language: vec!["eng".to_string()],
            ..Default::default()
        }),
        ..base
    }
}

/// Every `Pipeline::Native*` variant: xberg-native-pdf backend text extraction, optionally with
/// layout detection and/or reading-order reordering.
fn native_pipeline_config(pipeline: Pipeline, base: xberg::ExtractionConfig) -> xberg::ExtractionConfig {
    match pipeline {
        Pipeline::Native => xberg::ExtractionConfig {
            pdf_options: Some(xberg::PdfConfig { ..Default::default() }),
            ..base
        },
        Pipeline::NativeLayout => xberg::ExtractionConfig {
            pdf_options: Some(xberg::PdfConfig { ..Default::default() }),
            layout: Some(LayoutDetectionConfig::default()),
            use_layout_for_markdown: true,
            ocr: Some(xberg::core::config::OcrConfig {
                backend: "tesseract".to_string(),
                language: vec!["eng".to_string()],
                ..Default::default()
            }),
            ..base
        },
        Pipeline::NativeReadingOrder => xberg::ExtractionConfig {
            pdf_options: Some(xberg::PdfConfig {
                reading_order: true,
                ..Default::default()
            }),
            layout: Some(LayoutDetectionConfig::default()),
            use_layout_for_markdown: true,
            ocr: Some(xberg::core::config::OcrConfig {
                backend: "tesseract".to_string(),
                language: vec!["eng".to_string()],
                ..Default::default()
            }),
            ..base
        },
        _ => unreachable!("native_pipeline_config called with non-native-family pipeline: {pipeline:?}"),
    }
}

/// Every Candle-backed vision-language `Pipeline` variant.
fn candle_pipeline_config(pipeline: Pipeline, base: xberg::ExtractionConfig) -> xberg::ExtractionConfig {
    match pipeline {
        Pipeline::CandleTrocr => xberg::ExtractionConfig {
            force_ocr: true,
            ocr: Some(xberg::core::config::OcrConfig {
                backend: "candle-trocr".to_string(),
                language: vec!["eng".to_string()],
                ..Default::default()
            }),
            ..base
        },
        Pipeline::CandlePaddleocrVl => xberg::ExtractionConfig {
            force_ocr: true,
            ocr: Some(xberg::core::config::OcrConfig {
                backend: "candle-paddleocr-vl".to_string(),
                language: vec!["eng".to_string()],
                ..Default::default()
            }),
            ..base
        },
        Pipeline::CandleGlmOcr => xberg::ExtractionConfig {
            force_ocr: true,
            ocr: Some(xberg::core::config::OcrConfig {
                backend: "candle-glm-ocr".to_string(),
                language: vec!["en".to_string()],
                ..Default::default()
            }),
            ..base
        },
        Pipeline::CandleGlmOcrLayout => xberg::ExtractionConfig {
            force_ocr: true,
            ocr: Some(xberg::core::config::OcrConfig {
                backend: "candle-glm-ocr".to_string(),
                language: vec!["en".to_string()],
                ..Default::default()
            }),
            layout: Some(LayoutDetectionConfig::default()),
            ..base
        },
        Pipeline::CandleGlmOcrLayoutChart => xberg::ExtractionConfig {
            force_ocr: true,
            ocr: Some(xberg::core::config::OcrConfig {
                backend: "candle-glm-ocr".to_string(),
                language: vec!["en".to_string()],
                ..Default::default()
            }),
            layout: Some(LayoutDetectionConfig {
                enable_chart_understanding: true,
                ..Default::default()
            }),
            ..base
        },
        Pipeline::CandleDeepseekOcr => xberg::ExtractionConfig {
            force_ocr: true,
            ocr: Some(xberg::core::config::OcrConfig {
                backend: "candle-deepseek-ocr".to_string(),
                language: vec!["en".to_string()],
                ..Default::default()
            }),
            ..base
        },
        Pipeline::CandlePaddleocrVl15 => xberg::ExtractionConfig {
            force_ocr: true,
            ocr: Some(xberg::core::config::OcrConfig {
                backend: "candle-paddleocr-vl".to_string(),
                language: vec!["en".to_string()],
                ..Default::default()
            }),
            ..base
        },
        _ => unreachable!("candle_pipeline_config called with non-candle-family pipeline: {pipeline:?}"),
    }
}

/// Build a xberg ExtractionConfig for the given pipeline.
pub fn build_extraction_config(pipeline: Pipeline) -> xberg::ExtractionConfig {
    let base = base_extraction_config();

    let mut config = match pipeline {
        Pipeline::Baseline | Pipeline::Docling | Pipeline::PaddleOcrPython | Pipeline::RapidOcr => base,
        Pipeline::Layout => xberg::ExtractionConfig {
            layout: Some(LayoutDetectionConfig::default()),
            ..base
        },
        Pipeline::TesseractSingleBlock => build_tesseract_extraction_config(TESSERACT_PSM_SINGLE_BLOCK),
        Pipeline::TesseractVerticalBlock => build_tesseract_extraction_config(TESSERACT_PSM_VERTICAL_BLOCK),
        Pipeline::TesseractSparseText => build_tesseract_extraction_config(TESSERACT_PSM_SPARSE_TEXT),
        Pipeline::Tesseract => tesseract_inline_config(base, None, false),
        Pipeline::TesseractLayout => tesseract_inline_config(base, Some(LayoutDetectionConfig::default()), false),
        Pipeline::TesseractAutoRotate => tesseract_inline_config(base, None, true),
        Pipeline::Paddle
        | Pipeline::PaddleLayout
        | Pipeline::PaddleV6Small
        | Pipeline::PaddleV6SmallLayout
        | Pipeline::PaddleV6SmallLayoutDetSide1024
        | Pipeline::PaddleV6SmallLayoutDetSide1536
        | Pipeline::PaddleV6SmallLayoutDetSide2048
        | Pipeline::PaddleV6SmallLayoutDetDbThresh020
        | Pipeline::PaddleV6SmallLayoutDetDbBoxThresh035
        | Pipeline::PaddleV6SmallLayoutDropScore030
        | Pipeline::PaddleV6SmallLayoutDropScore040
        | Pipeline::PaddleV6Tiny
        | Pipeline::PaddleV6TinyLayout
        | Pipeline::PaddleServer
        | Pipeline::PaddleServerLayout
        | Pipeline::PaddleAutoRotate
        | Pipeline::PaddleNoRotate => paddle_pipeline_config(pipeline),
        Pipeline::Sceptre | Pipeline::SceptreLayout | Pipeline::SceptreAutoRotate => sceptre_pipeline_config(pipeline),
        Pipeline::LayoutSlanetAuto
        | Pipeline::LayoutSlanetWired
        | Pipeline::LayoutSlanetWireless
        | Pipeline::LayoutSlanetPlus => layout_slanet_pipeline_config(pipeline, base),
        Pipeline::Native | Pipeline::NativeLayout | Pipeline::NativeReadingOrder => {
            native_pipeline_config(pipeline, base)
        }
        Pipeline::CandleTrocr
        | Pipeline::CandlePaddleocrVl
        | Pipeline::CandleGlmOcr
        | Pipeline::CandleGlmOcrLayout
        | Pipeline::CandleGlmOcrLayoutChart
        | Pipeline::CandleDeepseekOcr
        | Pipeline::CandlePaddleocrVl15 => candle_pipeline_config(pipeline, base),
    };

    if config.layout.is_some() {
        config.use_layout_for_markdown = true;
    }
    disable_timed_extraction_caches(&mut config);
    config
}

pub(super) fn apply_fixture_ocr_language(config: &mut xberg::ExtractionConfig, doc: &CorpusDocument) {
    let Some(language) = doc.metadata.get("ocr_language").and_then(serde_json::Value::as_str) else {
        return;
    };
    let languages = crate::adapter::canonicalize_ocr_languages(language);
    if languages.is_empty() {
        return;
    }
    if let Some(ocr) = config.ocr.as_mut() {
        ocr.language = languages.clone();
        if let Some(pipeline) = ocr.pipeline.as_mut() {
            for stage in &mut pipeline.stages {
                if stage.backend != "tesseract" {
                    continue;
                }
                // A stage's own `language` override (`None` = inherit the parent
                // `OcrConfig.language`) must also be refreshed to the fixture's language, not
                // just the parent's — otherwise a stage that already pins its own language (e.g.
                // `Some(["eng"])`) stays stale at real extraction time (xberg prefers a present
                // stage override over the parent), AND `finalize_timed_ocr_result_cache` would
                // compute PSM from that stale language instead of the fixture's real one. ~keep
                stage.language = Some(languages.clone());
                if let Some(tesseract) = stage.tesseract_config.as_mut() {
                    tesseract.language = languages.clone();
                }
            }
        } else if ocr.backend == "tesseract"
            && let Some(tesseract) = ocr.tesseract_config.as_mut()
        {
            // The timed benchmark materializes this nested config to disable its cache. ~keep
            tesseract.language = languages;
        }
    }
}
