use lopdf::{Document, Object, dictionary};
use xberg::{ExtractedDocument, ExtractionConfig, OcrStrategy};

fn inherited_page(document: &Document, id: lopdf::ObjectId) -> lopdf::Dictionary {
    let mut page = document.get_dictionary(id).unwrap().clone();
    let mut parent = page.get(b"Parent").unwrap().as_reference().unwrap();
    loop {
        let ancestor = document.get_dictionary(parent).unwrap();
        for key in [b"Resources".as_slice(), b"MediaBox", b"CropBox", b"Rotate"] {
            if page.get(key).is_err()
                && let Ok(value) = ancestor.get(key)
            {
                page.set(key.to_vec(), value.clone());
            }
        }
        let Ok(next) = ancestor.get(b"Parent") else {
            break;
        };
        parent = next.as_reference().unwrap();
    }
    page
}

fn with_scanned_table(source_pages: &[u32], stamp: bool) -> Vec<u8> {
    let mut native = Document::load_mem(&super::excerpt(source_pages)).unwrap();
    let scan_bytes = if stamp {
        super::structured_output::stamped_table_pages()
    } else {
        include_bytes!("../fixtures/ocr/shaded_table_scan.pdf").to_vec()
    };
    let mut scan = Document::load_mem(&scan_bytes).unwrap();
    scan.renumber_objects_with(native.max_id + 1);
    let scan_id = *scan.get_pages().values().next().unwrap();
    let mut scan_page = inherited_page(&scan, scan_id);
    let root = native.catalog().unwrap().get(b"Pages").unwrap().as_reference().unwrap();
    let mut children: Vec<_> = native.get_pages().into_values().map(Object::Reference).collect();
    children.push(Object::Reference(scan_id));
    scan_page.set("Parent", root);
    native.max_id = scan.max_id.max(native.max_id);
    native.objects.extend(scan.objects);
    native.objects.insert(scan_id, Object::Dictionary(scan_page));
    let tree = native.get_dictionary_mut(root).unwrap();
    tree.set("Count", i64::try_from(children.len()).unwrap());
    tree.set("Kids", children);
    let mut bytes = Vec::new();
    native.save_to(&mut bytes).unwrap();
    assert_eq!(
        Document::load_mem(&bytes).unwrap().get_pages().len(),
        source_pages.len() + 1
    );
    bytes
}

async fn read(bytes: &[u8], config: &ExtractionConfig) -> ExtractedDocument {
    super::helpers::extract_bytes_document(bytes, "application/pdf", config)
        .await
        .unwrap()
}

fn assert_mixed_tables(actual: &ExtractedDocument, native: &ExtractedDocument, forced: &ExtractedDocument) {
    let pages = actual.pages.as_ref().unwrap();
    let last = pages.len() - 1;
    let source = native.pages.as_ref().unwrap();
    let recognized = forced.pages.as_ref().unwrap();
    assert_eq!(actual.extraction_method, Some(xberg::types::ExtractionMethod::Mixed));
    assert!(!source[last - 1].tables.is_empty(), "the SDM control has native tables");
    assert!(source[last].tables.is_empty(), "the raster control has no native table");
    assert_eq!(recognized[last].tables.len(), 1, "the scanned table is recognized");
    let mut expected = Vec::new();
    for (index, page) in pages.iter().enumerate() {
        let reference = if index == last {
            &recognized[index]
        } else {
            &source[index]
        };
        if index == last {
            super::plain_contract::assert_scanned_table_body(page, reference);
        } else {
            assert_eq!(page.content, reference.content);
        }
        assert_eq!(super::page_tables_json(page), super::page_tables_json(reference));
        expected.extend(reference.tables.iter().map(|table| table.as_ref()));
    }
    super::plain_contract::assert_page_ranges(actual);
    super::plain_contract::assert_page_ranges(forced);
    assert_eq!(
        serde_json::to_value(&actual.tables).unwrap(),
        serde_json::to_value(expected).unwrap()
    );
    assert_ne!(
        pages[last].content, source[last].content,
        "the raster page was replaced"
    );
}

