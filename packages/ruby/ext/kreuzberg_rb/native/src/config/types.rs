//! Configuration parsing and conversion for Ruby bindings
//!
//! Handles conversion between Ruby Hash configurations and Rust config types.
//! Includes parsing for all nested configuration structures.

use crate::error_handling::{runtime_error, validation_error};
use crate::helpers::{get_kw, json_value_to_ruby, ruby_value_to_json, symbol_to_string};

use html_to_markdown_rs::options::{
    CodeBlockStyle, ConversionOptions, HeadingStyle, HighlightStyle, ListIndentType, NewlineStyle,
    PreprocessingPreset,
};
use html_to_markdown_rs::WhitespaceMode;
use kreuzberg::core::config::PageConfig;
use kreuzberg::keywords::{
    KeywordAlgorithm as RustKeywordAlgorithm, KeywordConfig as RustKeywordConfig, RakeParams as RustRakeParams,
    YakeParams as RustYakeParams,
};
use kreuzberg::types::TesseractConfig as RustTesseractConfig;
use kreuzberg::pdf::HierarchyConfig;
use kreuzberg::{
    ChunkingConfig, EmbeddingConfig, ExtractionConfig, ImageExtractionConfig, ImagePreprocessingConfig,
    LanguageDetectionConfig, OcrConfig, OutputFormat, PdfConfig, PostProcessorConfig, TokenReductionConfig,
};
use magnus::{Error, RArray, RHash, Ruby, TryConvert, Value};
use magnus::value::ReprValue;
use std::fs;

/// Parse OcrConfig from Ruby Hash
pub fn parse_ocr_config(ruby: &Ruby, hash: RHash) -> Result<OcrConfig, Error> {
    let backend = if let Some(val) = get_kw(ruby, hash, "backend") {
        symbol_to_string(val)?
    } else {
        "tesseract".to_string()
    };

    let language = if let Some(val) = get_kw(ruby, hash, "language") {
        symbol_to_string(val)?
    } else {
        "eng".to_string()
    };

    let mut config = OcrConfig {
        backend,
        language,
        tesseract_config: None,
        output_format: None,
    };

    if let Some(val) = get_kw(ruby, hash, "tesseract_config")
        && !val.is_nil()
    {
        let tc_json = ruby_value_to_json(val)?;
        let parsed: RustTesseractConfig =
            serde_json::from_value(tc_json).map_err(|e| runtime_error(format!("Invalid tesseract_config: {}", e)))?;
        config.tesseract_config = Some(parsed);
    }

    Ok(config)
}

/// Parse ChunkingConfig from Ruby Hash
pub fn parse_chunking_config(ruby: &Ruby, hash: RHash) -> Result<ChunkingConfig, Error> {
    let max_chars = if let Some(val) = get_kw(ruby, hash, "max_chars") {
        usize::try_convert(val)?
    } else {
        1000
    };

    let max_overlap = if let Some(val) = get_kw(ruby, hash, "max_overlap") {
        usize::try_convert(val)?
    } else {
        200
    };

    let preset = if let Some(val) = get_kw(ruby, hash, "preset")
        && !val.is_nil()
    {
        Some(symbol_to_string(val)?)
    } else {
        None
    };

    let embedding = if let Some(val) = get_kw(ruby, hash, "embedding")
        && !val.is_nil()
    {
        let json_value = ruby_value_to_json(val)?;
        let parsed: EmbeddingConfig = serde_json::from_value(json_value)
            .map_err(|e| runtime_error(format!("Invalid chunking.embedding: {}", e)))?;
        Some(parsed)
    } else {
        None
    };

    let config = ChunkingConfig {
        max_chars,
        max_overlap,
        embedding,
        preset,
    };

    Ok(config)
}

/// Parse LanguageDetectionConfig from Ruby Hash
pub fn parse_language_detection_config(ruby: &Ruby, hash: RHash) -> Result<LanguageDetectionConfig, Error> {
    let enabled = if let Some(val) = get_kw(ruby, hash, "enabled") {
        bool::try_convert(val)?
    } else {
        true
    };

    let min_confidence = if let Some(val) = get_kw(ruby, hash, "min_confidence") {
        f64::try_convert(val)?
    } else {
        0.8
    };

    let detect_multiple = if let Some(val) = get_kw(ruby, hash, "detect_multiple") {
        bool::try_convert(val)?
    } else {
        false
    };

    let config = LanguageDetectionConfig {
        enabled,
        min_confidence,
        detect_multiple,
    };

    Ok(config)
}

