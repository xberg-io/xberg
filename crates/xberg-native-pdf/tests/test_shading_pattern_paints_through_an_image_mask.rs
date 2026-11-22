//! A shading pattern set as the current colour paints through an image mask.
//!
//! ISO 32000-1:2008 §8.7.4.1 (`docs/spec/pdf.md`:12899-12902) lists the
//! operators a shading pattern may be used with:
//!
//! > painting operators such as **f** (fill), **S** (stroke), **Tj** (show
//! > text), or **Do** (paint external object) **with an image mask**
//!
//! Only `f` was handled. The image-mask path painted the stencil in
//! `gs.fill_color_rgb`, which a *coloured* pattern never sets — it is selected
//! by `/P scn` with no operands — so the whole stencil came out in whatever
//! flat colour happened to be current. On the corpus file that surfaced this,
//! a page whose entire content is one CCITT stencil filled with an axial
//! gradient, we rendered a mean tone of 180.75 where the panel agrees on
//! 230.01–230.18.

use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::rendering::{RenderOptions, render_page};

/// A page that fills a 1-bit stencil with a shading pattern. The stencil is
/// 8x8 with its left half set, so half the page paints and half does not.
fn stencil_with_pattern() -> Vec<u8> {
    // MSB-first, one byte per row: 0xF0 = left four columns painted. Under the
    // default /Decode, sample 0 paints, so invert the bits we want painted.
    let stencil: Vec<u8> = vec![0x0F; 8];
    let content = b"/Cs1 cs /P1 scn 0 0 100 100 re W n \
                    q 100 0 0 100 0 0 cm /Im0 Do Q\n"
        .to_vec();

    let mut pdf = Vec::new();
    let mut off = [0usize; 9];
    macro_rules! push {
        ($s:expr) => {
            pdf.extend_from_slice($s.as_bytes())
        };
    }
    push!("%PDF-1.7\n");
    off[1] = pdf.len();
    push!("1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    off[2] = pdf.len();
    push!("2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    off[3] = pdf.len();
    push!(
        "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Contents 4 0 R /Resources << /ColorSpace << /Cs1 [/Pattern] >> \
         /Pattern << /P1 6 0 R >> /XObject << /Im0 5 0 R >> >> >>\nendobj\n"
    );
    off[4] = pdf.len();
    push!(format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()));
    pdf.extend_from_slice(&content);
    push!("endstream\nendobj\n");
    off[5] = pdf.len();
    push!(format!(
        "5 0 obj\n<< /Type /XObject /Subtype /Image /Width 8 /Height 8 \
         /ImageMask true /BitsPerComponent 1 /Length {} >>\nstream\n",
        stencil.len()
    ));
    pdf.extend_from_slice(&stencil);
    push!("\nendstream\nendobj\n");
    off[6] = pdf.len();
    push!(
        "6 0 obj\n<< /Type /Pattern /PatternType 2 /Matrix [1 0 0 1 0 0] \
         /Shading << /ShadingType 2 /ColorSpace /DeviceRGB /Coords [0 0 100 0] \
         /Extend [true true] /Function << /FunctionType 2 /Domain [0 1] /N 1 \
         /C0 [1 0 0] /C1 [1 0 0] >> >> >>\nendobj\n"
    );

    let xref = pdf.len();
    push!("xref\n0 7\n0000000000 65535 f \r\n");
    for offset in &off[1..=6] {
        push!(format!("{:010} 00000 n \r\n", offset));
    }
    push!(format!(
        "trailer\n<< /Size 7 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    ));
    pdf
}

#[test]
fn test_stencil_paints_the_pattern_not_a_flat_default() {
    let doc = PdfDocument::from_bytes(stencil_with_pattern()).expect("synthetic PDF parses");
    let img = render_page(&doc, 0, &RenderOptions::default()).expect("page renders");
    let px = image::load_from_memory(&img.data).expect("PNG decodes").to_rgba8();

    // The shading is constant red, so wherever the stencil paints, the pixel
    // must be red. Painting the stale flat fill colour gives black instead.
    let mut red = 0u32;
    let mut black = 0u32;
    for p in px.pixels() {
        if p[3] == 0 {
            continue;
        }
        let (r, g, b) = (u32::from(p[0]), u32::from(p[1]), u32::from(p[2]));
        if r > 200 && g < 80 && b < 80 {
            red += 1;
        } else if r < 60 && g < 60 && b < 60 {
            black += 1;
        }
    }
    assert!(
        red > 500,
        "the stencil painted no pattern colour (red={red}, black={black}) — a \
         shading pattern is not reaching the image-mask path"
    );
    assert!(
        black < red / 8,
        "the stencil painted mostly flat black (red={red}, black={black}), which \
         is gs.fill_color_rgb rather than the pattern"
    );
}
