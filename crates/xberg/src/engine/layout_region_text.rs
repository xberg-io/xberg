//! Opt-in region text for explicitly selected native PDF pages.

use std::{ops::Range, sync::Arc};

use parking_lot::Mutex;

use crate::core::config::{ExtractionConfig, ExtractionResult};
use crate::types::{ExtractionMethod, LayoutRegion};

/// An extraction with source-proven text for selected layout regions.
#[derive(Debug, serde::Serialize)]
pub struct LayoutRegionTextExtraction {
    /// The ordinary extraction result.
    pub extraction: ExtractionResult,
    /// Proven regions in document, page and reading order.
    pub regions: Vec<LayoutRegionText>,
}

/// Text owned by a detected native PDF layout region.
#[derive(Debug, serde::Serialize)]
pub struct LayoutRegionText {
    /// One-based page number within the extracted PDF.
    pub page_number: u32,
    /// Class, confidence, and bounds in PDF points.
    pub region: LayoutRegion,
    /// Verbatim text from the returned page.
    pub text: String,
}

pub(crate) type RegionTextCollector = Arc<Mutex<Vec<PageRegionRanges>>>;

#[derive(Debug)]
pub(crate) struct PageRegionRanges {
    pub(crate) page_number: u32,
    pub(crate) fingerprint: blake3::Hash,
    pub(crate) regions: Vec<(LayoutRegion, Range<usize>)>,
}

pub(crate) fn capture_allowed(config: &ExtractionConfig) -> bool {
    #[cfg(feature = "redaction")]
    if crate::text::redaction::external::external_redaction_is_scoped() {
        return false;
    }
    config.redaction.is_none()
}

pub(super) fn finish(extraction: ExtractionResult, pending: Vec<PageRegionRanges>) -> LayoutRegionTextExtraction {
    let mut regions = Vec::new();
    if let [doc] = extraction.results.as_slice()
        && doc.extraction_method == Some(ExtractionMethod::Native)
        && doc.redaction_report.is_none()
        && let Some(pages) = &doc.pages
    {
        for evidence in pending {
            let mut matches = pages.iter().filter(|page| page.page_number == evidence.page_number);
            let Some(page) = matches.next().filter(|_| matches.next().is_none()) else {
                continue;
            };
            if blake3::hash(page.content.as_bytes()) != evidence.fingerprint {
                continue;
            }
            regions.extend(evidence.regions.into_iter().filter_map(|(region, range)| {
                let text = page.content.get(range)?;
                (!text.is_empty()).then(|| LayoutRegionText {
                    page_number: page.page_number,
                    region,
                    text: text.to_owned(),
                })
            }));
        }
    }
    LayoutRegionTextExtraction { extraction, regions }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ExtractedDocument, types::PageContent};

    #[cfg(feature = "redaction")]
    #[tokio::test]
    async fn region_capture_is_disabled_even_for_empty_redaction_requests() {
        use crate::core::config::RedactionOffsetEncoding;
        use crate::text::redaction::external::{ExternalRedactionRequest, scope_external_redaction};

        let ordinary = ExtractionConfig::default();
        assert!(capture_allowed(&ordinary));
        let configured = ExtractionConfig {
            redaction: Some(Default::default()),
            ..Default::default()
        };
        assert!(!capture_allowed(&configured));
        let request = ExternalRedactionRequest::new(Vec::new(), RedactionOffsetEncoding::UnicodeCodePoints, 1, false);
        scope_external_redaction(request, async {
            assert!(!capture_allowed(&ordinary));
        })
        .await;
        assert!(capture_allowed(&ordinary));
    }

    #[test]
    fn region_text_requires_same_page_unmodified_utf8_and_native_provenance() {
        let page: PageContent =
            serde_json::from_value(serde_json::json!({ "page_number": 2, "content": "élan 東京" })).unwrap();
        let doc = ExtractedDocument {
            extraction_method: Some(ExtractionMethod::Native),
            pages: Some(vec![page.clone()]),
            ..Default::default()
        };
        let claim = |page_number, range| PageRegionRanges {
            page_number,
            fingerprint: blake3::hash(page.content.as_bytes()),
            regions: vec![(LayoutRegion::default(), range)],
        };
        let emit = |doc, claims| finish(ExtractionResult::single(doc), claims).regions;
        let result = emit(doc.clone(), vec![claim(1, 0..5), claim(2, 1..5), claim(2, 0..5)]);
        assert_eq!(result.len(), 1);
        assert_eq!((result[0].page_number, result[0].text.as_str()), (2, "élan"));

        let mut modified = doc.clone();
        modified.pages.as_mut().unwrap()[0].content.push('!');
        let mut ambiguous = doc.clone();
        ambiguous.pages.as_mut().unwrap().push(page.clone());
        let mut mixed = doc.clone();
        mixed.extraction_method = Some(ExtractionMethod::Mixed);
        let mut redacted = doc;
        redacted.redaction_report = Some(crate::types::RedactionReport {
            findings: Vec::new(),
            total_redacted: 0,
        });
        for unproved in [modified, ambiguous, mixed, redacted] {
            assert!(emit(unproved, vec![claim(2, 0..5)]).is_empty());
        }
    }
}
