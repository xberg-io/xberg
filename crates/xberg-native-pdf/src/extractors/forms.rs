//! AcroForm field extraction.
//!
//! Extracts form fields from PDF documents that use AcroForms (Interactive Forms).
//! See ISO 32000-1:2008, Section 12.7 - Interactive Forms.

use crate::document::PdfDocument;
use crate::error::{Error, Result};
use crate::object::{Object, ObjectRef};
use std::collections::HashSet;

const MAX_FIELD_DEPTH: usize = 64;
const MAX_FIELD_COUNT: usize = 100_000;
const MAX_PARENT_DEPTH: usize = 64;

/// Traversal state shared across one AcroForm field-tree walk. ~keep
#[derive(Default)]
struct FieldWalk {
    visited: HashSet<ObjectRef>,
    visited_fields: usize,
    /// Keep choice values as export values (what FDF/XFDF `/V` must carry) instead of display text.
    export_values: bool,
}

/// A form field extracted from a PDF AcroForm.
#[derive(Debug, Clone)]
pub struct FormField {
    /// Field name from /T key
    pub name: String,
    /// Field type from /FT key
    pub field_type: FieldType,
    /// Field value from /V key
    pub value: FieldValue,
    /// Tooltip/description from /TU key
    pub tooltip: Option<String>,
    /// Full qualified name (for hierarchical fields)
    pub full_name: String,
    /// Field bounding box from /Rect key [x1, y1, x2, y2]
    pub bounds: Option<[f64; 4]>,

    /// Object reference for updating existing fields
    pub object_ref: Option<ObjectRef>,
    /// Field flags from /Ff key (ReadOnly, Required, NoExport, etc.)
    pub flags: Option<u32>,
    /// Default value from /DV key
    pub default_value: Option<FieldValue>,
    /// Maximum length for text fields from /MaxLen key
    pub max_length: Option<u32>,
    /// Text alignment from /Q key (0=left, 1=center, 2=right)
    pub alignment: Option<u32>,
    /// Default appearance string from /DA key
    pub default_appearance: Option<String>,
    /// Border style from /BS key
    pub border_style: Option<BorderStyle>,
    /// Appearance characteristics from /MK key
    pub appearance_chars: Option<AppearanceCharacteristics>,
}

/// Per-field attributes resolved by [`FormExtractor::extract_field_attributes`],
/// mirroring the subset of [`FormField`] that doesn't depend on name resolution
/// or /Kids recursion. Internal to the extraction pass. ~keep
struct FieldAttributes {
    value: FieldValue,
    tooltip: Option<String>,
    bounds: Option<[f64; 4]>,
    flags: Option<u32>,
    default_value: Option<FieldValue>,
    max_length: Option<u32>,
    alignment: Option<u32>,
    default_appearance: Option<String>,
    border_style: Option<BorderStyle>,
    appearance_chars: Option<AppearanceCharacteristics>,
}

/// Field type from /FT key in field dictionary.
#[derive(Debug, Clone, PartialEq)]
pub enum FieldType {
    /// Button field (/Btn) - checkbox, radio button, push button
    Button,
    /// Text field (/Tx) - single or multi-line text
    Text,
    /// Choice field (/Ch) - list box or combo box
    Choice,
    /// Signature field (/Sig)
    Signature,
    /// Unknown/unrecognized field type
    Unknown(String),
}

/// Field value from /V key in field dictionary.
#[derive(Debug, Clone, PartialEq)]
pub enum FieldValue {
    /// Text string value
    Text(String),
    /// Boolean value (for checkboxes)
    Boolean(bool),
    /// Name value (for radio buttons, choice fields)
    Name(String),
    /// Array of values (for multi-select list boxes)
    Array(Vec<String>),
    /// No value present
    None,
}

