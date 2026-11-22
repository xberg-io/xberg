//! Capability warnings for the pdfium engine.
//!
//! Each [`Gap`] names one configuration option this engine cannot honour yet and the test that
//! says whether the caller asked for it. A warning is emitted only when its test holds, so
//! landing support for an option means deleting its entry from [`GAPS`].

use std::borrow::Cow;

use crate::core::config::{ExtractionConfig, OcrStrategy, OutputFormat, PdfConfig};
use crate::types::ProcessingWarning;

pub(super) const WARNING_SOURCE: &str = "pdf_backend_pdfium";
const TIME_LIMIT_SOURCE: &str = "pdf_time_limit";

struct Gap {
    requested: fn(&ExtractionConfig) -> bool,
    message: &'static str,
}

fn pdf_option(config: &ExtractionConfig, test: impl Fn(&PdfConfig) -> bool) -> bool {
    config.pdf_options.as_ref().is_some_and(test)
}

fn content_filter(config: &ExtractionConfig, test: impl Fn(&crate::core::config::ContentFilterConfig) -> bool) -> bool {
    config.content_filter.as_ref().is_some_and(test)
}

// `extract_tables` and `extract_form_fields` default to `true` and a config cannot tell a default
// from a request, so they warn whenever the effective value is on. ~keep
const GAPS: &[Gap] = &[
    Gap {
        requested: |config| config.force_ocr,
        message: "the pdfium backend cannot apply force_ocr; no OCR was run and the page text layer was used",
    },
    Gap {
        requested: |config| config.force_ocr_pages.as_ref().is_some_and(|pages| !pages.is_empty()),
        message: "the pdfium backend cannot apply force_ocr_pages; no OCR was run and the page text layer was used",
    },
    Gap {
        requested: |config| matches!(config.ocr_strategy, OcrStrategy::ScannedPages { .. }),
        message: "the pdfium backend cannot apply ocr_strategy scanned_pages; no OCR was run",
    },
    Gap {
        requested: |config| pdf_option(config, |pdf| pdf.ocr_inline_images),
        message: "the pdfium backend cannot apply pdf_options.ocr_inline_images; images were not read",
    },
    Gap {
        requested: |config| pdf_option(config, |pdf| pdf.extract_images),
        message: "the pdfium backend cannot apply pdf_options.extract_images; no images were extracted",
    },
    Gap {
        requested: |config| config.pdf_options.as_ref().is_none_or(|pdf| pdf.extract_tables),
        message: "the pdfium backend cannot apply pdf_options.extract_tables; no tables were detected",
    },
    Gap {
        requested: |config| pdf_option(config, |pdf| pdf.extract_annotations),
        message: "the pdfium backend cannot apply pdf_options.extract_annotations; no annotations were extracted",
    },
    Gap {
        requested: |config| config.pdf_options.as_ref().is_none_or(|pdf| pdf.extract_form_fields),
        message: "the pdfium backend cannot apply pdf_options.extract_form_fields; no form fields were read",
    },
    Gap {
        requested: |config| pdf_option(config, |pdf| pdf.hierarchy.is_some()),
        message: "the pdfium backend cannot apply pdf_options.hierarchy; no heading hierarchy was detected",
    },
    Gap {
        requested: |config| pdf_option(config, |pdf| pdf.reading_order),
        message: "the pdfium backend cannot apply pdf_options.reading_order; text follows pdfium's own order",
    },
    Gap {
        requested: |config| pdf_option(config, |pdf| pdf.extract_vector_geometry),
        message: "the pdfium backend cannot apply pdf_options.extract_vector_geometry; no geometry was extracted",
    },
    Gap {
        requested: |config| {
            pdf_option(config, |pdf| {
                pdf.plan_regions.as_ref().is_some_and(|regions| regions.enabled)
            })
        },
        message: "the pdfium backend cannot apply pdf_options.plan_regions; no regions were recovered",
    },
    Gap {
        requested: |config| config.include_document_structure,
        message: "the pdfium backend cannot apply include_document_structure; content is flat paragraphs",
    },
    Gap {
        requested: |config| config.output_format != OutputFormat::Plain,
        message: concat!(
            "the pdfium backend produces plain paragraphs only; the requested output_format is applied to them ",
            "without headings, lists or tables"
        ),
    },
    Gap {
        requested: |config| content_filter(config, |filter| filter.strip_repeating_text),
        message: "the pdfium backend cannot apply ContentFilterConfig.strip_repeating_text; repeated text was preserved",
    },
    Gap {
        requested: |config| content_filter(config, |filter| filter.include_footnotes),
        message: concat!(
            "the pdfium backend cannot apply ContentFilterConfig.include_footnotes; ",
            "footnote inclusion follows Pdfium's plain-text extraction"
        ),
    },
    Gap {
        requested: |config| content_filter(config, |filter| filter.include_watermarks),
        message: concat!(
            "the pdfium backend cannot apply ContentFilterConfig.include_watermarks; ",
            "watermark inclusion follows Pdfium's plain-text extraction"
        ),
    },
];

