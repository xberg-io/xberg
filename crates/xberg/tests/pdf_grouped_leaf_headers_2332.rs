#![cfg(feature = "pdf")]

mod helpers;

use helpers::extract_bytes_document_blocking;
use lopdf::{Document, Object, Stream, dictionary};
use std::fmt::Write;
use xberg::{ExtractedDocument, ExtractionConfig, NodeContent, OutputFormat, TableGrid};

#[derive(Clone, Copy, Default)]
struct Source {
    no_shading: bool,
    bold_body: bool,
    different_schema: bool,
    erased_shading: bool,
    repainted_shading: bool,
    partially_erased_leaf_shading: bool,
    zebra_body: bool,
    first_value: Option<&'static str>,
    separator: &'static str,
}

fn page(operators: String) -> Vec<u8> {
    let mut pdf = Document::with_version("1.4");
    let pages = pdf.new_object_id();
    let fonts: Vec<_> = ["Helvetica", "Helvetica-Bold"]
        .map(|name| {
            pdf.add_object(dictionary! {
                "Type" => "Font", "Subtype" => "Type1", "BaseFont" => name,
            })
        })
        .into();
    let stream = pdf.add_object(Stream::new(dictionary! {}, operators.into_bytes()));
    let page = pdf.add_object(dictionary! {
        "Type" => "Page", "Parent" => pages, "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
        "Resources" => dictionary! {
            "Font" => dictionary! { "F1" => fonts[0], "F2" => fonts[1] },
            "ExtGState" => dictionary! { "Invisible" => dictionary! { "CA" => 0 } },
        },
        "Contents" => stream,
    });
    pdf.objects.insert(
        pages,
        Object::Dictionary(dictionary! {
            "Type" => "Pages", "Kids" => vec![Object::Reference(page)], "Count" => 1,
        }),
    );
    let catalog = pdf.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
    pdf.trailer.set("Root", catalog);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    bytes
}

fn text(operators: &mut String, font: &str, x: i32, y: i32, label: &str) {
    writeln!(operators, "BT /{font} 9 Tf 1 0 0 1 {x} {y} Tm ({label}) Tj ET").unwrap();
}

fn table_pdf(source: Source) -> Vec<u8> {
    let mut operators = String::new();
    if !source.no_shading {
        operators.push_str(".9 g 60 616 400 22 re f 0 g\n");
    }
    if source.erased_shading || source.repainted_shading {
        operators.push_str("1 g 60 616 400 22 re f 0 g\n");
    }
    if source.repainted_shading {
        operators.push_str(".9 g 60 616 400 22 re f 0 g\n");
    }
    if source.partially_erased_leaf_shading {
        operators.push_str(".9 g 60 638 400 22 re f 1 g 130 650 6 1 re f 0 g\n");
    }
    if source.zebra_body {
        operators.push_str(".9 g 60 572 400 22 re f 0 g\n");
    }
    text(&mut operators, "F2", 150, 666, "North");
    text(&mut operators, "F2", 310, 666, "South");
    let labels = if source.different_schema {
        ["Item", "Value", "Count", "Value", "Other"]
    } else {
        ["Item", "Value", "Count", "Value", "Count"]
    };
    for (column, label) in labels.into_iter().enumerate() {
        text(&mut operators, "F2", 66 + column as i32 * 80, 644, label);
    }
    for (y, label) in [(622, "Alpha"), (600, "Beta"), (578, "Total")] {
        let font = if source.bold_body || y == 578 { "F2" } else { "F1" };
        for (column, value) in [label, source.first_value.unwrap_or("12"), "3", "24", "6"]
            .into_iter()
            .enumerate()
        {
            text(&mut operators, font, 66 + column as i32 * 80, y, value);
        }
    }
    operators.push_str("1 1 1 RG .5 w\n");
    for y in [572, 594, 616, 638, 660, 682] {
        writeln!(operators, "60 {y} m 460 {y} l S").unwrap();
    }
    for x in [60, 140, 220, 300, 380, 460] {
        let top = if [220, 380].contains(&x) { 660 } else { 682 };
        writeln!(operators, "{x} 572 m {x} {top} l S").unwrap();
    }
    operators.push_str(source.separator);
    page(operators)
}

