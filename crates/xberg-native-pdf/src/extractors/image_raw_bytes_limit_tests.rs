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

fn image_length_document(length_entry: &str) -> crate::PdfDocument {
    let content = "q 1 0 0 1 0 0 cm /Im0 Do Q";
    let image = format!(
        "<< /Type /XObject /Subtype /Image /Width 1 /Height 1 /ColorSpace /DeviceGray /BitsPerComponent 8 /Filter /JPXDecode /Length {length_entry} >>\nstream\nabcdefgh\nendstream"
    );
    let objects = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 1 1] /Resources << /XObject << /Im0 4 0 R >> >> /Contents 5 0 R >>".to_string(),
        image,
        format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len()),
        "1".to_string(),
    ];
    let mut bytes = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        bytes.extend_from_slice(format!("{} 0 obj\n{object}\nendobj\n", index + 1).as_bytes());
    }
    let start = bytes.len();
    bytes.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes());
    for offset in offsets {
        bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    bytes.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{start}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    crate::PdfDocument::from_bytes(bytes).unwrap()
}

#[test]
fn stored_stream_length_uses_actual_bytes_with_misleading_direct_length() {
    for declared in [0u64, 1, 8, 15] {
        let doc = image_length_document(&declared.to_string());
        let images = doc.page_image_handles(0).unwrap();
        assert_eq!(images.len(), 1);
        assert_eq!(images[0].byte_size_compressed, declared);
        assert_eq!(
            images[0].raw_compressed_bytes().unwrap(),
            PAYLOAD,
            "actual parser payload precondition"
        );
        let before = decode_call_count();
        assert_eq!(images[0].stored_stream_len().unwrap(), PAYLOAD.len());
        assert!(images[0].raw_compressed_bytes_with_limit(PAYLOAD.len() - 1).is_err());
        assert_eq!(
            images[0].raw_compressed_bytes_with_limit(PAYLOAD.len()).unwrap(),
            PAYLOAD
        );
        assert_eq!(decode_call_count(), before, "inspection must not invoke filters/pixels");
    }
}

#[test]
fn stored_stream_length_uses_actual_bytes_with_indirect_length() {
    let doc = image_length_document("6 0 R");
    let images = doc.page_image_handles(0).unwrap();
    assert_eq!(images.len(), 1);
    assert_eq!(images[0].byte_size_compressed, 1);
    assert_eq!(
        images[0].raw_compressed_bytes().unwrap(),
        PAYLOAD,
        "actual parser payload precondition"
    );
    assert_eq!(images[0].stored_stream_len().unwrap(), PAYLOAD.len());
}

#[test]
fn stored_stream_length_handles_inline_empty_and_reported_size_independently() {
    let doc = source_document();
    for reported in [0, u64::MAX] {
        let mut image = handle(&doc, true);
        image.byte_size_compressed = reported;
        assert_eq!(image.stored_stream_len().unwrap(), PAYLOAD.len());
    }
    let mut empty = handle(&doc, true);
    empty.source = PdfImageSource::Inline {
        stream_object: Object::Stream {
            dict: Default::default(),
            data: bytes::Bytes::new(),
        },
        compressed_bytes: bytes::Bytes::new(),
    };
    assert_eq!(empty.stored_stream_len().unwrap(), 0);
    assert_eq!(empty.raw_compressed_bytes_with_limit(0).unwrap(), Vec::<u8>::new());
}

#[test]
fn stored_stream_length_rejects_a_nonstream_xobject_reference() {
    let doc = source_document();
    let mut image = handle(&doc, false);
    image.source = PdfImageSource::XObject(ObjectRef::new(1, 0));
    assert!(image.stored_stream_len().is_err());
}

#[test]
fn consuming_jpeg_data_preserves_the_owned_allocation() {
    let mut payload = Vec::with_capacity(64);
    payload.extend_from_slice(PAYLOAD);
    let pointer = payload.as_ptr();
    let capacity = payload.capacity();
    let image = PdfImage::new(1, 1, ColorSpace::DeviceGray, 8, ImageData::Jpeg(payload));
    let ImageData::Jpeg(owned) = image.into_data() else {
        panic!("JPEG ownership variant changed")
    };
    assert_eq!(owned, PAYLOAD);
    assert_eq!(owned.as_ptr(), pointer);
    assert_eq!(owned.capacity(), capacity);
    let copied = owned.clone();
    assert_ne!(
        copied.as_ptr(),
        pointer,
        "live clone is a causal negative for allocation identity"
    );
    let bytes = bytes::Bytes::from(owned);
    assert_eq!(bytes.as_ptr(), pointer);
    assert_eq!(bytes.as_ref(), PAYLOAD);
}

#[test]
fn consuming_raw_data_preserves_pixels_format_and_allocation() {
    let mut payload = Vec::with_capacity(64);
    payload.extend_from_slice(&[1, 2, 3, 4, 5, 6]);
    let pointer = payload.as_ptr();
    let capacity = payload.capacity();
    let image = PdfImage::new(
        2,
        1,
        ColorSpace::DeviceRGB,
        8,
        ImageData::Raw {
            pixels: payload,
            format: PixelFormat::RGB,
        },
    );
    let ImageData::Raw { pixels, format } = image.into_data() else {
        panic!("raw ownership variant changed")
    };
    assert_eq!(pixels, vec![1, 2, 3, 4, 5, 6]);
    assert!(matches!(format, PixelFormat::RGB));
    assert_eq!(pixels.as_ptr(), pointer);
    assert_eq!(pixels.capacity(), capacity);
}
