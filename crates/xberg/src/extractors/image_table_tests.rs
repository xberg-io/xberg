use super::*;
use crate::core::config::{OcrConfig, OcrPipelineConfig, OcrPipelineStage, OcrQualityThresholds, OutputFormat};
use crate::plugins::{OcrBackend, OcrBackendType, Plugin};
use crate::types::document_structure::{AnnotationKind, DocumentNode, DocumentStructure, NodeContent, TextAnnotation};
use crate::types::internal::{ElementKind, InternalElement};
use crate::types::{BoundingBox, ExtractedDocument, Table, TableCellStyle};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

struct TableSidecarsBackend {
    name: &'static str,
    result: ExtractedDocument,
    invocations: Arc<AtomicUsize>,
}

impl Plugin for TableSidecarsBackend {
    fn name(&self) -> &str {
        self.name
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

#[async_trait::async_trait]
impl OcrBackend for TableSidecarsBackend {
    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Custom
    }
    fn supports_language(&self, _: &str) -> bool {
        true
    }
    async fn process_image(&self, _: &[u8], _: &OcrConfig) -> crate::Result<ExtractedDocument> {
        self.invocations.fetch_add(1, Ordering::SeqCst);
        Ok(self.result.clone())
    }
}

struct BackendRegistration(&'static str);

impl Drop for BackendRegistration {
    fn drop(&mut self) {
        crate::plugins::unregister_ocr_backend(self.0).expect("remove owned test backend");
    }
}

async fn extract_case(
    name: &'static str,
    result: ExtractedDocument,
    output_format: OutputFormat,
    thresholds: OcrQualityThresholds,
) -> InternalDocument {
    let invocations = Arc::new(AtomicUsize::new(0));
    crate::plugins::register_ocr_backend(Arc::new(TableSidecarsBackend {
        name,
        result,
        invocations: invocations.clone(),
    }))
    .unwrap();
    let _registration = BackendRegistration(name);
    let (config, pipeline) = case_config(name, output_format, thresholds);
    let mut png = std::io::Cursor::new(Vec::new());
    image::RgbImage::from_pixel(16, 16, image::Rgb([255, 255, 255]))
        .write_to(&mut png, image::ImageFormat::Png)
        .unwrap();
    let document = ImageExtractor::new()
        .extract_with_ocr_pipeline(&png.into_inner(), &config, &pipeline)
        .await
        .expect("deterministic image OCR pipeline");
    assert_eq!(invocations.load(Ordering::SeqCst), 1);
    document
}

fn case_config(
    name: &str,
    output_format: OutputFormat,
    thresholds: OcrQualityThresholds,
) -> (ExtractionConfig, OcrPipelineConfig) {
    let pipeline = OcrPipelineConfig {
        stages: vec![OcrPipelineStage {
            backend: name.into(),
            priority: 100,
            language: None,
            tesseract_config: None,
            paddle_ocr_config: None,
            paddle_ocr_settings: None,
            vlm_config: None,
            backend_options: None,
        }],
        quality_thresholds: thresholds,
    };
    let config = ExtractionConfig {
        use_cache: false,
        output_format,
        ocr: Some(OcrConfig {
            pipeline: Some(pipeline.clone()),
            ..Default::default()
        }),
        ..Default::default()
    };
    (config, pipeline)
}

fn accepting_thresholds() -> OcrQualityThresholds {
    OcrQualityThresholds {
        pipeline_min_quality: 0.0,
        ..Default::default()
    }
}

fn sidecar_tables() -> Vec<Table> {
    [
        (["Code", "Count", "Status"], ["ITEM2144", "17", "READY2144"], ""),
        (
            ["Other code", "Count", "Status"],
            ["SECOND2144", "23", "DONE2144"],
            "| Other code | Count | Status |\n| --- | --- | --- |\n| SECOND2144 | 23 | DONE2144 |",
        ),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, (header, row, markdown))| Table {
        cells: vec![header.map(str::to_owned).to_vec(), row.map(str::to_owned).to_vec()],
        markdown: markdown.into(),
        page_number: 1,
        table_id: Some(format!("backend-table-{index}")),
        columns: Some(header.map(str::to_owned).to_vec()),
        ..Default::default()
    })
    .collect()
}

fn backend_result(content: &str, tables: Vec<Table>) -> ExtractedDocument {
    ExtractedDocument {
        content: content.into(),
        tables,
        ..Default::default()
    }
}

fn derive_case(document: InternalDocument, expected_tables: usize) -> ExtractedDocument {
    let result = crate::extraction::derive::derive_extraction_result(document, true, OutputFormat::Markdown);
    assert_eq!(result.tables.len(), expected_tables);
    let pages = result.pages.as_ref().expect("standalone image page");
    assert_eq!(pages.len(), 1);
    assert_eq!(pages[0].tables.len(), expected_tables);
    assert_eq!(
        serde_json::to_value(&result.tables).unwrap(),
        serde_json::to_value(pages[0].tables.iter().map(Arc::as_ref).collect::<Vec<_>>()).unwrap(),
        "document and page must expose the same canonical table metadata"
    );
    result
}

fn assert_marker_count(result: &ExtractedDocument, marker: &str, expected: usize) {
    assert_eq!(result.content.matches(marker).count(), expected, "{}", result.content);
    let page = &result.pages.as_ref().expect("standalone image page")[0];
    assert_eq!(page.content.matches(marker).count(), expected, "{}", page.content);
}

#[tokio::test]
#[serial_test::serial]
async fn image_pipeline_preserves_cell_only_table_before_markdown_table() {
    let document = extract_case(
        "image-table-sidecars-2226",
        backend_result("Inventory report with two independent tables.", sidecar_tables()),
        OutputFormat::Markdown,
        accepting_thresholds(),
    )
    .await;
    assert_eq!(document.prebuilt_pages.as_ref().unwrap()[0].tables.len(), 2);
    let result = derive_case(document, 2);
    let expected = sidecar_tables();
    assert_eq!(
        result.tables.iter().map(|table| &table.cells).collect::<Vec<_>>(),
        expected.iter().map(|table| &table.cells).collect::<Vec<_>>()
    );
    for marker in ["ITEM2144", "SECOND2144"] {
        assert_marker_count(&result, marker, 1);
    }
}

#[tokio::test]
#[serial_test::serial]
async fn image_pipeline_renders_cell_only_table_without_prose() {
    let table = sidecar_tables().remove(0);
    let document = extract_case(
        "image-table-only-2226",
        backend_result("", vec![table.clone()]),
        OutputFormat::Markdown,
        accepting_thresholds(),
    )
    .await;
    let result = derive_case(document, 1);
    assert_eq!(result.tables[0].cells, table.cells);
    assert_eq!(result.tables[0].cells.iter().map(Vec::len).sum::<usize>(), 6);
    assert_marker_count(&result, "ITEM2144", 1);
}

#[tokio::test]
#[serial_test::serial]
async fn image_pipeline_uses_canonical_geometric_table_order() {
    let mut tables = sidecar_tables();
    tables[0].markdown = crate::rendering::common::render_table_markdown(&tables[0].cells);
    tables[0].bounding_box = Some(BoundingBox {
        x0: 1.0,
        y0: 1.0,
        x1: 15.0,
        y1: 4.0,
    });
    tables[1].bounding_box = Some(BoundingBox {
        x0: 1.0,
        y0: 10.0,
        x1: 15.0,
        y1: 14.0,
    });
    let expected = vec![tables[1].clone(), tables[0].clone()];
    let document = extract_case(
        "image-table-order-2226",
        backend_result("Inventory report.", tables),
        OutputFormat::Plain,
        accepting_thresholds(),
    )
    .await;
    let result = derive_case(document, 2);
    assert_eq!(
        serde_json::to_value(&result.tables).unwrap(),
        serde_json::to_value(&expected).unwrap()
    );
    for content in [&result.content, &result.pages.as_ref().unwrap()[0].content] {
        assert!(
            content.find("SECOND2144").unwrap() < content.find("ITEM2144").unwrap(),
            "{content}"
        );
    }
    for marker in ["ITEM2144", "SECOND2144"] {
        assert_marker_count(&result, marker, 1);
    }
}

#[tokio::test]
#[serial_test::serial]
async fn image_pipeline_preserves_equal_cells_with_distinct_metadata() {
    let mut first = sidecar_tables().remove(0);
    first.markdown = crate::rendering::common::render_table_markdown(&first.cells);
    first.bounding_box = Some(BoundingBox {
        x0: 1.0,
        y0: 10.0,
        x1: 15.0,
        y1: 14.0,
    });
    first.cell_styles = vec![TableCellStyle {
        row: 0,
        col: 0,
        heading_level: Some(2),
        style_name: Some("first".into()),
    }];
    let mut second = first.clone();
    second.table_id = Some("another-physical-table".into());
    second.bounding_box = Some(BoundingBox {
        x0: 1.0,
        y0: 1.0,
        x1: 15.0,
        y1: 4.0,
    });
    second.columns = Some(vec!["Alternate code".into(), "Units".into(), "State".into()]);
    second.cell_styles[0].style_name = Some("second".into());
    let expected = vec![first, second];
    let document = extract_case(
        "image-equal-tables-2226",
        backend_result("Independent equal grids.", expected.clone()),
        OutputFormat::Plain,
        accepting_thresholds(),
    )
    .await;
    let result = derive_case(document, 2);
    assert_eq!(
        serde_json::to_value(&result.tables).unwrap(),
        serde_json::to_value(&expected).unwrap()
    );
    assert_marker_count(&result, "ITEM2144", 2);
}

fn geometry_backed_result(tables: Vec<Table>) -> ExtractedDocument {
    let text = "Inventory commentary accompanies the financial schedule.";
    let mut document = InternalDocument::new("ocr");
    let mut element = InternalElement::text(
        ElementKind::OcrText {
            level: crate::types::ocr_elements::OcrElementLevel::Block,
        },
        text,
        0,
    );
    element.page = Some(1);
    element.bbox = Some(BoundingBox {
        x0: 1.0,
        y0: 1.0,
        x1: 15.0,
        y1: 2.0,
    });
    element.attributes = Some([("x_fsize".into(), "1".into())].into_iter().collect());
    document.push_element(element);
    ExtractedDocument {
        ocr_internal_document: Some(document),
        ..backend_result(text, tables)
    }
}

#[tokio::test]
#[serial_test::serial]
async fn image_pipeline_retains_canonical_normalized_financial_table() {
    let mut cells = vec![vec![
        "Security".into(),
        "".into(),
        "Par (000)".into(),
        "".into(),
        "Value".into(),
    ]];
    let mut expected: Vec<Vec<String>> = vec![vec!["Security".into(), "Par (000)".into(), "Value".into()]];
    for index in 0..12 {
        cells.push(vec![
            format!("Bond {index}"),
            if index == 0 { "USD".into() } else { "".into() },
            format!("{},000", index + 1),
            if index == 0 { "$".into() } else { "".into() },
            format!("{},500", index + 1),
        ]);
        expected.push(vec![
            format!("Bond {index}"),
            format!("{}{},000", if index == 0 { "USD " } else { "" }, index + 1),
            format!("{}{},500", if index == 0 { "$ " } else { "" }, index + 1),
        ]);
    }
    let table = Table {
        markdown: crate::rendering::common::render_table_markdown(&cells),
        cells,
        page_number: 1,
        table_id: Some("raw-financial-table".into()),
        ..Default::default()
    };
    assert_eq!(table.cells[0].len(), 5);
    let document = extract_case(
        "image-normalized-table-2226",
        geometry_backed_result(vec![table]),
        OutputFormat::Markdown,
        accepting_thresholds(),
    )
    .await;
    let result = derive_case(document, 1);
    assert_eq!(result.tables[0].cells, expected);
    assert_eq!(result.tables[0].columns.as_ref(), expected.first());
    assert_eq!(result.tables[0].table_id.as_deref(), Some("table-1"));
    assert_marker_count(&result, "Bond 11", 1);
}

#[tokio::test]
#[serial_test::serial]
async fn image_pipeline_retains_canonical_stitched_table() {
    let header: Vec<Vec<String>> = vec![vec!["Code".into(), "Count".into()]];
    let data: Vec<Vec<String>> = vec![vec!["STITCHED2226".into(), "17".into()]];
    let fragments = [
        (header.clone(), header.first().cloned(), 6.0, 8.0),
        (data.clone(), None, 3.0, 5.0),
    ]
    .into_iter()
    .map(|(cells, columns, y0, y1)| Table {
        markdown: crate::rendering::common::render_table_markdown(&cells),
        columns,
        cells,
        page_number: 1,
        bounding_box: Some(BoundingBox {
            x0: 1.0,
            y0,
            x1: 15.0,
            y1,
        }),
        ..Default::default()
    })
    .collect::<Vec<_>>();
    assert_eq!(fragments.len(), 2);
    let document = extract_case(
        "image-stitched-table-2226",
        geometry_backed_result(fragments),
        OutputFormat::Markdown,
        accepting_thresholds(),
    )
    .await;
    let result = derive_case(document, 1);
    assert_eq!(result.tables[0].cells, vec![header[0].clone(), data[0].clone()]);
    assert_eq!(result.tables[0].columns.as_ref(), header.first());
    assert_eq!(result.tables[0].table_id.as_deref(), Some("table-1"));
    assert_eq!(
        result.tables[0].bounding_box,
        Some(BoundingBox {
            x0: 1.0,
            y0: 3.0,
            x1: 15.0,
            y1: 8.0
        })
    );
    assert_marker_count(&result, "STITCHED2226", 1);
}

fn public_node(content: NodeContent) -> DocumentNode {
    DocumentNode {
        id: String::new(),
        content,
        parent: None,
        children: vec![],
        content_layer: Default::default(),
        page: Some(1),
        page_end: None,
        bbox: None,
        annotations: vec![],
        attributes: None,
    }
}

#[tokio::test]
#[serial_test::serial]
async fn image_pipeline_honors_authoritative_public_nodes() {
    let text = "Authoritative inventory commentary.";
    let annotation = TextAnnotation {
        start: 0,
        end: 13,
        kind: AnnotationKind::Bold,
    };
    let bbox = BoundingBox {
        x0: 1.0,
        y0: 3.0,
        x1: 15.0,
        y1: 5.0,
    };
    let mut structure = DocumentStructure {
        nodes: vec![
            public_node(NodeContent::Heading {
                level: 2,
                text: "INVENTORY2226".into(),
            }),
            public_node(NodeContent::Paragraph { text: text.into() }),
        ],
        source_format: Some("ocr".into()),
        relationships: vec![],
        node_types: vec![],
    };
    structure.nodes[1].bbox = Some(bbox);
    structure.nodes[1].annotations = vec![annotation.clone()];
    structure.nodes[1].attributes = Some([("class".into(), "inventory-note".into())].into_iter().collect());
    structure.finalize_node_types();
    structure.validate().expect("valid flat public OCR contribution");
    let backend = ExtractedDocument {
        content: format!("INVENTORY2226\n{text}"),
        document: Some(structure),
        ..Default::default()
    };
    let document = extract_case(
        "image-authoritative-nodes-2226",
        backend,
        OutputFormat::Markdown,
        accepting_thresholds(),
    )
    .await;
    let result = derive_case(document, 0);
    let nodes = &result.document.as_ref().unwrap().nodes;
    let paragraphs = nodes
        .iter()
        .filter(|node| matches!(&node.content, NodeContent::Paragraph { text: value } if value == text))
        .collect::<Vec<_>>();
    assert_eq!(paragraphs.len(), 1);
    assert_eq!(paragraphs[0].annotations, vec![annotation]);
    assert_eq!(paragraphs[0].bbox, Some(bbox));
    assert_eq!(
        paragraphs[0]
            .attributes
            .as_ref()
            .unwrap()
            .get("class")
            .map(String::as_str),
        Some("inventory-note")
    );
    assert_eq!(
        nodes
            .iter()
            .filter(|node| matches!(&node.content, NodeContent::Heading { level: 2, text } if text == "INVENTORY2226"))
            .count(),
        1
    );
    assert_marker_count(&result, "INVENTORY2226", 1);
    assert_marker_count(&result, "inventory commentary.", 1);
}

#[tokio::test]
#[serial_test::serial]
async fn image_pipeline_does_not_restore_rejected_table_payload() {
    let thresholds = OcrQualityThresholds {
        discard_suspected_ocr_noise: true,
        min_words_for_ocr_output_check: 1,
        max_ocr_output_fragmented_word_ratio: 0.0,
        ..accepting_thresholds()
    };
    let document = extract_case(
        "image-rejected-table-2226",
        backend_result("Rejected inventory prose.", sidecar_tables()),
        OutputFormat::Markdown,
        thresholds,
    )
    .await;
    assert_eq!(document.tables.len(), 0);
    let result = crate::extraction::derive::derive_extraction_result(document, true, OutputFormat::Markdown);
    assert_eq!(result.tables.len(), 0);
    assert_eq!(result.content.trim(), "");
    assert_eq!(
        result
            .processing_warnings
            .iter()
            .filter(|warning| warning.message.contains("discard_suspected_ocr_noise"))
            .count(),
        1
    );
    assert!(
        result.pages.is_none(),
        "a rejected standalone image has no accepted page content"
    );
}

#[tokio::test]
#[serial_test::serial]
async fn image_pipeline_does_not_create_nodes_for_empty_tables() {
    let document = extract_case(
        "image-empty-table-2226",
        backend_result(
            "Inventory prose without table cells.",
            vec![Table {
                page_number: 1,
                ..Default::default()
            }],
        ),
        OutputFormat::Markdown,
        accepting_thresholds(),
    )
    .await;
    let result = derive_case(document, 0);
    assert_eq!(
        result
            .document
            .as_ref()
            .unwrap()
            .nodes
            .iter()
            .filter(|node| matches!(node.content, NodeContent::Table { .. }))
            .count(),
        0
    );
    assert_marker_count(&result, "Inventory prose without table cells.", 1);
}

fn single_page_pdf() -> Vec<u8> {
    let objects = [
        "<</Type /Catalog /Pages 2 0 R>>",
        "<</Type /Pages /Kids [3 0 R] /Count 1>>",
        "<</Type /Page /MediaBox [0 0 16 16] /Parent 2 0 R>>",
    ];
    let mut pdf = "%PDF-1.4\n".to_owned();
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.push_str(&format!("{} 0 obj\n{object}\nendobj\n", index + 1));
    }
    let xref = pdf.len();
    pdf.push_str("xref\n0 4\n0000000000 65535 f \n");
    for offset in offsets {
        pdf.push_str(&format!("{offset:010} 00000 n \n"));
    }
    pdf.push_str(&format!("trailer\n<</Size 4 /Root 1 0 R>>\nstartxref\n{xref}\n%%EOF\n"));
    pdf.into_bytes()
}

