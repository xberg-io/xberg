use super::{
    ExtractionConfig, PageOcrHints, extract_with_ocr_with_page_hints, pre_rendered_page_source_dpi,
    render_selected_pages_for_ocr, rotated_landscape_pdf,
};

fn run_on_ocr_test_stack<F: std::future::Future<Output = ()>>(test: impl FnOnce() -> F + Send + 'static) {
    const OCR_TEST_STACK_BYTES: usize = 8 * 1024 * 1024;
    std::thread::Builder::new()
        .stack_size(OCR_TEST_STACK_BYTES)
        .spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("build layout OCR test runtime")
                .block_on(test())
        })
        .expect("spawn layout OCR test thread")
        .join()
        .unwrap_or_else(|panic| std::panic::resume_unwind(panic));
}

#[test]
#[serial_test::serial]
fn images_driven_ocr_uses_the_certified_page_frame_and_keeps_the_rotation_hint() {
    run_on_ocr_test_stack(check_images_driven_ocr_page_frame);
}

async fn check_images_driven_ocr_page_frame() {
    use crate::core::config::OcrConfig;
    use crate::plugins::{OcrBackend, OcrBackendType, Plugin};
    use crate::types::{BoundingBox, ExtractedDocument, Table};
    use std::sync::Mutex;

    const UPRIGHT_TABLE_PIXELS: BoundingBox = BoundingBox {
        x0: 120.0,
        y0: 80.0,
        x1: 310.0,
        y1: 180.0,
    };

    struct RotationCapturingBackend {
        captured_backend_options: Mutex<Vec<Option<serde_json::Value>>>,
    }

    #[async_trait::async_trait]
    impl OcrBackend for RotationCapturingBackend {
        fn backend_type(&self) -> OcrBackendType {
            OcrBackendType::Custom
        }
        fn supports_language(&self, _: &str) -> bool {
            true
        }
        async fn process_image(&self, _: &[u8], config: &OcrConfig) -> crate::Result<ExtractedDocument> {
            self.captured_backend_options
                .lock()
                .unwrap()
                .push(config.backend_options.clone());
            Ok(ExtractedDocument {
                content: "text".to_string(),
                metadata: crate::types::Metadata {
                    additional: [
                        ("ocr_processed_image_width".into(), serde_json::json!(800)),
                        ("ocr_processed_image_height".into(), serde_json::json!(500)),
                    ]
                    .into_iter()
                    .collect(),
                    ..Default::default()
                },
                tables: vec![Table {
                    bounding_box: Some(UPRIGHT_TABLE_PIXELS),
                    ..Default::default()
                }],
                ..Default::default()
            })
        }
    }

    impl Plugin for RotationCapturingBackend {
        fn name(&self) -> &str {
            "rotation-capturing-mock"
        }
        fn version(&self) -> String {
            "1.0.0".to_string()
        }
        fn initialize(&self) -> crate::Result<()> {
            Ok(())
        }
        fn shutdown(&self) -> crate::Result<()> {
            Ok(())
        }
    }

    let backend = std::sync::Arc::new(RotationCapturingBackend {
        captured_backend_options: Mutex::new(Vec::new()),
    });
    crate::plugins::register_ocr_backend(backend.clone()).unwrap();

    let content = rotated_landscape_pdf(270);
    let rendered = render_selected_pages_for_ocr(&content, &[0]).expect("rotated fixture page must render for OCR");
    let images: Vec<image::DynamicImage> = rendered.into_iter().map(|(_, image)| image).collect();

    assert!(pre_rendered_page_source_dpi((200.0, 100.0), images[0].width(), images[0].height()).is_some());

    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "rotation-capturing-mock".to_string(),
            ..Default::default()
        }),
        ..Default::default()
    };

    let table_pixels = BoundingBox {
        x0: 320.0,
        y0: 120.0,
        x1: 420.0,
        y1: 310.0,
    };
    let table_points = BoundingBox {
        x0: 128.0,
        y0: 61.25,
        x1: 168.0,
        y1: 85.0,
    };
    for (layout_rasters_in_page_frame, expected) in [(false, table_pixels), (true, table_points)] {
        let (_, _, tables, ..) = Box::pin(extract_with_ocr_with_page_hints(
            Some(&content),
            Some(&images),
            #[cfg(feature = "layout-detection")]
            None,
            &config,
            None,
            Some(PageOcrHints {
                layout_rasters_in_page_frame: Some([layout_rasters_in_page_frame].into()),
                ..Default::default()
            }),
        ))
        .await
        .expect("extract_with_ocr should succeed");
        assert_eq!(tables.len(), 1);
        assert_eq!(
            tables[0].bounding_box,
            Some(expected),
            "layout raster MediaBox frame certified: {layout_rasters_in_page_frame}"
        );
    }

    crate::plugins::unregister_ocr_backend("rotation-capturing-mock").unwrap();

    let captured = backend.captured_backend_options.lock().unwrap();
    assert_eq!(captured.len(), 2);
    for options in captured.iter() {
        let rotation_hint = options
            .as_ref()
            .and_then(|opts| opts.get("page_rotation_degrees"))
            .and_then(|v| v.as_u64());
        assert_eq!(rotation_hint, Some(270), "backend options: {options:?}");
    }
}

