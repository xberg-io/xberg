#![cfg(all(feature = "pdf", feature = "tokio-runtime"))]

mod helpers;

use helpers::extract_bytes_document_blocking;
use lopdf::{Document, Object, Stream, dictionary};
use xberg::{ExtractedDocument, ExtractionConfig, NodeContent, OutputFormat};

const LEAVES: [&str; 5] = ["Item", "Q1", "Q2", "Q1", "Q2"];
const RECORDS: [[&str; 5]; 2] = [["North", "1", "2", "3", "4"], ["South", "5", "6", "7", "8"]];

fn table_stream(separator_state: &str) -> String {
    let mut content = String::from("0 G 1 w 50 680 m 550 680 l S 50 660 m 550 660 l S\n");
    if separator_state == "form" {
        content.push_str("q /Dashed gs /Invisible gs /Separator Do Q\n");
    } else {
        content.push_str(&format!("q {separator_state} 50 640 m 550 640 l S Q\n"));
    }
    content.push_str("0.5 w 50 620 m 550 620 l S 50 600 m 550 600 l S\n");
    for x in [50, 150, 350, 550] {
        content.push_str(&format!("1 w {x} 600 m {x} 680 l S\n"));
    }
    for x in [250, 450] {
        content.push_str(&format!("1 w {x} 600 m {x} 660 l S\n"));
    }
    for (x, label) in [(158, "Revenue"), (358, "Costs")] {
        content.push_str(&format!("BT /Bold 10 Tf {x} 667 Td ({label}) Tj ET\n"));
    }
    for (column, label) in LEAVES.iter().enumerate() {
        let x = 58 + column * 100;
        content.push_str(&format!("BT /Bold 10 Tf {x} 647 Td ({label}) Tj ET\n"));
    }
    for (row, cells) in RECORDS.iter().enumerate() {
        for (column, text) in cells.iter().enumerate() {
            let (x, y) = (58 + column * 100, 627 - row * 20);
            content.push_str(&format!("BT /Regular 10 Tf {x} {y} Td ({text}) Tj ET\n"));
        }
    }
    content
}

fn table_pdf(separator_state: &str) -> Vec<u8> {
    let mut pdf = Document::with_version("1.4");
    let pages = pdf.new_object_id();
    let regular = pdf.add_object(dictionary! { "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica" });
    let bold = pdf.add_object(dictionary! { "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica-Bold" });
    let content = pdf.add_object(Stream::new(dictionary! {}, table_stream(separator_state).into_bytes()));
    let separator = pdf.add_object(Stream::new(
        dictionary! {
            "Type" => "XObject", "Subtype" => "Form", "BBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
            "Resources" => dictionary! { "ExtGState" => dictionary! {
                "Solid" => dictionary! { "D" => vec![Object::Array(vec![]), 0.into()], "CA" => 1 }
            } },
        },
        b"/Solid gs 50 640 m 550 640 l S".to_vec(),
    ));
    let resources = dictionary! {
        "XObject" => dictionary! { "Separator" => separator },
        "Font" => dictionary! { "Regular" => regular, "Bold" => bold },
        "ExtGState" => dictionary! {
            "Dashed" => dictionary! { "D" => vec![Object::Array(vec![3.into(), 2.into()]), 0.into()] },
            "Invisible" => dictionary! { "CA" => 0 },
            "Opaque" => dictionary! { "CA" => 1 },
            "BelowZero" => dictionary! { "CA" => -1 },
            "AboveOne" => dictionary! { "CA" => 2 },
        },
    };
    let page = pdf.add_object(dictionary! {
        "Type" => "Page", "Parent" => pages, "Contents" => content, "Resources" => resources,
        "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
    });
    pdf.objects.insert(
        pages,
        Object::Dictionary(dictionary! {
            "Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1,
        }),
    );
    let catalog = pdf.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
    pdf.trailer.set("Root", catalog);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).expect("synthetic PDF saved");
    bytes
}

fn extract(separator_state: &str) -> ExtractedDocument {
    let config = ExtractionConfig {
        disable_ocr: true,
        use_cache: false,
        enable_quality_processing: false,
        include_document_structure: true,
        output_format: OutputFormat::Html,
        ..Default::default()
    };
    extract_bytes_document_blocking(&table_pdf(separator_state), "application/pdf", &config)
        .expect("extract source table")
}

