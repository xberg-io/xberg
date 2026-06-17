//! PDF form (AcroForm / XFA) field extracted from a document.

use serde::{Deserialize, Serialize};

use super::extraction::BoundingBox;

/// Kind of a PDF form field.
///
/// Mirrors `pdf_oxide`'s widget field taxonomy without leaking the upstream
/// type across the binding surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum FormFieldType {
    /// Single- or multi-line text input.
    Text,
    /// Checkbox (on/off toggle).
    Checkbox,
    /// Radio-button group member.
    Radio,
    /// Choice field (dropdown or list box).
    Choice,
    /// Digital-signature field.
    Signature,
    /// Push button.
    Button,
    /// Field type that could not be classified.
    #[default]
    Unknown,
}

/// A form field extracted from a PDF's AcroForm or XFA structure.
///
/// Populated by the PDF extractor when [`PdfConfig::extract_form_fields`] is
/// enabled and the document is a fillable form. The collection is empty for
/// non-form PDFs and for non-PDF formats.
///
/// [`PdfConfig::extract_form_fields`]: crate::core::config::PdfConfig::extract_form_fields
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct PdfFormField {
    /// Partial field name (the leaf name within the field hierarchy).
    pub name: String,

    /// Fully-qualified field name (dotted path from the form root).
    pub full_name: String,

    /// Classified field type.
    pub field_type: FormFieldType,

    /// Current field value, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,

    /// Default field value, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_value: Option<String>,

    /// Raw field-flags bitmask (read-only, required, multiline, …).
    #[serde(default)]
    pub flags: u32,

    /// 1-indexed page the field's widget appears on, if known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,

    /// Widget bounding box on its page, if known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bbox: Option<BoundingBox>,

    /// Maximum input length for text fields, if specified.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_length: Option<u32>,

    /// Tooltip / alternate field description, if present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tooltip: Option<String>,
}
