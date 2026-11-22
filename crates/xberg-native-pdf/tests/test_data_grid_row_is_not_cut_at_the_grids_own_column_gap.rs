//! A printed grid row is one reading unit, whatever gaps sit between its cells.
//!
//! ISO 32000-1:2008 §14.8.4.3.4 (Table 337) makes `TR` "a row of headings or
//! data in a table", holding the `TD` cells that belong to it — the row, not the
//! column, is what a reader follows across a grid. An untagged page carries no
//! such markup, so the structure has to be reconstructed from geometry, and the
//! reconstruction must not hand back half a row.
//!
//! The page below is an appendix results table: a long label column, then eight
//! numeric columns typeset as two visual groups with a wider gap between the
//! groups than between the columns inside them. That wider gap is a genuine
//! empty vertical corridor running the height of the grid — exactly the shape a
//! central page gutter has. Read as a gutter, the page is emitted column-major:
//! every row's left-hand cells first, then, a whole column-run later, its
//! right-hand cells as anonymous numbers with no label attached.
//!
//! The counter-cases pin the opposite failure: a real two-column body must still
//! be read column-major, with and without a table on the page, so a fix cannot
//! buy the grid back by refusing to see columns anywhere.

use xberg_native_pdf::PdfDocument;

const LEFT: f32 = 72.0;
/// Three numeric columns, a wide gap, then five more. Every gap inside a group
/// is 12.5pt; the gap between the groups is 32.5pt. Both are empty corridors —
/// the wider one is the corridor the sweep picks.
const COLS: [f32; 8] = [225.0, 265.0, 305.0, 365.0, 405.0, 445.0, 485.0, 525.0];
const HEADS: [&str; 8] = [
    "PSNRup", "SSIMup", "LPIPSdn", "GSCup", "GPQup", "GOup", "Latency", "Speedup",
];
const ROWS: usize = 12;
const TOP: f32 = 690.0;
/// Leading wide enough that no two rows' spans are vertical neighbours, so the
/// page decomposes into per-cell fragments and the block-topological reader
/// declines it. That is the state the reported page is in; without it a
/// different reader handles the page and this fixture tests nothing.
const LEAD: f32 = 24.0;

fn label(r: usize) -> String {
    format!("Baseline configuration number {:02}", r + 1)
}

fn value(r: usize, c: usize) -> String {
    format!("{}{}.{:03}", c + 1, r + 1, r * 7 + c)
}

fn show(c: &mut String, x: f32, y: f32, t: &str) {
    c.push_str(&format!("BT /F1 9 Tf {x} {y} Td ({t}) Tj ET\n"));
}

fn hline(c: &mut String, x0: f32, x1: f32, y: f32) {
    c.push_str(&format!("{x0} {y} m {x1} {y} l 0.6 w S\n"));
}

fn vline(c: &mut String, x: f32, y0: f32, y1: f32) {
    c.push_str(&format!("{x} {y0} m {x} {y1} l 0.6 w S\n"));
}