fn assert_table(result: &ExtractedDocument, leaf_headers: bool) {
    assert_eq!(result.tables.len(), 1, "{:?}", result.tables);
    assert_eq!(result.content.matches("<table").count(), 1, "{}", result.content);
    let expected = [
        vec!["", "Revenue", "", "Costs", ""],
        LEAVES.to_vec(),
        RECORDS[0].to_vec(),
        RECORDS[1].to_vec(),
    ];
    assert_eq!(result.tables[0].cells, expected, "{}", result.content);
    let grids: Vec<_> = result
        .document
        .as_ref()
        .expect("public structure")
        .nodes
        .iter()
        .filter_map(|node| match &node.content {
            NodeContent::Table { grid } => Some(grid),
            _ => None,
        })
        .collect();
    assert_eq!(grids.len(), 1);
    let grid = grids[0];
    assert_eq!((grid.rows, grid.cols), (4, 5));
    let groups: Vec<_> = grid
        .cells
        .iter()
        .filter(|cell| cell.row == 0 && !cell.content.is_empty())
        .map(|cell| (cell.col, cell.col_span, cell.row_span, cell.content.as_str()))
        .collect();
    assert_eq!(groups, [(1, 2, 1, "Revenue"), (3, 2, 1, "Costs")]);
    let leaves: Vec<_> = grid
        .cells
        .iter()
        .filter(|cell| cell.row == 1)
        .map(|cell| (cell.content.as_str(), cell.is_header, cell.col_span, cell.row_span))
        .collect();
    assert_eq!(
        leaves,
        LEAVES.map(|label| (label, leaf_headers, 1, 1)),
        "{}",
        result.content
    );
    let tag = if leaf_headers { "th" } else { "td" };
    for (label, count) in [("Item", 1), ("Q1", 2), ("Q2", 2)] {
        assert_eq!(
            result.content.matches(&format!("<{tag}>{label}</{tag}>")).count(),
            count
        );
    }
    for (row, record) in RECORDS.iter().enumerate() {
        let cells: Vec<_> = grid
            .cells
            .iter()
            .filter(|cell| cell.row == row as u32 + 2)
            .map(|cell| (cell.content.as_str(), cell.is_header, cell.col_span, cell.row_span))
            .collect();
        assert_eq!(cells, record.map(|text| (text, false, 1, 1)));
        let html_row = record.map(|text| format!("<td>{text}</td>")).join("");
        assert_eq!(result.content.matches(&format!("<tr>{html_row}</tr>")).count(), 1);
    }
    for label in ["Revenue", "Costs"] {
        assert_eq!(
            result
                .content
                .matches(&format!("<th colspan=\"2\">{label}</th>"))
                .count(),
            1
        );
    }
    assert_eq!(result.content.matches("colspan=\"2\"").count(), 2);
}

#[test]
fn should_retain_leaf_headers_proven_by_solid_separator() {
    assert_table(&extract(""), true);
}

#[test]
fn should_not_infer_leaf_headers_from_direct_dashed_separator() {
    assert_table(&extract("[3 2] 0 d"), false);
}

#[test]
fn should_not_infer_leaf_headers_from_extgstate_dashed_separator() {
    assert_table(&extract("/Dashed gs"), false);
}

#[test]
fn should_not_infer_leaf_headers_from_invisible_separator() {
    assert_table(&extract("/Invisible gs"), false);
}

#[test]
fn should_retain_leaf_headers_proven_by_opaque_separator() {
    assert_table(&extract("/Opaque gs"), true);
}

#[test]
fn should_restore_solid_separator_after_scoped_extgstate() {
    assert_table(&extract("q /Dashed gs /Invisible gs Q"), true);
}

#[test]
fn should_retain_leaf_headers_from_form_local_solid_opaque_override() {
    assert_table(&extract("form"), true);
}

#[test]
fn should_clamp_negative_stroke_opacity_before_proving_headers() {
    assert_table(&extract("/BelowZero gs"), false);
}

#[test]
fn should_clamp_above_one_stroke_opacity_after_invisible_state() {
    assert_table(&extract("/Invisible gs /AboveOne gs"), true);
}
