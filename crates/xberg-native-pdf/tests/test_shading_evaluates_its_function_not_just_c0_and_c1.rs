//! A shading's gradient comes from evaluating its `/Function`, not from
//! reading `/C0` and `/C1` off the dictionary.
//!
//! ISO 32000-1:2008 §7.10 gives a function four possible types. Only type 2
//! (exponential) carries `/C0` and `/C1`. A **type 3 stitching function over
//! type 0 sampled sub-functions** — what most authoring tools export a
//! gradient as — carries neither, so endpoint resolution returned nothing and
//! the axial painter fell back to its black-to-white safety net: a grey ramp
//! where the file asks for colour.
//!
//! §7.10.4 also makes the stitching `/Encode` array part of the answer: it
//! remaps each sub-domain, and an `/Encode [1 0]` reverses one. Taking "the
//! first sub-function's `/C0` and the last one's `/C1`" cannot see that.

use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::rendering::{RenderOptions, render_page};

/// A 200x100 page filled with an axial shading pattern whose `/Function` is
/// the given object body, plus any extra objects it needs.
///
/// The sampled function is two 8-bit RGB samples: pure red then pure blue.
/// Neither endpoint is grey, so a fallback of any kind is visible immediately.
fn shading_page(function_ref: &str, extra: &[(usize, &str)], samples: &[u8]) -> Vec<u8> {
    let content = b"/Pattern cs /P0 scn 0 0 200 100 re f\n".to_vec();
    let mut pdf: Vec<u8> = Vec::new();
    let last = 9usize;
    let mut off = vec![0usize; last + 1];
    let obj = |pdf: &mut Vec<u8>, off: &mut Vec<usize>, id: usize, body: &str| {
        off[id] = pdf.len();
        pdf.extend_from_slice(format!("{id} 0 obj\n{body}\nendobj\n").as_bytes());
    };

    pdf.extend_from_slice(b"%PDF-1.7\n");
    obj(&mut pdf, &mut off, 1, "<< /Type /Catalog /Pages 2 0 R >>");
    obj(&mut pdf, &mut off, 2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>");
    obj(
        &mut pdf,
        &mut off,
        3,
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Contents 4 0 R \
         /Resources << /Pattern << /P0 5 0 R >> >> >>",
    );
    off[4] = pdf.len();
    pdf.extend_from_slice(format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()).as_bytes());
    pdf.extend_from_slice(&content);
    pdf.extend_from_slice(b"endstream\nendobj\n");
    obj(
        &mut pdf,
        &mut off,
        5,
        &format!(
            "<< /Type /Pattern /PatternType 2 /Matrix [1 0 0 1 0 0] \
             /Shading << /ShadingType 2 /ColorSpace /DeviceRGB /Coords [0 0 200 0] \
             /Extend [true true] /Function {function_ref} >> >>"
        ),
    );
    // Object 6: the sampled function stream, red -> blue.
    off[6] = pdf.len();
    pdf.extend_from_slice(
        format!(
            "6 0 obj\n<< /FunctionType 0 /Domain [0 1] /Range [0 1 0 1 0 1] /Size [{}] \
             /BitsPerSample 8 /Length {} >>\nstream\n",
            samples.len() / 3,
            samples.len()
        )
        .as_bytes(),
    );
    pdf.extend_from_slice(samples);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");
    for (id, body) in extra {
        obj(&mut pdf, &mut off, *id, body);
    }

    let xref = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \r\n", last + 1).as_bytes());
    for offset in &off[1..=last] {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \r\n").as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            last + 1
        )
        .as_bytes(),
    );
    pdf
}

const RED_THEN_BLUE: &[u8] = &[0xFF, 0x00, 0x00, 0x00, 0x00, 0xFF];

/// White, black, white. A ramp whose two ends agree and whose middle does
/// not — the shape that two endpoint colours cannot describe.
const WHITE_BLACK_WHITE: &[u8] = &[0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF];

