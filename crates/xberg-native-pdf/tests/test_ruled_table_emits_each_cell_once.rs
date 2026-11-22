//! A ruled table's cells appear exactly once on every surface.
//!
//! Three rules used to decide whether a span belonged to a table and they
//! disagreed: the detector claimed a span by its bbox **centre** with a 3 pt
//! snap over populated cells, while the converters claimed by the span's
//! **origin** with a 2 pt slack over the full lattice. A span caught one way
//! was rendered by the cell *and* emitted in the prose flow; caught the other
//! way it was suppressed from prose and rendered by nobody.
//!
//! Ownership is now answered from what the table actually renders — marked
//! content where the file is tagged, the cells' own spans otherwise — so both
//! directions resolve together. See `span_in_table` in
//! `src/pipeline/converters/mod.rs`.
//!
//! The fixture is deliberately trivial: four cells and six stroked rules. The
//! defect reproduced here, so it was never an edge case.

use xberg_native_pdf::PdfDocument;

/// A 2x2 ruled table: column rules at x=50/200/350, row rules at
/// y=600/660/720, and four cells.
fn ruled_table_pdf() -> Vec<u8> {
    let content: &[u8] = b"0.5 w\n\
        50 600 m 50 720 l S\n200 600 m 200 720 l S\n350 600 m 350 720 l S\n\
        50 600 m 350 600 l S\n50 660 m 350 660 l S\n50 720 m 350 720 l S\n\
        BT /F1 12 Tf\n\
        1 0 0 1 60 690 Tm (North) Tj\n1 0 0 1 210 690 Tm (120) Tj\n\
        1 0 0 1 60 630 Tm (South) Tj\n1 0 0 1 210 630 Tm (90) Tj\nET\n";

    let objects: Vec<Vec<u8>> = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R \
           /Resources << /Font << /F1 5 0 R >> >> >>"
            .to_vec(),
        [
            format!("<< /Length {} >>\nstream\n", content.len()).into_bytes(),
            content.to_vec(),
            b"\nendstream".to_vec(),
        ]
        .concat(),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>".to_vec(),
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

/// Every cell exactly once, on both surfaces. Twice means the span was
/// rendered by the cell and emitted into prose as well; zero means it was
/// suppressed from prose and rendered by nobody.
#[test]
fn each_cell_appears_exactly_once_on_every_surface() {
    let doc = PdfDocument::from_bytes(ruled_table_pdf()).expect("fixture parses");
    let table_cells: String = doc
        .extract_tables(0)
        .expect("tables")
        .iter()
        .flat_map(|t| t.rows.iter())
        .flat_map(|r| r.cells.iter())
        .map(|c| format!("{}\n", c.text))
        .collect();

    let surfaces = [
        ("text", doc.extract_text(0).expect("text")),
        ("table cells", table_cells),
    ];

    for (surface, out) in &surfaces {
        for cell in ["North", "South", "120"] {
            let n = out.matches(cell).count();
            assert_eq!(
                n, 1,
                "{surface}: cell {cell:?} appears {n} times, expected once \
                 (twice = rendered by the table and emitted in prose as well; \
                 zero = suppressed from prose and rendered by nobody)\n{out}"
            );
        }
    }
}
