//! GH#1900: a `/JPXDecode` image whose dictionary `/Width` or `/Height` disagrees with its
//! codestream must extract at the codestream's size, not the dictionary's.
//!
//! ISO 32000-1:2008 §7.4.9 treats `/Width` and `/Height` as informative for a JPEG 2000 image --
//! the codestream is authoritative for its own dimensions. Before this fix, `PdfImage` reported
//! the dictionary's declared size while its pixel buffer was sized to the codestream: a consumer
//! that trusted `width * height * components` either read a scrambled prefix of the buffer at the
//! wrong stride, or rejected the buffer outright as the wrong length and dropped the image -- and
//! neither case recorded why.
//!
//! Both directions from the issue are covered here: a declared width half the codestream's, and
//! double it. The fixture declares `/ColorSpace /DeviceCMYK` explicitly (matching the codestream's
//! real 4 components) so this test exercises only the size disagreement, not the separate
//! placeholder-`/ColorSpace` component-count logic covered by `gh1883_bare_cmyk_jpx_without_colorspace.rs`.

use xberg_native_pdf::document::PdfDocument;
use xberg_native_pdf::extractors::warnings::WarningCategory;
use xberg_native_pdf::extractors::{ColorSpace, ImageData, PixelFormat};

/// A 16x16 lossless four-component raw J2K codestream, one channel saturated per quadrant. Shared
/// with several other JPX regression tests; bare, so it carries no `colr`/`cdef` box. ~keep
const CMYK_QUADRANTS_J2K: &[u8] = include_bytes!("fixtures/jpx/gh1855_cmyk_quadrants.j2k");

/// A 120x40 lossless one-component index plane: dark text (index 0) on paper (index 255).
const INDICES_GREY_JP2: &[u8] = include_bytes!("fixtures/jpx/gh1885_indices_grey.jp2");

/// Build a single-page PDF painting `codestream` through a `/JPXDecode` image XObject in
/// `color_space` that declares `dict_width` x `dict_height` -- which may disagree with the
/// codestream's actual size.
fn build_pdf_with_jpx_image(codestream: &[u8], color_space: &str, dict_width: u32, dict_height: u32) -> Vec<u8> {
    let content: &[u8] = b"q\n50 0 0 50 25 25 cm\n/Im1 Do\nQ\n";
    let mut buf: Vec<u8> = Vec::new();
    let mut offsets: Vec<usize> = Vec::new();
    buf.extend_from_slice(b"%PDF-1.5\n");

    offsets.push(buf.len());
    buf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    offsets.push(buf.len());
    buf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

    offsets.push(buf.len());
    buf.extend_from_slice(
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R \
           /Resources << /XObject << /Im1 5 0 R >> >> >>\nendobj\n",
    );

    offsets.push(buf.len());
    buf.extend_from_slice(format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()).as_bytes());
    buf.extend_from_slice(content);
    buf.extend_from_slice(b"\nendstream\nendobj\n");

    offsets.push(buf.len());
    buf.extend_from_slice(
        format!(
            "5 0 obj\n<< /Type /XObject /Subtype /Image /Width {dict_width} /Height {dict_height} \
             /ColorSpace {color_space} /BitsPerComponent 8 /Filter /JPXDecode /Length {} >>\nstream\n",
            codestream.len()
        )
        .as_bytes(),
    );
    buf.extend_from_slice(codestream);
    buf.extend_from_slice(b"\nendstream\nendobj\n");

    let xref_offset = buf.len();
    buf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", offsets.len() + 1).as_bytes());
    for offset in &offsets {
        buf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    buf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            offsets.len() + 1,
            xref_offset
        )
        .as_bytes(),
    );
    buf
}

/// Assert the extracted image (from a fixture whose dictionary declared `dict_width` x
/// `dict_height` against the 16x16 codestream) landed at the codestream's own size, with all four
/// CMYK planes intact, and that a `JpxSizeMismatch` warning naming both sizes was recorded.
fn assert_extracted_at_codestream_size_and_warned(dict_width: u32, dict_height: u32) {
    let doc = PdfDocument::from_bytes(build_pdf_with_jpx_image(
        CMYK_QUADRANTS_J2K,
        "/DeviceCMYK",
        dict_width,
        dict_height,
    ))
    .expect("parse the size-mismatched JPX fixture");

    let images = doc.extract_images(0).expect("extract page 0");
    assert_eq!(
        images.len(),
        1,
        "the JPXDecode image must extract despite the dictionary/codestream size disagreement"
    );

    assert_eq!(
        images[0].width(),
        16,
        "GH#1900: the codestream's declared width must win over the dictionary's {dict_width}"
    );
    assert_eq!(
        images[0].height(),
        16,
        "GH#1900: the codestream's declared height must win over the dictionary's {dict_height}"
    );

    match images[0].data() {
        ImageData::Raw { pixels, format } => {
            assert_eq!(*format, PixelFormat::CMYK, "the decoded buffer must be four-channel");
            assert_eq!(
                pixels.len(),
                16 * 16 * 4,
                "the pixel buffer must be sized to the codestream, matching the reported width/height"
            );
        }
        other => panic!("a JPXDecode image must decode to raw samples, got {other:?}"),
    }
    assert_eq!(*images[0].color_space(), ColorSpace::DeviceCMYK);

    let needle = format!("dictionary declares {dict_width}x{dict_height}");
    let warnings = doc.structured_warnings();
    assert!(
        warnings
            .iter()
            .any(|w| w.category == WarningCategory::JpxSizeMismatch && w.message.contains(&needle)),
        "expected a JpxSizeMismatch warning naming the declared size {dict_width}x{dict_height}; got: {:?}",
        warnings
            .iter()
            .filter(|w| w.category == WarningCategory::JpxSizeMismatch)
            .map(|w| &w.message)
            .collect::<Vec<_>>()
    );
}

