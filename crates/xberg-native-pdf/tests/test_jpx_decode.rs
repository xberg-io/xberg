//! JPEG 2000 (`/JPXDecode`) extraction test.
//!
//! Gated on the `jpeg2000` feature (OpenJPEG via `jpeg2k`). Without it, JPX is
//! unsupported by design and this test is skipped. (Unit coverage of the decoder
//! itself lives in `src/decoders/jpx.rs`.)

use xberg_native_pdf::document::PdfDocument;
use xberg_native_pdf::extractors::ColorSpace;
use xberg_native_pdf::rendering::{ImageFormat, RenderOptions, render_page};

#[test]
fn extract_jpx_image_from_pdf() {
    let doc = PdfDocument::open("tests/fixtures/jpx/jpx_minimal.pdf").expect("open JPX repro");
    let images = doc.extract_images(0).expect("extract page-0 images");
    assert!(!images.is_empty(), "no images extracted from the JPX page");

    let png = images[0].to_png_bytes().expect("encode the extracted JPX image as PNG");
    assert!(
        png.len() > 8 && &png[1..4] == b"PNG",
        "extracted JPX image did not encode to a valid PNG"
    );
}

/// One `/JPXDecode` stream in each shape ISO 32000-1 §7.4.9 allows, with its pixel size
/// and the colour space its component count names: a one-component JP2 file, the bare
/// codestream inside it (from the SOC marker on), and a three-component codestream.
fn jpx_streams() -> [(&'static str, Vec<u8>, u32, u32, ColorSpace); 3] {
    let jp2 = std::fs::read("tests/fixtures/jpx/sample_gray.jp2").expect("read the JP2 fixture");
    let soc = jp2
        .windows(4)
        .position(|w| w == [0xFF, 0x4F, 0xFF, 0x51])
        .expect("the JP2 fixture carries a codestream");
    let codestream = jp2[soc..].to_vec();
    let rgb = std::fs::read("tests/fixtures/jpx/sample_rgb.j2k").expect("read the RGB codestream fixture");
    [
        ("grey JP2 file", jp2, 816, 1056, ColorSpace::DeviceGray),
        ("grey codestream", codestream, 816, 1056, ColorSpace::DeviceGray),
        ("RGB codestream", rgb, 64, 64, ColorSpace::DeviceRGB),
    ]
}

/// A one-page PDF whose only content is a full-page `/JPXDecode` image. The image
/// dictionary names no `/ColorSpace` and no `/BitsPerComponent`: ISO 32000-1 §8.9.5
/// (Table 89) lets a JPEG 2000 image omit both, because the stream carries them.
fn pdf_with_jpx_image_without_colorspace(stream: &[u8], width: u32, height: u32) -> Vec<u8> {
    let content: &[u8] = b"q 200 0 0 200 0 0 cm /Im0 Do Q";
    let mut pdf: Vec<u8> = b"%PDF-1.5\n".to_vec();
    let mut offsets = Vec::new();

    offsets.push(pdf.len());
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    offsets.push(pdf.len());
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    offsets.push(pdf.len());
    pdf.extend_from_slice(
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] \
          /Resources << /XObject << /Im0 5 0 R >> >> /Contents 4 0 R >>\nendobj\n",
    );
    offsets.push(pdf.len());
    pdf.extend_from_slice(format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()).as_bytes());
    pdf.extend_from_slice(content);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");
    offsets.push(pdf.len());
    pdf.extend_from_slice(
        format!(
            "5 0 obj\n<< /Type /XObject /Subtype /Image /Width {width} /Height {height} \
             /Filter /JPXDecode /Length {} >>\nstream\n",
            stream.len()
        )
        .as_bytes(),
    );
    pdf.extend_from_slice(stream);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");

    let xref = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \r\n", offsets.len() + 1).as_bytes());
    for offset in &offsets {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \r\n").as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            offsets.len() + 1
        )
        .as_bytes(),
    );
    pdf
}

/// The colour space comes from the stream's component count, rather than the image
/// failing for want of a `/ColorSpace` entry.
#[test]
fn jpx_image_without_colorspace_takes_its_colour_space_from_the_stream() {
    for (shape, stream, width, height, expected) in jpx_streams() {
        let doc = PdfDocument::from_bytes(pdf_with_jpx_image_without_colorspace(&stream, width, height))
            .expect("open the PDF");
        let images = doc
            .extract_images(0)
            .unwrap_or_else(|e| panic!("{shape}: extract page-0 images: {e}"));
        assert_eq!(images.len(), 1, "{shape}: the page holds exactly one image");
        assert_eq!(
            images[0].color_space(),
            &expected,
            "{shape}: the colour space must follow the stream's component count"
        );
        assert_eq!(
            images[0].bits_per_component(),
            8,
            "{shape}: samples are stored at 8 bits"
        );
    }
}

/// The page render paints the image. Before the fix the renderer skipped it as
/// unrenderable and the page came out blank white.
#[test]
fn jpx_image_without_colorspace_is_painted_by_the_page_render() {
    for (shape, stream, width, height, _) in jpx_streams() {
        let doc = PdfDocument::from_bytes(pdf_with_jpx_image_without_colorspace(&stream, width, height))
            .expect("open the PDF");
        let mut options = RenderOptions::default();
        options.dpi = 72;
        options.format = ImageFormat::RawRgba8;
        let page = render_page(&doc, 0, &options).unwrap_or_else(|e| panic!("{shape}: render page 0: {e}"));
        // Each fixture paints well over 1% of the page below near-white; a page whose image
        // was skipped renders pure white. ~keep
        let pixels = page.data.len() / 4;
        let grey_pixels = page.data.chunks_exact(4).filter(|px| px[0] < 240).count();
        assert!(
            grey_pixels * 100 > pixels,
            "{shape}: only {grey_pixels} of {pixels} pixels are painted, so the image was not drawn"
        );
    }
}
