//! Unified OCR element types for structured output.
//!
//! This module provides a unified representation of OCR results that preserves
//! all spatial and confidence information from both Tesseract and PaddleOCR backends.
//!
//! # Design Goals
//!
//! - **Full fidelity preservation**: Keep all data from both backends (bounding boxes, confidence scores, rotation)
//! - **Unified API**: Same types work for both Tesseract and PaddleOCR
//! - **Format flexibility**: Support text, markdown, djot, and structured output formats
//! - **Table detection support**: Enable table reconstruction from element geometry

use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;

fn deserialize_quadrilateral_points<'de, D>(deserializer: D) -> Result<Vec<OcrPoint>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let points = Vec::<OcrPoint>::deserialize(deserializer)?;
    if points.len() != 4 {
        return Err(serde::de::Error::invalid_length(
            points.len(),
            &"exactly four quadrilateral points",
        ));
    }
    Ok(points)
}

/// A point in OCR raster pixel coordinates.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct OcrPoint {
    /// Horizontal coordinate.
    pub x: u32,
    /// Vertical coordinate.
    pub y: u32,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum OcrPointWire {
    Positional((u32, u32)),
    Named { x: u32, y: u32 },
}

// ~keep Deserialize stays hand-written (not derived) so a legacy positional `[x, y]` array from
// pre-migration callers still parses; only Serialize now emits the named object.
impl<'de> Deserialize<'de> for OcrPoint {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(match OcrPointWire::deserialize(deserializer)? {
            OcrPointWire::Positional(point) => point.into(),
            OcrPointWire::Named { x, y } => Self { x, y },
        })
    }
}

#[cfg(feature = "api")]
impl utoipa::PartialSchema for OcrPoint {
    fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
        use utoipa::openapi::schema::{Object, ObjectBuilder, Type};

        ObjectBuilder::new()
            .property("x", Object::with_type(Type::Integer))
            .required("x")
            .property("y", Object::with_type(Type::Integer))
            .required("y")
            .into()
    }
}

#[cfg(feature = "api")]
impl utoipa::ToSchema for OcrPoint {}

impl From<(u32, u32)> for OcrPoint {
    fn from((x, y): (u32, u32)) -> Self {
        Self { x, y }
    }
}

impl From<OcrPoint> for (u32, u32) {
    fn from(point: OcrPoint) -> Self {
        (point.x, point.y)
    }
}

/// Bounding geometry for an OCR element.
///
/// Supports both axis-aligned rectangles (from Tesseract) and 4-point quadrilaterals
/// (from PaddleOCR and rotated text detection).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub enum OcrBoundingGeometry {
    /// Axis-aligned bounding box (typical for Tesseract output).
    Rectangle {
        /// Left x-coordinate in pixels
        left: u32,
        /// Top y-coordinate in pixels
        top: u32,
        /// Width in pixels
        width: u32,
        /// Height in pixels
        height: u32,
    },
    /// 4-point quadrilateral for rotated/skewed text (PaddleOCR).
    ///
    /// Points are in clockwise order starting from top-left:
    /// `[top_left, top_right, bottom_right, bottom_left]`
    Quadrilateral {
        /// Exactly four corner points in clockwise order.
        #[serde(deserialize_with = "deserialize_quadrilateral_points")]
        points: Vec<OcrPoint>,
    },
}

impl Default for OcrBoundingGeometry {
    fn default() -> Self {
        OcrBoundingGeometry::Rectangle {
            left: 0,
            top: 0,
            width: 0,
            height: 0,
        }
    }
}