/// Parse HierarchyConfig from Ruby Hash
pub fn parse_hierarchy_config(ruby: &Ruby, hash: RHash) -> Result<HierarchyConfig, Error> {
    let enabled = if let Some(val) = get_kw(ruby, hash, "enabled") {
        bool::try_convert(val)?
    } else {
        true
    };

    let k_clusters = if let Some(val) = get_kw(ruby, hash, "k_clusters") {
        usize::try_convert(val)?
    } else {
        6
    };

    let include_bbox = if let Some(val) = get_kw(ruby, hash, "include_bbox") {
        bool::try_convert(val)?
    } else {
        true
    };

    let ocr_coverage_threshold = if let Some(val) = get_kw(ruby, hash, "ocr_coverage_threshold") {
        if !val.is_nil() {
            Some(f64::try_convert(val)? as f32)
        } else {
            None
        }
    } else {
        None
    };

    let config = HierarchyConfig {
        enabled,
        k_clusters,
        include_bbox,
        ocr_coverage_threshold,
    };

    Ok(config)
}

/// Parse PdfConfig from Ruby Hash
pub fn parse_pdf_config(ruby: &Ruby, hash: RHash) -> Result<PdfConfig, Error> {
    let extract_images = if let Some(val) = get_kw(ruby, hash, "extract_images") {
        bool::try_convert(val)?
    } else {
        false
    };

    let passwords = if let Some(val) = get_kw(ruby, hash, "passwords") {
        if !val.is_nil() {
            let arr = RArray::try_convert(val)?;
            Some(arr.to_vec::<String>()?)
        } else {
            None
        }
    } else {
        None
    };

    let extract_metadata = if let Some(val) = get_kw(ruby, hash, "extract_metadata") {
        bool::try_convert(val)?
    } else {
        true
    };

    let hierarchy = if let Some(val) = get_kw(ruby, hash, "hierarchy") {
        if !val.is_nil() {
            let h_hash = RHash::try_convert(val)?;
            Some(parse_hierarchy_config(ruby, h_hash)?)
        } else {
            None
        }
    } else {
        None
    };

    let config = PdfConfig {
        extract_images,
        passwords,
        extract_metadata,
        hierarchy,
    };

    Ok(config)
}

/// Parse ImageExtractionConfig from Ruby Hash
pub fn parse_image_extraction_config(ruby: &Ruby, hash: RHash) -> Result<ImageExtractionConfig, Error> {
    let extract_images = if let Some(val) = get_kw(ruby, hash, "extract_images") {
        bool::try_convert(val)?
    } else {
        true
    };

    let target_dpi = if let Some(val) = get_kw(ruby, hash, "target_dpi") {
        i32::try_convert(val)?
    } else {
        300
    };

    let max_image_dimension = if let Some(val) = get_kw(ruby, hash, "max_image_dimension") {
        i32::try_convert(val)?
    } else {
        4096
    };

    let auto_adjust_dpi = if let Some(val) = get_kw(ruby, hash, "auto_adjust_dpi") {
        bool::try_convert(val)?
    } else {
        true
    };

    let min_dpi = if let Some(val) = get_kw(ruby, hash, "min_dpi") {
        i32::try_convert(val)?
    } else {
        72
    };

    let max_dpi = if let Some(val) = get_kw(ruby, hash, "max_dpi") {
        i32::try_convert(val)?
    } else {
        600
    };

    let config = ImageExtractionConfig {
        extract_images,
        target_dpi,
        max_image_dimension,
        auto_adjust_dpi,
        min_dpi,
        max_dpi,
    };

    Ok(config)
}

/// Parse ImagePreprocessingConfig from Ruby Hash
///
/// Note: Currently not used in ExtractionConfig but provided for completeness.
/// ImagePreprocessingConfig is typically used in OCR operations.
#[allow(dead_code)]
pub fn parse_image_preprocessing_config(ruby: &Ruby, hash: RHash) -> Result<ImagePreprocessingConfig, Error> {
    let target_dpi = if let Some(val) = get_kw(ruby, hash, "target_dpi") {
        i32::try_convert(val)?
    } else {
        300
    };

    let auto_rotate = if let Some(val) = get_kw(ruby, hash, "auto_rotate") {
        bool::try_convert(val)?
    } else {
        true
    };

    let deskew = if let Some(val) = get_kw(ruby, hash, "deskew") {
        bool::try_convert(val)?
    } else {
        true
    };

    let denoise = if let Some(val) = get_kw(ruby, hash, "denoise") {
        bool::try_convert(val)?
    } else {
        false
    };

    let contrast_enhance = if let Some(val) = get_kw(ruby, hash, "contrast_enhance") {
        bool::try_convert(val)?
    } else {
        false
    };

    let binarization_method = if let Some(val) = get_kw(ruby, hash, "binarization_method") {
        symbol_to_string(val)?
    } else {
        "otsu".to_string()
    };

    let invert_colors = if let Some(val) = get_kw(ruby, hash, "invert_colors") {
        bool::try_convert(val)?
    } else {
        false
    };

    let config = ImagePreprocessingConfig {
        target_dpi,
        auto_rotate,
        deskew,
        denoise,
        contrast_enhance,
        binarization_method,
        invert_colors,
    };

    Ok(config)
}

