use super::{capture_retained_hocr_read, flatten_hocr_elements_to_text};
use crate::core::config::{ExtractionConfig, OutputFormat};
use crate::extractors::security::SecurityLimits;
use crate::ocr::error::OcrError;
use crate::ocr::hocr_parser::{
    DictionaryLineFilter, RetainedWordConfidenceStats, parse_hocr_to_internal_document_with_page_offset_and_stats,
    parse_hocr_with_occurrences,
};
use crate::ocr::processor::script_retry;
use crate::ocr::types::TesseractConfig;
use crate::types::formats::RetainedHocrRead;
use crate::types::internal::InternalDocument;
use xberg_tesseract::TesseractAPI;

use super::fragmented_tables::fragmented_hocr_budget;

pub(super) struct Request<'a> {
    pub(super) api: &'a TesseractAPI,
    pub(super) config: &'a TesseractConfig,
    pub(super) extraction_config: Option<&'a ExtractionConfig>,
    pub(super) dictionary_filter: &'a DictionaryLineFilter<'a>,
    pub(super) merged_read: Option<&'a script_retry::Merged>,
    pub(super) tsv_data_for_tables: Option<&'a str>,
    pub(super) security_limits: &'a SecurityLimits,
    pub(super) dimensions: (u32, u32),
    pub(super) capture_region_hocr: bool,
}

#[derive(Default)]
pub(super) struct Output {
    pub(super) raw_content: String,
    pub(super) mime_type: String,
    pub(super) hocr_document: Option<InternalDocument>,
    pub(super) hocr_for_tables: Option<String>,
    pub(super) dictionary_filtered_line_count: usize,
    pub(super) dictionary_kept_numeral_line_count: usize,
    pub(super) retained_hocr_confidence_stats: Option<RetainedWordConfidenceStats>,
    pub(super) retained_hocr_read: Option<RetainedHocrRead>,
}

pub(super) fn read(request: Request<'_>) -> Result<Output, OcrError> {
    let mut output = Output::default();
    let api = request.api;
    let config = request.config;
    let (raw_content, mime_type) = match config.output_format.as_str() {
        "text" => {
            let text = api
                .get_utf8_text()
                .map_err(|e| OcrError::ProcessingFailed(format!("Failed to extract text: {}", e)))?;
            (text, "text/plain".to_string())
        }
        "markdown" => read_markdown(&request, &mut output)?,
        "hocr" => {
            let hocr = api
                .get_hocr_text(0)
                .map_err(|e| OcrError::ProcessingFailed(format!("Failed to extract hOCR: {}", e)))?;
            (hocr, "text/html".to_string())
        }
        "tsv" => {
            let tsv = request
                .tsv_data_for_tables
                .ok_or_else(|| OcrError::ProcessingFailed("TSV data not available".to_string()))?
                .to_owned();
            (tsv, "text/plain".to_string())
        }
        _ => {
            return Err(OcrError::InvalidConfiguration(format!(
                "Unsupported output format: {}",
                config.output_format
            )));
        }
    };

    output.raw_content = raw_content;
    output.mime_type = mime_type;
    if let Some(merged) = request.merged_read {
        // The merged page has no hOCR: its lines come from two engines, so the text is rebuilt
        // from the merged word table and the hOCR-only steps (dictionary filter, table repair
        // from hOCR) do not run for it. ~keep
        output.raw_content = merged.text();
        output.hocr_document = None;
        output.hocr_for_tables = None;
        output.dictionary_filtered_line_count = 0;
        output.dictionary_kept_numeral_line_count = 0;
        output.retained_hocr_confidence_stats = None;
    }

    Ok(output)
}

fn read_markdown(request: &Request<'_>, output: &mut Output) -> Result<(String, String), OcrError> {
    let Request {
        api,
        config,
        extraction_config,
        dictionary_filter,
        merged_read,
        tsv_data_for_tables,
        security_limits,
        dimensions,
        capture_region_hocr,
    } = *request;
    let hocr = api
        .get_hocr_text(0)
        .map_err(|e| OcrError::ProcessingFailed(format!("Failed to extract hOCR: {}", e)))?;

    let (parse_result, occurrences) = if capture_region_hocr && merged_read.is_none() {
        parse_hocr_with_occurrences(&hocr, Some(dictionary_filter), config.page_number)
    } else {
        (
            parse_hocr_to_internal_document_with_page_offset_and_stats(
                &hocr,
                Some(dictionary_filter),
                config.page_number,
            ),
            None,
        )
    };
    if capture_region_hocr {
        tracing::trace!(target: "xberg::ocr::processor::execution", parsed_occurrences = ?occurrences.as_ref().map(Vec::len),
            elements = parse_result.document.elements.len(), "parsed selected hOCR occurrences");
    }
    output.retained_hocr_read =
        occurrences.and_then(|words| capture_retained_hocr_read(&parse_result.document.elements, words, dimensions));
    output.dictionary_filtered_line_count = parse_result.dictionary_filtered_line_count;
    output.dictionary_kept_numeral_line_count = parse_result.dictionary_kept_numeral_line_count;
    if config.enable_table_detection
        && (output.dictionary_filtered_line_count > 0
            || parse_result
                .document
                .elements
                .iter()
                .any(|element| element.text.contains('\n'))
            || tsv_data_for_tables
                .and_then(|tsv| fragmented_hocr_budget(&hocr, tsv, security_limits))
                .is_some())
    {
        output.hocr_for_tables = Some(hocr);
    }
    let content = flatten_hocr_elements_to_text(&parse_result.document.elements);
    output.retained_hocr_confidence_stats = Some(parse_result.retained_word_confidence_stats);
    output.hocr_document = Some(parse_result.document);

    let mime_type = extraction_config
        .map(|c| match c.output_format {
            OutputFormat::Djot => "text/djot",
            _ => "text/markdown",
        })
        .unwrap_or("text/markdown");

    Ok((content, mime_type.to_string()))
}
