use super::*;

#[test]
#[ignore = "Run mise run rust:test:explicit-sideways with verified cached models"]
fn explicit_language_block_recovers_the_sideways_source_paragraph() {
    const CASE: &str = "prose::explicit_language_block_recovers_the_sideways_source_paragraph";
    if std::env::var(CHILD).as_deref() != Ok(CASE) {
        run_private_child(CASE);
        return;
    }
    recovers_the_sideways_source_paragraph_case();
}

fn recovers_the_sideways_source_paragraph_case() {
    let pdf = prose_source();
    let native = PdfDocument::from_bytes(pdf.clone()).unwrap();
    assert_eq!(native.page_count().unwrap(), 1);
    assert!(
        native
            .extract_page_text_with_options(0, xberg_native_pdf::document::ReadingOrder::TopToBottom)
            .unwrap()
            .spans
            .is_empty()
    );
    let mut config = ExtractionConfig {
        force_ocr: true,
        use_cache: false,
        include_document_structure: true,
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ocr: Some(OcrConfig::default()),
        ..Default::default()
    };
    let source = "Docling bundles PDF document conversion to JSON and Markdown in an easy self contained package";
    config
        .ocr
        .as_mut()
        .unwrap()
        .tesseract_config
        .get_or_insert_default()
        .psm = Some(1);
    let forced = extract("prose-source-psm1", &pdf, &config);
    assert_eq!(
        forced.content.split_whitespace().collect::<Vec<_>>(),
        source.split_whitespace().collect::<Vec<_>>()
    );
    assert!(forced.tables.is_empty());
    config.ocr.as_mut().unwrap().tesseract_config.as_mut().unwrap().psm = Some(11);
    let sparse = extract("prose-source-psm11", &pdf, &config);
    assert_eq!(sparse.content.split_whitespace().count(), 3);
    config.ocr.as_mut().unwrap().tesseract_config = None;
    let actual = extract("prose-source-language-only", &pdf, &config);
    assert!(actual.tables.is_empty());
    assert!(actual.ocr_elements.is_none());
    assert!(!actual.metadata.additional.contains_key("auto_rotated"));
    // Correct behavior in every build: the language-only read recovers the source paragraph.
    assert_eq!(
        actual.content.split_whitespace().collect::<Vec<_>>(),
        source.split_whitespace().collect::<Vec<_>>()
    );
    let actual_document = actual.document.as_ref().unwrap();
    let forced_document = forced.document.as_ref().unwrap();
    assert_eq!(actual_document.nodes.len(), 1);
    assert_eq!(actual_document.nodes[0].bbox, forced_document.nodes[0].bbox);
    assert_eq!(
        actual_document.nodes[0].content.text(),
        Some("Docling bundles PDF document conversion to\nJSON and Markdown in an easy self contained\npackage")
    );

    let mut upright = Document::load_mem(&pdf).unwrap();
    let page = upright.get_pages()[&1];
    upright
        .get_object_mut(page)
        .unwrap()
        .as_dict_mut()
        .unwrap()
        .set("Rotate", 0);
    let mut bytes = Vec::new();
    upright.save_to(&mut bytes).unwrap();
    let upright = extract("prose-source-upright", &bytes, &config);
    assert_eq!(
        upright.content.split_whitespace().collect::<Vec<_>>(),
        source.split_whitespace().collect::<Vec<_>>()
    );
    assert_psm(&upright, "11");
}

fn prose_source() -> Vec<u8> {
    let path = std::env::var_os("XBERG_2496_PROSE_SOURCE")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| helpers::get_test_documents_dir().join("pdf/ocr_test_rotated_90.pdf"));
    let pdf = std::fs::read(&path).unwrap_or_else(|error| panic!("required source {}: {error}", path.display()));
    assert_eq!(
        hex::encode(Sha256::digest(&pdf)),
        "39d7fab939f52c2e88a0f51d6ae7ae5e83ece0576f6ec00860fef325ca6e6c5e"
    );
    pdf
}

