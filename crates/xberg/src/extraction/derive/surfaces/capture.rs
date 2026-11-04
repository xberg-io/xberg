use super::{DerivationObserver, HtmlPageBody, ProcessingDocument, RedactionSurfaces};
use crate::types::ExtractedDocument;
use sha2::{Digest, Sha256};
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

#[derive(Clone, PartialEq, Eq)]
struct Fingerprint {
    length: usize,
    digest: [u8; 32],
}

impl Fingerprint {
    fn new(text: &str) -> Self {
        let mut digest = [0; 32];
        digest.copy_from_slice(&Sha256::digest(text.as_bytes()));
        Self {
            length: text.len(),
            digest,
        }
    }
    fn matches(&self, text: &str) -> bool {
        *self == Self::new(text)
    }
}

struct PlainPageBackup(String);

struct PageSurface {
    index: usize,
    number: u32,
    rendered: Fingerprint,
    plain: Option<PlainPageBackup>,
    tag_exemptions: bool,
    literal: bool,
}

struct HtmlSurface {
    rendered: Fingerprint,
    verified: bool,
}

struct HtmlOutputSeal {
    plain: Fingerprint,
    formatted: Fingerprint,
    pages: Vec<Fingerprint>,
}

struct HtmlPage {
    index: usize,
    rendered: Fingerprint,
    body: HtmlPageBody,
}

#[derive(Default)]
pub(crate) struct SurfaceCapture {
    formatted: Option<Fingerprint>,
    html: Option<HtmlSurface>,
    html_pages: Vec<HtmlPage>,
    pages: Vec<PageSurface>,
    tag_exemptions: bool,
}

impl DerivationObserver for SurfaceCapture {
    fn captures_pages(&self) -> bool {
        true
    }
    fn captures_html(&self) -> bool {
        true
    }

    fn formatted_html(&mut self, rendered: &str, verified: bool) {
        self.html = Some(HtmlSurface {
            rendered: Fingerprint::new(rendered),
            verified,
        });
    }

    fn page_html(&mut self, index: usize, rendered: &str, body: HtmlPageBody) {
        self.html_pages.push(HtmlPage {
            index,
            rendered: Fingerprint::new(rendered),
            body,
        });
    }

    fn formatted(&mut self, markdown: bool, text: &str) {
        if markdown {
            self.formatted = Some(Fingerprint::new(text));
        }
    }
    fn formatted_from_source(&mut self, markdown: bool, text: &str, tag_exemptions: bool) {
        self.formatted(markdown, text);
        self.tag_exemptions = tag_exemptions;
    }
    fn page_from_source(&mut self, index: usize, number: u32, rendered: &str, plain: String, tag_exemptions: bool) {
        self.page(index, number, rendered, plain);
        if let Some(page) = self.pages.last_mut() {
            page.tag_exemptions = tag_exemptions;
        }
    }
    fn page(&mut self, index: usize, number: u32, rendered: &str, plain: String) {
        self.pages.push(PageSurface {
            index,
            number,
            rendered: Fingerprint::new(rendered),
            plain: Some(PlainPageBackup(plain)),
            tag_exemptions: false,
            literal: false,
        });
    }
}

pub(crate) struct SurfaceContext {
    identity: Arc<()>,
    plain_document: Fingerprint,
    capture: SurfaceCapture,
    formatted_invalidated: bool,
    formatted_literal: Option<String>,
    prefer_current_plain: bool,
    page_numbers: Option<Vec<u32>>,
    processor_changed_page_shape: bool,
    html_mutated: bool,
    html_output_seal: Option<HtmlOutputSeal>,
}

impl SurfaceContext {
    fn html_preimage_matches(&self, document: &ExtractedDocument, html: &HtmlSurface) -> bool {
        let pages = document.pages.as_deref().unwrap_or_default();
        let pages_verified = self.page_shape_matches(document)
            && pages.len() == self.capture.html_pages.len()
            && pages
                .iter()
                .zip(&self.capture.html_pages)
                .enumerate()
                .all(|(index, (page, proof))| {
                    proof.index == index
                        && !matches!(proof.body, HtmlPageBody::Unverified)
                        && proof.rendered.matches(&page.content)
                        && page.native_content.is_none()
                });
        html.verified
            && self.plain_document.matches(&document.content)
            && document
                .formatted_content
                .as_deref()
                .is_some_and(|text| html.rendered.matches(text))
            && pages_verified
    }

