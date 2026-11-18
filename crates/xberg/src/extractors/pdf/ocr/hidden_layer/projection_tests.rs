#[test]
fn omitted_and_explicit_zero_margins_preserve_hidden_layer_projection() {
    let mut config = crate::ExtractionConfig::default();
    assert!(super::super::preserves_requested_projection(&config));

    for (top, bottom, expected) in [
        (None, None, true),
        (Some(0.0), None, true),
        (None, Some(0.0), true),
        (Some(-0.0), Some(0.0), true),
        (Some(f32::EPSILON), None, false),
        (None, Some(0.10), false),
    ] {
        config.pdf_options = Some(crate::PdfConfig {
            top_margin_fraction: top,
            bottom_margin_fraction: bottom,
            ..Default::default()
        });
        assert_eq!(
            super::super::preserves_requested_projection(&config),
            expected,
            "margins: {top:?}, {bottom:?}"
        );
    }
}
