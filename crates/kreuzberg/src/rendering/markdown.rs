//! Render an `InternalDocument` to GFM-compliant Markdown via comrak.

use comrak::{Arena, Options, format_commonmark};

use crate::types::internal::InternalDocument;

use super::comrak_bridge::build_comrak_ast;

/// Render an `InternalDocument` to GFM Markdown.
pub fn render_markdown(doc: &InternalDocument) -> String {
    tracing::debug!(element_count = doc.elements.len(), "markdown rendering starting");
    let arena = Arena::new();
    let root = build_comrak_ast(doc, &arena);

    let mut options = comrak_options();
    options.render.width = 0; // no line wrapping

    let mut output = String::new();
    format_commonmark(root, &options, &mut output).expect("comrak formatting should not fail");

    // Trim trailing whitespace but keep single trailing newline
    let trimmed_len = output.trim_end().len();
    if trimmed_len == 0 {
        return String::new();
    }
    output.truncate(trimmed_len);
    output.push('\n');
    tracing::debug!(output_length = output.len(), "markdown rendering complete");
    output
}

/// Shared comrak options with all GFM extensions enabled.
pub(crate) fn comrak_options<'a>() -> Options<'a> {
    let mut options = Options::default();
    options.extension.table = true;
    options.extension.strikethrough = true;
    options.extension.footnotes = true;
    options.extension.description_lists = true;
    options.extension.math_dollars = true;
    options.extension.underline = true;
    options.extension.subscript = true;
    options.extension.superscript = true;
    options.extension.highlight = true;
    options.extension.alerts = true;
    options
}
