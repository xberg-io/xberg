//! PDF annotations support.
//!
//! Provides access to PDF annotations including text notes, highlights,
//! comments, and other markup per PDF spec ISO 32000-1:2008, Section 12.5.
//!
//! # Supported Annotation Types
//!
//! - Text (sticky notes)
//! - Link (hyperlinks)
//! - Text Markup (Highlight, Underline, StrikeOut, Squiggly)
//! - FreeText (text boxes)
//! - Shape (Line, Square, Circle, Polygon, PolyLine)
//! - Stamp, Ink, FileAttachment, Popup, Caret, Redact, Widget

use crate::annotation_types::{AnnotationFlags, AnnotationSubtype, WidgetFieldType};
use crate::document::PdfDocument;
use crate::error::Result;
use crate::object::{Object, decode_pdf_text_string};

mod geometry;
pub use geometry::{MeasureDictionary, PageViewport, PolygonAnnotation};

/// A PDF annotation.
///
/// Represents all PDF annotation types per ISO 32000-1:2008, Section 12.5.
#[derive(Debug, Clone)]
pub struct Annotation {
    /// Type of annotation (always "Annot" for annotations)
    pub annotation_type: String,

    /// Annotation subtype (Text, Highlight, Link, etc.)
    pub subtype: Option<String>,

    /// Parsed annotation subtype enum
    pub subtype_enum: AnnotationSubtype,

    /// Text contents of the annotation
    pub contents: Option<String>,

    /// Rectangle bounds [x1, y1, x2, y2]
    pub rect: Option<[f64; 4]>,

    /// Author/creator of the annotation (T entry)
    pub author: Option<String>,

    /// Creation date
    pub creation_date: Option<String>,

    /// Modification date (M entry)
    pub modification_date: Option<String>,

    /// Subject of the annotation
    pub subject: Option<String>,

    /// Link destination (for Link annotations)
    /// PDF Spec: ISO 32000-1:2008, Section 12.3.2 - Destinations
    pub destination: Option<LinkDestination>,

    /// Link action (for Link annotations)
    /// PDF Spec: ISO 32000-1:2008, Section 12.6 - Actions
    pub action: Option<LinkAction>,

    /// QuadPoints for text markup annotations (Highlight, Underline, StrikeOut, Squiggly)
    /// Each quad is 8 values: x1,y1, x2,y2, x3,y3, x4,y4
    /// PDF Spec: ISO 32000-1:2008, Section 12.5.6.10
    pub quad_points: Option<Vec<[f64; 8]>>,

    /// Color array (C entry) - RGB or other color space
    pub color: Option<Vec<f64>>,

    /// Opacity (CA entry) - 0.0 to 1.0
    pub opacity: Option<f64>,

    /// Annotation flags (F entry)
    pub flags: AnnotationFlags,

    /// Border style array (Border entry)
    pub border: Option<[f64; 3]>,

    /// Interior color for closed shapes (IC entry)
    pub interior_color: Option<Vec<f64>>,

    /// Field type for Widget annotations (FT entry: Btn, Tx, Ch, Sig)
    pub field_type: Option<WidgetFieldType>,

    /// Field name for Widget annotations (T entry)
    /// Note: This is different from author (also T entry) for other annotation types
    pub field_name: Option<String>,

    /// Field value for Widget annotations (V entry)
    pub field_value: Option<String>,

    /// Default value for Widget annotations (DV entry)
    pub default_value: Option<String>,

    /// Field flags for Widget annotations (Ff entry)
    pub field_flags: Option<u32>,

    /// Options for choice fields (Opt entry)
    pub options: Option<Vec<String>>,

    /// Appearance state for checkboxes/radios (AS entry)
    pub appearance_state: Option<String>,

    /// Geometry vertices for Line, Polygon and PolyLine annotations.
    ///
    /// `Line` annotations use their `/L` entry (one segment, two points);
    /// `Polygon` and `PolyLine` use `/Vertices`. Coordinates are in default user
    /// space, the same frame as [`rect`](Self::rect). `None` for every other
    /// subtype.
    pub vertices: Option<Vec<[f64; 2]>>,

