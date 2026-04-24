use serde::{Deserialize, Serialize};
use std::fmt;

/// Bounding box in original image coordinates (x1, y1) top-left, (x2, y2) bottom-right.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BBox {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
}

impl BBox {
    pub(crate) fn new(x1: f32, y1: f32, x2: f32, y2: f32) -> Self {
        Self { x1, y1, x2, y2 }
    }

    pub(crate) fn width(&self) -> f32 {
        (self.x2 - self.x1).max(0.0)
    }

    pub(crate) fn height(&self) -> f32 {
        (self.y2 - self.y1).max(0.0)
    }

    pub(crate) fn area(&self) -> f32 {
        self.width() * self.height()
    }

    #[cfg(test)]
    pub(crate) fn center(&self) -> (f32, f32) {
        ((self.x1 + self.x2) / 2.0, (self.y1 + self.y2) / 2.0)
    }

    /// Area of intersection with another bounding box.
    pub(crate) fn intersection_area(&self, other: &BBox) -> f32 {
        let x1 = self.x1.max(other.x1);
        let y1 = self.y1.max(other.y1);
        let x2 = self.x2.min(other.x2);
        let y2 = self.y2.min(other.y2);
        (x2 - x1).max(0.0) * (y2 - y1).max(0.0)
    }

    /// Intersection over Union with another bounding box.
    pub(crate) fn iou(&self, other: &BBox) -> f32 {
        let inter = self.intersection_area(other);
        let union = self.area() + other.area() - inter;
        if union <= 0.0 { 0.0 } else { inter / union }
    }

    /// Fraction of `other` that is contained within `self`.
    /// Returns 0.0..=1.0 where 1.0 means `other` is fully inside `self`.
    pub(crate) fn containment_of(&self, other: &BBox) -> f32 {
        let other_area = other.area();
        if other_area <= 0.0 {
            return 0.0;
        }
        self.intersection_area(other) / other_area
    }

    /// Fraction of page area this bbox covers.
    pub(crate) fn page_coverage(&self, page_width: f32, page_height: f32) -> f32 {
        let page_area = page_width * page_height;
        if page_area <= 0.0 {
            return 0.0;
        }
        self.area() / page_area
    }
}

impl fmt::Display for BBox {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{:.1}, {:.1}, {:.1}, {:.1}]", self.x1, self.y1, self.x2, self.y2)
    }
}

/// The 17 canonical document layout classes.
///
/// All model backends (RT-DETR, YOLO, etc.) map their native class IDs
/// to this shared set. Models with fewer classes (DocLayNet: 11, PubLayNet: 5)
/// map to the closest equivalent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LayoutClass {
    Caption,
    Footnote,
    Formula,
    ListItem,
    PageFooter,
    PageHeader,
    Picture,
    SectionHeader,
    Table,
    Text,
    Title,
    DocumentIndex,
    Code,
    CheckboxSelected,
    CheckboxUnselected,
    Form,
    KeyValueRegion,
}

impl LayoutClass {
    /// Map from Docling RT-DETR model label ID (0-16) to LayoutClass.
    pub(crate) fn from_docling_id(id: i64) -> Option<Self> {
        match id {
            0 => Some(Self::Caption),
            1 => Some(Self::Footnote),
            2 => Some(Self::Formula),
            3 => Some(Self::ListItem),
            4 => Some(Self::PageFooter),
            5 => Some(Self::PageHeader),
            6 => Some(Self::Picture),
            7 => Some(Self::SectionHeader),
            8 => Some(Self::Table),
            9 => Some(Self::Text),
            10 => Some(Self::Title),
            11 => Some(Self::DocumentIndex),
            12 => Some(Self::Code),
            13 => Some(Self::CheckboxSelected),
            14 => Some(Self::CheckboxUnselected),
            15 => Some(Self::Form),
            16 => Some(Self::KeyValueRegion),
            _ => None,
        }
    }

