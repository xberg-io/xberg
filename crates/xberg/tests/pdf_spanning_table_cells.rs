//! Text inside a row-spanning cell must survive public extraction (#1802).
#![cfg(feature = "pdf")]

mod helpers;
use helpers::extract_bytes_document_blocking;
use xberg::{ExtractionConfig, NodeContent, OutputFormat, PdfConfig};

fn table_pdf(label_y: u32) -> Vec<u8> {
    let content = format!(
        "0.5 w 50 600 m 450 600 l S 50 660 m 450 660 l S \
         50 620 m 450 620 l S 150 640 m 450 640 l S \
         50 600 m 50 660 l S 150 600 m 150 660 l S \
         300 600 m 300 660 l S 450 600 m 450 660 l S \
         BT /F1 10 Tf 60 {label_y} Td (SPAN) Tj ET \
         BT /F1 10 Tf 160 645 Td (Alpha) Tj ET BT /F1 10 Tf 310 645 Td (10) Tj ET \
         BT /F1 10 Tf 160 625 Td (Beta) Tj ET BT /F1 10 Tf 310 625 Td (20) Tj ET \
         BT /F1 10 Tf 60 605 Td (Anchor) Tj ET BT /F1 10 Tf 160 605 Td (Gamma) Tj ET BT /F1 10 Tf 310 605 Td (30) Tj ET \
         BT /F1 10 Tf 60 690 Td (Outside the table) Tj ET"
    );
    pdf_with_content(content)
}

fn pdf_with_content(content: String) -> Vec<u8> {
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
fn row_spanning_label_is_retained_once_at_every_vertical_position() {
    for y in [645, 635, 625] {
        let config = ExtractionConfig {
            disable_ocr: true,
            use_cache: false,
            enable_quality_processing: false,
            pdf_options: Some(PdfConfig {
                extract_tables: true,
                ..Default::default()
            }),
            ..Default::default()
        };
        let result = extract_bytes_document_blocking(&table_pdf(y), "application/pdf", &config).unwrap();
        assert_eq!(result.tables.len(), 1, "label y={y}: {:?}", result.tables);
        let cells = &result.tables[0].cells;
        assert_eq!(
            cells
                .iter()
                .filter(|row| row.first().is_some_and(|c| c == "SPAN"))
                .count(),
            1,
            "label y={y}: {cells:?}"
        );
        for text in ["SPAN", "Anchor", "Alpha", "Beta", "Gamma", "10", "20", "30"] {
            assert_eq!(
                cells.iter().flatten().filter(|c| c.as_str() == text).count(),
                1,
                "{text}, label y={y}: {cells:?}"
            );
        }
        assert!(!cells.iter().flatten().any(|c| c.contains("Outside")));
    }
}

#[test]
fn neighbouring_description_rules_do_not_erase_alignment_cells() {
    for separators in ["150 640 m 300 640 l S", "150 635 m 300 635 l S 150 640 m 300 640 l S"] {
        let pdf = pdf_with_content(format!(
            "0.5 w 50 600 m 550 600 l S 50 620 m 550 620 l S 50 660 m 550 660 l S 50 690 m 550 690 l S \
             50 600 m 50 690 l S 150 600 m 150 690 l S 300 600 m 300 690 l S 450 600 m 450 690 l S 550 600 m 550 690 l S \
             {separators} \
             BT /F1 10 Tf 60 675 Td (Code) Tj ET BT /F1 10 Tf 160 675 Td (Description) Tj ET BT /F1 10 Tf 310 675 Td (Alignment) Tj ET BT /F1 10 Tf 460 675 Td (Example) Tj ET \
             BT /F1 10 Tf 60 645 Td (DT) Tj ET BT /F1 10 Tf 160 645 Td (Date) Tj ET BT /F1 10 Tf 160 625 Td (Details) Tj ET \
             BT /F1 10 Tf 310 645 Td (Destra) Tj ET BT /F1 10 Tf 310 634 Td (con 8 spazi) Tj ET BT /F1 10 Tf 460 645 Td (05051998) Tj ET \
             BT /F1 10 Tf 60 605 Td (Next) Tj ET BT /F1 10 Tf 160 605 Td (Other) Tj ET BT /F1 10 Tf 310 605 Td (Sinistra) Tj ET BT /F1 10 Tf 460 605 Td (value) Tj ET \
             BT /F1 10 Tf 310 715 Td (Outside) Tj ET"
        ));
        for output_format in [OutputFormat::Plain, OutputFormat::Markdown] {
            let config = ExtractionConfig {
                disable_ocr: true,
                use_cache: false,
                enable_quality_processing: false,
                include_document_structure: true,
                output_format,
                pdf_options: Some(PdfConfig {
                    extract_tables: true,
                    ..Default::default()
                }),
                ..Default::default()
            };
            let result = extract_bytes_document_blocking(&pdf, "application/pdf", &config).unwrap();
            assert_eq!(result.tables.len(), 1);
            let cells = &result.tables[0].cells;
            let group_start = cells.iter().position(|row| row[0] == "DT").unwrap();
            let group_end = cells.iter().position(|row| row[0] == "Next").unwrap();
            assert!(group_start < group_end, "{cells:?}");
            for text in ["Destra", "con 8 spazi"] {
                let matching_rows: Vec<_> = cells
                    .iter()
                    .enumerate()
                    .filter(|(_, row)| row.get(2).is_some_and(|cell| cell.contains(text)))
                    .map(|(row, _)| row)
                    .collect();
                assert_eq!(matching_rows.len(), 1, "{cells:?}");
                assert!((group_start..group_end).contains(&matching_rows[0]), "{cells:?}");
            }
            assert_eq!(cells[group_end][2], "Sinistra");
            assert!(!cells.iter().flatten().any(|cell| cell.contains("Outside")));
            let doc = result.document.unwrap();
            assert!(!doc.nodes.iter().any(|node| matches!(&node.content, NodeContent::Paragraph {text} if text.contains("con 8 spazi") || text.contains("Destra"))));
            let grid = doc
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
            let structured_start = grid
                .cells
                .iter()
                .find(|cell| cell.col == 0 && cell.content == "DT")
                .unwrap()
                .row;
            let structured_end = grid
                .cells
                .iter()
                .find(|cell| cell.col == 0 && cell.content == "Next")
                .unwrap()
                .row;
            for text in ["Destra", "con 8 spazi"] {
                let matches: Vec<_> = grid.cells.iter().filter(|cell| cell.content.contains(text)).collect();
                assert_eq!(matches.len(), 1, "{grid:?}");
                assert_eq!(matches[0].col, 2, "{grid:?}");
                assert!((structured_start..structured_end).contains(&matches[0].row), "{grid:?}");
            }
        }
    }
}
