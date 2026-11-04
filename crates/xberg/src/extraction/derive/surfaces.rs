use crate::types::ExtractedDocument;
use std::ops::Range;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum VerifiedMarkup {
    Html,
    Djot,
}

impl VerifiedMarkup {
    #[cfg(feature = "redaction")]
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Html => "HTML",
            Self::Djot => "Djot",
        }
    }
}

pub(crate) enum MarkupPageBody {
    Rendered(#[cfg(feature = "redaction")] String),
    Literal,
    Unverified,
}

pub(crate) trait DerivationObserver {
    fn captures_pages(&self) -> bool {
        false
    }
    fn captures_verified_markup(&self) -> bool {
        false
    }
    fn formatted_html(&mut self, rendered: &str, verified: bool) {
        self.formatted_markup(VerifiedMarkup::Html, rendered, verified);
    }
    fn formatted_markup(&mut self, _kind: VerifiedMarkup, _rendered: &str, _verified: bool) {}
    fn page_markup(&mut self, _index: usize, _rendered: &str, _body: MarkupPageBody) {}
    fn formatted(&mut self, _markdown: bool, _text: &str) {}
    fn markdown_tag_budget(&mut self) -> Option<&mut usize> {
        None
    }
    fn formatted_tag_ranges(&mut self, _ranges: Option<Vec<Range<usize>>>) {}
    fn page_tag_ranges(&mut self, _ranges: Option<Vec<Range<usize>>>) {}
    fn page(&mut self, _index: usize, _number: u32, _rendered: &str, _plain: String) {}
    fn formatted_from_source(&mut self, markdown: bool, text: &str, _tag_exemptions: bool) {
        self.formatted(markdown, text);
    }
    fn page_from_source(&mut self, index: usize, number: u32, rendered: &str, plain: String, _tag_exemptions: bool) {
        self.page(index, number, rendered, plain);
    }
}

impl DerivationObserver for () {}

#[cfg(feature = "redaction")]
pub(crate) fn has_structured_html_djot_prerender(doc: &crate::types::internal::InternalDocument) -> bool {
    use crate::types::internal::ElementKind;

    // The built-in HTML converter wraps its opaque Djot in a single verbatim paragraph
    // when it cannot map a real body. Keep that output unverified. ~keep
    doc.metadata.output_format.as_deref() == Some("djot")
        && doc.pre_rendered_content.as_deref().is_some_and(|raw| {
            crate::rendering::has_djot_plain_body(doc)
                && !matches!(
                    doc.elements.as_slice(),
                    [element] if element.kind == ElementKind::Paragraph && element.text == raw
                )
        })
}

pub(super) fn derive_markdown(
    doc: &crate::types::internal::InternalDocument,
    observer: &mut impl DerivationObserver,
) -> String {
    let all_owned = crate::rendering::markdown_tag_exemptions_are_owned(doc);
    let budget = if all_owned {
        None
    } else {
        observer.markdown_tag_budget()
    };
    let (rendered, ranges) = crate::rendering::render_markdown_observed(doc, budget);
    observer.formatted_from_source(true, &rendered, all_owned);
    observer.formatted_tag_ranges(ranges);
    rendered
}

pub(super) fn derive_djot(
    doc: &mut crate::types::internal::InternalDocument,
    observer: &mut impl DerivationObserver,
    plain_from_ir: bool,
) -> Option<String> {
    let (rendered, verified) =
        if doc.pre_rendered_content.is_some() && doc.metadata.output_format.as_deref() == Some("djot") {
            (doc.pre_rendered_content.take()?, false)
        } else {
            let rendered = crate::rendering::render_djot(doc);
            let verified =
                observer.captures_verified_markup() && plain_from_ir && crate::rendering::has_djot_plain_body(doc);
            (rendered, verified)
        };
    observer.formatted_markup(VerifiedMarkup::Djot, &rendered, verified);
    Some(rendered)
}

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
    fn seal_redacted_markup(&mut self) -> crate::Result<()> {
        Ok(())
    }

    #[cfg(feature = "redaction")]
    fn validate_sealed_markup(&self, _config: &crate::ExtractionConfig) -> crate::Result<()> {
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
    fn formatted_markup(&self) -> Option<VerifiedMarkup> {
        None
    }
    fn formatted_html(&self) -> bool {
        self.formatted_markup() == Some(VerifiedMarkup::Html)
    }
    fn page_markup_backup(&self, _index: usize) -> Option<&str> {
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
    fn formatted_owned_tags(&self) -> Option<&[Range<usize>]> {
        None
    }
    fn page_owned_tags(&self, _index: usize) -> Option<&[Range<usize>]> {
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
