use super::*;
use lopdf::{Document, Object, Stream, dictionary};

const JP2: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../xberg-native-pdf/tests/fixtures/jpx/gh1885_indices_grey.jp2"
));

fn image_document(width: i64, height: i64, outer_filter: bool) -> xberg_native_pdf::PdfDocument {
    image_stream_document(vec![jpeg2000_stream(width, height, outer_filter)])
}

fn jpeg2000_stream(width: i64, height: i64, outer_filter: bool) -> Stream {
    let mut stream = Stream::new(
        dictionary! {
            "Type" => "XObject", "Subtype" => "Image", "Width" => width, "Height" => height,
            "ColorSpace" => "DeviceGray", "BitsPerComponent" => 8,
        },
        JP2.to_vec(),
    );
    if outer_filter {
        let mut encoded = JP2.iter().map(|byte| format!("{byte:02x}")).collect::<String>();
        encoded.push('>');
        stream.set_content(encoded.into_bytes());
        stream.dict.set(
            "Filter",
            vec![
                Object::Name(b"ASCIIHexDecode".to_vec()),
                Object::Name(b"JPXDecode".to_vec()),
            ],
        );
    } else {
        stream.dict.set("Filter", "JPXDecode");
    }
    stream
}

fn image_stream_document(streams: Vec<Stream>) -> xberg_native_pdf::PdfDocument {
    let mut pdf = Document::with_version("1.7");
    let pages = pdf.new_object_id();
    let mut xobjects = lopdf::Dictionary::new();
    let mut commands = Vec::new();
    let mut identifiers = std::collections::HashSet::new();
    for (index, stream) in streams.into_iter().enumerate() {
        let image = pdf.add_object(stream);
        assert!(identifiers.insert(image), "every image is a distinct XObject");
        xobjects.set(format!("Im{index}"), image);
        commands.extend_from_slice(format!("q 72 0 0 72 0 0 cm /Im{index} Do Q\n").as_bytes());
    }
    let content = pdf.add_object(Stream::new(dictionary! {}, commands));
    let page = pdf.add_object(dictionary! {
        "Type" => "Page", "Parent" => pages,
        "MediaBox" => vec![Object::Integer(0), Object::Integer(0), Object::Integer(72), Object::Integer(72)],
        "Contents" => content, "Resources" => dictionary! { "XObject" => xobjects },
    });
    pdf.objects.insert(
        pages,
        Object::Dictionary(dictionary! {
            "Type" => "Pages", "Kids" => vec![Object::Reference(page)], "Count" => 1,
        }),
    );
    let catalog = pdf.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
    pdf.trailer.set("Root", catalog);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    xberg_native_pdf::PdfDocument::from_bytes(bytes).unwrap()
}

#[test]
fn jpeg2000_fallback_preserves_compressed_pixels_without_png_reencoding() {
    let doc = image_document(120, 40, false);
    let images =
        page_ocr_fallback_image_bytes_with_limits(&doc, 0, &crate::extractors::security::SecurityLimits::default());
    assert_eq!(images.len(), 1, "the valid tiny JP2 must remain recoverable");
    assert_eq!(images[0].format, "jpeg2000");
    assert_eq!(images[0].bytes.as_ref(), JP2);
    assert_eq!(images[0].recovery, XObjectRecovery::RawCompressedStream);
}

#[test]
fn oversized_pdf_dimensions_do_not_require_decoding_a_small_jpeg2000_payload() {
    let doc = image_document(i64::from(u32::MAX), i64::from(u32::MAX), false);
    let handles = doc.page_image_handles(0).unwrap();
    assert_eq!((handles[0].width, handles[0].height), (u32::MAX, u32::MAX));
    let images =
        page_ocr_fallback_image_bytes_with_limits(&doc, 0, &crate::extractors::security::SecurityLimits::default());
    assert_eq!(
        images.len(),
        1,
        "dictionary dimensions must not block bounded compressed passthrough"
    );
    assert_eq!(images[0].format, "jpeg2000");
    assert_eq!(images[0].bytes.as_ref(), JP2);
}

#[test]
fn jpeg2000_behind_an_outer_filter_is_not_treated_as_direct_compressed_bytes() {
    let doc = image_document(120, 40, true);
    let handles = doc.page_image_handles(0).unwrap();
    assert_eq!(handles[0].filter_chain.len(), 2);
    assert!(
        page_ocr_fallback_image_bytes_with_limits(&doc, 0, &crate::extractors::security::SecurityLimits::default())
            .is_empty()
    );
}

#[test]
fn unlimited_legacy_recovery_retains_valid_outer_filtered_jpeg2000() {
    let doc = image_document(120, 40, true);
    let images = page_ocr_fallback_image_bytes(&doc, 0);
    assert_eq!(images.len(), 1);
    assert_eq!(images[0].format, "png");
    let decoded = image::load_from_memory(&images[0].bytes).unwrap();
    assert_eq!((decoded.width(), decoded.height()), (120, 40));
}

#[test]
fn bounded_compressed_fallback_rejects_actual_payload_larger_than_the_request_limit() {
    let doc = image_document(120, 40, false);
    let limits = crate::extractors::security::SecurityLimits {
        max_content_size: JP2.len() - 1,
        ..Default::default()
    };
    assert!(page_ocr_fallback_image_bytes_with_limits(&doc, 0, &limits).is_empty());
}

#[test]
fn bounded_fallback_applies_one_inclusive_copy_limit_to_distinct_image_objects() {
    assert_eq!(JP2.len(), 1497);
    let first = jpeg2000_stream(120, 40, false);
    let mut second = first.clone();
    let marker = b"Created by OpenJPEG";
    let position = second
        .content
        .windows(marker.len())
        .position(|bytes| bytes == marker)
        .unwrap();
    second.content[position] = b'c';
    assert_ne!(first.content, second.content, "change only valid JP2 comment metadata");
    assert_eq!(first.content.len(), second.content.len());
    let second_bytes = second.content.clone();
    let doc = image_stream_document(vec![first, second]);
    let handles = doc.page_image_handles(0).unwrap();
    assert_eq!(handles.len(), 2);
    assert_eq!(
        handles.iter().map(|handle| handle.paint_order).collect::<Vec<_>>(),
        vec![0, 1]
    );
    let legacy = page_ocr_fallback_image_bytes(&doc, 0);
    assert_eq!(legacy.len(), 2, "both distinct JPEG 2000 streams must decode");
    for image in legacy {
        let decoded = image::load_from_memory(&image.bytes).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (120, 40));
    }
    for (cap, expected_count) in [(2 * JP2.len() - 1, 1), (2 * JP2.len(), 2)] {
        let limits = crate::extractors::security::SecurityLimits {
            max_content_size: cap,
            ..Default::default()
        };
        let images = page_ocr_fallback_image_bytes_with_limits(&doc, 0, &limits);
        assert_eq!(images.len(), expected_count, "inclusive total copy cap {cap}");
        assert_eq!(images[0].bytes.as_ref(), JP2, "retain the first painted XObject");
        assert!(images.iter().map(|image| image.bytes.len()).sum::<usize>() <= cap);
        if expected_count == 2 {
            assert_eq!(images[1].bytes.as_ref(), second_bytes.as_slice());
        }
    }
}