    /// One point list per stroke of an Ink annotation (PDF `/InkList`, ISO
    /// 32000-1 §12.5.6.13). Each inner vector is one freehand stroke in default
    /// user space. `None` for every other subtype.
    pub ink_list: Option<Vec<Vec<[f64; 2]>>>,

    /// Measurement dictionary carried by a geometric annotation (PDF `/Measure`,
    /// ISO 32000-1 §12.9). Measured takeoff tools use it to turn a polygon's
    /// page-space coordinates into a real-world quantity.
    pub measure: Option<MeasureDictionary>,

    /// Raw annotation dictionary for preserving unknown properties during round-trip.
    ///
    /// This contains the complete original PDF dictionary, enabling faithful
    /// preservation of properties that aren't explicitly parsed (appearance streams,
    /// popup references, vendor-specific extensions, etc.).
    pub raw_dict: Option<std::collections::HashMap<String, crate::object::Object>>,
}

/// Link destination within a PDF document.
///
/// Specifies a location within the PDF to navigate to.
#[derive(Debug, Clone, PartialEq)]
pub enum LinkDestination {
    /// Named destination (string reference to destination dictionary)
    Named(String),
    /// Explicit destination: [page fit_type params...]
    Explicit {
        /// Target page number (0-indexed)
        page: u32,
        /// Fit type (XYZ, Fit, FitH, FitV, FitR, FitB, FitBH, FitBV)
        fit_type: String,
        /// Additional parameters (coordinates, zoom factor, etc.)
        params: Vec<f32>,
    },
}

/// Link action associated with an annotation.
///
/// Specifies what happens when the annotation is activated.
#[derive(Debug, Clone, PartialEq)]
pub enum LinkAction {
    /// URI action - navigate to a web URL
    Uri(String),
    /// GoTo action - navigate to a destination within the document
    GoTo(LinkDestination),
    /// GoToR action - navigate to a destination in another document
    GoToRemote {
        /// File specification
        file: String,
        /// Destination in remote file
        destination: Option<LinkDestination>,
    },
    /// Other action types (Launch, Named, etc.)
    Other {
        /// Action type (/S field)
        action_type: String,
    },
}

struct FlatAnnotationFields {
    annotation_type: String,
    subtype: Option<String>,
    subtype_enum: AnnotationSubtype,
    contents: Option<String>,
    rect: Option<[f64; 4]>,
    author: Option<String>,
    creation_date: Option<String>,
    modification_date: Option<String>,
    subject: Option<String>,
    flags: AnnotationFlags,
    color: Option<Vec<f64>>,
    opacity: Option<f64>,
    border: Option<[f64; 3]>,
    interior_color: Option<Vec<f64>>,
}

impl PdfDocument {
    /// Get all annotations for a specific page.
    ///
    /// Returns a list of annotations (comments, highlights, notes, etc.)
    /// present on the specified page.
    ///
    /// # Arguments
    ///
    /// * `page_index` - Zero-based page index
    ///
    /// # Returns
    ///
    /// - `Ok(Vec<Annotation>)` - List of annotations (may be empty)
    /// - `Err` - Error accessing page or annotations
    ///
    /// # Example
    ///
    /// ```no_run
    /// use xberg_native_pdf::PdfDocument;
    ///
    /// let mut doc = PdfDocument::open("sample.pdf")?;
    /// let annotations = doc.get_annotations(0)?;
    ///
    /// for annot in annotations {
    ///     if let Some(contents) = annot.contents {
    ///         println!("Comment: {}", contents);
    ///     }
    /// }
    /// # Ok::<(), xberg_native_pdf::error::Error>(())
    /// ```
    pub fn get_annotations(&self, page_index: usize) -> Result<Vec<Annotation>> {
        let span = tracing::trace_span!(
            "xberg_native_pdf::get_annotations",
            page_index,
            annotations = tracing::field::Empty
        );
        let _entered = span.enter();
        let page_ref = self.get_page_ref(page_index)?;
        let page_obj = self.load_object(page_ref)?;

        let annots = match page_obj.as_dict() {
            Some(dict) => match dict.get("Annots") {
                Some(Object::Array(arr)) => arr.clone(),
                Some(Object::Reference(annot_ref)) => match self.load_object(*annot_ref)? {
                    Object::Array(arr) => arr,
                    _ => return Ok(Vec::new()),
                },
                _ => return Ok(Vec::new()),
            },
            None => return Ok(Vec::new()),
        };

        let mut result = Vec::new();

        for annot_obj in annots {
            let annot_ref = match annot_obj {
                Object::Reference(r) => r,
                _ => continue,
            };

            if let Ok(annotation) = self.parse_annotation(annot_ref) {
                result.push(annotation);
            }
        }

        span.record("annotations", result.len());
        Ok(result)
    }