async fn check_mixed_tables(source_pages: &[u32], stamp: bool, force_scan: bool) {
    let models = tempfile::tempdir().unwrap();
    let mut config = super::pinned_config(models.path());
    let bytes = with_scanned_table(source_pages, stamp);
    let native = read(
        &bytes,
        &ExtractionConfig {
            disable_ocr: true,
            ..config.clone()
        },
    )
    .await;
    let forced = read(
        &bytes,
        &ExtractionConfig {
            force_ocr: true,
            ..config.clone()
        },
    )
    .await;
    if stamp && !force_scan {
        let whole = read(
            &bytes,
            &ExtractionConfig {
                include_document_structure: true,
                ..config.clone()
            },
        )
        .await;
        assert_eq!(
            whole.extraction_method,
            Some(xberg::types::ExtractionMethod::Ocr),
            "an unforced request that excludes the adapter takes the whole-document gate"
        );
    }
    if force_scan {
        config.force_ocr_pages = Some(vec![u32::try_from(source_pages.len() + 1).unwrap()]);
    }
    let actual = read(&bytes, &config).await;
    assert_mixed_tables(&actual, &native, &forced);
}

#[tokio::test]
async fn automatic_all_pages_keeps_native_tables_and_replaces_the_stamped_scan() {
    check_mixed_tables(&[super::SOURCE_PAGE], true, false).await;
}

#[tokio::test]
async fn forced_page_ocr_attaches_only_the_replaced_pages_tables() {
    check_mixed_tables(&[super::SOURCE_PAGE], false, true).await;
}

#[tokio::test]
async fn detected_scanned_page_attaches_ocr_tables_and_keeps_native_tables() {
    check_mixed_tables(&[super::SOURCE_PAGE - 1, super::SOURCE_PAGE], false, false).await;
}

#[tokio::test]
async fn detected_scan_without_native_elements_renders_the_accepted_table() {
    let models = tempfile::tempdir().unwrap();
    let mut config = super::pinned_config(models.path());
    let bytes = include_bytes!("../fixtures/ocr/shaded_table_scan.pdf");
    let forced = read(
        bytes,
        &ExtractionConfig {
            force_ocr: true,
            ..config.clone()
        },
    )
    .await;
    config.ocr_strategy = OcrStrategy::ScannedPages { min_confidence: 0.9 };
    let actual = read(bytes, &config).await;
    assert_eq!(actual.extraction_method, Some(xberg::types::ExtractionMethod::Mixed));
    assert_eq!(actual.tables.len(), 1);
    let bbox = actual.tables[0]
        .bounding_box
        .expect("the OCR table keeps its rectangle");
    // ~keep The 612x792 pt page renders at 1667x2156 px and recognition reads a 2550x3298 px
    // resample, so the table's pixel rectangle (273, 267)-(2237, 1450) lies at these points. A
    // rectangle scaled by the render instead lands at x1 = 821, off the page. The tolerance absorbs
    // a few pixels of platform-dependent table detection.
    let expected = [65.52, 443.79, 536.88, 727.88];
    let actual_points = [bbox.x0, bbox.y0, bbox.x1, bbox.y1];
    assert!(
        actual_points
            .iter()
            .zip(expected)
            .all(|(actual, expected)| (actual - expected).abs() <= 2.0),
        "the table rectangle is its recognition-frame rectangle in page points: {bbox:?}"
    );
    super::plain_contract::assert_page_ranges(&actual);
    super::plain_contract::assert_scanned_table_body(
        &actual.pages.as_ref().unwrap()[0],
        &forced.pages.as_ref().unwrap()[0],
    );
}

#[tokio::test]
async fn selected_scan_renders_prose_before_and_after_the_ocr_table() {
    let mut source = Document::load_mem(include_bytes!("../fixtures/ocr/shaded_table_scan.pdf")).unwrap();
    let page_id = *source.get_pages().values().next().unwrap();
    let mut page = source.get_dictionary(page_id).unwrap().clone();
    let font = source.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica",
    });
    let mut resources = super::structured_output::resources(&source, &page);
    resources.set("Font", dictionary! {"ReviewFont" => font});
    page.set("Resources", resources);
    let bounds = vec![0.into(), (-55).into(), 612.into(), 847.into()];
    page.set("MediaBox", bounds.clone());
    page.set("CropBox", bounds);
    source.objects.insert(page_id, Object::Dictionary(page));
    source.add_page_contents(
        page_id,
        b"BT /ReviewFont 18 Tf 48 816 Td (Quarterly report introduction) Tj ET\nBT /ReviewFont 18 Tf 48 -28 Td (End of quarterly report) Tj ET".to_vec(),
    ).unwrap();
    let mut bytes = Vec::new();
    source.save_to(&mut bytes).unwrap();

    let models = tempfile::tempdir().unwrap();
    let mut config = super::pinned_config(models.path());
    config.force_ocr_pages = Some(vec![1]);
    let actual = read(&bytes, &config).await;
    assert_eq!(actual.extraction_method, Some(xberg::types::ExtractionMethod::Mixed));
    super::plain_contract::assert_page_ranges(&actual);
    assert_eq!(actual.tables.len(), 1);
    let body = &actual.pages.as_ref().unwrap()[0].content;
    let before = body
        .find("Quarterly report introduction")
        .expect("recognized introductory prose");
    let table = body.find("48,210").expect("recognized table cell");
    let after = body.find("End of quarterly report").expect("recognized closing prose");
    assert!(before < table && table < after, "{body}");
}