impl OcrBoundingGeometry {
    /// Convert to axis-aligned bounding box (AABB).
    ///
    /// For rectangles, returns the exact bounds.
    /// For quadrilaterals, computes the minimal enclosing axis-aligned rectangle.
    ///
    /// # Returns
    ///
    /// Tuple of `(left, top, width, height)` in pixels.
    #[cfg(any(
        paddle_ocr,
        feature = "ocr",
        all(
            feature = "layout-detection",
            feature = "pdf",
            any(feature = "ocr", feature = "ocr-wasm")
        ),
        all(test, any(paddle_ocr, feature = "layout-detection", feature = "ocr"))
    ))]
    pub(crate) fn to_aabb(&self) -> (u32, u32, u32, u32) {
        match self {
            Self::Rectangle {
                left,
                top,
                width,
                height,
            } => (*left, *top, *width, *height),
            Self::Quadrilateral { points } => {
                let min_x = points.iter().map(|point| point.x).min().unwrap_or(0);
                let max_x = points.iter().map(|point| point.x).max().unwrap_or(0);
                let min_y = points.iter().map(|point| point.y).min().unwrap_or(0);
                let max_y = points.iter().map(|point| point.y).max().unwrap_or(0);
                (min_x, min_y, max_x.saturating_sub(min_x), max_y.saturating_sub(min_y))
            }
        }
    }

    /// Get the center point of the bounding geometry.
    #[cfg(any(
        all(
            feature = "layout-detection",
            feature = "pdf",
            any(feature = "ocr", feature = "ocr-wasm")
        ),
        all(test, feature = "layout-detection")
    ))]
    pub(crate) fn center(&self) -> (f64, f64) {
        let (left, top, width, height) = self.to_aabb();
        (left as f64 + width as f64 / 2.0, top as f64 + height as f64 / 2.0)
    }

    /// Check if this geometry overlaps with another.
    #[cfg(all(test, feature = "ocr"))]
    pub(crate) fn overlaps(&self, other: &Self) -> bool {
        let (l1, t1, w1, h1) = self.to_aabb();
        let (l2, t2, w2, h2) = other.to_aabb();

        let r1 = l1 + w1;
        let b1 = t1 + h1;
        let r2 = l2 + w2;
        let b2 = t2 + h2;

        l1 < r2 && r1 > l2 && t1 < b2 && b1 > t2
    }
}

/// Confidence scores for an OCR element.
///
/// Separates detection confidence (how confident that text exists at this location)
/// from recognition confidence (how confident about the actual text content).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct OcrConfidence {
    /// Detection confidence: how confident the OCR engine is that text exists here.
    ///
    /// PaddleOCR provides this as `box_score`, Tesseract doesn't have a direct equivalent.
    /// Range: 0.0 to 1.0 (or None if not available).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub detection: Option<f64>,

    /// Recognition confidence: how confident about the text content.
    ///
    /// Range: 0.0 to 1.0.
    pub recognition: f64,
}

impl OcrConfidence {
    /// Create confidence from Tesseract's single confidence value.
    ///
    /// Tesseract provides confidence as 0-100, which we normalize to 0.0-1.0.
    #[cfg(any(feature = "ocr", feature = "ocr-wasm"))]
    pub(crate) fn from_tesseract(confidence: f64) -> Self {
        Self {
            detection: None,
            recognition: (confidence / 100.0).clamp(0.0, 1.0),
        }
    }

    /// Create confidence from PaddleOCR scores.
    ///
    /// Both scores should be in 0.0-1.0 range, but PaddleOCR may occasionally return
    /// values slightly above 1.0 due to model calibration. This method clamps both
    /// values to ensure they stay within the valid 0.0-1.0 range.
    #[cfg(paddle_ocr)]
    pub(crate) fn from_paddle(box_score: f32, text_score: f32) -> Self {
        Self {
            detection: Some((box_score as f64).clamp(0.0, 1.0)),
            recognition: (text_score as f64).clamp(0.0, 1.0),
        }
    }
}

/// Rotation information for an OCR element.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct OcrRotation {
    /// Rotation angle in degrees (0, 90, 180, 270 for PaddleOCR).
    pub angle_degrees: f64,

    /// Confidence score for the rotation detection.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
}

impl OcrRotation {
    /// Create rotation from PaddleOCR angle classification.
    ///
    /// PaddleOCR uses angle_index (0-3) representing 0, 90, 180, 270 degrees.
    ///
    /// # Arguments
    ///
    /// * `angle_index` - Must be in range 0..=3; invalid values return an error
    /// * `angle_score` - Confidence score for rotation detection
    ///
    /// # Errors
    ///
    /// Returns an error if `angle_index` is not in the valid range (0-3).
    #[cfg(paddle_ocr)]
    pub(crate) fn from_paddle(angle_index: i32, angle_score: f32) -> std::result::Result<Self, String> {
        if !(0..=3).contains(&angle_index) {
            return Err(format!(
                "Invalid angle_index: {}. Must be 0-3 (representing 0°, 90°, 180°, 270°)",
                angle_index
            ));
        }

        Ok(Self {
            angle_degrees: match angle_index {
                0 => 0.0,
                1 => 180.0,
                2 => 90.0,
                3 => 270.0,
                _ => unreachable!("angle_index validated to 0..=3 above"),
            },
            confidence: Some((angle_score as f64).clamp(0.0, 1.0)),
        })
    }
}