/// Wrap a content stream in the smallest PDF that carries it.
fn pdf_with(content: String) -> Vec<u8> {
    let content = content.into_bytes();
    let objects: Vec<Vec<u8>> = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R \
           /Resources << /Font << /F1 5 0 R >> >> >>"
            .to_vec(),
        [
            format!("<< /Length {} >>\nstream\n", content.len()).into_bytes(),
            content,
            b"\nendstream".to_vec(),
        ]
        .concat(),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec(),
    ];

    let mut pdf = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (i, body) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        pdf.extend_from_slice(body);
        pdf.extend_from_slice(b"\nendobj\n");
    }
    let xref = pdf.len();
    let n = objects.len() + 1;
    pdf.extend_from_slice(format!("xref\n0 {n}\n0000000000 65535 f \n").as_bytes());
    for off in &offsets {
        pdf.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(format!("trailer\n<< /Size {n} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes());
    pdf
}

/// A small ruled key/value table. This is the only grid on the page the
/// line-driven table detector recognises — the results grid above it is
/// typeset without rules, which is why its cells stay in the flow.
fn ruled_settings_table(c: &mut String, base: f32) {
    for (i, (k, v)) in [
        ("Sampler steps", "fifty"),
        ("Guidance scale", "seven"),
        ("Resolution", "1024"),
        ("Precision", "bf16"),
    ]
    .iter()
    .enumerate()
    {
        let y = base - 16.0 * i as f32;
        show(c, 96.0, y, k);
        show(c, 210.0, y, v);
        hline(c, 90.0, 260.0, y + 12.0);
    }
    hline(c, 90.0, 260.0, base - 52.0);
    for x in [90.0, 200.0, 260.0] {
        vline(c, x, base + 12.0, base - 52.0);
    }
}

fn appendix_grid_page() -> Vec<u8> {
    let mut c = String::new();
    for (i, line) in [
        "This appendix reports the per-task quantitative results for every baseline that we",
        "evaluated, together with the efficiency figures measured on the same hardware, so",
        "that the two families of numbers can be compared directly against one another all.",
    ]
    .iter()
    .enumerate()
    {
        show(&mut c, LEFT, 760.0 - 14.0 * i as f32, line);
    }
    show(&mut c, LEFT, TOP + 16.0, "Model");
    for (i, h) in HEADS.iter().enumerate() {
        show(&mut c, COLS[i], TOP + 16.0, h);
    }
    for r in 0..ROWS {
        let y = TOP - LEAD * r as f32;
        show(&mut c, LEFT, y, &label(r));
        for (i, &x) in COLS.iter().enumerate() {
            show(&mut c, x, y, &value(r, i));
        }
    }
    ruled_settings_table(&mut c, TOP - LEAD * ROWS as f32 - 30.0);
    pdf_with(c)
}

/// Every row of the grid must survive as one line: the label and the row's
/// last-column value are printed on one baseline, so they belong to one line of
/// output.
#[test]
fn every_grid_row_keeps_its_label_and_its_last_column_on_one_line() {
    let doc = PdfDocument::from_bytes(appendix_grid_page()).expect("open");
    let out = doc.extract_text(0).expect("text");
    for r in 0..ROWS {
        let (l, v) = (label(r), value(r, 7));
        let line = out
            .lines()
            .find(|line| line.contains(&l))
            .unwrap_or_else(|| panic!("`{l}` missing from output:\n{out}"));
        assert!(
            line.contains(&v),
            "the grid row was cut at its own column gap: `{l}` reads as `{line}`, \
             and its last cell `{v}` is emitted somewhere else. A printed row is \
             one row (ISO 32000-1 §14.8.4.3.4, Table 337: TR holds its TD cells). \
             Got:\n{out}"
        );
    }
}

/// The fixture only tests what it claims to if the page really does carry a
/// detected table and a corridor between the two column groups.
#[test]
fn test_fixture_is_a_grid_with_a_corridor_and_a_detected_table() {
    let doc = PdfDocument::from_bytes(appendix_grid_page()).expect("open");
    let tables = doc.extract_tables(0).expect("tables");
    assert!(
        !tables.is_empty(),
        "the page must carry a table the detector recognises, else the ordering \
         path under test is never reached"
    );
    let spans = doc.extract_spans(0).expect("spans");
    // Nothing is drawn between the third and fourth numeric column: the gap is
    // as empty as a page gutter, which is the whole difficulty. Measure it from
    // the grid's own spans rather than asserting the nominal geometry, so a
    // change in font metrics fails loudly here instead of quietly turning this
    // page into a different test.
    let grid_bottom = TOP - LEAD * (ROWS as f32 - 1.0);
    let grid: Vec<&xberg_native_pdf::layout::TextSpan> = spans
        .iter()
        .filter(|s| s.bbox.y >= grid_bottom - 1.0 && s.bbox.y <= TOP + 20.0)
        .collect();
    let hi = COLS[3];
    let lo = grid
        .iter()
        .map(|s| s.bbox.x + s.bbox.width)
        .filter(|r| *r <= hi + 0.5)
        .fold(f32::MIN, f32::max);
    assert!(
        hi - lo > 20.0,
        "the corridor between the two column groups measures {}pt; it has to be \
         the widest gap on the row for the corridor sweep to land in it",
        hi - lo
    );
    for s in &grid {
        let (x0, x1) = (s.bbox.x, s.bbox.x + s.bbox.width);
        assert!(
            x1 <= lo + 0.5 || x0 >= hi - 0.5,
            "span {:?} at x {x0}..{x1} crosses the corridor {lo}..{hi}",
            s.text
        );
    }
}

// --- counter-cases: a real two-column body still reads column-major ---

const BODY: &str = "an ordinary sentence of running body text here";
const COL_W: f32 = 210.11;
const RIGHT_X: f32 = LEFT + COL_W + 8.0;
const PROSE_ROWS: usize = 14;

fn two_column_body(with_table: bool) -> Vec<u8> {
    let mut c = String::new();
    for r in 0..PROSE_ROWS {
        let y = 700.0 - 14.0 * r as f32;
        show(&mut c, LEFT, y, &format!("LR{:02} {BODY}", r + 1));
        show(&mut c, RIGHT_X, y, &format!("RL{:02} {BODY}", r + 1));
    }
    if with_table {
        ruled_settings_table(&mut c, 700.0 - 14.0 * PROSE_ROWS as f32 - 40.0);
    }
    pdf_with(c)
}

fn assert_column_major(out: &str) {
    let at = |tag: &str| {
        out.find(tag)
            .unwrap_or_else(|| panic!("`{tag}` missing from output:\n{out}"))
    };
    let last_left = at(&format!("LR{PROSE_ROWS:02}"));
    let first_right = at("RL01");
    assert!(
        last_left < first_right,
        "the right column is spliced into the left column — a two-column body \
         reads column-major. Got:\n{out}"
    );
}

#[test]
fn test_two_column_body_still_reads_column_major() {
    let doc = PdfDocument::from_bytes(two_column_body(false)).expect("open");
    assert_column_major(&doc.extract_text(0).expect("text"));
}

/// The same body with a table on the page: a rule that keys off "this page has
/// a table" must not switch a genuine two-column body to row-major.
#[test]
fn test_two_column_body_beside_a_table_still_reads_column_major() {
    let doc = PdfDocument::from_bytes(two_column_body(true)).expect("open");
    assert_column_major(&doc.extract_text(0).expect("text"));
}