/// Border style from /BS dictionary (PDF Table 166).
#[derive(Debug, Clone, PartialEq)]
pub struct BorderStyle {
    /// Border width in points
    pub width: f32,
    /// Border style type
    pub style: BorderStyleType,
    /// Dash pattern for dashed borders
    pub dash_array: Option<Vec<u32>>,
}

impl Default for BorderStyle {
    fn default() -> Self {
        Self {
            width: 1.0,
            style: BorderStyleType::Solid,
            dash_array: None,
        }
    }
}

/// Border style type from /S key in /BS dictionary.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum BorderStyleType {
    /// Solid border
    #[default]
    Solid,
    /// Dashed border
    Dashed,
    /// Beveled border (3D effect, raised)
    Beveled,
    /// Inset border (3D effect, recessed)
    Inset,
    /// Underline only
    Underline,
}

impl BorderStyleType {
    /// Parse from PDF name.
    pub fn from_pdf_name(name: &str) -> Self {
        match name {
            "S" => BorderStyleType::Solid,
            "D" => BorderStyleType::Dashed,
            "B" => BorderStyleType::Beveled,
            "I" => BorderStyleType::Inset,
            "U" => BorderStyleType::Underline,
            _ => BorderStyleType::Solid,
        }
    }

    /// Convert to PDF name.
    pub fn to_pdf_name(&self) -> &'static str {
        match self {
            BorderStyleType::Solid => "S",
            BorderStyleType::Dashed => "D",
            BorderStyleType::Beveled => "B",
            BorderStyleType::Inset => "I",
            BorderStyleType::Underline => "U",
        }
    }
}

/// Appearance characteristics from /MK dictionary (PDF Table 189).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AppearanceCharacteristics {
    /// Background color (BG) - RGB values 0.0-1.0
    pub background_color: Option<[f32; 3]>,
    /// Border color (BC) - RGB values 0.0-1.0
    pub border_color: Option<[f32; 3]>,
    /// Normal caption (CA) - text shown on button
    pub caption: Option<String>,
    /// Rollover caption (RC) - text shown when hovering
    pub rollover_caption: Option<String>,
    /// Alternate caption (AC) - text shown when pressed
    pub alternate_caption: Option<String>,
    /// Rotation angle in degrees (R)
    pub rotation: Option<u32>,
}

/// Field flag constants from PDF Table 221.
pub mod field_flags {
    /// Field is read-only (bit 1)
    pub const READ_ONLY: u32 = 1;
    /// Field is required (bit 2)
    pub const REQUIRED: u32 = 1 << 1;
    /// Field should not be exported (bit 3)
    pub const NO_EXPORT: u32 = 1 << 2;

    // Text field flags (PDF Table 228) ~keep
    /// Text field allows multiple lines (bit 13)
    pub const MULTILINE: u32 = 1 << 12;
    /// Text field is a password field (bit 14)
    pub const PASSWORD: u32 = 1 << 13;
    /// Text field does not scroll (bit 24)
    pub const DO_NOT_SCROLL: u32 = 1 << 23;
    /// Text field allows comb formatting (bit 25)
    pub const COMB: u32 = 1 << 24;
    /// Text field is a rich text field (bit 26)
    pub const RICH_TEXT: u32 = 1 << 25;

    // Button field flags (PDF Table 226) ~keep
    /// Button is a push button (bit 17)
    pub const PUSH_BUTTON: u32 = 1 << 16;
    /// Radio button (bit 16)
    pub const RADIO: u32 = 1 << 15;
    /// Radio buttons in group are exclusive (bit 26)
    pub const RADIOS_IN_UNISON: u32 = 1 << 25;

    // Choice field flags (PDF Table 230) ~keep
    /// Choice is a combo box (bit 18)
    pub const COMBO: u32 = 1 << 17;
    /// Choice field is editable (bit 19)
    pub const EDIT: u32 = 1 << 18;
    /// Choice field is sorted (bit 20)
    pub const SORT: u32 = 1 << 19;
    /// Choice field allows multiple selection (bit 22)
    pub const MULTI_SELECT: u32 = 1 << 21;
    /// Do not spell check (bit 23)
    pub const DO_NOT_SPELL_CHECK: u32 = 1 << 22;
    /// Commit on change (bit 27)
    pub const COMMIT_ON_SEL_CHANGE: u32 = 1 << 26;
}

