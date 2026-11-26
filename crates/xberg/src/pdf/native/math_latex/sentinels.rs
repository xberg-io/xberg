//! Private delimiters that carry a formula from native spans to document assembly.
//!
//! The structure pipeline reflows span text into lines and paragraphs before it builds
//! elements, and Markdown escapes a literal `$`. A formula therefore travels between Unicode
//! noncharacters (U+FDD0..U+FDD3), which no PDF text decodes to. Paragraph assembly turns them
//! into `$`-delimited math annotations; [`strip_document`] removes any left in other elements
//! and tables, so they never reach output.

use crate::types::Table;
use crate::types::document_structure::TextAnnotation;
use crate::types::internal::InternalDocument;

pub(crate) const INLINE_OPEN: char = '\u{FDD0}';
pub(crate) const INLINE_CLOSE: char = '\u{FDD1}';
pub(crate) const DISPLAY_OPEN: char = '\u{FDD2}';
pub(crate) const DISPLAY_CLOSE: char = '\u{FDD3}';

pub(crate) fn is_sentinel(c: char) -> bool {
    matches!(c, INLINE_OPEN | INLINE_CLOSE | DISPLAY_OPEN | DISPLAY_CLOSE)
}

pub(crate) fn contains_sentinel(text: &str) -> bool {
    text.contains(is_sentinel)
}

/// A formula in resolved text, delimiters included.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MathRange {
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) display: bool,
}

/// Text with its sentinels replaced, and the map from source to resolved byte offsets.
#[derive(Debug)]
pub(crate) struct Resolved {
    pub(crate) text: String,
    pub(crate) math: Vec<MathRange>,
    /// (source offset, source length, replacement length) of each sentinel.
    replacements: Vec<(usize, usize, usize)>,
}

impl Resolved {
    /// The resolved offset of a source offset outside a sentinel.
    pub(crate) fn remap(&self, offset: usize) -> usize {
        let shift: isize = self
            .replacements
            .iter()
            .filter(|(at, len, _)| at + len <= offset)
            .map(|&(_, len, replacement)| replacement as isize - len as isize)
            .sum();
        offset.saturating_add_signed(shift)
    }

    /// Move `annotations` onto the resolved text, dropping any that become empty.
    pub(crate) fn remap_annotations(&self, annotations: &mut Vec<TextAnnotation>) {
        for annotation in annotations.iter_mut() {
            annotation.start = self.remap(annotation.start as usize) as u32;
            annotation.end = self.remap(annotation.end as usize) as u32;
        }
        annotations.retain(|annotation| annotation.start < annotation.end);
    }
}

/// Replace each sentinel with its `$`/`$$` delimiter (`delimit`) or remove it.
pub(crate) fn resolve(text: &str, delimit: bool) -> Resolved {
    let mut out = String::with_capacity(text.len());
    let mut math = Vec::new();
    let mut replacements = Vec::new();
    let mut open: Option<(usize, bool)> = None;
    for (offset, c) in text.char_indices() {
        let delimiter = match c {
            INLINE_OPEN | INLINE_CLOSE => "$",
            DISPLAY_OPEN | DISPLAY_CLOSE => "$$",
            _ => {
                out.push(c);
                continue;
            }
        };
        let delimiter = if delimit { delimiter } else { "" };
        if matches!(c, INLINE_OPEN | DISPLAY_OPEN) {
            open = Some((out.len(), c == DISPLAY_OPEN));
        }
        out.push_str(delimiter);
        if matches!(c, INLINE_CLOSE | DISPLAY_CLOSE)
            && let Some((start, display)) = open.take()
        {
            math.push(MathRange {
                start,
                end: out.len(),
                display,
            });
        }
        replacements.push((offset, c.len_utf8(), delimiter.len()));
    }
    Resolved {
        text: out,
        math,
        replacements,
    }
}

/// `text` without sentinels: the formula's TeX source, undelimited.
pub(crate) fn strip(text: &str) -> String {
    text.chars().filter(|&c| !is_sentinel(c)).collect()
}

pub(crate) fn strip_table(table: &mut Table) {
    for cell in table.cells.iter_mut().flatten() {
        if contains_sentinel(cell) {
            *cell = strip(cell);
        }
    }
    if contains_sentinel(&table.markdown) {
        table.markdown = strip(&table.markdown);
    }
}

/// Remove every sentinel still in `doc`: elements other than prose paragraphs (headings,
/// list items, captions, formulas), their attributes, and tables.
pub(crate) fn strip_document(doc: &mut InternalDocument) {
    for element in &mut doc.elements {
        if contains_sentinel(&element.text) {
            let resolved = resolve(&element.text, false);
            resolved.remap_annotations(&mut element.annotations);
            element.text = resolved.text;
        }
        if let Some(attributes) = element.attributes.as_mut() {
            for value in attributes.values_mut() {
                if contains_sentinel(value) {
                    *value = strip(value);
                }
            }
        }
    }
    for table in &mut doc.tables {
        strip_table(table);
    }
}
