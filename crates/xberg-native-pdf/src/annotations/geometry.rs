//! Annotation geometry, measurement dictionaries and page viewports.
//!
//! Split from the parent annotation module for file size; the inherent `impl`
//! is the same impl and sees the module's private helpers.

use super::*;

/// A measurement dictionary attached to an annotation or a page viewport.
///
/// Number format arrays are reported as their flattened numeric entries: the
/// PDF specification lets each entry be a nested array, and the exact meaning of
/// the entries depends on the producer. Exposing the numbers without inventing
/// units keeps the value lossless while [`scale_ratio`](Self::scale_ratio) gives
/// the producer's own human-readable scale.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MeasureDictionary {
    /// Measure subtype: `RL` for a rectilinear drawing, `GEO` for geospatial
    /// data. `None` when the producer omitted it.
    pub subtype: Option<String>,

    /// The producer's scale-ratio text (PDF `/R`), for example
    /// `"3/8 in = 1 ft"`. This is the original string, not a parsed number.
    pub scale_ratio: Option<String>,

    /// Flattened numeric entries of the `/X` number format array.
    pub x: Option<Vec<f64>>,

    /// Flattened numeric entries of the `/Y` number format array.
    pub y: Option<Vec<f64>>,

    /// Flattened numeric entries of the `/D` distance number format array.
    pub distance: Option<Vec<f64>>,

    /// Flattened numeric entries of the `/A` area number format array.
    pub area: Option<Vec<f64>>,

    /// Flattened numeric entries of the `/T` angle number format array.
    pub angle: Option<Vec<f64>>,

    /// Flattened numeric entries of the `/S` slope number format array.
    pub slope: Option<Vec<f64>>,

    /// Name of the coordinate origin (PDF `/O`), when the producer set one.
    pub origin: Option<String>,
}

/// A page viewport dictionary (PDF `/VP`, ISO 32000-1 §12.5.6.19).
///
/// A viewport names a rectangle on the page and, optionally, the measurement
/// scale that applies to it. Construction drawings use one viewport per plan,
/// detail or elevation so a measured annotation inside it carries real-world
/// units.
#[derive(Debug, Clone, PartialEq)]
pub struct PageViewport {
    /// Viewport bounds `[x0, y0, x1, y1]` in default user space.
    pub bbox: [f64; 4],

    /// Viewport name, read from `/Name`, then `/NM`, then `/T`.
    pub name: Option<String>,

    /// Measurement dictionary for this viewport, when present.
    pub measure: Option<MeasureDictionary>,
}

/// A polygon annotation to append to an existing document.
///
/// The writer copies the measure dictionary verbatim, so a measured takeoff tool
/// can reopen the result and keep its scale. Coordinates are in default user
/// space, the same frame the reader reports.
#[derive(Debug, Clone, PartialEq)]
pub struct PolygonAnnotation {
    /// Polygon vertices `[x, y]` in default user space. At least three are
    /// required.
    pub vertices: Vec<[f64; 2]>,

    /// Annotation rectangle `[x0, y0, x1, y1]`. `None` computes the tight
    /// bounding box of the vertices.
    pub rect: Option<[f64; 4]>,

    /// Annotation text (PDF `/Contents`).
    pub contents: Option<String>,

    /// Annotation subject (PDF `/Subj`).
    pub subject: Option<String>,

    /// Measurement dictionary to copy onto the annotation (PDF `/Measure`).
    pub measure: Option<MeasureDictionary>,

    /// Colour components (PDF `/C`): gray, RGB or CMYK values in `0.0..=1.0`.
    pub color: Option<Vec<f64>>,
}

impl PolygonAnnotation {
    /// Build a polygon annotation from its vertices.
    pub fn new(vertices: Vec<[f64; 2]>) -> Self {
        Self {
            vertices,
            rect: None,
            contents: None,
            subject: None,
            measure: None,
            color: None,
        }
    }
}

/// Flatten a number format array (possibly nested) into its numeric leaves.
fn flatten_number_array(object: &Object) -> Option<Vec<f64>> {
    let mut numbers = Vec::new();
    flatten_numbers_into(object, &mut numbers);
    (!numbers.is_empty()).then_some(numbers)
}

