fn key_value_span(text: &str, x: f32, y: f32) -> crate::layout::TextSpan {
    crate::layout::TextSpan {
        text: text.to_string(),
        bbox: crate::geometry::Rect::new(x, y, 90.0, 10.0),
        font_size: 11.0,
        ..crate::layout::TextSpan::default()
    }
}

#[test]
fn key_value_rows_keep_centered_labels_before_wrapped_values() {
    let mut spans = vec![
        key_value_span("Applicant:", 77.0, 600.0),
        key_value_span("Example Holdings", 231.0, 600.0),
        key_value_span("Respondent:", 77.0, 574.0),
        key_value_span("Alex Sample", 231.0, 574.0),
        key_value_span("12 Demo Street", 231.0, 555.0),
        key_value_span("Address:", 77.0, 548.0),
        key_value_span("X00Y000", 231.0, 541.0),
        key_value_span("Jordan Example", 231.0, 515.0),
        key_value_span("Panel:", 77.0, 508.0),
        key_value_span("Casey Placeholder", 231.0, 501.0),
        key_value_span("Venue:", 77.0, 475.0),
        key_value_span("Virtual", 231.0, 475.0),
    ];

    super::super::reorder_ltr_key_value_rows(&mut spans);

    let texts: Vec<&str> = spans.iter().map(|span| span.text.as_str()).collect();
    assert_eq!(
        texts,
        vec![
            "Applicant:",
            "Example Holdings",
            "Respondent:",
            "Alex Sample",
            "Address:",
            "12 Demo Street",
            "X00Y000",
            "Panel:",
            "Jordan Example",
            "Casey Placeholder",
            "Venue:",
            "Virtual",
        ]
    );
}

#[test]
fn key_value_repair_leaves_aligned_two_column_rows_unchanged() {
    let mut spans = vec![
        key_value_span("Name:", 77.0, 600.0),
        key_value_span("Example", 231.0, 600.0),
        key_value_span("Role:", 77.0, 574.0),
        key_value_span("Reviewer", 231.0, 574.0),
        key_value_span("Venue:", 77.0, 548.0),
        key_value_span("Virtual", 231.0, 548.0),
        key_value_span("Date:", 77.0, 522.0),
        key_value_span("2030-01-15", 231.0, 522.0),
    ];
    let original: Vec<String> = spans.iter().map(|span| span.text.clone()).collect();

    super::super::reorder_ltr_key_value_rows(&mut spans);

    assert_eq!(spans.iter().map(|span| span.text.clone()).collect::<Vec<_>>(), original);
}

#[test]
fn key_value_repair_leaves_rtl_rows_unchanged() {
    let mut spans = vec![
        key_value_span("שם:", 231.0, 600.0),
        key_value_span("ערך", 77.0, 607.0),
        key_value_span("נוסף", 77.0, 593.0),
        key_value_span("תפקיד:", 231.0, 574.0),
        key_value_span("בודק", 77.0, 574.0),
        key_value_span("מקום:", 231.0, 548.0),
        key_value_span("מרחוק", 77.0, 548.0),
        key_value_span("תאריך:", 231.0, 522.0),
        key_value_span("2030-01-15", 77.0, 522.0),
    ];
    let original: Vec<String> = spans.iter().map(|span| span.text.clone()).collect();

    super::super::reorder_ltr_key_value_rows(&mut spans);

    assert_eq!(spans.iter().map(|span| span.text.clone()).collect::<Vec<_>>(), original);
}

#[test]
fn key_value_repair_ignores_a_larger_unrelated_right_column() {
    let mut spans = vec![
        key_value_span("Name:", 77.0, 600.0),
        key_value_span("Example", 231.0, 600.0),
        key_value_span("Role:", 77.0, 574.0),
        key_value_span("Reviewer", 231.0, 574.0),
        key_value_span("First address line", 231.0, 555.0),
        key_value_span("Address:", 77.0, 548.0),
        key_value_span("Second address line", 231.0, 541.0),
        key_value_span("Venue:", 77.0, 522.0),
        key_value_span("Virtual", 231.0, 522.0),
    ];
    for row in 0..8 {
        spans.push(key_value_span(
            &format!("unrelated-{row}"),
            500.0,
            600.0 - row as f32 * 10.0,
        ));
    }

    super::super::reorder_ltr_key_value_rows(&mut spans);

    let texts: Vec<&str> = spans.iter().map(|span| span.text.as_str()).collect();
    assert_eq!(
        &texts[..9],
        [
            "Name:",
            "Example",
            "Role:",
            "Reviewer",
            "Address:",
            "First address line",
            "Second address line",
            "Venue:",
            "Virtual",
        ]
    );
    assert_eq!(texts[9], "unrelated-0");
}

#[test]
fn key_value_repair_handles_two_non_overlapping_blocks() {
    let mut spans = Vec::new();
    for (label_x, value_x, top, prefix) in [(50.0, 170.0, 600.0, "A"), (330.0, 450.0, 400.0, "B")] {
        spans.extend([
            key_value_span(&format!("{prefix}1:"), label_x, top),
            key_value_span(&format!("{prefix}v1"), value_x, top),
            key_value_span(&format!("{prefix}upper"), value_x, top - 19.0),
            key_value_span(&format!("{prefix}2:"), label_x, top - 26.0),
            key_value_span(&format!("{prefix}lower"), value_x, top - 33.0),
            key_value_span(&format!("{prefix}3:"), label_x, top - 52.0),
            key_value_span(&format!("{prefix}v3"), value_x, top - 52.0),
            key_value_span(&format!("{prefix}4:"), label_x, top - 78.0),
            key_value_span(&format!("{prefix}v4"), value_x, top - 78.0),
        ]);
    }

    super::super::reorder_ltr_key_value_rows(&mut spans);

    let texts: Vec<&str> = spans.iter().map(|span| span.text.as_str()).collect();
    for prefix in ["A", "B"] {
        let label = texts
            .iter()
            .position(|text| *text == format!("{prefix}2:"))
            .expect("label");
        let upper = texts
            .iter()
            .position(|text| *text == format!("{prefix}upper"))
            .expect("upper");
        let lower = texts
            .iter()
            .position(|text| *text == format!("{prefix}lower"))
            .expect("lower");
        assert!(label < upper && upper < lower, "{prefix} block order: {texts:?}");
    }
}
