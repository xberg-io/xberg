//! GH#2323: two side-by-side panels whose baselines step by one degree come out
//! with the panels' lines interleaved, run by run, instead of panel by panel.
//!
//! Each panel line is drawn as several runs. A tilt moves every run a little
//! higher than the run before it, so the XY-cut's line grouping breaks one panel
//! line into pieces about a point apart. Pieces of the two panels then pair up
//! as short label/value rows, the column cut is declined, and the page reads
//! row by row across both panels. The untilted copy of the same page is the
//! control: it reads panel by panel at the base already.
#![cfg(feature = "pdf")]

mod helpers;
use helpers::extract_bytes_document_blocking;
use xberg::core::config::{ExtractionConfig, OutputFormat};

const WORDS: [&str; 32] = [
    "harbor", "meadow", "lantern", "orchard", "compass", "thimble", "granite", "willow", "ferry", "quarry",
    "pasture", "beacon", "cobble", "saddle", "marble", "timber", "anchor", "bridge", "cellar", "dune", "embers",
    "fjord", "glacier", "hollow", "island", "juniper", "kettle", "lagoon", "mast", "nectar", "oasis", "pebble",
];
/// Characters per run, cycled along each line.
const RUNS: [usize; 8] = [14, 2, 9, 1, 12, 3, 10, 2];
const PANEL_X: [f32; 2] = [72.0, 320.0];
const LINES_PER_PANEL: usize = 4;
const WORDS_PER_LINE: usize = 6;
const FONT_SIZE: f32 = 8.0;
const LEADING: f32 = 9.0;
const TOP_Y: f32 = 700.0;

/// Advance of Helvetica text, close enough to place the runs end to end.
fn advance(text: &str, size: f32) -> f32 {
    text.chars()
        .map(|c| match c {
            'i' | 'j' | 'l' => 0.22,
            ' ' | 'f' | 't' => 0.28,
            _ => 0.56,
        })
        .sum::<f32>()
        * size
}

fn text_at(x: f32, y: f32, size: f32, text: &str) -> String {
    format!("BT /F1 {size} Tf 1 0 0 1 {x:.2} {y:.2} Tm ({text}) Tj ET\n")
}

/// The panel lines, panel 1 first, as the reader expects them.
fn expected_lines() -> Vec<String> {
    (0..PANEL_X.len() * LINES_PER_PANEL)
        .map(|k| {
            (0..WORDS_PER_LINE)
                .map(|j| WORDS[(k * 5 + j) % WORDS.len()])
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect()
}

/// One page, two panels, every line drawn as runs whose baselines rise with `tilt_degrees`.
fn panel_page(tilt_degrees: f32) -> String {
    let tan = tilt_degrees.to_radians().tan();
    let lines = expected_lines();
    let mut ops = String::new();
    for (panel, &panel_x) in PANEL_X.iter().enumerate() {
        for line in 0..LINES_PER_PANEL {
            let text = &lines[panel * LINES_PER_PANEL + line];
            let mut start = 0usize;
            let mut run = 0usize;
            while start < text.len() {
                let end = (start + RUNS[run % RUNS.len()]).min(text.len());
                let x = advance(&text[..start], FONT_SIZE);
                let y = TOP_Y - LEADING * line as f32 + x * tan;
                ops.push_str(&text_at(panel_x + x, y, FONT_SIZE, &text[start..end]));
                start = end;
                run += 1;
            }
        }
    }
    ops
}

/// One US-letter page, Helvetica, no embedded font.
fn build_pdf(page: &str) -> Vec<u8> {
    let objects: Vec<Vec<u8>> = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 4 0 R >> >> \
          /Contents 5 0 R >>"
            .to_vec(),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec(),
        [
            format!("<< /Length {} >>\nstream\n", page.len()).into_bytes(),
            page.as_bytes().to_vec(),
            b"\nendstream".to_vec(),
        ]
        .concat(),
    ];
    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
        pdf.extend_from_slice(object);
        pdf.extend_from_slice(b"\nendobj\n");
    }
    let xref = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes());
    for offset in &offsets {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n", objects.len() + 1).as_bytes(),
    );
    pdf
}

/// Plain text of the page through the public extraction path, whitespace squeezed to single spaces.
fn extract_plain(tilt_degrees: f32) -> String {
    let config = ExtractionConfig {
        output_format: OutputFormat::Plain,
        use_cache: false,
        ..Default::default()
    };
    let result = extract_bytes_document_blocking(&build_pdf(&panel_page(tilt_degrees)), "application/pdf", &config)
        .expect("fixture must extract");
    result.content.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Every panel line is contiguous in the text, and the lines come panel by panel.
fn assert_panel_major(label: &str, text: &str) {
    let mut cursor = 0usize;
    for line in expected_lines() {
        let found = text[cursor..].find(&line).map(|at| cursor + at);
        assert!(
            found.is_some(),
            "{label}: the panel line {line:?} is not whole, or is out of panel order, in {text:?}"
        );
        cursor = found.expect("checked above") + line.len();
    }
}

#[test]
fn tilted_panels_read_panel_by_panel_with_whole_lines() {
    assert_panel_major("tilt 1", &extract_plain(1.0));
}

#[test]
fn flat_panels_read_panel_by_panel_with_whole_lines() {
    assert_panel_major("tilt 0", &extract_plain(0.0));
}