/// Parse PostProcessorConfig from Ruby Hash
pub fn parse_postprocessor_config(ruby: &Ruby, hash: RHash) -> Result<PostProcessorConfig, Error> {
    let enabled = if let Some(val) = get_kw(ruby, hash, "enabled") {
        bool::try_convert(val)?
    } else {
        true
    };

    let enabled_processors = if let Some(val) = get_kw(ruby, hash, "enabled_processors")
        && !val.is_nil()
    {
        let arr = RArray::try_convert(val)?;
        Some(arr.to_vec::<String>()?)
    } else {
        None
    };

    let disabled_processors = if let Some(val) = get_kw(ruby, hash, "disabled_processors")
        && !val.is_nil()
    {
        let arr = RArray::try_convert(val)?;
        Some(arr.to_vec::<String>()?)
    } else {
        None
    };

    let config = PostProcessorConfig {
        enabled,
        enabled_processors,
        disabled_processors,
        enabled_set: None,
        disabled_set: None,
    };

    Ok(config)
}

/// Parse TokenReductionConfig from Ruby Hash
pub fn parse_token_reduction_config(ruby: &Ruby, hash: RHash) -> Result<TokenReductionConfig, Error> {
    let mode = if let Some(val) = get_kw(ruby, hash, "mode") {
        symbol_to_string(val)?
    } else {
        "off".to_string()
    };

    let preserve_important_words = if let Some(val) = get_kw(ruby, hash, "preserve_important_words") {
        bool::try_convert(val)?
    } else {
        true
    };

    let config = TokenReductionConfig {
        mode,
        preserve_important_words,
    };

    Ok(config)
}

/// Parse KeywordConfig from Ruby Hash
pub fn parse_keyword_config(ruby: &Ruby, hash: RHash) -> Result<RustKeywordConfig, Error> {
    let mut config = RustKeywordConfig::default();

    if let Some(val) = get_kw(ruby, hash, "algorithm") {
        let algo = symbol_to_string(val)?;
        config.algorithm = match algo.to_lowercase().as_str() {
            "yake" => RustKeywordAlgorithm::Yake,
            "rake" => RustKeywordAlgorithm::Rake,
            other => {
                return Err(runtime_error(format!(
                    "Invalid keywords.algorithm '{}', expected 'yake' or 'rake'",
                    other
                )));
            }
        };
    }

    if let Some(val) = get_kw(ruby, hash, "max_keywords") {
        config.max_keywords = usize::try_convert(val)?;
    }

    if let Some(val) = get_kw(ruby, hash, "min_score") {
        config.min_score = f64::try_convert(val)? as f32;
    }

    if let Some(val) = get_kw(ruby, hash, "ngram_range") {
        let ary = RArray::try_convert(val)?;
        if ary.len() == 2 {
            let values = ary.to_vec::<i64>()?;
            config.ngram_range = (values[0] as usize, values[1] as usize);
        } else {
            return Err(runtime_error("keywords.ngram_range must have exactly two values"));
        }
    }

    if let Some(val) = get_kw(ruby, hash, "language")
        && !val.is_nil()
    {
        config.language = Some(symbol_to_string(val)?);
    }

    if let Some(val) = get_kw(ruby, hash, "yake_params")
        && !val.is_nil()
    {
        let yake_hash = RHash::try_convert(val)?;
        let window = if let Some(window_val) = get_kw(ruby, yake_hash, "window_size") {
            usize::try_convert(window_val)?
        } else {
            2
        };
        config.yake_params = Some(RustYakeParams { window_size: window });
    }

    if let Some(val) = get_kw(ruby, hash, "rake_params")
        && !val.is_nil()
    {
        let rake_hash = RHash::try_convert(val)?;
        let mut params = RustRakeParams::default();
        if let Some(val) = get_kw(ruby, rake_hash, "min_word_length") {
            params.min_word_length = usize::try_convert(val)?;
        }
        if let Some(val) = get_kw(ruby, rake_hash, "max_words_per_phrase") {
            params.max_words_per_phrase = usize::try_convert(val)?;
        }
        config.rake_params = Some(params);
    }

    Ok(config)
}

