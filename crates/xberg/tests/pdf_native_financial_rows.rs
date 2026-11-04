//! Native PDF financial rows retain their amounts and do not acquire a data header.
#![cfg(feature = "pdf")]

mod helpers;

use helpers::extract_bytes_document_blocking;
use lopdf::{Document, Object, Stream, dictionary};
use std::fmt::Write as _;
use xberg::{ExtractedDocument, ExtractionConfig, ExtractionMethod, NodeContent, OutputFormat, PageConfig};

const LABELS: [&str; 10] = [
    "Cash and equivalents",
    "Accounts receivable",
    "Inventory",
    "Prepaid expenses",
    "Total current assets",
    "Property and equipment",
    "Accounts payable",
    "Accrued liabilities",
    "Long-term debt",
    "Total liabilities",
];
const AMOUNT_LEFTS: [f32; 4] = [304.98, 374.98, 444.98, 514.98];
const CAPTION: &str = "Financial schedule and explanatory note.";

#[derive(Clone, Copy)]
enum Painting {
    Rows,
    Columns,
    Caption,
    AuthoredHeader,
}

fn expected_rows() -> Vec<Vec<String>> {
    LABELS
        .iter()
        .enumerate()
        .map(|(row, label)| {
            std::iter::once((*label).to_owned())
                .chain((0..4).map(|column| {
                    let amount = 1204 + 317 * row + 59 * column;
                    format!("{},{:03}", amount / 1000, amount % 1000)
                }))
                .collect()
        })
        .collect()
}

fn draw(operators: &mut String, font: &str, x: f32, y: usize, text: &str) {
    writeln!(operators, "BT /{font} 10 Tf 1 0 0 1 {x} {y} Tm ({text}) Tj ET").unwrap();
}

fn paint_financial_rows(operators: &mut String, painting: Painting, rows: &[Vec<String>]) {
    if matches!(painting, Painting::Caption) {
        draw(operators, "F1", 72.0, 740, CAPTION);
    }
    if matches!(painting, Painting::AuthoredHeader) {
        draw(operators, "F2", 72.0, 722, "Metric");
        for (column, x) in [292.2, 362.2, 432.2, 502.2].into_iter().enumerate() {
            draw(operators, "F2", x, 722, &format!("FY {}", 2021 + column));
        }
    }
    for (row_index, row) in rows.iter().enumerate() {
        let y = 700 - 22 * row_index;
        draw(operators, "F1", 72.0, y, &row[0]);
        if !matches!(painting, Painting::Columns) {
            for (x, amount) in AMOUNT_LEFTS.into_iter().zip(&row[1..]) {
                draw(operators, "F1", x, y, amount);
            }
        }
    }
    if matches!(painting, Painting::Columns) {
        for (column, x) in AMOUNT_LEFTS.iter().enumerate() {
            for (row_index, row) in rows.iter().enumerate() {
                draw(operators, "F1", *x, 700 - 22 * row_index, &row[column + 1]);
            }
        }
    }
}

fn paint_rules(operators: &mut String, painting: Painting) {
    let extra = usize::from(matches!(painting, Painting::AuthoredHeader));
    let top = 714 + 22 * extra;
    let bottom = 714 - 22 * LABELS.len();
    for row in 0..=LABELS.len() + extra {
        let y = top - 22 * row;
        writeln!(operators, "66 {y} m 560 {y} l S").unwrap();
    }
    for x in [66, 220, 350, 420, 490, 560] {
        writeln!(operators, "{x} {top} m {x} {bottom} l S").unwrap();
    }
}

fn financial_pdf(painting: Painting) -> Vec<u8> {
    let mut pdf = Document::with_version("1.4");
    let pages = pdf.new_object_id();
    let font = pdf.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica", "Encoding" => "WinAnsiEncoding",
    });
    let bold = pdf.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica-Bold", "Encoding" => "WinAnsiEncoding",
    });
    let rows = expected_rows();
    let kids: Vec<_> = [false, true]
        .into_iter()
        .map(|ruled| {
            let mut operators = String::new();
            paint_financial_rows(&mut operators, painting, &rows);
            if ruled {
                paint_rules(&mut operators, painting);
            }
            let content = pdf.add_object(Stream::new(dictionary! {}, operators.into_bytes()));
            Object::Reference(pdf.add_object(dictionary! {
                "Type" => "Page", "Parent" => pages, "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
                "Resources" => dictionary! { "Font" => dictionary! { "F1" => font, "F2" => bold } },
                "Contents" => content,
            }))
        })
        .collect();
    pdf.objects.insert(
        pages,
        Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => kids, "Count" => 2 }),
    );
    let catalog = pdf.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
    pdf.trailer.set("Root", catalog);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).expect("write synthetic two-page PDF");
    bytes
}

fn extract(painting: Painting, config: ExtractionConfig) -> ExtractedDocument {
    let result = extract_bytes_document_blocking(&financial_pdf(painting), "application/pdf", &config)
        .expect("extract native PDF");
    assert_eq!(result.extraction_method, Some(ExtractionMethod::Native));
    result
}

