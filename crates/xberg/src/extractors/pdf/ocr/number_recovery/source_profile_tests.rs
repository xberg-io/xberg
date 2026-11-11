use super::tests::{edit_font_program, edit_font_unicode, install_unused_latin_o, mutated, rebuild_font};
use super::*;

const BUDGET: usize = 32 * 1024 * 1024;

fn content(document: &mut lopdf::Document, bytes: Vec<u8>) {
    let stream = document.add_object(Stream::new(Dictionary::new(), bytes));
    let page = document.get_pages()[&1];
    document
        .get_object_mut(page)
        .unwrap()
        .as_dict_mut()
        .unwrap()
        .set("Contents", stream);
}

fn declined(input: &[u8], budget: usize) {
    let document = PdfDocument::from_bytes(input.to_vec()).unwrap();
    assert!(
        !preflight(&document, 0, budget).unwrap(),
        "source must decline before extraction"
    );
}

#[test]
fn should_decline_inline_actual_text_before_native_byte_reconstruction() {
    let input = mutated(|document, _| {
        content(
            document,
            b"BT /F2+0 24 Tf /Span << /ActualText <FEFF17E0> >> BDC <02> Tj EMC ET".to_vec(),
        );
    });
    declined(&input, BUDGET);
}

#[test]
fn should_decline_named_escaped_actual_text_properties_before_native() {
    let input = mutated(|document, _| {
        let properties =
            document.add_object(dictionary! { "ActualText" => lopdf::Object::string_literal("replacement") });
        let page = document.get_pages()[&1];
        let page = document.get_object_mut(page).unwrap().as_dict_mut().unwrap();
        page.get_mut(b"Resources")
            .unwrap()
            .as_dict_mut()
            .unwrap()
            .set("Properties", dictionary! { "P" => properties });
        content(document, b"BT /F2+0 24 Tf /Span /#50 BDC <02> Tj EMC ET".to_vec());
    });
    declined(&input, BUDGET);
}

#[test]
fn should_decline_all_marked_content_and_unknown_text_producers() {
    for bytes in [
        b"/Span BMC EMC".as_slice(),
        b"/GS1 gs",
        b"BI /W 1 ID a EI",
        b"BT (x) unknown ET",
    ] {
        declined(&mutated(|document, _| content(document, bytes.to_vec())), BUDGET);
    }
}

#[test]
fn should_decline_oversized_compressed_contents_before_native() {
    let input = mutated(|document, _| {
        content(document, vec![b' '; 16 * 1024 + 1]);
        let page = document.get_pages()[&1];
        let stream = document
            .get_object(page)
            .unwrap()
            .as_dict()
            .unwrap()
            .get(b"Contents")
            .unwrap()
            .as_reference()
            .unwrap();
        document
            .get_object_mut(stream)
            .unwrap()
            .as_stream_mut()
            .unwrap()
            .compress()
            .unwrap();
    });
    declined(&input, BUDGET);
}

#[test]
fn should_decline_nested_content_arrays_before_recursive_parser() {
    for bytes in [b"[[[(x)]]] TJ".as_slice(), b"<< /A << /B 1 >> >> unknown"] {
        declined(&mutated(|document, _| content(document, bytes.to_vec())), BUDGET);
    }
}

#[test]
fn should_count_repeated_shows_before_native_overlap_deduplication() {
    let input = mutated(|document, _| {
        let mut bytes = b"BT /F2+0 24 Tf ".to_vec();
        for _ in 0..100 {
            bytes.extend_from_slice(b"1 0 0 1 100 650 Tm (aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa) Tj ");
        }
        bytes.extend_from_slice(b"1 0 0 1 300 650 Tm <02> Tj ET");
        content(document, bytes);
    });
    let document = PdfDocument::from_bytes(input.clone()).unwrap();
    let chars = document.extract_chars(0).unwrap();
    assert!(
        chars.len() <= 44,
        "100 overlapping shows deduplicate to a small final vector"
    );
    assert_eq!(super::super::candidates(&chars).len(), 1);
    declined(&input, BUDGET);
}

#[test]
fn should_decline_huge_font_names_before_character_string_allocation() {
    let input = mutated(|document, font| {
        document
            .get_object_mut(font)
            .unwrap()
            .as_dict_mut()
            .unwrap()
            .set("BaseFont", "X".repeat(257).as_str());
    });
    declined(&input, BUDGET);
}

