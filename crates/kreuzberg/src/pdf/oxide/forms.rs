//! PDF form field extraction using the pdf_oxide backend.
//!
//! Maps pdf_oxide's `FormField` types to Kreuzberg's `PdfFormField` model,
//! extracting field names, types, values, bounding boxes, and metadata.
//! Supports both AcroForm and XFA-based forms.

use super::OxideDocument;
use crate::types::{BoundingBox, PdfFormField, FormFieldType};

/// Extract form fields from a PDF document using pdf_oxide.
///
/// Calls `FormExtractor::extract_fields` to get all AcroForm fields from the document,
/// then maps each field's type, value, and bounding box to Kreuzberg's types.
///
/// # Arguments
///
/// * `doc` - The opened PDF document
///
/// # Returns
///
/// A `Vec<PdfFormField>` containing all successfully extracted form fields,
/// or an empty vector if the document has no forms or extraction fails.
pub(crate) fn extract_form_fields(doc: &OxideDocument) -> Vec<PdfFormField> {
    match pdf_oxide::extractors::forms::FormExtractor::extract_fields(&doc.doc) {
        Ok(oxide_fields) => {
            oxide_fields
                .into_iter()
                .map(map_form_field)
                .collect()
        }
        Err(e) => {
            tracing::debug!("pdf_oxide form field extraction failed: {e}");
            Vec::new()
        }
    }
}

/// Maps a single pdf_oxide FormField to a Kreuzberg PdfFormField.
///
/// Converts all field properties including type, value, bounds, and metadata.
/// Type mapping and value extraction handle various field subtypes (text, checkbox, radio, choice, signature, button).
fn map_form_field(oxide_field: pdf_oxide::extractors::forms::FormField) -> PdfFormField {
    let field_type = map_field_type(&oxide_field.field_type);
    let value = map_field_value(&oxide_field.value);
    let default_value = oxide_field
        .default_value
        .as_ref()
        .and_then(map_field_value);
    let bbox = oxide_field.bounds.map(|bounds| BoundingBox {
        x0: bounds[0],
        y0: bounds[1],
        x1: bounds[2],
        y1: bounds[3],
    });

    PdfFormField {
        name: oxide_field.name,
        full_name: oxide_field.full_name,
        field_type,
        value,
        default_value,
        flags: oxide_field.flags.unwrap_or(0),
        page: None, // Page assignment is done downstream via spatial analysis
        bbox,
        max_length: oxide_field.max_length,
        tooltip: oxide_field.tooltip,
    }
}

/// Maps pdf_oxide's `FieldType` to Kreuzberg's `FormFieldType`.
///
/// Matches the PDF form field type hierarchy:
/// - Button (/Btn) includes checkboxes, radio buttons, and push buttons
/// - Text (/Tx) is single- or multi-line text input
/// - Choice (/Ch) is dropdown or list box
/// - Signature (/Sig) is digital signature
/// - Unknown for unrecognized types
fn map_field_type(oxide_type: &pdf_oxide::extractors::forms::FieldType) -> FormFieldType {
    use pdf_oxide::extractors::forms::FieldType;
    match oxide_type {
        FieldType::Button => {
            // Further classification of button subtypes (checkbox, radio, push) happens
            // via the field flags (PUSH_BUTTON, RADIO, etc.). For the basic type, we
            // return Button and let downstream code inspect flags if needed.
            FormFieldType::Button
        }
        FieldType::Text => FormFieldType::Text,
        FieldType::Choice => FormFieldType::Choice,
        FieldType::Signature => FormFieldType::Signature,
        FieldType::Unknown(_) => FormFieldType::Unknown,
    }
}

/// Maps pdf_oxide's `FieldValue` to a String representation.
///
/// Handles:
/// - Text: returned as-is
/// - Boolean: converted to "true" / "false"
/// - Name: value returned as-is (used for radio buttons, dropdown selections)
/// - Array: values joined with ", " (multi-select list boxes)
/// - None: returns None
fn map_field_value(oxide_value: &pdf_oxide::extractors::forms::FieldValue) -> Option<String> {
    use pdf_oxide::extractors::forms::FieldValue;
    match oxide_value {
        FieldValue::Text(s) => Some(s.clone()),
        FieldValue::Boolean(b) => Some(b.to_string()),
        FieldValue::Name(n) => Some(n.clone()),
        FieldValue::Array(arr) => {
            if arr.is_empty() {
                None
            } else {
                Some(arr.join(", "))
            }
        }
        FieldValue::None => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdf_oxide::extractors::forms::{FieldType, FieldValue};

    #[test]
    fn test_map_field_type_button() {
        assert_eq!(map_field_type(&FieldType::Button), FormFieldType::Button);
    }

    #[test]
    fn test_map_field_type_text() {
        assert_eq!(map_field_type(&FieldType::Text), FormFieldType::Text);
    }

    #[test]
    fn test_map_field_type_choice() {
        assert_eq!(map_field_type(&FieldType::Choice), FormFieldType::Choice);
    }

    #[test]
    fn test_map_field_type_signature() {
        assert_eq!(map_field_type(&FieldType::Signature), FormFieldType::Signature);
    }

    #[test]
    fn test_map_field_type_unknown() {
        assert_eq!(map_field_type(&FieldType::Unknown("custom".to_string())), FormFieldType::Unknown);
    }

    #[test]
    fn test_map_field_value_text() {
        let value = FieldValue::Text("hello".to_string());
        assert_eq!(map_field_value(&value), Some("hello".to_string()));
    }

    #[test]
    fn test_map_field_value_boolean_true() {
        let value = FieldValue::Boolean(true);
        assert_eq!(map_field_value(&value), Some("true".to_string()));
    }

    #[test]
    fn test_map_field_value_boolean_false() {
        let value = FieldValue::Boolean(false);
        assert_eq!(map_field_value(&value), Some("false".to_string()));
    }

    #[test]
    fn test_map_field_value_name() {
        let value = FieldValue::Name("Yes".to_string());
        assert_eq!(map_field_value(&value), Some("Yes".to_string()));
    }

    #[test]
    fn test_map_field_value_array() {
        let value = FieldValue::Array(vec!["option1".to_string(), "option2".to_string()]);
        assert_eq!(map_field_value(&value), Some("option1, option2".to_string()));
    }

    #[test]
    fn test_map_field_value_empty_array() {
        let value = FieldValue::Array(vec![]);
        assert_eq!(map_field_value(&value), None);
    }

    #[test]
    fn test_map_field_value_none() {
        let value = FieldValue::None;
        assert_eq!(map_field_value(&value), None);
    }
}
