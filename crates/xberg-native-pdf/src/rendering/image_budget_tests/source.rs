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
    let mut content = b"q 32 0 0 32 0 0 cm /Im0 Do Q\n".to_vec();
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
    let mut bytes = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::new();
    for object in objects {
        offsets.push(bytes.len());
        bytes.extend(format!("{} 0 obj\n", offsets.len()).as_bytes());
        bytes.extend(object);
        bytes.extend(b"\nendobj\n");
    }
    let xref = bytes.len();
    bytes.extend(b"xref\n0 6\n0000000000 65535 f \n");
    for offset in offsets {
        bytes.extend(format!("{offset:010} 00000 n \n").as_bytes());
    }
    bytes.extend(format!("trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes());
    PdfDocument::from_bytes(bytes).unwrap()
}
