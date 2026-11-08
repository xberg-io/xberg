use super::{
    ImageEncoding, ImageSource, StreamFilters, adds_text_preserving_native, decode_probe_stream,
    source_audit::hidden_text_image_name,
};
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
fn split_source_audit_requires_independently_complete_paint_and_hidden_text() {
    use super::{
        Object,
        source_audit::{audit_hidden_text_image, audit_split_contents},
    };

    let doc = super::PdfDocument::from_bytes(CCITT_PROBE_PDF.to_vec()).unwrap();
    let audit = |first: &[u8], second: &[u8], byte_limit, mut work| {
        let streams = [first, second].map(|bytes| Object::Stream {
            dict: Default::default(),
            data: bytes::Bytes::copy_from_slice(bytes),
        });
        let joined = audit_split_contents(&doc, &streams, byte_limit, &mut work)?;
        audit_hidden_text_image(&joined, true, |font| font == "F")
    };
    let paint = b"q 612 0 0 792 0 0 cm /Scan Do Q";
    let hidden = b"BT /F 12 Tf 3 Tr\n1 0 0 1 20 20 Tm (layer) Tj\nET";
    let work = paint.len() + hidden.len() + 14;
    assert_eq!(audit(paint, hidden, 1024, work).as_deref(), Some("Scan"));
    assert_eq!(audit(paint, hidden, 1024, work - 1), None, "both parser passes");
    assert_eq!(
        audit(paint, hidden, 1024, paint.len() + hidden.len() - 1),
        None,
        "raw bytes are debited before copying"
    );
    assert_eq!(audit(paint, hidden, 128, work), None, "cumulative bytes");
    for bad_paint in [
        b"q 612 0 0 792 0 0 cm /Scan".as_slice(),
        b"q 612 0 0 792 0 0 cm /Scan Do",
        b"99 q 612 0 0 792 0 0 cm /Scan Do Q",
        b"q 612 0 0 792 cm /Scan Do Q",
        b"q 612 0 0 792 0 0 cm /Scan Do 1 Q",
    ] {
        assert_eq!(audit(bad_paint, hidden, 1024, 4096), None);
    }
    for bad_text in [
        b"Do Q BT /F 12 Tf 3 Tr\n1 0 0 1 20 20 Tm (layer) Tj\nET".as_slice(),
        b"BT /F 12 Tf 3 Tr\n1 0 0 1 20 20 Tm (layer) Tj",
        b"BT /F 12 Tf 3 Tr\n1 0 0 1 20 20 Tm (layer) Tj\nET /Cover Do",
        b"BT /F 12 Tf 0 Tr\n1 0 0 1 20 20 Tm (layer) Tj\nET",
        b"BT /F 12 Tf 259 Tr\n1 0 0 1 20 20 Tm (layer) Tj\nET",
        b"BT /F 12 Tf 3 0 Tr\n1 0 0 1 20 20 Tm (layer) Tj\nET",
    ] {
        assert_eq!(audit(paint, bad_text, 1024, 4096), None);
    }
}

#[test]
fn split_source_audit_rejects_compressed_content() {
    use super::{Object, source_audit::audit_split_contents};
    use std::io::Write;

    let doc = super::PdfDocument::from_bytes(CCITT_PROBE_PDF.to_vec()).unwrap();
    let paint = b"q 612 0 0 792 0 0 cm /Scan Do Q";
    let hidden = b"BT /F 12 Tf 3 Tr\n1 0 0 1 20 20 Tm (layer) Tj\nET";
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
    encoder.write_all(hidden).unwrap();
    let streams = [
        Object::Stream {
            dict: Default::default(),
            data: bytes::Bytes::copy_from_slice(paint),
        },
        Object::Stream {
            dict: std::collections::HashMap::from([("Filter".to_owned(), Object::Name("FlateDecode".into()))]),
            data: encoder.finish().unwrap().into(),
        },
    ];
    assert!(audit_split_contents(&doc, &streams, 1024, &mut 4096).is_none());
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
        encoding: ImageEncoding::Stream {
            filters: StreamFilters::Flate,
            predictor: None,
        },
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

const CCITT_PROBE_PDF: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/ocr/hidden_layer_ccitt_2425/ccitt-partial.pdf"
));