/// One warning for each option in `config` that this engine cannot honour yet.
pub(super) fn unsupported_option_warnings(config: &ExtractionConfig) -> Vec<ProcessingWarning> {
    GAPS.iter()
        .filter(|gap| (gap.requested)(config))
        .map(|gap| ProcessingWarning {
            source: Cow::Borrowed(WARNING_SOURCE),
            message: Cow::Borrowed(gap.message),
        })
        .collect()
}

/// Warning for pages that came back without text when OCR was not disabled: a native extraction
/// would have routed them to OCR, this engine has no OCR fallback.
pub(super) fn missing_ocr_fallback_warning(blank_pages: &[u32], ocr_disabled: bool) -> Option<ProcessingWarning> {
    if blank_pages.is_empty() || ocr_disabled {
        return None;
    }
    let pages = blank_pages.iter().map(u32::to_string).collect::<Vec<_>>().join(", ");
    Some(ProcessingWarning {
        source: Cow::Borrowed(WARNING_SOURCE),
        message: Cow::Owned(format!(
            "the pdfium backend has no OCR fallback; page(s) {pages} have no text layer and were returned empty"
        )),
    })
}

/// Warning for a page loop the extraction time limit stopped before `first_unread_page`.
pub(super) fn time_limit_warning(first_unread_page: u32, page_count: u32) -> ProcessingWarning {
    ProcessingWarning {
        source: Cow::Borrowed(TIME_LIMIT_SOURCE),
        message: Cow::Owned(format!(
            "Stopped after extraction_timeout_secs was reached: pages {first_unread_page} to {page_count} were not read."
        )),
    }
}

