//! Extracted raster pixels must match the image's decoded samples:
//!
//! * A sub-byte image (1/2/4 bits per component) must be unpacked to one
//!   8-bit sample per pixel — `PixelFormat::Grayscale` promises
//!   `width × height × bytes_per_pixel` bytes, and a packed buffer breaks
//!   every consumer that builds an image from it.
//! * A non-default `/Decode` array must be applied to the samples
//!   (ISO 32000-1 §8.9.5.2) at every bit depth, not only for 1-bit CCITT.

use xberg_native_pdf::document::PdfDocument;
use xberg_native_pdf::extractors::{ImageData, PixelFormat};

fn build_pdf(image_dict: &str, image_data: &[u8]) -> Vec<u8> {
    let content: &[u8] = b"q 100 0 0 100 0 0 cm /Im0 Do Q";
    let mut bodies: Vec<Vec<u8>> = vec![Vec::new(); 6];
    bodies[1] = b"<< /Type /Catalog /Pages 2 0 R >>".to_vec();
    bodies[2] = b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec();
    bodies[3] = b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
/Resources << /XObject << /Im0 4 0 R >> >> /Contents 5 0 R >>"
        .to_vec();
    let mut img = format!("{image_dict}\nstream\n").into_bytes();
    img.extend_from_slice(image_data);
    img.extend_from_slice(b"\nendstream");
    bodies[4] = img;
    let mut cs = format!("<< /Length {} >>\nstream\n", content.len()).into_bytes();
    cs.extend_from_slice(content);
    cs.extend_from_slice(b"\nendstream");
    bodies[5] = cs;

    let mut out: Vec<u8> = Vec::new();
    out.extend_from_slice(b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n");
    let mut offsets = [0usize; 6];
    for id in 1..=5 {
        offsets[id] = out.len();
        out.extend_from_slice(format!("{id} 0 obj\n").as_bytes());
        out.extend_from_slice(&bodies[id]);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref = out.len();
    out.extend_from_slice(b"xref\n0 6\n0000000000 65535 f \n");
    for offset in &offsets[1..=5] {
        out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(format!("trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes());
    out
}

fn extract_first_raw(pdf: Vec<u8>) -> (u32, u32, u8, PixelFormat, Vec<u8>) {
    let doc = PdfDocument::from_bytes(pdf).expect("open pdf");
    let imgs = doc.extract_images(0).expect("extract_images");
    assert!(!imgs.is_empty(), "no images extracted");
    let img = &imgs[0];
    let (fmt, pixels) = match img.data() {
        ImageData::Raw { pixels, format } => (*format, pixels.clone()),
        ImageData::Jpeg(_) => panic!("expected Raw image data"),
    };
    (img.width(), img.height(), img.bits_per_component(), fmt, pixels)
}

/// 8×8 1-bit DeviceGray, every row byte 0x55 (alternating 0/1 bits).
/// Correct extraction: 64 unpacked samples alternating 0, 255.
#[test]
fn one_bit_devicegray_unpacks_to_full_sample_plane() {
    let (w, h, bpc, fmt, pixels) = extract_first_raw(build_pdf(
        "<< /Type /XObject /Subtype /Image /Width 8 /Height 8 \
         /ColorSpace /DeviceGray /BitsPerComponent 1 /Length 8 >>",
        &[0x55u8; 8],
    ));

    assert_eq!((w, h), (8, 8));
    assert_eq!(fmt, PixelFormat::Grayscale);
    assert_eq!(
        pixels.len(),
        (w as usize) * (h as usize) * fmt.bytes_per_pixel(),
        "buffer must hold one byte per sample, got {} bytes for {w}x{h} (bpc={bpc})",
        pixels.len()
    );
    let expected_row = [0u8, 255, 0, 255, 0, 255, 0, 255];
    for (i, row) in pixels.chunks(8).enumerate() {
        assert_eq!(row, expected_row, "row {i} mismatch");
    }
}

/// 8×8 8-bit DeviceGray with samples 0..63 and `/Decode [1 0]`: the samples
/// must come out inverted (255, 254, …, 192), not raw.
#[test]
fn decode_array_applied_to_eight_bit_gray() {
    let data: Vec<u8> = (0..64u8).collect();
    let (w, h, bpc, fmt, pixels) = extract_first_raw(build_pdf(
        "<< /Type /XObject /Subtype /Image /Width 8 /Height 8 \
         /ColorSpace /DeviceGray /BitsPerComponent 8 /Decode [1 0] /Length 64 >>",
        &data,
    ));

    assert_eq!((w, h), (8, 8));
    assert_eq!(fmt, PixelFormat::Grayscale);
    assert_eq!(bpc, 8);
    let expected: Vec<u8> = (0..64u16).map(|s| (255 - s) as u8).collect();
    assert_eq!(pixels, expected, "/Decode [1 0] ignored: samples extracted un-inverted");
}

/// A 1-bpc image over the unpack size cap stays packed. `/Decode [1 0]` is a
/// pure inversion there, so it must still be applied as a byte-wise NOT rather
/// than leaving the exact negative of the picture.
fn oversized_one_bit_image(decode: Option<&str>) -> xberg_native_pdf::extractors::PdfImage {
    use std::collections::HashMap;
    use xberg_native_pdf::extractors::extract_image_from_xobject;
    use xberg_native_pdf::object::Object;

    let (width, height) = (20_000i64, 14_000i64);
    let mut dict = HashMap::new();
    dict.insert("Subtype".to_string(), Object::Name("Image".to_string()));
    dict.insert("Width".to_string(), Object::Integer(width));
    dict.insert("Height".to_string(), Object::Integer(height));
    dict.insert("ColorSpace".to_string(), Object::Name("DeviceGray".to_string()));
    dict.insert("BitsPerComponent".to_string(), Object::Integer(1));
    if let Some(decode) = decode {
        let values = decode.split(' ').map(|v| Object::Integer(v.parse().unwrap())).collect();
        dict.insert("Decode".to_string(), Object::Array(values));
    }
    let row_bytes = (width as usize).div_ceil(8);
    let xobject = Object::Stream {
        dict,
        data: bytes::Bytes::from(vec![0x55u8; row_bytes * height as usize]),
    };
    extract_image_from_xobject(None, &xobject, None, None).expect("oversized image extracts")
}

#[test]
fn oversized_one_bit_image_with_decode_1_0_is_inverted() {
    let image = oversized_one_bit_image(Some("1 0"));
    assert_eq!(image.bits_per_component(), 1, "over the cap the samples stay packed");
    assert!(image.decode_folded_in(), "the inversion is the /Decode mapping");
    let ImageData::Raw { pixels, .. } = image.data() else {
        panic!("expected Raw image data");
    };
    assert!(
        pixels.iter().all(|&b| b == 0xAA),
        "0x55 under /Decode [1 0] must read 0xAA"
    );
}

#[test]
fn oversized_one_bit_image_without_decode_stays_as_stored() {
    let image = oversized_one_bit_image(None);
    assert_eq!(image.bits_per_component(), 1);
    assert!(!image.decode_folded_in());
    let ImageData::Raw { pixels, .. } = image.data() else {
        panic!("expected Raw image data");
    };
    assert!(pixels.iter().all(|&b| b == 0x55));
}