    /// Parse a single annotation object.
    fn parse_annotation(&self, annot_ref: crate::object::ObjectRef) -> Result<Annotation> {
        let annot_obj = self.load_object(annot_ref)?;

        let dict = annot_obj
            .as_dict()
            .ok_or_else(|| crate::error::Error::InvalidPdf("Annotation is not a dictionary".to_string()))?;

        let f = Self::parse_flat_annotation_fields(dict);
        let quad_points = if f.subtype_enum.is_text_markup() {
            Self::parse_quad_points(dict.get("QuadPoints"))
        } else {
            None
        };
        let (destination, action) = self.parse_link_destination_and_action(dict, f.subtype_enum);
        let (field_type, field_name, field_value, default_value, field_flags, options, appearance_state) =
            if f.subtype_enum == AnnotationSubtype::Widget {
                self.parse_widget_fields(dict)
            } else {
                (None, None, None, None, None, None, None)
            };
        let vertices = if matches!(
            f.subtype_enum,
            AnnotationSubtype::Line | AnnotationSubtype::Polygon | AnnotationSubtype::PolyLine
        ) {
            Self::parse_annotation_vertices(dict, f.subtype_enum)
        } else {
            None
        };
        let ink_list = if f.subtype_enum == AnnotationSubtype::Ink {
            self.parse_ink_list(dict)
        } else {
            None
        };
        let measure = Self::parse_measure_dictionary(dict.get("Measure"));

        Ok(Annotation {
            annotation_type: f.annotation_type,
            subtype: f.subtype,
            subtype_enum: f.subtype_enum,
            contents: f.contents,
            rect: f.rect,
            author: f.author,
            creation_date: f.creation_date,
            modification_date: f.modification_date,
            subject: f.subject,
            destination,
            action,
            quad_points,
            color: f.color,
            opacity: f.opacity,
            flags: f.flags,
            border: f.border,
            interior_color: f.interior_color,
            field_type,
            field_name,
            field_value,
            default_value,
            field_flags,
            options,
            appearance_state,
            vertices,
            ink_list,
            measure,
            raw_dict: Some(dict.clone()),
        })
    }

    fn parse_link_destination_and_action(
        &self,
        dict: &std::collections::HashMap<String, Object>,
        subtype_enum: AnnotationSubtype,
    ) -> (Option<LinkDestination>, Option<LinkAction>) {
        if subtype_enum != AnnotationSubtype::Link {
            return (None, None);
        }
        let dest = dict.get("Dest").and_then(|d| self.parse_destination(d).ok());
        let act = dict.get("A").and_then(|a| self.parse_action(a).ok());
        (dest, act)
    }

    /// Parse Widget annotation fields (form fields).
    ///
    /// PDF Spec: ISO 32000-1:2008, Section 12.7 (Interactive Forms)
    fn parse_widget_fields(
        &self,
        dict: &std::collections::HashMap<String, Object>,
    ) -> (
        Option<WidgetFieldType>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<u32>,
        Option<Vec<String>>,
        Option<String>,
    ) {
        let mut ft = dict.get("FT").and_then(|f| f.as_name()).map(|s| s.to_string());
        let mut field_flags = dict.get("Ff").and_then(|f| match f {
            Object::Integer(n) => Some(*n as u32),
            _ => None,
        });
        let mut field_value = Self::parse_string_value(dict.get("V"));
        let mut default_value = Self::parse_string_value(dict.get("DV"));

        self.inherit_widget_parent_fields(dict, &mut ft, &mut field_flags, &mut field_value, &mut default_value);

        let field_name = Self::pdf_string_field(dict, "T");
        let appearance_state = dict.get("AS").and_then(|a| a.as_name()).map(|s| s.to_string());
        let options = Self::parse_options_array(dict.get("Opt"));

        let field_type =
            Self::resolve_widget_field_type(ft.as_deref(), field_flags, &appearance_state, &field_value, &options);

        (
            field_type,
            field_name,
            field_value,
            default_value,
            field_flags,
            options,
            appearance_state,
        )
    }