fn extract(source: Source) -> ExtractedDocument {
    let config = ExtractionConfig {
        disable_ocr: true,
        use_cache: false,
        enable_quality_processing: false,
        include_document_structure: true,
        output_format: OutputFormat::Html,
        ..Default::default()
    };
    extract_bytes_document_blocking(&table_pdf(source), "application/pdf", &config).unwrap()
}

fn grid(result: &ExtractedDocument) -> &TableGrid {
    assert_eq!(result.tables.len(), 1);
    assert_eq!(result.content.matches("<table").count(), 1);
    let grids: Vec<_> = result
        .document
        .as_ref()
        .unwrap()
        .nodes
        .iter()
        .filter_map(|node| match &node.content {
            NodeContent::Table { grid } => Some(grid),
            _ => None,
        })
        .collect();
    assert_eq!(grids.len(), 1);
    assert_eq!((grids[0].rows, grids[0].cols), (5, 5));
    grids[0]
}

#[test]
fn repeated_group_leaves_over_a_shaded_body_reach_public_html_and_grid() {
    let result = extract(Source::default());
    let grid = grid(&result);
    let leaves: Vec<_> = grid
        .cells
        .iter()
        .filter(|cell| cell.row == 1)
        .map(|cell| (cell.content.as_str(), cell.is_header, cell.row_span, cell.col_span))
        .collect();
    assert_eq!(
        leaves,
        [
            ("Item", true, 1, 1),
            ("Value", true, 1, 1),
            ("Count", true, 1, 1),
            ("Value", true, 1, 1),
            ("Count", true, 1, 1)
        ]
    );
    assert_eq!(result.content.matches("<th>Value</th>").count(), 2);
    assert_eq!(result.content.matches("<th>Count</th>").count(), 2);
    assert_eq!(result.content.matches("<td>Alpha</td>").count(), 1);
    assert_eq!(result.content.matches("<td>Total</td>").count(), 1);
    assert!(
        grid.cells
            .iter()
            .filter(|cell| cell.row >= 2)
            .all(|cell| !cell.is_header)
    );
    let bounds = result.tables[0].bounding_box.unwrap();
    assert_eq!(
        (bounds.x0, bounds.y0, bounds.x1, bounds.y1),
        (60.0, 572.0, 460.0, 682.0)
    );
    let parents: Vec<_> = grid
        .cells
        .iter()
        .filter(|cell| cell.row == 0)
        .map(|cell| (cell.col, cell.row_span, cell.col_span))
        .collect();
    assert_eq!(parents, [(0, 1, 1), (1, 1, 2), (3, 1, 2)]);
}

#[test]
fn bold_body_or_different_group_schema_does_not_establish_leaf_headers() {
    for source in [
        Source {
            bold_body: true,
            ..Source::default()
        },
        Source {
            different_schema: true,
            ..Source::default()
        },
    ] {
        let result = extract(source);
        assert!(
            grid(&result)
                .cells
                .iter()
                .filter(|cell| cell.row == 1)
                .all(|cell| !cell.is_header)
        );
        assert_eq!(result.content.matches("<td>Value</td>").count(), 2);
    }
}

#[test]
fn separators_without_independent_shading_do_not_establish_leaf_headers() {
    for separator in [
        "",
        "0 G 60 638 m 460 638 l S",
        "0 G [3 2] 0 d 60 638 m 460 638 l S",
        "q /Invisible gs 0 G 60 638 m 460 638 l S Q",
        ".98 G 60 638 m 460 638 l S",
    ] {
        let result = extract(Source {
            no_shading: true,
            separator,
            ..Source::default()
        });
        assert!(
            grid(&result)
                .cells
                .iter()
                .filter(|cell| cell.row == 1)
                .all(|cell| !cell.is_header)
        );
        assert_eq!(result.content.matches("<td>Value</td>").count(), 2);
    }
}