#[test]
fn ccitt_probe_samples_equal_the_independent_luma_oracle() {
    use sha2::{Digest, Sha256};

    let doc = super::PdfDocument::from_bytes(CCITT_PROBE_PDF.to_vec()).unwrap();
    let source = super::image_source(&doc, 0, "Scan").unwrap();
    let packed_bytes = (source.width as usize).div_ceil(8) * source.height as usize;
    let exact_budget = source.data.len() + packed_bytes + source.decoded_size().unwrap();
    let samples = source.decode_samples(exact_budget).unwrap();
    let with_boxes = source.decode_budget(exact_budget, 1).unwrap();
    assert!(source.decode_samples(with_boxes).is_none());
    assert_eq!(samples.len(), 2_103_750);
    assert_eq!(
        hex::encode(Sha256::digest(&samples)),
        "96b76deb96146ead1a486b9f6ba115424a6bee7183467eb7347930f20f35d885"
    );
}

#[test]
fn ccitt_probe_refuses_real_truncation_and_small_budget() {
    let doc = super::PdfDocument::from_bytes(CCITT_PROBE_PDF.to_vec()).unwrap();
    let mut source = super::image_source(&doc, 0, "Scan").unwrap();
    assert!(source.decode_samples(2 * 1024 * 1024).is_none());
    source.data.truncate(source.data.len() - 1);
    assert!(source.decode_samples(4 * 1024 * 1024).is_none());
}

fn ccitt_probe_samples(black_is_1: Option<bool>, decode: Option<[f64; 2]>) -> Option<Vec<u8>> {
    let mut pdf = lopdf::Document::load_mem(CCITT_PROBE_PDF).unwrap();
    let image = pdf
        .objects
        .values_mut()
        .find_map(|object| {
            let stream = object.as_stream_mut().ok()?;
            (stream.dict.get(b"Filter").ok()?.as_name().ok()? == b"CCITTFaxDecode").then_some(stream)
        })
        .unwrap();
    let params = image.dict.get_mut(b"DecodeParms").unwrap().as_dict_mut().unwrap();
    match black_is_1 {
        Some(value) => params.set("BlackIs1", value),
        None => {
            params.remove(b"BlackIs1");
        }
    }
    if let Some([low, high]) = decode {
        image.dict.set("Decode", vec![low.into(), high.into()]);
    }
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    let doc = super::PdfDocument::from_bytes(bytes).unwrap();
    super::image_source(&doc, 0, "Scan")?.decode_samples(4 * 1024 * 1024)
}

#[test]
fn ccitt_probe_applies_blackis1_and_decode_to_the_gray_samples() {
    let oracle = ccitt_probe_samples(Some(true), None).unwrap();
    let inverse: Vec<u8> = oracle.iter().map(|&sample| 255 - sample).collect();
    assert_ne!(oracle, inverse);
    assert_eq!(
        ccitt_probe_samples(Some(true), Some([0.0, 1.0])).as_ref(),
        Some(&oracle)
    );
    assert_eq!(
        ccitt_probe_samples(Some(false), Some([1.0, 0.0])).as_ref(),
        Some(&oracle)
    );
    assert_eq!(ccitt_probe_samples(Some(false), None).as_ref(), Some(&inverse));
    assert_eq!(ccitt_probe_samples(None, Some([0.0, 1.0])).as_ref(), Some(&inverse));
    assert_eq!(
        ccitt_probe_samples(Some(true), Some([1.0, 0.0])).as_ref(),
        Some(&inverse)
    );
    for unsupported in [[0.0, 0.5], [1.0, 1.0], [0.0, 2.0]] {
        assert_eq!(
            ccitt_probe_samples(Some(true), Some(unsupported)),
            None,
            "{unsupported:?}"
        );
    }
}

#[test]
fn eight_bit_hidden_images_still_refuse_a_decode_array() {
    let probe = |decode: bool| {
        let mut pdf = lopdf::Document::load_mem(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/ocr/hidden_layer_ccitt_2425/flate8-partial.pdf"
        )))
        .unwrap();
        if decode {
            let image = pdf
                .objects
                .values_mut()
                .find_map(|object| {
                    let stream = object.as_stream_mut().ok()?;
                    (stream.dict.get(b"Subtype").ok()?.as_name().ok()? == b"Image").then_some(stream)
                })
                .unwrap();
            image.dict.set("Decode", vec![1.into(), 0.into()]);
        }
        let mut bytes = Vec::new();
        pdf.save_to(&mut bytes).unwrap();
        super::image_source(&super::PdfDocument::from_bytes(bytes).unwrap(), 0, "Scan").is_some()
    };
    assert!(probe(false));
    assert!(!probe(true));
}