    /// Walk up /Parent to inherit unset fields (PDF spec 12.7.3.1). ~keep
    fn inherit_widget_parent_fields(
        &self,
        dict: &std::collections::HashMap<String, Object>,
        ft: &mut Option<String>,
        field_flags: &mut Option<u32>,
        field_value: &mut Option<String>,
        default_value: &mut Option<String>,
    ) {
        if ft.is_some() && field_flags.is_some() && field_value.is_some() && default_value.is_some() {
            return;
        }
        let mut parent_ref = dict
            .get("Parent")
            .and_then(|p| if let Object::Reference(r) = p { Some(*r) } else { None });
        let mut depth = 0;
        while let Some(pref) = parent_ref {
            if depth >= 10 {
                break;
            }
            depth += 1;
            let Ok(parent_obj) = self.load_object(pref) else {
                break;
            };
            let Some(parent_dict) = parent_obj.as_dict() else {
                break;
            };
            if ft.is_none() {
                *ft = parent_dict.get("FT").and_then(|f| f.as_name()).map(|s| s.to_string());
            }
            if field_flags.is_none() {
                *field_flags = parent_dict.get("Ff").and_then(|f| match f {
                    Object::Integer(n) => Some(*n as u32),
                    _ => None,
                });
            }
            if field_value.is_none() {
                *field_value = Self::parse_string_value(parent_dict.get("V"));
            }
            if default_value.is_none() {
                *default_value = Self::parse_string_value(parent_dict.get("DV"));
            }
            parent_ref = parent_dict
                .get("Parent")
                .and_then(|p| if let Object::Reference(r) = p { Some(*r) } else { None });
        }
    }

    fn resolve_widget_field_type(
        ft: Option<&str>,
        field_flags: Option<u32>,
        appearance_state: &Option<String>,
        field_value: &Option<String>,
        options: &Option<Vec<String>>,
    ) -> Option<WidgetFieldType> {
        match ft {
            Some("Tx") => Some(WidgetFieldType::Text),
            Some("Btn") => {
                let ff = field_flags.unwrap_or(0);
                // Bit 17 (0x10000): Radio buttons
                // Bit 16 (0x8000): Push buttons ~keep
                if ff & 0x10000 != 0 {
                    Some(WidgetFieldType::Radio {
                        selected: appearance_state.clone(),
                    })
                } else if ff & 0x8000 != 0 {
                    Some(WidgetFieldType::Button)
                } else {
                    let checked = appearance_state
                        .as_deref()
                        .map(|s| s != "Off" && !s.is_empty())
                        .unwrap_or(false);
                    Some(WidgetFieldType::Checkbox { checked })
                }
            }
            Some("Ch") => Some(WidgetFieldType::Choice {
                options: options.clone().unwrap_or_default(),
                selected: field_value.clone(),
            }),
            Some("Sig") => Some(WidgetFieldType::Signature),
            _ => None,
        }
    }

    fn pdf_string_field(dict: &std::collections::HashMap<String, Object>, key: &str) -> Option<String> {
        dict.get(key).and_then(|v| match v {
            Object::String(s) => Some(decode_pdf_text_string(s)),
            _ => None,
        })
    }

    /// Parse a string value from various PDF object types.
    fn parse_string_value(obj: Option<&Object>) -> Option<String> {
        match obj {
            Some(Object::String(s)) => Some(decode_pdf_text_string(s)),
            Some(Object::Name(n)) => Some(n.clone()),
            Some(Object::Integer(i)) => Some(i.to_string()),
            Some(Object::Real(f)) => Some(f.to_string()),
            _ => None,
        }
    }