#[test]
fn erased_body_shading_does_not_establish_leaf_headers() {
    let result = extract(Source {
        erased_shading: true,
        ..Source::default()
    });
    assert!(
        grid(&result)
            .cells
            .iter()
            .filter(|cell| cell.row == 1)
            .all(|cell| !cell.is_header)
    );
    assert_eq!(result.content.matches("<td>Value</td>").count(), 2);
}

#[test]
fn later_gray_paint_restores_independent_body_shading() {
    let result = extract(Source {
        repainted_shading: true,
        ..Source::default()
    });
    assert_eq!(
        grid(&result)
            .cells
            .iter()
            .filter(|cell| cell.row == 1 && cell.is_header)
            .count(),
        5
    );
    assert_eq!(result.content.matches("<th>Value</th>").count(), 2);
}

#[test]
fn partially_erased_leaf_shading_does_not_prove_contrast_with_the_body() {
    let result = extract(Source {
        partially_erased_leaf_shading: true,
        ..Source::default()
    });
    assert!(
        grid(&result)
            .cells
            .iter()
            .filter(|cell| cell.row == 1)
            .all(|cell| !cell.is_header)
    );
    assert_eq!(result.content.matches("<td>Value</td>").count(), 2);
}

fn assert_leaf_roles(result: &ExtractedDocument, expected: bool) {
    let grid = grid(result);
    let leaves: Vec<_> = grid.cells.iter().filter(|cell| cell.row == 1).collect();
    assert_eq!(leaves.len(), 5);
    assert!(leaves.iter().all(|cell| cell.is_header == expected));
    let tag = if expected { "th" } else { "td" };
    assert_eq!(result.content.matches(&format!("<{tag}>Value</{tag}>")).count(), 2);
    assert_eq!(result.content.matches("<td>Alpha</td>").count(), 1);
    assert_eq!(result.content.matches("<td>Total</td>").count(), 1);
    assert!(
        grid.cells
            .iter()
            .filter(|cell| cell.row >= 2)
            .all(|cell| !cell.is_header)
    );
    assert!(grid.cells.iter().all(|cell| cell.row_span == 1));
    let bounds = result.tables[0].bounding_box.unwrap();
    assert_eq!(
        (bounds.x0, bounds.y0, bounds.x1, bounds.y1),
        (60.0, 572.0, 460.0, 682.0)
    );
    let parents: Vec<_> = grid
        .cells
        .iter()
        .filter(|cell| cell.row == 0)
        .map(|cell| (cell.col, cell.row_span, cell.col_span))
        .collect();
    assert_eq!(parents, [(0, 1, 1), (1, 1, 2), (3, 1, 2)]);
}

#[test]
fn independent_shading_proof_does_not_require_visible_separator_rules() {
    for separator in [
        "0 G [3 2] 0 d 60 638 m 460 638 l S 60 616 m 460 616 l S",
        "q /Invisible gs 0 G 60 638 m 460 638 l S 60 616 m 460 616 l S Q",
        ".98 G 60 638 m 460 638 l S 60 616 m 460 616 l S",
        "0 G 60 638 m 460 638 l S",
        "0 G 60 616 m 460 616 l S",
    ] {
        assert_leaf_roles(
            &extract(Source {
                separator,
                ..Source::default()
            }),
            true,
        );
    }
}

#[test]
fn matching_dark_leaf_and_body_rules_veto_the_shading_proof() {
    let source = Source {
        separator: "0 G 60 638 m 460 638 l S 60 616 m 460 616 l S",
        ..Source::default()
    };
    assert_leaf_roles(&extract(source), false);
}

#[test]
fn highlighting_or_zebra_rows_alone_do_not_establish_headers() {
    for zebra_body in [false, true] {
        assert_leaf_roles(
            &extract(Source {
                zebra_body,
                different_schema: true,
                ..Source::default()
            }),
            false,
        );
    }
    assert_leaf_roles(
        &extract(Source {
            zebra_body: true,
            ..Source::default()
        }),
        true,
    );
}

#[test]
fn first_body_values_need_an_ascii_digit_not_a_numeric_only_string() {
    for (first_value, expected) in [("A1", true), ("Note", false)] {
        assert_leaf_roles(
            &extract(Source {
                first_value: Some(first_value),
                ..Source::default()
            }),
            expected,
        );
    }
}