#[tokio::test]
#[serial_test::serial]
async fn pdf_pipeline_preserves_cell_only_sidecars_without_image_canonicalization() {
    let name = "pdf-cell-only-control-2226";
    let invocations = Arc::new(AtomicUsize::new(0));
    let table = sidecar_tables().remove(0);
    crate::plugins::register_ocr_backend(Arc::new(TableSidecarsBackend {
        name,
        result: backend_result("", vec![table.clone()]),
        invocations: invocations.clone(),
    }))
    .unwrap();
    let _registration = BackendRegistration(name);
    let (config, pipeline) = case_config(name, OutputFormat::Plain, accepting_thresholds());
    let pdf = single_page_pdf();
    let parsed = xberg_native_pdf::PdfDocument::from_bytes(pdf.clone()).expect("valid synthetic PDF");
    assert_eq!(parsed.page_count().unwrap(), 1);
    let images = [image::DynamicImage::new_rgb8(16, 16)];
    let (text, tables, _, document, _, _, _, _, _, _) = crate::extractors::pdf::ocr::run_ocr_pipeline(
        Some(&pdf),
        Some(&images),
        #[cfg(feature = "layout-detection")]
        None,
        &config,
        &pipeline,
        None,
        None,
    )
    .await
    .expect("PDF-backed pipeline preserves its existing table handoff");
    assert_eq!(invocations.load(Ordering::SeqCst), 1);
    assert_eq!(text, "");
    assert_eq!(tables.len(), 1);
    assert_eq!(
        serde_json::to_value(&tables[0]).unwrap(),
        serde_json::to_value(&table).unwrap()
    );
    assert!(
        document.is_none(),
        "a PDF-backed cell-only sidecar must not force image assembly"
    );
}