    /// Parse options array for choice fields.
    fn parse_options_array(obj: Option<&Object>) -> Option<Vec<String>> {
        match obj {
            Some(Object::Array(arr)) if !arr.is_empty() => {
                let opts: Vec<String> = arr
                    .iter()
                    .filter_map(|o| match o {
                        Object::String(s) => Some(decode_pdf_text_string(s)),
                        Object::Name(n) => Some(n.clone()),
                        Object::Array(inner) if !inner.is_empty() => {
                            // Option can be [export_value, display_value] ~keep
                            inner.first().and_then(|first| match first {
                                Object::String(s) => Some(decode_pdf_text_string(s)),
                                _ => None,
                            })
                        }
                        _ => None,
                    })
                    .collect();
                if opts.is_empty() { None } else { Some(opts) }
            }
            _ => None,
        }
    }

    /// Parse an array of numbers (for color, etc.)
    fn parse_number_array(obj: Option<&Object>) -> Option<Vec<f64>> {
        match obj {
            Some(Object::Array(arr)) if !arr.is_empty() => {
                let nums: Vec<f64> = arr
                    .iter()
                    .filter_map(|o| match o {
                        Object::Integer(n) => Some(*n as f64),
                        Object::Real(f) => Some(*f),
                        _ => None,
                    })
                    .collect();
                if nums.is_empty() { None } else { Some(nums) }
            }
            _ => None,
        }
    }

    /// Parse QuadPoints array into groups of 8 values.
    ///
    /// QuadPoints are 8 values per quad: x1,y1, x2,y2, x3,y3, x4,y4
    fn parse_quad_points(obj: Option<&Object>) -> Option<Vec<[f64; 8]>> {
        match obj {
            Some(Object::Array(arr)) if arr.len() >= 8 => {
                let nums: Vec<f64> = arr
                    .iter()
                    .filter_map(|o| match o {
                        Object::Integer(n) => Some(*n as f64),
                        Object::Real(f) => Some(*f),
                        _ => None,
                    })
                    .collect();

                let quads: Vec<[f64; 8]> = nums
                    .as_chunks::<8>()
                    .0
                    .iter()
                    .map(|chunk| {
                        let mut quad = [0.0; 8];
                        quad.copy_from_slice(chunk);
                        quad
                    })
                    .collect();

                if quads.is_empty() { None } else { Some(quads) }
            }
            _ => None,
        }
    }

    /// Parse a destination object.
    ///
    /// PDF Spec: ISO 32000-1:2008, Section 12.3.2 - Destinations
    fn parse_destination(&self, dest_obj: &Object) -> Result<LinkDestination> {
        match dest_obj {
            Object::String(s) => Ok(LinkDestination::Named(String::from_utf8_lossy(s).to_string())),
            Object::Name(n) => Ok(LinkDestination::Named(n.clone())),
            Object::Array(arr) if !arr.is_empty() => {
                let page = match &arr[0] {
                    Object::Integer(n) => *n as u32,
                    Object::Reference(r) => {
                        // A reference names a page *object*; `Explicit.page`
                        // is the 0-based position in the page tree
                        // (ISO 32000-1 §12.3.2.2). ~keep
                        self.page_index_of_ref(*r)? as u32
                    }
                    _ => 0,
                };

                let fit_type = if arr.len() > 1 {
                    arr[1].as_name().unwrap_or("Fit").to_string()
                } else {
                    "Fit".to_string()
                };

                let params: Vec<f32> = arr
                    .iter()
                    .skip(2)
                    .filter_map(|obj| match obj {
                        Object::Integer(i) => Some(*i as f32),
                        Object::Real(r) => Some(*r as f32),
                        _ => None,
                    })
                    .collect();

                Ok(LinkDestination::Explicit { page, fit_type, params })
            }
            Object::Reference(r) => {
                let dest_loaded = self.load_object(*r)?;
                self.parse_destination(&dest_loaded)
            }
            _ => Err(crate::error::Error::InvalidPdf(
                "Invalid destination format".to_string(),
            )),
        }
    }