/// Parse HTML conversion options from Ruby Hash
pub fn parse_html_options(ruby: &Ruby, hash: RHash) -> Result<ConversionOptions, Error> {
    let mut options = ConversionOptions::default();

    if let Some(val) = get_kw(ruby, hash, "heading_style") {
        let style = symbol_to_string(val)?;
        options.heading_style = match style.to_lowercase().as_str() {
            "atx" => HeadingStyle::Atx,
            "underlined" => HeadingStyle::Underlined,
            "atx_closed" | "atx-closed" => HeadingStyle::AtxClosed,
            other => return Err(runtime_error(format!("Invalid html_options.heading_style '{}'", other))),
        };
    }

    if let Some(val) = get_kw(ruby, hash, "list_indent_type") {
        let val_str = symbol_to_string(val)?;
        options.list_indent_type = match val_str.to_lowercase().as_str() {
            "spaces" => ListIndentType::Spaces,
            "tabs" => ListIndentType::Tabs,
            other => {
                return Err(runtime_error(format!(
                    "Invalid html_options.list_indent_type '{}'",
                    other
                )));
            }
        };
    }

    if let Some(val) = get_kw(ruby, hash, "list_indent_width") {
        options.list_indent_width = usize::try_convert(val)?;
    }

    if let Some(val) = get_kw(ruby, hash, "bullets") {
        options.bullets = String::try_convert(val)?;
    }

    if let Some(val) = get_kw(ruby, hash, "strong_em_symbol") {
        let symbol = String::try_convert(val)?;
        let mut chars = symbol.chars();
        options.strong_em_symbol = chars
            .next()
            .ok_or_else(|| runtime_error("html_options.strong_em_symbol must not be empty"))?;
    }

    if let Some(val) = get_kw(ruby, hash, "escape_asterisks") {
        options.escape_asterisks = bool::try_convert(val)?;
    }
    if let Some(val) = get_kw(ruby, hash, "escape_underscores") {
        options.escape_underscores = bool::try_convert(val)?;
    }
    if let Some(val) = get_kw(ruby, hash, "escape_misc") {
        options.escape_misc = bool::try_convert(val)?;
    }
    if let Some(val) = get_kw(ruby, hash, "escape_ascii") {
        options.escape_ascii = bool::try_convert(val)?;
    }

    if let Some(val) = get_kw(ruby, hash, "code_language") {
        options.code_language = String::try_convert(val)?;
    }

    if let Some(val) = get_kw(ruby, hash, "autolinks") {
        options.autolinks = bool::try_convert(val)?;
    }

    if let Some(val) = get_kw(ruby, hash, "default_title") {
        options.default_title = bool::try_convert(val)?;
    }

    if let Some(val) = get_kw(ruby, hash, "br_in_tables") {
        options.br_in_tables = bool::try_convert(val)?;
    }

    if let Some(val) = get_kw(ruby, hash, "hocr_spatial_tables") {
        options.hocr_spatial_tables = bool::try_convert(val)?;
    }

    if let Some(val) = get_kw(ruby, hash, "highlight_style") {
        let style = symbol_to_string(val)?;
        options.highlight_style = match style.to_lowercase().as_str() {
            "double_equal" | "double-equal" => HighlightStyle::DoubleEqual,
            "html" => HighlightStyle::Html,
            "bold" => HighlightStyle::Bold,
            "none" => HighlightStyle::None,
            other => {
                return Err(runtime_error(format!(
                    "Invalid html_options.highlight_style '{}'",
                    other
                )));
            }
        };
    }

    if let Some(val) = get_kw(ruby, hash, "extract_metadata") {
        options.extract_metadata = bool::try_convert(val)?;
    }

    if let Some(val) = get_kw(ruby, hash, "whitespace_mode") {
        let mode = symbol_to_string(val)?;
        options.whitespace_mode = match mode.to_lowercase().as_str() {
            "normalized" => WhitespaceMode::Normalized,
            "strict" => WhitespaceMode::Strict,
            other => {
                return Err(runtime_error(format!(
                    "Invalid html_options.whitespace_mode '{}'",
                    other
                )));
            }
        };
    }

    if let Some(val) = get_kw(ruby, hash, "strip_newlines") {
        options.strip_newlines = bool::try_convert(val)?;
    }

    if let Some(val) = get_kw(ruby, hash, "wrap") {
        options.wrap = bool::try_convert(val)?;
    }

    if let Some(val) = get_kw(ruby, hash, "wrap_width") {
        options.wrap_width = usize::try_convert(val)?;
    }

    if let Some(val) = get_kw(ruby, hash, "convert_as_inline") {
        options.convert_as_inline = bool::try_convert(val)?;
    }

    if let Some(val) = get_kw(ruby, hash, "sub_symbol") {
        options.sub_symbol = String::try_convert(val)?;
    }

    if let Some(val) = get_kw(ruby, hash, "sup_symbol") {
        options.sup_symbol = String::try_convert(val)?;
    }

    if let Some(val) = get_kw(ruby, hash, "newline_style") {
        let style = symbol_to_string(val)?;
        options.newline_style = match style.to_lowercase().as_str() {
            "spaces" => NewlineStyle::Spaces,
            "backslash" => NewlineStyle::Backslash,
            other => return Err(runtime_error(format!("Invalid html_options.newline_style '{}'", other))),
        };
    }

    if let Some(val) = get_kw(ruby, hash, "code_block_style") {
        let style = symbol_to_string(val)?;
        options.code_block_style = match style.to_lowercase().as_str() {
            "indented" => CodeBlockStyle::Indented,
            "backticks" => CodeBlockStyle::Backticks,
            "tildes" => CodeBlockStyle::Tildes,
            other => {
                return Err(runtime_error(format!(
                    "Invalid html_options.code_block_style '{}'",
                    other
                )));
            }
        };
    }

    if let Some(val) = get_kw(ruby, hash, "keep_inline_images_in") {
        let arr = RArray::try_convert(val)?;
        options.keep_inline_images_in = arr.to_vec::<String>()?;
    }

    if let Some(val) = get_kw(ruby, hash, "encoding") {
        options.encoding = String::try_convert(val)?;
    }

    if let Some(val) = get_kw(ruby, hash, "debug") {
        options.debug = bool::try_convert(val)?;
    }

    if let Some(val) = get_kw(ruby, hash, "strip_tags") {
        let arr = RArray::try_convert(val)?;
        options.strip_tags = arr.to_vec::<String>()?;
    }

    if let Some(val) = get_kw(ruby, hash, "preserve_tags") {
        let arr = RArray::try_convert(val)?;
        options.preserve_tags = arr.to_vec::<String>()?;
    }

    if let Some(val) = get_kw(ruby, hash, "preprocessing")
        && !val.is_nil()
    {
        let pre_hash = RHash::try_convert(val)?;
        let mut preprocessing = options.preprocessing.clone();
        if let Some(v) = get_kw(ruby, pre_hash, "enabled") {
            preprocessing.enabled = bool::try_convert(v)?;
        }
        if let Some(v) = get_kw(ruby, pre_hash, "preset") {
            let preset = symbol_to_string(v)?;
            preprocessing.preset = match preset.to_lowercase().as_str() {
                "minimal" => PreprocessingPreset::Minimal,
                "standard" => PreprocessingPreset::Standard,
                "aggressive" => PreprocessingPreset::Aggressive,
                other => {
                    return Err(runtime_error(format!(
                        "Invalid html_options.preprocessing.preset '{}'",
                        other
                    )));
                }
            };
        }
        if let Some(v) = get_kw(ruby, pre_hash, "remove_navigation") {
            preprocessing.remove_navigation = bool::try_convert(v)?;
        }
        if let Some(v) = get_kw(ruby, pre_hash, "remove_forms") {
            preprocessing.remove_forms = bool::try_convert(v)?;
        }
        options.preprocessing = preprocessing;
    }

    Ok(options)
}

