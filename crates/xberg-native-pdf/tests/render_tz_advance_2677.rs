use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::rendering::{RenderOptions, render_page, render_separation};

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
    runs_for(&format!("BT /F1 {FONT_SIZE} Tf {tz} Tz 20 40 Td (I) Tj (I) Tj ET"))
}

fn runs_for(content: &str) -> Vec<(u32, u32)> {
    let document = PdfDocument::from_bytes(build_pdf(content)).expect("valid PDF");
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
    let span = |tz: u32| {
        let runs = runs_for(&format!("BT /F1 {FONT_SIZE} Tf {tz} Tz 10 40 Td (HHHH) Tj ET"));
        let first = runs.first().expect("ink");
        let last = runs.last().expect("ink");
        (last.0 + last.1 - first.0) as f32
    };
    let ratio = span(50) / span(100);
    assert!(
        (ratio - 0.5).abs() <= 0.05,
        "Tz 50 ink width must be half of Tz 100, ratio {ratio}"
    );
}

#[test]
fn should_scale_a_tj_number_offset_by_the_horizontal_scaling() {
    let runs = runs_for(&format!("BT /F1 {FONT_SIZE} Tf 50 Tz 20 40 Td [(I) -500 (I)] TJ ET"));
    assert_eq!(runs.len(), 2, "{runs:?}");
    let pitch = (runs[1].0 - runs[0].0) as f32;
    let expected = (HELVETICA_I_WIDTH + 0.5) * FONT_SIZE * 0.5 * PX_PER_PT;
    assert!(
        (pitch - expected).abs() <= 2.0,
        "TJ pitch must be {expected:.1} px, got {pitch}"
    );
}

#[test]
fn should_scale_character_spacing_by_the_horizontal_scaling() {
    let runs = runs_for(&format!("BT /F1 {FONT_SIZE} Tf 50 Tz 2 Tc 20 40 Td (I) Tj (I) Tj ET"));
    assert_eq!(runs.len(), 2, "{runs:?}");
    let pitch = (runs[1].0 - runs[0].0) as f32;
    let expected = (HELVETICA_I_WIDTH * FONT_SIZE + 2.0) * 0.5 * PX_PER_PT;
    assert!(
        (pitch - expected).abs() <= 2.0,
        "Tc pitch must be {expected:.1} px, got {pitch}"
    );
}

#[test]
fn should_scale_word_spacing_by_the_horizontal_scaling() {
    let runs = runs_for(&format!("BT /F1 {FONT_SIZE} Tf 50 Tz 10 Tw 20 40 Td (I I) Tj ET"));
    assert_eq!(runs.len(), 2, "{runs:?}");
    let pitch = (runs[1].0 - runs[0].0) as f32;
    let space_width = 0.278;
    let expected = ((2.0 * space_width) * FONT_SIZE + 10.0) * 0.5 * PX_PER_PT;
    assert!(
        (pitch - expected).abs() <= 2.0,
        "Tw pitch must be {expected:.1} px, got {pitch}"
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

#[test]
fn should_scale_a_tj_number_offset_by_the_horizontal_scaling_on_a_separation_plate() {
    let content = format!("0 0 0 1 k BT /F1 {FONT_SIZE} Tf 50 Tz 20 40 Td [(I) -500 (I)] TJ ET");
    let document = PdfDocument::from_bytes(build_pdf(&content)).expect("valid PDF");
    let plate = render_separation(&document, 0, "Black", DPI).expect("render plate");
    let ink = |x: u32| (0..plate.height).any(|y| plate.data[(y * plate.width + x) as usize] > 127);
    let mut starts = Vec::new();
    for x in 1..plate.width {
        if ink(x) && !ink(x - 1) {
            starts.push(x);
        }
    }
    assert_eq!(starts.len(), 2, "{starts:?}");
    let pitch = (starts[1] - starts[0]) as f32;
    let expected = (HELVETICA_I_WIDTH + 0.5) * FONT_SIZE * 0.5 * PX_PER_PT;
    assert!(
        (pitch - expected).abs() <= 2.0,
        "plate TJ pitch must be {expected:.1} px, got {pitch}"
    );
}
