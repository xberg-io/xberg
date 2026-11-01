use super::*;
use lopdf::{dictionary, Document, Object, Stream};

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
    assert!(page_ocr_fallback_image_bytes_with_limits(
        &doc,
        0,
        &crate::extractors::security::SecurityLimits::default()
    )
    .is_empty());
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

fn admission_jpeg_stream(padding: usize) -> Stream {
    let pixels = image::RgbImage::from_pixel(32, 24, image::Rgb([64, 64, 64]));
    let mut jpeg = Vec::new();
    image::codecs::jpeg::JpegEncoder::new(&mut jpeg)
        .encode(&pixels, 32, 24, image::ColorType::Rgb8.into())
        .unwrap();
    if padding != 0 {
        let length = u16::try_from(padding + 2).unwrap().to_be_bytes();
        let mut padded = vec![0xff, 0xd8, 0xff, 0xfe, length[0], length[1]];
        padded.extend(std::iter::repeat_n(b'x', padding));
        padded.extend_from_slice(&jpeg[2..]);
        jpeg = padded;
    }
    let decoded = image::load_from_memory(&jpeg).expect("genuine valid baseline JPEG, including its COM segment");
    assert_eq!((decoded.width(), decoded.height()), (32, 24));
    Stream::new(
        dictionary! { "Type"=>"XObject","Subtype"=>"Image","Width"=>32,"Height"=>24,
        "ColorSpace"=>"DeviceRGB","BitsPerComponent"=>8,"Filter"=>"DCTDecode" },
        jpeg,
    )
}

fn observed_stored_collection(doc: &xberg_native_pdf::PdfDocument, cap: usize) -> (Vec<PageFallbackImage>, Vec<usize>) {
    let mut entered = Vec::new();
    let images = collect_legacy_page_fallback_images(doc, 0, Some(cap), |paint| entered.push(paint));
    (images, entered)
}

#[test]
fn stored_copy_admission_precedes_decode_and_keeps_a_later_fitting_paint() {
    let big = admission_jpeg_stream(1024);
    let small = admission_jpeg_stream(0);
    let cap = small.content.len();
    assert!(big.content.len() > cap);
    let sum = big.content.len() + cap;
    let expected = small.content.clone();
    let doc = image_stream_document(vec![big, small]);
    assert_eq!(doc.page_image_handles(0).unwrap().len(), 2);
    let (images, entered) = observed_stored_collection(&doc, cap);
    assert_eq!(
        entered,
        vec![1],
        "rejected valid JPEG must never enter the actual legacy decode call"
    );
    assert_eq!(images.len(), 1);
    assert_eq!(images[0].paint_order, 1);
    assert_eq!(images[0].format, "jpeg");
    assert_eq!(images[0].bytes.as_ref(), expected);
    let (none, zero) = observed_stored_collection(&doc, 0);
    assert!(none.is_empty() && zero.is_empty());
    let (both, positive) = observed_stored_collection(&doc, sum);
    assert_eq!(
        positive,
        vec![0, 1],
        "entry witness must fire on genuine decoder executions"
    );
    assert_eq!(both.len(), 2);
}

#[test]
fn unsuccessful_legacy_decode_releases_its_stored_copy_reservation() {
    let jpeg = admission_jpeg_stream(0);
    let cap = jpeg.content.len();
    let invalid = Stream::new(
        dictionary! { "Type"=>"XObject","Subtype"=>"Image","Width"=>1,"Height"=>1,
        "ColorSpace"=>"DeviceGray","BitsPerComponent"=>8,"Filter"=>"UnsupportedStoredAdmissionCodec" },
        vec![1; cap],
    );
    let doc = image_stream_document(vec![invalid, jpeg]);
    let handles = doc.page_image_handles(0).unwrap();
    assert_eq!(handles.len(), 2);
    assert!(
        handles[0].decode().is_err(),
        "actual unsupported source codec, never a mock Err"
    );
    drop(handles);
    let (images, entered) = observed_stored_collection(&doc, cap);
    assert_eq!(entered, vec![0, 1]);
    assert_eq!(images.len(), 1);
    assert_eq!(images[0].paint_order, 1);
    assert_eq!(images[0].format, "jpeg");
}