fn alternate_actual_text() -> Vec<u8> {
    mutated(|document, font| {
        edit_font_unicode(document, font, |text| {
            text.replace("<02> <17E0>", "<02> <0041>")
                .replace("endcmap", "1 beginbfchar\n<E1> <17E0>\nendbfchar\nendcmap")
        });
        let dict = document.get_object_mut(font).unwrap().as_dict_mut().unwrap();
        let widths = dict.get_mut(b"Widths").unwrap().as_array_mut().unwrap();
        let advance = widths[2].clone();
        widths.resize(256, 0.into());
        widths[225] = advance;
        dict.set("LastChar", 255);
        edit_font_program(document, font, |program| {
            install_unused_latin_o(program);
            let tables = font_tables(program).unwrap();
            let old = tables[b"cmap".as_slice()];
            let offset = read_u32(old, 8).unwrap() as usize;
            let mut cmap = old[..offset + 10].to_vec();
            cmap[offset + 2..offset + 4].copy_from_slice(&522_u16.to_be_bytes());
            cmap[offset + 8..offset + 10].copy_from_slice(&256_u16.to_be_bytes());
            cmap.resize(offset + 522, 0);
            cmap[offset + 10 + 225 * 2..offset + 12 + 225 * 2].copy_from_slice(&16_u16.to_be_bytes());
            rebuild_font(program, &[(b"cmap", cmap)]);
            assert!(byte_indexed_truetype(program));
        });
        content(
            document,
            b"BT /F2+0 24 Tf 1 0 0 1 100 650 Tm /Span << /ActualText <FEFF17E0> >> BDC <02> Tj EMC ET".to_vec(),
        );
    })
}

#[test]
fn should_decline_real_unused_latin_o_byte_selected_by_actual_text_utf8() {
    let input = alternate_actual_text();
    let document = PdfDocument::from_bytes(input).unwrap();
    let chars = document.extract_chars(0).unwrap();
    assert_eq!(
        chars.iter().filter(|glyph| glyph.char == '០').count(),
        1,
        "Native decodes unused E1 as Khmer zero"
    );
    assert!(!preflight(&document, 0, BUDGET).unwrap());
}

#[test]
fn should_reserve_source_peak_before_nonzero_small_budget_extraction() {
    let input = mutated(|_, _| {});
    declined(&input, 11 * 1024 * 1024);
}

#[test]
fn should_decline_unsupported_contents_filters_and_parameters() {
    for key in ["Filter", "DecodeParms"] {
        let input = mutated(|document, _| {
            let mut dict = Dictionary::new();
            dict.set(key, "ASCIIHexDecode");
            let stream = document.add_object(Stream::new(dict, b"20>".to_vec()));
            let page = document.get_pages()[&1];
            document
                .get_object_mut(page)
                .unwrap()
                .as_dict_mut()
                .unwrap()
                .set("Contents", stream);
        });
        declined(&input, BUDGET);
    }
}

#[test]
fn should_decline_cyclic_and_excessive_indirect_contents_arrays() {
    for count in [1, 33] {
        let input = mutated(|document, _| {
            let stream = document.add_object(Stream::new(Dictionary::new(), b"q Q".to_vec()));
            let array = document.new_object_id();
            let values = if count == 1 {
                vec![array.into()]
            } else {
                vec![stream.into(); count]
            };
            document.objects.insert(array, lopdf::Object::Array(values));
            let page = document.get_pages()[&1];
            document
                .get_object_mut(page)
                .unwrap()
                .as_dict_mut()
                .unwrap()
                .set("Contents", array);
        });
        declined(&input, BUDGET);
    }
}

#[test]
fn should_decline_tagged_pages_and_external_graphics_state_resources() {
    for key in ["StructTreeRoot", "ExtGState", "StructParents"] {
        let input = mutated(|document, _| {
            let page = document.get_pages()[&1];
            if key == "StructTreeRoot" {
                let root = document.trailer.get(b"Root").unwrap().as_reference().unwrap();
                document
                    .get_object_mut(root)
                    .unwrap()
                    .as_dict_mut()
                    .unwrap()
                    .set(key, Dictionary::new());
            } else {
                let page = document.get_object_mut(page).unwrap().as_dict_mut().unwrap();
                if key == "StructParents" {
                    page.set(key, 0);
                } else {
                    page.get_mut(b"Resources")
                        .unwrap()
                        .as_dict_mut()
                        .unwrap()
                        .set(key, Dictionary::new());
                }
            }
        });
        declined(&input, BUDGET);
    }
}