/// Hierarchical level of an OCR element.
///
/// Maps to Tesseract's page segmentation hierarchy and provides
/// equivalent semantics for PaddleOCR.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum OcrElementLevel {
    /// Individual word
    Word,
    /// Line of text (default for PaddleOCR)
    #[default]
    Line,
    /// Paragraph or text block
    Block,
    /// Page-level element
    Page,
}

impl OcrElementLevel {
    /// Convert from Tesseract's numeric level (1-5).
    ///
    /// Tesseract levels: 1=Page, 2=Block, 3=Paragraph, 4=Line, 5=Word
    #[cfg(feature = "ocr")]
    pub(crate) fn from_tesseract_level(level: i32) -> Self {
        match level {
            1 => Self::Page,
            2 => Self::Block,
            3 => Self::Block,
            4 => Self::Line,
            5 => Self::Word,
            _ => Self::Line,
        }
    }
}

/// A unified OCR element representing detected text with full metadata.
///
/// This is the primary type for structured OCR output, preserving all information
/// from both Tesseract and PaddleOCR backends.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct OcrElement {
    /// The recognized text content.
    pub text: String,

    /// Bounding geometry (rectangle or quadrilateral).
    pub geometry: OcrBoundingGeometry,

    /// Confidence scores for detection and recognition.
    pub confidence: OcrConfidence,

    /// Hierarchical level (word, line, block, page).
    #[serde(default)]
    pub level: OcrElementLevel,

    /// Rotation information (if detected).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "api", schema(nullable = false))]
    pub rotation: Option<OcrRotation>,

    /// Page number (1-indexed).
    #[serde(default = "default_page_number")]
    pub page_number: u32,

    /// Parent element ID for hierarchical relationships.
    ///
    /// ~keep When hierarchy output is enabled, this resolves to another emitted element's
    /// `backend_metadata["element_id"]` value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,

    /// Backend-specific metadata that doesn't fit the unified schema.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub backend_metadata: HashMap<String, serde_json::Value>,
}

const OCR_ELEMENT_ID_METADATA_KEY: &str = "element_id";

impl OcrElement {
    /// Return this element's hierarchy ID when hierarchy output was requested.
    ///
    /// ~keep The ID is stored in `backend_metadata["element_id"]` for compatibility with the
    /// existing serialized and binding-safe shape of [`OcrElement`].
    #[cfg_attr(alef, alef(skip))]
    pub fn element_id(&self) -> Option<&str> {
        self.backend_metadata
            .get(OCR_ELEMENT_ID_METADATA_KEY)
            .and_then(serde_json::Value::as_str)
    }
}

fn default_page_number() -> u32 {
    1
}

#[cfg(any(feature = "ocr", feature = "ocr-wasm"))]
impl OcrElement {
    /// Create a new OCR element with minimal required fields.
    pub(crate) fn new(text: impl Into<String>, geometry: OcrBoundingGeometry, confidence: OcrConfidence) -> Self {
        Self {
            text: text.into(),
            geometry,
            confidence,
            level: OcrElementLevel::default(),
            rotation: None,
            page_number: 1,
            parent_id: None,
            backend_metadata: HashMap::new(),
        }
    }

    /// Set the hierarchical level.
    pub(crate) fn with_level(mut self, level: OcrElementLevel) -> Self {
        self.level = level;
        self
    }

    /// Set rotation information.
    #[cfg(all(test, paddle_ocr))]
    pub(crate) fn with_rotation(mut self, rotation: OcrRotation) -> Self {
        self.rotation = Some(rotation);
        self
    }

    /// Set page number.
    pub(crate) fn with_page_number(mut self, page_number: u32) -> Self {
        self.page_number = page_number;
        self
    }

    /// Add backend-specific metadata.
    pub(crate) fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.backend_metadata.insert(key.into(), value);
        self
    }
}