/// AcroForm extractor.
pub struct FormExtractor;

impl FormExtractor {
    /// Helper function to resolve an Object (handles indirect references).
    ///
    /// If the object is an indirect reference, loads it. Otherwise returns clone.
    fn resolve_object(doc: &PdfDocument, obj: &Object) -> Result<Object> {
        if let Some(ref_val) = obj.as_reference() {
            doc.load_object(ref_val)
        } else {
            Ok(obj.clone())
        }
    }

    /// Decode a PDF string that may be UTF-16BE (with BOM) or PDFDocEncoding.
    ///
    /// Per ISO 32000-1:2008, Section 7.9.2.2 - Text String Type:
    /// - If bytes start with 0xFE 0xFF, the string is UTF-16BE with BOM
    /// - Otherwise, it's PDFDocEncoding (superset of ISO Latin-1)
    fn decode_text_string(bytes: &[u8]) -> Option<String> {
        if bytes.len() >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF {
            let utf16_bytes = &bytes[2..];

            let utf16_pairs: Vec<u16> = utf16_bytes
                .as_chunks::<2>()
                .0
                .iter()
                .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
                .collect();

            String::from_utf16(&utf16_pairs).ok()
        } else {
            // PDFDocEncoding - use proper character mapping
            // ISO 32000-1:2008, Appendix D.2, Table D.2 ~keep
            Some(
                bytes
                    .iter()
                    .filter_map(|&b| crate::fonts::font_dict::pdfdoc_encoding_lookup(b))
                    .collect(),
            )
        }
    }

    /// Extract all form fields from a PDF document.
    ///
    /// This function:
    /// 1. Gets the document catalog
    /// 2. Looks for /AcroForm dictionary
    /// 3. Extracts /Fields array
    /// 4. Recursively processes field hierarchy
    ///
    /// # Arguments
    ///
    /// * `doc` - The PDF document to extract fields from
    ///
    /// # Returns
    ///
    /// A vector of form fields, or an error if extraction fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use xberg_native_pdf::document::PdfDocument;
    /// use xberg_native_pdf::extractors::forms::FormExtractor;
    ///
    /// let mut doc = PdfDocument::open("form.pdf")?;
    /// let fields = FormExtractor::extract_fields(&mut doc)?;
    ///
    /// for field in &fields {
    ///     println!("Field: {} = {:?}", field.name, field.value);
    /// }
    /// # Ok::<(), xberg_native_pdf::error::Error>(())
    /// ```
    #[tracing::instrument(name = "pdf.extract_fields", skip_all)]
    pub fn extract_fields(doc: &PdfDocument) -> Result<Vec<FormField>> {
        let result = Self::extract_fields_impl(doc, false);
        if let Err(error) = &result {
            crate::error::trace_failure("extract_form_fields", error);
        }
        result
    }

    fn extract_fields_impl(doc: &PdfDocument, export_values: bool) -> Result<Vec<FormField>> {
        let catalog = doc.catalog()?;
        let catalog_dict = catalog
            .as_dict()
            .ok_or_else(|| Error::InvalidPdf("Catalog is not a dictionary".to_string()))?;

        let acroform_ref = match catalog_dict.get("AcroForm") {
            Some(obj) => obj,
            None => {
                return Ok(Vec::new());
            }
        };

        let acroform = Self::resolve_object(doc, acroform_ref)?;
        let acroform_dict = acroform
            .as_dict()
            .ok_or_else(|| Error::InvalidPdf("AcroForm is not a dictionary".to_string()))?;

        let fields_ref = match acroform_dict.get("Fields") {
            Some(obj) => obj,
            None => {
                return Ok(Vec::new());
            }
        };

        let fields_obj = Self::resolve_object(doc, fields_ref)?;
        let fields_array = fields_obj
            .as_array()
            .ok_or_else(|| Error::InvalidPdf("AcroForm /Fields is not an array".to_string()))?;

        let mut result = Vec::new();
        let mut walk = FieldWalk {
            export_values,
            ..FieldWalk::default()
        };
        for field_ref in fields_array {
            if let Err(error) = Self::extract_field_recursive(doc, field_ref, "", 0, &mut walk, &mut result) {
                tracing::warn!("skipping unreadable form field: {error}");
            }
        }

        Ok(result)
    }