    /// Parse an action dictionary.
    ///
    /// PDF Spec: ISO 32000-1:2008, Section 12.6 - Actions
    fn parse_action(&self, action_obj: &Object) -> Result<LinkAction> {
        let action = if let Object::Reference(r) = action_obj {
            self.load_object(*r)?
        } else {
            action_obj.clone()
        };

        let dict = action
            .as_dict()
            .ok_or_else(|| crate::error::Error::InvalidPdf("Action is not a dictionary".to_string()))?;

        let action_type = dict
            .get("S")
            .and_then(|s| s.as_name())
            .ok_or_else(|| crate::error::Error::InvalidPdf("Action missing /S field".to_string()))?;

        match action_type {
            "URI" => {
                let uri = dict
                    .get("URI")
                    .and_then(|u| match u {
                        Object::String(s) => Some(String::from_utf8_lossy(s).to_string()),
                        _ => None,
                    })
                    .ok_or_else(|| crate::error::Error::InvalidPdf("URI action missing /URI field".to_string()))?;

                Ok(LinkAction::Uri(uri))
            }
            "GoTo" => {
                let dest_obj = dict
                    .get("D")
                    .ok_or_else(|| crate::error::Error::InvalidPdf("GoTo action missing /D field".to_string()))?;

                // A destination that cannot be resolved (e.g. a dangling
                // reference to a deleted page) invalidates the action, not
                // the annotation: report the action type without inventing
                // a page number. ~keep
                match self.parse_destination(dest_obj) {
                    Ok(destination) => Ok(LinkAction::GoTo(destination)),
                    Err(error) => {
                        tracing::warn!(
                            operation = "resolve_goto_destination",
                            error_code = error.telemetry_code(),
                            error_offset = ?error.telemetry_offset(),
                            "GoTo destination unresolvable"
                        );
                        Ok(LinkAction::Other {
                            action_type: "GoTo".to_string(),
                        })
                    }
                }
            }
            "GoToR" => {
                let file = dict
                    .get("F")
                    .and_then(|f| match f {
                        Object::String(s) => Some(String::from_utf8_lossy(s).to_string()),
                        Object::Dictionary(d) => d.get("F").and_then(|f| match f {
                            Object::String(s) => Some(String::from_utf8_lossy(s).to_string()),
                            _ => None,
                        }),
                        _ => None,
                    })
                    .ok_or_else(|| crate::error::Error::InvalidPdf("GoToR action missing /F field".to_string()))?;

                let destination = dict.get("D").and_then(|d| self.parse_destination(d).ok());

                Ok(LinkAction::GoToRemote { file, destination })
            }
            other => Ok(LinkAction::Other {
                action_type: other.to_string(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_annotation_creation() {
        let annot = Annotation {
            annotation_type: "Annot".to_string(),
            subtype: Some("Text".to_string()),
            subtype_enum: AnnotationSubtype::Text,
            contents: Some("This is a comment".to_string()),
            rect: Some([100.0, 200.0, 150.0, 250.0]),
            author: Some("John Doe".to_string()),
            creation_date: Some("D:20231030120000".to_string()),
            modification_date: None,
            subject: Some("Review".to_string()),
            destination: None,
            action: None,
            quad_points: None,
            color: None,
            opacity: None,
            flags: AnnotationFlags::empty(),
            border: None,
            interior_color: None,
            field_type: None,
            field_name: None,
            field_value: None,
            default_value: None,
            field_flags: None,
            options: None,
            appearance_state: None,
            vertices: None,
            ink_list: None,
            measure: None,
            raw_dict: None,
        };

        assert_eq!(annot.annotation_type, "Annot");
        assert_eq!(annot.subtype, Some("Text".to_string()));
        assert_eq!(annot.subtype_enum, AnnotationSubtype::Text);
        assert_eq!(annot.contents, Some("This is a comment".to_string()));
        assert!(annot.rect.is_some());
    }

    #[test]
    fn test_highlight_annotation() {
        let annot = Annotation {
            annotation_type: "Annot".to_string(),
            subtype: Some("Highlight".to_string()),
            subtype_enum: AnnotationSubtype::Highlight,
            contents: Some("Highlighted text".to_string()),
            rect: Some([100.0, 700.0, 200.0, 720.0]),
            author: Some("Reviewer".to_string()),
            creation_date: None,
            modification_date: None,
            subject: None,
            destination: None,
            action: None,
            quad_points: Some(vec![[100.0, 700.0, 200.0, 700.0, 200.0, 720.0, 100.0, 720.0]]),
            color: Some(vec![1.0, 1.0, 0.0]),
            opacity: Some(0.5),
            flags: AnnotationFlags::printable(),
            border: None,
            interior_color: None,
            field_type: None,
            field_name: None,
            field_value: None,
            default_value: None,
            field_flags: None,
            options: None,
            appearance_state: None,
            vertices: None,
            ink_list: None,
            measure: None,
            raw_dict: None,
        };

        assert!(annot.subtype_enum.is_text_markup());
        assert!(annot.quad_points.is_some());
        assert_eq!(annot.quad_points.as_ref().unwrap().len(), 1);
        assert_eq!(annot.color, Some(vec![1.0, 1.0, 0.0]));
        assert_eq!(annot.opacity, Some(0.5));
        assert!(annot.flags.is_printable());
    }

    #[test]
    fn test_parse_number_array() {
        use crate::object::Object;

        let arr = vec![Object::Real(1.0), Object::Real(0.5), Object::Real(0.0)];
        let result = PdfDocument::parse_number_array(Some(&Object::Array(arr)));
        assert_eq!(result, Some(vec![1.0, 0.5, 0.0]));

        let arr2 = vec![Object::Integer(1), Object::Real(0.5)];
        let result2 = PdfDocument::parse_number_array(Some(&Object::Array(arr2)));
        assert_eq!(result2, Some(vec![1.0, 0.5]));

        let result3 = PdfDocument::parse_number_array(None);
        assert!(result3.is_none());
    }

    #[test]
    fn test_parse_quad_points() {
        use crate::object::Object;

        let arr: Vec<Object> = vec![
            Object::Real(100.0),
            Object::Real(700.0),
            Object::Real(200.0),
            Object::Real(700.0),
            Object::Real(200.0),
            Object::Real(720.0),
            Object::Real(100.0),
            Object::Real(720.0),
        ];
        let result = PdfDocument::parse_quad_points(Some(&Object::Array(arr)));
        assert!(result.is_some());
        let quads = result.unwrap();
        assert_eq!(quads.len(), 1);
        assert_eq!(quads[0][0], 100.0);
        assert_eq!(quads[0][6], 100.0);
    }

    #[test]
    fn test_widget_text_field_annotation() {
        let annot = Annotation {
            annotation_type: "Annot".to_string(),
            subtype: Some("Widget".to_string()),
            subtype_enum: AnnotationSubtype::Widget,
            contents: None,
            rect: Some([100.0, 700.0, 300.0, 720.0]),
            author: None,
            creation_date: None,
            modification_date: None,
            subject: None,
            destination: None,
            action: None,
            quad_points: None,
            color: None,
            opacity: None,
            flags: AnnotationFlags::empty(),
            border: None,
            interior_color: None,
            field_type: Some(WidgetFieldType::Text),
            field_name: Some("FirstName".to_string()),
            field_value: Some("John".to_string()),
            default_value: None,
            field_flags: None,
            options: None,
            appearance_state: None,
            vertices: None,
            ink_list: None,
            measure: None,
            raw_dict: None,
        };

        assert_eq!(annot.subtype_enum, AnnotationSubtype::Widget);
        assert_eq!(annot.field_type, Some(WidgetFieldType::Text));
        assert_eq!(annot.field_name, Some("FirstName".to_string()));
        assert_eq!(annot.field_value, Some("John".to_string()));
    }

    #[test]
    fn test_widget_checkbox_annotation() {
        let annot = Annotation {
            annotation_type: "Annot".to_string(),
            subtype: Some("Widget".to_string()),
            subtype_enum: AnnotationSubtype::Widget,
            contents: None,
            rect: Some([100.0, 600.0, 120.0, 620.0]),
            author: None,
            creation_date: None,
            modification_date: None,
            subject: None,
            destination: None,
            action: None,
            quad_points: None,
            color: None,
            opacity: None,
            flags: AnnotationFlags::empty(),
            border: None,
            interior_color: None,
            field_type: Some(WidgetFieldType::Checkbox { checked: true }),
            field_name: Some("AcceptTerms".to_string()),
            field_value: Some("Yes".to_string()),
            default_value: None,
            field_flags: None,
            options: None,
            appearance_state: Some("Yes".to_string()),
            vertices: None,
            ink_list: None,
            measure: None,
            raw_dict: None,
        };

        assert_eq!(annot.subtype_enum, AnnotationSubtype::Widget);
        match &annot.field_type {
            Some(WidgetFieldType::Checkbox { checked }) => assert!(*checked),
            _ => panic!("Expected Checkbox field type"),
        }
        assert_eq!(annot.appearance_state, Some("Yes".to_string()));
    }

    #[test]
    fn test_widget_choice_annotation() {
        let annot = Annotation {
            annotation_type: "Annot".to_string(),
            subtype: Some("Widget".to_string()),
            subtype_enum: AnnotationSubtype::Widget,
            contents: None,
            rect: Some([100.0, 500.0, 250.0, 520.0]),
            author: None,
            creation_date: None,
            modification_date: None,
            subject: None,
            destination: None,
            action: None,
            quad_points: None,
            color: None,
            opacity: None,
            flags: AnnotationFlags::empty(),
            border: None,
            interior_color: None,
            field_type: Some(WidgetFieldType::Choice {
                options: vec!["Option A".to_string(), "Option B".to_string(), "Option C".to_string()],
                selected: Some("Option B".to_string()),
            }),
            field_name: Some("Selection".to_string()),
            field_value: Some("Option B".to_string()),
            default_value: Some("Option A".to_string()),
            field_flags: None,
            options: Some(vec![
                "Option A".to_string(),
                "Option B".to_string(),
                "Option C".to_string(),
            ]),
            appearance_state: None,
            vertices: None,
            ink_list: None,
            measure: None,
            raw_dict: None,
        };

        assert_eq!(annot.subtype_enum, AnnotationSubtype::Widget);
        match &annot.field_type {
            Some(WidgetFieldType::Choice { options, selected }) => {
                assert_eq!(options.len(), 3);
                assert_eq!(selected, &Some("Option B".to_string()));
            }
            _ => panic!("Expected Choice field type"),
        }
        assert_eq!(annot.options.as_ref().unwrap().len(), 3);
    }

    #[test]
    fn test_widget_field_type_default() {
        assert_eq!(WidgetFieldType::default(), WidgetFieldType::Text);
    }

    #[test]
    fn test_parse_string_value() {
        assert_eq!(
            PdfDocument::parse_string_value(Some(&Object::String(b"Hello".to_vec()))),
            Some("Hello".to_string())
        );
        assert_eq!(
            PdfDocument::parse_string_value(Some(&Object::Name("MyName".to_string()))),
            Some("MyName".to_string())
        );
        assert_eq!(
            PdfDocument::parse_string_value(Some(&Object::Integer(42))),
            Some("42".to_string())
        );
        assert_eq!(PdfDocument::parse_string_value(None), None);
    }

    #[test]
    fn test_parse_options_array() {
        let arr = vec![
            Object::String(b"Option 1".to_vec()),
            Object::String(b"Option 2".to_vec()),
        ];
        let result = PdfDocument::parse_options_array(Some(&Object::Array(arr)));
        assert!(result.is_some());
        let opts = result.unwrap();
        assert_eq!(opts.len(), 2);
        assert_eq!(opts[0], "Option 1");
        assert_eq!(opts[1], "Option 2");

        let empty: Vec<Object> = vec![];
        assert!(PdfDocument::parse_options_array(Some(&Object::Array(empty))).is_none());

        assert!(PdfDocument::parse_options_array(None).is_none());
    }
}

#[cfg(test)]
mod geometry_tests;
