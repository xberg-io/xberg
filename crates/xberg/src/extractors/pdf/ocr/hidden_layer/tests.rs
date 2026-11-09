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
        audit_hidden_text_image(&joined, true, super::MAX_OPERATORS, |font| font == "F")
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
        audit_hidden_text_image(&joined, false, super::MAX_AUDIT_OPERATORS, |font| font == "F")
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
fn only_the_filtered_two_stream_audit_allows_more_than_the_single_stream_operator_limit() {
    use super::source_audit::audit_hidden_text_image;

    let many = |count: usize| {
        let mut content = b"q 612 0 0 792 0 0 cm /Scan Do Q BT /F 12 Tf 3 Tr\n".to_vec();
        for _ in 0..count {
            content.extend_from_slice(b"1 0 TD ");
        }
        content.extend_from_slice(b"(layer) Tj ET");
        content
    };
    let audit = |content: &[u8], limit| audit_hidden_text_image(content, false, limit, |font| font == "F");
    let long = many(super::MAX_OPERATORS + 1);
    assert!(hidden_text_image_name(&long, |font| font == "F").is_none());
    assert_eq!(audit(&long, super::MAX_AUDIT_OPERATORS).as_deref(), Some("Scan"));
    assert!(audit(&many(super::MAX_AUDIT_OPERATORS), super::MAX_AUDIT_OPERATORS).is_none());
}

#[test]
fn a_layer_yields_to_ocr_only_when_nearly_disjoint_from_it() {
    use super::layer_judgement::ocr_supersedes_disjoint_layer as supersedes;

    let words = |prefix: &str, count: usize| (0..count).map(|n| format!("{prefix}{n}")).collect::<Vec<_>>();
    let layer = words("junk", 20).join(" ");
    let read = |shared: usize| {
        words("junk", shared)
            .into_iter()
            .chain(words("alpha", 30))
            .collect::<Vec<_>>()
            .join(" ")
    };
    assert!(supersedes(&layer, &read(3)), "3 of 20 words is 15%");
    assert!(!supersedes(&layer, &read(4)), "4 of 20 words is 20%");
    assert!(!supersedes(&layer, &layer), "a layer that matches the read stays");
    assert!(!supersedes("", &read(0)));
    assert!(!supersedes(&layer, ""));

    let repeated = ["1"; 20].join(" ");
    assert!(
        supersedes(&repeated, "1 alpha beta gamma"),
        "a repeated word is shared once"
    );
    assert!(!supersedes(&repeated, &repeated));
}

#[test]
fn disjointness_compares_normalized_words_of_a_layer_with_enough_words() {
    use super::layer_judgement::ocr_supersedes_disjoint_layer as supersedes;

    let spaced = (0..20).map(|n| format!("Word{n}.")).collect::<Vec<_>>().join(" ");
    let read = (0..20).map(|n| format!("(word{n})")).collect::<Vec<_>>().join(" ");
    assert!(
        !supersedes(&spaced, &read),
        "case and edge punctuation do not make words differ"
    );
    let stamp = "RECEIVED 14 JUN 1952 ARCHIVE";
    assert!(
        !supersedes(stamp, "Received 14 Jun. 1952 Archive"),
        "a 5-word stamp respelled by OCR stays"
    );
    assert!(
        !supersedes(stamp, "something else entirely"),
        "a layer under 20 words is never judged junk"
    );
    assert!(!supersedes("- - - - -", "x"), "punctuation alone is not a word");
}

#[test]
fn replacement_needs_the_default_confidence_gate_whatever_the_caller_set() {
    use super::layer_judgement::confidence_clears_replacement_floor as clears;

    assert!(clears(Some(0.76)));
    assert!(clears(Some(0.75)));
    assert!(!clears(Some(0.74)));
    assert!(!clears(None), "an uncalibrated backend never clears it");
}

