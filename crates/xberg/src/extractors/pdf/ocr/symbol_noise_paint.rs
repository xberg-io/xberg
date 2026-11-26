use image::{GrayImage, ImageEncoder};
use xberg_native_pdf::{PdfDocument, geometry::Rect, object::ObjectRef};

use super::super::document::LineBox;
use super::super::native_word_coverage::{ocr_word_in_page_points, upright_media_box};
use super::{is_symbol_noise_line, page_is_dominated_by_noise_lines};
use crate::core::config::{ExtractionConfig, OcrConfig};
use crate::ocr::processor::{ImageOcrPolicy, OcrProcessor};
use crate::types::{ExtractedDocument, OcrBoundingGeometry, OcrElementLevel};

const MAX_IMAGES: usize = 16;
const MAX_PAINT_PIXELS: u32 = 64 * 1024;
const CROP_PADDING: u32 = 12;
const OCR_FRAME_DPI: u32 = 300;
const PROBE_DPI: u32 = 150;
const FRAME_RATIO: u32 = OCR_FRAME_DPI / PROBE_DPI;
const MAX_WORKING_BYTES: usize = 96 * 1024 * 1024;

pub(in crate::extractors::pdf::ocr) struct Retention {
    transcript: blake3::Hash,
    word: String,
}

impl Retention {
    pub(in crate::extractors::pdf::ocr) fn reconcile(&self, text: &str) -> Option<&str> {
        (blake3::hash(text.as_bytes()) == self.transcript && unique_word(text, &self.word))
            .then_some(self.word.as_str())
    }
}

fn unique_word(text: &str, word: &str) -> bool {
    text.split(|ch: char| !ch.is_alphanumeric())
        .filter(|token| *token == word)
        .take(2)
        .count()
        == 1
}

pub(in crate::extractors::pdf::ocr) fn paragraphs_own_line(
    paragraphs: &[crate::pdf::structure::types::PdfParagraph],
    word: &str,
) -> bool {
    let mut owners = paragraphs
        .iter()
        .flat_map(|paragraph| paragraph.text.lines())
        .filter(|line| line.split(|ch: char| !ch.is_alphanumeric()).any(|token| token == word));
    owners
        .next()
        .is_some_and(|line| line.trim() == word && owners.next().is_none())
}

fn overlaps(a: Rect, b: Rect) -> bool {
    a.x < b.x + b.width && b.x < a.x + a.width && a.y < b.y + b.height && b.y < a.y + a.height
}

fn owns_paint(line: LineBox, paint: Rect) -> bool {
    // Allow rounding at the edges, while requiring the paint to account for most of the line.
    let slack = (line.bottom - line.top) / 10.0;
    paint.x >= line.left - slack
        && paint.x + paint.width <= line.right + slack
        && paint.y >= line.top - slack
        && paint.y + paint.height <= line.bottom + slack
        && paint.width > (line.right - line.left) / 2.0
        && paint.height > (line.bottom - line.top) / 2.0
}

fn later_paint(pdf: &PdfDocument, page: usize, media: Rect) -> Option<(ObjectRef, Rect)> {
    let images = pdf.page_image_handles(page).ok()?;
    if images.len() > MAX_IMAGES || images.iter().any(|image| image.is_inline) {
        return None;
    }
    let mut painted = images
        .iter()
        .filter_map(|image| Some((image, image.painted_object_reference()?)));
    let (backdrop, _) = painted.next()?;
    let (last, selected) = painted.next_back()?;
    if painted.any(|(_, reference)| reference == selected) || backdrop.painted_object_reference() == Some(selected) {
        return None;
    }
    let paint = last.bbox;
    (backdrop.bbox.width >= media.width * 0.8
        && backdrop.bbox.height >= media.height * 0.8
        && overlaps(backdrop.bbox, paint)
        && paint.x >= 0.0
        && paint.y >= 0.0
        && paint.x + paint.width <= media.width
        && paint.y + paint.height <= media.height
        && last.rotation_degrees == 0.0
        && last.placed_extent_points().is_some()
        && (1..=MAX_PAINT_PIXELS).contains(&last.width.saturating_mul(last.height)))
    .then_some((selected, paint))
}

