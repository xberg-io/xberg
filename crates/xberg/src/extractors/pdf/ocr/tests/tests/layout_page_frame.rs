use super::{
    ExtractionConfig, PageOcrHints, extract_with_ocr_with_page_hints, pre_rendered_page_source_dpi,
    render_selected_pages_for_ocr, rotated_landscape_pdf,
};

#[tokio::test]
#[serial_test::serial]
async fn images_driven_ocr_uses_the_certified_page_frame_and_keeps_the_rotation_hint() {
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
