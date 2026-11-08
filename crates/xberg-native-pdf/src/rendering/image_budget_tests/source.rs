use super::super::PdfDocument;

pub(super) fn jpeg(dimension: u32) -> Vec<u8> {
    let pixels = vec![0; dimension as usize * dimension as usize * 3];
    let mut bytes = Vec::new();
    image::codecs::jpeg::JpegEncoder::new(&mut bytes)
        .encode(&pixels, dimension, dimension, image::ExtendedColorType::Rgb8)
        .unwrap();
    bytes
}

fn stream(entries: &str, bytes: &[u8]) -> Vec<u8> {
    let mut output = format!("<< {entries} /Length {} >>\nstream\n", bytes.len()).into_bytes();
    output.extend(bytes);
    output.extend(b"\nendstream");
    output
}

pub(super) fn document(jpeg: &[u8], declared: u32, overlay: bool) -> PdfDocument {
    document_with_paint(jpeg, declared, overlay, false)
}

pub(super) fn document_with_paint(jpeg: &[u8], declared: u32, overlay: bool, inset: bool) -> PdfDocument {
    let mut content = if inset {
        b"q 16 0 0 16 4 4 cm /Im0 Do Q\n".to_vec()
    } else {
        b"q 32 0 0 32 0 0 cm /Im0 Do Q\n".to_vec()
    };
    if overlay {
        content.extend(b"1 0 0 rg 0 0 10 10 re f\n");
    }
    let objects = [
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 32 32] /Contents 4 0 R /Resources << /XObject << /Im0 5 0 R >> >> >>".to_vec(),
        stream("", &content),
        stream(&format!("/Type /XObject /Subtype /Image /Width {declared} /Height {declared} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode"), jpeg),
    ];
    from_objects(&objects)
}

fn from_objects(objects: &[Vec<u8>]) -> PdfDocument {
    let mut bytes = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::new();
    for object in objects {
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
    PdfDocument::from_bytes(bytes).unwrap()
}

pub(super) fn mask_document(mask: &[u8], dimension: u32, soft: bool) -> PdfDocument {
    mask_document_with_filter(mask, dimension, soft, "/Filter /DCTDecode")
}

pub(super) fn raw_mask_document() -> PdfDocument {
    mask_document_with_filter(&vec![255; 32 * 32], 32, true, "")
}

fn mask_document_with_filter(mask: &[u8], dimension: u32, soft: bool, filter: &str) -> PdfDocument {
    let kind = if soft { "SMask" } else { "Mask" };
    let stencil = if soft {
        "/BitsPerComponent 8"
    } else {
        "/BitsPerComponent 1 /ImageMask true"
    };
    from_objects(&[
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 32 32] /Contents 4 0 R /Resources << /XObject << /Im0 5 0 R >> >> >>".to_vec(),
        stream("", b"q 32 0 0 32 0 0 cm /Im0 Do Q"),
        stream(&format!("/Type /XObject /Subtype /Image /Width 32 /Height 32 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode /{kind} 6 0 R"), &jpeg(32)),
        stream(&format!("/Type /XObject /Subtype /Image /Width {dimension} /Height {dimension} /ColorSpace /DeviceGray {stencil} {filter}"), mask),
    ])
}

pub(super) fn gray_jpeg(dimension: u32) -> Vec<u8> {
    gray_jpeg_of(dimension, 255)
}

pub(super) fn gray_jpeg_of(dimension: u32, value: u8) -> Vec<u8> {
    let pixels = vec![value; dimension as usize * dimension as usize];
    let mut bytes = Vec::new();
    image::codecs::jpeg::JpegEncoder::new(&mut bytes)
        .encode(&pixels, dimension, dimension, image::ExtendedColorType::L8)
        .unwrap();
    bytes
}

pub(super) fn inline_document(jpeg: &[u8], dimension: u32) -> PdfDocument {
    let mut content =
        format!("q 32 0 0 32 0 0 cm BI /W {dimension} /H {dimension} /CS /G /BPC 8 /F /DCT ID\n").into_bytes();
    content.extend(jpeg);
    content.extend(b"\nEI Q\n");
    from_objects(&[
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 32 32] /Contents 4 0 R /Resources << >> >>".to_vec(),
        stream("", &content),
    ])
}

pub(super) fn contrast_title_document(image: &[u8], encoding: &str) -> PdfDocument {
    from_objects(&[
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 32 32] /Contents 4 0 R /Resources << /Font << /F 6 0 R >> /XObject << /Im0 5 0 R >> >> >>".to_vec(),
        stream("", b"q 32 0 0 32 0 0 cm /Im0 Do Q 1 1 1 rg BT /F 18 Tf 1 0 0 1 8 10 Tm (A) Tj ET"),
        stream(&format!("/Type /XObject /Subtype /Image /Width 32 /Height 32 /ColorSpace /DeviceRGB /BitsPerComponent 8 {encoding}"), image),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec(),
    ])
}

pub(super) fn wrap_ascii85(source: &[u8]) -> Vec<u8> {
    let mut wrapped = Vec::new();
    for chunk in source.chunks(4) {
        let mut bytes = [0; 4];
        bytes[..chunk.len()].copy_from_slice(chunk);
        let mut value = u32::from_be_bytes(bytes);
        let mut encoded = [b'!'; 5];
        for byte in encoded.iter_mut().rev() {
            *byte += (value % 85) as u8;
            value /= 85;
        }
        wrapped.extend_from_slice(&encoded[..chunk.len() + 1]);
    }
    wrapped.extend_from_slice(b"~>");
    wrapped
}
