//! Regression coverage for `gather_page_signals`' image classification (GH#1732).
//!
//! Lives beside `open.rs` rather than in `document/tests.rs` (322 KB, close to the
//! project's 500 KiB file-safety limit) for the same reason `pdf/native/images/
//! parallel_tests.rs` gives in the `xberg` crate: it keeps that file from growing
//! further while still sitting next to the code it covers.

use super::PdfDocument;
use crate::extractors::auto::ImageCodecClass;
use crate::extractors::images::decode_call_count;

/// A one-page PDF, `page_side`×`page_side`, with a single uncompressed DeviceRGB
/// image XObject painted at `(inset, inset)` with size `image_side`×`image_side`
/// via `cm`. Uncompressed (no `/Filter`) so the image decodes through the same
/// per-pixel raw-buffer path CCITT/JBIG2/Flate images do (the expensive path
/// GH#1732 is about), without needing a real compressed codestream fixture.
fn build_single_page_pdf_with_raw_image(page_side: u32, image_side: u32, inset: u32) -> Vec<u8> {
    let mut buf = Vec::<u8>::new();
    buf.extend_from_slice(b"%PDF-1.4\n");
    let mut offsets = Vec::new();

    offsets.push(buf.len());
    buf.extend_from_slice(b"1 0 obj\n<</Type /Catalog /Pages 2 0 R>>\nendobj\n");

    offsets.push(buf.len());
    buf.extend_from_slice(b"2 0 obj\n<</Type /Pages /Kids [3 0 R] /Count 1>>\nendobj\n");

    offsets.push(buf.len());
    buf.extend_from_slice(
        format!(
            "3 0 obj\n<</Type /Page /Parent 2 0 R /MediaBox [0 0 {page_side} {page_side}] \
             /Resources <</XObject <</Im0 5 0 R>>>> /Contents 4 0 R>>\nendobj\n"
        )
        .as_bytes(),
    );

    let content = format!("q {image_side} 0 0 {image_side} {inset} {inset} cm /Im0 Do Q");
    offsets.push(buf.len());
    buf.extend_from_slice(format!("4 0 obj\n<</Length {}>>\nstream\n", content.len()).as_bytes());
    buf.extend_from_slice(content.as_bytes());
    buf.extend_from_slice(b"\nendstream\nendobj\n");

    let pixel_count = (image_side as usize) * (image_side as usize) * 3;
    offsets.push(buf.len());
    buf.extend_from_slice(
        format!(
            "5 0 obj\n<</Type /XObject /Subtype /Image /Width {image_side} /Height {image_side} \
             /ColorSpace /DeviceRGB /BitsPerComponent 8 /Length {pixel_count}>>\nstream\n"
        )
        .as_bytes(),
    );
    buf.extend((0..pixel_count).map(|i| ((i * 37 + i / 7) % 251) as u8));
    buf.extend_from_slice(b"\nendstream\nendobj\n");

    let xref_offset = buf.len();
    let total_objs = 6;
    buf.extend_from_slice(b"xref\n");
    buf.extend_from_slice(format!("0 {total_objs}\n").as_bytes());
    buf.extend_from_slice(b"0000000000 65535 f \n");
    for off in &offsets {
        buf.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    buf.extend_from_slice(format!("trailer\n<</Size {total_objs} /Root 1 0 R>>\n").as_bytes());
    buf.extend_from_slice(format!("startxref\n{xref_offset}\n%%EOF\n").as_bytes());
    buf
}

/// `classify_page` (via `gather_page_signals`) must decode zero embedded images: it
/// only needs bbox + filter-chain metadata, both available from the cheap Phase 1
/// `page_image_handles` walk. Before the GH#1732 fix this called `extract_images`,
/// which fully decoded every embedded image just to classify its codec.
///
/// Asserts a *delta* around each call rather than an absolute counter value so the
/// test is immune to other tests incrementing the same process-wide-if-it-were-an-
/// AtomicUsize counter; `decode_call_count` is thread-local instead; see its doc
/// comment. The second half proves the counter itself is wired to real decode work
/// (`extract_images` must move it) rather than reading 0 vacuously either way. ~keep
#[test]
fn classify_page_does_not_decode_embedded_images() {
    let pdf = build_single_page_pdf_with_raw_image(200, 150, 25);
    let doc = PdfDocument::from_bytes(pdf).expect("fixture must open");

    let before_classify = decode_call_count();
    let classification = doc.classify_page(0).expect("classify_page must succeed");
    let after_classify = decode_call_count();
    assert_eq!(
        after_classify,
        before_classify,
        "classify_page must not decode any embedded image, got a decode-count delta of {}",
        after_classify - before_classify
    );

    // The page's image still contributes its area/codec to the signals -- avoiding
    // the decode must not make the image invisible to classification.
    assert_eq!(classification.signals.codec, ImageCodecClass::Other);

    let before_extract = decode_call_count();
    let images = doc.extract_images(0).expect("extract_images must succeed");
    let after_extract = decode_call_count();
    assert_eq!(images.len(), 1, "fixture has exactly one embedded image");
    assert_eq!(
        after_extract - before_extract,
        1,
        "extract_images must decode exactly the one embedded image on the page \
         (proves decode_call_count is wired to real decode work, not vacuously 0)"
    );
}

/// `image_area_ratio` computed from the Phase 1 handle's `bbox` (GH#1732's
/// no-decode path) must equal the value the old full-decode path produced: the
/// image is painted at (25,25) sized 150x150 on a 200x200 page, so the
/// intersection area is exactly 150*150 = 22500 and the ratio is 22500/40000.
#[test]
fn classify_page_image_area_ratio_matches_expected_bbox_intersection() {
    let pdf = build_single_page_pdf_with_raw_image(200, 150, 25);
    let doc = PdfDocument::from_bytes(pdf).expect("fixture must open");

    let classification = doc.classify_page(0).expect("classify_page must succeed");
    let expected_ratio = (150.0 * 150.0) / (200.0 * 200.0);
    assert!(
        (classification.signals.image_area_ratio - expected_ratio).abs() < 1e-5,
        "expected image_area_ratio {expected_ratio}, got {}",
        classification.signals.image_area_ratio
    );
}

/// A one-page PDF with no `/XObject` resource and an empty content stream.
fn build_single_page_pdf_without_images(page_side: u32) -> Vec<u8> {
    let mut buf = Vec::<u8>::new();
    buf.extend_from_slice(b"%PDF-1.4\n");
    let mut offsets = Vec::new();

    offsets.push(buf.len());
    buf.extend_from_slice(b"1 0 obj\n<</Type /Catalog /Pages 2 0 R>>\nendobj\n");

    offsets.push(buf.len());
    buf.extend_from_slice(b"2 0 obj\n<</Type /Pages /Kids [3 0 R] /Count 1>>\nendobj\n");

    offsets.push(buf.len());
    buf.extend_from_slice(
        format!(
            "3 0 obj\n<</Type /Page /Parent 2 0 R /MediaBox [0 0 {page_side} {page_side}] \
             /Resources <<>> /Contents 4 0 R>>\nendobj\n"
        )
        .as_bytes(),
    );

    offsets.push(buf.len());
    buf.extend_from_slice(b"4 0 obj\n<</Length 0>>\nstream\n\nendstream\nendobj\n");

    let xref_offset = buf.len();
    let total_objs = 5;
    buf.extend_from_slice(b"xref\n");
    buf.extend_from_slice(format!("0 {total_objs}\n").as_bytes());
    buf.extend_from_slice(b"0000000000 65535 f \n");
    for off in &offsets {
        buf.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    buf.extend_from_slice(format!("trailer\n<</Size {total_objs} /Root 1 0 R>>\n").as_bytes());
    buf.extend_from_slice(format!("startxref\n{xref_offset}\n%%EOF\n").as_bytes());
    buf
}

/// A page with no embedded images must still classify without decoding anything,
/// and must report `ImageCodecClass::None` and a zero image area ratio.
#[test]
fn classify_page_with_no_images_reports_none_codec() {
    let pdf = build_single_page_pdf_without_images(200);
    let doc = PdfDocument::from_bytes(pdf).expect("fixture must open");

    let before = decode_call_count();
    let classification = doc.classify_page(0).expect("classify_page must succeed");
    let after = decode_call_count();
    assert_eq!(after, before, "a page with no images must still decode nothing");
    assert_eq!(classification.signals.codec, ImageCodecClass::None);
    assert_eq!(classification.signals.image_area_ratio, 0.0);
}

/// A PDF whose objects are `bodies`, numbered from 1, with object 1 the catalog.
fn pdf_from_objects(bodies: &[Vec<u8>]) -> Vec<u8> {
    let mut buf = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::with_capacity(bodies.len());
    for (index, body) in bodies.iter().enumerate() {
        offsets.push(buf.len());
        buf.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
        buf.extend_from_slice(body);
        buf.extend_from_slice(b"\nendobj\n");
    }
    let xref_offset = buf.len();
    let total_objs = bodies.len() + 1;
    buf.extend_from_slice(format!("xref\n0 {total_objs}\n0000000000 65535 f \n").as_bytes());
    for off in &offsets {
        buf.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    buf.extend_from_slice(format!("trailer\n<</Size {total_objs} /Root 1 0 R>>\n").as_bytes());
    buf.extend_from_slice(format!("startxref\n{xref_offset}\n%%EOF\n").as_bytes());
    buf
}

fn stream_object(dict: &str, data: &[u8]) -> Vec<u8> {
    let mut body = format!("<<{dict} /Length {}>>\nstream\n", data.len()).into_bytes();
    body.extend_from_slice(data);
    body.extend_from_slice(b"\nendstream");
    body
}

/// A 200x200 page painting a 150x150 DeviceRGB image, a 4x4 CCITT stamp under the 8x8 px size
/// floor, and then `text_ops`.
fn image_and_text_page(text_ops: &str) -> Vec<u8> {
    let content = format!("q 150 0 0 150 25 25 cm /Im0 Do Q q 4 0 0 4 0 0 cm /Im1 Do Q {text_ops}");
    let pixels: Vec<u8> = (0..150 * 150 * 3).map(|i| (i % 251) as u8).collect();
    pdf_from_objects(&[
        b"<</Type /Catalog /Pages 2 0 R>>".to_vec(),
        b"<</Type /Pages /Kids [3 0 R] /Count 1>>".to_vec(),
        b"<</Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Contents 4 0 R /Resources \
           <</XObject <</Im0 5 0 R /Im1 6 0 R>> \
           /Font <</F1 <</Type /Font /Subtype /Type1 /BaseFont /Helvetica>>>>>>>>"
            .to_vec(),
        stream_object("", content.as_bytes()),
        stream_object(
            "/Type /XObject /Subtype /Image /Width 150 /Height 150 /ColorSpace /DeviceRGB /BitsPerComponent 8",
            &pixels,
        ),
        stream_object(
            "/Type /XObject /Subtype /Image /Width 4 /Height 4 /ColorSpace /DeviceGray /BitsPerComponent 1 \
             /Filter /CCITTFaxDecode",
            &[0, 0],
        ),
    ])
}

/// `scan_text_signals` and `image_codec_class` exist so scan grading can skip the rest of
/// `classify_page`; every field they report must equal the one `classify_page` reports.
#[test]
fn scan_signals_equal_the_classify_page_signals() {
    let cases = [
        (
            "BT /F1 12 Tf 20 180 Td (A running header over a scanned sheet) Tj ET",
            false,
        ),
        (
            "BT 3 Tr /F1 12 Tf 20 100 Td (An invisible OCR sidecar line of text) Tj ET",
            true,
        ),
        ("", false),
    ];
    for (text_ops, invisible) in cases {
        let doc = PdfDocument::from_bytes(image_and_text_page(text_ops)).expect("fixture must open");
        let classified = doc.classify_page(0).expect("classify_page must succeed").signals;
        let scan = doc.scan_text_signals(0).expect("scan_text_signals must succeed");
        let handles = doc.page_image_handles(0).expect("image handles must enumerate");

        assert_eq!(scan.text_glyph_count, classified.text_glyph_count, "{text_ops:?}");
        assert_eq!(
            scan.text_area_ratio.to_bits(),
            classified.text_area_ratio.to_bits(),
            "{text_ops:?}"
        );
        assert_eq!(
            scan.invisible_text_ratio.to_bits(),
            classified.invisible_text_ratio.to_bits(),
            "{text_ops:?}"
        );
        assert_eq!(scan.producer_prior, classified.producer_prior, "{text_ops:?}");
        assert_eq!(
            PdfDocument::image_codec_class(&handles),
            classified.codec,
            "{text_ops:?}"
        );

        assert_eq!(handles.len(), 2, "both images must be enumerated, the stamp included");
        assert_eq!(
            classified.codec,
            ImageCodecClass::Other,
            "the CCITT stamp is under the size floor and must not set the codec"
        );
        if text_ops.is_empty() {
            assert_eq!(scan.text_glyph_count, 0);
        } else if !invisible {
            assert!(scan.text_glyph_count > 0, "visible text must be counted: {text_ops:?}");
            assert!(scan.text_area_ratio > 0.0, "visible text must cover area: {text_ops:?}");
        }
        assert_eq!(scan.invisible_text_ratio > 0.0, invisible, "{text_ops:?}");
    }
}

#[test]
fn image_codec_class_prefers_ccitt_and_skips_images_under_the_size_floor() {
    let doc = PdfDocument::from_bytes(image_and_text_page("")).expect("fixture must open");
    let handles = doc.page_image_handles(0).expect("image handles must enumerate");
    assert_eq!(PdfDocument::image_codec_class(&handles[1..]), ImageCodecClass::None);
    assert_eq!(PdfDocument::image_codec_class(&handles[..1]), ImageCodecClass::Other);
    assert_eq!(PdfDocument::image_codec_class(&[]), ImageCodecClass::None);
}