    fn validate_html(&self, document: &ExtractedDocument, html: &HtmlSurface) -> crate::Result<()> {
        if !self.html_mutated && self.html_preimage_matches(document, html) {
            return Ok(());
        }
        Err(crate::XbergError::validation(
            "HTML redaction requires unchanged renderer output and complete verified plain text",
        ))
    }
}

impl RedactionSurfaces for SurfaceContext {
    fn validate(&self, document: &ExtractedDocument) -> crate::Result<()> {
        if let Some(html) = &self.capture.html {
            return self.validate_html(document, html);
        }
        if self.processor_changed_page_shape && !self.page_shape_matches(document) {
            return Err(crate::XbergError::validation(
                "A post-processor changed the pages of the document, and redaction cannot continue",
            ));
        }
        let formatted_valid = match &self.capture.formatted {
            Some(expected) => document
                .formatted_content
                .as_deref()
                .is_some_and(|text| expected.matches(text)),
            None => !self.formatted_invalidated || document.formatted_content.is_none(),
        };
        let pages_valid = self.pages_match(document);
        if self.plain_document.matches(&document.content) && formatted_valid && pages_valid {
            return Ok(());
        }
        Err(crate::XbergError::validation(
            "Generated Markdown provenance changed before redaction",
        ))
    }
    fn formatted_html(&self) -> bool {
        self.capture.html.is_some()
    }

    fn page_html_backup(&self, index: usize) -> Option<&str> {
        match &self.capture.html_pages.get(index)?.body {
            HtmlPageBody::Rendered(plain) => Some(plain),
            HtmlPageBody::Literal | HtmlPageBody::Unverified => None,
        }
    }

    fn formatted_markdown(&self) -> bool {
        self.formatted_invalidated || self.capture.formatted.is_some()
    }
    fn discard_formatted(&self) -> bool {
        self.formatted_invalidated
    }
    fn formatted_tag_exemptions(&self) -> bool {
        self.capture.tag_exemptions
    }
    fn formatted_literal_backup(&self) -> Option<&str> {
        self.formatted_literal.as_deref()
    }
    fn page_tag_exemptions(&self, index: usize) -> bool {
        self.capture
            .pages
            .binary_search_by_key(&index, |page| page.index)
            .ok()
            .is_some_and(|position| self.capture.pages[position].tag_exemptions)
    }
    fn page_is_literal(&self, index: usize) -> bool {
        self.capture
            .pages
            .binary_search_by_key(&index, |page| page.index)
            .ok()
            .is_some_and(|position| self.capture.pages[position].literal)
    }
    fn page_markdown(&self, index: usize) -> bool {
        self.capture
            .pages
            .binary_search_by_key(&index, |page| page.index)
            .is_ok()
    }
    fn page_backup(&self, index: usize) -> Option<&str> {
        self.capture
            .pages
            .binary_search_by_key(&index, |page| page.index)
            .ok()
            .and_then(|position| self.capture.pages[position].plain.as_ref())
            .map(|plain| plain.0.as_str())
    }
}

pub(crate) struct DerivedResult {
    document: ExtractedDocument,
    identity: Option<Arc<()>>,
    context: Option<SurfaceContext>,
    witness_active: bool,
}

impl DerivedResult {
    pub(crate) fn new(document: ExtractedDocument, capture: Option<SurfaceCapture>) -> Self {
        let capture = capture
            .filter(|capture| capture.formatted.is_some() || capture.html.is_some() || !capture.pages.is_empty());
        let identity = capture.as_ref().map(|_| Arc::new(()));
        let context = capture
            .zip(identity.as_ref())
            .map(|(capture, identity)| SurfaceContext {
                identity: Arc::clone(identity),
                plain_document: Fingerprint::new(&document.content),
                capture,
                formatted_invalidated: false,
                formatted_literal: None,
                prefer_current_plain: false,
                processor_changed_page_shape: false,
                html_mutated: false,
                html_output_seal: None,
                page_numbers: document
                    .pages
                    .as_ref()
                    .map(|pages| pages.iter().map(|page| page.page_number).collect()),
            });
        Self {
            document,
            identity,
            witness_active: context.is_some(),
            context,
        }
    }
    #[cfg(feature = "html")]
    pub(crate) fn set_styled_html(&mut self, rendered: String, verified_plain: Option<&str>) {
        if let Some(context) = &mut self.context {
            let verified = verified_plain == Some(self.document.content.as_str());
            context.capture.formatted_html(&rendered, verified);
        }
        self.document.formatted_content = Some(rendered);
    }