    /// Recursively extract a field and its children.
    ///
    /// PDF forms can have hierarchical field structure using /Kids arrays.
    /// This function handles:
    /// - Terminal fields (with /FT and /V)
    /// - Non-terminal fields (with /Kids but no /FT)
    /// - Inherited properties from parent fields
    ///
    /// # Arguments
    ///
    /// * `doc` - The PDF document
    /// * `field_ref` - Reference to the field object
    /// * `parent_name` - Full qualified name of parent field (for hierarchy)
    /// * `result` - Vector to accumulate extracted fields
    fn extract_field_recursive(
        doc: &PdfDocument,
        field_ref: &Object,
        parent_name: &str,
        depth: usize,
        walk: &mut FieldWalk,
        result: &mut Vec<FormField>,
    ) -> Result<()> {
        let object_ref = field_ref.as_reference();
        if depth > MAX_FIELD_DEPTH || walk.visited_fields >= MAX_FIELD_COUNT {
            return Ok(());
        }
        if let Some(r) = object_ref
            && !walk.visited.insert(r)
        {
            return Ok(());
        }
        walk.visited_fields += 1;

        let field = Self::resolve_object(doc, field_ref)?;
        let field_dict = match field.as_dict() {
            Some(d) => d,
            None => return Ok(()),
        };

        let (partial_name, full_name) = Self::resolve_field_names(doc, field_dict, parent_name);
        let has_kids = Self::recurse_into_kids(doc, field_dict, &full_name, depth, walk, result)?;

        let field_type = Self::inherited_attr(doc, field_dict, "FT")
            .and_then(|obj| obj.as_name().map(|s| s.to_string()))
            .map(|name| Self::parse_field_type(&name))
            .unwrap_or(FieldType::Unknown("".to_string()));

        // only skip the parent when it has NEITHER /FT
        // NOR /T. A parent with /T but no /FT (logical grouping like
        // `topmostSubform[0].Page1[0].FilingStatus[0]`) IS surfaced in
        // results, matching pypdf's behaviour. The terminal kids that
        // already got their own row carry the actual /V (checkbox
        // /Off/Yes state). ~keep
        let no_ft = matches!(field_type, FieldType::Unknown(ref s) if s.is_empty());
        if no_ft && (has_kids && partial_name.is_empty()) {
            return Ok(());
        }
        if no_ft && !has_kids && partial_name.is_empty() {
            return Ok(());
        }

        let attrs = Self::extract_field_attributes(doc, field_dict, &field_type, !walk.export_values);

        let form_field = FormField {
            name: partial_name,
            field_type,
            value: attrs.value,
            tooltip: attrs.tooltip,
            full_name,
            bounds: attrs.bounds,
            object_ref,
            flags: attrs.flags,
            default_value: attrs.default_value,
            max_length: attrs.max_length,
            alignment: attrs.alignment,
            default_appearance: attrs.default_appearance,
            border_style: attrs.border_style,
            appearance_chars: attrs.appearance_chars,
        };

        result.push(form_field);
        Ok(())
    }