fn owner(read: &ExtractedDocument, raster: (u32, u32), media: Rect, paint: Rect) -> Option<&crate::types::OcrElement> {
    let elements = read.ocr_elements.as_deref()?;
    if elements.len() > 8192 {
        return None;
    }
    let mut lines = elements.iter().filter(|line| {
        line.level == OcrElementLevel::Line
            && ocr_word_in_page_points(line, raster, media).is_some_and(|bbox| owns_paint(bbox, paint))
    });
    let line = lines.next()?;
    let word = line.text.trim();
    (lines.next().is_none()
        && word.len() == 3
        && word.bytes().all(|byte| byte.is_ascii_alphabetic())
        && is_symbol_noise_line(word)
        && unique_word(&read.content, word)
        && read.content.lines().any(|line| line.trim() == word)
        && elements
            .iter()
            .filter(|el| el.level == OcrElementLevel::Line && el.text.trim() == word)
            .count()
            == 1)
        .then_some(line)
}

struct PaintProbe<'a> {
    pdf: &'a PdfDocument,
    page: usize,
    raster: (u32, u32),
    image: ObjectRef,
    bytes: usize,
    cancel: Option<&'a crate::cancellation::CancellationToken>,
}

impl PaintProbe<'_> {
    fn crop(&self, geometry: &OcrBoundingGeometry, include: Option<bool>) -> Option<GrayImage> {
        if self.cancel.is_some_and(|token| token.is_cancelled()) {
            return None;
        }
        let OcrBoundingGeometry::Rectangle {
            left,
            top,
            width,
            height,
        } = *geometry
        else {
            return None;
        };
        let x = left.saturating_sub(CROP_PADDING) / FRAME_RATIO;
        let y = top.saturating_sub(CROP_PADDING) / FRAME_RATIO;
        let w = left
            .checked_add(width)?
            .checked_add(CROP_PADDING)?
            .min(self.raster.0)
            .div_ceil(FRAME_RATIO)
            - x;
        let h = top
            .checked_add(height)?
            .checked_add(CROP_PADDING)?
            .min(self.raster.1)
            .div_ceil(FRAME_RATIO)
            - y;
        if w.checked_mul(h)? > MAX_PAINT_PIXELS {
            return None;
        }
        let (width, height, data) = xberg_native_pdf::rendering::render_image_paint_luminance(
            self.pdf, self.page, PROBE_DPI, self.bytes, self.image, include,
        )
        .ok()?;
        if (width * FRAME_RATIO, height * FRAME_RATIO) != self.raster {
            return None;
        }
        let image = GrayImage::from_raw(width, height, data)?;
        Some(image::imageops::crop_imm(&image, x, y, w, h).to_image())
    }
}

pub(in crate::extractors::pdf::ocr) fn plan(
    pdf: &PdfDocument,
    page: usize,
    read: &ExtractedDocument,
    config: &ExtractionConfig,
    ocr: &OcrConfig,
    work: &mut usize,
) -> Option<PlannedPaint> {
    if ocr.effective_tesseract_language().as_slice() != ["eng"]
        || !page_is_dominated_by_noise_lines(read.content.lines())
    {
        return None;
    }
    let raster = super::super::document::processed_ocr_layout_dimensions(&read.metadata)?;
    let media = upright_media_box(pdf, page)?;
    if pdf.get_page_user_unit(page).ok()? != 1.0
        || (media.width * OCR_FRAME_DPI as f32 / 72.0).ceil() as u32 != raster.0
        || (media.height * OCR_FRAME_DPI as f32 / 72.0).ceil() as u32 != raster.1
    {
        return None;
    }
    let (image, paint) = later_paint(pdf, page, media)?;
    let line = owner(read, raster, media, paint)?;
    let pixels = (raster.0 as usize).checked_mul(raster.1 as usize)?;
    let bytes = super::super::hidden_layer::preview_byte_limit(config).min(MAX_WORKING_BYTES);
    if pixels.checked_mul(5)? > bytes {
        return None;
    }
    *work = work.checked_sub(pixels.checked_mul(3)?)?;
    Some(PlannedPaint {
        page,
        raster,
        image,
        bytes,
        geometry: line.geometry.clone(),
        retention: Retention {
            transcript: blake3::hash(read.content.as_bytes()),
            word: line.text.trim().to_owned(),
        },
    })
}

pub(in crate::extractors::pdf::ocr) struct PlannedPaint {
    page: usize,
    raster: (u32, u32),
    image: ObjectRef,
    bytes: usize,
    geometry: OcrBoundingGeometry,
    retention: Retention,
}