    pub(crate) fn into_output(
        self,
        requested: crate::core::config::OutputFormat,
    ) -> (ExtractedDocument, crate::core::config::OutputFormat) {
        let plain = matches!(requested, crate::core::config::OutputFormat::Markdown)
            && self
                .context
                .as_ref()
                .is_some_and(|context| context.formatted_markdown())
            && self.document.formatted_content.is_none();
        let format = if plain {
            crate::core::config::OutputFormat::Plain
        } else {
            requested
        };
        (self.document, format)
    }
}

impl Deref for DerivedResult {
    type Target = ExtractedDocument;
    fn deref(&self) -> &Self::Target {
        &self.document
    }
}
impl DerefMut for DerivedResult {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.document
    }
}
impl ProcessingDocument for DerivedResult {
    fn document(&self) -> &ExtractedDocument {
        &self.document
    }
    fn document_mut(&mut self) -> &mut ExtractedDocument {
        &mut self.document
    }
    fn processor_witness(&self) -> Option<ProcessorWitness> {
        self.begin_processor_mutation()
    }
    fn observe_processor(&mut self, witness: Option<ProcessorWitness>) {
        self.finish_processor_mutation(witness);
    }
    fn seal_processor_witness(&mut self) {
        self.witness_active = false;
    }
    fn seal_redacted_html(&mut self) -> crate::Result<()> {
        let Some(context) = &mut self.context else {
            return Ok(());
        };
        if context.capture.html.is_none() || self.document.redaction_report.is_none() {
            return Ok(());
        }
        let formatted = self
            .document
            .formatted_content
            .as_deref()
            .ok_or_else(html_output_changed)?;
        context.html_output_seal = Some(HtmlOutputSeal {
            plain: Fingerprint::new(&self.document.content),
            formatted: Fingerprint::new(formatted),
            pages: self
                .document
                .pages
                .as_deref()
                .unwrap_or_default()
                .iter()
                .map(|page| Fingerprint::new(&page.content))
                .collect(),
        });
        Ok(())
    }

    fn validate_sealed_html(&self, config: &crate::ExtractionConfig) -> crate::Result<()> {
        let Some(context) = &self.context else {
            return Ok(());
        };
        let Some(seal) = &context.html_output_seal else {
            return Ok(());
        };
        let pages = self.document.pages.as_deref().unwrap_or_default();
        let unchanged = seal.plain.matches(&self.document.content)
            && self
                .document
                .formatted_content
                .as_deref()
                .is_some_and(|html| seal.formatted.matches(html))
            && context.page_shape_matches(&self.document)
            && pages.len() == seal.pages.len()
            && pages
                .iter()
                .zip(&seal.pages)
                .all(|(page, hash)| page.native_content.is_none() && hash.matches(&page.content));
        if !unchanged {
            return Err(html_output_changed());
        }
        let limit = config.security_limits.as_ref().map_or_else(
            || crate::extractors::security::SecurityLimits::default().max_iterations,
            |limits| limits.max_iterations,
        );
        crate::text::redaction::validate_nested_html(&self.document, limit).map_err(|error| crate::XbergError::Plugin {
            plugin_name: "redaction".into(),
            message: error.to_string(),
        })
    }

    fn redaction_parts(&mut self) -> crate::Result<(&mut ExtractedDocument, Option<&SurfaceContext>)> {
        if self.context.as_ref().is_some_and(|context| {
            !self
                .identity
                .as_ref()
                .is_some_and(|identity| Arc::ptr_eq(identity, &context.identity))
        }) {
            return Err(crate::XbergError::Plugin {
                plugin_name: "redaction".into(),
                message: "Generated Markdown belongs to a different result".into(),
            });
        }
        Ok((&mut self.document, self.context.as_ref()))
    }
}

fn html_output_changed() -> crate::XbergError {
    crate::XbergError::Plugin {
        plugin_name: "redaction".into(),
        message: "A post-processor changed verified HTML after redaction".into(),
    }
}

#[cfg(all(test, feature = "tokio-runtime"))]
mod tests;

pub(crate) fn derive_with_surfaces(
    doc: crate::types::internal::InternalDocument,
    include_document_structure: bool,
    output_format: crate::core::config::OutputFormat,
    capture: bool,
) -> DerivedResult {
    if !capture {
        return DerivedResult::new(
            super::super::derive_extraction_result(doc, include_document_structure, output_format),
            None,
        );
    }
    let mut observer = SurfaceCapture::default();
    let document = super::super::derive(doc, include_document_structure, output_format, true, &mut observer);
    DerivedResult::new(document, Some(observer))
}

mod processor;
pub(crate) use processor::ProcessorWitness;

#[cfg(all(test, feature = "tokio-runtime", feature = "office"))]
mod coverage_tests;
