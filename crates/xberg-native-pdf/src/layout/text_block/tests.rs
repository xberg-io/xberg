use super::*;

// The provenance fact reaches every JSON/serde binding (WASM, Go, Ruby,
// Java's structured extraction, ...) through span serialization: present as
// a stable label when known, omitted when absent so existing output is
// byte-identical. ~keep
#[test]
fn provenance_serializes_as_stable_label_and_omits_when_absent() {
    let mut span = TextSpan {
        text: "x".to_string(),
        ..TextSpan::default()
    };
    span.provenance = Some(crate::fonts::MappingProvenance::Fallback);
    let json = serde_json::to_string(&span).unwrap();
    assert!(json.contains("\"provenance\":\"fallback\""), "got {json}");

    let plain = TextSpan {
        text: "y".to_string(),
        ..TextSpan::default()
    };
    let json = serde_json::to_string(&plain).unwrap();
    assert!(
        !json.contains("provenance"),
        "absent provenance must be omitted: {json}"
    );
}

fn mock_char(c: char, x: f32, y: f32) -> TextChar {
    let bbox = Rect::new(x, y, 10.0, 12.0);
    TextChar {
        char: c,
        bbox,
        font_name: "Times".into(),
        font_size: 12.0,
        font_weight: FontWeight::Normal,
        is_italic: false,
        is_monospace: false,
        color: Color::black(),
        mcid: None,
        origin_x: bbox.x,
        origin_y: bbox.y,
        rotation_degrees: 0.0,
        advance_width: bbox.width,
        rendered_advance: bbox.width,
        ascent: 0.95 * 12.0,
        descent: -0.35 * 12.0,
        matrix: None,
    }
}

// A block whose two fonts carry equal character counts must resolve to the
// lexicographically smaller name, every run. `max_by_key` over the backing
// `HashMap` returned whichever entry the per-process-randomized iteration
// order visited last, so this assertion failed roughly half the time before
// the explicit tie-break. ~keep
#[test]
fn dominant_font_tie_resolves_to_lexicographically_smaller_name() {
    let mut chars = Vec::new();
    for (i, c) in "abcd".chars().enumerate() {
        let mut ch = mock_char(c, i as f32 * 10.0, 0.0);
        ch.font_name = "Times".into();
        chars.push(ch);
    }
    for (i, c) in "efgh".chars().enumerate() {
        let mut ch = mock_char(c, 40.0 + i as f32 * 10.0, 0.0);
        ch.font_name = "Courier".into();
        chars.push(ch);
    }
    let block = TextBlock::from_chars(chars);
    assert_eq!(
        block.dominant_font, "Courier",
        "4 chars of Times vs 4 of Courier must resolve to the smaller name"
    );
}

#[test]
fn dominant_font_unique_winner_is_unaffected_by_tie_break() {
    let mut chars = Vec::new();
    for (i, c) in "abcdef".chars().enumerate() {
        let mut ch = mock_char(c, i as f32 * 10.0, 0.0);
        ch.font_name = "Times".into();
        chars.push(ch);
    }
    for (i, c) in "gh".chars().enumerate() {
        let mut ch = mock_char(c, 60.0 + i as f32 * 10.0, 0.0);
        ch.font_name = "Courier".into();
        chars.push(ch);
    }
    let block = TextBlock::from_chars(chars);
    assert_eq!(block.dominant_font, "Times");
}

#[test]
fn test_text_block_from_chars() {
    let chars = vec![
        mock_char('H', 0.0, 0.0),
        mock_char('e', 10.0, 0.0),
        mock_char('l', 20.0, 0.0),
        mock_char('l', 30.0, 0.0),
        mock_char('o', 40.0, 0.0),
    ];

    let block = TextBlock::from_chars(chars);
    assert_eq!(block.text, "Hello");
    assert_eq!(block.avg_font_size, 12.0);
}

#[test]
fn test_text_span_is_monospace_default() {
    let span = TextSpan::default();
    assert!(!span.is_monospace, "Default spans should not be monospace");
}

#[test]
fn test_text_span_is_monospace_set() {
    let span = TextSpan {
        is_monospace: true,
        text: "AB".to_string(),
        bbox: Rect::new(0.0, 0.0, 20.0, 12.0),
        ..TextSpan::default()
    };
    assert!(span.is_monospace);

    let chars = span.to_chars();
    for c in &chars {
        assert!(c.is_monospace, "TextChar should inherit is_monospace from span");
    }
}

