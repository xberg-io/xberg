/// #2312: a pruned page hands back its unpruned text beside the pruned one, so pipeline stage
/// scoring judges the text the backend produced and pruning cannot change stage selection.
#[cfg(feature = "ocr")]
#[tokio::test]
#[serial_test::serial]
async fn pruned_page_returns_its_unpruned_text_for_stage_scoring() {
    use crate::core::config::{OcrConfig, OcrQualityThresholds};
    use crate::plugins::{ConfidenceSemantics, OcrBackend, OcrBackendType, Plugin};
    use crate::types::ExtractedDocument;
    use std::sync::Arc;

    const BACKEND_NAME: &str = "symbol-noise-pruning-test-backend";
    const REAL_LINE: &str = "Quarterly maintenance schedule";

    fn noisy_content() -> String {
        let fragments = ["a", "oe", "ae", "ee", "rs", "ie", "~", "=|"];
        let mut lines: Vec<&str> = vec![REAL_LINE];
        lines.extend(fragments.iter().cycle().take(30));
        lines.join("\n")
    }

    struct NoisyBackend;

    #[async_trait::async_trait]
    impl OcrBackend for NoisyBackend {
        fn backend_type(&self) -> OcrBackendType {
            OcrBackendType::Custom
        }
        fn supports_language(&self, _: &str) -> bool {
            true
        }
        async fn process_image(&self, _: &[u8], _: &OcrConfig) -> crate::Result<ExtractedDocument> {
            Ok(ExtractedDocument {
                content: noisy_content(),
                ..Default::default()
            })
        }
        fn confidence_semantics(&self) -> ConfidenceSemantics {
            ConfidenceSemantics::Uncalibrated
        }
    }

    impl Plugin for NoisyBackend {
        fn name(&self) -> &str {
            BACKEND_NAME
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

    crate::plugins::register_ocr_backend(Arc::new(NoisyBackend)).unwrap();
    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: BACKEND_NAME.to_string(),
            quality_thresholds: Some(OcrQualityThresholds::default()),
            ..Default::default()
        }),
        ..Default::default()
    };
    let images = [image::DynamicImage::new_rgb8(100, 100)];
    let result = extract_with_ocr_for_page(
        None,
        Some(&images),
        #[cfg(feature = "layout-detection")]
        None,
        &config,
        None,
        0,
        false,
        None,
        None,
        0,
        None,
    )
    .await
    .unwrap();
    crate::plugins::unregister_ocr_backend(BACKEND_NAME).unwrap();

    assert_eq!(result.0, REAL_LINE);
    let unpruned = result
        .13
        .unpruned_text
        .expect("a pruned page records its unpruned text");
    assert_eq!(unpruned, noisy_content());
    let thresholds = OcrQualityThresholds::default();
    assert!(
        compute_quality_score(&unpruned, &thresholds) < compute_quality_score(&result.0, &thresholds),
        "pruning raises the text score, which is why stage scoring must read the unpruned text"
    );
}

/// #2312: the single-page first pass is scored on the page text before symbol-noise pruning, so
/// pruning cannot lift a noisy page over `pipeline_min_quality` and change the route.
#[test]
fn first_pass_gate_scores_the_unpruned_text() {
    let thresholds = OcrQualityThresholds {
        pipeline_min_quality: 0.85,
        ..OcrQualityThresholds::default()
    };
    let noise: Vec<&str> = ["a", "oe", "ae", "ee", "rs", "ie", "~", "=|"]
        .into_iter()
        .cycle()
        .take(200)
        .collect();
    let pruned = "The committee met on Tuesday to review the annual budget and agreed that the library needs a new \
        roof before winter. The treasurer will publish the final figures next month.";
    let unpruned = format!("{pruned}\n{}", noise.join("\n"));
    assert!(
        compute_quality_score(pruned, &thresholds) >= thresholds.pipeline_min_quality
            && compute_quality_score(&unpruned, &thresholds) < thresholds.pipeline_min_quality,
        "the fixture must straddle the gate"
    );
    assert!(!first_pass_clears_quality_gate(
        pruned,
        Some(&unpruned),
        None,
        &thresholds
    ));
    assert!(first_pass_clears_quality_gate(pruned, None, None, &thresholds));
}