/// Warning for a margin filter that could not be placed on a page (no usable page box).
pub(super) fn margin_filter_unavailable_warning(pages: &[u32]) -> ProcessingWarning {
    let pages = pages.iter().map(u32::to_string).collect::<Vec<_>>().join(", ");
    ProcessingWarning {
        source: Cow::Borrowed(WARNING_SOURCE),
        message: Cow::Owned(format!(
            "the page margin filter could not be applied to page(s) {pages}: no usable page box; \
             the whole page text was kept"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::ContentFilterConfig;

    fn quiet_pdf_options() -> PdfConfig {
        PdfConfig {
            extract_tables: false,
            extract_form_fields: false,
            ..Default::default()
        }
    }

    fn quiet_config() -> ExtractionConfig {
        ExtractionConfig {
            pdf_options: Some(quiet_pdf_options()),
            ..Default::default()
        }
    }

    fn messages(config: &ExtractionConfig) -> Vec<String> {
        unsupported_option_warnings(config)
            .into_iter()
            .map(|warning| warning.message.into_owned())
            .collect()
    }

    #[test]
    fn should_not_warn_when_nothing_unsupported_is_requested() {
        assert!(messages(&quiet_config()).is_empty());
    }

    #[test]
    fn should_warn_about_the_default_enabled_table_and_form_options_only() {
        let warnings = messages(&ExtractionConfig::default());
        assert_eq!(warnings.len(), 2, "{warnings:?}");
        assert!(warnings[0].contains("extract_tables"));
        assert!(warnings[1].contains("extract_form_fields"));
    }

    #[test]
    fn should_warn_for_exactly_the_option_that_is_requested() {
        type Mutate = fn(&mut ExtractionConfig);
        let cases: [(&str, Mutate); 15] = [
            ("force_ocr;", |c| c.force_ocr = true),
            ("force_ocr_pages", |c| c.force_ocr_pages = Some(vec![1])),
            ("scanned_pages", |c| {
                c.ocr_strategy = OcrStrategy::ScannedPages { min_confidence: 0.5 }
            }),
            ("ocr_inline_images", |c| {
                c.pdf_options.as_mut().unwrap().ocr_inline_images = true
            }),
            ("extract_images", |c| {
                c.pdf_options.as_mut().unwrap().extract_images = true
            }),
            ("extract_tables", |c| {
                c.pdf_options.as_mut().unwrap().extract_tables = true
            }),
            ("extract_annotations", |c| {
                c.pdf_options.as_mut().unwrap().extract_annotations = true
            }),
            ("extract_form_fields", |c| {
                c.pdf_options.as_mut().unwrap().extract_form_fields = true
            }),
            ("hierarchy", |c| {
                c.pdf_options.as_mut().unwrap().hierarchy = Some(Default::default())
            }),
            ("reading_order", |c| {
                c.pdf_options.as_mut().unwrap().reading_order = true
            }),
            ("extract_vector_geometry", |c| {
                c.pdf_options.as_mut().unwrap().extract_vector_geometry = true
            }),
            ("include_document_structure", |c| c.include_document_structure = true),
            ("output_format", |c| c.output_format = OutputFormat::Markdown),
            ("strip_repeating_text", |c| {
                c.content_filter = Some(ContentFilterConfig {
                    strip_repeating_text: true,
                    ..Default::default()
                })
            }),
            ("include_footnotes", |c| {
                c.content_filter = Some(ContentFilterConfig {
                    strip_repeating_text: false,
                    include_footnotes: true,
                    ..Default::default()
                })
            }),
        ];

        for (needle, mutate) in cases {
            let mut config = quiet_config();
            mutate(&mut config);
            let warnings = messages(&config);
            assert_eq!(warnings.len(), 1, "{needle}: {warnings:?}");
            assert!(warnings[0].contains(needle), "{needle}: {warnings:?}");
        }
    }

    #[test]
    fn should_warn_about_watermarks_and_tag_every_warning_with_the_engine_source() {
        let mut config = quiet_config();
        config.content_filter = Some(ContentFilterConfig {
            strip_repeating_text: false,
            include_watermarks: true,
            ..Default::default()
        });
        let warnings = unsupported_option_warnings(&config);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].source, WARNING_SOURCE);
        assert!(warnings[0].message.contains("include_watermarks"));
    }

    #[test]
    fn should_not_warn_about_ocr_strategy_auto_or_disable_ocr() {
        let mut config = quiet_config();
        config.disable_ocr = true;
        assert!(messages(&config).is_empty());
    }

    #[test]
    fn should_warn_when_an_explicit_default_content_filter_requests_repeating_text_removal() {
        let mut config = quiet_config();
        config.content_filter = Some(ContentFilterConfig::default());
        let warnings = messages(&config);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("strip_repeating_text"));
    }

    #[test]
    fn should_report_pages_without_text_unless_ocr_is_disabled() {
        let warning = missing_ocr_fallback_warning(&[2, 5], false).expect("blank pages warn");
        assert!(warning.message.contains("page(s) 2, 5"));

        assert!(missing_ocr_fallback_warning(&[], false).is_none());
        assert!(missing_ocr_fallback_warning(&[2], true).is_none());
    }

    #[test]
    fn should_name_the_first_unread_page_in_the_time_limit_warning() {
        let warning = time_limit_warning(4, 9);
        assert_eq!(warning.source, "pdf_time_limit");
        assert!(warning.message.contains("pages 4 to 9"));
    }

    #[test]
    fn should_list_pages_in_the_margin_filter_warning() {
        let warning = margin_filter_unavailable_warning(&[1, 3]);
        assert!(warning.message.contains("page(s) 1, 3"));
    }
}
