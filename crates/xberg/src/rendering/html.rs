//! Render an `InternalDocument` to HTML5 via comrak.

use std::fmt::{self, Write};

use comrak::Arena;
use comrak::html::{ChildRendering, Context, format_document_with_formatter, format_node_default};
use comrak::nodes::{AstNode, NodeValue};
use comrak::options::Plugins;

use crate::types::annotations::PdfAnnotation;
use crate::types::internal::InternalDocument;

use super::common::{annotation_display_text, annotation_type_label, escape_html_text};
use super::comrak_bridge::build_html_comrak_ast;
use super::markdown::comrak_options;

/// Render an `InternalDocument` to HTML5.
pub(crate) fn render_html(doc: &InternalDocument) -> String {
    render_html_checked(doc, None).0
}

pub(crate) fn render_html_checked(doc: &InternalDocument, plain: Option<&str>) -> (String, bool) {
    let arena = Arena::new();
    let root = build_html_comrak_ast(doc, &arena);
    #[cfg(debug_assertions)]
    root.validate().expect("comrak HTML AST should be valid");

    let mut options = comrak_options();
    options.render.r#unsafe = true;
    options.render.github_pre_lang = true;
    options.render.full_info_string = true;

    let mut output = String::new();
    format_document_with_formatter(root, &options, &mut output, &Plugins::default(), format_html_node, ())
        .expect("comrak HTML formatting should not fail");

    if let Some(annotations) = doc.annotations.as_deref() {
        output.push_str(&render_annotations_html(annotations));
    }

    let verified =
        plain.is_some_and(|plain| doc.annotations.as_ref().is_none_or(Vec::is_empty) && html_body_matches(root, plain));
    (output, verified)
}

fn html_body_matches<'a>(root: &'a AstNode<'a>, plain: &str) -> bool {
    let Some(mut text) = HtmlBody::new(plain) else {
        return false;
    };
    for node in root.descendants() {
        let value = &node.data.borrow().value;
        let piece = match value {
            NodeValue::Document
            | NodeValue::Emph
            | NodeValue::Strong
            | NodeValue::Underline
            | NodeValue::Strikethrough
            | NodeValue::Subscript
            | NodeValue::Superscript
            | NodeValue::Highlight
            | NodeValue::Link(_) => "",
            NodeValue::Paragraph | NodeValue::Heading(_) | NodeValue::SoftBreak | NodeValue::LineBreak => " ",
            NodeValue::Text(text) => text.as_ref(),
            NodeValue::Code(code) => code.literal.as_str(),
            _ => return false,
        };
        if !text.append(piece) {
            return false;
        }
    }
    text.complete()
}

pub(super) struct HtmlBody<'a> {
    plain: &'a str,
    started: bool,
    separator: bool,
    remaining: usize,
}

fn html_space(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r' | '\x0c')
}

impl<'a> HtmlBody<'a> {
    pub(super) fn new(plain: &'a str) -> Option<Self> {
        let remaining = crate::extractors::security::SecurityLimits::default()
            .max_iterations
            .checked_sub(plain.len())?;
        Some(Self {
            plain: plain.trim_matches(html_space),
            started: false,
            separator: false,
            remaining,
        })
    }

    pub(super) fn append(&mut self, text: &str) -> bool {
        let Some(remaining) = self.remaining.checked_sub(text.len().saturating_add(1)) else {
            return false;
        };
        self.remaining = remaining;
        for ch in text.chars() {
            if html_space(ch) {
                self.separator |= self.started;
                continue;
            }
            if self.separator {
                let trimmed = self.plain.trim_start_matches(html_space);
                if trimmed.len() == self.plain.len() {
                    return false;
                }
                self.plain = trimmed;
            }
            let Some(rest) = self.plain.strip_prefix(ch).filter(|_| ch != '\0') else {
                return false;
            };
            self.plain = rest;
            self.started = true;
            self.separator = false;
        }
        true
    }

    pub(super) fn complete(&self) -> bool {
        self.plain.is_empty()
    }
}

fn format_html_node<'a>(
    context: &mut Context<'_, '_, ()>,
    node: &'a AstNode<'a>,
    entering: bool,
) -> Result<ChildRendering, fmt::Error> {
    let headerless_table = matches!(node.data.borrow().value, NodeValue::Table(_))
        && node
            .first_child()
            .is_some_and(|row| matches!(row.data.borrow().value, NodeValue::TableRow(false)));
    if !headerless_table {
        return format_node_default(context, node, entering);
    }
    // Comrak opens tbody only after a header, but closes it based on row count.
    // Headerless tables need one balanced body even when they have a single row. ~keep
    if entering {
        format_node_default(context, node, true)?;
        context.write_str("<tbody>")?;
    } else {
        context.cr()?;
        context.write_str("</tbody>")?;
        context.lf()?;
        context.write_str("</table>")?;
    }
    context.lf()?;
    Ok(ChildRendering::HTML)
}

/// Render the document-level PDF annotations (issue #63) as an HTML section:
/// an `<h2>` heading followed by a `<ul>` list, one `<li>` per annotation.
///
/// Returns an empty string when `annotations` is empty.
fn render_annotations_html(annotations: &[PdfAnnotation]) -> String {
    if annotations.is_empty() {
        return String::new();
    }

    let mut out = String::from("<h2>Annotations</h2>\n<ul>\n");
    for annotation in annotations {
        out.push_str("<li><strong>");
        out.push_str(annotation_type_label(annotation.annotation_type));
        out.push_str("</strong> (page ");
        out.push_str(&annotation.page_number.to_string());
        out.push(')');

        if let Some(author) = annotation.author.as_deref().filter(|s| !s.is_empty()) {
            out.push_str(" by ");
            out.push_str(&escape_html_text(author));
        }

        if let Some(text) = annotation_display_text(annotation) {
            out.push_str(": ");
            out.push_str(&escape_html_text(text));
        }

        out.push_str("</li>\n");
    }
    out.push_str("</ul>\n");
    out
}