/// Convert KeywordAlgorithm to string
#[allow(dead_code)]
pub fn keyword_algorithm_to_str(algo: RustKeywordAlgorithm) -> &'static str {
    match algo {
        RustKeywordAlgorithm::Yake => "yake",
        RustKeywordAlgorithm::Rake => "rake",
    }
}

/// Convert KeywordConfig to Ruby Hash
#[allow(dead_code)]
pub fn keyword_config_to_ruby_hash(ruby: &Ruby, config: &RustKeywordConfig) -> Result<RHash, Error> {
    let hash = ruby.hash_new();
    hash.aset("algorithm", keyword_algorithm_to_str(config.algorithm))?;
    hash.aset("max_keywords", config.max_keywords as i64)?;
    hash.aset("min_score", config.min_score)?;
    hash.aset("language", config.language.clone().unwrap_or_default())?;

    let range_array = ruby.ary_new();
    range_array.push(config.ngram_range.0 as i64)?;
    range_array.push(config.ngram_range.1 as i64)?;
    hash.aset("ngram_range", range_array)?;

    if let Some(yake) = &config.yake_params {
        let yake_hash = ruby.hash_new();
        yake_hash.aset("window_size", yake.window_size as i64)?;
        hash.aset("yake_params", yake_hash)?;
    }

    if let Some(rake) = &config.rake_params {
        let rake_hash = ruby.hash_new();
        rake_hash.aset("min_word_length", rake.min_word_length as i64)?;
        rake_hash.aset("max_words_per_phrase", rake.max_words_per_phrase as i64)?;
        hash.aset("rake_params", rake_hash)?;
    }

    Ok(hash)
}