    /// Resolves a field's partial name (/T) and its fully qualified name
    /// (parent name joined with '.'), matching pypdf's dotted-path
    /// convention for hierarchical AcroForm fields. ~keep
    fn resolve_field_names(
        doc: &PdfDocument,
        field_dict: &std::collections::HashMap<String, Object>,
        parent_name: &str,
    ) -> (String, String) {
        let partial_name = field_dict
            .get("T")
            .and_then(|obj| Self::resolve_object(doc, obj).ok())
            .and_then(|obj| obj.as_string().map(|s| s.to_vec()))
            .and_then(|bytes| Self::decode_text_string(&bytes))
            .unwrap_or_default();

        let full_name = if parent_name.is_empty() {
            partial_name.clone()
        } else if partial_name.is_empty() {
            parent_name.to_string()
        } else {
            format!("{}.{}", parent_name, partial_name)
        };

        (partial_name, full_name)
    }

    /// Recurses into a field's /Kids array (if any), extracting each child
    /// field. Returns whether the field carried a /Kids array at all.
    ///
    /// Recurse into Kids and also emit a row for parent fields that carry a
    /// /T name, matching pypdf's traversal. The previous behaviour recursed
    /// into Kids and dropped any parent without an explicit /FT, which
    /// under-counted IRS AcroForms by 15-30% (parent dictionaries for
    /// grouped fields like multi-digit SSN boxes have /T but no /FT — pypdf
    /// surfaces them, we did not). ~keep
    fn recurse_into_kids(
        doc: &PdfDocument,
        field_dict: &std::collections::HashMap<String, Object>,
        full_name: &str,
        depth: usize,
        walk: &mut FieldWalk,
        result: &mut Vec<FormField>,
    ) -> Result<bool> {
        let Some(kids_ref) = field_dict.get("Kids") else {
            return Ok(false);
        };
        let kids = Self::resolve_object(doc, kids_ref)?;
        let Some(kids_array) = kids.as_array() else {
            return Ok(false);
        };
        for kid_ref in kids_array {
            if let Err(error) = Self::extract_field_recursive(doc, kid_ref, full_name, depth + 1, walk, result) {
                tracing::warn!("skipping unreadable form field {full_name:?}: {error}");
            }
        }
        Ok(true)
    }

    /// Looks up an inheritable field attribute (ISO 32000-1 §12.7.3.1) on the
    /// field itself, then up its /Parent chain, bounded and cycle-safe.
    fn inherited_attr(
        doc: &PdfDocument,
        field_dict: &std::collections::HashMap<String, Object>,
        key: &str,
    ) -> Option<Object> {
        if let Some(value) = field_dict.get(key) {
            return Self::resolve_object(doc, value).ok();
        }
        let mut seen = HashSet::new();
        let mut parent = field_dict.get("Parent").cloned();
        for _ in 0..MAX_PARENT_DEPTH {
            let parent_obj = parent.take()?;
            let resolved = match parent_obj.as_reference() {
                Some(r) => {
                    if !seen.insert(r) {
                        return None;
                    }
                    doc.load_object_shared(r).ok()?
                }
                None => std::sync::Arc::new(parent_obj),
            };
            let dict = resolved.as_dict()?;
            if let Some(value) = dict.get(key) {
                return Self::resolve_object(doc, value).ok();
            }
            parent = dict.get("Parent").cloned();
        }
        None
    }

