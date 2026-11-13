mod common;

use std::io::Write;

use flate2::{Compression, write::ZlibEncoder};
use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::extractors::{ImageData, PdfImage, PixelFormat};

const WIDTH: usize = 9;
const HEIGHT: usize = 8;
const ROW_BYTES: usize = WIDTH * 3;
const PREDICTOR: &str = "<< /Predictor 15 /Columns 9 /Colors 3 /BitsPerComponent 8 >>";

fn rgb_samples() -> Vec<u8> {
    (0..ROW_BYTES * HEIGHT).map(|index| (index * 29 + 17) as u8).collect()
}

fn predictor_rows(samples: &[u8]) -> Vec<u8> {
    let mut framed = Vec::new();
    for (row_index, row) in samples.as_chunks::<ROW_BYTES>().0.iter().enumerate() {
        if row_index == 0 {
            framed.push(0);
            framed.extend_from_slice(row);
        } else {
            framed.push(2);
            let previous_row = &samples[(row_index - 1) * ROW_BYTES..];
            framed.extend(
                row.iter()
                    .zip(previous_row)
                    .map(|(&pixel, &above)| pixel.wrapping_sub(above)),
            );
        }
    }
    framed
}

fn image_pdf(entries: &str, samples: &[u8]) -> PdfDocument {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(samples).unwrap();
    let compressed = encoder.finish().unwrap();
    image_pdf_from_stream(entries, &compressed, PREDICTOR)
}

fn image_pdf_from_stream(entries: &str, stream: &[u8], predictor: &str) -> PdfDocument {
    let page_content = "q 90 0 0 80 0 0 cm /Im0 Do Q";
    let objects = [
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 90 80] /Resources << /XObject << /Im0 4 0 R >> >> /Contents 5 0 R >>".to_vec(),
        [
            format!(
                "<< /Type /XObject /Subtype /Image /Width {WIDTH} /Height {HEIGHT} /ColorSpace /DeviceRGB \
                 /BitsPerComponent 8 {entries} /Length {} >>\nstream\n",
                stream.len()
            ).into_bytes(),
            stream.to_vec(),
            b"\nendstream".to_vec(),
        ].concat(),
        format!("<< /Length {} >>\nstream\n{page_content}\nendstream", page_content.len()).into_bytes(),
        predictor.as_bytes().to_vec(),
    ];
    let mut pdf = b"%PDF-1.7\n".to_vec();
    let mut offsets = vec![0];
    for (index, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        writeln!(&mut pdf, "{} 0 obj", index + 1).unwrap();
        pdf.extend_from_slice(object);
        pdf.extend_from_slice(b"\nendobj\n");
    }
    PdfDocument::from_bytes(common::finalize_pdf(pdf, &offsets)).expect("fixture parses")
}

fn assert_packed_rgb(doc: &PdfDocument, expected: &[u8]) {
    let handles = doc.page_image_handles(0).expect("enumerate image handles");
    assert_eq!(handles.len(), 1);
    let image = handles[0].decode().expect("decode Flate RGB image handle");
    assert_rgb_image(&image, expected);

    let images = doc.extract_images(0).expect("extract page images");
    assert_eq!(images.len(), 1);
    assert_rgb_image(&images[0], expected);
}

fn assert_rgb_image(image: &PdfImage, expected: &[u8]) {
    assert_eq!((image.width(), image.height()), (WIDTH as u32, HEIGHT as u32));
    let ImageData::Raw { pixels, format } = image.data() else {
        panic!("expected raw pixels");
    };
    assert_eq!(*format, PixelFormat::RGB);
    assert_eq!(
        pixels.len(),
        ROW_BYTES * HEIGHT,
        "predictor tags leaked into RGB pixels"
    );
    assert_eq!(pixels.as_slice(), expected);
    let png = image.to_png_bytes().expect("encode packed RGB");
    assert_eq!(
        image::load_from_memory(&png).unwrap().to_rgb8().into_raw().as_slice(),
        expected
    );
}