/// Convert HTML conversion options to Ruby Hash
#[allow(dead_code)]
pub fn html_options_to_ruby_hash(ruby: &Ruby, options: &ConversionOptions) -> Result<RHash, Error> {
    let hash = ruby.hash_new();
    hash.aset(
        "heading_style",
        match options.heading_style {
            HeadingStyle::Atx => "atx",
            HeadingStyle::Underlined => "underlined",
            HeadingStyle::AtxClosed => "atx_closed",
        },
    )?;
    hash.aset(
        "list_indent_type",
        match options.list_indent_type {
            ListIndentType::Spaces => "spaces",
            ListIndentType::Tabs => "tabs",
        },
    )?;
    hash.aset("list_indent_width", options.list_indent_width as i64)?;
    hash.aset("bullets", options.bullets.clone())?;
    hash.aset("strong_em_symbol", options.strong_em_symbol.to_string())?;
    hash.aset("escape_asterisks", options.escape_asterisks)?;
    hash.aset("escape_underscores", options.escape_underscores)?;
    hash.aset("escape_misc", options.escape_misc)?;
    hash.aset("escape_ascii", options.escape_ascii)?;
    hash.aset("code_language", options.code_language.clone())?;
    hash.aset("autolinks", options.autolinks)?;
    hash.aset("default_title", options.default_title)?;
    hash.aset("br_in_tables", options.br_in_tables)?;
    hash.aset("hocr_spatial_tables", options.hocr_spatial_tables)?;
    hash.aset(
        "highlight_style",
        match options.highlight_style {
            HighlightStyle::DoubleEqual => "double_equal",
            HighlightStyle::Html => "html",
            HighlightStyle::Bold => "bold",
            HighlightStyle::None => "none",
        },
    )?;
    hash.aset("extract_metadata", options.extract_metadata)?;
    hash.aset(
        "whitespace_mode",
        match options.whitespace_mode {
            WhitespaceMode::Normalized => "normalized",
            WhitespaceMode::Strict => "strict",
        },
    )?;
    hash.aset("strip_newlines", options.strip_newlines)?;
    hash.aset("wrap", options.wrap)?;
    hash.aset("wrap_width", options.wrap_width as i64)?;
    hash.aset("convert_as_inline", options.convert_as_inline)?;
    hash.aset("sub_symbol", options.sub_symbol.clone())?;
    hash.aset("sup_symbol", options.sup_symbol.clone())?;
    hash.aset(
        "newline_style",
        match options.newline_style {
            NewlineStyle::Spaces => "spaces",
            NewlineStyle::Backslash => "backslash",
        },
    )?;
    hash.aset(
        "code_block_style",
        match options.code_block_style {
            CodeBlockStyle::Indented => "indented",
            CodeBlockStyle::Backticks => "backticks",
            CodeBlockStyle::Tildes => "tildes",
        },
    )?;

    let keep_inline = ruby.ary_new();
    for tag in &options.keep_inline_images_in {
        keep_inline.push(tag.as_str())?;
    }
    hash.aset("keep_inline_images_in", keep_inline)?;

    hash.aset("encoding", options.encoding.clone())?;
    hash.aset("debug", options.debug)?;

    let strip_tags = ruby.ary_new();
    for tag in &options.strip_tags {
        strip_tags.push(tag.as_str())?;
    }
    hash.aset("strip_tags", strip_tags)?;

    let preserve_tags = ruby.ary_new();
    for tag in &options.preserve_tags {
        preserve_tags.push(tag.as_str())?;
    }
    hash.aset("preserve_tags", preserve_tags)?;

    let pre_hash = ruby.hash_new();
    pre_hash.aset("enabled", options.preprocessing.enabled)?;
    pre_hash.aset(
        "preset",
        match options.preprocessing.preset {
            PreprocessingPreset::Minimal => "minimal",
            PreprocessingPreset::Standard => "standard",
            PreprocessingPreset::Aggressive => "aggressive",
        },
    )?;
    pre_hash.aset("remove_navigation", options.preprocessing.remove_navigation)?;
    pre_hash.aset("remove_forms", options.preprocessing.remove_forms)?;
    hash.aset("preprocessing", pre_hash)?;

    Ok(hash)
}

