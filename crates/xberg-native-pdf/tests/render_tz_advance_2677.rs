use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::rendering::{RenderOptions, render_page, render_separation};

const DPI: u32 = 144;
const PX_PER_PT: f32 = DPI as f32 / 72.0;
const FONT_SIZE: f32 = 40.0;
const HELVETICA_I_WIDTH: f32 = 0.278;

fn build_pdf(content: &str) -> Vec<u8> {
    build_pdf_with(content, &[], "5 0 R")
}

/// Objects 6.. are `extra_objects`; `font_resource` is what `/F1` points at.
fn build_pdf_with(content: &str, extra_objects: &[&str], font_resource: &str) -> Vec<u8> {
    let mut objects = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Contents 4 0 R \
             /Resources << /Font << /F1 {font_resource} >> >> >>"
        ),
        format!("<< /Length {} >>\nstream\n{}\nendstream", content.len(), content),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string(),
    ];
    objects.extend(extra_objects.iter().map(|object| (*object).to_string()));
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
    runs_for_pdf(build_pdf(content))
}

fn runs_for_pdf(pdf: Vec<u8>) -> Vec<(u32, u32)> {
    let document = PdfDocument::from_bytes(pdf).expect("valid PDF");
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

/// Type 3 font whose single glyph `A` is an 8 pt wide bar with a 40 pt advance at 40 pt, so
/// two glyphs stay separate runs under Tz 50 and the pitch is readable.
fn type3_runs(content: &str) -> Vec<(u32, u32)> {
    let font = "<< /Type /Font /Subtype /Type3 /FontBBox [0 0 1000 1000] /FontMatrix [0.001 0 0 0.001 0 0] \
                /CharProcs << /bar 7 0 R >> /Encoding << /Type /Encoding /Differences [65 /bar] >> \
                /FirstChar 65 /LastChar 65 /Widths [1000] >>";
    let char_proc = "<< /Length 38 >>\nstream\n1000 0 0 0 200 700 d1 0 0 200 700 re f\nendstream";
    runs_for_pdf(build_pdf_with(content, &[font, char_proc], "6 0 R"))
}

#[test]
fn should_place_type3_glyphs_at_the_horizontally_scaled_pitch() {
    let runs = type3_runs(&format!("BT /F1 {FONT_SIZE} Tf 50 Tz 20 40 Td (AA) Tj ET"));
    assert_eq!(runs.len(), 2, "{runs:?}");
    let pitch = (runs[1].0 - runs[0].0) as f32;
    let expected = FONT_SIZE * 0.5 * PX_PER_PT;
    assert!(
        (pitch - expected).abs() <= 2.0,
        "Type 3 placement pitch must be {expected:.1} px, got {pitch}"
    );
}

#[test]
fn should_advance_past_a_type3_string_by_the_horizontally_scaled_width() {
    let runs = type3_runs(&format!("BT /F1 {FONT_SIZE} Tf 50 Tz 20 40 Td (A) Tj (A) Tj ET"));
    assert_eq!(runs.len(), 2, "{runs:?}");
    let pitch = (runs[1].0 - runs[0].0) as f32;
    let expected = FONT_SIZE * 0.5 * PX_PER_PT;
    assert!(
        (pitch - expected).abs() <= 2.0,
        "Type 3 string advance must be {expected:.1} px, got {pitch}"
    );
}

fn plate_run_starts(content: &str) -> Vec<u32> {
    let document = PdfDocument::from_bytes(build_pdf(content)).expect("valid PDF");
    let plate = render_separation(&document, 0, "Black", DPI).expect("render plate");
    let ink = |x: u32| (0..plate.height).any(|y| plate.data[(y * plate.width + x) as usize] > 127);
    (1..plate.width).filter(|&x| ink(x) && !ink(x - 1)).collect()
}

fn visible_i_start_after(invisible: &str) -> f32 {
    let control = plate_run_starts(&format!("0 0 0 1 k BT /F1 {FONT_SIZE} Tf 50 Tz 20 40 Td (I) Tj ET"));
    let starts = plate_run_starts(&format!(
        "0 0 0 1 k BT /F1 {FONT_SIZE} Tf 50 Tz 20 40 Td 3 Tr {invisible} Tj 0 Tr (I) Tj ET"
    ));
    assert_eq!((control.len(), starts.len()), (1, 1), "{control:?} {starts:?}");
    starts[0] as f32 - control[0] as f32
}

#[test]
fn should_advance_invisible_text_on_a_plate_by_the_horizontally_scaled_width() {
    let shift = visible_i_start_after("(I)");
    let expected = HELVETICA_I_WIDTH * FONT_SIZE * 0.5 * PX_PER_PT;
    assert!(
        (shift - expected).abs() <= 2.0,
        "invisible text must advance {expected:.1} px, got {shift}"
    );
}

#[test]
fn should_advance_invisible_text_on_a_plate_by_word_spacing_scaled_by_tz() {
    let content_tw = |tw: u32| {
        let control = plate_run_starts(&format!("0 0 0 1 k BT /F1 {FONT_SIZE} Tf 50 Tz 20 40 Td (I) Tj ET"));
        let starts = plate_run_starts(&format!(
            "0 0 0 1 k BT /F1 {FONT_SIZE} Tf 50 Tz {tw} Tw 20 40 Td 3 Tr (I I) Tj 0 Tr (I) Tj ET"
        ));
        assert_eq!((control.len(), starts.len()), (1, 1), "{control:?} {starts:?}");
        starts[0] as f32 - control[0] as f32
    };
    let shift = content_tw(10);
    let expected = (3.0 * HELVETICA_I_WIDTH * FONT_SIZE + 10.0) * 0.5 * PX_PER_PT;
    assert!(
        (shift - expected).abs() <= 2.0,
        "invisible text with Tw 10 must advance {expected:.1} px, got {shift}"
    );
}

/// `(HH) Tj (I) Tj` at x = 100 pt, so the origin sits at x = 200 px.
fn zero_probe_content(tz: &str) -> String {
    format!("BT /F1 {FONT_SIZE} Tf {tz} Tz 100 40 Td (HH) Tj (I) Tj ET")
}

/// `(ink pixels, first ink column, last ink column)` on the page.
fn page_ink(content: &str) -> (u32, Option<u32>, Option<u32>) {
    let document = PdfDocument::from_bytes(build_pdf(content)).expect("valid PDF");
    let rendered = render_page(&document, 0, &RenderOptions::with_dpi(DPI)).expect("render page");
    let image = image::load_from_memory(&rendered.data).unwrap().to_rgba8();
    let (mut count, mut first, mut last) = (0, None, None);
    for (x, _, pixel) in image.enumerate_pixels() {
        if pixel[0] < 128 {
            count += 1;
            first.get_or_insert(x);
            last = Some(x);
        }
    }
    (count, first, last)
}

fn plate_ink_pixels(content: &str) -> u32 {
    let document = PdfDocument::from_bytes(build_pdf(content)).expect("valid PDF");
    let plate = render_separation(&document, 0, "Black", DPI).expect("render plate");
    plate.data.iter().filter(|&&v| v > 127).count() as u32
}

fn plate_content(tz: &str) -> String {
    format!("0 0 0 1 k {}", zero_probe_content(tz))
}

#[test]
fn should_draw_nothing_and_not_fail_at_zero_and_vanishing_tz() {
    for tz in ["0", "0.0001"] {
        assert_eq!(page_ink(&zero_probe_content(tz)).0, 0, "page Tz {tz}");
        assert_eq!(plate_ink_pixels(&plate_content(tz)), 0, "plate Tz {tz}");
    }
}

#[test]
fn should_mirror_text_to_the_left_of_the_origin_under_negative_tz() {
    let origin_px = (100.0 * PX_PER_PT) as u32;
    for (negative, positive) in [("-50", "50"), ("-100", "100")] {
        let (neg_ink, _, neg_last) = page_ink(&zero_probe_content(negative));
        let (pos_ink, pos_first, _) = page_ink(&zero_probe_content(positive));
        assert!(pos_first.expect("positive Tz draws") >= origin_px - 2, "Tz {positive}");
        let neg_last = neg_last.expect("negative Tz draws");
        assert!(
            neg_last <= origin_px + 2,
            "Tz {negative} must stay left of {origin_px}, got {neg_last}"
        );
        let tolerance = pos_ink / 50;
        assert!(
            neg_ink.abs_diff(pos_ink) <= tolerance,
            "Tz {negative} ink {neg_ink} must match Tz {positive} ink {pos_ink}"
        );
        let plate_neg = plate_ink_pixels(&plate_content(negative));
        assert!(
            plate_neg.abs_diff(plate_ink_pixels(&plate_content(positive))) <= tolerance,
            "plate Tz {negative} ink {plate_neg}"
        );
    }
}