    /// Maps a choice field's export value(s) to display text through /Opt
    /// `[export display]` pairs; plain-string options and unmatched values pass through.
    fn map_choice_value(
        doc: &PdfDocument,
        field_dict: &std::collections::HashMap<String, Object>,
        value: FieldValue,
    ) -> FieldValue {
        let Some(Object::Array(opts)) = Self::inherited_attr(doc, field_dict, "Opt") else {
            return value;
        };
        let mut displays: std::collections::HashMap<String, String> = std::collections::HashMap::new();
        for opt in &opts {
            let pair = Self::resolve_object(doc, opt)
                .ok()
                .and_then(|resolved| resolved.as_array().cloned());
            let Some(pair) = pair else { continue };
            let key = pair
                .first()
                .and_then(|k| Self::resolve_object(doc, k).ok())
                .and_then(|k| k.as_string().and_then(Self::decode_text_string));
            let shown = pair
                .get(1)
                .and_then(|v| Self::resolve_object(doc, v).ok())
                .and_then(|v| v.as_string().and_then(Self::decode_text_string));
            if let (Some(key), Some(shown)) = (key, shown) {
                displays.entry(key).or_insert(shown);
            }
        }
        let display_for = |export: &str| -> Option<String> { displays.get(export).cloned() };
        match value {
            FieldValue::Text(t) => FieldValue::Text(display_for(&t).unwrap_or(t)),
            FieldValue::Array(items) => {
                FieldValue::Array(items.into_iter().map(|t| display_for(&t).unwrap_or(t)).collect())
            }
            other => other,
        }
    }

    /// Resolves the remaining per-field attributes (/V, /TU, /Rect, /Ff,
    /// /DV, /MaxLen, /Q, /DA, /BS, /MK) once a field is known to be worth
    /// surfacing. Split out of [`Self::extract_field_recursive`] purely to
    /// keep that function's control flow (name resolution, Kids recursion,
    /// the no-/FT skip check) legible on its own. ~keep
    fn extract_field_attributes(
        doc: &PdfDocument,
        field_dict: &std::collections::HashMap<String, Object>,
        field_type: &FieldType,
        map_choices: bool,
    ) -> FieldAttributes {
        let value = Self::inherited_attr(doc, field_dict, "V")
            .map(|obj| Self::parse_field_value(&obj, field_type))
            .unwrap_or(FieldValue::None);
        let value = if map_choices && *field_type == FieldType::Choice {
            Self::map_choice_value(doc, field_dict, value)
        } else {
            value
        };

        let tooltip = field_dict
            .get("TU")
            .and_then(|obj| Self::resolve_object(doc, obj).ok())
            .and_then(|obj| obj.as_string().map(|s| s.to_vec()))
            .and_then(|bytes| Self::decode_text_string(&bytes));

        let bounds = Self::parse_field_bounds(doc, field_dict);
        let flags = Self::field_dict_integer(doc, field_dict, "Ff");

        let default_value =
            Self::inherited_attr(doc, field_dict, "DV").map(|obj| Self::parse_field_value(&obj, field_type));

        let max_length = Self::field_dict_integer(doc, field_dict, "MaxLen");
        // Extract text alignment /Q (0=left, 1=center, 2=right) ~keep
        let alignment = Self::field_dict_integer(doc, field_dict, "Q");

        let default_appearance = Self::inherited_attr(doc, field_dict, "DA")
            .and_then(|obj| obj.as_string().map(|s| s.to_vec()))
            .and_then(|bytes| Self::decode_text_string(&bytes));

        let border_style = field_dict
            .get("BS")
            .and_then(|obj| Self::resolve_object(doc, obj).ok())
            .and_then(|obj| Self::parse_border_style(&obj));

        let appearance_chars = field_dict
            .get("MK")
            .and_then(|obj| Self::resolve_object(doc, obj).ok())
            .and_then(|obj| Self::parse_appearance_characteristics(doc, &obj));

        FieldAttributes {
            value,
            tooltip,
            bounds,
            flags,
            default_value,
            max_length,
            alignment,
            default_appearance,
            border_style,
            appearance_chars,
        }
    }

    /// Parses a field's /Rect bounding box, [x1, y1, x2, y2] in default user
    /// space, from an array of 4 Integer/Real objects.
    fn parse_field_bounds(
        doc: &PdfDocument,
        field_dict: &std::collections::HashMap<String, Object>,
    ) -> Option<[f64; 4]> {
        let arr = field_dict
            .get("Rect")
            .and_then(|obj| Self::resolve_object(doc, obj).ok())
            .and_then(|obj| obj.as_array().cloned())?;
        if arr.len() != 4 {
            return None;
        }
        let mut coords = Vec::with_capacity(4);
        for item in &arr {
            let val = match item {
                Object::Integer(i) => Some(*i as f64),
                Object::Real(f) => Some(*f),
                _ => None,
            }?;
            coords.push(val);
        }
        Some([coords[0], coords[1], coords[2], coords[3]])
    }

