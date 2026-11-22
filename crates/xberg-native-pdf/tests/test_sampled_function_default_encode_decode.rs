//! A sampled tint transform that writes out its *default* `/Encode` and
//! `/Decode` must still be evaluated.
//!
//! ISO 32000-1:2008 Table 39 (`docs/spec/pdf.md:6903`) gives a Type 0 function
//! the defaults `/Encode [0 (Size_0 − 1) 0 (Size_1 − 1) …]` and `/Decode`
//! "same as the value of Range". A dictionary that states those explicitly
//! means exactly what an absent entry means.
//!
//! The evaluator declined on *presence* rather than on value, while its own
//! doc comment said it declined a "non-default" `/Encode`/`/Decode` — so a
//! conforming file fell back to the `1 - tint` grey approximation instead of
//! its real colour. Measured over a 154-document sample, 11 of 122 sampled
//! function dictionaries carried both keys and all 11 held the defaults.

use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::rendering::{ImageFormat, RenderOptions, render_page};

/// A page filled at tint 1.0 through a `/Separation` whose tint transform is
/// a Type 0 sampled function. `extra` is spliced into the function dictionary.
///
/// The sample table is 2 entries x 3 outputs, 8 bits: tint 0 -> white,
/// tint 1 -> pure blue. `/Size [2]`, so the default `/Encode` is `[0 1]` and
/// the default `/Decode` is `/Range`, i.e. `[0 1 0 1 0 1]`.
fn sampled_separation_pdf(extra: &str) -> Vec<u8> {
    let samples: [u8; 6] = [255, 255, 255, 0, 0, 255];

    let mut buf: Vec<u8> = Vec::new();
    let mut off = vec![0usize; 6];
    let obj = |buf: &mut Vec<u8>, off: &mut Vec<usize>, id: usize, body: &str| {
        off[id] = buf.len();
        buf.extend_from_slice(format!("{id} 0 obj\n{body}\nendobj\n").as_bytes());
    };

    buf.extend_from_slice(b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n");
    obj(&mut buf, &mut off, 1, "<< /Type /Catalog /Pages 2 0 R >>");
    obj(&mut buf, &mut off, 2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>");
    obj(
        &mut buf,
        &mut off,
        3,
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << /ColorSpace << /CS0 [/Separation /Spot /DeviceRGB 5 0 R] >> >> \
         /Contents 4 0 R >>",
    );
    let content = b"/CS0 cs 1 scn 0 0 100 100 re f\n";
    off[4] = buf.len();
    buf.extend_from_slice(format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()).as_bytes());
    buf.extend_from_slice(content);
    buf.extend_from_slice(b"\nendstream\nendobj\n");

    off[5] = buf.len();
    buf.extend_from_slice(
        format!(
            "5 0 obj\n<< /FunctionType 0 /Domain [0 1] /Range [0 1 0 1 0 1] /Size [2] \
             /BitsPerSample 8 {extra} /Length {} >>\nstream\n",
            samples.len()
        )
        .as_bytes(),
    );
    buf.extend_from_slice(&samples);
    buf.extend_from_slice(b"\nendstream\nendobj\n");

    let xref = buf.len();
    buf.extend_from_slice(b"xref\n0 6\n0000000000 65535 f \n");
    for offset in &off[1..=5] {
        buf.extend_from_slice(format!("{:010} 00000 n \n", offset).as_bytes());
    }
    buf.extend_from_slice(b"trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n");
    buf.extend_from_slice(format!("{xref}\n%%EOF\n").as_bytes());
    buf
}

fn centre_pixel(pdf: Vec<u8>) -> [u8; 3] {
    let doc = PdfDocument::from_bytes(pdf).expect("parse");
    let mut opts = RenderOptions::default();
    opts.dpi = 72;
    opts.format = ImageFormat::RawRgba8;
    let img = render_page(&doc, 0, &opts).expect("render");
    let (w, h) = (img.width as usize, img.height as usize);
    let at = (h / 2 * w + w / 2) * 4;
    [img.data[at], img.data[at + 1], img.data[at + 2]]
}

/// Is this pixel the evaluated blue, rather than the `1 - tint` grey fallback
/// (which for tint 1.0 is black)?
fn is_evaluated_blue(px: [u8; 3]) -> bool {
    px[2] > 200 && px[0] < 80 && px[1] < 80
}

/// The baseline case: no `/Encode` or `/Decode` at all.
#[test]
fn sampled_function_without_encode_or_decode_is_evaluated() {
    let px = centre_pixel(sampled_separation_pdf(""));
    assert!(
        is_evaluated_blue(px),
        "the sampled tint transform should have been evaluated, got {px:?}"
    );
}

/// The defect: the same function, with its defaults written out.
#[test]
fn sampled_function_with_explicit_default_encode_is_evaluated() {
    let px = centre_pixel(sampled_separation_pdf("/Encode [0 1]"));
    assert!(
        is_evaluated_blue(px),
        "an explicit default /Encode [0 1] means what its absence means, got {px:?}"
    );
}

/// `/Decode` default is the value of `/Range`.
#[test]
fn sampled_function_with_explicit_default_decode_is_evaluated() {
    let px = centre_pixel(sampled_separation_pdf("/Decode [0 1 0 1 0 1]"));
    assert!(
        is_evaluated_blue(px),
        "an explicit default /Decode means what its absence means, got {px:?}"
    );
}

/// Both together — the shape actually measured in the wild.
#[test]
fn sampled_function_with_both_defaults_is_evaluated() {
    let px = centre_pixel(sampled_separation_pdf("/Encode [0 1] /Decode [0 1 0 1 0 1]"));
    assert!(
        is_evaluated_blue(px),
        "both defaults written out is still the default mapping, got {px:?}"
    );
}

/// The guard must still decline a genuinely non-default `/Encode`: this
/// evaluator implements only the default mapping, and evaluating anyway would
/// silently produce the wrong colour.
#[test]
fn sampled_function_with_non_default_encode_is_declined() {
    let px = centre_pixel(sampled_separation_pdf("/Encode [1 0]"));
    assert!(
        !is_evaluated_blue(px),
        "a reversed /Encode is not the default mapping and must not be evaluated, got {px:?}"
    );
}

/// Same for a non-default `/Decode`.
#[test]
fn sampled_function_with_non_default_decode_is_declined() {
    let px = centre_pixel(sampled_separation_pdf("/Decode [1 0 1 0 1 0]"));
    assert!(
        !is_evaluated_blue(px),
        "an inverted /Decode is not the default mapping and must not be evaluated, got {px:?}"
    );
}

/// A malformed array (wrong length) is not "default" either.
#[test]
fn sampled_function_with_malformed_encode_is_declined() {
    let px = centre_pixel(sampled_separation_pdf("/Encode [0]"));
    assert!(
        !is_evaluated_blue(px),
        "a malformed /Encode must not be evaluated, got {px:?}"
    );
}