#[test]
fn hidden_image_predictor_defaults_must_match_the_image_geometry() {
    use super::{Object, image_predictor};

    let mut params = std::collections::HashMap::from([
        ("Predictor".to_owned(), Object::Integer(12)),
        ("Columns".to_owned(), Object::Integer(3)),
    ]);
    let decoded = image_predictor(&params, 3, 1).expect("default gray predictor parameters");
    assert_eq!(
        (
            decoded.predictor,
            decoded.columns,
            decoded.colors,
            decoded.bits_per_component
        ),
        (12, 3, 1, 8)
    );
    assert!(image_predictor(&params, 3, 3).is_none(), "default Colors is one");

    params.insert("Colors".into(), Object::Integer(3));
    assert_eq!(image_predictor(&params, 3, 3).unwrap().colors, 3);
    params.remove("Colors");
    params.remove("Columns");
    assert!(image_predictor(&params, 3, 1).is_none(), "default Columns is one");
    assert_eq!(image_predictor(&params, 1, 1).unwrap().columns, 1);

    for key in ["Predictor", "Columns", "Colors", "BitsPerComponent", "Unknown"] {
        let mut malformed = params.clone();
        malformed.insert(key.into(), Object::Name("invalid".into()));
        assert!(image_predictor(&malformed, 1, 1).is_none(), "{key}");
    }
}

#[test]
fn ccitt_group4_parameters_require_exact_geometry_and_grammar() {
    use super::Object;

    let mut params = std::collections::HashMap::from([
        ("K".to_owned(), Object::Integer(-1)),
        ("Columns".to_owned(), Object::Integer(1275)),
        ("Rows".to_owned(), Object::Integer(1650)),
        ("BlackIs1".to_owned(), Object::Boolean(true)),
    ]);
    assert_eq!(super::supported_group4_params(&params, 1275, 1650), Some(true));
    assert_eq!(super::supported_group4_params(&params, 1276, 1650), None);
    assert_eq!(super::supported_group4_params(&params, 1275, 1649), None);
    let mut defaults = params.clone();
    defaults.remove("Rows");
    defaults.remove("BlackIs1");
    assert_eq!(super::supported_group4_params(&defaults, 1275, 1650), Some(false));
    let mut other_group = defaults.clone();
    other_group.insert("K".to_owned(), Object::Integer(0));
    assert_eq!(super::supported_group4_params(&other_group, 1275, 1650), None);
    let mut not_boolean = params.clone();
    not_boolean.insert("BlackIs1".to_owned(), Object::Integer(1));
    assert_eq!(super::supported_group4_params(&not_boolean, 1275, 1650), None);
    params.insert("DamagedRowsBeforeError".to_owned(), Object::Integer(1));
    assert_eq!(super::supported_group4_params(&params, 1275, 1650), None);
    let filter = Object::Name("CCITTFaxDecode".to_owned());
    assert!(super::stream_filters(Some(&filter)).is_none());
}

#[test]
fn ccitt_work_is_debited_per_request_without_charging_legacy_streams() {
    let doc = super::PdfDocument::from_bytes(CCITT_PROBE_PDF.to_vec()).unwrap();
    let ccitt = super::image_source(&doc, 0, "Scan").unwrap();
    let mut remaining = 10_000_000;
    assert_eq!(ccitt.charge_ccitt_work(&mut remaining), Some(()));
    assert_eq!(remaining, 270_510);
    assert_eq!(ccitt.charge_ccitt_work(&mut remaining), None);
    assert_eq!(remaining, 270_510);

    let legacy = ImageSource {
        encoding: ImageEncoding::Stream {
            filters: StreamFilters::Raw,
            predictor: None,
        },
        ..ccitt
    };
    assert_eq!(legacy.charge_ccitt_work(&mut remaining), Some(()));
    assert_eq!(remaining, 270_510);
}

#[test]
fn outlined_prose_may_be_added_between_native_lines_without_splitting_them() {
    let native = "North Gate 42\nPage 7";
    for text in [
        "North WRONG Gate 42 Page 7",
        "North Gate 42 Page WRONG 7",
        "Page 7 additional prose North Gate 42",
    ] {
        assert!(!super::keeps_native_lines_in_order(native, text), "{text}");
    }
    assert!(super::keeps_native_lines_in_order(
        native,
        "North Gate 42 additional prose Page 7"
    ));
}

