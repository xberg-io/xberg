//! Configuration parsing and conversion for Ruby bindings
//!
//! Handles conversion between Ruby Hash configurations and Rust config types.
//! Includes parsing for all nested configuration structures.

use crate::error_handling::{runtime_error, validation_error};
use crate::helpers::{get_kw, json_value_to_ruby, ruby_value_to_json, symbol_to_string};

use html_to_markdown_rs::WhitespaceMode;
use html_to_markdown_rs::options::{
    CodeBlockStyle, ConversionOptions, HeadingStyle, HighlightStyle, ListIndentType, NewlineStyle, PreprocessingPreset,
};
use kreuzberg::core::config::PageConfig;
use kreuzberg::keywords::{
    KeywordAlgorithm as RustKeywordAlgorithm, KeywordConfig as RustKeywordConfig, RakeParams as RustRakeParams,
    YakeParams as RustYakeParams,
};
use kreuzberg::pdf::HierarchyConfig;
use kreuzberg::types::TesseractConfig as RustTesseractConfig;
use kreuzberg::{
    AccelerationConfig, ChunkingConfig, EmailConfig, EmbeddingConfig, ExecutionProviderType, ExtractionConfig,
    ImageExtractionConfig, LanguageDetectionConfig, LayoutDetectionConfig, OcrConfig, OutputFormat, PdfConfig,
    PostProcessorConfig, TokenReductionConfig,
};
use magnus::value::ReprValue;
use magnus::{Error, RArray, RHash, Ruby, TryConvert, Value};
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
        paddle_ocr_config: None,
        element_config: None,
        tesseract_config: None,
        output_format: None,
        auto_rotate: false,
        pipeline: None,
        quality_thresholds: None,
    };

    if let Some(val) = get_kw(ruby, hash, "tesseract_config")
        && val.equal(ruby.qnil()).ok() != Some(true)
    {
        let tc_json = ruby_value_to_json(val)?;
        let parsed: RustTesseractConfig =
            serde_json::from_value(tc_json).map_err(|e| runtime_error(format!("Invalid tesseract_config: {}", e)))?;
        config.tesseract_config = Some(parsed);
    }

    if let Some(val) = get_kw(ruby, hash, "paddle_ocr_config")
        && val.equal(ruby.qnil()).ok() != Some(true)
    {
        config.paddle_ocr_config = Some(ruby_value_to_json(val)?);
    }

    if let Some(val) = get_kw(ruby, hash, "element_config")
        && val.equal(ruby.qnil()).ok() != Some(true)
    {
        let ec_json = ruby_value_to_json(val)?;
        let parsed: kreuzberg::types::OcrElementConfig =
            serde_json::from_value(ec_json).map_err(|e| runtime_error(format!("Invalid element_config: {}", e)))?;
        config.element_config = Some(parsed);
    }

    if let Some(val) = get_kw(ruby, hash, "output_format")
        && val.equal(ruby.qnil()).ok() != Some(true)
    {
        let format_str = symbol_to_string(val)?;
        let format: OutputFormat = match format_str.as_str() {
            "plain" | "Plain" => OutputFormat::Plain,
            "markdown" | "Markdown" => OutputFormat::Markdown,
            "djot" | "Djot" => OutputFormat::Djot,
            "html" | "Html" => OutputFormat::Html,
            other => return Err(runtime_error(format!("Invalid ocr output_format: '{}'", other))),
        };
        config.output_format = Some(format);
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
        && val.equal(ruby.qnil()).ok() != Some(true)
    {
        Some(symbol_to_string(val)?)
    } else {
        None
    };

    let embedding = if let Some(val) = get_kw(ruby, hash, "embedding")
        && val.equal(ruby.qnil()).ok() != Some(true)
    {
        let json_value = ruby_value_to_json(val)?;
        let parsed: EmbeddingConfig = serde_json::from_value(json_value)
            .map_err(|e| runtime_error(format!("Invalid chunking.embedding: {}", e)))?;
        Some(parsed)
    } else {
        None
    };

    let chunker_type = if let Some(val) = get_kw(ruby, hash, "chunker_type")
        && val.equal(ruby.qnil()).ok() != Some(true)
    {
        match symbol_to_string(val)?.as_str() {
            "markdown" => kreuzberg::ChunkerType::Markdown,
            _ => kreuzberg::ChunkerType::Text,
        }
    } else {
        kreuzberg::ChunkerType::Text
    };

    let sizing = parse_chunk_sizing(ruby, hash)?;

    let prepend_heading_context = if let Some(val) = get_kw(ruby, hash, "prepend_heading_context") {
        bool::try_convert(val)?
    } else {
        false
    };

    let config = ChunkingConfig {
        max_characters: max_chars,
        overlap: max_overlap,
        trim: true,
        chunker_type,
        embedding,
        preset,
        sizing,
        prepend_heading_context,
    };

    Ok(config)
}