fn flatten_numbers_into(object: &Object, numbers: &mut Vec<f64>) {
    match object {
        Object::Integer(value) => numbers.push(*value as f64),
        Object::Real(value) => numbers.push(*value),
        Object::Array(items) => {
            for item in items {
                flatten_numbers_into(item, numbers);
            }
        }
        _ => {}
    }
}

/// Read a four-number rectangle (`/Rect`, `/BBox`) without reordering it; the
/// caller decides which corners the producer wrote.
fn parse_rect_array(object: &Object) -> Option<[f64; 4]> {
    let array = object.as_array()?;
    if array.len() != 4 {
        return None;
    }
    let mut rect = [0.0; 4];
    for (slot, item) in rect.iter_mut().zip(array) {
        *slot = match item {
            Object::Integer(value) => *value as f64,
            Object::Real(value) => *value,
            _ => return None,
        };
    }
    Some(rect)
}

impl PdfDocument {
    /// Read `Line`'s `/L` entry or `Polygon`/`PolyLine`'s `/Vertices` entry as
    /// a list of `[x, y]` points. A trailing odd coordinate pair is ignored
    /// rather than fabricating a point.
    pub(super) fn parse_annotation_vertices(
        dict: &std::collections::HashMap<String, Object>,
        subtype: AnnotationSubtype,
    ) -> Option<Vec<[f64; 2]>> {
        let key = if subtype == AnnotationSubtype::Line {
            "L"
        } else {
            "Vertices"
        };
        let points = Self::parse_points(dict.get(key))?;
        // A Line's /L must hold exactly one segment; any other length is a
        // malformed entry that we report as absent instead of guessing. ~keep
        if subtype == AnnotationSubtype::Line && points.len() != 2 {
            return None;
        }
        Some(points)
    }

    /// Read `/InkList` as one point list per stroke.
    pub(super) fn parse_ink_list(
        &self,
        dict: &std::collections::HashMap<String, Object>,
    ) -> Option<Vec<Vec<[f64; 2]>>> {
        let ink = dict.get("InkList")?;
        let array_obj = match ink {
            Object::Reference(reference) => self.load_object(*reference).ok()?,
            other => other.clone(),
        };
        let array = array_obj.as_array()?;
        let mut strokes = Vec::new();
        for stroke in array {
            let resolved = match stroke {
                Object::Reference(nested) => self.load_object(*nested).unwrap_or(Object::Null),
                other => other.clone(),
            };
            if let Some(points) = Self::parse_points(Some(&resolved)).filter(|points| !points.is_empty()) {
                strokes.push(points);
            }
        }
        (!strokes.is_empty()).then_some(strokes)
    }

    /// Parse a flat numeric coordinate array into `[x, y]` pairs. An odd final
    /// number is dropped, never combined with an invented value.
    fn parse_points(object: Option<&Object>) -> Option<Vec<[f64; 2]>> {
        let object = object?;
        let array = object.as_array()?;
        let numbers: Vec<f64> = array
            .iter()
            .filter_map(|item| match item {
                Object::Integer(value) => Some(*value as f64),
                Object::Real(value) => Some(*value),
                _ => None,
            })
            .collect();
        let (pairs, _) = numbers.as_chunks::<2>();
        let points: Vec<[f64; 2]> = pairs.to_vec();
        (!points.is_empty()).then_some(points)
    }

    /// Parse a `/Measure` dictionary. Number format arrays may nest; their
    /// numeric leaves are flattened in order.
    pub(super) fn parse_measure_dictionary(object: Option<&Object>) -> Option<MeasureDictionary> {
        let dict = object?.as_dict()?;
        let number_array = |key: &str| dict.get(key).and_then(flatten_number_array);
        Some(MeasureDictionary {
            subtype: dict
                .get("Subtype")
                .and_then(|value| value.as_name())
                .map(str::to_string),
            scale_ratio: dict.get("R").and_then(|value| match value {
                Object::String(bytes) => Some(decode_pdf_text_string(bytes)),
                _ => None,
            }),
            x: number_array("X"),
            y: number_array("Y"),
            distance: number_array("D"),
            area: number_array("A"),
            angle: number_array("T"),
            slope: number_array("S"),
            origin: dict.get("O").and_then(|value| value.as_name()).map(str::to_string),
        })
    }