/// Configuration for OCR element extraction.
///
/// Controls how OCR elements are extracted and filtered.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[serde(deny_unknown_fields)]
pub struct OcrElementConfig {
    /// Whether to include OCR elements in the extraction result.
    ///
    /// When true, the `ocr_elements` field in `ExtractedDocument` will be populated.
    #[serde(default)]
    pub include_elements: bool,

    /// Minimum hierarchical level to include.
    ///
    /// Elements below this level (e.g., words when min_level is Line) will be excluded.
    #[serde(default)]
    pub min_level: OcrElementLevel,

    /// Minimum recognition confidence threshold (0.0-1.0).
    ///
    /// Elements with confidence below this threshold will be filtered out.
    #[serde(default)]
    pub min_confidence: f64,

    /// Whether to build hierarchical relationships between elements.
    ///
    /// ~keep When true, emitted elements receive an `element_id` metadata value and `parent_id`
    /// references are populated only when a spatially containing parent is also emitted.
    #[serde(default)]
    pub build_hierarchy: bool,
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
impl OcrElementConfig {
    pub(crate) fn select_elements(&self, elements: &[OcrElement]) -> Vec<OcrElement> {
        if !self.include_elements {
            return Vec::new();
        }

        let minimum_rank = element_level_rank(self.min_level);
        let mut selected = elements
            .iter()
            .filter(|element| element.confidence.recognition >= self.min_confidence)
            .filter(|element| element_level_rank(element.level) >= minimum_rank)
            .cloned()
            .collect::<Vec<_>>();

        for element in &mut selected {
            element.parent_id = None;
            element.backend_metadata.remove(OCR_ELEMENT_ID_METADATA_KEY);
        }
        if !self.build_hierarchy {
            return selected;
        }

        let element_ids = selected
            .iter()
            .enumerate()
            .map(|(index, element)| format!("ocr-p{}-e{}", element.page_number, index + 1))
            .collect::<Vec<_>>();
        for (element, element_id) in selected.iter_mut().zip(&element_ids) {
            element.backend_metadata.insert(
                OCR_ELEMENT_ID_METADATA_KEY.to_string(),
                serde_json::Value::String(element_id.clone()),
            );
        }
        for child_index in 0..selected.len() {
            if let Some(parent_index) = hierarchy_parent_index(&selected, child_index) {
                selected[child_index].parent_id = Some(element_ids[parent_index].clone());
            }
        }
        selected
    }
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline", feature = "heuristics"))]
pub(crate) fn element_level_rank(level: OcrElementLevel) -> u8 {
    match level {
        OcrElementLevel::Word => 0,
        OcrElementLevel::Line => 1,
        OcrElementLevel::Block => 2,
        OcrElementLevel::Page => 3,
    }
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
fn hierarchy_parent_index(elements: &[OcrElement], child_index: usize) -> Option<usize> {
    let child = &elements[child_index];
    let child_rank = element_level_rank(child.level);
    let child_bounds = geometry_bounds(&child.geometry)?;
    elements
        .iter()
        .enumerate()
        .filter(|(_, candidate)| candidate.page_number == child.page_number)
        .filter_map(|(index, candidate)| {
            let candidate_rank = element_level_rank(candidate.level);
            if candidate_rank <= child_rank {
                return None;
            }
            let candidate_bounds = geometry_bounds(&candidate.geometry)?;
            bounds_contain(candidate_bounds, child_bounds).then_some((
                index,
                candidate_rank - child_rank,
                candidate_bounds.2 * candidate_bounds.3,
            ))
        })
        .min_by_key(|(index, rank_distance, area)| (*rank_distance, *area, *index))
        .map(|(index, _, _)| index)
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
fn geometry_bounds(geometry: &OcrBoundingGeometry) -> Option<(u64, u64, u64, u64)> {
    match geometry {
        OcrBoundingGeometry::Rectangle {
            left,
            top,
            width,
            height,
        } => (*width > 0 && *height > 0).then_some((
            u64::from(*left),
            u64::from(*top),
            u64::from(*width),
            u64::from(*height),
        )),
        OcrBoundingGeometry::Quadrilateral { points } => {
            let left = u64::from(points.iter().map(|point| point.x).min()?);
            let top = u64::from(points.iter().map(|point| point.y).min()?);
            let right = u64::from(points.iter().map(|point| point.x).max()?);
            let bottom = u64::from(points.iter().map(|point| point.y).max()?);
            (right > left && bottom > top).then_some((left, top, right - left, bottom - top))
        }
    }
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
fn bounds_contain(parent: (u64, u64, u64, u64), child: (u64, u64, u64, u64)) -> bool {
    parent.0 <= child.0
        && parent.1 <= child.1
        && parent.0 + parent.2 >= child.0 + child.2
        && parent.1 + parent.3 >= child.1 + child.3
}

/// Geometry-only tests that do not require the `ocr` feature.
#[cfg(all(test, any(paddle_ocr, feature = "layout-detection")))]
mod geometry_tests {
    use super::*;

    #[test]
    fn test_rectangle_to_aabb() {
        let geom = OcrBoundingGeometry::Rectangle {
            left: 10,
            top: 20,
            width: 100,
            height: 50,
        };
        assert_eq!(geom.to_aabb(), (10, 20, 100, 50));
    }

    #[test]
    fn test_quadrilateral_to_aabb() {
        let geom = OcrBoundingGeometry::Quadrilateral {
            points: [(10, 22), (108, 20), (110, 72), (12, 74)]
                .into_iter()
                .map(Into::into)
                .collect(),
        };
        let (left, top, width, height) = geom.to_aabb();
        assert_eq!(left, 10);
        assert_eq!(top, 20);
        assert_eq!(width, 100);
        assert_eq!(height, 54);
    }

    #[cfg(feature = "layout-detection")]
    #[test]
    fn test_geometry_center() {
        let geom = OcrBoundingGeometry::Rectangle {
            left: 0,
            top: 0,
            width: 100,
            height: 50,
        };
        let (cx, cy) = geom.center();
        assert!((cx - 50.0).abs() < 0.001);
        assert!((cy - 25.0).abs() < 0.001);
    }
}

#[cfg(test)]
mod binding_value_serde_tests {
    use super::{OcrBoundingGeometry, OcrPoint};
    use serde_json::json;

    #[cfg(feature = "api")]
    #[test]
    fn should_describe_ocr_point_as_named_object_schema() {
        let schema =
            serde_json::to_value(<OcrPoint as utoipa::PartialSchema>::schema()).expect("schema must serialize");
        assert_eq!(schema["type"], "object");
        assert_eq!(schema["properties"]["x"]["type"], "integer");
        assert_eq!(schema["properties"]["y"]["type"], "integer");
        let required = schema["required"].as_array().expect("schema must have required fields");
        assert_eq!(required.len(), 2);
        assert!(required.contains(&json!("x")));
        assert!(required.contains(&json!("y")));
    }

    #[test]
    fn should_still_accept_legacy_positional_array_for_ocr_point() {
        let legacy = json!([10, 20]);
        let positional: OcrPoint = serde_json::from_value(legacy).expect("legacy OCR point must deserialize");

        assert_eq!(positional, OcrPoint { x: 10, y: 20 });
    }

    #[test]
    fn should_serialize_ocr_point_as_named_object() {
        let named: OcrPoint =
            serde_json::from_value(json!({"x": 10, "y": 20})).expect("named OCR point must deserialize");

        assert_eq!(named, OcrPoint { x: 10, y: 20 });
        assert_eq!(
            serde_json::to_value(named).expect("named OCR point must serialize"),
            json!({"x": 10, "y": 20})
        );
    }

    #[test]
    fn should_still_accept_legacy_positional_array_for_quadrilateral_points() {
        let legacy = json!({
            "type": "quadrilateral",
            "points": [[10, 20], [100, 22], [98, 70], [8, 68]]
        });
        let geometry: OcrBoundingGeometry = serde_json::from_value(legacy).expect("legacy geometry must deserialize");
        let OcrBoundingGeometry::Quadrilateral { points } = &geometry else {
            panic!("expected quadrilateral geometry");
        };

        assert_eq!(points[0], OcrPoint { x: 10, y: 20 });
        assert_eq!(points[1], OcrPoint { x: 100, y: 22 });
        assert_eq!(points[2], OcrPoint { x: 98, y: 70 });
        assert_eq!(points[3], OcrPoint { x: 8, y: 68 });
    }

    #[test]
    fn should_serialize_quadrilateral_points_as_named_objects() {
        let geometry: OcrBoundingGeometry = serde_json::from_value(json!({
            "type": "quadrilateral",
            "points": [
                {"x": 10, "y": 20},
                {"x": 100, "y": 22},
                {"x": 98, "y": 70},
                {"x": 8, "y": 68}
            ]
        }))
        .expect("named geometry must deserialize");

        assert_eq!(
            serde_json::to_value(geometry).expect("geometry must serialize"),
            json!({
                "type": "quadrilateral",
                "points": [
                    {"x": 10, "y": 20},
                    {"x": 100, "y": 22},
                    {"x": 98, "y": 70},
                    {"x": 8, "y": 68}
                ]
            })
        );
    }

    #[test]
    fn should_reject_quadrilateral_with_invalid_point_count() {
        let error = serde_json::from_value::<OcrBoundingGeometry>(json!({
            "type": "quadrilateral",
            "points": [[0, 0], [10, 0], [10, 10]]
        }))
        .expect_err("quadrilateral with three points must be rejected");

        assert!(
            error.to_string().contains("exactly four quadrilateral points"),
            "unexpected deserialization error: {error}"
        );
    }
}

#[cfg(all(test, feature = "ocr"))]
mod tests {
    use super::*;

    fn positioned_element(
        text: &str,
        level: OcrElementLevel,
        left: u32,
        top: u32,
        width: u32,
        height: u32,
    ) -> OcrElement {
        OcrElement {
            text: text.to_string(),
            geometry: OcrBoundingGeometry::Rectangle {
                left,
                top,
                width,
                height,
            },
            level,
            confidence: OcrConfidence {
                detection: None,
                recognition: 1.0,
            },
            ..Default::default()
        }
    }

    #[test]
    fn should_omit_hierarchy_when_not_requested() {
        let mut element = positioned_element("word", OcrElementLevel::Word, 0, 0, 10, 5);
        element.parent_id = Some("stale-parent".to_string());
        element.backend_metadata.insert(
            OCR_ELEMENT_ID_METADATA_KEY.to_string(),
            serde_json::json!("stale-element"),
        );
        let config = OcrElementConfig {
            include_elements: true,
            min_level: OcrElementLevel::Word,
            min_confidence: 0.0,
            build_hierarchy: false,
        };

        let selected = config.select_elements(&[element]);

        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].parent_id, None);
        assert_eq!(selected[0].element_id(), None);
    }

    #[test]
    fn should_emit_only_resolvable_hierarchy_references() {
        let elements = vec![
            positioned_element("page", OcrElementLevel::Page, 0, 0, 100, 100),
            positioned_element("block", OcrElementLevel::Block, 5, 5, 90, 90),
            positioned_element("line", OcrElementLevel::Line, 10, 10, 80, 20),
            positioned_element("word", OcrElementLevel::Word, 15, 15, 10, 5),
            positioned_element("orphan", OcrElementLevel::Word, 150, 150, 10, 5),
        ];
        let config = OcrElementConfig {
            include_elements: true,
            min_level: OcrElementLevel::Word,
            min_confidence: 0.0,
            build_hierarchy: true,
        };

        let selected = config.select_elements(&elements);
        let repeated = config.select_elements(&elements);
        let element_ids = selected
            .iter()
            .map(|element| element.element_id().expect("hierarchy-enabled element ID"))
            .collect::<std::collections::HashSet<_>>();

        assert_eq!(element_ids.len(), selected.len());
        assert_eq!(
            selected.iter().map(OcrElement::element_id).collect::<Vec<_>>(),
            repeated.iter().map(OcrElement::element_id).collect::<Vec<_>>()
        );
        assert!(selected.iter().all(|element| {
            element
                .parent_id
                .as_deref()
                .is_none_or(|parent_id| element_ids.contains(parent_id))
        }));
        assert_eq!(selected[3].parent_id.as_deref(), selected[2].element_id());
        assert_eq!(selected[4].parent_id, None);
    }

    #[test]
    fn test_confidence_from_tesseract() {
        let conf = OcrConfidence::from_tesseract(85.0);
        assert!(conf.detection.is_none());
        assert!((conf.recognition - 0.85).abs() < 0.001);
    }

    #[cfg(paddle_ocr)]
    #[test]
    fn test_confidence_from_paddle() {
        let conf = OcrConfidence::from_paddle(0.95, 0.88);
        assert!(conf.detection.is_some());
        assert!((conf.detection.unwrap() - 0.95).abs() < 0.001);
        assert!((conf.recognition - 0.88).abs() < 0.001);
    }

    #[cfg(paddle_ocr)]
    #[test]
    fn test_rotation_from_paddle() {
        let rot = OcrRotation::from_paddle(1, 0.92).expect("Valid angle_index");
        assert_eq!(rot.angle_degrees, 180.0);
        assert!(rot.confidence.is_some());
        assert!((rot.confidence.unwrap() - 0.92).abs() < 0.001);
    }

    #[cfg(paddle_ocr)]
    #[test]
    fn test_rotation_from_paddle_invalid_angle_index() {
        assert!(OcrRotation::from_paddle(-1, 0.92).is_err());
        assert!(OcrRotation::from_paddle(4, 0.92).is_err());
        assert!(OcrRotation::from_paddle(100, 0.92).is_err());

        assert!(OcrRotation::from_paddle(0, 0.92).is_ok());
        assert!(OcrRotation::from_paddle(1, 0.92).is_ok());
        assert!(OcrRotation::from_paddle(2, 0.92).is_ok());
        assert!(OcrRotation::from_paddle(3, 0.92).is_ok());
    }

    #[test]
    fn test_element_level_from_tesseract() {
        assert_eq!(OcrElementLevel::from_tesseract_level(1), OcrElementLevel::Page);
        assert_eq!(OcrElementLevel::from_tesseract_level(2), OcrElementLevel::Block);
        assert_eq!(OcrElementLevel::from_tesseract_level(3), OcrElementLevel::Block);
        assert_eq!(OcrElementLevel::from_tesseract_level(4), OcrElementLevel::Line);
        assert_eq!(OcrElementLevel::from_tesseract_level(5), OcrElementLevel::Word);
    }

    #[test]
    fn test_ocr_element_builder() {
        let geom = OcrBoundingGeometry::Rectangle {
            left: 0,
            top: 0,
            width: 100,
            height: 20,
        };
        let conf = OcrConfidence::from_tesseract(90.0);

        let element = OcrElement::new("Hello", geom, conf)
            .with_level(OcrElementLevel::Word)
            .with_page_number(2)
            .with_metadata("backend", serde_json::json!("tesseract"));

        assert_eq!(element.text, "Hello");
        assert_eq!(element.level, OcrElementLevel::Word);
        assert_eq!(element.page_number, 2);
        assert!(element.backend_metadata.contains_key("backend"));
    }

    #[test]
    fn test_geometry_overlaps() {
        let geom1 = OcrBoundingGeometry::Rectangle {
            left: 0,
            top: 0,
            width: 100,
            height: 50,
        };
        let geom2 = OcrBoundingGeometry::Rectangle {
            left: 50,
            top: 25,
            width: 100,
            height: 50,
        };
        let geom3 = OcrBoundingGeometry::Rectangle {
            left: 200,
            top: 0,
            width: 50,
            height: 50,
        };

        assert!(geom1.overlaps(&geom2));
        assert!(!geom1.overlaps(&geom3));
    }

    #[cfg(paddle_ocr)]
    #[test]
    fn test_serialization_roundtrip() {
        let geom = OcrBoundingGeometry::Quadrilateral {
            points: [(10, 20), (100, 22), (98, 70), (8, 68)]
                .into_iter()
                .map(Into::into)
                .collect(),
        };
        let conf = OcrConfidence::from_paddle(0.95, 0.88);
        let rot = OcrRotation::from_paddle(0, 0.99).expect("Valid angle_index");

        let element = OcrElement::new("Test text", geom, conf)
            .with_rotation(rot)
            .with_level(OcrElementLevel::Line);

        let json = serde_json::to_string(&element).expect("Failed to serialize");
        let deserialized: OcrElement = serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(deserialized.text, element.text);
        assert_eq!(deserialized.level, element.level);
        assert!(deserialized.rotation.is_some());
    }
}
