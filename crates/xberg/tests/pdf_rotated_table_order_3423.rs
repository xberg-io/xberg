//! A table drawn with a 90, 180 or 270 degree text matrix must extract row by row, top row first as
//! displayed, with each label next to its value and each row on its own line (xberg-io/xberg#3423).
//! Before the fix the cells came out in reverse order (90) or column by column (270) through the
//! default extraction path.
#![cfg(all(feature = "pdf", feature = "tokio-runtime"))]

mod helpers;

use helpers::extract_bytes_document_blocking;
use xberg::core::config::ExtractionConfig;

const ROWS: [[&str; 3]; 3] = [
    ["Engine oil", "6.0 quarts (5.7L)", "Motorcraft SAE 5W-20"],
    ["Coolant", "12.1 quarts (11.5L)", "Motorcraft Orange"],
    ["Brake fluid", "as needed", "DOT 4 LV"],
];
const COL_X: [i32; 3] = [0, 160, 340];
const ROW_PITCH: i32 = 24;

/// How the table rows are laid out in the table's own frame.
#[derive(Clone, Copy)]
enum Rows {
    /// Row 0 on top, later rows below it (a table as people draw it).
    Down,
    /// Row 0 at the bottom, later rows above it, as in the issue's reportlab reproducer.
    Up,
}

/// Text matrix `(a, b, c, d)` and a page origin that keeps the whole table on the page.
fn frame(rotation: i32) -> ([i32; 4], (i32, i32)) {
    match rotation {
        0 => ([1, 0, 0, 1], (50, 600)),
        90 => ([0, 1, -1, 0], (300, 50)),
        180 => ([-1, 0, 0, -1], (560, 600)),
        270 => ([0, -1, 1, 0], (300, 560)),
        _ => unreachable!(),
    }
}

/// One Letter page with the table drawn through a `rotation`-degree text matrix. `order` lists the
/// cells (row-major index) in draw order; `page_rotate` is the page dictionary `/Rotate`.
fn build_pdf(rotation: i32, rows: Rows, page_rotate: i32, order: &[usize]) -> Vec<u8> {
    let ([a, b, c, d], (ox, oy)) = frame(rotation);
    let mut content = String::from("BT /F1 12 Tf\n");
    for &cell_index in order {
        let (n, column) = (cell_index / 3, cell_index % 3);
        let cell = ROWS[n][column];
        let cx = COL_X[column];
        let cy = match rows {
            Rows::Down => -ROW_PITCH * n as i32,
            Rows::Up => ROW_PITCH * n as i32,
        };
        let (e, f) = (ox + cx * a + cy * c, oy + cx * b + cy * d);
        content.push_str(&format!("{a} {b} {c} {d} {e} {f} Tm ({cell}) Tj\n"));
    }
    content.push_str("ET");

    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Rotate {page_rotate} /Contents 4 0 R \
             /Resources << /Font << /F1 5 0 R >> >> >>"
        ),
        format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len()),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>".to_string(),
    ];
    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::new();
    for (i, body) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n{body}\nendobj\n", i + 1).as_bytes());
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

/// The extracted text as one normalized string per non-empty line.
fn extracted_rows(pdf: &[u8]) -> Vec<String> {
    let config = ExtractionConfig {
        disable_ocr: true,
        use_cache: false,
        enable_quality_processing: false,
        ..Default::default()
    };
    let document = extract_bytes_document_blocking(pdf, "application/pdf", &config).unwrap();
    document
        .content
        .lines()
        .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|line| !line.is_empty())
        .collect()
}

fn row_text(row: usize) -> String {
    ROWS[row].join(" ")
}

fn rows_in(order: [usize; 3]) -> Vec<String> {
    order.into_iter().map(row_text).collect()
}

const DRAW_ORDERS: [[usize; 9]; 3] = [
    [0, 1, 2, 3, 4, 5, 6, 7, 8],
    [8, 7, 6, 5, 4, 3, 2, 1, 0],
    [4, 8, 0, 6, 2, 5, 1, 7, 3],
];

#[test]
fn upright_table_reads_row_by_row() {
    for order in DRAW_ORDERS {
        assert_eq!(extracted_rows(&build_pdf(0, Rows::Down, 0, &order)), rows_in([0, 1, 2]));
    }
}

#[test]
fn table_rotated_90_in_content_stream_reads_row_by_row() {
    for order in DRAW_ORDERS {
        assert_eq!(
            extracted_rows(&build_pdf(90, Rows::Down, 0, &order)),
            rows_in([0, 1, 2])
        );
    }
}

#[test]
fn table_rotated_180_in_content_stream_reads_row_by_row() {
    for order in DRAW_ORDERS {
        assert_eq!(
            extracted_rows(&build_pdf(180, Rows::Down, 0, &order)),
            rows_in([0, 1, 2])
        );
    }
}

#[test]
fn table_rotated_270_in_content_stream_reads_row_by_row() {
    for order in DRAW_ORDERS {
        assert_eq!(
            extracted_rows(&build_pdf(270, Rows::Down, 0, &order)),
            rows_in([0, 1, 2])
        );
    }
}

/// The issue's reproducer draws row 0 lowest: displayed, the last-drawn row is on top, so the
/// top-to-bottom order is the reverse of the draw order, with every label still beside its value.
#[test]
fn issue_geometry_rotated_90_reads_top_to_bottom_with_labels_beside_values() {
    for order in DRAW_ORDERS {
        assert_eq!(extracted_rows(&build_pdf(90, Rows::Up, 0, &order)), rows_in([2, 1, 0]));
    }
}

#[test]
fn issue_geometry_rotated_270_reads_top_to_bottom_with_labels_beside_values() {
    for order in DRAW_ORDERS {
        assert_eq!(extracted_rows(&build_pdf(270, Rows::Up, 0, &order)), rows_in([2, 1, 0]));
    }
}

/// A 90 degree matrix on a `/Rotate 90` page displays upright.
#[test]
fn rotate_90_page_with_90_degree_matrix_reads_row_by_row() {
    let order = DRAW_ORDERS[2];
    assert_eq!(
        extracted_rows(&build_pdf(90, Rows::Down, 90, &order)),
        rows_in([0, 1, 2])
    );
}

/// An upright matrix on a `/Rotate 90` page displays turned a quarter; each row stays whole.
#[test]
fn rotate_90_page_with_upright_matrix_keeps_each_row_whole() {
    let order = DRAW_ORDERS[2];
    let mut rows = extracted_rows(&build_pdf(0, Rows::Down, 90, &order));
    rows.sort();
    let mut expected = rows_in([0, 1, 2]);
    expected.sort();
    assert_eq!(rows, expected);
}
