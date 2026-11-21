//! Incremental-update writer for polygon annotations.
//!
//! Split from the parent annotation module for file size; a child module's
//! `impl` is the same inherent impl and sees the parent's private items.

use super::*;
use crate::annotations::PolygonAnnotation;

/// Appending a polygon annotation to an existing document.
impl PdfDocument {
    /// Append a Polygon annotation to one page and return the complete PDF bytes.
    ///
    /// The original bytes are preserved verbatim; the page dictionary is
    /// rewritten as a single incremental-update object with the new annotation
    /// reference appended to `/Annots`, and a new xref section plus trailer is
    /// added. Every other object, the document's `/Root`, `/Info`, `/ID` and any
    /// existing incremental updates stay untouched.
    ///
    /// The supplied measure dictionary is copied onto the annotation, so a
    /// measured takeoff tool that opens the result keeps the scale. Encrypted
    /// documents are rejected rather than written unencrypted: without a
    /// re-encryption pass the output would not open in the source document's
    /// context.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Unsupported`] for an encrypted document, and
    /// [`Error::InvalidPdf`] when the page, page dictionary, trailer or
    /// `/Annots` array is unusable, or when fewer than three vertices are given.
    pub fn append_polygon_annotation(&self, page_index: usize, annotation: &PolygonAnnotation) -> Result<Vec<u8>> {
        use std::collections::HashMap;

        use crate::object::{Object, encode_pdf_text_string};
        use crate::redaction::serialize::serialize_object;

        if self.is_encrypted() {
            return Err(crate::error::Error::Unsupported(
                "appending an annotation to an encrypted PDF is not supported".to_string(),
            ));
        }
        if annotation.vertices.len() < 3 {
            return Err(crate::error::Error::InvalidPdf(format!(
                "a Polygon annotation needs at least three vertices, got {}",
                annotation.vertices.len()
            )));
        }

        let page_ref = self.get_page_ref(page_index)?;
        let page_obj = self.load_object(page_ref)?;
        let page_dict = page_obj
            .as_dict()
            .ok_or_else(|| crate::error::Error::InvalidPdf("page object is not a dictionary".to_string()))?
            .clone();
        let trailer_dict = self
            .trailer
            .as_dict()
            .ok_or_else(|| crate::error::Error::InvalidPdf("the trailer is not a dictionary".to_string()))?
            .clone();

        let existing_annots = match page_dict.get("Annots") {
            Some(Object::Array(array)) => array.clone(),
            Some(Object::Reference(reference)) => match self.load_object(*reference)? {
                Object::Array(array) => array,
                _ => {
                    return Err(crate::error::Error::InvalidPdf(
                        "the page's /Annots reference does not hold an array".to_string(),
                    ));
                }
            },
            Some(Object::Null) | None => Vec::new(),
            Some(_) => {
                return Err(crate::error::Error::InvalidPdf(
                    "the page's /Annots entry is not an array".to_string(),
                ));
            }
        };

        let size = trailer_dict
            .get("Size")
            .and_then(Object::as_integer)
            .and_then(|value| u32::try_from(value).ok())
            .unwrap_or(0);
        let annotation_id = size.max(page_ref.id.saturating_add(1)).max(1);
        let next_size = annotation_id.max(page_ref.id).saturating_add(1);

        let rect = annotation.rect.unwrap_or_else(|| polygon_rect(&annotation.vertices));
        let mut annot_dict: HashMap<String, Object> = HashMap::new();
        annot_dict.insert("Type".to_string(), Object::Name("Annot".to_string()));
        annot_dict.insert("Subtype".to_string(), Object::Name("Polygon".to_string()));
        annot_dict.insert(
            "Rect".to_string(),
            Object::Array(rect.iter().map(|value| Object::Real(*value)).collect()),
        );
        annot_dict.insert(
            "Vertices".to_string(),
            Object::Array(
                annotation
                    .vertices
                    .iter()
                    .flat_map(|[x, y]| [Object::Real(*x), Object::Real(*y)])
                    .collect(),
            ),
        );
        // Print flag (bit 3): the annotation is part of the printable page.
        annot_dict.insert("F".to_string(), Object::Integer(4));
        annot_dict.insert(
            "P".to_string(),
            Object::Reference(crate::object::ObjectRef::new(page_ref.id, page_ref.generation)),
        );
        if let Some(contents) = &annotation.contents {
            annot_dict.insert("Contents".to_string(), Object::String(encode_pdf_text_string(contents)));
        }
        if let Some(subject) = &annotation.subject {
            annot_dict.insert("Subj".to_string(), Object::String(encode_pdf_text_string(subject)));
        }
        if let Some(color) = &annotation.color {
            annot_dict.insert(
                "C".to_string(),
                Object::Array(color.iter().map(|value| Object::Real(*value)).collect()),
            );
        }
        if let Some(measure) = &annotation.measure {
            annot_dict.insert("Measure".to_string(), measure_to_object(measure));
        }

        let mut updated_annots = existing_annots;
        // `detached`: whether the annotation id collides with a reachable object
        // id. `Size` normally exceeds every id, so a valid file never collides. ~keep
        updated_annots.push(Object::Reference(crate::object::ObjectRef::new(annotation_id, 0)));
        let mut updated_page = page_dict;
        updated_page.insert("Annots".to_string(), Object::Array(updated_annots));

        let previous_xref = last_startxref_offset(&self.source_bytes).unwrap_or(0);
        let mut output = self.source_bytes.clone();
        if !output.ends_with(b"\n") {
            output.push(b'\n');
        }
        let mut entries: Vec<(u32, u64)> = Vec::with_capacity(2);
        let annotation_offset = write_indirect_object(
            &mut output,
            annotation_id,
            0,
            &Object::Dictionary(annot_dict),
            &serialize_object,
        );
        entries.push((annotation_id, annotation_offset));
        let page_offset = write_indirect_object(
            &mut output,
            page_ref.id,
            page_ref.generation,
            &Object::Dictionary(updated_page),
            &serialize_object,
        );
        entries.push((page_ref.id, page_offset));

        entries.sort_by_key(|(id, _)| *id);
        entries.dedup_by_key(|(id, _)| *id);
        write_xref_and_trailer(
            &mut output,
            &entries,
            next_size,
            previous_xref,
            &trailer_dict,
            &serialize_object,
        );
        Ok(output)
    }
}