/// Two tables drawn below the scanned table, each with a one-word section label on the line above
/// its first row. Recognition joins each label to its table's rows and table assembly drops it as a
/// leading caption row, while the table's rectangle still covers it, so the Markdown transcript
/// loses the label and only the page elements keep it (xberg-io/xberg#3046). ~keep
const LABELLED_TABLES: [(u16, u8, [&str; 5]); 13] = [
    (360, 14, ["Fund sources by year", "", "", "", ""]),
    (330, 11, ["", "2024", "Share", "2025", "Share"]),
    (290, 11, ["Revenues", "", "", "", ""]),
    (274, 11, ["Taxes", "1,200", "56.6%", "1,350", "58.1%"]),
    (260, 11, ["Grants", "400", "18.9%", "420", "18.1%"]),
    (246, 11, ["Fees", "320", "15.1%", "330", "14.2%"]),
    (232, 11, ["Rents", "200", "9.4%", "225", "9.7%"]),
    (184, 11, ["Spending", "", "", "", ""]),
    (168, 11, ["Staff", "900", "50.5%", "950", "51.0%"]),
    (154, 11, ["Supplies", "300", "16.8%", "310", "16.6%"]),
    (140, 11, ["Services", "380", "21.3%", "390", "20.9%"]),
    (126, 11, ["Capital", "205", "11.5%", "215", "11.5%"]),
    (80, 11, ["Net change for the year", "", "", "", ""]),
];

fn scan_with_labelled_tables() -> Vec<u8> {
    let mut source = Document::load_mem(include_bytes!("../fixtures/ocr/shaded_table_scan.pdf")).unwrap();
    let page_id = *source.get_pages().values().next().unwrap();
    let mut page = source.get_dictionary(page_id).unwrap().clone();
    let font = source.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica",
    });
    let mut resources = super::structured_output::resources(&source, &page);
    resources.set("Font", dictionary! {"ReviewFont" => font});
    page.set("Resources", resources);
    source.objects.insert(page_id, Object::Dictionary(page));
    let mut content = String::new();
    for (y, size, cells) in LABELLED_TABLES {
        for (x, text) in [72, 220, 300, 380, 460].into_iter().zip(cells) {
            if !text.is_empty() {
                content.push_str(&format!("BT /ReviewFont {size} Tf {x} {y} Td ({text}) Tj ET\n"));
            }
        }
    }
    source.add_page_contents(page_id, content.into_bytes()).unwrap();
    let mut bytes = Vec::new();
    source.save_to(&mut bytes).unwrap();
    bytes
}

#[tokio::test]
async fn selected_scan_keeps_section_labels_that_table_assembly_drops() {
    let models = tempfile::tempdir().unwrap();
    let mut config = super::pinned_config(models.path());
    config.force_ocr_pages = Some(vec![1]);
    let actual = read(&scan_with_labelled_tables(), &config).await;
    assert_eq!(actual.extraction_method, Some(xberg::types::ExtractionMethod::Mixed));
    let body = &actual.pages.as_ref().unwrap()[0].content;
    let at = |word: &str| {
        assert_eq!(body.matches(word).count(), 1, "{word} once in the page: {body}");
        body.find(word).unwrap()
    };
    let order = [
        "Revenues",
        "Taxes",
        "58.1%",
        "Spending",
        "Staff",
        "51.0%",
        "Net change for the year",
    ];
    let positions: Vec<usize> = order.into_iter().map(at).collect();
    assert!(positions.is_sorted(), "{order:?} in reading order: {body}");
    super::plain_contract::assert_page_ranges(&actual);
}