#[cfg(feature = "layout-detection")]
mod handoff {
    use super::{ExtractionConfig, rotated_landscape_pdf, run_on_ocr_test_stack};
    use crate::core::config::layout::TableModel;
    use crate::core::config::{LayoutDetectionConfig, OcrConfig, OutputFormat};
    use crate::core::layout_pages::LayoutPageSelection;
    use crate::extractors::pdf::{
        PageOcrLayoutContext, layout_runner::TEST_SELECTED_PAGE_DETECTIONS, run_ocr_with_layout_pages,
    };
    use crate::layout::DetectionResult;
    use crate::plugins::{OcrBackend, OcrBackendType, Plugin};
    use crate::types::internal::{ElementKind, InternalDocument, InternalElement};
    use crate::types::ocr_elements::OcrElementLevel;
    use crate::types::{BoundingBox, ExtractedDocument, Table};

    const ABOVE: &str = "The introduction comes before the table.";
    const INSIDE: &str = "Recognized balance carried by the table";
    const BELOW: &str = "The conclusion comes after the table.";

    struct TableAndProseBackend;

    impl Plugin for TableAndProseBackend {
        fn name(&self) -> &str {
            "layout-page-frame-table-and-prose"
        }

        fn version(&self) -> String {
            "1.0.0".to_owned()
        }

        fn initialize(&self) -> crate::Result<()> {
            Ok(())
        }

        fn shutdown(&self) -> crate::Result<()> {
            Ok(())
        }
    }

    #[async_trait::async_trait]
    impl OcrBackend for TableAndProseBackend {
        fn backend_type(&self) -> OcrBackendType {
            OcrBackendType::Custom
        }

        fn supports_language(&self, _: &str) -> bool {
            true
        }

        async fn process_image(&self, _: &[u8], _: &OcrConfig) -> crate::Result<ExtractedDocument> {
            let mut document = InternalDocument::new("pdf");
            for (text, top) in [(ABOVE, 40.0), (INSIDE, 160.0), (BELOW, 280.0)] {
                let mut element = InternalElement::text(
                    ElementKind::OcrText {
                        level: OcrElementLevel::Block,
                    },
                    text,
                    0,
                );
                element.bbox = Some(BoundingBox {
                    x0: 120.0,
                    y0: top,
                    x1: 550.0,
                    y1: top + 20.0,
                });
                document.push_element(element);
            }

            Ok(ExtractedDocument {
                content: [ABOVE, INSIDE, BELOW].join("\n"),
                metadata: crate::types::Metadata {
                    additional: [
                        ("ocr_processed_image_width".into(), serde_json::json!(800)),
                        ("ocr_processed_image_height".into(), serde_json::json!(400)),
                    ]
                    .into_iter()
                    .collect(),
                    ..Default::default()
                },
                tables: vec![Table {
                    bounding_box: Some(BoundingBox {
                        x0: 100.0,
                        y0: 140.0,
                        x1: 700.0,
                        y1: 260.0,
                    }),
                    markdown: format!("| Balance |\n| --- |\n| {INSIDE} |"),
                    cells: vec![vec!["Balance".to_owned()], vec![INSIDE.to_owned()]],
                    ..Default::default()
                }],
                ocr_internal_document: Some(document),
                ..Default::default()
            })
        }
    }

    #[test]
    #[serial_test::serial]
    fn layout_handoff_places_the_table_between_prose_and_suppresses_its_duplicate_text() {
        run_on_ocr_test_stack(check_layout_handoff);
    }

    async fn check_layout_handoff() {
        let backend = TableAndProseBackend;
        let backend_name = backend.name().to_owned();
        crate::plugins::register_ocr_backend(std::sync::Arc::new(backend)).unwrap();
        let config = ExtractionConfig {
            force_ocr: true,
            output_format: OutputFormat::Plain,
            layout: Some(LayoutDetectionConfig {
                table_model: TableModel::Disabled,
                ..Default::default()
            }),
            ocr: Some(OcrConfig {
                backend: backend_name.clone(),
                ..Default::default()
            }),
            ..Default::default()
        };
        let content = rotated_landscape_pdf(0);
        let selection = LayoutPageSelection::new(vec![1]);
        let result = TEST_SELECTED_PAGE_DETECTIONS
            .scope(
                DetectionResult::new(600, 300, Vec::new()),
                Box::pin(run_ocr_with_layout_pages(
                    &content,
                    &config,
                    None,
                    PageOcrLayoutContext {
                        pages: Some(&selection),
                        hints: None,
                        rendered_layout_pages: Vec::new(),
                    },
                    None,
                    None,
                    None,
                )),
            )
            .await;
        crate::plugins::unregister_ocr_backend(&backend_name).unwrap();

        let (_, tables, _, document, ..) = result.expect("layout OCR must succeed");
        assert_eq!(tables.len(), 1);
        assert_eq!(
            tables[0].bounding_box,
            Some(BoundingBox {
                x0: 25.0,
                y0: 35.0,
                x1: 175.0,
                y1: 65.0
            })
        );
        let document = document.expect("layout OCR must assemble a document");
        let output = crate::rendering::render_plain(&document);
        let above = output.find(ABOVE).expect("prose above the table");
        let table = output.find("Balance").expect("the table header");
        let below = output.find(BELOW).expect("prose below the table");
        assert!(above < table && table < below, "{output}");
        assert_eq!(output.matches(INSIDE).count(), 1, "{output}");
    }
}