/// Tight bounding box of a polygon, ordered `[x0, y0, x1, y1]`.
fn polygon_rect(vertices: &[[f64; 2]]) -> [f64; 4] {
    let mut rect = [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY];
    for [x, y] in vertices {
        rect[0] = rect[0].min(*x);
        rect[1] = rect[1].min(*y);
        rect[2] = rect[2].max(*x);
        rect[3] = rect[3].max(*y);
    }
    if rect.iter().all(|value| value.is_finite()) {
        rect
    } else {
        [0.0, 0.0, 0.0, 0.0]
    }
}

/// Build the PDF `/Measure` dictionary from the public measure shape.
fn measure_to_object(measure: &crate::annotations::MeasureDictionary) -> crate::object::Object {
    use std::collections::HashMap;

    use crate::object::Object;

    let number_array = |values: &Option<Vec<f64>>| {
        values
            .as_ref()
            .map(|values| Object::Array(values.iter().map(|value| Object::Real(*value)).collect()))
    };
    let mut dict: HashMap<String, Object> = HashMap::new();
    dict.insert("Type".to_string(), Object::Name("Measure".to_string()));
    if let Some(subtype) = &measure.subtype {
        dict.insert("Subtype".to_string(), Object::Name(subtype.clone()));
    }
    if let Some(ratio) = &measure.scale_ratio {
        dict.insert(
            "R".to_string(),
            Object::String(crate::object::encode_pdf_text_string(ratio)),
        );
    }
    for (key, value) in [
        ("X", number_array(&measure.x)),
        ("Y", number_array(&measure.y)),
        ("D", number_array(&measure.distance)),
        ("A", number_array(&measure.area)),
        ("T", number_array(&measure.angle)),
        ("S", number_array(&measure.slope)),
    ] {
        if let Some(value) = value {
            dict.insert(key.to_string(), value);
        }
    }
    if let Some(origin) = &measure.origin {
        dict.insert("O".to_string(), Object::Name(origin.clone()));
    }
    Object::Dictionary(dict)
}

/// Write one indirect object and return its byte offset.
fn write_indirect_object(
    output: &mut Vec<u8>,
    id: u32,
    generation: u16,
    object: &crate::object::Object,
    serialize: &dyn Fn(&mut Vec<u8>, &crate::object::Object),
) -> u64 {
    let offset = output.len() as u64;
    output.extend_from_slice(format!("{id} {generation} obj\n").as_bytes());
    serialize(output, object);
    output.extend_from_slice(b"\nendobj\n");
    offset
}

/// Append a classic xref table and trailer for the objects rewritten above.
fn write_xref_and_trailer(
    output: &mut Vec<u8>,
    entries: &[(u32, u64)],
    size: u32,
    previous_xref: u64,
    trailer: &std::collections::HashMap<String, crate::object::Object>,
    serialize: &dyn Fn(&mut Vec<u8>, &crate::object::Object),
) {
    use std::collections::HashMap;

    use crate::object::Object;

    let xref_offset = output.len() as u64;
    output.extend_from_slice(b"xref\n");

    let mut index = 0;
    while index < entries.len() {
        let start = entries[index].0;
        let mut end = start;
        while index + 1 < entries.len() && entries[index + 1].0 == end.saturating_add(1) {
            index += 1;
            end = entries[index].0;
        }
        output.extend_from_slice(format!("{} {}\n", start, end - start + 1).as_bytes());
        for &(id, offset) in &entries[index - (end - start) as usize..=index] {
            output.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
            let _ = id;
        }
        index += 1;
    }

    let mut trailer_dict: HashMap<String, Object> = HashMap::new();
    for key in ["Root", "Info", "ID"] {
        if let Some(value) = trailer.get(key) {
            trailer_dict.insert(key.to_string(), value.clone());
        }
    }
    trailer_dict.insert("Size".to_string(), Object::Integer(i64::from(size)));
    if previous_xref > 0 {
        trailer_dict.insert("Prev".to_string(), Object::Integer(previous_xref as i64));
    }

    output.extend_from_slice(b"trailer\n");
    serialize(output, &Object::Dictionary(trailer_dict));
    output.extend_from_slice(format!("\nstartxref\n{xref_offset}\n%%EOF\n").as_bytes());
}

/// Read the last `startxref` offset in the source bytes.
fn last_startxref_offset(source: &[u8]) -> Option<u64> {
    let needle = b"startxref";
    let start = source.len().saturating_sub(1024 * 1024);
    let window = &source[start..];
    let position = window.windows(needle.len()).rposition(|slice| slice == needle)?;
    let digits = &window[position + needle.len()..];
    let text = std::str::from_utf8(digits).ok()?;
    let number: String = text.trim_start().chars().take_while(char::is_ascii_digit).collect();
    number.parse().ok()
}
