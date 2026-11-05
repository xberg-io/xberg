use super::document::{
    LineBox, ocr_page_has_only_plain_payload_with, processed_ocr_layout_dimensions, valid_ocr_layout_dimension,
};
use crate::ocr_metadata_keys::{
    OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY, OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY,
    OCR_TESSERACT_DICT_INVALID_WORD_RATIO_METADATA_KEY,
};
use crate::types::internal::InternalDocument;
use crate::types::{ExtractedDocument, OcrBoundingGeometry, OcrElement, OcrElementLevel};
use xberg_native_pdf::{PdfDocument, geometry::Rect};

/// Compare the source's glyph-derived words against the actual Tesseract result iterator before
/// the backend's dictionary filtering. Neither reflowed lines nor approximate block subdivisions
/// establish word ownership.
pub(super) fn covers_complete_native_page(
    pdf: &PdfDocument,
    page_index: usize,
    layer: &str,
    read: &ExtractedDocument,
) -> bool {
    if layer.contains(super::super::unmapped_glyphs::LOST_GLYPH_MARKER) {
        return false;
    }
    let Some(elements) = read.ocr_elements.as_deref() else {
        return false;
    };
    let words = || elements.iter().filter(|element| element.level == OcrElementLevel::Word);
    if !layer.split_whitespace().eq(words().map(|word| word.text.trim())) {
        return false;
    }
    let Some(first) = words().next() else {
        return false;
    };
    if !is_iterator_word(first) {
        return false;
    }
    let Some(raster) = processed_ocr_layout_dimensions(&read.metadata) else {
        return false;
    };
    let Ok(info) = pdf.get_page_info(page_index) else {
        return false;
    };
    let media = info.media_box;
    if info.rotation != 0 || media.x != 0.0 || media.y != 0.0 || info.crop_box.is_some_and(|crop| crop != media) {
        return false;
    }
    let Ok(lines) = pdf.extract_text_lines(page_index) else {
        return false;
    };
    let native = || lines.iter().flat_map(|line| &line.words);
    layer.split_whitespace().eq(native().map(|word| word.text.as_str()))
        && native().zip(words()).all(|(source, ocr)| {
            is_iterator_word(ocr)
                && ocr_word_in_page_points(ocr, raster, media)
                    .is_some_and(|placed| native_box(source.bbox).owns(&placed))
        })
}

fn is_iterator_word(word: &OcrElement) -> bool {
    word.backend_metadata.get("backend").and_then(serde_json::Value::as_str) == Some("tesseract-iterator")
}

fn native_box(rect: Rect) -> LineBox {
    LineBox {
        left: rect.x,
        top: rect.y,
        right: rect.x + rect.width,
        bottom: rect.y + rect.height,
    }
}

fn ocr_word_in_page_points(word: &OcrElement, raster: (u32, u32), media: Rect) -> Option<LineBox> {
    let OcrBoundingGeometry::Rectangle {
        left,
        top,
        width,
        height,
    } = word.geometry
    else {
        return None;
    };
    let (raster_width, raster_height) = raster;
    let right = left.checked_add(width)?;
    let bottom = top.checked_add(height)?;
    if width == 0 || height == 0 || right > raster_width || bottom > raster_height {
        return None;
    }
    let scale_x = media.width / raster_width as f32;
    let scale_y = media.height / raster_height as f32;
    Some(LineBox {
        left: left as f32 * scale_x,
        right: right as f32 * scale_x,
        top: (raster_height - bottom) as f32 * scale_y,
        bottom: (raster_height - top) as f32 * scale_y,
    })
}

/// These entries describe the detached hOCR run, not extracted document metadata. The mixed merge
/// publishes only the segmentation modes of retained OCR pages from this page-local map.
pub(super) fn has_only_certified_tesseract_text(doc: &InternalDocument) -> bool {
    doc.prebuilt_ocr_elements.is_none() && ocr_page_has_only_plain_payload_with(doc, is_hocr_bookkeeping)
}

fn is_hocr_bookkeeping(key: &str, value: &serde_json::Value) -> bool {
    match key {
        "source_format" => value.as_str() == Some("hocr"),
        "language" => value.as_str().is_some_and(|language| !language.is_empty()),
        "psm" => value
            .as_str()
            .and_then(|mode| mode.parse::<u8>().ok())
            .is_some_and(|mode| mode <= 13),
        "mean_text_conf" | "p10_word_conf" | "median_word_conf" => {
            value.as_u64().is_some_and(|confidence| confidence <= 100)
        }
        "word_count" | "low_conf_word_count" => value.as_u64().is_some(),
        "tables_detected" | "table_count" => value.as_u64() == Some(0),
        OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY | OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY => {
            valid_ocr_layout_dimension(value).is_some()
        }
        OCR_TESSERACT_DICT_INVALID_WORD_RATIO_METADATA_KEY => {
            value.as_f64().is_some_and(|ratio| (0.0..=1.0).contains(&ratio))
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::internal::ElementKind;

    #[test]
    fn should_ignore_only_known_hocr_run_bookkeeping_in_a_plain_page() {
        let mut doc = super::super::document::flat_ocr_page_document("real source words");
        doc.metadata
            .additional
            .insert("source_format".into(), serde_json::json!("hocr"));
        doc.metadata
            .additional
            .insert("table_count".into(), serde_json::json!(0));
        assert!(has_only_certified_tesseract_text(&doc));
        assert!(!super::super::document::ocr_page_has_only_plain_payload(&doc));

        let mut unknown = doc.clone();
        unknown
            .metadata
            .additional
            .insert("unknown".into(), serde_json::json!(0));
        assert!(!has_only_certified_tesseract_text(&unknown));

        let mut invalid = doc.clone();
        invalid
            .metadata
            .additional
            .insert("table_count".into(), serde_json::json!(1));
        assert!(!has_only_certified_tesseract_text(&invalid));

        doc.elements[0].kind = ElementKind::Heading { level: 1 };
        assert!(!has_only_certified_tesseract_text(&doc));
    }
}