#[test]
fn flate_rgb_resolves_indirect_png_predictor_parameters() {
    let expected = rgb_samples();
    let framed = predictor_rows(&expected);
    assert_eq!(framed.len(), (ROW_BYTES + 1) * HEIGHT);
    for entries in [
        "/Filter /FlateDecode /DecodeParms 6 0 R",
        "/Filter [/FlateDecode] /DecodeParms [6 0 R]",
    ] {
        assert_packed_rgb(&image_pdf(entries, &framed), &expected);
    }
}

#[test]
fn flate_rgb_keeps_direct_predictor_and_unframed_samples_exact() {
    let expected = rgb_samples();
    let direct = format!("/Filter /FlateDecode /DecodeParms {PREDICTOR}");
    assert_packed_rgb(&image_pdf(&direct, &predictor_rows(&expected)), &expected);
    assert_packed_rgb(&image_pdf("/Filter /FlateDecode", &expected), &expected);
}

#[test]
fn flate_rgb_rejects_invalid_row_tag_in_indirect_predictor_stream() {
    let mut framed = predictor_rows(&rgb_samples());
    framed[ROW_BYTES + 1] = 5;
    let doc = image_pdf("/Filter /FlateDecode /DecodeParms 6 0 R", &framed);
    let handles = doc.page_image_handles(0).expect("enumerate image handles");
    assert_eq!(handles.len(), 1);
    let error = handles[0].decode().expect_err("invalid predictor tag must fail");
    assert!(error.to_string().contains("predictor"), "{error}");
}

#[test]
fn flate_rgb_applies_indirect_predictor_before_the_next_filter() {
    let mut framed = vec![0];
    let mut expected = Vec::new();
    for value in 0..13u8 {
        let count = if value == 12 { 24 } else { 16 };
        framed.extend([(257 - count) as u8, value]);
        expected.extend(std::iter::repeat_n(value, count));
    }
    framed.push(128);
    assert_eq!(expected.len(), ROW_BYTES * HEIGHT);
    assert_packed_rgb(
        &image_pdf(
            "/Filter [/FlateDecode /RunLengthDecode] /DecodeParms [6 0 R null]",
            &framed,
        ),
        &expected,
    );
}

#[test]
fn flate_rgb_preserves_predictor_after_a_null_parameter_slot() {
    let expected = rgb_samples();
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&predictor_rows(&expected)).unwrap();
    let compressed = encoder.finish().unwrap();
    let hex: String = compressed.iter().map(|byte| format!("{byte:02x}")).collect();
    let pdf = image_pdf_from_stream(
        "/Filter [/ASCIIHexDecode /FlateDecode] /DecodeParms [null 6 0 R]",
        format!("{hex}>").as_bytes(),
        PREDICTOR,
    );
    assert_packed_rgb(&pdf, &expected);
}

#[test]
fn flate_predictor_before_dct_preserves_the_original_jpeg() {
    let mut jpeg = Vec::new();
    jpeg_encoder::Encoder::new(&mut jpeg, 90)
        .encode(
            &rgb_samples(),
            WIDTH as u16,
            HEIGHT as u16,
            jpeg_encoder::ColorType::Rgb,
        )
        .unwrap();

    let mut framed = vec![1, jpeg[0]];
    framed.extend(jpeg.windows(2).map(|pair| pair[1].wrapping_sub(pair[0])));
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&framed).unwrap();
    let compressed = encoder.finish().unwrap();
    let predictor = format!(
        "<< /Predictor 15 /Columns {} /Colors 1 /BitsPerComponent 8 >>",
        jpeg.len()
    );
    let doc = image_pdf_from_stream(
        "/Filter [/FlateDecode /DCTDecode] /DecodeParms [6 0 R null]",
        &compressed,
        &predictor,
    );

    let handles = doc.page_image_handles(0).expect("enumerate JPEG handle");
    assert_eq!(handles.len(), 1);
    let image = handles[0].decode().expect("decode Flate wrapper");
    let ImageData::Jpeg(bytes) = image.data() else {
        panic!("expected JPEG passthrough");
    };
    assert_eq!(bytes, &jpeg);
    let eager = doc.extract_images(0).expect("extract JPEG from page");
    assert_eq!(eager.len(), 1);
    assert_eq!(eager[0].data(), image.data());
}