fn ends(pdf: Vec<u8>) -> ([u8; 3], [u8; 3]) {
    let doc = PdfDocument::from_bytes(pdf).expect("synthetic PDF parses");
    let img = render_page(&doc, 0, &RenderOptions::default()).expect("page renders");
    let px = image::load_from_memory(&img.data).expect("PNG decodes").to_rgba8();
    let y = px.height() / 2;
    let l = px.get_pixel(3, y);
    let r = px.get_pixel(px.width() - 4, y);
    ([l[0], l[1], l[2]], [r[0], r[1], r[2]])
}

/// A sampled function used directly. Before, this painted black-to-white.
#[test]
fn test_sampled_function_supplies_the_gradient_colours() {
    let (left, right) = ends(shading_page("6 0 R", &[], RED_THEN_BLUE));
    assert!(
        left[0] > 180 && left[2] < 80,
        "left end should be red, got {left:?} — the shading fell back to its \
         black-to-white default instead of evaluating /Function"
    );
    assert!(
        right[2] > 180 && right[0] < 80,
        "right end should be blue, got {right:?}"
    );
}

/// A stitching function wrapping that sampled one, with `/Encode [1 0]`, which
/// §7.10.4 says reverses the sub-domain. So the ends swap.
#[test]
fn test_stitching_encode_reverses_the_sub_domain() {
    let stitch = "<< /FunctionType 3 /Domain [0 1] /Functions [6 0 R] \
                  /Bounds [] /Encode [1 0] >>";
    let (left, right) = ends(shading_page("7 0 R", &[(7, stitch)], RED_THEN_BLUE));
    assert!(
        left[2] > 180 && left[0] < 80,
        "/Encode [1 0] reverses the sub-domain, so the left end should be blue, \
         got {left:?}"
    );
    assert!(
        right[0] > 180 && right[2] < 80,
        "right end should be red, got {right:?}"
    );
}

/// The type 2 case that always worked must keep working.
#[test]
fn test_exponential_function_still_supplies_its_endpoints() {
    let exp = "<< /FunctionType 2 /Domain [0 1] /N 1 /C0 [1 0 0] /C1 [0 0 1] >>";
    let (left, right) = ends(shading_page("8 0 R", &[(8, exp)], RED_THEN_BLUE));
    assert!(left[0] > 180 && left[2] < 80, "left should be red, got {left:?}");
    assert!(right[2] > 180 && right[0] < 80, "right should be blue, got {right:?}");
}

/// The colour at the middle of the painted band.
fn middle(pdf: Vec<u8>) -> [u8; 3] {
    let doc = PdfDocument::from_bytes(pdf).expect("synthetic PDF parses");
    let img = render_page(&doc, 0, &RenderOptions::default()).expect("page renders");
    let px = image::load_from_memory(&img.data).expect("PNG decodes").to_rgba8();
    let p = px.get_pixel(px.width() / 2, px.height() / 2);
    [p[0], p[1], p[2]]
}

/// A gradient is only its two endpoints when the function between them is
/// monotonic. A sampled function need not be: one axial shading in the corpus
/// ramps white to near-black and back to white across 5120 samples, so reading
/// its two ends gives white at both and the gradient paints a flat white page
/// — the entire ramp discarded, and the page rendered blank where poppler and
/// PyMuPDF both paint a gradient.
///
/// ISO 32000-1:2008 §8.7.4.5.3 defines the colour at parametric distance t by
/// the shading's function, so the middle of the band is the function's value
/// there and not a blend of its ends.
#[test]
fn test_ramp_that_returns_to_its_starting_colour_still_paints_its_middle() {
    let ends_of = ends(shading_page("6 0 R", &[], WHITE_BLACK_WHITE));
    assert!(
        ends_of.0[0] > 180 && ends_of.1[0] > 180,
        "both ends of this ramp are white by construction, got {:?} and {:?}",
        ends_of.0,
        ends_of.1
    );
    let mid = middle(shading_page("6 0 R", &[], WHITE_BLACK_WHITE));
    assert!(
        mid[0] < 100 && mid[1] < 100 && mid[2] < 100,
        "the middle of the ramp is black in the function and must be painted \
         dark; got {mid:?}. Sampling only the two ends makes this gradient \
         white-to-white and the band vanishes"
    );
}
