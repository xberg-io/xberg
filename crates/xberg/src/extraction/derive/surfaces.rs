use crate::types::ExtractedDocument;

pub(crate) enum HtmlPageBody {
    Rendered(#[cfg(feature = "redaction")] String),
    Literal,
    Unverified,
}

pub(crate) trait DerivationObserver {
    fn captures_pages(&self) -> bool {
        false
    }
    fn captures_html(&self) -> bool {
        false
    }
    fn formatted_html(&mut self, _rendered: &str, _verified: bool) {}
    fn page_html(&mut self, _index: usize, _rendered: &str, _body: HtmlPageBody) {}
    fn formatted(&mut self, _markdown: bool, _text: &str) {}
    fn page(&mut self, _index: usize, _number: u32, _rendered: &str, _plain: String) {}
    fn formatted_from_source(&mut self, markdown: bool, text: &str, _tag_exemptions: bool) {
        self.formatted(markdown, text);
    }
    fn page_from_source(&mut self, index: usize, number: u32, rendered: &str, plain: String, _tag_exemptions: bool) {
        self.page(index, number, rendered, plain);
    }
}

impl DerivationObserver for () {}

pub(crate) trait ProcessingDocument: Send {
    fn document(&self) -> &ExtractedDocument;
    fn document_mut(&mut self) -> &mut ExtractedDocument;

    #[cfg(feature = "redaction")]
    fn processor_witness(&self) -> Option<ProcessorWitness> {
        None
    }
    #[cfg(feature = "redaction")]
    fn observe_processor(&mut self, _witness: Option<ProcessorWitness>) {}
    #[cfg(feature = "redaction")]
    fn seal_processor_witness(&mut self) {}

    #[cfg(feature = "redaction")]
    fn seal_redacted_html(&mut self) -> crate::Result<()> {
        Ok(())
    }

    #[cfg(feature = "redaction")]
    fn validate_sealed_html(&self, _config: &crate::ExtractionConfig) -> crate::Result<()> {
        Ok(())
    }

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
    fn formatted_html(&self) -> bool {
        false
    }
    fn page_html_backup(&self, _index: usize) -> Option<&str> {
        None
    }
    fn formatted_markdown(&self) -> bool {
        false
    }
    fn discard_formatted(&self) -> bool {
        false
    }
    fn page_markdown(&self, _index: usize) -> bool {
        false
    }
    fn page_backup(&self, _index: usize) -> Option<&str> {
        None
    }
    fn formatted_tag_exemptions(&self) -> bool {
        false
    }
    fn formatted_literal_backup(&self) -> Option<&str> {
        None
    }
    fn page_tag_exemptions(&self, _index: usize) -> bool {
        false
    }
    fn page_is_literal(&self, _index: usize) -> bool {
        false
    }
}
#[cfg(feature = "redaction")]
impl RedactionSurfaces for () {}

#[cfg(feature = "redaction")]
mod capture;
#[cfg(all(test, feature = "redaction", feature = "tokio-runtime"))]
pub(crate) use capture::DerivedResult;
#[cfg(feature = "redaction")]
pub(crate) use capture::{ProcessorWitness, SurfaceContext, derive_with_surfaces};
