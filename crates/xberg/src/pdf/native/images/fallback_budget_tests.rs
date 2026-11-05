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
fn one_undecodable_enumerated_xobject_sets_the_failure_flag() {
    let broken = Stream::new(
        dictionary! {
            "Type" => "XObject", "Subtype" => "Image", "Width" => 8, "Height" => 8,
            "ColorSpace" => "DeviceGray", "BitsPerComponent" => 8, "Filter" => "FlateDecode",
        },
        b"This is not zlib compressed data".to_vec(),
    );
    for (painted, expected) in [(0, false), (1, true), (2, false)] {
        let doc = image_stream_document(vec![broken.clone(); painted]);
        assert_eq!(doc.page_image_handles(0).unwrap().len(), painted);
        let (images, sole_failure) = page_ocr_fallback_image_bytes(&doc, 0, None, None);
        assert_eq!(images.len(), 0);
        assert_eq!(sole_failure, expected, "{painted} painted XObjects");
    }
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
    let (images, sole_failure) = page_ocr_fallback_image_bytes(&doc, 0, None, None);
    assert!(!sole_failure);
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
    let (legacy, _) = page_ocr_fallback_image_bytes(&doc, 0, None, None);
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

#[test]
fn stored_jpeg_admission_keeps_legacy_jpeg2000_png_bytes_and_provenance() {
    for outer_filter in [false, true] {
        let doc = image_document(120, 40, outer_filter);
        let (legacy, legacy_failure) = page_ocr_fallback_image_bytes(&doc, 0, None, None);
        let mut admission = StoredJpegRecoveryBudget::new(0);
        let (recovered, failure) = page_ocr_fallback_image_bytes(&doc, 0, None, Some(&mut admission));
        assert_eq!((legacy_failure, failure), (false, false));
        assert_eq!(recovered.len(), 1);
        assert_eq!(recovered[0].format, "png");
        assert_eq!(recovered[0].recovery, XObjectRecovery::ReencodedPixels);
        assert_eq!(recovered[0].paint_order, 0);
        assert_eq!(recovered[0].bytes, legacy[0].bytes);
        let decoded = image::load_from_memory(&recovered[0].bytes).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (120, 40));
    }
}

fn padded_jpeg(size: usize) -> Vec<u8> {
    let mut jpeg = Vec::new();
    let pixels = image::GrayImage::from_pixel(32, 24, image::Luma([192]));
    image::codecs::jpeg::JpegEncoder::new(&mut jpeg)
        .encode(pixels.as_raw(), 32, 24, image::ColorType::L8.into())
        .unwrap();
    let mut padded = jpeg[..2].to_vec();
    let mut remaining = size - jpeg.len();
    while remaining != 0 {
        const MAX_COMMENT_SIZE: usize = u16::MAX as usize + 2;
        let mut segment_size = remaining.min(MAX_COMMENT_SIZE);
        if (1..4).contains(&(remaining - segment_size)) {
            segment_size -= 4;
        }
        padded.extend([0xff, 0xfe]);
        padded.extend(u16::try_from(segment_size - 2).unwrap().to_be_bytes());
        padded.resize(padded.len() + segment_size - 4, b'x');
        remaining -= segment_size;
    }
    padded.extend(&jpeg[2..]);
    assert_eq!(padded.len(), size);
    let decoded = image::load_from_memory(&padded).unwrap();
    assert_eq!((decoded.width(), decoded.height()), (32, 24));
    padded
}

fn assert_jpeg_recovery_at_native_cap(payload: Vec<u8>, expected: Option<XObjectRecovery>) {
    let stream = Stream::new(
        dictionary! {
            "Type" => "XObject", "Subtype" => "Image", "Width" => 32, "Height" => 24,
            "ColorSpace" => "DeviceGray", "BitsPerComponent" => 8, "Filter" => "DCTDecode",
        },
        payload.clone(),
    );
    let doc = image_stream_document(vec![stream]);
    let (legacy, legacy_failed) = page_ocr_fallback_image_bytes(&doc, 0, None, None);
    let mut admission = StoredJpegRecoveryBudget::new(payload.len());
    let (recovered, failed) = page_ocr_fallback_image_bytes(&doc, 0, None, Some(&mut admission));
    assert_eq!((legacy_failed, failed), (expected.is_none(), expected.is_none()));
    for images in [legacy, recovered] {
        assert_eq!(images.len(), usize::from(expected.is_some()));
        if let Some(mode) = expected {
            assert_eq!(images[0].recovery, mode);
            assert_eq!(images[0].format, "jpeg");
            assert_eq!(images[0].paint_order, 0);
            assert_eq!(images[0].bytes.as_ref(), payload);
        }
    }
}

#[test]
fn stored_jpeg_admission_preserves_native_cap_fallback_and_magic_check() {
    const CHILD: &str = "XBERG_TEST_STORED_JPEG_NATIVE_CAP_CHILD";
    const NATIVE_CAP: usize = 1024 * 1024;
    if std::env::var_os(CHILD).is_some() {
        assert_jpeg_recovery_at_native_cap(padded_jpeg(NATIVE_CAP), Some(XObjectRecovery::EmbeddedJpeg));
        assert_jpeg_recovery_at_native_cap(padded_jpeg(NATIVE_CAP + 1), Some(XObjectRecovery::RawCompressedStream));
        assert_jpeg_recovery_at_native_cap(vec![0; NATIVE_CAP + 1], None);
        return;
    }

    let name = concat!(
        module_path!(),
        "::stored_jpeg_admission_preserves_native_cap_fallback_and_magic_check"
    );
    let name = name.strip_prefix("xberg::").unwrap();
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", name])
        .env(CHILD, "1")
        .env("XBERG_NATIVE_PDF_MAX_DECOMPRESS_MB", "1")
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success() && stdout.contains("1 passed; 0 failed; 0 ignored"),
        "{stdout}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