struct TablePage {
    name: &'static str,
    pdf: &'static [u8],
    rectangles: &'static [[f64; 4]],
}

/// A raster-only page and the table rectangles, in PDF points, that its recognition must report
/// however the pipeline reaches it. Each page is read at a resampled frame, so a rectangle left in
/// that frame, or scaled into the render twice, lands outside the page. A rotated page is recovered
/// only by the detached route; whole-document OCR needs the orientation model for it. ~keep
const TABLE_PAGES: [TablePage; 7] = [
    TablePage {
        name: "shaded table",
        pdf: include_bytes!("../fixtures/ocr/shaded_table_scan.pdf"),
        rectangles: &[[65.52, 443.79, 536.88, 727.88]],
    },
    TablePage {
        name: "heading above a table",
        pdf: include_bytes!("../fixtures/ocr/detached_heading_frame_2497/heading-table-0.pdf"),
        rectangles: &[[28.8, 53.76, 576.0, 689.28]],
    },
    TablePage {
        name: "heading above a table, rotated",
        pdf: include_bytes!("../fixtures/ocr/detached_heading_frame_2497/heading-table-90.pdf"),
        rectangles: &[[53.76, 36.48, 689.28, 583.68]],
    },
    TablePage {
        name: "two tables",
        pdf: include_bytes!("../fixtures/ocr/multitable_orientation_2497/two-tables-0.pdf"),
        rectangles: &[[38.4, 459.84, 475.2, 742.08], [38.4, 75.84, 475.2, 358.08]],
    },
    TablePage {
        name: "two tables, rotated",
        pdf: include_bytes!("../fixtures/ocr/multitable_orientation_2497/two-tables-90.pdf"),
        rectangles: &[[459.84, 137.28, 742.08, 574.08], [75.84, 137.28, 358.08, 574.08]],
    },
    TablePage {
        name: "strict two tables",
        pdf: include_bytes!("../fixtures/ocr/multitable_orientation_2497/strict-two-tables-0.pdf"),
        rectangles: &[[28.8, 449.52, 270.48, 741.84], [28.8, 80.64, 270.48, 372.0]],
    },
    TablePage {
        name: "strict two tables, rotated",
        pdf: include_bytes!("../fixtures/ocr/multitable_orientation_2497/strict-two-tables-90.pdf"),
        rectangles: &[[449.52, 342.0, 741.84, 583.68], [80.64, 342.0, 372.0, 583.68]],
    },
];

#[track_caller]
fn assert_table_rectangles(document: &ExtractedDocument, expected: &[[f64; 4]], page: &str) {
    let actual: Vec<[f64; 4]> = document
        .tables
        .iter()
        .map(|table| {
            let bbox = table.bounding_box.expect("the OCR table keeps its rectangle");
            [bbox.x0, bbox.y0, bbox.x1, bbox.y1]
        })
        .collect();
    assert_eq!(actual.len(), expected.len(), "{page}: {actual:?}");
    assert!(
        actual
            .iter()
            .zip(expected)
            .all(|(actual, expected)| { actual.iter().zip(expected).all(|(a, e)| (a - e).abs() <= 2.0) }),
        "{page}: expected rectangles {expected:?}, got {actual:?}"
    );
}

async fn table_rectangles_under(config: &ExtractionConfig, method: xberg::types::ExtractionMethod, rotated: bool) {
    for page in TABLE_PAGES
        .iter()
        .filter(|page| rotated || !page.name.ends_with("rotated"))
    {
        let actual = read(page.pdf, config).await;
        assert_eq!(actual.extraction_method, Some(method), "{}", page.name);
        assert_table_rectangles(&actual, page.rectangles, page.name);
    }
}

#[tokio::test]
async fn detected_scans_report_every_table_rectangle_in_page_points() {
    let models = tempfile::tempdir().unwrap();
    let mut config = super::pinned_config(models.path());
    config.ocr_strategy = OcrStrategy::ScannedPages { min_confidence: 0.9 };
    table_rectangles_under(&config, xberg::types::ExtractionMethod::Mixed, true).await;
}

#[tokio::test]
async fn forced_ocr_reports_every_table_rectangle_in_page_points() {
    let models = tempfile::tempdir().unwrap();
    let config = ExtractionConfig {
        force_ocr: true,
        ..super::pinned_config(models.path())
    };
    table_rectangles_under(&config, xberg::types::ExtractionMethod::Ocr, false).await;
}