#[test]
fn naca_1135_p25_reaches_the_coverage_check_within_the_default_preview_budget() {
    use sha2::{Digest, Sha256};

    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../test_documents/pdf/regression_pages/nasa-naca-report-1135-p25.pdf");
    let bytes = std::fs::read(&path).unwrap_or_else(|e| {
        panic!("NACA 1135 p25 fixture {path:?} is missing ({e}). Fetch the corpus: python3 test_documents/scripts/fetch_corpus.py")
    });
    assert_eq!(
        hex::encode(Sha256::digest(&bytes)),
        "1696465613177c9059fea2edd8a109c753c649fdfb80cf33712d9d890f0a3ed0"
    );
    let doc = super::PdfDocument::from_bytes(bytes).unwrap();
    let byte_limit = 100 * 1024 * 1024;
    // Two Flate streams (text, then picture), ~6.7k operators, a 300 dpi Group 4 image with
    // /Decode [0 1] and no /Rows or /BlackIs1; its junk glyph boxes cover every ink band.
    let default_work = 10_000_000 * super::PREVIEW_WORK_PER_ITERATION;
    assert_eq!(
        super::needs_ocr(&doc, 0, byte_limit, &mut { default_work }, || false),
        Some(false)
    );
    assert_eq!(
        super::needs_ocr(&doc, 0, byte_limit, &mut { 10_000_000 }, || false),
        None
    );
}

#[test]
fn preview_work_is_budgeted_per_page_under_a_document_ceiling() {
    use super::PreviewWorkBudget;

    let spend = |budget: &mut PreviewWorkBudget, cost: usize| {
        budget.probe_page(|page_work| {
            *page_work = page_work.checked_sub(cost)?;
            Some(())
        })
    };
    let mut budget = PreviewWorkBudget::with_pages(100, 3);
    assert_eq!(spend(&mut budget, 90), Some(()));
    assert_eq!(
        spend(&mut budget, 90),
        Some(()),
        "a page does not inherit the previous page's spend"
    );
    assert_eq!(
        spend(&mut budget, 101),
        None,
        "one page never exceeds the per-page limit"
    );
    assert_eq!(
        spend(&mut budget, 90),
        Some(()),
        "a refused page spends nothing it did not debit"
    );
    assert_eq!(
        spend(&mut budget, 31),
        None,
        "only 30 units of the three-page ceiling remain"
    );
    assert_eq!(spend(&mut budget, 30), Some(()));
    assert_eq!(
        spend(&mut budget, 1),
        None,
        "the document ceiling of three per-page limits is spent"
    );
}

#[test]
fn a_refused_charge_debits_nothing_and_the_ceiling_is_a_multiple_of_the_page_limit() {
    use super::{PREVIEW_FULL_BUDGET_PAGES, PREVIEW_WORK_PER_ITERATION, PreviewWorkBudget};

    let mut budget = PreviewWorkBudget::with_pages(100, 2);
    let refused: Option<()> = budget.probe_page(|page_work| {
        *page_work = page_work.checked_sub(101)?;
        Some(())
    });
    assert_eq!(refused, None);
    assert_eq!(budget.remaining, 200, "a checked_sub refusal leaves the work untouched");

    let defaults = PreviewWorkBudget::new(&crate::ExtractionConfig::default());
    assert_eq!(defaults.per_page, 10_000_000 * PREVIEW_WORK_PER_ITERATION);
    assert_eq!(
        PREVIEW_FULL_BUDGET_PAGES, 6,
        "the document ceiling is six per-page budgets"
    );
    assert_eq!(
        defaults.remaining, 960_000_000,
        "six budgets of 160 million units at the default limit; this literal also moves with the default max_iterations"
    );

    let mut unbounded = PreviewWorkBudget::with_pages(usize::MAX, PREVIEW_FULL_BUDGET_PAGES);
    assert_eq!(
        unbounded.remaining,
        usize::MAX,
        "the ceiling saturates instead of overflowing"
    );
    assert_eq!(unbounded.probe_page(|page_work| *page_work), usize::MAX);
}

