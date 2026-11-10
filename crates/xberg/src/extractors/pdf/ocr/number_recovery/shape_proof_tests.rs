use super::*;
use xberg_native_pdf::rendering::{self, RenderOptions};

const MATCHING: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/pdf_khmer_number_2408/matching.pdf"
));
const LATIN_O: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/pdf_khmer_number_2408/painted_latin_o.pdf"
));
const ASCII_ZERO: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/pdf_khmer_number_2408/painted_ascii_zero.pdf"
));
const BUDGET: usize = 32 * 1024 * 1024;

fn fixture(input: &[u8]) -> (PdfDocument, Rect, Vec<Number>, DynamicImage) {
    let (document, numbers) = native_fixture(input);
    let media = document.get_page_info(0).unwrap().media_box;
    let rendered = rendering::render_page(&document, 0, &RenderOptions::with_dpi(150)).unwrap();
    let raster = image::load_from_memory(&rendered.data).unwrap();
    (document, media, numbers, raster)
}

fn proves(input: &[u8]) -> bool {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<ShapeProof>();
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter("xberg::extractors::pdf::ocr::number_recovery::shape_proof=debug")
        .with_test_writer()
        .finish();
    let _guard = tracing::subscriber::set_default(subscriber);
    let (document, media, numbers, raster) = fixture(input);
    let proof = prepare(&document, 0, &numbers, BUDGET).unwrap().unwrap_or_else(|| {
        panic!(
            "supported real embedded font: {:?}",
            prepare_inner(&document, 0, &numbers, BUDGET).err()
        );
    });
    println!(
        "shape template: candidates={}, retained_charge={}",
        proof.candidates.len(),
        proof.retained_bytes
    );
    proof.corroborates(0, &raster, media, &numbers[0], BUDGET).unwrap()
}

#[test]
fn should_corroborate_genuine_khmer_zero_from_actual_font_program() {
    assert!(proves(MATCHING));
}

#[test]
fn should_corroborate_genuine_shape_with_native_canvas_rounding() {
    let input = mutated(|document, _| {
        let page = document.get_pages()[&1];
        let media: Vec<lopdf::Object> = [0.0_f32, 0.0, 595.28, 841.89].map(lopdf::Object::from).to_vec();
        document
            .get_object_mut(page)
            .unwrap()
            .as_dict_mut()
            .unwrap()
            .set("MediaBox", media);
    });
    assert!(proves(&input));
}

#[test]
fn should_refuse_painted_latin_o_under_hidden_khmer_zero() {
    assert!(!proves(LATIN_O));
}

#[test]
fn should_refuse_painted_ascii_zero_under_hidden_khmer_zero() {
    assert!(!proves(ASCII_ZERO));
}

#[test]
fn should_corroborate_unchanged_outline_despite_wrong_to_unicode() {
    let input = wrong_unicode();
    let (_, numbers) = native_fixture(&input);
    assert_eq!(numbers[0].text, "១");
    assert!(proves(&input), "shape does not certify the Unicode value");
}

#[test]
fn should_refuse_blank_source_and_zero_render_budget() {
    let (document, media, numbers, raster) = fixture(MATCHING);
    let proof = prepare(&document, 0, &numbers, BUDGET).unwrap().unwrap();
    let blank = DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
        raster.width(),
        raster.height(),
        image::Rgb([255; 3]),
    ));
    assert!(!proof.corroborates(0, &blank, media, &numbers[0], BUDGET).unwrap());
    assert!(!proof.corroborates(0, &raster, media, &numbers[0], 0).unwrap());
}

#[test]
fn should_decline_preparation_without_byte_budget() {
    let (document, _, numbers, _) = fixture(MATCHING);
    assert!(prepare(&document, 0, &numbers, 0).unwrap().is_none());
}

#[test]
fn should_decline_unsupported_font_before_native_character_decode() {
    let input = mutated(|document, font| {
        document
            .get_object_mut(font)
            .unwrap()
            .as_dict_mut()
            .unwrap()
            .set("Subtype", "Type0");
    });
    assert_declined(&input, "unsupported page font subtype");
}

#[test]
fn should_decline_unbounded_unicode_ranges_before_native_character_decode() {
    let input = appended_cmap("1 beginbfrange\n<00> <FFFFFFFF> <17E0>\nendbfrange\n");
    assert_declined(&input, "unsupported ToUnicode program");
}

#[test]
fn should_decline_ambiguous_physical_code_even_when_claim_is_unchanged() {
    let input = appended_cmap("1 beginbfchar\n<03> <17E0>\nendbfchar\n");
    assert_declined(&input, "ambiguous physical character code");
}

