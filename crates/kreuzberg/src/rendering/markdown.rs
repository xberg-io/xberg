//! Render an `InternalDocument` to GFM-compliant Markdown via comrak.

use comrak::{Arena, Options, format_commonmark};
use std::sync::LazyLock;

use crate::types::internal::InternalDocument;

use super::comrak_bridge::build_comrak_ast;

static ARXIV_WATERMARK_REGEX: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(
        r"(?:\s+\S+(?:\s+\S+){0,8})?\s*arXiv:\d{4}\.\d{4,5}(?:v\d+)?(?:\s*\[[\w.-]+\])?\s*(?:\d{1,2}\s+\w+\s+\d{4})?",
    )
    .expect("static regex compiles")
});

/// Render an `InternalDocument` to GFM Markdown.
pub(crate) fn render_markdown(doc: &InternalDocument) -> String {
    tracing::debug!(element_count = doc.elements.len(), "markdown rendering starting");
    let arena = Arena::new();
    let root = build_comrak_ast(doc, &arena);

    // Guard: empty AST causes index-out-of-bounds in comrak's formatter.
    if root.first_child().is_none() {
        tracing::debug!("markdown rendering: empty AST, returning empty string");
        return String::new();
    }

    let mut options = comrak_options();
    options.render.width = 0; // no line wrapping

    let mut output = String::new();
    format_commonmark(root, &options, &mut output).expect("comrak formatting should not fail");

    // Strip comrak-generated HTML comments (e.g. `<!-- end list -->`) that leak
    // into markdown output when adjacent lists are rendered. Only page marker
    // comments (`<!-- PAGE N -->`) should appear in output, and those are inserted
    // by our own code, not by comrak.
    if output.contains("<!--") {
        output = output
            .lines()
            .filter(|line| {
                let trimmed = line.trim();
                !trimmed.starts_with("<!--") || !trimmed.ends_with("-->")
            })
            .collect::<Vec<_>>()
            .join("\n");
    }

    // Safety net: decode any HTML entities that slipped through from other code paths.
    // `&#10;` (newline) → space, `&#2;` (STX control char) → removed.
    if output.contains("&#") {
        output = output.replace("&#10;", " ").replace("&#2;", "");
    }

    // Un-escape underscores: comrak's format_commonmark escapes underscores as `\_`
    // to prevent emphasis interpretation, but our rendered content uses underscores
    // literally (e.g. sheet names like `first_sheet`). Since we never emit intentional
    // `\_` sequences, globally replacing is safe.
    if output.contains("\\_") {
        output = output.replace("\\_", "_");
    }

    // Un-escape brackets and parentheses: comrak's format_commonmark escapes `[`, `]`,
    // `(`, `)` in text nodes to prevent accidental link syntax. Since the AST already
    // handles real links via NodeValue::Link (rendered as `[text](url)` without
    // escaping), all remaining `\[`, `\]`, `\(`, `\)` are literal characters that
    // should appear un-escaped.
    if output.contains("\\[") || output.contains("\\]") || output.contains("\\(") || output.contains("\\)") {
        output = output
            .replace("\\[", "[")
            .replace("\\]", "]")
            .replace("\\(", "(")
            .replace("\\)", ")");
    }

    // Un-escape stars and hashes at the START of lines only.
    // comrak escapes `*` → `\*` and `#` → `\#` to prevent false emphasis / ATX-heading
    // interpretation. We need to un-escape these for RST list markers (`\* item` → `* item`)
    // and auto-numbered lists (`\#. item` → `#. item`), but NOT inside table cells where
    // `\*\*text\*\*` should remain escaped as literal asterisks.
    if output.contains("\\*") || output.contains("\\#") {
        output = output
            .lines()
            .map(|line| {
                let trimmed = line.trim_start();
                if trimmed.starts_with("\\* ") || trimmed.starts_with("\\#.") || trimmed.starts_with("\\#\\.") {
                    line.replacen("\\*", "*", 1).replacen("\\#", "#", 1)
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
    }

    // Collapse runs of 3+ newlines (double blank lines) into exactly 2 newlines.
    // comrak emits an extra blank line after lists when followed by a code block or table.
    // MD012 forbids multiple consecutive blank lines.
    while output.contains("\n\n\n") {
        output = output.replace("\n\n\n", "\n\n");
    }

    // Strip arXiv watermark/sidebar noise that gets concatenated with body text.
    // Only applies to the first ~2000 chars (first page area) to avoid touching references.
    output = strip_arxiv_watermark_noise(output);

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

/// Strip arXiv watermark noise from rendered markdown.
///
/// LaTeX-generated PDFs often have a rotated sidebar with the arXiv identifier
/// that the PDF extractor concatenates with body text. This strips patterns like:
/// "Title N arXiv:NNNN.NNNNNvN [cat.SC] DD Mon YYYY" from the first pages.
fn strip_arxiv_watermark_noise(mut text: String) -> String {
    // Only search the first portion of the text (roughly first 2 pages)
    let search_limit = text.floor_char_boundary(text.len().min(6000));
    let search_area = &text[..search_limit];

    if let Some(m) = ARXIV_WATERMARK_REGEX.find(search_area) {
        // Only strip if it looks like a watermark (appears near end of a paragraph,
        // not in the middle of a sentence about arXiv).
        let after = &search_area[m.end()..];
        let before_char = if m.start() > 0 {
            search_area[..m.start()].chars().last()
        } else {
            None
        };

        // Strip if preceded by a sentence-ending period or is at end of paragraph
        let is_at_paragraph_boundary = before_char == Some('.') || after.starts_with('\n') || after.starts_with("\n\n");
        if is_at_paragraph_boundary {
            let start = m.start();
            let end = m.end();
            tracing::trace!(
                stripped = %&text[start..end].chars().take(80).collect::<String>(),
                "stripping arXiv watermark from markdown output"
            );
            text.replace_range(start..end, "");
        }
    }

    text
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
    // Use fenced code blocks (```) instead of 4-space indentation.
    options.render.prefer_fenced = true;
    options
}