/// Parse ChunkSizing from Ruby Hash fields.
fn parse_chunk_sizing(ruby: &Ruby, hash: RHash) -> Result<kreuzberg::ChunkSizing, Error> {
    let sizing_type = if let Some(val) = get_kw(ruby, hash, "sizing_type")
        && val.equal(ruby.qnil()).ok() != Some(true)
    {
        Some(symbol_to_string(val)?)
    } else {
        None
    };

    match sizing_type.as_deref() {
        Some("tokenizer") => {
            let model = if let Some(val) = get_kw(ruby, hash, "sizing_model")
                && val.equal(ruby.qnil()).ok() != Some(true)
            {
                String::try_convert(val)?
            } else {
                "Xenova/gpt-4o".to_string()
            };
            let cache_dir = if let Some(val) = get_kw(ruby, hash, "sizing_cache_dir")
                && val.equal(ruby.qnil()).ok() != Some(true)
            {
                Some(std::path::PathBuf::from(String::try_convert(val)?))
            } else {
                None
            };
            Ok(kreuzberg::ChunkSizing::Tokenizer { model, cache_dir })
        }
        _ => Ok(kreuzberg::ChunkSizing::Characters),
    }
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
        if val.equal(ruby.qnil()).ok() != Some(true) {
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
        if val.equal(ruby.qnil()).ok() != Some(true) {
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
        if val.equal(ruby.qnil()).ok() != Some(true) {
            let h_hash = RHash::try_convert(val)?;
            Some(parse_hierarchy_config(ruby, h_hash)?)
        } else {
            None
        }
    } else {
        None
    };

    let extract_annotations = if let Some(val) = get_kw(ruby, hash, "extract_annotations") {
        bool::try_convert(val)?
    } else {
        false
    };

    let top_margin_fraction = if let Some(val) = get_kw(ruby, hash, "top_margin_fraction") {
        if val.equal(ruby.qnil()).ok() != Some(true) {
            Some(f32::try_convert(val)?)
        } else {
            None
        }
    } else {
        None
    };

    let bottom_margin_fraction = if let Some(val) = get_kw(ruby, hash, "bottom_margin_fraction") {
        if val.equal(ruby.qnil()).ok() != Some(true) {
            Some(f32::try_convert(val)?)
        } else {
            None
        }
    } else {
        None
    };

    let allow_single_column_tables = if let Some(val) = get_kw(ruby, hash, "allow_single_column_tables") {
        bool::try_convert(val)?
    } else {
        false
    };

    let config = PdfConfig {
        extract_images,
        passwords,
        extract_metadata,
        hierarchy,
        extract_annotations,
        top_margin_fraction,
        bottom_margin_fraction,
        allow_single_column_tables,
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

    let inject_placeholders = if let Some(val) = get_kw(ruby, hash, "inject_placeholders") {
        bool::try_convert(val)?
    } else {
        true
    };

    let config = ImageExtractionConfig {
        extract_images,
        target_dpi,
        max_image_dimension,
        inject_placeholders,
        auto_adjust_dpi,
        min_dpi,
        max_dpi,
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
        && val.equal(ruby.qnil()).ok() != Some(true)
    {
        let arr = RArray::try_convert(val)?;
        Some(arr.to_vec::<String>()?)
    } else {
        None
    };

    let disabled_processors = if let Some(val) = get_kw(ruby, hash, "disabled_processors")
        && val.equal(ruby.qnil()).ok() != Some(true)
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
        && val.equal(ruby.qnil()).ok() != Some(true)
    {
        config.language = Some(symbol_to_string(val)?);
    }

    if let Some(val) = get_kw(ruby, hash, "yake_params")
        && val.equal(ruby.qnil()).ok() != Some(true)
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
        && val.equal(ruby.qnil()).ok() != Some(true)
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
        && val.equal(ruby.qnil()).ok() != Some(true)
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

/// Parse AccelerationConfig from Ruby Hash
pub fn parse_acceleration_config(ruby: &Ruby, hash: RHash) -> Result<AccelerationConfig, Error> {
    let provider = if let Some(val) = get_kw(ruby, hash, "provider") {
        let provider_str = symbol_to_string(val)?;
        match provider_str.to_lowercase().as_str() {
            "auto" => ExecutionProviderType::Auto,
            "cpu" => ExecutionProviderType::Cpu,
            "coreml" => ExecutionProviderType::CoreMl,
            "cuda" => ExecutionProviderType::Cuda,
            "tensorrt" => ExecutionProviderType::TensorRt,
            other => return Err(runtime_error(format!("Invalid acceleration provider: '{}'", other))),
        }
    } else {
        ExecutionProviderType::Auto
    };

    let device_id = if let Some(val) = get_kw(ruby, hash, "device_id") {
        u32::try_convert(val)?
    } else {
        0
    };

    let config = AccelerationConfig { provider, device_id };

    Ok(config)
}

/// Parse LayoutDetectionConfig from Ruby Hash
pub fn parse_layout_detection_config(ruby: &Ruby, hash: RHash) -> Result<LayoutDetectionConfig, Error> {
    let preset = if let Some(val) = get_kw(ruby, hash, "preset")
        && val.equal(ruby.qnil()).ok() != Some(true)
    {
        symbol_to_string(val)?
    } else {
        "accurate".to_string()
    };

    let confidence_threshold = if let Some(val) = get_kw(ruby, hash, "confidence_threshold")
        && val.equal(ruby.qnil()).ok() != Some(true)
    {
        Some(f32::try_convert(val)?)
    } else {
        None
    };

    let apply_heuristics = if let Some(val) = get_kw(ruby, hash, "apply_heuristics") {
        bool::try_convert(val)?
    } else {
        true
    };

    let table_model = if let Some(val) = get_kw(ruby, hash, "table_model")
        && val.equal(ruby.qnil()).ok() != Some(true)
    {
        Some(String::try_convert(val)?)
    } else {
        None
    };

    let config = LayoutDetectionConfig {
        preset,
        confidence_threshold,
        apply_heuristics,
        table_model,
    };

    Ok(config)
}

/// Parse EmailConfig from Ruby Hash
pub fn parse_email_config(ruby: &Ruby, hash: RHash) -> Result<EmailConfig, Error> {
    let msg_fallback_codepage = if let Some(val) = get_kw(ruby, hash, "msg_fallback_codepage")
        && val.equal(ruby.qnil()).ok() != Some(true)
    {
        Some(u32::try_convert(val)?)
    } else {
        None
    };

    let config = EmailConfig { msg_fallback_codepage };

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

        if let Some(val) = get_kw(ruby, hash, "force_ocr_pages")
            && val.equal(ruby.qnil()).ok() != Some(true)
        {
            let pages_array = magnus::RArray::try_convert(val)?;
            let pages: Vec<usize> = pages_array
                .into_iter()
                .map(|v| usize::try_convert(v))
                .collect::<Result<Vec<_>, _>>()?;
            config.force_ocr_pages = Some(pages);
        }

        if let Some(val) = get_kw(ruby, hash, "include_document_structure") {
            config.include_document_structure = bool::try_convert(val)?;
        }

        if let Some(val) = get_kw(ruby, hash, "ocr")
            && val.equal(ruby.qnil()).ok() != Some(true)
        {
            let ocr_hash = RHash::try_convert(val)?;
            config.ocr = Some(parse_ocr_config(ruby, ocr_hash)?);
        }

        if let Some(val) = get_kw(ruby, hash, "chunking")
            && val.equal(ruby.qnil()).ok() != Some(true)
        {
            let chunking_hash = RHash::try_convert(val)?;
            config.chunking = Some(parse_chunking_config(ruby, chunking_hash)?);
        }

        if let Some(val) = get_kw(ruby, hash, "language_detection")
            && val.equal(ruby.qnil()).ok() != Some(true)
        {
            let lang_hash = RHash::try_convert(val)?;
            config.language_detection = Some(parse_language_detection_config(ruby, lang_hash)?);
        }

        if let Some(val) = get_kw(ruby, hash, "pdf_options")
            && val.equal(ruby.qnil()).ok() != Some(true)
        {
            let pdf_hash = RHash::try_convert(val)?;
            config.pdf_options = Some(parse_pdf_config(ruby, pdf_hash)?);
        }

        if let Some(val) = get_kw(ruby, hash, "images")
            && val.equal(ruby.qnil()).ok() != Some(true)
        {
            let images_hash = RHash::try_convert(val)?;
            config.images = Some(parse_image_extraction_config(ruby, images_hash)?);
        }

        if let Some(val) = get_kw(ruby, hash, "postprocessor")
            && val.equal(ruby.qnil()).ok() != Some(true)
        {
            let postprocessor_hash = RHash::try_convert(val)?;
            config.postprocessor = Some(parse_postprocessor_config(ruby, postprocessor_hash)?);
        }

        if let Some(val) = get_kw(ruby, hash, "token_reduction")
            && val.equal(ruby.qnil()).ok() != Some(true)
        {
            let token_reduction_hash = RHash::try_convert(val)?;
            config.token_reduction = Some(parse_token_reduction_config(ruby, token_reduction_hash)?);
        }

        if let Some(val) = get_kw(ruby, hash, "keywords")
            && val.equal(ruby.qnil()).ok() != Some(true)
        {
            let keywords_hash = RHash::try_convert(val)?;
            config.keywords = Some(parse_keyword_config(ruby, keywords_hash)?);
        }

        if let Some(val) = get_kw(ruby, hash, "acceleration")
            && val.equal(ruby.qnil()).ok() != Some(true)
        {
            let accel_hash = RHash::try_convert(val)?;
            config.acceleration = Some(parse_acceleration_config(ruby, accel_hash)?);
        }

        if let Some(val) = get_kw(ruby, hash, "html_options")
            && val.equal(ruby.qnil()).ok() != Some(true)
        {
            let html_hash = RHash::try_convert(val)?;
            config.html_options = Some(parse_html_options(ruby, html_hash)?);
        }

        if let Some(val) = get_kw(ruby, hash, "pages")
            && val.equal(ruby.qnil()).ok() != Some(true)
        {
            let pages_hash = RHash::try_convert(val)?;
            config.pages = Some(parse_page_config(ruby, pages_hash)?);
        }

        if let Some(val) = get_kw(ruby, hash, "layout")
            && val.equal(ruby.qnil()).ok() != Some(true)
        {
            let layout_hash = RHash::try_convert(val)?;
            config.layout = Some(parse_layout_detection_config(ruby, layout_hash)?);
        }

        if let Some(val) = get_kw(ruby, hash, "security_limits")
            && val.equal(ruby.qnil()).ok() != Some(true)
        {
            let security_json = ruby_value_to_json(val)?;
            let parsed: kreuzberg::extractors::security::SecurityLimits = serde_json::from_value(security_json)
                .map_err(|e| runtime_error(format!("Invalid security_limits: {}", e)))?;
            config.security_limits = Some(parsed);
        }

        if let Some(val) = get_kw(ruby, hash, "email")
            && val.equal(ruby.qnil()).ok() != Some(true)
        {
            let email_hash = RHash::try_convert(val)?;
            config.email = Some(parse_email_config(ruby, email_hash)?);
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
                    )));
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
                    )));
                }
            };
        }

        if let Some(val) = get_kw(ruby, hash, "extraction_timeout_secs")
            && val.equal(ruby.qnil()).ok() != Some(true)
        {
            let secs = u64::try_convert(val)?;
            config.extraction_timeout_secs = Some(secs);
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
        Some("toml") => toml::from_str(&content)
            .map_err(|e| validation_error(format!("Invalid TOML in config file '{}': {}", path, e)))?,
        Some("yaml") | Some("yml") => serde_yaml_ng::from_str(&content)
            .map_err(|e| validation_error(format!("Invalid YAML in config file '{}': {}", path, e)))?,
        Some("json") => serde_json::from_str(&content)
            .map_err(|e| validation_error(format!("Invalid JSON in config file '{}': {}", path, e)))?,
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

/// Discover extraction config from current directory or parent directories
pub fn config_discover() -> Result<Value, Error> {
    use std::path::PathBuf;

    let ruby = Ruby::get().expect("Ruby not initialized");

    // Search for config files in order of precedence
    let config_files = vec![
        ("kreuzberg.toml", "toml"),
        ("kreuzberg.yaml", "yaml"),
        ("kreuzberg.yml", "yaml"),
        ("kreuzberg.json", "json"),
        (".kreuzbergrc", "json"),
    ];

    // Start from current directory and search up to parent directories
    let mut current_dir: Option<PathBuf> = std::env::current_dir().ok();

    while let Some(dir) = current_dir {
        for (name, format) in &config_files {
            let config_path = dir.join(name);
            if let Ok(content) = fs::read_to_string(&config_path) {
                let json_value: serde_json::Value = match *format {
                    "toml" => toml::from_str(&content)
                        .map_err(|e| validation_error(format!("Invalid TOML in {}: {}", config_path.display(), e)))?,
                    "yaml" => serde_yaml_ng::from_str(&content)
                        .map_err(|e| validation_error(format!("Invalid YAML in {}: {}", config_path.display(), e)))?,
                    "json" => serde_json::from_str(&content)
                        .map_err(|e| validation_error(format!("Invalid JSON in {}: {}", config_path.display(), e)))?,
                    _ => unreachable!(),
                };
                return json_value_to_ruby(&ruby, &json_value);
            }
        }
        // Move to parent directory
        current_dir = dir.parent().map(|p| p.to_path_buf());
    }

    // Return nil if no config found
    Ok(ruby.qnil().as_value())
}