#[test]
fn should_decline_cyclic_or_excessive_font_object_graphs() {
    let cycle = mutated(|document, font| {
        document
            .get_object_mut(font)
            .unwrap()
            .as_dict_mut()
            .unwrap()
            .set("Cycle", font);
    });
    assert_declined(&cycle, "cyclic font dependency");
    let deep = mutated(|document, font| {
        let nested = (0..MAX_DEPTH + 1).fold(lopdf::Object::Null, |value, _| lopdf::Object::Array(vec![value]));
        document
            .get_object_mut(font)
            .unwrap()
            .as_dict_mut()
            .unwrap()
            .set("Nested", nested);
    });
    assert_declined(&deep, "font dependency depth");
    let many = mutated(|document, font| {
        let objects: Vec<_> = (0..MAX_OBJECTS + 1)
            .map(|_| document.add_object(1_i64).into())
            .collect();
        document
            .get_object_mut(font)
            .unwrap()
            .as_dict_mut()
            .unwrap()
            .set("Objects", objects);
    });
    assert_declined(&many, "font object count");
}

#[test]
fn should_decline_nonbyte_cmap_formats_before_native_font_loading() {
    let input = mutated(|document, font| {
        let descriptor = document
            .get_object(font)
            .unwrap()
            .as_dict()
            .unwrap()
            .get(b"FontDescriptor")
            .unwrap()
            .as_reference()
            .unwrap();
        let file = document
            .get_object(descriptor)
            .unwrap()
            .as_dict()
            .unwrap()
            .get(b"FontFile2")
            .unwrap()
            .as_reference()
            .unwrap();
        let stream = document.get_object_mut(file).unwrap().as_stream_mut().unwrap();
        let mut program = stream.decompressed_content().unwrap();
        assert!(byte_indexed_truetype(&program));
        let record = program.windows(4).position(|tag| tag == b"cmap").unwrap();
        let cmap = u32::from_be_bytes(program[record + 8..record + 12].try_into().unwrap()) as usize;
        let offset = u32::from_be_bytes(program[cmap + 8..cmap + 12].try_into().unwrap()) as usize;
        assert_eq!(read_u16(&program, cmap + offset), Some(6));
        program[cmap + offset + 1] = 12;
        assert!(
            !byte_indexed_truetype(&program),
            "unbounded cmap formats must not reach Native"
        );
        stream.set_plain_content(program);
    });
    assert_declined(&input, "unsupported embedded font cmap");
}

#[test]
fn should_decline_oversized_raw_and_compressed_font_streams() {
    for compressed in [false, true] {
        let input = mutated(|document, font| {
            let descriptor = document
                .get_object(font)
                .unwrap()
                .as_dict()
                .unwrap()
                .get(b"FontDescriptor")
                .unwrap()
                .as_reference()
                .unwrap();
            let file = document
                .get_object(descriptor)
                .unwrap()
                .as_dict()
                .unwrap()
                .get(b"FontFile2")
                .unwrap()
                .as_reference()
                .unwrap();
            let stream = document.get_object_mut(file).unwrap().as_stream_mut().unwrap();
            stream.set_plain_content(vec![0; MAX_FONT_BYTES + 1]);
            if compressed {
                stream.compress().unwrap();
            }
        });
        assert_declined(&input, "font byte budget");
    }
}

#[test]
fn should_decline_ambiguous_fonts_and_transformed_glyphs() {
    let duplicate = mutated(|document, font| {
        let copy = document.add_object(document.get_object(font).unwrap().clone());
        let resources = document.get_object_mut((1, 0)).unwrap().as_dict_mut().unwrap();
        resources.set("Duplicate", copy);
    });
    assert_declined(&duplicate, "ambiguous font identity");
    let transformed = mutated(|document, _| {
        let mut changed = 0;
        for object in document.objects.values_mut() {
            let Ok(stream) = object.as_stream_mut() else { continue };
            let Ok(bytes) = stream.decompressed_content() else {
                continue;
            };
            let Ok(text) = String::from_utf8(bytes) else { continue };
            if text.contains("1 0 0 1 100 650 Tm") {
                stream.set_plain_content(text.replace("1 0 0 1 100 650 Tm", "1.1 0 0 1 100 650 Tm").into_bytes());
                changed += 1;
            }
        }
        assert_eq!(changed, 1);
    });
    let (document, numbers) = native_fixture(&transformed);
    assert!(prepare(&document, 0, &numbers, BUDGET).unwrap().is_none());
}

