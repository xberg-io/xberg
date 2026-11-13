#![cfg(all(feature = "pdf", feature = "ocr", feature = "tokio-runtime"))]

mod helpers;

use async_trait::async_trait;
use lopdf::{Document, Stream, dictionary};
use std::sync::Arc;
use xberg::core::config::{ExtractionConfig, OcrConfig, OutputFormat, PageConfig};
use xberg::plugins::{OcrBackend, OcrBackendType, Plugin, register_ocr_backend, unregister_ocr_backend};
use xberg::types::{ExtractedDocument, ExtractionMethod, Table};

const BACKEND: &str = "declined-plain-table-3145";
const PROSE: &str = "The accepted paragraph includes **literal** marks and a | pipe.";
const TABLE: &str = "| Region | Total |\n| --- | --- |\n| North | 10 |\n";

struct TableRead;

impl Plugin for TableRead {
    fn name(&self) -> &str {
        BACKEND
    }

    fn version(&self) -> String {
        "1.0.0".into()
    }

    fn initialize(&self) -> xberg::Result<()> {
        Ok(())
    }

    fn shutdown(&self) -> xberg::Result<()> {
        Ok(())
    }
}

#[async_trait]
impl OcrBackend for TableRead {
    async fn process_image(&self, image: &[u8], _: &OcrConfig) -> xberg::Result<ExtractedDocument> {
        assert!(image.starts_with(b"\x89PNG\r\n\x1a\n"));
        let mut read = ExtractedDocument::default();
        read.content = format!("{PROSE}\n\n{TABLE}\nEvery accepted word must survive.");
        read.tables = vec![Table {
            cells: vec![vec!["Region".into(), "Total".into()], vec!["North".into(), "10".into()]],
            markdown: TABLE.into(),
            page_number: 1,
            ..Default::default()
        }];
        Ok(read)
    }

    fn supports_language(&self, _: &str) -> bool {
        true
    }

    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Custom
    }
}

fn synthetic_pdf() -> Vec<u8> {
    let mut pdf = Document::with_version("1.7");
    let pages = pdf.new_object_id();
    let font = pdf.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica",
    });
    let native = pdf.add_object(Stream::new(
        dictionary! {},
        b"BT /F1 12 Tf 24 160 Td (This first page has a complete native sentence.) Tj ET".to_vec(),
    ));
    let scan = pdf.add_object(Stream::new(dictionary! {}, b"0.5 g 25 25 150 150 re f".to_vec()));
    let children: [lopdf::Object; 2] = [native, scan].map(|contents| {
        pdf.add_object(dictionary! {
            "Type" => "Page", "Parent" => pages, "MediaBox" => vec![0.into(), 0.into(), 300.into(), 200.into()],
            "Contents" => contents, "Resources" => dictionary! {"Font" => dictionary! {"F1" => font}},
        })
        .into()
    });
    pdf.set_object(
        pages,
        dictionary! {"Type" => "Pages", "Kids" => children.to_vec(), "Count" => 2},
    );
    let root = pdf.add_object(dictionary! {"Type" => "Catalog", "Pages" => pages});
    pdf.trailer.set("Root", root);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    bytes
}

#[test]
fn declined_mixed_plain_keeps_accepted_prose_and_renders_table_rows() {
    register_ocr_backend(Arc::new(TableRead)).unwrap();
    let config = ExtractionConfig {
        use_cache: false,
        enable_quality_processing: false,
        output_format: OutputFormat::Plain,
        force_ocr_pages: Some(vec![2]),
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ocr: Some(OcrConfig {
            backend: BACKEND.into(),
            ..Default::default()
        }),
        ..Default::default()
    };
    let result = helpers::extract_bytes_document_blocking(&synthetic_pdf(), "application/pdf", &config);
    unregister_ocr_backend(BACKEND).unwrap();
    let result = result.unwrap();
    assert_eq!(result.extraction_method, Some(ExtractionMethod::Mixed));
    let pages = result.pages.as_ref().unwrap();
    assert_eq!(pages.len(), 2);
    assert_eq!(
        pages[0].content.trim(),
        "This first page has a complete native sentence."
    );
    let expected = format!("{PROSE}\n\nRegion Total\nNorth 10\n\nEvery accepted word must survive.");
    assert_eq!(pages[1].content.trim(), expected);
    assert!(result.content.contains(&expected));
}
