//! The `sh` operator evaluates its shading's `/Function` across `/Domain` instead of reading `/C0` and
//! `/C1` off the dictionary (ISO 32000-1:2008 §7.10, §8.7.4.5.3).

use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::rendering::{RenderOptions, render_page};

fn build_pdf(objects: &[Vec<u8>]) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    out.extend_from_slice(b"%PDF-1.4\n");
    let mut offsets = Vec::new();
    for (i, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref_pos = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes());
    for off in &offsets {
        out.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            objects.len() + 1,
            xref_pos
        )
        .as_bytes(),
    );
    out
}

fn stream_obj(dict: &str, data: &[u8]) -> Vec<u8> {
    let mut v = format!("<< {} /Length {} >>\nstream\n", dict, data.len()).into_bytes();
    v.extend_from_slice(data);
    v.extend_from_slice(b"\nendstream");
    v
}

const RED_THEN_BLUE: &[u8] = &[0xFF, 0x00, 0x00, 0x00, 0x00, 0xFF];
const WHITE_BLACK_WHITE: &[u8] = &[0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF];

/// A 200x100 page painting an axial shading with `sh`. Object 6 is a sampled function over `samples`;
/// `function_ref` names the one the shading uses; `extra` supplies additional objects from number 7.
fn shading_page(function_ref: &str, extra: &[&str], samples: &[u8]) -> Vec<u8> {
    let mut objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Contents 4 0 R \
          /Resources << /Shading << /Sh0 5 0 R >> >> >>"
            .to_vec(),
        stream_obj("", b"/Sh0 sh"),
        format!(
            "<< /ShadingType 2 /ColorSpace /DeviceRGB /Coords [0 0 200 0] /Extend [true true] \
             /Function {function_ref} >>"
        )
        .into_bytes(),
        stream_obj(
            &format!(
                "/FunctionType 0 /Domain [0 1] /Range [0 1 0 1 0 1] /Size [{}] /BitsPerSample 8",
                samples.len() / 3
            ),
            samples,
        ),
    ];
    objects.extend(extra.iter().map(|body| body.as_bytes().to_vec()));
    build_pdf(&objects)
}

/// Pixel on the page's middle row: `Ok(x)` counts from the left edge, `Err(x)` from the right.
fn pixel(pdf: Vec<u8>, x: Result<u32, u32>) -> [u8; 3] {
    let doc = PdfDocument::from_bytes(pdf).expect("synthetic PDF parses");
    let img = render_page(&doc, 0, &RenderOptions::default()).expect("page renders");
    let px = image::load_from_memory(&img.data).expect("PNG decodes").to_rgba8();
    let column = match x {
        Ok(x) => x,
        Err(from_right) => px.width() - 1 - from_right,
    };
    let p = px.get_pixel(column, px.height() / 2);
    [p[0], p[1], p[2]]
}

#[test]
fn sampled_function_supplies_the_gradient_colours() {
    let left = pixel(shading_page("6 0 R", &[], RED_THEN_BLUE), Ok(3));
    let right = pixel(shading_page("6 0 R", &[], RED_THEN_BLUE), Err(3));
    assert!(left[0] > 180 && left[2] < 80, "left end should be red, got {left:?}");
    assert!(
        right[2] > 180 && right[0] < 80,
        "right end should be blue, got {right:?}"
    );
}

#[test]
fn stitching_encode_reverses_the_sub_domain() {
    let stitch = "<< /FunctionType 3 /Domain [0 1] /Functions [6 0 R] /Bounds [] /Encode [1 0] >>";
    let left = pixel(shading_page("7 0 R", &[stitch], RED_THEN_BLUE), Ok(3));
    let right = pixel(shading_page("7 0 R", &[stitch], RED_THEN_BLUE), Err(3));
    assert!(left[2] > 180 && left[0] < 80, "left end should be blue, got {left:?}");
    assert!(
        right[0] > 180 && right[2] < 80,
        "right end should be red, got {right:?}"
    );
}

#[test]
fn exponential_function_still_supplies_its_endpoints() {
    let exp = "<< /FunctionType 2 /Domain [0 1] /N 1 /C0 [1 0 0] /C1 [0 0 1] >>";
    let left = pixel(shading_page("7 0 R", &[exp], RED_THEN_BLUE), Ok(3));
    let right = pixel(shading_page("7 0 R", &[exp], RED_THEN_BLUE), Err(3));
    assert!(left[0] > 180 && left[2] < 80, "left should be red, got {left:?}");
    assert!(right[2] > 180 && right[0] < 80, "right should be blue, got {right:?}");
}

#[test]
fn ramp_that_returns_to_its_starting_colour_still_paints_its_middle() {
    let pdf = || shading_page("6 0 R", &[], WHITE_BLACK_WHITE);
    let (left, right) = (pixel(pdf(), Ok(3)), pixel(pdf(), Err(3)));
    assert!(
        left[0] > 180 && right[0] > 180,
        "both ends are white, got {left:?} and {right:?}"
    );
    let mid = pixel(pdf(), Ok(RenderOptions::default().dpi * 200 / 72 / 2));
    assert!(
        mid.iter().all(|c| *c < 100),
        "the middle of the ramp is black, got {mid:?}"
    );
}
