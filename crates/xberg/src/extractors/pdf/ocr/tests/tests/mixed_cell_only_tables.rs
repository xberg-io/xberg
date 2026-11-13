use super::{build_minimal_two_page_pdf, native_two_page_document, pdf_config_with_disabled_page_margins};
use crate::core::config::{ExtractionConfig, OcrConfig, OutputFormat};
use crate::extractors::pdf::ocr::document::{
    flat_ocr_page_document, merge_structured_ocr_pages_into_internal_document,
};
use crate::extractors::pdf::ocr::pipeline::extract_mixed_ocr_native;
use crate::plugins::{OcrBackend, OcrBackendType, Plugin};
use crate::types::{ExtractedDocument, PageBoundary, Table};
use std::sync::Arc;

const BACKEND: &str = "mixed-cell-only-table";
const READ: &str = "An ordinary OCR paragraph with enough distinct words to keep the page.";
const CELL: &str = "OCR-only inventory";

struct CellOnlyTableBackend;

#[async_trait::async_trait]
impl OcrBackend for CellOnlyTableBackend {
    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Custom
    }

    fn supports_language(&self, _: &str) -> bool {
        true
    }

    async fn process_image(&self, _: &[u8], _: &OcrConfig) -> crate::Result<ExtractedDocument> {
        Ok(ExtractedDocument {
            content: READ.into(),
            ocr_internal_document: Some(flat_ocr_page_document(READ)),
            tables: vec![Table {
                cells: vec![vec![CELL.into(), "42".into()]],
                ..Default::default()
            }],
            ..Default::default()
        })
    }

    fn supports_document_processing(&self) -> bool {
        false
    }
}

impl Plugin for CellOnlyTableBackend {
    fn name(&self) -> &str {
        BACKEND
    }

    fn version(&self) -> String {
        "1.0.0".into()
    }

    fn initialize(&self) -> crate::Result<()> {
        Ok(())
    }

    fn shutdown(&self) -> crate::Result<()> {
        Ok(())
    }
}

#[tokio::test]
#[serial_test::serial]
async fn mixed_ocr_retains_cell_only_sidecar_tables_in_page_output() {
    crate::plugins::register_ocr_backend(Arc::new(CellOnlyTableBackend)).unwrap();
    let pdf = build_minimal_two_page_pdf(612.0, 792.0);
    let native = "native page one\nnative page two";
    let split = native.find('\n').unwrap();
    let bounds = [
        PageBoundary {
            page_number: 1,
            byte_start: 0,
            byte_end: split,
        },
        PageBoundary {
            page_number: 2,
            byte_start: split + 1,
            byte_end: native.len(),
        },
    ];
    let mut outputs = Vec::new();
    for output_format in [OutputFormat::Plain, OutputFormat::Markdown] {
        let config = ExtractionConfig {
            ocr: Some(OcrConfig {
                backend: BACKEND.into(),
                ..Default::default()
            }),
            output_format: output_format.clone(),
            pdf_options: Some(pdf_config_with_disabled_page_margins()),
            ..Default::default()
        };
        outputs.push((
            output_format,
            extract_mixed_ocr_native(native, &bounds, &[2], &pdf, &config, None).await,
        ));
    }
    crate::plugins::unregister_ocr_backend(BACKEND).unwrap();
    for (format, output) in outputs {
        let (_, replacements, pages, ..) = output.unwrap();
        assert!(!replacements.contains_key(&1));
        assert_eq!(replacements.get(&2).map(String::as_str), Some(READ));
        let mut document = native_two_page_document();
        merge_structured_ocr_pages_into_internal_document(&mut document, &replacements, &pages);
        let [table] = document.tables.as_slice() else {
            panic!(
                "{format:?} must keep one cell-only sidecar table, got {:?}",
                document.tables
            );
        };
        assert_eq!(table.page_number, 2);
        assert_eq!(table.cells, [[CELL, "42"]]);
        let rendered = if format == OutputFormat::Plain {
            crate::rendering::render_plain(&document)
        } else {
            crate::rendering::render_markdown(&document)
        };
        assert_eq!(rendered.matches(CELL).count(), 1, "{format:?}: {rendered}");
        assert!(rendered.contains("native page one"), "{format:?}: {rendered}");
    }
}
