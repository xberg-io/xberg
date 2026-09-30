//! PDF writer.
//!
//! Lays out a document from Markdown rather than from the element tree, for the same
//! reason as the DOCX writer: redaction and the other post-processors rewrite text, so
//! the pipeline carries the Markdown rendering through every processor stage and only
//! turns it into a PDF here, after the last one has run.
//!
//! Text is set in DejaVu Sans, embedded as a subset, with a `ToUnicode` map so that it
//! stays selectable and extractable. Code uses the standard Courier faces, which every
//! reader provides, for any character their `WinAnsiEncoding` can hold.

mod blocks;
mod font;
mod layout;
mod writer;

#[cfg(test)]
mod tests;

use crate::{Result, XbergError};

/// Build a PDF document holding `markdown`.
pub(crate) fn render_pdf(markdown: &str) -> Result<Vec<u8>> {
    let document = blocks::parse(markdown);
    let pages = layout::lay_out(&document)?;
    writer::write(&pages, &document.links)
}

fn document_error(message: impl std::fmt::Display) -> XbergError {
    XbergError::Serialization {
        message: format!("failed to write PDF document: {message}"),
        source: None,
    }
}