impl PlannedPaint {
    pub(in crate::extractors::pdf::ocr) async fn verify(
        self,
        pdf: std::sync::Arc<PdfDocument>,
        config: &ExtractionConfig,
        ocr: &OcrConfig,
    ) -> Option<Retention> {
        let cancel = config.cancel_token.clone();
        let ocr = ocr.clone();
        tokio::task::spawn_blocking(crate::core::extractor::cacheability::blocking(move || {
            let probe = PaintProbe {
                pdf: &pdf,
                page: self.page,
                raster: self.raster,
                image: self.image,
                bytes: self.bytes,
                cancel: cancel.as_ref(),
            };
            let isolated = probe.crop(&self.geometry, Some(true))?;
            let without = probe.crop(&self.geometry, Some(false))?;
            let drawn = probe.crop(&self.geometry, None)?;
            let visible = visible_ink(&isolated, &drawn, &without)?;
            recognition_matches(&isolated, &without, Some(&visible), &self.retention.word, &ocr)
                .then_some(self.retention)
        }))
        .await
        .ok()?
    }
}

fn visible_ink(isolated: &GrayImage, drawn: &GrayImage, without: &GrayImage) -> Option<GrayImage> {
    if isolated.dimensions() != drawn.dimensions() || drawn.dimensions() != without.dimensions() {
        return None;
    }
    // On white paper, retain only darkness attributable to the selected paint in both views.
    // Unchanged pixels and pixels lightened by the selected paint contribute no stroke ink. ~keep
    let pixels = isolated
        .as_raw()
        .iter()
        .zip(drawn.as_raw())
        .zip(without.as_raw())
        .map(|((&ink, &shown), &omitted)| ink.max(255 - omitted.saturating_sub(shown)))
        .collect();
    GrayImage::from_raw(isolated.width(), isolated.height(), pixels)
}

