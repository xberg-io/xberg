//! Fixture helpers shared by the tall-source test modules: a synthetic line-scan PDF
//! painted at a chosen density, the ink runs of a strip, and a recognition result.

use crate::ocr_metadata_keys::OCR_TESSERACT_DICT_INVALID_WORD_RATIO_METADATA_KEY;
use crate::types::{ExtractedDocument, FormatMetadata, OcrMetadata};
use std::borrow::Cow;
use std::io::Write;
use std::ops::Range;
use xberg_native_pdf::PdfDocument;

pub(super) const WIDTH: u32 = 1200;
pub(super) const HEIGHT: u32 = 4400;
pub(super) const LINE_PITCH: u32 = 40;
pub(super) const INK_ROWS: Range<u32> = 10..20;
pub(super) const INK_COLUMNS: Range<usize> = 96..904;

pub(super) const GRAY: &str = "/ColorSpace /DeviceGray /BitsPerComponent 8";
pub(super) const BILEVEL: &str = "/ColorSpace /DeviceGray /BitsPerComponent 1";

/// The fixture scan: ink bars `INK_ROWS` tall every `LINE_PITCH` rows across `INK_COLUMNS`.
pub(super) fn is_ink_row(y: u32) -> bool {
    INK_ROWS.contains(&(y % LINE_PITCH))
}

pub(super) fn is_ink(y: u32, x: usize, ink_row: fn(u32) -> bool) -> bool {
    ink_row(y) && INK_COLUMNS.contains(&x)
}

/// Contiguous ink row runs of a strip, top to bottom.
pub(super) fn ink_runs(strip: &image::GrayImage) -> Vec<Range<u32>> {
    let (width, height) = strip.dimensions();
    let mut runs: Vec<Range<u32>> = Vec::new();
    for y in 0..height {
        if (0..width).any(|x| strip.get_pixel(x, y).0[0] < 128) {
            match runs.last_mut() {
                Some(run) if run.end == y => run.end += 1,
                _ => runs.push(y..y + 1),
            }
        }
    }
    runs
}

/// Ink line runs of a scan, counted from the fixture constants.
pub(super) fn source_lines(ink_row: fn(u32) -> bool) -> usize {
    (0..HEIGHT)
        .filter(|&y| ink_row(y) && (y == 0 || !ink_row(y - 1)))
        .count()
}

pub(super) fn recognized(content: &str, mean: f64, invalid: f64, words: u32) -> ExtractedDocument {
    let mut result = ExtractedDocument {
        content: content.into(),
        ..Default::default()
    };
    result.metadata.format = Some(FormatMetadata::Ocr(OcrMetadata {
        language: "eng".into(),
        psm: 11,
        output_format: "markdown".into(),
        table_count: 0,
        table_rows: None,
        table_cols: None,
    }));
    result.metadata.additional.extend([
        (Cow::Borrowed("mean_text_conf"), mean.into()),
        (Cow::Borrowed("word_count"), words.into()),
        (Cow::Borrowed("low_conf_word_count"), 0.into()),
        (
            Cow::Borrowed(OCR_TESSERACT_DICT_INVALID_WORD_RATIO_METADATA_KEY),
            invalid.into(),
        ),
    ]);
    result
}

pub(super) fn flate(bytes: &[u8]) -> Vec<u8> {
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
    encoder.write_all(bytes).expect("in-memory write");
    encoder.finish().expect("in-memory flush")
}

/// Text-line-like black bars on white, one sample per pixel.
pub(super) fn gray_samples(ink_row: fn(u32) -> bool) -> Vec<u8> {
    let mut samples = Vec::with_capacity(WIDTH as usize * HEIGHT as usize);
    for y in 0..HEIGHT {
        samples.extend((0..WIDTH as usize).map(|x| if is_ink(y, x, ink_row) { 0 } else { 255 }));
    }
    samples
}

/// Bars one bit per pixel; a byte is ink when both of its ends are ink columns.
pub(super) fn bilevel_samples(ink_row: fn(u32) -> bool) -> Vec<u8> {
    let mut samples = Vec::new();
    for y in 0..HEIGHT {
        for byte in 0..WIDTH as usize / 8 {
            let ink = is_ink(y, byte * 8, ink_row) && is_ink(y, byte * 8 + 7, ink_row);
            samples.push(if ink_row(y) && ink { 0x00 } else { 0xff });
        }
    }
    samples
}

pub(super) fn stream(entries: &str, bytes: &[u8]) -> Vec<u8> {
    let mut output = format!("<< {entries} /Length {} >>\nstream\n", bytes.len()).into_bytes();
    output.extend(bytes);
    output.extend(b"\nendstream");
    output
}

/// One scan image of `dpi` painted over the whole page.
pub(super) fn scan_pdf_at(color: &str, samples: &[u8], dpi: f64) -> PdfDocument {
    let (page_width, page_height) = (f64::from(WIDTH) * 72.0 / dpi, f64::from(HEIGHT) * 72.0 / dpi);
    let image = stream(
        &format!("/Type /XObject /Subtype /Image /Width {WIDTH} /Height {HEIGHT} {color} /Filter /FlateDecode"),
        &flate(samples),
    );
    let objects = [
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {page_width} {page_height}] /Contents 4 0 R /Resources << /XObject << /Im0 5 0 R >> >> >>"
        )
        .into_bytes(),
        stream("", format!("q {page_width} 0 0 {page_height} 0 0 cm /Im0 Do Q").as_bytes()),
        image,
    ];
    let mut bytes = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::new();
    for object in &objects {
        offsets.push(bytes.len());
        bytes.extend(format!("{} 0 obj\n", offsets.len()).as_bytes());
        bytes.extend(object);
        bytes.extend(b"\nendobj\n");
    }
    let xref = bytes.len();
    bytes.extend(format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes());
    for offset in offsets {
        bytes.extend(format!("{offset:010} 00000 n \n").as_bytes());
    }
    bytes.extend(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    PdfDocument::from_bytes(bytes).expect("a well-formed synthetic scan")
}
