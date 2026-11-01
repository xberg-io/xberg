use crate::types::ExtractedDocument;

pub(crate) trait DerivationObserver {
    fn captures_pages(&self) -> bool {
        false
    }
    fn formatted(&mut self, _markdown: bool, _text: &str) {}
    fn page(&mut self, _index: usize, _number: u32, _rendered: &str, _plain: String) {}
}

impl DerivationObserver for () {}

pub(crate) trait ProcessingDocument: Send {
    fn document(&self) -> &ExtractedDocument;
    fn document_mut(&mut self) -> &mut ExtractedDocument;

    #[cfg(feature = "redaction")]
    fn redaction_parts(&mut self) -> crate::Result<(&mut ExtractedDocument, Option<&SurfaceContext>)> {
        Ok((self.document_mut(), None))
    }
}

impl ProcessingDocument for ExtractedDocument {
    fn document(&self) -> &ExtractedDocument {
        self
    }
    fn document_mut(&mut self) -> &mut ExtractedDocument {
        self
    }
}

#[cfg(feature = "redaction")]
pub(crate) trait RedactionSurfaces: Send + Sync {
    fn validate(&self, _document: &ExtractedDocument) -> crate::Result<()> {
        Ok(())
    }
    fn formatted_markdown(&self) -> bool {
        false
    }
    fn page_markdown(&self, _index: usize) -> bool {
        false
    }
    fn page_backup(&self, _index: usize) -> Option<&str> {
        None
    }
}
#[cfg(feature = "redaction")]
impl RedactionSurfaces for () {}

#[cfg(feature = "redaction")]
mod capture;
#[cfg(all(test, feature = "redaction", feature = "tokio-runtime"))]
pub(crate) use capture::DerivedResult;
#[cfg(feature = "redaction")]
pub(crate) use capture::{SurfaceContext, derive_with_surfaces};