#[test]
fn test_text_char_is_monospace() {
    let c = TextChar {
        char: 'A',
        bbox: Rect::new(0.0, 0.0, 10.0, 12.0),
        font_name: "Courier".into(),
        font_size: 12.0,
        font_weight: FontWeight::Normal,
        is_italic: false,
        is_monospace: true,
        color: Color::black(),
        mcid: None,
        origin_x: 0.0,
        origin_y: 0.0,
        rotation_degrees: 0.0,
        advance_width: 10.0,
        rendered_advance: 10.0,
        ascent: 0.95 * 12.0,
        descent: -0.35 * 12.0,
        matrix: None,
    };
    assert!(c.is_monospace);
}

#[test]
fn test_to_chars_uses_char_widths_when_available() {
    let span = TextSpan {
        text: "AB".to_string(),
        bbox: Rect::new(10.0, 20.0, 30.0, 12.0),
        char_widths: vec![10.0, 20.0],
        char_x_offsets: Vec::new(),
        ..TextSpan::default()
    };
    let chars = span.to_chars();
    assert_eq!(chars.len(), 2);
    assert!((chars[0].bbox.x - 10.0).abs() < 0.001);
    assert!((chars[0].bbox.width - 10.0).abs() < 0.001);
    assert!((chars[0].advance_width - 10.0).abs() < 0.001);
    assert!((chars[1].bbox.x - 20.0).abs() < 0.001);
    assert!((chars[1].bbox.width - 20.0).abs() < 0.001);
    assert!((chars[1].advance_width - 20.0).abs() < 0.001);
}

#[test]
fn test_to_chars_falls_back_to_uniform_when_no_widths() {
    let span = TextSpan {
        text: "AB".to_string(),
        bbox: Rect::new(10.0, 20.0, 30.0, 12.0),
        ..TextSpan::default()
    };
    let chars = span.to_chars();
    assert_eq!(chars.len(), 2);
    assert!((chars[0].bbox.width - 15.0).abs() < 0.001);
    assert!((chars[1].bbox.width - 15.0).abs() < 0.001);
    assert!((chars[0].bbox.x - 10.0).abs() < 0.001);
    assert!((chars[1].bbox.x - 25.0).abs() < 0.001);
}

#[test]
fn test_to_chars_handles_mismatched_widths_gracefully() {
    let span = TextSpan {
        text: "ABC".to_string(),
        bbox: Rect::new(0.0, 0.0, 30.0, 12.0),
        char_widths: vec![5.0, 10.0],
        ..TextSpan::default()
    };
    let chars = span.to_chars();
    assert_eq!(chars.len(), 3);
    assert!((chars[0].bbox.width - 10.0).abs() < 0.001);
    assert!((chars[1].bbox.width - 10.0).abs() < 0.001);
    assert!((chars[2].bbox.width - 10.0).abs() < 0.001);
}

#[test]
fn test_to_chars_prefers_char_x_offsets_over_widths() {
    // char_x_offsets carry positions that DIVERGE from a prefix-sum of
    // char_widths (simulating TJ-kerning drift). to_chars must honor the
    // offsets for origin_x / bbox.x while still taking widths from
    // char_widths (the unchanged Indic-guarded model). ~keep
    let span = TextSpan {
        text: "AB".to_string(),
        bbox: Rect::new(10.0, 20.0, 30.0, 12.0),
        char_widths: vec![10.0, 20.0],
        char_x_offsets: vec![10.0, 25.0], // NOT 10.0, 20.0 (prefix-sum) ~keep
        ..TextSpan::default()
    };
    let chars = span.to_chars();
    assert_eq!(chars.len(), 2);
    assert!((chars[0].origin_x - 10.0).abs() < 0.001);
    assert!((chars[0].bbox.x - 10.0).abs() < 0.001);
    assert!((chars[1].origin_x - 25.0).abs() < 0.001);
    assert!((chars[1].bbox.x - 25.0).abs() < 0.001);
    assert!((chars[0].bbox.width - 10.0).abs() < 0.001);
    assert!((chars[1].bbox.width - 20.0).abs() < 0.001);
}