#[test]
fn stored_copy_admission_uses_one_inclusive_per_page_allowance() {
    let first = admission_jpeg_stream(0);
    let second = admission_jpeg_stream(7);
    let sum = first.content.len() + second.content.len();
    let expected = vec![first.content.clone(), second.content.clone()];
    let doc = image_stream_document(vec![first, second]);
    for (cap, count) in [(sum - 1, 1), (sum, 2)] {
        let (images, entered) = observed_stored_collection(&doc, cap);
        assert_eq!(images.len(), count);
        assert_eq!(entered, (0..count).collect::<Vec<_>>());
        for (index, recovered) in images.iter().enumerate() {
            assert_eq!(recovered.paint_order, index);
            assert_eq!(recovered.bytes.as_ref(), expected[index]);
        }
    }
}

fn repeated_paint_document(stream: Stream) -> xberg_native_pdf::PdfDocument {
    let mut pdf = Document::with_version("1.7");
    let pages = pdf.new_object_id();
    let image = pdf.add_object(stream);
    let commands = b"q 32 0 0 24 0 0 cm /Im0 Do Q\nq 32 0 0 24 36 0 cm /Im0 Do Q\n";
    let content = pdf.add_object(Stream::new(dictionary! {}, commands.to_vec()));
    let page = pdf.add_object(dictionary! { "Type"=>"Page","Parent"=>pages,
    "MediaBox"=>vec![0.into(),0.into(),72.into(),72.into()],"Contents"=>content,
    "Resources"=>dictionary! {"XObject"=>dictionary! {"Im0"=>image}} });
    pdf.objects.insert(
        pages,
        Object::Dictionary(dictionary! {"Type"=>"Pages","Count"=>1,
        "Kids"=>vec![Object::Reference(page)]}),
    );
    let catalog = pdf.add_object(dictionary! {"Type"=>"Catalog","Pages"=>pages});
    pdf.trailer.set("Root", catalog);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    xberg_native_pdf::PdfDocument::from_bytes(bytes).unwrap()
}

#[test]
fn repeated_paints_of_one_xobject_are_separate_copy_admissions() {
    let source = admission_jpeg_stream(0);
    let cap = source.content.len();
    let doc = repeated_paint_document(source);
    let handles = doc.page_image_handles(0).unwrap();
    assert_eq!(
        handles.iter().map(|handle| handle.paint_order).collect::<Vec<_>>(),
        vec![0, 1]
    );
    drop(handles);
    for (budget, count) in [(cap, 1), (2 * cap, 2)] {
        let (images, entered) = observed_stored_collection(&doc, budget);
        assert_eq!(images.len(), count);
        assert_eq!(entered, (0..count).collect::<Vec<_>>());
    }
}

#[test]
fn stored_copy_admission_preserves_legacy_raw_and_outer_filter_outputs() {
    let jpeg = admission_jpeg_stream(0);
    let mut outer = jpeg.clone();
    outer.set_content(format!("{}>", hex::encode_upper(&jpeg.content)).into_bytes());
    outer.dict.set(
        "Filter",
        vec![
            Object::Name(b"ASCIIHexDecode".to_vec()),
            Object::Name(b"DCTDecode".to_vec()),
        ],
    );
    let raw = Stream::new(
        dictionary! {"Type"=>"XObject","Subtype"=>"Image","Width"=>32,"Height"=>24,
        "ColorSpace"=>"DeviceGray","BitsPerComponent"=>8},
        vec![128; 32 * 24],
    );
    for (stream, expected_format) in [
        (jpeg, "jpeg"),
        (outer, "jpeg"),
        (raw, "png"),
        (jpeg2000_stream(120, 40, true), "png"),
    ] {
        let cap = stream.content.len();
        let doc = image_stream_document(vec![stream]);
        let legacy = page_ocr_fallback_image_bytes(&doc, 0);
        assert_eq!(legacy.len(), 1);
        assert_eq!(legacy[0].format, expected_format);
        let (bounded, entered) = observed_stored_collection(&doc, cap);
        assert_eq!(entered, vec![0]);
        assert_eq!(bounded.len(), 1);
        assert_eq!(bounded[0].format, legacy[0].format);
        assert_eq!(bounded[0].recovery, legacy[0].recovery);
        assert_eq!(bounded[0].paint_order, legacy[0].paint_order);
        assert_eq!(bounded[0].bytes, legacy[0].bytes);
        assert_eq!(
            image::load_from_memory(&bounded[0].bytes).unwrap().to_rgb8(),
            image::load_from_memory(&legacy[0].bytes).unwrap().to_rgb8()
        );
    }
}
