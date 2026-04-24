//! Bounding box geometry for PDF text positioning.
//!
//! This module provides the BoundingBox type and geometric operations used
//! for spatial analysis of text elements in PDF documents.

use serde::{Deserialize, Serialize};

/// A bounding box for text or elements.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct BoundingBox {
    /// Left x-coordinate
    pub left: f32,
    /// Top y-coordinate
    pub top: f32,
    /// Right x-coordinate
    pub right: f32,
    /// Bottom y-coordinate
    pub bottom: f32,
}

impl BoundingBox {
    /// Create a new bounding box with zero-size validation.
    ///
    /// # Arguments
    ///
    /// * `left` - Left x-coordinate
    /// * `top` - Top y-coordinate
    /// * `right` - Right x-coordinate
    /// * `bottom` - Bottom y-coordinate
    ///
    /// # Returns
    ///
    /// `Ok(BoundingBox)` if the box has non-zero area, or
    /// `Err` if the box has zero width or height
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Width (`right - left`) is less than 1e-10 (near-zero)
    /// - Height (`bottom - top`) is less than 1e-10 (near-zero)
    pub(crate) fn new(left: f32, top: f32, right: f32, bottom: f32) -> std::result::Result<BoundingBox, String> {
        let width = (right - left).abs();
        let height = (bottom - top).abs();

        if width < 1e-10 || height < 1e-10 {
            return Err(format!(
                "BoundingBox has zero or near-zero area: width={}, height={}",
                width, height
            ));
        }

        Ok(BoundingBox {
            left,
            top,
            right,
            bottom,
        })
    }

    /// Create a new bounding box without validation (unchecked).
    ///
    /// This is useful when you know the coordinates are valid or want to
    /// defer validation. Use with caution - invalid boxes may cause issues
    /// in calculations like area, width, and height.
    ///
    /// # Arguments
    ///
    /// * `left` - Left x-coordinate
    /// * `top` - Top y-coordinate
    /// * `right` - Right x-coordinate
    /// * `bottom` - Bottom y-coordinate
    ///
    /// # Returns
    ///
    /// A BoundingBox without any validation
    pub(crate) fn new_unchecked(left: f32, top: f32, right: f32, bottom: f32) -> BoundingBox {
        BoundingBox {
            left,
            top,
            right,
            bottom,
        }
    }

    /// Get the width of the bounding box.
    ///
    /// # Returns
    ///
    /// The width (right - left). No absolute value is taken as
    /// the BoundingBox::new() constructor ensures correct ordering.
    pub(crate) fn width(&self) -> f32 {
        self.right - self.left
    }

    /// Get the height of the bounding box.
    ///
    /// # Returns
    ///
    /// The height (bottom - top). No absolute value is taken as
    /// the BoundingBox::new() constructor ensures correct ordering.
    pub(crate) fn height(&self) -> f32 {
        self.bottom - self.top
    }

    /// Calculate the intersection ratio relative to this bounding box's area.
    ///
    /// intersection_ratio = intersection_area / self_area
    ///
    /// # Arguments
    ///
    /// * `other` - The other bounding box to compare with
    ///
    /// # Returns
    ///
    /// The intersection ratio between 0.0 and 1.0
    pub(crate) fn intersection_ratio(&self, other: &BoundingBox) -> f32 {
        let intersection_area = self.calculate_intersection_area(other);
        let self_area = self.calculate_area();

        if self_area <= 0.0 {
            0.0
        } else {
            intersection_area / self_area
        }
    }

    /// Calculate the center coordinates of this bounding box.
    pub(crate) fn center(&self) -> (f32, f32) {
        ((self.left + self.right) / 2.0, (self.top + self.bottom) / 2.0)
    }

    /// Calculate the area of this bounding box.
    fn calculate_area(&self) -> f32 {
        let width = (self.right - self.left).max(0.0);
        let height = (self.bottom - self.top).max(0.0);
        width * height
    }

    /// Calculate the intersection area between this bounding box and another.
    fn calculate_intersection_area(&self, other: &BoundingBox) -> f32 {
        let left = self.left.max(other.left);
        let top = self.top.max(other.top);
        let right = self.right.min(other.right);
        let bottom = self.bottom.min(other.bottom);

        let width = (right - left).max(0.0);
        let height = (bottom - top).max(0.0);
        width * height
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bounding_box_new_valid() {
        let bbox = BoundingBox::new(10.0, 20.0, 30.0, 40.0);
        assert!(bbox.is_ok());
        let bbox = bbox.unwrap();
        assert_eq!(bbox.width(), 20.0);
        assert_eq!(bbox.height(), 20.0);
    }

    #[test]
    fn test_bounding_box_new_zero_width() {
        let bbox = BoundingBox::new(10.0, 20.0, 10.0, 40.0);
        assert!(bbox.is_err());
        let error_msg = bbox.unwrap_err();
        assert!(error_msg.contains("zero or near-zero area"));
    }

    #[test]
    fn test_bounding_box_new_zero_height() {
        let bbox = BoundingBox::new(10.0, 20.0, 30.0, 20.0);
        assert!(bbox.is_err());
        let error_msg = bbox.unwrap_err();
        assert!(error_msg.contains("zero or near-zero area"));
    }

    #[test]
    fn test_bounding_box_new_unchecked() {
        let bbox = BoundingBox::new_unchecked(10.0, 20.0, 30.0, 40.0);
        assert_eq!(bbox.width(), 20.0);
        assert_eq!(bbox.height(), 20.0);
    }

    #[test]
    fn test_bounding_box_width_and_height() {
        let bbox = BoundingBox {
            left: 5.0,
            top: 10.0,
            right: 25.0,
            bottom: 50.0,
        };
        assert_eq!(bbox.width(), 20.0);
        assert_eq!(bbox.height(), 40.0);
    }
}
