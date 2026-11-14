//! Confirm a simple row's glyph identities from its drawing commands, independently of extraction.

use super::{LineBox, MAX_ROWS, PdfDocument, normalize};
use std::collections::HashSet;
use xberg_native_pdf::content::{Operator, parser::visit_complete_content_stream};
use xberg_native_pdf::object::Object;

const MAX_CONTENT_BYTES: usize = 64 * 1024;
const MAX_OPERATORS: usize = 1024;

pub(super) struct DrawnRow {
    text: String,
    x: f32,
    baseline: f32,
}

pub(super) fn confirms_row(rows: &[DrawnRow], text: &str, place: &LineBox) -> bool {
    let mut positioned = rows.iter().filter(|row| {
        (row.x - place.left).abs() <= 2.0 && (place.top - 2.0..=place.bottom + 2.0).contains(&row.baseline)
    });
    positioned.next().is_some_and(|row| row.text == text) && positioned.next().is_none()
}

pub(super) fn painted_standard_rows(pdf: &PdfDocument, page_index: usize) -> Option<Vec<DrawnRow>> {
    let resources = pdf.get_page_resources(page_index).ok()?;
    let resources = resources.as_dict()?;
    if ["XObject", "ExtGState", "Pattern", "Properties"]
        .iter()
        .any(|key| resources.contains_key(*key))
    {
        return None;
    }
    let (fonts, _) = pdf
        .resolve_object_for_proof(resources.get("Font")?, 128 * 1024)
        .ok()??;
    let fonts = fonts.as_dict()?;
    if fonts.len() > 8 {
        return None;
    }
    let mut standard = HashSet::new();
    for (key, font) in fonts {
        let (font, _) = pdf.resolve_object_for_proof(font, 128 * 1024).ok()??;
        if standard_font(&font) {
            standard.insert(key.as_str());
        }
    }
    if standard.is_empty() {
        return None;
    }

    let page = pdf.get_page(page_index).ok()?;
    let (Object::Stream { dict, data }, _) = pdf
        .resolve_object_for_proof(page.as_dict()?.get("Contents")?, MAX_CONTENT_BYTES * 256)
        .ok()??
    else {
        return None;
    };
    // Only inspect short, directly encoded drawing commands.
    if dict.contains_key("Filter") || data.len() > MAX_CONTENT_BYTES || pdf.is_encrypted() {
        return None;
    }
    let mut audit = DrawingAudit::default();
    visit_complete_content_stream(&data, MAX_OPERATORS, |op| audit.visit(op, &standard))
        .ok()?
        .then_some(())?;
    (!audit.in_text).then_some(audit.rows)
}

fn standard_font(font: &Object) -> bool {
    let Some(dict) = font.as_dict() else { return false };
    dict.get("Type").and_then(Object::as_name) == Some("Font")
        && dict.get("Subtype").and_then(Object::as_name) == Some("Type1")
        && dict.get("Encoding").and_then(Object::as_name) == Some("WinAnsiEncoding")
        && matches!(
            dict.get("BaseFont").and_then(Object::as_name),
            Some(
                "Helvetica"
                    | "Helvetica-Bold"
                    | "Helvetica-Oblique"
                    | "Helvetica-BoldOblique"
                    | "Times-Roman"
                    | "Times-Bold"
                    | "Times-Italic"
                    | "Times-BoldItalic"
                    | "Courier"
                    | "Courier-Bold"
                    | "Courier-Oblique"
                    | "Courier-BoldOblique"
            )
        )
        && dict
            .keys()
            .all(|key| matches!(key.as_str(), "Type" | "Subtype" | "BaseFont" | "Encoding"))
}

#[derive(Default)]
struct DrawingAudit {
    rows: Vec<DrawnRow>,
    in_text: bool,
    standard: bool,
    mode: u8,
    x: f32,
    y: f32,
    leading: f32,
    showed: bool,
}

impl DrawingAudit {
    fn visit(&mut self, op: Operator, fonts: &HashSet<&str>) -> bool {
        match op {
            Operator::BeginText if !self.in_text => {
                self.in_text = true;
                self.x = 0.0;
                self.y = 0.0;
                self.showed = false;
            }
            Operator::EndText if self.in_text => self.in_text = false,
            Operator::Tf { font, size } if self.in_text && size.is_finite() && size > 0.0 => {
                self.standard = fonts.contains(font.as_str());
            }
            Operator::Tr { render } if self.in_text && matches!(render, 0 | 3) => self.mode = render,
            Operator::TL { leading } if self.in_text && leading.is_finite() => self.leading = leading,
            Operator::Td { tx, ty } if self.in_text && tx.is_finite() && ty.is_finite() => {
                self.x += tx;
                self.y += ty;
                self.showed = false;
            }
            Operator::TStar if self.in_text => {
                self.y -= self.leading;
                self.showed = false;
            }
            Operator::Tj { text } if self.in_text && !self.showed => {
                self.showed = true;
                if self.standard && self.mode == 0 {
                    let (text, invalid) = encoding_rs::WINDOWS_1252.decode_without_bom_handling(&text);
                    if invalid || self.rows.len() == MAX_ROWS {
                        return false;
                    }
                    self.rows.push(DrawnRow {
                        text: normalize(&text),
                        x: self.x,
                        baseline: self.y,
                    });
                }
            }
            _ => return false,
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn real_word_confusion_requires_independent_positioned_glyph_identity() {
        use super::super::{PixelBox, RasterRow, row_owner};

        const REPORTER: &[u8] = include_bytes!("../../../../../../tests/fixtures/ocr_spanish_layer_2951/spanish.pdf");
        let native = "La dirección del proyecto presentó el informe anual ante el cono.";
        let recognized = "La dirección del proyecto presentó el informe anual ante el cone.";
        let pdf_with_word = |word: &[u8; 7]| {
            let mut bytes = REPORTER.to_vec();
            let at = bytes.windows(7).position(|window| window == b"consejo").unwrap();
            bytes[at..at + 7].copy_from_slice(word);
            PdfDocument::from_bytes(bytes).unwrap()
        };
        let place = LineBox {
            left: 72.0,
            top: 437.0,
            right: 425.0,
            bottom: 452.0,
        };
        let recognized = RasterRow {
            text: recognized.into(),
            place,
            area: PixelBox {
                left: 0,
                top: 0,
                right: 100,
                bottom: 12,
            },
            element: 0,
        };
        let matching = painted_standard_rows(&pdf_with_word(b"   cono"), 0).unwrap();
        let conflicting = painted_standard_rows(&pdf_with_word(b"   cone"), 0).unwrap();
        assert!(confirms_row(&matching, native, &place));
        assert!(!confirms_row(&conflicting, native, &place));
        let rows = [recognized];
        assert_eq!(
            row_owner(native, &place, &rows, confirms_row(&conflicting, native, &place)),
            None
        );
        assert_eq!(
            row_owner(native, &place, &rows, confirms_row(&matching, native, &place)),
            Some(0)
        );
    }

    #[test]
    fn glyph_source_must_match_text_and_position() {
        let place = LineBox {
            left: 72.0,
            top: 98.0,
            right: 150.0,
            bottom: 112.0,
        };
        let row = |text: &str, x| DrawnRow {
            text: text.into(),
            x,
            baseline: 100.0,
        };
        assert!(confirms_row(&[row("cono", 72.0)], "cono", &place));
        assert!(!confirms_row(&[row("cone", 72.0), row("cono", 160.0)], "cono", &place));
        assert!(!confirms_row(&[row("cone", 72.0), row("cono", 72.0)], "cono", &place));
    }
}
