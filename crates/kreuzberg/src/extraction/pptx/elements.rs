//! Internal types for PPTX extraction.
//!
//! This module defines the internal data structures used to represent
//! slide elements, formatting, and text runs during XML parsing.

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct ElementPosition {
    pub(super) x: i64,
    pub(super) y: i64,
    /// Width in EMUs (from `a:ext cx`).
    pub(super) cx: i64,
    /// Height in EMUs (from `a:ext cy`).
    pub(super) cy: i64,
}

#[derive(Debug, Clone, Default)]
pub(super) struct Formatting {
    pub(super) bold: bool,
    pub(super) italic: bool,
    pub(super) underlined: bool,
    pub(super) strikethrough: bool,
    /// Font size in hundredths of a point (from `a:rPr sz`).
    pub(super) font_size: Option<u32>,
    pub(super) lang: String,
}

#[derive(Debug, Clone)]
pub(super) struct Run {
    pub(super) text: String,
    pub(super) formatting: Formatting,
    /// Relationship ID for a hyperlink attached to this run (`a:hlinkClick r:id`).
    pub(super) hyperlink_id: Option<String>,
}

impl Run {
    pub(super) fn extract(&self) -> String {
        self.text.clone()
    }

    pub(super) fn render_as_md(&self) -> String {
        let mut result = self.text.clone();

        if self.formatting.bold {
            result = format!("**{}**", result);
        }
        if self.formatting.italic {
            result = format!("*{}*", result);
        }
        if self.formatting.underlined {
            result = format!("<u>{}</u>", result);
        }
        if self.formatting.strikethrough {
            result = format!("~~{}~~", result);
        }

        result
    }
}

#[derive(Debug, Clone)]
pub(super) struct TextElement {
    pub(super) runs: Vec<Run>,
    /// Whether this text element comes from a title placeholder shape.
    pub(super) is_title: bool,
}

#[derive(Debug, Clone)]
pub(super) struct ListItem {
    pub(super) level: u32,
    pub(super) is_ordered: bool,
    pub(super) runs: Vec<Run>,
    /// Whether this paragraph has an explicit bullet marker (`buAutoNum` or `buChar`).
    /// When false, the paragraph is a plain text preamble within a list shape.
    pub(super) has_bullet: bool,
}

#[derive(Debug, Clone)]
pub(super) struct ListElement {
    pub(super) items: Vec<ListItem>,
}

#[derive(Debug, Clone)]
pub(super) struct TableCell {
    pub(super) runs: Vec<Run>,
}

#[derive(Debug, Clone)]
pub(super) struct TableRow {
    pub(super) cells: Vec<TableCell>,
}

#[derive(Debug, Clone)]
pub(super) struct TableElement {
    pub(super) rows: Vec<TableRow>,
}

#[derive(Debug, Clone)]
pub(super) struct ImageReference {
    pub(super) id: String,
    pub(super) target: String,
    /// Alt text / description from shape `descr` attribute.
    pub(super) description: Option<String>,
}

/// A hyperlink relationship resolved from a slide rels file.
#[derive(Debug, Clone)]
pub(super) struct HyperlinkReference {
    pub(super) id: String,
    pub(super) url: String,
}

#[derive(Debug, Clone)]
pub(super) enum SlideElement {
    Text(TextElement, ElementPosition),
    Table(TableElement, ElementPosition),
    Image(ImageReference, ElementPosition),
    List(ListElement, ElementPosition),
    Unknown,
}

impl SlideElement {
    pub(super) fn position(&self) -> ElementPosition {
        match self {
            SlideElement::Text(_, pos)
            | SlideElement::Table(_, pos)
            | SlideElement::Image(_, pos)
            | SlideElement::List(_, pos) => *pos,
            SlideElement::Unknown => ElementPosition::default(),
        }
    }
}

#[derive(Debug)]
pub(super) struct Slide {
    pub(super) slide_number: u32,
    pub(super) elements: Vec<SlideElement>,
    pub(super) images: Vec<ImageReference>,
    /// Hyperlink relationships resolved from the slide rels file.
    pub(super) hyperlinks: Vec<HyperlinkReference>,
}

#[derive(Debug, Clone)]
pub(super) struct ParserConfig {
    pub(super) extract_images: bool,
    pub(super) include_slide_comment: bool,
    pub(super) plain: bool,
    /// When `false`, `![alt](target)` image references are omitted from the
    /// markdown output even though the slide element is present. Mirrors
    /// `ImageExtractionConfig::inject_placeholders`. Default: `true`.
    pub(super) inject_placeholders: bool,
}

impl Default for ParserConfig {
    fn default() -> Self {
        Self {
            extract_images: true,
            include_slide_comment: false,
            plain: false,
            inject_placeholders: true,
        }
    }
}

pub(super) enum ParsedContent {
    Text(TextElement),
    List(ListElement),
}