#[test]
fn should_decline_excessive_source_operations_and_literal_nesting() {
    for bytes in [b"0 g ".repeat(1025), b"((((((((((a)))))))))) Tj".to_vec()] {
        declined(&mutated(|document, _| content(document, bytes)), BUDGET);
    }
}

#[test]
fn should_prepare_genuine_source_through_the_single_bounded_entry() {
    let input = mutated(|_, _| {});
    let document = PdfDocument::from_bytes(input).unwrap();
    let (numbers, proof) = prepare_source(&document, 0, BUDGET, 64).unwrap().unwrap();
    assert_eq!(numbers.len(), 1);
    assert_eq!(numbers[0].text, "០");
    let admitted = source_profile::admit(&document, 0, BUDGET).unwrap();
    assert!(proof.retained_bytes() >= admitted.working_bytes);
    let media = document.get_page_info(0).unwrap().media_box;
    let rendered = rendering::render_page(&document, 0, &rendering::RenderOptions::with_dpi(150)).unwrap();
    let raster = image::load_from_memory(&rendered.data).unwrap();
    assert!(proof.corroborates(0, &raster, media, &numbers[0], BUDGET).unwrap());
    assert!(prepare_source(&document, 0, BUDGET, 0).unwrap().is_none());
}

#[test]
fn should_accept_bounded_indirect_contents_arrays_and_count_tj_spaces() {
    let input = mutated(|document, _| {
        let first = document.add_object(Stream::new(
            Dictionary::new(),
            b"BT /F2+0 24 Tf 1 0 0 1 100 650 Tm".to_vec(),
        ));
        let second = document.add_object(Stream::new(
            Dictionary::new(),
            b"[<02> -3000 -3000 -3000 -3000] TJ ET".to_vec(),
        ));
        let array = document.add_object(lopdf::Object::Array(vec![first.into(), second.into()]));
        let page = document.get_pages()[&1];
        document
            .get_object_mut(page)
            .unwrap()
            .as_dict_mut()
            .unwrap()
            .set("Contents", array);
    });
    let document = PdfDocument::from_bytes(input).unwrap();
    assert!(preflight(&document, 0, BUDGET).unwrap());
    let admitted = source_profile::admit(&document, 0, BUDGET).unwrap();
    assert_eq!(admitted.max_chars, 12, "eight scalars for one byte plus four TJ spaces");
    let (numbers, _) = prepare_source(&document, 0, BUDGET, 64).unwrap().unwrap();
    assert_eq!(numbers.len(), 1);
    assert_eq!(numbers[0].text, "០");
}

#[test]
fn should_decline_actual_text_in_both_source_and_shape_entry_points() {
    let document = PdfDocument::from_bytes(alternate_actual_text()).unwrap();
    let chars = document.extract_chars(0).unwrap();
    let numbers = super::super::candidates(&chars);
    assert_eq!(numbers.len(), 1);
    assert_eq!(numbers[0].text, "០");
    assert!(prepare(&document, 0, &numbers, BUDGET).unwrap().is_none());
    assert!(prepare_source(&document, 0, BUDGET, 64).unwrap().is_none());
}

#[test]
fn should_bound_font_aliases_and_descriptor_names_before_loading() {
    for descriptor in [false, true] {
        let input = mutated(|document, font| {
            if descriptor {
                let descriptor = document
                    .get_object(font)
                    .unwrap()
                    .as_dict()
                    .unwrap()
                    .get(b"FontDescriptor")
                    .unwrap()
                    .as_reference()
                    .unwrap();
                document
                    .get_object_mut(descriptor)
                    .unwrap()
                    .as_dict_mut()
                    .unwrap()
                    .set("FontName", "X".repeat(129).as_str());
            } else {
                let fonts = document.get_object_mut((1, 0)).unwrap().as_dict_mut().unwrap();
                let object = fonts.remove(b"F2+0").unwrap();
                fonts.set("X".repeat(129).as_str(), object);
            }
        });
        declined(&input, BUDGET);
    }
}