#[test]
fn should_refuse_rebound_candidates_anisotropic_rasters_and_extra_source_ink() {
    let (document, media, numbers, raster) = fixture(MATCHING);
    let proof = prepare(&document, 0, &numbers, BUDGET).unwrap().unwrap();
    assert!(!proof.corroborates(1, &raster, media, &numbers[0], BUDGET).unwrap());
    let rebound = Number {
        text: "១".into(),
        bbox: numbers[0].bbox,
        baseline: numbers[0].baseline,
    };
    assert!(!proof.corroborates(0, &raster, media, &rebound, BUDGET).unwrap());
    let anisotropic = raster.crop_imm(0, 0, raster.width() - 1, raster.height());
    assert!(!proof.corroborates(0, &anisotropic, media, &numbers[0], BUDGET).unwrap());
    let black = DynamicImage::ImageRgb8(image::RgbImage::new(raster.width(), raster.height()));
    assert!(!proof.corroborates(0, &black, media, &numbers[0], BUDGET).unwrap());
    let mut transparent = raster.to_rgba8();
    transparent.pixels_mut().for_each(|pixel| pixel[3] = 0);
    let transparent = DynamicImage::ImageRgba8(transparent);
    assert!(!proof.corroborates(0, &transparent, media, &numbers[0], BUDGET).unwrap());
}

fn native_fixture(input: &[u8]) -> (PdfDocument, Vec<Number>) {
    let document = PdfDocument::from_bytes(input.to_vec()).unwrap();
    assert!(preflight(&document, 0, BUDGET).unwrap());
    let source = bounded_source(&document, 0, BUDGET).unwrap();
    let numbers = super::super::candidates(&source.chars);
    assert_eq!(numbers.len(), 1, "exactly one real numeric candidate");
    (document, numbers)
}

fn assert_declined(input: &[u8], reason: &str) {
    let (_, numbers) = native_fixture(MATCHING);
    let document = PdfDocument::from_bytes(input.to_vec()).unwrap();
    match prepare_inner(&document, 0, &numbers, BUDGET) {
        Err(Failure::Declined(observed)) => assert_eq!(observed, reason),
        Err(error) => panic!("wrong decline for {reason}: {error}"),
        Ok(_) => panic!("must decline: {reason}"),
    }
}

pub(super) fn mutated(change: impl FnOnce(&mut lopdf::Document, ObjectId)) -> Vec<u8> {
    let mut document = lopdf::Document::load_mem(MATCHING).unwrap();
    let fonts: Vec<_> = document
        .objects
        .iter()
        .filter_map(|(id, object)| {
            (object.as_dict().ok()?.get(b"Subtype").ok()?.as_name().ok()? == b"TrueType").then_some(*id)
        })
        .collect();
    assert_eq!(fonts.len(), 1);
    change(&mut document, fonts[0]);
    let mut bytes = Vec::new();
    document.save_to(&mut bytes).unwrap();
    bytes
}

pub(super) fn edit_font_unicode(document: &mut lopdf::Document, font: ObjectId, edit: impl FnOnce(String) -> String) {
    let cmap = document
        .get_object(font)
        .unwrap()
        .as_dict()
        .unwrap()
        .get(b"ToUnicode")
        .unwrap()
        .as_reference()
        .unwrap();
    let stream = document.get_object_mut(cmap).unwrap().as_stream_mut().unwrap();
    let content = String::from_utf8(stream.decompressed_content().unwrap()).unwrap();
    stream.set_plain_content(edit(content).into_bytes());
}

fn edited_cmap(edit: impl FnOnce(String) -> String) -> Vec<u8> {
    mutated(|document, font| edit_font_unicode(document, font, edit))
}

fn appended_cmap(addition: &str) -> Vec<u8> {
    edited_cmap(|content| {
        assert_eq!(content.matches("endcmap").count(), 1);
        content.replace("endcmap", &format!("{addition}endcmap"))
    })
}

fn wrong_unicode() -> Vec<u8> {
    edited_cmap(|content| {
        assert_eq!(content.matches("<02> <17E0>").count(), 1, "one ToUnicode claim");
        content.replace("<02> <17E0>", "<02> <17E1>")
    })
}

pub(super) fn edit_font_program(document: &mut lopdf::Document, font: ObjectId, edit: impl FnOnce(&mut Vec<u8>)) {
    let descriptor = document
        .get_object(font)
        .unwrap()
        .as_dict()
        .unwrap()
        .get(b"FontDescriptor")
        .unwrap()
        .as_reference()
        .unwrap();
    let file = document
        .get_object(descriptor)
        .unwrap()
        .as_dict()
        .unwrap()
        .get(b"FontFile2")
        .unwrap()
        .as_reference()
        .unwrap();
    let stream = document.get_object_mut(file).unwrap().as_stream_mut().unwrap();
    let mut program = stream.decompressed_content().unwrap();
    edit(&mut program);
    stream.set_plain_content(program);
}