    /// Read the page's `/VP` viewport dictionaries (PDF 1.6, ISO 32000-1
    /// §12.5.6.19).
    ///
    /// A viewport without a four-number `/BBox` cannot describe a region and is
    /// skipped. References are resolved; malformed entries only drop that entry.
    pub fn get_page_viewports(&self, page_index: usize) -> Result<Vec<PageViewport>> {
        let page_ref = self.get_page_ref(page_index)?;
        let page_obj = self.load_object(page_ref)?;
        let Some(page_dict) = page_obj.as_dict() else {
            return Ok(Vec::new());
        };
        let vp_obj = match page_dict.get("VP") {
            Some(Object::Reference(reference)) => self.load_object(*reference)?,
            Some(object) => object.clone(),
            None => return Ok(Vec::new()),
        };
        let Some(vp_array) = vp_obj.as_array() else {
            return Ok(Vec::new());
        };

        let mut viewports = Vec::new();
        for entry in vp_array {
            let resolved = match entry {
                Object::Reference(reference) => match self.load_object(*reference) {
                    Ok(object) => object,
                    Err(_) => continue,
                },
                other => other.clone(),
            };
            let Some(dict) = resolved.as_dict() else {
                continue;
            };
            let Some(bbox) = dict.get("BBox").and_then(parse_rect_array) else {
                continue;
            };
            let name = ["Name", "NM", "T"].iter().find_map(|key| {
                dict.get(*key).and_then(|value| match value {
                    Object::String(bytes) => Some(decode_pdf_text_string(bytes)),
                    Object::Name(name) => Some(name.clone()),
                    _ => None,
                })
            });
            let measure = Self::parse_measure_dictionary(dict.get("Measure"));
            viewports.push(PageViewport { bbox, name, measure });
        }
        Ok(viewports)
    }

    pub(super) fn parse_flat_annotation_fields(
        dict: &std::collections::HashMap<String, Object>,
    ) -> FlatAnnotationFields {
        let annotation_type = dict
            .get("Type")
            .and_then(|t| t.as_name())
            .unwrap_or("Unknown")
            .to_string();
        let subtype = dict.get("Subtype").and_then(|s| s.as_name()).map(|s| s.to_string());
        let subtype_enum = subtype
            .as_deref()
            .map(AnnotationSubtype::from_pdf_name)
            .unwrap_or(AnnotationSubtype::Unknown);
        let contents = Self::pdf_string_field(dict, "Contents");
        let rect = dict.get("Rect").and_then(|r| match r {
            Object::Array(arr) if arr.len() == 4 => {
                let mut rect_arr = [0.0; 4];
                for (i, obj) in arr.iter().enumerate() {
                    rect_arr[i] = match obj {
                        Object::Integer(n) => *n as f64,
                        Object::Real(f) => *f,
                        _ => 0.0,
                    };
                }
                Some(rect_arr)
            }
            _ => None,
        });
        let author = Self::pdf_string_field(dict, "T");
        let creation_date = Self::pdf_string_field(dict, "CreationDate");
        let modification_date = Self::pdf_string_field(dict, "M");
        let subject = Self::pdf_string_field(dict, "Subj");
        let flags = dict
            .get("F")
            .and_then(|f| match f {
                Object::Integer(n) => Some(AnnotationFlags::new(*n as u32)),
                _ => None,
            })
            .unwrap_or_default();
        let color = Self::parse_number_array(dict.get("C"));
        let opacity = dict.get("CA").and_then(|o| match o {
            Object::Real(f) => Some(*f),
            Object::Integer(n) => Some(*n as f64),
            _ => None,
        });
        let border = dict.get("Border").and_then(|b| match b {
            Object::Array(arr) if arr.len() >= 3 => {
                let mut border_arr = [0.0; 3];
                for (i, obj) in arr.iter().take(3).enumerate() {
                    border_arr[i] = match obj {
                        Object::Integer(n) => *n as f64,
                        Object::Real(f) => *f,
                        _ => 0.0,
                    };
                }
                Some(border_arr)
            }
            _ => None,
        });
        let interior_color = Self::parse_number_array(dict.get("IC"));

        FlatAnnotationFields {
            annotation_type,
            subtype,
            subtype_enum,
            contents,
            rect,
            author,
            creation_date,
            modification_date,
            subject,
            flags,
            color,
            opacity,
            border,
            interior_color,
        }
    }
}