#[test]
fn should_bound_total_decoded_contents_across_indirect_streams() {
    let input = mutated(|document, _| {
        let mut streams = Vec::new();
        for _ in 0..2 {
            let mut stream = Stream::new(Dictionary::new(), vec![b' '; 9000]);
            stream.compress().unwrap();
            streams.push(document.add_object(stream).into());
        }
        let array = document.add_object(lopdf::Object::Array(streams));
        let page = document.get_pages()[&1];
        document
            .get_object_mut(page)
            .unwrap()
            .as_dict_mut()
            .unwrap()
            .set("Contents", array);
    });
    declined(&input, BUDGET);
}

fn source_with_array_comment(payload: &[u8]) -> (Vec<u8>, Vec<u8>) {
    let mut document = lopdf::Document::load_mem(&alternate_actual_text()).unwrap();
    let mut bytes = b"BT /F2+0 24 Tf ( ) Tj 1 0 0 1 100 650 Tm [ % ".to_vec();
    bytes.extend_from_slice(payload);
    bytes.extend_from_slice(b"\n] TJ ET");
    content(&mut document, bytes.clone());
    let mut input = Vec::new();
    document.save_to(&mut input).unwrap();
    (input, bytes)
}

#[test]
fn should_decline_array_comments_that_conceal_actual_text_from_admission() {
    let (input, _) = source_with_array_comment(b"] TJ /Span << /ActualText <FEFF17E0> >> BDC <02> Tj EMC");
    let document = PdfDocument::from_bytes(input.clone()).unwrap();
    assert!(
        document.extract_chars(0).unwrap().iter().any(|glyph| glyph.char == '០'),
        "Native actually executes the commented ActualText replacement"
    );
    declined(&input, BUDGET);
}

#[test]
fn should_decline_array_comments_that_hide_character_allocation() {
    use xberg_native_pdf::content::{Operator, TextElement};
    let mut literal = vec![b'('; 1];
    literal.extend_from_slice(&vec![b'a'; 5000]);
    literal.push(b')');
    let (input, bytes) = source_with_array_comment(&literal);
    let mut oversized = false;
    xberg_native_pdf::content::parser::parse_and_execute_text_only(&bytes, |operation| {
        oversized |= matches!(operation,
            Operator::TJ {array} if array.iter().any(|item| matches!(item,
                TextElement::String(text) if text.len() == 5000)));
        Ok(())
    })
    .unwrap();
    assert!(
        oversized,
        "the real character extractor's parser consumes the commented literal"
    );
    declined(&input, BUDGET);
}

fn source_with_terminal_comment(payload: &[u8]) -> (Vec<u8>, Vec<u8>) {
    let mut document = lopdf::Document::load_mem(&alternate_actual_text()).unwrap();
    let mut bytes = b"BT /F2+0 24 Tf ( ) Tj 1 0 0 1 100 650 Tm % ".to_vec();
    bytes.extend_from_slice(payload);
    bytes.push(b'\n');
    content(&mut document, bytes.clone());
    let mut input = Vec::new();
    document.save_to(&mut input).unwrap();
    (input, bytes)
}

#[test]
fn should_decline_terminal_comments_that_replay_actual_text_at_eof() {
    let (input, _) = source_with_terminal_comment(b"/Span << /ActualText <FEFF17E0> >> BDC <02> Tj EMC ET");
    let document = PdfDocument::from_bytes(input.clone()).unwrap();
    assert!(
        document.extract_chars(0).unwrap().iter().any(|glyph| glyph.char == '០'),
        "Native replays terminal ActualText even with the appended newline"
    );
    declined(&input, BUDGET);
}

#[test]
fn should_decline_terminal_comments_that_hide_character_allocation_at_eof() {
    use xberg_native_pdf::content::Operator;
    let mut literal = vec![b'('; 1];
    literal.extend_from_slice(&vec![b'a'; 5000]);
    literal.extend_from_slice(b") Tj ET");
    let (input, bytes) = source_with_terminal_comment(&literal);
    let mut oversized = false;
    xberg_native_pdf::content::parser::parse_and_execute_text_only(&bytes, |operation| {
        oversized |= matches!(operation, Operator::Tj {text} if text.len() == 5000);
        Ok(())
    })
    .unwrap();
    assert!(
        oversized,
        "Native replays terminal text before the allocation invariant runs"
    );
    declined(&input, BUDGET);
}