#[test]
fn test_to_chars_empty_offsets_is_byte_identical_fallback() {
    let with_offsets = TextSpan {
        text: "AB".to_string(),
        bbox: Rect::new(10.0, 20.0, 30.0, 12.0),
        char_widths: vec![10.0, 20.0],
        char_x_offsets: Vec::new(),
        ..TextSpan::default()
    };
    let legacy = TextSpan {
        text: "AB".to_string(),
        bbox: Rect::new(10.0, 20.0, 30.0, 12.0),
        char_widths: vec![10.0, 20.0],
        char_x_offsets: Vec::new(),
        ..TextSpan::default()
    };
    let a = with_offsets.to_chars();
    let b = legacy.to_chars();
    assert_eq!(a.len(), b.len());
    for (ca, cb) in a.iter().zip(b.iter()) {
        assert!((ca.origin_x - cb.origin_x).abs() < 1e-6);
        assert!((ca.bbox.x - cb.bbox.x).abs() < 1e-6);
        assert!((ca.bbox.width - cb.bbox.width).abs() < 1e-6);
    }
    assert!((a[0].bbox.x - 10.0).abs() < 0.001);
    assert!((a[1].bbox.x - 20.0).abs() < 0.001);
}

#[test]
fn test_to_chars_offset_count_mismatch_falls_back() {
    let span = TextSpan {
        text: "ABC".to_string(),
        bbox: Rect::new(0.0, 0.0, 30.0, 12.0),
        char_widths: vec![10.0, 10.0, 10.0],
        char_x_offsets: vec![0.0, 15.0],
        ..TextSpan::default()
    };
    let chars = span.to_chars();
    assert_eq!(chars.len(), 3);
    assert!((chars[0].bbox.x - 0.0).abs() < 0.001);
    assert!((chars[1].bbox.x - 10.0).abs() < 0.001);
    assert!((chars[2].bbox.x - 20.0).abs() < 0.001);
}

#[test]
fn test_to_chars_out_of_bounds_offsets_fall_back_to_span_geometry() {
    let span = TextSpan {
        text: "1".to_string(),
        bbox: Rect::new(216.0, 315.0, 4.0, 7.0),
        char_widths: vec![4.0],
        // A repeated digit elsewhere on the same baseline was incorrectly
        // stamped onto this superscript run. ~keep
        char_x_offsets: vec![252.0],
        ..TextSpan::default()
    };

    let chars = span.to_chars();

    assert_eq!(chars.len(), 1);
    assert!((chars[0].origin_x - 216.0).abs() < 0.001);
    assert!((chars[0].bbox.width - 4.0).abs() < 0.001);
}

#[test]
fn text_rise_zero_serde_omitted() {
    // A default (on-baseline) span must NOT serialize a `text_rise` key, so
    // existing fixtures stay byte-identical now that the field exists. ~keep
    let span = TextSpan {
        text: "x".to_string(),
        ..TextSpan::default()
    };
    let json = serde_json::to_string(&span).unwrap();
    assert!(
        !json.contains("text_rise"),
        "zero text_rise must be omitted from serialized output: {json}"
    );

    // A non-zero rise IS serialized (the rejoin signal must survive a round-trip). ~keep
    let raised = TextSpan {
        text: "2".to_string(),
        text_rise: 0.33,
        ..TextSpan::default()
    };
    let json = serde_json::to_string(&raised).unwrap();
    assert!(
        json.contains("text_rise"),
        "non-zero text_rise must be serialized: {json}"
    );
}

#[test]
fn to_chars_shares_the_span_font_name() {
    let span = TextSpan {
        text: "abc".to_string(),
        font_name: "ABCDEF+CMR10".into(),
        bbox: Rect::new(0.0, 0.0, 30.0, 10.0),
        ..TextSpan::default()
    };
    let chars = span.to_chars();
    assert_eq!(chars.len(), 3);
    for ch in &chars {
        assert_eq!(ch.font_name, "ABCDEF+CMR10");
        assert!(std::ptr::eq(
            ch.font_name.as_str().as_ptr(),
            span.font_name.as_str().as_ptr()
        ));
    }
    let block = TextBlock::from_chars(chars);
    assert!(std::ptr::eq(
        block.dominant_font.as_str().as_ptr(),
        span.font_name.as_str().as_ptr()
    ));
}

#[test]
fn font_name_serializes_as_a_plain_string() {
    let span = TextSpan {
        font_name: "Helvetica-Bold".into(),
        ..TextSpan::default()
    };
    let json = serde_json::to_value(&span).unwrap();
    assert_eq!(
        json["font_name"],
        serde_json::Value::String("Helvetica-Bold".to_string())
    );
}
