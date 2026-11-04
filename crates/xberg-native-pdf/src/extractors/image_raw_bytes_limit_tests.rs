use super::*;
use crate::object::Object;

const PAYLOAD: &[u8] = b"abcdefgh";

fn source_document() -> crate::PdfDocument {
    let objects = [
        b"<< /Type /Catalog /Pages 2 0 R >>".as_slice(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".as_slice(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 1 1] >>".as_slice(),
        b"<< /Length 8 >>\nstream\nabcdefgh\nendstream".as_slice(),
    ];
    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
        pdf.extend_from_slice(object);
        pdf.extend_from_slice(b"\nendobj\n");
    }
    let start = pdf.len();
    pdf.extend_from_slice(b"xref\n0 5\n0000000000 65535 f \n");
    for offset in offsets {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(format!("trailer\n<< /Size 5 /Root 1 0 R >>\nstartxref\n{start}\n%%EOF\n").as_bytes());
    crate::PdfDocument::from_bytes(pdf).unwrap()
}

fn handle(doc: &crate::PdfDocument, inline: bool) -> PdfImageHandle<'_> {
    let source = if inline {
        PdfImageSource::Inline {
            stream_object: Object::Stream {
                dict: Default::default(),
                data: bytes::Bytes::from_static(PAYLOAD),
            },
            compressed_bytes: bytes::Bytes::from_static(PAYLOAD),
        }
    } else {
        PdfImageSource::XObject(ObjectRef::new(4, 0))
    };
    PdfImageHandle {
        width: 1,
        height: 1,
        color_space: ColorSpace::DeviceGray,
        bits_per_component: 8,
        byte_size_compressed: 0,
        filter_chain: vec![PdfFilter::JPXDecode],
        is_inline: inline,
        paint_order: 0,
        bbox: crate::geometry::Rect::new(0.0, 0.0, 1.0, 1.0),
        rotation_degrees: 0.0,
        ctm: crate::content::Matrix::identity(),
        doc,
        source,
        color_space_resources: Default::default(),
        indexed_base: None,
    }
}

#[test]
fn bounded_raw_copy_rejects_actual_bytes_above_limit_despite_small_metadata() {
    let doc = source_document();
    for inline in [false, true] {
        let image = handle(&doc, inline);
        assert_eq!(image.byte_size_compressed, 0);
        assert!(image.raw_compressed_bytes_with_limit(PAYLOAD.len() - 1).is_err());
    }
}

#[test]
fn bounded_raw_copy_accepts_exact_limit_despite_oversized_metadata() {
    let doc = source_document();
    for inline in [false, true] {
        let mut image = handle(&doc, inline);
        image.byte_size_compressed = u64::MAX;
        image.width = u32::MAX;
        image.height = u32::MAX;
        assert_eq!(image.raw_compressed_bytes_with_limit(PAYLOAD.len()).unwrap(), PAYLOAD);
    }
}

#[test]
fn zero_raw_copy_limit_rejects_nonempty_streams() {
    let doc = source_document();
    for inline in [false, true] {
        assert!(handle(&doc, inline).raw_compressed_bytes_with_limit(0).is_err());
    }
}

#[test]
fn zero_raw_copy_limit_accepts_an_empty_inline_stream() {
    let doc = source_document();
    let mut image = handle(&doc, true);
    image.source = PdfImageSource::Inline {
        stream_object: Object::Stream {
            dict: Default::default(),
            data: bytes::Bytes::new(),
        },
        compressed_bytes: bytes::Bytes::new(),
    };
    assert_eq!(image.raw_compressed_bytes_with_limit(0).unwrap(), Vec::<u8>::new());
}

#[test]
fn bounded_raw_copy_does_not_run_filters_or_pixel_decoders() {
    let doc = source_document();
    let before = decode_call_count();
    for inline in [false, true] {
        assert_eq!(
            handle(&doc, inline)
                .raw_compressed_bytes_with_limit(PAYLOAD.len())
                .unwrap(),
            PAYLOAD
        );
    }
    assert_eq!(decode_call_count(), before);
}

#[test]
fn unlimited_raw_copy_retains_its_existing_behavior() {
    let doc = source_document();
    for inline in [false, true] {
        assert_eq!(handle(&doc, inline).raw_compressed_bytes().unwrap(), PAYLOAD);
    }
}
