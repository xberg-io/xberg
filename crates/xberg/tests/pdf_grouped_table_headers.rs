//! Grouped headings stay attached to their body columns (#1803).
#![cfg(feature = "pdf")]

mod helpers;
use helpers::extract_bytes_document_blocking;
use xberg::{ExtractionConfig, NodeContent, PdfConfig};

fn table_pdf(padding: Option<f32>, filled_rules: bool, heading_offset: u32) -> Vec<u8> {
    let mut content = String::new();
    if let Some(padding) = padding {
        content.push_str("0.8 g ");
        for x in [50., 190., 330.] {
            content.push_str(&format!(
                "{x} 660 140 24 re f {} 660.5 {} 23 re f ",
                x + padding,
                140. - 2. * padding
            ));
        }
    }
    content.push_str("0 g 0.5 w ");
    for y in [600, 620, 640, 660, 684] {
        content.push_str(&if filled_rules {
            format!("50 {y} 420 0.5 re f ")
        } else {
            format!("50 {y} m 470 {y} l S ")
        });
    }
    for x in [50, 120, 190, 260, 330, 400, 470] {
        let top = if [120, 260, 400].contains(&x) { 660 } else { 684 };
        content.push_str(&if filled_rules {
            format!("{x} 600 0.5 {} re f ", top - 600)
        } else {
            format!("{x} 600 m {x} {top} l S ")
        });
    }
    for (x, text) in [(58, "First"), (198, "Second"), (338, "Third")] {
        let x = x + heading_offset;
        content.push_str(&format!("BT /F1 10 Tf {x} 669 Td ({text}) Tj ET "));
    }
    for y in [605, 625, 645] {
        for (column, x) in [58, 128, 198, 268, 338, 408].into_iter().enumerate() {
            content.push_str(&format!("BT /F1 10 Tf {x} {y} Td (R{y}C{column}) Tj ET "));
        }
    }
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>".to_owned(),
        format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len()),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned(),
    ];
    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::new();
    for (i, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n{object}\nendobj\n", i + 1).as_bytes());
    }
    let xref = pdf.len();
    pdf.extend_from_slice(b"xref\n0 6\n0000000000 65535 f \n");
    for offset in offsets {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(format!("trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes());
    pdf
}

#[test]
fn grouped_headers_survive_with_and_without_inset_backgrounds() {
    // The second position puts the heading in the right-hand grid slot of its
    // spanning cell, exercising the text-retention dependency on #1804.
    for heading_offset in [0, 85] {
        for padding in [None, Some(0.), Some(4.), Some(8.)] {
            for filled_rules in [false, true] {
                let config = ExtractionConfig {
                    disable_ocr: true,
                    include_document_structure: true,
                    use_cache: false,
                    enable_quality_processing: false,
                    pdf_options: Some(PdfConfig {
                        extract_tables: true,
                        ..Default::default()
                    }),
                    ..Default::default()
                };
                let result = extract_bytes_document_blocking(
                    &table_pdf(padding, filled_rules, heading_offset),
                    "application/pdf",
                    &config,
                )
                .unwrap();
                assert_eq!(
                    result.tables.len(),
                    1,
                    "offset={heading_offset}, padding={padding:?}, filled rules={filled_rules}: {:?}",
                    result.tables
                );
                let cells = &result.tables[0].cells;
                assert_eq!(cells.len(), 4, "{cells:?}");
                assert_eq!(cells[0], ["First", "", "Second", "", "Third", ""], "{cells:?}");
                let grid = result
                    .document
                    .as_ref()
                    .unwrap()
                    .nodes
                    .iter()
                    .find_map(|node| {
                        if let NodeContent::Table { grid } = &node.content {
                            Some(grid)
                        } else {
                            None
                        }
                    })
                    .unwrap();
                assert_eq!(grid.cols, 6);
                let headers: Vec<_> = grid
                    .cells
                    .iter()
                    .filter(|c| c.row == 0)
                    .map(|c| (c.content.as_str(), c.col, c.col_span))
                    .collect();
                assert_eq!(headers, [("First", 0, 2), ("Second", 2, 2), ("Third", 4, 2)]);
                assert_eq!(
                    result.tables[0].markdown,
                    concat!(
                        "| First |  | Second |  | Third |  |\n",
                        "| --- | --- | --- | --- | --- | --- |\n",
                        "| R645C0 | R645C1 | R645C2 | R645C3 | R645C4 | R645C5 |\n",
                        "| R625C0 | R625C1 | R625C2 | R625C3 | R625C4 | R625C5 |\n",
                        "| R605C0 | R605C1 | R605C2 | R605C3 | R605C4 | R605C5 |\n",
                    )
                );
                for (row, y) in [645, 625, 605].into_iter().enumerate() {
                    let expected: Vec<_> = (0..6).map(|column| format!("R{y}C{column}")).collect();
                    assert_eq!(cells[row + 1], expected, "{cells:?}");
                }
            }
        }
    }
}