#[test]
fn filtered_split_source_audit_accepts_hidden_text_and_paint_in_either_order() {
    use super::{
        Object,
        source_audit::{audit_filtered_contents, audit_hidden_text_image},
    };
    use std::io::Write;

    let doc = super::PdfDocument::from_bytes(CCITT_PROBE_PDF.to_vec()).unwrap();
    let flate = |bytes: &[u8]| {
        let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
        encoder.write_all(bytes).unwrap();
        Object::Stream {
            dict: std::collections::HashMap::from([("Filter".to_owned(), Object::Name("FlateDecode".into()))]),
            data: encoder.finish().unwrap().into(),
        }
    };
    let raw = |bytes: &[u8]| Object::Stream {
        dict: Default::default(),
        data: bytes::Bytes::copy_from_slice(bytes),
    };
    let audit = |streams: &[Object], byte_limit, mut work| {
        let joined = audit_filtered_contents(&doc, streams, byte_limit, &mut work)?;
        audit_hidden_text_image(&joined, false, |font| font == "F")
    };
    let paint = b"q 612 0 0 792 0 0 cm /Scan Do Q";
    let hidden = b"BT /F 12 Tf 3 Tr\n0 0 TD (layer) Tj 5 0 TD (more) Tj\nET";
    let work = paint.len() + hidden.len();
    for streams in [[flate(paint), flate(hidden)], [flate(hidden), flate(paint)]] {
        assert_eq!(audit(&streams, 4096, work).as_deref(), Some("Scan"));
        assert_eq!(audit(&streams, 4096, work - 1), None, "decoded bytes are debited");
        assert_eq!(audit(&streams, 64, work), None, "cumulative bytes");
    }
    let one_filtered = [raw(paint), flate(hidden)];
    assert_eq!(
        audit(&one_filtered, 4096, work),
        None,
        "an unfiltered stream keeps the strict raw audit"
    );
    assert_eq!(audit(&[flate(paint)], 4096, work), None);
    assert_eq!(audit(&[flate(paint), flate(hidden), flate(paint)], 4096, work), None);
    for visible in [
        b"BT /F 12 Tf 0 Tr\n0 0 TD (layer) Tj\nET".as_slice(),
        b"BT /F 12 Tf 3 Tr\n0 0 TD (layer) Tj 0 Tr (shown) Tj\nET",
        b"BT /F 12 Tf\n0 0 TD (layer) Tj\nET",
        b"BT /F 12 Tf 3 Tr\n0 0 TD (layer) Tj\nET /Cover Do",
        b"BT /F 12 Tf 3 Tr\n0 0 TD (layer) Tj",
    ] {
        assert_eq!(audit(&[flate(visible), flate(paint)], 4096, work + 64), None);
    }
}

#[test]
fn source_audit_operator_limit_follows_the_content_byte_limit() {
    let mut content = b"q 612 0 0 792 0 0 cm /Scan Do Q BT /F 12 Tf 3 Tr\n".to_vec();
    let long_enough = super::MAX_OPERATORS + 1;
    for _ in 0..long_enough {
        content.extend_from_slice(b"1 0 TD ");
    }
    content.extend_from_slice(b"(layer) Tj ET");
    assert_eq!(
        hidden_text_image_name(&content, |font| font == "F").as_deref(),
        Some("Scan")
    );
    let mut over_limit = b"BT /F 12 Tf 3 Tr ".to_vec();
    for _ in 0..super::MAX_AUDIT_OPERATORS {
        over_limit.extend_from_slice(b"1 0 TD ");
    }
    over_limit.extend_from_slice(b"(layer) Tj ET /Scan Do");
    assert!(hidden_text_image_name(&over_limit, |font| font == "F").is_none());
}

#[test]
fn a_layer_yields_to_ocr_only_when_nearly_disjoint_from_it() {
    use super::ocr_supersedes_disjoint_layer as supersedes;

    let ocr = "REPORT 1135 NATIONAL ADVISORY COMMITTEE FOR AERONAUTICS TABLE II SUPERSONIC FLOW";
    assert!(supersedes("3.39 3.40 3.34 3.35 3.36 3.37 3.38 3.25 3.26 3.27", ocr));
    assert!(!supersedes(ocr, ocr), "a layer that matches the read stays");
    assert!(
        !supersedes("REPORT 1135 NATIONAL ADVISORY zzq vvk plm", ocr),
        "most words are shared"
    );
    assert!(!supersedes("", ocr));
    assert!(!supersedes("3.39 3.40", ""));

    let layer = (0..20).map(|n| format!("junk{n}")).collect::<Vec<_>>();
    let read = |shared: usize| {
        layer[..shared]
            .iter()
            .map(String::as_str)
            .chain(["alpha", "beta", "gamma"])
            .collect::<Vec<_>>()
            .join(" ")
    };
    assert!(supersedes(&layer.join(" "), &read(3)), "3 of 20 words is 0.15");
    assert!(!supersedes(&layer.join(" "), &read(4)), "4 of 20 words is 0.20");
    assert!(
        supersedes("1 1 1 1 1 1 1 1 1 1 1 1 1 1 1 1 1 1 1 1", "1 alpha beta gamma"),
        "a repeated word is shared once"
    );
    assert!(!supersedes(
        "1 1 1 1 1 1 1 1 1 1 1 1 1 1 1 1 1 1 1 1",
        "1 1 1 1 1 1 1 1 1 1 1 1 1 1 1 1 1 1 1 1"
    ));
}