#[test]
fn the_ocr_security_limit_lowers_the_preview_budget() {
    use super::{PREVIEW_WORK_PER_ITERATION, PreviewWorkBudget};
    use crate::extractors::security::SecurityLimits;

    let limited = |top: usize, ocr: Option<usize>| crate::ExtractionConfig {
        security_limits: Some(SecurityLimits {
            max_iterations: top,
            ..Default::default()
        }),
        ocr: Some(crate::OcrConfig {
            security_limits: ocr.map(|max_iterations| SecurityLimits {
                max_iterations,
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    };
    let per_page = |config: crate::ExtractionConfig| PreviewWorkBudget::new(&config).per_page;
    assert_eq!(
        per_page(limited(5_000, Some(1_000))),
        1_000 * PREVIEW_WORK_PER_ITERATION
    );
    assert_eq!(
        per_page(limited(5_000, Some(9_000))),
        5_000 * PREVIEW_WORK_PER_ITERATION
    );
    assert_eq!(per_page(limited(5_000, None)), 5_000 * PREVIEW_WORK_PER_ITERATION);
}

#[test]
fn the_document_ceiling_stops_the_probe_after_the_pages_it_buys() {
    use super::PreviewWorkBudget;

    let bytes = include_bytes!("../../../../../tests/fixtures/ocr/hidden_layer_ccitt_2694/four-page-junk-layer.pdf");
    let doc = super::PdfDocument::from_bytes(bytes.to_vec()).unwrap();
    let page_cost = {
        let mut work = usize::MAX / 2;
        let start = work;
        assert_eq!(
            super::needs_ocr(&doc, 0, 100 * 1024 * 1024, &mut work, || false),
            Some(false)
        );
        start - work
    };
    // Each page may draw 1.2 page costs and the ceiling is two of those, 2.4 costs: two pages
    // fit and leave 0.4 of a cost, so the third is refused and so is the fourth.
    let per_page = page_cost + page_cost / 5;
    let mut budget = PreviewWorkBudget::with_pages(per_page, 2);
    let verdicts: Vec<Option<bool>> = (0..4)
        .map(|page| budget.probe_page(|work| super::needs_ocr(&doc, page, 100 * 1024 * 1024, work, || false)))
        .collect();
    assert_eq!(verdicts, [Some(false), Some(false), None, None]);
}

#[test]
fn the_default_budget_covers_six_pages_and_refuses_the_seventh() {
    use super::{PREVIEW_WORK_PER_ITERATION, PreviewWorkBudget};

    let bytes = include_bytes!("../../../../../tests/fixtures/ocr/hidden_layer_ccitt_2694/four-page-junk-layer.pdf");
    let doc = super::PdfDocument::from_bytes(bytes.to_vec()).unwrap();
    let mut work = usize::MAX / 2;
    let start = work;
    assert_eq!(
        super::needs_ocr(&doc, 0, 100 * 1024 * 1024, &mut work, || false),
        Some(false)
    );
    let page_cost = start - work;
    // Page 0 is probed seven times: only the budget accounting matters, not the page. Each
    // budget is 1.1 page costs, so six budgets are 6.6 costs; six probes spend 6.0 and the
    // remaining 0.6 of a cost is less than the seventh probe needs, so it is refused.
    let max_iterations = (page_cost + page_cost / 10).div_ceil(PREVIEW_WORK_PER_ITERATION);
    let config = crate::ExtractionConfig {
        security_limits: Some(crate::extractors::security::SecurityLimits {
            max_iterations,
            ..Default::default()
        }),
        ..Default::default()
    };
    let mut budget = PreviewWorkBudget::new(&config);
    let mut verdicts = Vec::new();
    for _ in 0..7 {
        verdicts.push(budget.probe_page(|work| super::needs_ocr(&doc, 0, 100 * 1024 * 1024, work, || false)));
    }
    assert_eq!(
        verdicts,
        [
            Some(false),
            Some(false),
            Some(false),
            Some(false),
            Some(false),
            Some(false),
            None
        ]
    );
}

#[test]
fn only_an_audited_covered_layer_that_is_not_judged_plausible_prose_is_probed() {
    use super::probes_page;

    assert!(probes_page(Some(true), true), "uncovered ink is always read");
    assert!(probes_page(Some(true), false));
    assert!(
        probes_page(Some(false), false),
        "the language check abstained or flagged the layer"
    );
    assert!(
        !probes_page(Some(false), true),
        "a plausible covering layer is kept without a read"
    );
    assert!(!probes_page(None, false), "an unaudited page is never probed");
    assert!(!probes_page(None, true));
}

const JUNK_LAYER: &str = "qzx vbk wrt plm ghj fdn cxs ztr bnm kld \
    hgf jdk slq wpe rty uio mnb vcx zas qwe";
const CONFIDENT_READ: &str = "The lighthouse keeper recorded every vessel that crossed the channel";

#[test]
fn a_confident_disjoint_read_replaces_the_layer_only_without_a_veto() {
    use super::confident_read_replaces_layer as replaces;
    use crate::core::config::OcrQualityThresholds;

    assert_eq!(JUNK_LAYER.split_whitespace().count(), 20);
    assert_eq!(
        OcrQualityThresholds::default().min_ocr_mean_confidence,
        75.0,
        "the 0.74 below sits just under the default floor it reads"
    );
    assert!(replaces(false, false, Some(0.9), JUNK_LAYER, CONFIDENT_READ));
    assert!(
        !replaces(true, false, Some(0.9), JUNK_LAYER, CONFIDENT_READ),
        "a low-confidence verdict keeps the layer"
    );
    assert!(
        !replaces(false, true, Some(0.9), JUNK_LAYER, CONFIDENT_READ),
        "a dictionary-noise verdict keeps the layer"
    );
    assert!(
        !replaces(false, false, Some(0.74), JUNK_LAYER, CONFIDENT_READ),
        "a score under the default floor keeps the layer"
    );
    assert!(
        !replaces(false, false, None, JUNK_LAYER, CONFIDENT_READ),
        "an uncalibrated score keeps the layer"
    );
    assert!(
        !replaces(false, false, Some(0.9), JUNK_LAYER, JUNK_LAYER),
        "a read that repeats the layer does not replace it"
    );
}

const PLAUSIBLE_PROSE: &str = "When in the course of human events it becomes necessary for one \
    people to dissolve the political bands which have connected them with another and to assume \
    among the powers of the earth the separate and equal station to which the laws of nature and \
    of natures god entitle them a decent respect to the opinions of mankind requires that they \
    should declare the causes which impel them to the separation we hold these truths to be self \
    evident that all men are created equal that they are endowed by their creator with certain \
    unalienable rights that among these are life liberty and the pursuit of happiness that to \
    secure these rights governments are instituted among men deriving their just powers from the \
    consent of the governed";

#[test]
fn a_covered_layer_is_plausible_only_when_the_language_check_says_so() {
    use super::layer_judged_plausible as judged;
    use crate::core::config::OcrQualityThresholds;

    let thresholds = OcrQualityThresholds::default();
    let shifted: String = PLAUSIBLE_PROSE
        .chars()
        .map(|c| match c {
            'a'..='z' => (((c as u8 - b'a' + 3) % 26) + b'a') as char,
            other => other,
        })
        .collect();
    assert!(judged(Some(false), Some(PLAUSIBLE_PROSE), &thresholds));
    assert!(
        !judged(Some(false), Some(&shifted), &thresholds),
        "a layer that reads as no language is not plausible"
    );
    assert!(
        !judged(Some(true), Some(PLAUSIBLE_PROSE), &thresholds),
        "uncovered ink is never plausible"
    );
    assert!(!judged(None, Some(PLAUSIBLE_PROSE), &thresholds));
    assert!(!judged(Some(false), None, &thresholds), "a page without layer text");
    let routing_off = OcrQualityThresholds {
        enable_plausibility_ocr_routing: false,
        ..OcrQualityThresholds::default()
    };
    assert!(
        !judged(Some(false), Some(PLAUSIBLE_PROSE), &routing_off),
        "with plausibility routing disabled nothing is judged plausible"
    );
}

#[test]
fn only_pages_whose_layer_is_not_judged_plausible_are_probed() {
    let decisions = vec![
        (0, Some(true), false),
        (1, Some(false), true),
        (2, Some(false), false),
        (3, None, false),
        (4, Some(true), true),
    ];
    assert_eq!(super::pages_to_probe(decisions), [0, 2, 4]);
}

#[test]
fn should_preserve_raw_and_ascii85_flate_stream_decoding_and_reject_other_chains() {
    use super::{Object, stream_filters};

    let expected = b"known stream samples ".repeat(8);
    let chain = Object::Array(vec![
        Object::Name("ASCII85Decode".into()),
        Object::Name("FlateDecode".into()),
    ]);
    let filters = stream_filters(Some(&chain)).expect("supported ASCII85 then Flate stream");
    assert!(matches!(filters, StreamFilters::Ascii85Flate));
    assert_eq!(
        decode_probe_stream(
            b"Gaut&b:h1?-nZr@:!?Tc:<Y<28M*ua*diP>>amuu~>",
            filters,
            expected.len(),
            4096
        ),
        Some(expected.clone())
    );
    assert_eq!(
        decode_probe_stream(&expected, stream_filters(None).unwrap(), expected.len(), 4096),
        Some(expected)
    );
    for filter in [
        Object::Array(vec![]),
        Object::Array(vec![Object::Name("ASCII85Decode".into())]),
        Object::Array(vec![
            Object::Name("FlateDecode".into()),
            Object::Name("ASCII85Decode".into()),
        ]),
        Object::Array(vec![
            Object::Name("FlateDecode".into()),
            Object::Name("FlateDecode".into()),
        ]),
        Object::Array(vec![Object::Null]),
    ] {
        assert!(stream_filters(Some(&filter)).is_none(), "{filter:?}");
    }
}