/// Parse PageConfig from Ruby Hash
pub fn parse_page_config(ruby: &Ruby, hash: RHash) -> Result<PageConfig, Error> {
    let extract_pages = if let Some(val) = get_kw(ruby, hash, "extract_pages") {
        bool::try_convert(val)?
    } else {
        false
    };

    let insert_page_markers = if let Some(val) = get_kw(ruby, hash, "insert_page_markers") {
        bool::try_convert(val)?
    } else {
        false
    };

    let marker_format = if let Some(val) = get_kw(ruby, hash, "marker_format") {
        String::try_convert(val)?
    } else {
        "\n\n<!-- PAGE {page_num} -->\n\n".to_string()
    };

    let config = PageConfig {
        extract_pages,
        insert_page_markers,
        marker_format,
    };

    Ok(config)
}

/// Parse ExtractionConfig from Ruby Hash
pub fn parse_extraction_config(ruby: &Ruby, opts: Option<RHash>) -> Result<ExtractionConfig, Error> {
    let mut config = ExtractionConfig::default();

    if let Some(hash) = opts {
        if let Some(val) = get_kw(ruby, hash, "use_cache") {
            config.use_cache = bool::try_convert(val)?;
        }

        if let Some(val) = get_kw(ruby, hash, "enable_quality_processing") {
            config.enable_quality_processing = bool::try_convert(val)?;
        }

        if let Some(val) = get_kw(ruby, hash, "force_ocr") {
            config.force_ocr = bool::try_convert(val)?;
        }

        if let Some(val) = get_kw(ruby, hash, "ocr")
            && !val.is_nil()
        {
            let ocr_hash = RHash::try_convert(val)?;
            config.ocr = Some(parse_ocr_config(ruby, ocr_hash)?);
        }

        if let Some(val) = get_kw(ruby, hash, "chunking")
            && !val.is_nil()
        {
            let chunking_hash = RHash::try_convert(val)?;
            config.chunking = Some(parse_chunking_config(ruby, chunking_hash)?);
        }

        if let Some(val) = get_kw(ruby, hash, "language_detection")
            && !val.is_nil()
        {
            let lang_hash = RHash::try_convert(val)?;
            config.language_detection = Some(parse_language_detection_config(ruby, lang_hash)?);
        }

        if let Some(val) = get_kw(ruby, hash, "pdf_options")
            && !val.is_nil()
        {
            let pdf_hash = RHash::try_convert(val)?;
            config.pdf_options = Some(parse_pdf_config(ruby, pdf_hash)?);
        }

        if let Some(val) = get_kw(ruby, hash, "images")
            && !val.is_nil()
        {
            let images_hash = RHash::try_convert(val)?;
            config.images = Some(parse_image_extraction_config(ruby, images_hash)?);
        }

        if let Some(val) = get_kw(ruby, hash, "postprocessor")
            && !val.is_nil()
        {
            let postprocessor_hash = RHash::try_convert(val)?;
            config.postprocessor = Some(parse_postprocessor_config(ruby, postprocessor_hash)?);
        }

        if let Some(val) = get_kw(ruby, hash, "token_reduction")
            && !val.is_nil()
        {
            let token_reduction_hash = RHash::try_convert(val)?;
            config.token_reduction = Some(parse_token_reduction_config(ruby, token_reduction_hash)?);
        }

        if let Some(val) = get_kw(ruby, hash, "keywords")
            && !val.is_nil()
        {
            let keywords_hash = RHash::try_convert(val)?;
            config.keywords = Some(parse_keyword_config(ruby, keywords_hash)?);
        }

        if let Some(val) = get_kw(ruby, hash, "html_options")
            && !val.is_nil()
        {
            let html_hash = RHash::try_convert(val)?;
            config.html_options = Some(parse_html_options(ruby, html_hash)?);
        }

        if let Some(val) = get_kw(ruby, hash, "pages")
            && !val.is_nil()
        {
            let pages_hash = RHash::try_convert(val)?;
            config.pages = Some(parse_page_config(ruby, pages_hash)?);
        }

        if let Some(val) = get_kw(ruby, hash, "max_concurrent_extractions") {
            let value = usize::try_convert(val)?;
            config.max_concurrent_extractions = Some(value);
        }

        if let Some(val) = get_kw(ruby, hash, "result_format") {
            let format_str = String::try_convert(val)?;
            config.result_format = match format_str.as_str() {
                "unified" | "Unified" => kreuzberg::types::OutputFormat::Unified,
                "element_based" | "ElementBased" | "elements" => kreuzberg::types::OutputFormat::ElementBased,
                _ => {
                    return Err(runtime_error(format!(
                        "Invalid result_format: '{}'. Expected 'unified' or 'element_based'",
                        format_str
                    )))
                }
            };
        }

        if let Some(val) = get_kw(ruby, hash, "output_format") {
            let format_str = String::try_convert(val)?;
            config.output_format = match format_str.as_str() {
                "plain" | "Plain" => OutputFormat::Plain,
                "markdown" | "Markdown" => OutputFormat::Markdown,
                "djot" | "Djot" => OutputFormat::Djot,
                "html" | "Html" => OutputFormat::Html,
                _ => {
                    return Err(runtime_error(format!(
                        "Invalid output_format: '{}'. Expected 'plain', 'markdown', 'djot', or 'html'",
                        format_str
                    )))
                }
            };
        }
    }

    Ok(config)
}