#[test]
fn a_dictionary_width_half_the_codestreams_extracts_at_the_codestreams_size() {
    assert_extracted_at_codestream_size_and_warned(8, 16);
}

#[test]
fn a_dictionary_width_double_the_codestreams_extracts_at_the_codestreams_size() {
    assert_extracted_at_codestream_size_and_warned(32, 16);
}

/// Scope control: a dictionary that agrees with the codestream must extract cleanly with no
/// `JpxSizeMismatch` warning naming its size -- the check above must not fire on every JPX image.
#[test]
fn a_dictionary_that_agrees_with_the_codestream_extracts_with_no_size_warning() {
    let doc = PdfDocument::from_bytes(build_pdf_with_jpx_image(CMYK_QUADRANTS_J2K, "/DeviceCMYK", 16, 16))
        .expect("parse the size-agreeing JPX fixture");
    let images = doc.extract_images(0).expect("extract page 0");
    assert_eq!(images.len(), 1);
    assert_eq!(images[0].width(), 16);
    assert_eq!(images[0].height(), 16);

    assert!(
        !doc.structured_warnings()
            .iter()
            .any(|w| w.category == WarningCategory::JpxSizeMismatch),
        "an agreeing dictionary must not raise a size-mismatch warning"
    );
}

/// An `/Indexed` image decodes its codestream to palette indices, and the codestream's size wins
/// there too: the image extracts at 120x40 with the same pixels whatever the dictionary declares,
/// and the disagreement is recorded.
#[test]
fn an_indexed_image_extracts_at_the_codestreams_size() {
    let entries: String = (0..=255u8).map(|i| format!("{i:02X}{i:02X}{i:02X}")).collect();
    let palette = format!("[/Indexed /DeviceRGB 255 <{entries}>]");
    let extract = |dict_width: u32| {
        let doc = PdfDocument::from_bytes(build_pdf_with_jpx_image(INDICES_GREY_JP2, &palette, dict_width, 40))
            .expect("parse the indexed JPX fixture");
        let images = doc.extract_images(0).expect("extract page 0");
        assert_eq!(images.len(), 1, "the image must extract, not drop");
        let ImageData::Raw { pixels, .. } = images[0].data() else {
            panic!("an indexed JPX image decodes to raw samples");
        };
        (
            images[0].width(),
            images[0].height(),
            pixels.clone(),
            doc.structured_warnings(),
        )
    };
    let (_, _, control, _) = extract(120);
    for dict_width in [60, 240] {
        let (width, height, pixels, warnings) = extract(dict_width);
        assert_eq!(
            (width, height),
            (120, 40),
            "/Width {dict_width}: the codestream's size wins"
        );
        assert!(
            pixels == control,
            "/Width {dict_width}: the pixels must be the control's"
        );
        let needle = format!("dictionary declares {dict_width}x40");
        assert!(
            warnings
                .iter()
                .any(|w| w.category == WarningCategory::JpxSizeMismatch && w.message.contains(&needle)),
            "/Width {dict_width}: the disagreement must be recorded"
        );
    }
}

#[test]
fn concurrent_documents_keep_their_own_jpx_size_warnings() {
    let widths = [37, 39];
    let documents = std::thread::scope(|scope| {
        let threads = widths.map(|width| {
            scope.spawn(move || {
                let doc =
                    PdfDocument::from_bytes(build_pdf_with_jpx_image(CMYK_QUADRANTS_J2K, "/DeviceCMYK", width, 16))
                        .expect("open document");
                let images = doc.extract_images(0).expect("decode mismatched image");
                assert_eq!(images.len(), 1);
                assert_eq!(images[0].width(), 16);
                doc
            })
        });
        threads.map(|thread| thread.join().expect("extract document image"))
    });

    for (width, doc) in widths.into_iter().zip(documents) {
        let warnings = doc.structured_warnings();
        let sized_warnings: Vec<_> = warnings
            .iter()
            .filter(|warning| warning.category == WarningCategory::JpxSizeMismatch)
            .collect();
        assert_eq!(
            sized_warnings.len(),
            1,
            "document with declared width {width}: {sized_warnings:?}"
        );
        assert!(
            sized_warnings[0]
                .message
                .contains(&format!("dictionary declares {width}x16"))
        );
    }
}
