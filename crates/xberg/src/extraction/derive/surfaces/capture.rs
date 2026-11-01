use super::{DerivationObserver, ProcessingDocument, RedactionSurfaces};
use crate::types::ExtractedDocument;
use sha2::{Digest, Sha256};
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

#[derive(PartialEq, Eq)]
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
}

#[derive(Default)]
pub(crate) struct SurfaceCapture {
    formatted: Option<Fingerprint>,
    pages: Vec<PageSurface>,
}

impl DerivationObserver for SurfaceCapture {
    fn captures_pages(&self) -> bool {
        true
    }
    fn formatted(&mut self, markdown: bool, text: &str) {
        if markdown {
            self.formatted = Some(Fingerprint::new(text));
        }
    }
    fn page(&mut self, index: usize, number: u32, rendered: &str, plain: String) {
        self.pages.push(PageSurface {
            index,
            number,
            rendered: Fingerprint::new(rendered),
            plain: Some(PlainPageBackup(plain)),
        });
    }
}

pub(crate) struct SurfaceContext {
    identity: Arc<()>,
    plain_document: Fingerprint,
    capture: SurfaceCapture,
}

impl RedactionSurfaces for SurfaceContext {
    fn validate(&self, document: &ExtractedDocument) -> crate::Result<()> {
        let formatted_valid = self.capture.formatted.as_ref().is_none_or(|expected| {
            document
                .formatted_content
                .as_deref()
                .is_some_and(|text| expected.matches(text))
        });
        let pages_valid = self.capture.pages.iter().all(|expected| {
            document
                .pages
                .as_ref()
                .and_then(|pages| pages.get(expected.index))
                .is_some_and(|page| page.page_number == expected.number && expected.rendered.matches(&page.content))
        });
        if self.plain_document.matches(&document.content) && formatted_valid && pages_valid {
            return Ok(());
        }
        Err(crate::XbergError::validation(
            "Generated Markdown provenance changed before redaction",
        ))
    }
    fn formatted_markdown(&self) -> bool {
        self.capture.formatted.is_some()
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
}

impl DerivedResult {
    pub(crate) fn new(document: ExtractedDocument, capture: Option<SurfaceCapture>) -> Self {
        let capture = capture.filter(|capture| capture.formatted.is_some() || !capture.pages.is_empty());
        let identity = capture.as_ref().map(|_| Arc::new(()));
        let context = capture
            .zip(identity.as_ref())
            .map(|(capture, identity)| SurfaceContext {
                identity: Arc::clone(identity),
                plain_document: Fingerprint::new(&document.content),
                capture,
            });
        Self {
            document,
            identity,
            context,
        }
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

#[cfg(test)]
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