#[test]
#[ignore = "Run mise run rust:test:explicit-sideways with verified cached models"]
fn explicit_language_block_abstains_on_extra_prose_regions() {
    const CASE: &str = "prose::explicit_language_block_abstains_on_extra_prose_regions";
    if std::env::var(CHILD).as_deref() != Ok(CASE) {
        run_private_child(CASE);
        return;
    }
    let pdf = prose_source();
    let config = ExtractionConfig {
        force_ocr: true,
        use_cache: false,
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ocr: Some(OcrConfig {
            element_config: Some(xberg::types::OcrElementConfig {
                include_elements: true,
                min_level: xberg::types::OcrElementLevel::Word,
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    };
    for (protected, label) in [(false, "package"), (true, "Docling")] {
        let source = prose_with_extra_region(&pdf, protected);
        let mut explicit = config.clone();
        explicit
            .ocr
            .as_mut()
            .unwrap()
            .tesseract_config
            .get_or_insert_default()
            .psm = Some(1);
        let candidate = extract(&format!("prose-extra-{label}-psm1"), &source, &explicit);
        assert_eq!(
            candidate
                .content
                .split_whitespace()
                .filter(|word| *word == label)
                .count(),
            2
        );
        assert!(candidate.tables.is_empty());
        explicit.ocr.as_mut().unwrap().tesseract_config.as_mut().unwrap().psm = Some(11);
        let sparse = extract(&format!("prose-extra-{label}-psm11"), &source, &explicit);
        if protected {
            assert!(
                sparse.ocr_elements.as_ref().unwrap().iter().any(|word| {
                    word.level == xberg::types::OcrElementLevel::Word
                        && word.text == label
                        && word.confidence.recognition >= 0.75
                }),
                "sparse primary has a high-confidence standalone occurrence"
            );
        }
        let actual = extract(&format!("prose-extra-{label}-language-only"), &source, &config);
        assert_eq!(actual.content, sparse.content);
        assert_eq!(
            serde_json::to_value(actual.ocr_elements).unwrap(),
            serde_json::to_value(sparse.ocr_elements).unwrap()
        );
        assert!(actual.tables.is_empty());
    }
}

fn prose_with_extra_region(pdf: &[u8], protected: bool) -> Vec<u8> {
    let native = PdfDocument::from_bytes(pdf.to_vec()).unwrap();
    let handles = native.page_image_handles(0).unwrap();
    let [handle] = handles.as_slice() else {
        panic!("expected one full-page image")
    };
    let mut raster = handle.decode().unwrap().to_dynamic_image().unwrap().to_luma8();
    assert_eq!(raster.dimensions(), (4960, 7016));
    let mark = if protected {
        let word = image::imageops::crop_imm(&raster, 590, 642, 660, 170).to_image();
        image::imageops::rotate90(&word)
    } else {
        image::imageops::crop_imm(&raster, 580, 1070, 730, 210).to_image()
    };
    image::imageops::overlay(&mut raster, &mark, 2300, 3040);
    let mut document = Document::load_mem(pdf).unwrap();
    let mut images = document
        .objects
        .values_mut()
        .filter_map(|object| object.as_stream_mut().ok())
        .filter(|stream| stream.dict.get(b"Subtype").and_then(Object::as_name).ok() == Some(b"Image"));
    let stream = images.next().expect("sole source scan");
    assert!(images.next().is_none());
    stream.set_plain_content(raster.into_raw());
    stream.compress().unwrap();
    let mut bytes = Vec::new();
    document.save_to(&mut bytes).unwrap();
    bytes
}

#[test]
#[ignore = "Run mise run rust:test:explicit-sideways with verified cached models"]
fn mixed_document_reads_a_sideways_prose_scan_page_alike_in_every_build() {
    const CASE: &str = "prose::mixed_document_reads_a_sideways_prose_scan_page_alike_in_every_build";
    if std::env::var(CHILD).as_deref() != Ok(CASE) {
        run_private_child(CASE);
        return;
    }
    assert_mixed_page_keeps_the_sparse_read("mixed-prose-source", &prose_source());
}

#[test]
#[ignore = "Run mise run rust:test:explicit-sideways with verified cached models"]
fn mixed_document_abstains_on_extra_prose_regions_alike_in_every_build() {
    const CASE: &str = "prose::mixed_document_abstains_on_extra_prose_regions_alike_in_every_build";
    if std::env::var(CHILD).as_deref() != Ok(CASE) {
        run_private_child(CASE);
        return;
    }
    let pdf = prose_source();
    for (protected, label) in [(false, "package"), (true, "Docling")] {
        assert_mixed_page_keeps_the_sparse_read(
            &format!("mixed-prose-extra-{label}"),
            &prose_with_extra_region(&pdf, protected),
        );
    }
}

/// The default build's mixed route runs no orientation retry, so a Paddle build must not recover the
/// scan page's paragraph either: the page reads exactly as the sparse-text read of the same scan. ~keep
fn assert_mixed_page_keeps_the_sparse_read(name: &str, scan: &[u8]) {
    let mut config = ExtractionConfig {
        use_cache: false,
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ocr: Some(OcrConfig::default()),
        ..Default::default()
    };
    let mixed = helpers::extract_bytes_document_blocking(&with_native_first_page(scan), "application/pdf", &config)
        .expect("actual public mixed PDF extraction");
    record_result(name, &mixed);
    let pages = mixed.pages.as_ref().expect("requested pages");
    assert_eq!(pages.len(), 2);
    assert_eq!(
        pages[0].content.split_whitespace().collect::<Vec<_>>(),
        NATIVE_LINES
            .iter()
            .flat_map(|line| line.split_whitespace())
            .collect::<Vec<_>>(),
        "{name}: the native page keeps its text layer"
    );
    assert_eq!(mixed.extraction_method, Some(ExtractionMethod::Mixed), "{name}");
    assert!(!mixed.metadata.additional.contains_key("auto_rotated"), "{name}");

    config.force_ocr = true;
    config
        .ocr
        .as_mut()
        .unwrap()
        .tesseract_config
        .get_or_insert_default()
        .psm = Some(11);
    let sparse = extract(&format!("{name}-sparse"), scan, &config);
    assert_eq!(
        pages[1].content.split_whitespace().collect::<Vec<_>>(),
        sparse.content.split_whitespace().collect::<Vec<_>>(),
        "{name}: the scan page must read as the sparse read, in every build"
    );
}

fn with_native_first_page(scan: &[u8]) -> Vec<u8> {
    let mut document = Document::load_mem(scan).unwrap();
    let scan_page = document.get_pages()[&1];
    let pages_root = document
        .get_object(scan_page)
        .unwrap()
        .as_dict()
        .unwrap()
        .get(b"Parent")
        .unwrap()
        .as_reference()
        .unwrap();
    let font = document.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica",
    });
    let mut text = String::from("BT /F1 12 Tf 72 720 Td 14 TL\n");
    for line in NATIVE_LINES {
        text.push_str(&format!("({line}) Tj T*\n"));
    }
    text.push_str("ET");
    let contents = document.add_object(Stream::new(dictionary! {}, text.into_bytes()));
    let native_page = document.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => pages_root,
        "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
        "Resources" => dictionary! { "Font" => dictionary! { "F1" => font } },
        "Contents" => contents,
    });
    let root = document.get_object_mut(pages_root).unwrap().as_dict_mut().unwrap();
    root.set("Kids", vec![native_page.into(), scan_page.into()]);
    root.set("Count", 2);
    let mut bytes = Vec::new();
    document.save_to(&mut bytes).unwrap();
    bytes
}

const NATIVE_LINES: [&str; 4] = [
    "This first page carries a real text layer written directly into the document.",
    "It holds enough ordinary English prose that no scan classification applies to it.",
    "The second page of the same file is a photographed scan turned onto its side.",
    "Only that scan needs optical character recognition to be read at all.",
];