    /// Resolves a field-dictionary key to a `u32`, shared by /Ff, /MaxLen
    /// and /Q, all of which are a single Integer object. ~keep
    fn field_dict_integer(
        doc: &PdfDocument,
        field_dict: &std::collections::HashMap<String, Object>,
        key: &str,
    ) -> Option<u32> {
        Self::inherited_attr(doc, field_dict, key).and_then(|obj| match obj {
            Object::Integer(i) => Some(i as u32),
            _ => None,
        })
    }

    /// Parse field type from /FT value.
    fn parse_field_type(ft: &str) -> FieldType {
        match ft {
            "Btn" => FieldType::Button,
            "Tx" => FieldType::Text,
            "Ch" => FieldType::Choice,
            "Sig" => FieldType::Signature,
            _ => FieldType::Unknown(ft.to_string()),
        }
    }

    /// Parse field value from /V object.
    fn parse_field_value(obj: &Object, field_type: &FieldType) -> FieldValue {
        match obj {
            Object::String(bytes) => {
                if let Some(text) = Self::decode_text_string(bytes) {
                    FieldValue::Text(text)
                } else {
                    FieldValue::None
                }
            }
            Object::Name(name) => {
                if *field_type == FieldType::Button {
                    // ISO 32000-1 §12.7.4.2.3: a button's /V names its selected
                    // appearance state and only /Off means "unselected". Any
                    // other name — including /No — is a real on-state whose
                    // export value must be preserved. ~keep
                    if name == "Yes" || name == "On" {
                        FieldValue::Boolean(true)
                    } else if name == "Off" {
                        FieldValue::Boolean(false)
                    } else {
                        FieldValue::Name(name.clone())
                    }
                } else {
                    FieldValue::Name(name.clone())
                }
            }
            Object::Array(array) => {
                let values: Vec<String> = array
                    .iter()
                    .filter_map(|item| match item {
                        Object::String(bytes) => Self::decode_text_string(bytes),
                        Object::Name(name) => Some(name.clone()),
                        _ => None,
                    })
                    .collect();
                FieldValue::Array(values)
            }
            Object::Boolean(b) => FieldValue::Boolean(*b),
            _ => FieldValue::None,
        }
    }

    /// Parse border style from /BS dictionary.
    fn parse_border_style(obj: &Object) -> Option<BorderStyle> {
        let dict = obj.as_dict()?;

        let width = dict
            .get("W")
            .and_then(|o| match o {
                Object::Integer(i) => Some(*i as f32),
                Object::Real(f) => Some(*f as f32),
                _ => None,
            })
            .unwrap_or(1.0);

        let style = dict
            .get("S")
            .and_then(|o| o.as_name())
            .map(BorderStyleType::from_pdf_name)
            .unwrap_or(BorderStyleType::Solid);

        let dash_array = dict.get("D").and_then(|o| o.as_array()).map(|arr| {
            arr.iter()
                .filter_map(|item| match item {
                    Object::Integer(i) => Some(*i as u32),
                    _ => None,
                })
                .collect()
        });

        Some(BorderStyle {
            width,
            style,
            dash_array,
        })
    }