fn single_stage_pipeline(config: ExtractionConfig) -> ExtractionConfig {
    let mut config = config;
    config.force_ocr = true;
    config.ocr.as_mut().unwrap().pipeline = Some(xberg::OcrPipelineConfig {
        stages: vec![serde_json::from_value(serde_json::json!({"backend": "tesseract", "priority": 100})).unwrap()],
        quality_thresholds: Default::default(),
    });
    config
}

/// An explicit pipeline keeps the whole-document scope, so each page is rendered lazily rather
/// than handed over as a detached image.
#[tokio::test]
async fn single_stage_pipeline_reports_every_table_rectangle_in_page_points() {
    let models = tempfile::tempdir().unwrap();
    let config = single_stage_pipeline(super::pinned_config(models.path()));
    table_rectangles_under(&config, xberg::types::ExtractionMethod::Ocr, false).await;
}

#[cfg(feature = "layout-detection")]
async fn read_with_layout(bytes: &[u8], mime: &str) -> ExtractedDocument {
    let models = tempfile::tempdir().unwrap();
    let mut config = single_stage_pipeline(super::pinned_config(models.path()));
    config.layout = Some(xberg::LayoutDetectionConfig {
        acceleration: Some(xberg::AccelerationConfig {
            provider: xberg::ExecutionProviderType::Cpu,
            ..Default::default()
        }),
        ..Default::default()
    });
    let actual = super::helpers::extract_bytes_document(bytes, mime, &config)
        .await
        .expect("layout route extraction");
    assert!(
        actual.processing_warnings.iter().all(|warning| !matches!(
            warning.source.as_ref(),
            "layout" | "layout-ocr" | "pdf_tables" | "image_tables"
        )),
        "layout or table inference fell back: {:?}",
        actual.processing_warnings
    );
    actual
}

#[cfg(feature = "layout-detection")]
#[tokio::test]
#[ignore = "requires RT-DETR and TATR model files"]
async fn layout_route_reports_the_backend_and_layout_tables_in_page_points() {
    let actual = read_with_layout(TABLE_PAGES[0].pdf, "application/pdf").await;
    let backend = TABLE_PAGES[0].rectangles[0];
    let detected = [61.88, 442.17, 547.54, 693.63];
    assert_table_rectangles(&actual, &[backend, detected], "layout OCR of the shaded table");
}

#[cfg(feature = "layout-detection")]
#[tokio::test]
#[ignore = "requires RT-DETR and TATR model files"]
async fn layout_route_reports_the_detected_table_in_page_points_after_undoing_pdf_rotation() {
    let mut pdf = Document::load_mem(TABLE_PAGES[0].pdf).unwrap();
    let page = *pdf.get_pages().values().next().unwrap();
    pdf.get_dictionary_mut(page).unwrap().set("Rotate", 90);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();

    let actual = read_with_layout(&bytes, "application/pdf").await;
    let detected = [61.88, 442.17, 547.54, 693.63];
    assert_table_rectangles(&actual, &[detected], "layout OCR after undoing /Rotate");
}

#[cfg(feature = "layout-detection")]
#[tokio::test]
#[ignore = "requires RT-DETR and TATR model files"]
async fn layout_route_keeps_a_standalone_image_table_in_pixels() {
    let image = include_bytes!("../fixtures/ocr/gh1785_shaded_rows.png");
    let actual = read_with_layout(image, "image/png").await;
    let regions = actual.pages.as_ref().unwrap()[0].layout_regions.as_ref().unwrap();
    let table_region = regions
        .iter()
        .find(|region| region.class_name == "table")
        .expect("standalone image layout must detect a table")
        .bounding_box;
    assert_table_rectangles(&actual, &[[280.0, 426.0, 2239.0, 1451.0]], "standalone image pixels");
    let table = &actual.tables[0];
    let bbox = table.bounding_box.unwrap();
    assert!(
        bbox.x0 >= table_region.x0
            && bbox.y0 >= table_region.y0
            && bbox.x1 <= table_region.x1
            && bbox.y1 <= table_region.y1,
        "table {bbox:?} must lie inside its detected pixel region {table_region:?}"
    );
    assert_eq!(table.cells[0][1], "FY2024-25");
}