/// Load extraction config from file
///
/// Supports TOML, YAML, and JSON file formats. The format is detected from the file extension.
pub fn config_from_file(path: String) -> Result<RHash, Error> {
    use std::path::Path;

    let ruby = Ruby::get().expect("Ruby not initialized");
    let file_path = Path::new(&path);

    let content = fs::read_to_string(&path)
        .map_err(|e| validation_error(format!("Failed to read config file '{}': {}", path, e)))?;

    // Detect file format from extension
    let extension = file_path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|s| s.to_lowercase());

    let json_value: serde_json::Value = match extension.as_deref() {
        Some("toml") => {
            toml::from_str(&content)
                .map_err(|e| validation_error(format!("Invalid TOML in config file '{}': {}", path, e)))?
        }
        Some("yaml") | Some("yml") => {
            serde_yaml_ng::from_str(&content)
                .map_err(|e| validation_error(format!("Invalid YAML in config file '{}': {}", path, e)))?
        }
        Some("json") => {
            serde_json::from_str(&content)
                .map_err(|e| validation_error(format!("Invalid JSON in config file '{}': {}", path, e)))?
        }
        Some(ext) => {
            return Err(validation_error(format!(
                "Unsupported config file format: .{}. Supported formats: .toml, .yaml, .yml, .json",
                ext
            )));
        }
        None => {
            return Err(validation_error(format!(
                "Cannot determine file format: no extension found in '{}'",
                path
            )));
        }
    };

    json_value_to_ruby(&ruby, &json_value)
        .and_then(|v| magnus::RHash::try_convert(v).map_err(|_| validation_error("Config must be a Hash")))
}

/// Discover extraction config from current directory
pub fn config_discover() -> Result<Value, Error> {
    let ruby = Ruby::get().expect("Ruby not initialized");

    // Search for config files in order of precedence
    let config_files = vec![
        ("kreuzberg.toml", "toml"),
        ("kreuzberg.yaml", "yaml"),
        ("kreuzberg.yml", "yaml"),
        ("kreuzberg.json", "json"),
        (".kreuzbergrc", "json"),
    ];

    for (name, format) in config_files {
        if let Ok(content) = fs::read_to_string(name) {
            let json_value: serde_json::Value = match format {
                "toml" => toml::from_str(&content)
                    .map_err(|e| validation_error(format!("Invalid TOML in {}: {}", name, e)))?,
                "yaml" => serde_yaml_ng::from_str(&content)
                    .map_err(|e| validation_error(format!("Invalid YAML in {}: {}", name, e)))?,
                "json" => serde_json::from_str(&content)
                    .map_err(|e| validation_error(format!("Invalid JSON in {}: {}", name, e)))?,
                _ => unreachable!(),
            };
            return json_value_to_ruby(&ruby, &json_value);
        }
    }

    // Return nil if no config found
    Ok(ruby.qnil().as_value())
}