fn recognition_matches(
    isolated: &GrayImage,
    without: &GrayImage,
    visible: Option<&GrayImage>,
    word: &str,
    ocr: &OcrConfig,
) -> bool {
    let Ok(processor) = OcrProcessor::new(None) else {
        return false;
    };
    let recognition = crate::ocr::types::TesseractConfig {
        language: "eng".into(),
        tessdata_path: ocr.tessdata_path.clone(),
        tessdata_model: ocr.effective_tessdata_model(),
        psm: 7,
        output_format: "text".into(),
        use_cache: false,
        enable_table_detection: false,
        source_dpi: Some(PROBE_DPI as f64),
        ..Default::default()
    };
    let recognize = |image: &GrayImage| {
        let mut png = Vec::new();
        image::codecs::png::PngEncoder::new(&mut png)
            .write_image(
                image.as_raw(),
                image.width(),
                image.height(),
                image::ExtendedColorType::L8,
            )
            .ok()?;
        processor
            .process_image_with_policy(&png, &recognition, None, ImageOcrPolicy::ProvisionedModelsOnly)
            .ok()
            .map(|read| read.content)
    };
    recognize(isolated).is_some_and(|text| text.trim() == word)
        && recognize(without).is_some_and(|text| text.trim() != word)
        && visible.is_none_or(|visible| recognize(visible).is_some_and(|text| text.trim() == word))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "requires the qualified CAS PDF and English model in XBERG_2943_PAINT and XBERG_2943_TESSDATA"]
    async fn opaque_annotation_appearance_cannot_prove_covered_letter_strokes() {
        use lopdf::{Object, Stream, dictionary};
        use std::sync::Arc;

        let directory = std::env::var_os("XBERG_2943_PAINT").unwrap();
        let path = std::path::Path::new(&directory).join("cas-ink-280.pdf");
        let ocr = OcrConfig {
            tessdata_path: Some(std::env::var_os("XBERG_2943_TESSDATA").unwrap().into()),
            ..Default::default()
        };
        for (x, y, expected) in [(10, 10, true), (63, 483, false)] {
            let mut doc = lopdf::Document::load(&path).unwrap();
            let page = doc.get_pages()[&1];
            let page_dict = doc.get_object(page).unwrap().as_dict().unwrap();
            assert!(!page_dict.has(b"Annots"));
            let appearance = doc.add_object(Stream::new(
                dictionary! {
                    "Type" => "XObject", "Subtype" => "Form",
                    "BBox" => vec![0.into(), 0.into(), 41.into(), 27.into()],
                    "Resources" => lopdf::Dictionary::new(),
                },
                b"q 1 g 0 0 41 27 re f Q".to_vec(),
            ));
            let annotation = doc.add_object(dictionary! {
                "Type" => "Annot", "Subtype" => "Square",
                "Rect" => vec![x.into(), y.into(), (x + 41).into(), (y + 27).into()],
                "AP" => dictionary! { "N" => Object::Reference(appearance) },
                "F" => 4,
            });
            doc.get_object_mut(page)
                .unwrap()
                .as_dict_mut()
                .unwrap()
                .set("Annots", vec![Object::Reference(annotation)]);
            let mut bytes = Vec::new();
            doc.save_to(&mut bytes).unwrap();
            let pdf = Arc::new(PdfDocument::from_bytes(bytes).unwrap());
            let (width, height, luminance) =
                xberg_native_pdf::rendering::render_page_without_glyphs(&pdf, 0, PROBE_DPI, MAX_WORKING_BYTES).unwrap();
            let image = GrayImage::from_raw(width, height, luminance).unwrap();
            let glyph_area = image::imageops::crop_imm(&image, 141, 601, 64, 31).to_image();
            assert_eq!(
                glyph_area.pixels().all(|pixel| pixel.0 == [255]),
                !expected,
                "the normal page render must paint the overlapping opaque appearance"
            );

            let planned = PlannedPaint {
                page: 0,
                raster: (2550, 3300),
                image: ObjectRef::new(9, 0),
                bytes: MAX_WORKING_BYTES,
                geometry: OcrBoundingGeometry::Rectangle {
                    left: 281,
                    top: 1201,
                    width: 130,
                    height: 64,
                },
                retention: Retention {
                    transcript: blake3::hash(b"CAS"),
                    word: "CAS".to_owned(),
                },
            };
            assert_eq!(
                planned.verify(pdf, &ExtractionConfig::default(), &ocr).await.is_some(),
                expected,
                "annotation at ({x}, {y})"
            );
        }
    }

    #[tokio::test]
    #[ignore = "requires the qualified CAS PDF and English model in XBERG_2943_PAINT and XBERG_2943_TESSDATA"]
    async fn exposed_margin_cannot_prove_occluded_letter_strokes() {
        let directory = std::env::var_os("XBERG_2943_PAINT").unwrap();
        let path = std::path::Path::new(&directory).join("cas-ink-280.pdf");
        let mut doc = lopdf::Document::load(path).unwrap();
        let image = (9, 0);
        let mask_ref = doc
            .get_object(image)
            .unwrap()
            .as_stream()
            .unwrap()
            .dict
            .get(b"SMask")
            .unwrap()
            .as_reference()
            .unwrap();
        let mask = doc.get_object_mut(mask_ref).unwrap().as_stream_mut().unwrap();
        let mut alpha = mask.decompressed_content().unwrap();
        assert_eq!(alpha.len(), 130 * 65);
        for row in alpha.as_chunks_mut::<130>().0 {
            row[..4].fill(255);
        }
        mask.set_plain_content(alpha);
        let page = doc.get_pages()[&1];
        // Leave the added stripe exposed while covering the glyph area with vector paint.
        doc.add_page_contents(page, b"q 1 g 68.4 488 31 17 re f Q\n".to_vec())
            .unwrap();
        let mut bytes = Vec::new();
        doc.save_to(&mut bytes).unwrap();
        let pdf = std::sync::Arc::new(PdfDocument::from_bytes(bytes).unwrap());
        let planned = PlannedPaint {
            page: 0,
            raster: (2550, 3300),
            image: ObjectRef::new(image.0, image.1),
            bytes: MAX_WORKING_BYTES,
            geometry: OcrBoundingGeometry::Rectangle {
                left: 281,
                top: 1201,
                width: 130,
                height: 64,
            },
            retention: Retention {
                transcript: blake3::hash(b"CAS"),
                word: "CAS".to_owned(),
            },
        };
        let probe = PaintProbe {
            pdf: &pdf,
            page: planned.page,
            raster: planned.raster,
            image: planned.image,
            bytes: planned.bytes,
            cancel: None,
        };
        let isolated = probe.crop(&planned.geometry, Some(true)).unwrap();
        let without = probe.crop(&planned.geometry, Some(false)).unwrap();
        let drawn = probe.crop(&planned.geometry, None).unwrap();
        assert_ne!(drawn, without, "the exposed stripe supplies the old pixel-delta gate");
        let ocr = OcrConfig {
            tessdata_path: Some(std::env::var_os("XBERG_2943_TESSDATA").unwrap().into()),
            ..Default::default()
        };
        assert!(recognition_matches(&isolated, &without, None, "CAS", &ocr));
        assert!(
            planned.verify(pdf, &ExtractionConfig::default(), &ocr).await.is_none(),
            "an exposed stripe must not authorize retention of covered letters"
        );
    }
}