fn mutated_program(edit: impl FnOnce(&mut Vec<u8>)) -> Vec<u8> {
    mutated(|document, font| edit_font_program(document, font, edit))
}

pub(super) fn table_start(program: &[u8], tag: &[u8]) -> usize {
    let count = read_u16(program, 4).unwrap() as usize;
    let record = program[12..12 + count * 16]
        .as_chunks::<16>()
        .0
        .iter()
        .find(|record| &record[..4] == tag)
        .unwrap();
    u32::from_be_bytes(record[8..12].try_into().unwrap()) as usize
}

fn assert_audit_declined(input: &[u8]) {
    let document = PdfDocument::from_bytes(input.to_vec()).unwrap();
    let resources = document.get_page_resources(0).unwrap();
    assert!(!preflight(&document, 0, BUDGET).unwrap());
    let audit = page_fonts(&document, &resources).and_then(|fonts| Builder::new(&document, BUDGET).audit_fonts(&fonts));
    assert!(
        audit.is_err(),
        "hostile font must decline before native font loading/extraction"
    );
}

#[test]
fn should_decline_long_claim_with_unused_alternate_byte_before_native() {
    let input = mutated(|document, font| {
        edit_font_unicode(document, font, |text| {
            text.replace("<02> <17E0>", "<02> <17E000610061006100610061006100610061>")
                .replace("<03> <0000>", "<03> <17E0>")
        });
        let widths = document
            .get_object_mut(font)
            .unwrap()
            .as_dict_mut()
            .unwrap()
            .get_mut(b"Widths")
            .unwrap()
            .as_array_mut()
            .unwrap();
        widths[3] = widths[2].clone();
        assert_eq!(widths[2], widths[3], "equal source and alternate PDF advances");
        edit_font_program(document, font, install_unused_latin_o);
    });
    assert_audit_declined(&input);
}

#[test]
fn should_decline_multiscalar_and_surrogate_claims_before_native() {
    for claim in ["17E00061", "D800", "D800DC00"] {
        assert_audit_declined(&edited_cmap(|text| {
            text.replace("<02> <17E0>", &format!("<02> <{claim}>"))
        }));
    }
}

#[test]
fn should_decline_real_font_cmap_ranges_and_truncation_before_native() {
    for (field, value) in [(6, 255_u16), (8, 257), (2, 9)] {
        let input = mutated_program(|program| {
            let cmap = table_start(program, b"cmap");
            let offset = u32::from_be_bytes(program[cmap + 8..cmap + 12].try_into().unwrap()) as usize;
            let start = cmap + offset + field;
            program[start..start + 2].copy_from_slice(&value.to_be_bytes());
        });
        assert_audit_declined(&input);
    }
}

#[test]
fn should_decline_real_font_allocator_maxima_and_composites_before_native() {
    for field in [4, 6, 8, 10, 12, 14, 16, 18, 20, 22, 24, 26, 28, 30] {
        let input = mutated_program(|program| {
            let start = table_start(program, b"maxp") + field;
            program[start..start + 2].copy_from_slice(&u16::MAX.to_be_bytes());
        });
        assert_audit_declined(&input);
    }
    let input = mutated_program(|program| {
        let start = table_start(program, b"glyf");
        program[start..start + 2].copy_from_slice(&(-1_i16).to_be_bytes());
    });
    assert_audit_declined(&input);
}

#[test]
fn should_decline_real_font_actual_outline_overflow_despite_small_maxp() {
    for (field, value) in [(0, 32767_u16), (10, 65534)] {
        let input = mutated_program(|program| {
            let start = table_start(program, b"glyf") + field;
            program[start..start + 2].copy_from_slice(&value.to_be_bytes());
        });
        assert_audit_declined(&input);
    }
}

#[test]
fn should_reserve_font_working_bytes_for_nonzero_render_budget() {
    let (document, media, numbers, raster) = fixture(MATCHING);
    let proof = prepare(&document, 0, &numbers, BUDGET).unwrap().unwrap();
    let budget = proof.retained_bytes() + 512 * 1024;
    assert!(matches!(
        proof.compare(0, &raster, media, &numbers[0], budget),
        Err(Failure::Declined("glyph font working byte budget"))
    ));
}

