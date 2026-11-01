//! Core types for the PDF-to-Markdown pipeline.

use crate::pdf::hierarchy::SegmentData;

/// A line of text composed of segments sharing a common baseline.
#[derive(Debug, Clone)]
pub(crate) struct PdfLine {
    pub segments: Vec<SegmentData>,
    pub native_source_origin: Option<super::pipeline::native_scripts::NativeSourceOrigin>,
    pub native_scripts: Vec<crate::pdf::native::hierarchy::script_ownership::page_segments::SegmentScriptOwner>,
    pub baseline_y: f32,
    #[allow(dead_code)]
    pub dominant_font_size: f32,
    #[allow(dead_code)]
    pub is_bold: bool,
    pub is_monospace: bool,
}

impl PdfLine {
    pub(crate) fn prepend_segment(&mut self, segment: SegmentData) {
        self.native_source_origin = None;
        for owner in &mut self.native_scripts {
            owner.base_index += 1;
            owner.script_index += 1;
        }
        self.segments.insert(0, segment);
    }

    pub(crate) fn valid_native_scripts(
        &self,
    ) -> Vec<&crate::pdf::native::hierarchy::script_ownership::page_segments::SegmentScriptOwner> {
        let mut endpoints = std::collections::HashMap::new();
        crate::pdf::native::hierarchy::script_ownership::page_segments::complete_script_runs(
            self.native_scripts
                .iter()
                .filter(|owner| {
                    [(owner.base_index, false), (owner.script_index, true)]
                        .into_iter()
                        .all(|(index, script)| {
                            *endpoints
                                .entry((index, owner.original_index(script)))
                                .or_insert_with(|| {
                                    self.segments
                                        .get(index)
                                        .is_some_and(|segment| owner.endpoint_matches(segment, script))
                                })
                        })
                })
                .collect(),
        )
    }
}

/// A paragraph composed of lines, with optional heading classification.
#[derive(Debug, Clone)]
pub(crate) struct PdfParagraph {
    /// Full text content from `page.text().all()` (heuristic path).
    /// When populated, assembly uses this directly instead of joining segment texts.
    /// Empty for the structure tree path (which uses lines/segments).
    pub text: String,
    pub lines: Vec<PdfLine>,
    pub dominant_font_size: f32,
    pub heading_level: Option<u8>,
    pub is_bold: bool,
    pub is_list_item: bool,
    pub is_code_block: bool,
    pub is_formula: bool,
    pub is_page_furniture: bool,
    pub layout_class: Option<LayoutHintClass>,
    /// Stable page-local layout ancestry assigned by the region planner.
    ///
    /// `None` preserves the legacy no-layout path byte-for-byte. A populated
    /// path is carried through document-level classification and consumed only
    /// during final assembly, where it becomes invisible `Group` containers.
    pub layout_region_path: Option<LayoutRegionPath>,
    /// Index of the parent element this caption is associated with (tables/pictures).
    pub caption_for: Option<usize>,
    /// Block-level bounding box from structure tree extraction.
    /// Used for spatial matching when per-segment positions aren't available.
    /// Format: (left, bottom, right, top) in PDF coordinate space.
    pub block_bbox: Option<(f32, f32, f32, f32)>,
    /// Cached word count, computed at construction time.
    pub word_count: usize,
}

impl PdfParagraph {
    /// Check if this paragraph is monospace (full-text path uses is_code_block flag,
    /// structure tree path checks line-level flags).
    pub(crate) fn is_monospace_hint(&self) -> bool {
        self.is_code_block
    }

    /// Compute word count from either the full-text path or segment path.
    pub(crate) fn compute_word_count(text: &str, lines: &[PdfLine]) -> usize {
        if !text.is_empty() {
            text.split_whitespace().count()
        } else {
            lines
                .iter()
                .flat_map(|l| l.segments.iter())
                .map(|s| s.text.split_whitespace().count())
                .sum()
        }
    }
}

/// Simplified layout class for the markdown pipeline.
///
/// Decoupled from `crate::layout::LayoutClass` so the markdown module
/// compiles without the `layout-detection` feature.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(dead_code)]
pub(crate) enum LayoutHintClass {
    Title,
    SectionHeader,
    Code,
    Formula,
    ListItem,
    Caption,
    Footnote,
    PageHeader,
    PageFooter,
    Table,
    Picture,
    DocumentIndex,
    Form,
    KeyValueRegion,
    Text,
    Other,
}

impl LayoutHintClass {
    pub(crate) const fn is_wrapper(self) -> bool {
        matches!(
            self,
            Self::Table | Self::Picture | Self::DocumentIndex | Self::Form | Self::KeyValueRegion
        )
    }

    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Title => "title",
            Self::SectionHeader => "section_header",
            Self::Code => "code",
            Self::Formula => "formula",
            Self::ListItem => "list_item",
            Self::Caption => "caption",
            Self::Footnote => "footnote",
            Self::PageHeader => "page_header",
            Self::PageFooter => "page_footer",
            Self::Table => "table",
            Self::Picture => "picture",
            Self::DocumentIndex => "document_index",
            Self::Form => "form",
            Self::KeyValueRegion => "key_value_region",
            Self::Text => "text",
            Self::Other => "other",
        }
    }
}

/// One stable component of a page-local layout region path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct LayoutRegionTag {
    pub(crate) id: usize,
    pub(crate) class_name: Option<LayoutHintClass>,
}

/// Layout ancestry is at most two levels: a top-level wrapper/root and an
/// optional semantic child region contained by that wrapper.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct LayoutRegionPath {
    pub(crate) root: LayoutRegionTag,
    pub(crate) child: Option<LayoutRegionTag>,
}

impl LayoutRegionPath {
    pub(crate) fn tags(self) -> impl Iterator<Item = LayoutRegionTag> {
        std::iter::once(self.root).chain(self.child)
    }
}

/// A layout hint for paragraph classification.
///
/// Contains a simplified layout class with confidence and bounding box
/// in PDF coordinate space (points, y=0 at bottom of page).
#[derive(Debug, Clone)]
pub(crate) struct LayoutHint {
    pub class_name: LayoutHintClass,
    pub confidence: f32,
    pub left: f32,
    pub bottom: f32,
    pub right: f32,
    pub top: f32,
}

/// Layout detection results for a single page.
///
/// Carries the raw (unrotated MediaBox) page dimensions — the space text
/// segments live in. Used by region validation and table recognition to map
/// pixel predictions back to PDF coordinates.
#[cfg(feature = "layout-detection")]
#[derive(Debug, Clone)]
pub struct PageLayoutResult {
    pub page_width_pts: f32,
    pub page_height_pts: f32,
}