    /// Map from DocLayNet class ID (0-10) to LayoutClass.
    ///
    /// DocLayNet classes: Caption, Footnote, Formula, List-item, Page-footer,
    /// Page-header, Picture, Section-header, Table, Text, Title.
    pub(crate) fn from_doclaynet_id(id: i64) -> Option<Self> {
        match id {
            0 => Some(Self::Caption),
            1 => Some(Self::Footnote),
            2 => Some(Self::Formula),
            3 => Some(Self::ListItem),
            4 => Some(Self::PageFooter),
            5 => Some(Self::PageHeader),
            6 => Some(Self::Picture),
            7 => Some(Self::SectionHeader),
            8 => Some(Self::Table),
            9 => Some(Self::Text),
            10 => Some(Self::Title),
            _ => None,
        }
    }

    /// Map from DocStructBench class ID (0-9) to LayoutClass.
    ///
    /// DocStructBench classes: Title, Plain Text, Abandoned Text, Figure,
    /// Figure Caption, Table, Table Caption, Table Footnote, Isolated Formula, Formula Caption.
    pub(crate) fn from_docstructbench_id(id: i64) -> Option<Self> {
        match id {
            0 => Some(Self::Title),
            1 => Some(Self::Text),
            2 => Some(Self::Text),    // Abandoned Text → Text
            3 => Some(Self::Picture), // Figure
            4 => Some(Self::Caption), // Figure Caption
            5 => Some(Self::Table),
            6 => Some(Self::Caption),  // Table Caption
            7 => Some(Self::Footnote), // Table Footnote
            8 => Some(Self::Formula),  // Isolated Formula
            9 => Some(Self::Caption),  // Formula Caption
            _ => None,
        }
    }

    /// Map from PubLayNet class ID (0-4) to LayoutClass.
    ///
    /// PubLayNet classes: Text, Title, List, Table, Figure.
    #[cfg(test)]
    pub(crate) fn from_publaynet_id(id: i64) -> Option<Self> {
        match id {
            0 => Some(Self::Text),
            1 => Some(Self::Title),
            2 => Some(Self::ListItem),
            3 => Some(Self::Table),
            4 => Some(Self::Picture),
            _ => None,
        }
    }

    pub(crate) fn name(&self) -> &'static str {
        match self {
            Self::Caption => "caption",
            Self::Footnote => "footnote",
            Self::Formula => "formula",
            Self::ListItem => "list_item",
            Self::PageFooter => "page_footer",
            Self::PageHeader => "page_header",
            Self::Picture => "picture",
            Self::SectionHeader => "section_header",
            Self::Table => "table",
            Self::Text => "text",
            Self::Title => "title",
            Self::DocumentIndex => "document_index",
            Self::Code => "code",
            Self::CheckboxSelected => "checkbox_selected",
            Self::CheckboxUnselected => "checkbox_unselected",
            Self::Form => "form",
            Self::KeyValueRegion => "key_value_region",
        }
    }

    /// Whether this class is a "wrapper" type that can contain child elements.
    #[cfg(test)]
    pub(crate) fn is_wrapper(&self) -> bool {
        matches!(
            self,
            Self::Form | Self::KeyValueRegion | Self::Table | Self::DocumentIndex
        )
    }
}

impl fmt::Display for LayoutClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// A single layout detection result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutDetection {
    pub class: LayoutClass,
    pub confidence: f32,
    pub bbox: BBox,
}

impl LayoutDetection {
    /// Sort detections by confidence in descending order.
    pub(crate) fn sort_by_confidence_desc(mut detections: Vec<LayoutDetection>) -> Vec<LayoutDetection> {
        detections.sort_by(|a, b| b.confidence.total_cmp(&a.confidence));
        detections
    }

    pub(crate) fn new(class: LayoutClass, confidence: f32, bbox: BBox) -> Self {
        Self {
            class,
            confidence,
            bbox,
        }
    }
}

impl fmt::Display for LayoutDetection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:20} conf={:.3}  bbox={}",
            self.class.name(),
            self.confidence,
            self.bbox
        )
    }
}

/// Page-level detection result containing all detections and page metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionResult {
    pub page_width: u32,
    pub page_height: u32,
    pub detections: Vec<LayoutDetection>,
}

impl DetectionResult {
    pub(crate) fn new(page_width: u32, page_height: u32, detections: Vec<LayoutDetection>) -> Self {
        Self {
            page_width,
            page_height,
            detections,
        }
    }
}
