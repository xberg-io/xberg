#![cfg(feature = "pdf")]

mod helpers;

use flate2::read::GzDecoder;
use helpers::extract_bytes_document_blocking;
use std::io::Read;
use xberg::{ExtractedDocument, ExtractionConfig, OutputFormat};

const TOP: [[&str; 4]; 4] = [
    ["Service", "Zone", "Period", "Capacity"],
    ["Iris", "North", "Winter", "350"],
    ["Hazel", "East", "Autumn", "470"],
    ["Linden", "South", "Summer", "590"],
];
const BOTTOM: [[&str; 2]; 5] = [
    ["Review", "Cadence"],
    ["Clock", "Week one"],
    ["Signal", "Week two"],
    ["Meter", "Week three"],
    ["Beacon", "Week four"],
];

fn extract(compressed_pdf: &[u8], output_format: OutputFormat) -> ExtractedDocument {
    let mut pdf = Vec::new();
    GzDecoder::new(compressed_pdf).read_to_end(&mut pdf).unwrap();
    let config = ExtractionConfig {
        disable_ocr: true,
        use_cache: false,
        enable_quality_processing: false,
        output_format,
        ..Default::default()
    };
    extract_bytes_document_blocking(&pdf, "application/pdf", &config).unwrap()
}

#[test]
fn adjacent_ruled_tables_follow_page_order_in_plain_and_markdown() {
    let fixtures: [&[u8]; 2] = [
        include_bytes!("fixtures/pdf_adjacent_ruled_tables_3286/two-grids.pdf.gz"),
        include_bytes!("fixtures/pdf_adjacent_ruled_tables_3286/reverse-paint.pdf.gz"),
    ];
    for pdf in fixtures {
        for format in [OutputFormat::Plain, OutputFormat::Markdown] {
            let markdown = format == OutputFormat::Markdown;
            let result = extract(pdf, format);
            let [upper, lower] = result.tables.as_slice() else {
                panic!("expected two separate source tables, got {}", result.tables.len());
            };
            assert_eq!(upper.cells, TOP);
            assert_eq!(lower.cells, BOTTOM);
            assert!(upper.bounding_box.unwrap().y0 > lower.bounding_box.unwrap().y1);
            if markdown {
                let upper = result.content.find("| Service | Zone | Period | Capacity |").unwrap();
                let lower = result.content.find("| Review | Cadence |").unwrap();
                assert!(upper < lower, "the physically higher table renders first");
                assert_eq!(result.content.matches("\n| --- | --- |").count(), 2);
            }
        }
    }
}

#[test]
fn continuous_ruled_table_remains_one_grid() {
    let bytes = include_bytes!("fixtures/pdf_adjacent_ruled_tables_3286/continuous-grid.pdf.gz");
    for format in [OutputFormat::Plain, OutputFormat::Markdown] {
        let result = extract(bytes, format);
        assert_eq!(result.tables.len(), 1);
        let cells = &result.tables[0].cells;
        assert_eq!(cells.len(), 6);
        assert_eq!(cells[..4], TOP);
        assert_eq!(cells[4], ["Juniper", "West", "Spring", "610"]);
        assert_eq!(cells[5], ["Aster", "Central", "Year", "730"]);
    }
}
