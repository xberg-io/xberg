use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::rendering::{RenderOptions, render_page};

const DPI: u32 = 144;
const PX_PER_PT: f32 = DPI as f32 / 72.0;
const FONT_SIZE: f32 = 40.0;
const HELVETICA_I_WIDTH: f32 = 0.278;

fn build_pdf(content: &str) -> Vec<u8> {
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Contents 4 0 R \
         /Resources << /Font << /F1 5 0 R >> >> >>"
            .to_string(),
        format!("<< /Length {} >>\nstream\n{}\nendstream", content.len(), content),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string(),
    ];
    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::new();
    for (i, body) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n{}\nendobj\n", i + 1, body).as_bytes());
    }
    let xref = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes());
    for offset in offsets {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    pdf
}

/// Column runs `(start, width)` of dark pixels across the page, in order.
fn ink_runs(tz: u32) -> Vec<(u32, u32)> {
    let content = format!("BT /F1 {FONT_SIZE} Tf {tz} Tz 20 40 Td (I) Tj (I) Tj ET");
    let document = PdfDocument::from_bytes(build_pdf(&content)).expect("valid PDF");
    let rendered = render_page(&document, 0, &RenderOptions::with_dpi(DPI)).expect("render page");
    let image = image::load_from_memory(&rendered.data).unwrap().to_rgba8();
    let dark = |x: u32| (0..image.height()).any(|y| image.get_pixel(x, y)[0] < 128);
    let mut runs = Vec::new();
    let mut start = None;
    for x in 0..image.width() {
        match (dark(x), start) {
            (true, None) => start = Some(x),
            (false, Some(s)) => {
                runs.push((s, x - s));
                start = None;
            }
            _ => {}
        }
    }
    runs
}

#[test]
fn should_advance_the_pen_by_the_horizontally_scaled_width_between_two_strings() {
    let runs = ink_runs(50);
    assert_eq!(runs.len(), 2, "expected two separate glyphs, got {runs:?}");
    let pitch = (runs[1].0 - runs[0].0) as f32;
    let expected = HELVETICA_I_WIDTH * FONT_SIZE * 0.5 * PX_PER_PT;
    assert!(
        (pitch - expected).abs() <= 2.0,
        "Tz 50 pitch must be {expected:.1} px, got {pitch}"
    );
}

#[test]
fn should_stretch_the_glyph_outline_by_the_horizontal_scaling() {
    let full = ink_runs(100);
    let half = ink_runs(50);
    assert_eq!(full.len(), 2, "{full:?}");
    assert_eq!(half.len(), 2, "{half:?}");
    let ratio = half[0].1 as f32 / full[0].1 as f32;
    assert!(
        (ratio - 0.5).abs() <= 0.15,
        "Tz 50 stem width must be half of Tz 100, ratio {ratio} ({half:?} vs {full:?})"
    );
}

#[test]
fn should_leave_the_unscaled_pitch_unchanged_at_tz_100() {
    let runs = ink_runs(100);
    assert_eq!(runs.len(), 2, "{runs:?}");
    let pitch = (runs[1].0 - runs[0].0) as f32;
    let expected = HELVETICA_I_WIDTH * FONT_SIZE * PX_PER_PT;
    assert!((pitch - expected).abs() <= 2.0, "pitch {pitch}, expected {expected:.1}");
}
