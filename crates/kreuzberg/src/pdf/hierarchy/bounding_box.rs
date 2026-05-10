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