fn nonempty_lines(text: &str) -> Vec<&str> {
    text.lines().filter(|line| !line.trim().is_empty()).collect()
}

fn expected_plain() -> Vec<String> {
    expected_rows().into_iter().map(|row| row.join(" ")).collect()
}

fn expected_markdown() -> Vec<String> {
    ["|  |  |  |  |  |", "| --- | --- | --- | --- | --- |"]
        .into_iter()
        .map(str::to_owned)
        .chain(
            expected_rows()
                .into_iter()
                .map(|row| format!("| {} |", row.join(" | "))),
        )
        .collect()
}

#[test]
fn default_plain_pairs_all_amounts_with_their_labels_on_both_pages() {
    let result = extract(Painting::Rows, ExtractionConfig::default());
    let page = expected_plain();
    assert_eq!(
        nonempty_lines(&result.content),
        [page.as_slice(), page.as_slice()].concat()
    );
}

#[test]
fn markdown_preserves_every_data_row_with_an_empty_syntax_header() {
    let result = extract(
        Painting::Rows,
        ExtractionConfig {
            output_format: OutputFormat::Markdown,
            pages: Some(PageConfig {
                extract_pages: true,
                ..Default::default()
            }),
            ..Default::default()
        },
    );
    let page = expected_markdown();
    assert_eq!(
        nonempty_lines(&result.content),
        [page.as_slice(), page.as_slice()].concat()
    );
    let pages = result.pages.expect("requested both pages");
    assert_eq!(pages.len(), 2);
    for page_content in pages {
        assert_eq!(nonempty_lines(&page_content.content), page);
    }
    assert_eq!(result.tables.len(), 2);
    for table in result.tables {
        assert_eq!(nonempty_lines(&table.markdown), page);
        assert_eq!(table.cells, expected_rows());
        assert_eq!(table.columns, None);
    }
}

#[test]
fn plain_pages_and_structured_grids_preserve_the_exact_body_rows() {
    let result = extract(
        Painting::Rows,
        ExtractionConfig {
            include_document_structure: true,
            pages: Some(PageConfig {
                extract_pages: true,
                ..Default::default()
            }),
            ..Default::default()
        },
    );
    let pages = result.pages.expect("requested both pages");
    assert_eq!(pages.len(), 2);
    for page in pages {
        assert_eq!(nonempty_lines(&page.content), expected_plain());
    }
    let document = result.document.expect("requested document structure");
    for node in &document.nodes {
        assert!(
            node.attributes.as_ref().is_none_or(|attributes| {
                !attributes.contains_key("native_pdf_headerless_table")
                    && !attributes.contains_key("xberg:internal:native-pdf-headerless-table")
            }),
            "native headerless evidence is private"
        );
    }
    let grids: Vec<_> = document
        .nodes
        .iter()
        .filter_map(|node| match &node.content {
            NodeContent::Table { grid } => Some(grid),
            _ => None,
        })
        .collect();
    assert_eq!(grids.len(), 2);
    let rows = expected_rows();
    let expected: Vec<_> = rows
        .iter()
        .enumerate()
        .flat_map(|(row, cells)| {
            cells
                .iter()
                .enumerate()
                .map(move |(col, cell)| (row, col, cell.as_str(), false))
        })
        .collect();
    for grid in grids {
        assert_eq!((grid.rows, grid.cols), (10, 5));
        assert_eq!(
            grid.cells
                .iter()
                .map(|cell| (
                    cell.row as usize,
                    cell.col as usize,
                    cell.content.as_str(),
                    cell.is_header
                ))
                .collect::<Vec<_>>(),
            expected
        );
    }
}

#[test]
fn column_painting_and_independent_caption_keep_legacy_plain_order() {
    for painting in [Painting::Columns, Painting::Caption] {
        let page: Vec<_> = matches!(painting, Painting::Caption)
            .then_some(CAPTION.to_owned())
            .into_iter()
            .chain(LABELS.map(str::to_owned))
            .chain(expected_rows().into_iter().map(|row| row[1..].join(" ")))
            .collect();
        let result = extract(painting, ExtractionConfig::default());
        assert_eq!(
            nonempty_lines(&result.content),
            [page.as_slice(), page.as_slice()].concat()
        );
    }
}

#[test]
fn an_authored_header_stays_the_markdown_header() {
    let result = extract(
        Painting::AuthoredHeader,
        ExtractionConfig {
            output_format: OutputFormat::Markdown,
            ..Default::default()
        },
    );
    let header = "| Metric | FY 2021 | FY 2022 | FY 2023 | FY 2024 |";
    let body = expected_markdown();
    let mut page = vec![header];
    page.extend(body[1..].iter().map(String::as_str));
    assert_eq!(
        nonempty_lines(&result.content),
        [page.as_slice(), page.as_slice()].concat()
    );
    assert_eq!(result.tables.len(), 2);
    for table in result.tables {
        assert_eq!(nonempty_lines(&table.markdown), page);
    }
}