    /// Parse appearance characteristics from /MK dictionary.
    fn parse_appearance_characteristics(doc: &PdfDocument, obj: &Object) -> Option<AppearanceCharacteristics> {
        let dict = obj.as_dict()?;

        let parse_color = |arr: &[Object]| -> Option<[f32; 3]> {
            if arr.len() == 3 {
                let r = match &arr[0] {
                    Object::Integer(i) => *i as f32,
                    Object::Real(f) => *f as f32,
                    _ => return None,
                };
                let g = match &arr[1] {
                    Object::Integer(i) => *i as f32,
                    Object::Real(f) => *f as f32,
                    _ => return None,
                };
                let b = match &arr[2] {
                    Object::Integer(i) => *i as f32,
                    Object::Real(f) => *f as f32,
                    _ => return None,
                };
                Some([r, g, b])
            } else {
                None
            }
        };

        let background_color = dict
            .get("BG")
            .and_then(|o| Self::resolve_object(doc, o).ok())
            .and_then(|o| o.as_array().cloned())
            .and_then(|arr| parse_color(&arr));

        let border_color = dict
            .get("BC")
            .and_then(|o| Self::resolve_object(doc, o).ok())
            .and_then(|o| o.as_array().cloned())
            .and_then(|arr| parse_color(&arr));

        let caption = dict
            .get("CA")
            .and_then(|o| Self::resolve_object(doc, o).ok())
            .and_then(|o| o.as_string().map(|s| s.to_vec()))
            .and_then(|bytes| Self::decode_text_string(&bytes));

        let rollover_caption = dict
            .get("RC")
            .and_then(|o| Self::resolve_object(doc, o).ok())
            .and_then(|o| o.as_string().map(|s| s.to_vec()))
            .and_then(|bytes| Self::decode_text_string(&bytes));

        let alternate_caption = dict
            .get("AC")
            .and_then(|o| Self::resolve_object(doc, o).ok())
            .and_then(|o| o.as_string().map(|s| s.to_vec()))
            .and_then(|bytes| Self::decode_text_string(&bytes));

        let rotation = dict.get("R").and_then(|o| match o {
            Object::Integer(i) => Some(*i as u32),
            _ => None,
        });

        Some(AppearanceCharacteristics {
            background_color,
            border_color,
            caption,
            rollover_caption,
            alternate_caption,
            rotation,
        })
    }

    /// Export form field data to FDF format.
    ///
    /// Extracts all form fields from the document and writes them to an FDF file.
    ///
    /// # Arguments
    ///
    /// * `doc` - The PDF document to extract fields from
    /// * `output_path` - Path to write the FDF file
    ///
    /// # Example
    ///
    /// ```ignore
    /// use xberg_native_pdf::document::PdfDocument;
    /// use xberg_native_pdf::extractors::forms::FormExtractor;
    ///
    /// let mut doc = PdfDocument::open("form.pdf")?;
    /// FormExtractor::export_fdf(&mut doc, "form_data.fdf")?;
    /// ```
    pub fn export_fdf(doc: &PdfDocument, output_path: impl AsRef<std::path::Path>) -> Result<()> {
        let fields = Self::extract_fields_impl(doc, true)?;
        let writer = crate::fdf::FdfWriter::from_fields(fields);
        writer.write_to_file(output_path)
    }

    /// Export form field data to XFDF format.
    ///
    /// Extracts all form fields from the document and writes them to an XFDF file.
    ///
    /// # Arguments
    ///
    /// * `doc` - The PDF document to extract fields from
    /// * `output_path` - Path to write the XFDF file
    ///
    /// # Example
    ///
    /// ```ignore
    /// use xberg_native_pdf::document::PdfDocument;
    /// use xberg_native_pdf::extractors::forms::FormExtractor;
    ///
    /// let mut doc = PdfDocument::open("form.pdf")?;
    /// FormExtractor::export_xfdf(&mut doc, "form_data.xfdf")?;
    /// ```
    pub fn export_xfdf(doc: &PdfDocument, output_path: impl AsRef<std::path::Path>) -> Result<()> {
        let fields = Self::extract_fields_impl(doc, true)?;
        let writer = crate::fdf::XfdfWriter::from_fields(fields);
        writer.write_to_file(output_path)
    }
}

#[cfg(test)]
mod tests;