pub(super) fn install_unused_latin_o(program: &mut Vec<u8>) {
    let latin = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../xberg-native-pdf/src/fonts/assets/DejaVuSans.ttf"
    ));
    // Pinned DejaVuSans glyph 50 is Latin O; transplant its real simple outline into unused GID 16. ~keep
    let loca = table_start(latin, b"loca");
    let glyf = table_start(latin, b"glyf");
    assert_eq!(read_u16(latin, table_start(latin, b"head") + 50), Some(1));
    let start = read_u32(latin, loca + 50 * 4).unwrap() as usize;
    let end = read_u32(latin, loca + 51 * 4).unwrap() as usize;
    let outline = &latin[glyf + start..glyf + end];
    assert_eq!(read_u16(outline, 0), Some(2));
    let tables = font_tables(program).unwrap();
    let mut glyf = tables[b"glyf".as_slice()].to_vec();
    let mut loca = tables[b"loca".as_slice()].to_vec();
    let start = usize::from(read_u16(&loca, 16 * 2).unwrap()) * 2;
    glyf.truncate(start);
    glyf.extend_from_slice(outline);
    if !glyf.len().is_multiple_of(2) {
        glyf.push(0);
    }
    loca[34..36].copy_from_slice(&((glyf.len() / 2) as u16).to_be_bytes());
    let mut cmap = tables[b"cmap".as_slice()].to_vec();
    let subtable = read_u32(&cmap, 8).unwrap() as usize;
    cmap[subtable + 10 + 3 * 2..subtable + 12 + 3 * 2].copy_from_slice(&16_u16.to_be_bytes());
    let replacements = [
        (b"glyf".as_slice(), glyf),
        (b"loca".as_slice(), loca),
        (b"cmap".as_slice(), cmap),
    ];
    rebuild_font(program, &replacements);
    assert!(byte_indexed_truetype(program), "font profile itself remains admitted");
}

pub(super) fn rebuild_font(program: &mut Vec<u8>, replacements: &[(&[u8], Vec<u8>)]) {
    let count = usize::from(read_u16(program, 4).unwrap());
    let tables = font_tables(program).unwrap();
    let mut output = program[..12 + count * 16].to_vec();
    for index in 0..count {
        let record = 12 + index * 16;
        let tag = &program[record..record + 4];
        let data = replacements
            .iter()
            .find(|(key, _)| *key == tag)
            .map_or(tables[tag], |(_, bytes)| bytes.as_slice());
        while !output.len().is_multiple_of(4) {
            output.push(0);
        }
        let offset = output.len() as u32;
        output[record + 8..record + 12].copy_from_slice(&offset.to_be_bytes());
        output[record + 12..record + 16].copy_from_slice(&(data.len() as u32).to_be_bytes());
        output.extend_from_slice(data);
    }
    *program = output;
}

#[test]
fn should_decline_broken_loca_and_actual_instruction_lengths_before_native() {
    for (tag, field) in [(b"loca", 2), (b"glyf", 14)] {
        let input = mutated_program(|program| {
            let offset = table_start(program, tag) + field;
            program[offset..offset + 2].copy_from_slice(&u16::MAX.to_be_bytes());
        });
        assert_audit_declined(&input);
    }
}

#[test]
fn should_decline_unaudited_form_font_resources_before_native() {
    let input = mutated(|document, _| {
        let form = document.add_object(lopdf::Stream::new(
            dictionary! {"Subtype" => "Form"},
            b"BT /Foreign 1 Tf ET".to_vec(),
        ));
        let page = document.get_pages()[&1];
        document
            .get_object_mut(page)
            .unwrap()
            .as_dict_mut()
            .unwrap()
            .get_mut(b"Resources")
            .unwrap()
            .as_dict_mut()
            .unwrap()
            .set("XObject", dictionary! {"Hostile" => form});
    });
    assert_audit_declined(&input);
}

#[test]
fn should_decline_nonzero_preflight_budget_below_native_font_cache_reservation() {
    let document = PdfDocument::from_bytes(MATCHING.to_vec()).unwrap();
    assert!(!preflight(&document, 0, 4 * 1024 * 1024).unwrap());
    assert!(preflight(&document, 0, BUDGET).unwrap());
}

#[test]
fn should_decline_complete_format12_billion_code_group_before_native() {
    let input = mutated_program(|program| {
        let mut cmap = vec![0, 0, 0, 1, 0, 1, 0, 0, 0, 0, 0, 12];
        cmap.extend_from_slice(&[0, 12, 0, 0]);
        for value in [28_u32, 0, 1, 0, u32::MAX, 1] {
            cmap.extend_from_slice(&value.to_be_bytes());
        }
        assert_eq!(cmap.len(), 40);
        rebuild_font(program, &[(b"cmap", cmap)]);
    });
    assert_audit_declined(&input);
}
