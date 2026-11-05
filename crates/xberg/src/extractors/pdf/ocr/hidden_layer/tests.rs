use super::{ImageSource, StreamFilters, adds_text_preserving_native, decode_probe_stream, hidden_text_image_name};
use xberg_native_pdf::content::parser::visit_complete_content_stream;

#[test]
fn visible_native_line_audit_rejects_hidden_or_unowned_paint() {
    let prove = |stream: &str, text| super::painted_literal_image(stream.as_bytes(), text, |font| font == "F");
    let painted = "/Scan Do BT /F 56 Tf 1 0 0 1 150 640 Tm (SECTION TITLE) Tj ET";
    assert_eq!(prove(painted, "SECTION TITLE").as_deref(), Some("Scan"));
    assert_eq!(prove(painted, "SECTION TITLES"), None);
    for incompatible in [
        painted.replace("56 Tf", "56 Tf 3 Tr"),
        painted.replace("56 Tf", "56 Tf 7 Tr"),
        painted.replace("/F 56", "/Other 56"),
        painted.replace("/Scan Do ", ""),
        format!("{painted} /Cover Do"),
        format!("{painted} 0 0 10 10 re f"),
        painted.replace("Tj", "Tj (REPEAT) Tj"),
        painted.replace("Tj ET", "Tj"),
    ] {
        assert_eq!(prove(&incompatible, "SECTION TITLE"), None, "{incompatible}");
    }
}

#[test]
fn literal_growth_must_preserve_native_order_and_repeated_words() {
    assert!(!adds_text_preserving_native("Page 7", "Page 7 Page 7"));
    assert!(!adds_text_preserving_native("Page 7", "Page 7 more words Page 7"));
    let native = "Page 7. naïve total total";
    assert!(adds_text_preserving_native(
        native,
        "Page 7. additional naïve total total"
    ));
    assert!(!adds_text_preserving_native(native, "Page 7. additional naïve total"));
    assert!(!adds_text_preserving_native(
        native,
        "Page 7. total total naïve additional"
    ));
    assert!(!adds_text_preserving_native(native, "Page 7. naïve total total"));
    assert!(!adds_text_preserving_native(native, "Page 7. naïve total total !"));
    assert!(!adds_text_preserving_native(
        native,
        "Page 7 naive total total additional"
    ));
}

#[test]
fn source_audit_requires_one_image_and_only_invisible_text_paint() {
    let supported = b"q 612 0 0 792 0 0 cm /Scan Do Q BT /F 12 Tf 3 Tr (layer) Tj ET";
    assert_eq!(
        hidden_text_image_name(supported, |font| font == "F").as_deref(),
        Some("Scan")
    );

    for unsupported in [
        b"q 612 0 0 792 0 0 cm /Scan Do Q BT /F 12 Tf (visible) Tj ET".as_slice(),
        b"q 612 0 0 792 0 0 cm /Scan Do Q /Second Do BT /F 12 Tf 3 Tr (layer) Tj ET",
        b"q 612 0 0 792 0 0 cm /Scan Do Q 1 1 20 20 re f BT /F 12 Tf 3 Tr (layer) Tj ET",
        b"q 612 0 0 792 0 0 cm /Scan Do Q 1 1 20 20 re W n BT /F 12 Tf 3 Tr (layer) Tj ET",
        b"q 0 612 -792 0 792 0 cm /Scan Do Q BT /F 12 Tf 3 Tr (layer) Tj ET",
        b"q 612 0 0 792 0 0 cm /Scan Do Q BT /F 12 Tf 3 Tr (layer) Tj 0 Tr (visible) Tj ET",
        b"q 612 0 0 792 0 0 cm /Scan Do BT /F 12 Tf 3 Tr (layer) Tj ET",
    ] {
        assert!(
            hidden_text_image_name(unsupported, |font| font == "F").is_none(),
            "{}",
            String::from_utf8_lossy(unsupported)
        );
    }
}

#[test]
fn source_audit_tracks_the_bound_font_across_text_objects_and_saved_state() {
    let audit = |content: &[u8]| hidden_text_image_name(content, |font| font == "F");
    let retained = b"/Scan Do BT /F 12 Tf 3 Tr ET q BT 0 Tr ET Q BT (layer) Tj ET";
    assert_eq!(audit(retained).as_deref(), Some("Scan"));

    for unsupported in [
        b"/Scan Do BT /Missing 12 Tf 3 Tr (layer) Tj ET".as_slice(),
        b"/Scan Do BT /F 0 Tf 3 Tr (layer) Tj ET",
        b"/Scan Do BT /F -12 Tf 3 Tr (layer) Tj ET",
        b"/Scan Do BT 3 Tr (layer) Tj ET",
        b"/Scan Do q BT /F 12 Tf ET Q BT 3 Tr (layer) Tj ET",
    ] {
        assert!(audit(unsupported).is_none(), "{}", String::from_utf8_lossy(unsupported));
    }
}

#[test]
fn complete_source_audit_rejects_truncated_and_over_limit_operator_streams() {
    let supported = b"q 612 0 0 792 0 0 cm /Scan Do Q BT /F 12 Tf 3 Tr (layer) Tj ET";
    let malformed = [supported.as_slice(), b" <"].concat();
    assert!(hidden_text_image_name(&malformed, |font| font == "F").is_none());
    assert!(visit_complete_content_stream(b"q Q", 1, |_| true).is_err());
    assert!(visit_complete_content_stream(b"q Q % whitespace and a comment", 2, |_| true).expect("complete stream"));
}

#[test]
fn actual_inflate_is_bounded_even_for_tiny_compressed_streams() {
    let uncompressed = vec![b'A'; 32 * 1024];
    let mut stream = lopdf::Stream::new(lopdf::Dictionary::new(), uncompressed.clone());
    stream.compress().expect("compress repeatable test stream");
    assert!(stream.content.len() < 128);
    assert_eq!(
        decode_probe_stream(&stream.content, StreamFilters::Flate, uncompressed.len(), 64 * 1024),
        Some(uncompressed)
    );
    assert!(decode_probe_stream(&stream.content, StreamFilters::Flate, 1024, 4096).is_none());
    let malformed_image = ImageSource {
        width: 16,
        height: 16,
        channels: 1,
        data: stream.content.into(),
        filters: StreamFilters::Flate,
        predictor: None,
    };
    assert!(malformed_image.decode_samples(4096).is_none());
    assert!(decode_probe_stream(&[b'z'; 32], StreamFilters::Ascii85Flate, 256, 64).is_none());
}

#[test]
fn trailing_text_requires_separated_glyphs_past_the_native_box() {
    let width = 96;
    let height = 8;
    let band = super::Band {
        rows: 0..height,
        columns: 8..80,
        runs: 9,
    };
    let mut mask = vec![false; width * height];
    for x in [40, 52, 64, 76] {
        for row in 0..height {
            mask[row * width + x] = true;
        }
        mask[(height - 1) * width + x..(height - 1) * width + x + 4].fill(true);
    }
    assert!(super::has_trailing_text(&mask, width, &band, 24));
    assert!(!super::has_trailing_text(&mask, width, &band, 72));

    mask.fill(false);
    for x in [40, 52, 64, 76] {
        for row in 0..height {
            mask[row * width + x..row * width + x + 4].fill(true);
        }
    }
    assert!(!super::has_trailing_text(&mask, width, &band, 24));
}
