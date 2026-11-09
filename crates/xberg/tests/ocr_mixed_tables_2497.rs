#![cfg(all(
    feature = "pdf",
    feature = "ocr",
    any(feature = "auto-rotate", feature = "auto-rotate-tract")
))]

use std::{env, fs, process::Command};

use sha2::{Digest, Sha256};
use xberg::{BoundingBox, ExtractInput, ExtractionConfig};

const CHILD: &str = "XBERG_MIXED_TABLES_2497_CHILD";
const MODEL_CACHE_PATH: &str = concat!(
    "models--xberg-io--paddleocr-onnx-models/snapshots/bc5ec866cf0e798e667808dfa51b0ba8ad0dafc8/",
    "v2/classifiers/PP-LCNet_x1_0_doc_ori.onnx"
);
const MODEL_HASH: &str = "6b742aebce6f0f7f71f747931ac7becfc7c96c51641e14943b291eeb334e7947";

fn in_pinned_child(case: &str) -> bool {
    if env::var(CHILD).as_deref() == Ok(case) {
        return true;
    }
    let Some(model) = env::var_os("XBERG_ORIENTATION_TEST_MODEL") else {
        assert_ne!(
            env::var("XBERG_REQUIRE_MODELS").as_deref(),
            Ok("1"),
            "provide the pinned orientation model"
        );
        return false;
    };
    let temp = tempfile::tempdir().unwrap();
    let hub = temp.path().join("hub");
    let model_path = hub.join(MODEL_CACHE_PATH);
    fs::create_dir_all(model_path.parent().unwrap()).unwrap();
    let bytes = fs::read(model).unwrap();
    assert_eq!(hex::encode(Sha256::digest(&bytes)), MODEL_HASH);
    fs::write(model_path, bytes).unwrap();
    let tessdata = temp.path().join("tessdata");
    fs::create_dir(&tessdata).unwrap();
    let english = xberg_tesseract::bundled_eng_traineddata().expect("enable bundle-tessdata-eng");
    assert_eq!(
        hex::encode(Sha256::digest(english)),
        "7d4322bd2a7749724879683fc3912cb542f19906c83bcc1a52132556427170b2"
    );
    fs::write(tessdata.join("eng.traineddata"), english).unwrap();
    let output = Command::new(env::current_exe().unwrap())
        .args(["--exact", case, "--nocapture"])
        .env(CHILD, case)
        .env("HF_HOME", temp.path())
        .env("HF_HUB_CACHE", hub)
        .env("HF_HUB_OFFLINE", "1")
        .env("TESSDATA_PREFIX", tessdata)
        .env("XBERG_CACHE_DIR", temp.path().join("xberg"))
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success() && stdout.contains("test result: ok. 1 passed; 0 failed; 0 ignored;"),
        "{stdout}\n{}",
        String::from_utf8_lossy(&output.stderr),
    );
    false
}

#[cfg(feature = "pdf-native")]
#[tokio::test]
async fn automatic_two_tables_and_heading_preserve_every_source_word() {
    if !in_pinned_child("automatic_two_tables_and_heading_preserve_every_source_word") {
        return;
    }
    let expected: Vec<Vec<Vec<String>>> = serde_json::from_str(include_str!(
        "fixtures/ocr/multitable_orientation_2497/strict-cells.json"
    ))
    .unwrap();
    let heading = "Cedar Company Ledger Summary For The March Quarter Report 2026";
    let words = heading
        .split_whitespace()
        .chain(
            expected
                .iter()
                .flatten()
                .flatten()
                .flat_map(|cell| cell.split_whitespace()),
        )
        .collect::<Vec<_>>();
    assert_eq!(words.len(), 210);
    let upright = two_tables_with_source_heading();
    if let Some(root) = env::var_os("XBERG_2497_RECEIPTS") {
        fs::create_dir_all(&root).unwrap();
        upright
            .save(std::path::PathBuf::from(root).join("source-two-tables-heading.png"))
            .unwrap();
    }
    let mut readings = Vec::new();
    for (angle, raster) in [
        (0, upright.clone()),
        (90, image::imageops::rotate90(&upright)),
        (270, image::imageops::rotate270(&upright)),
    ] {
        let pdf = grayscale_scan_pdf(&raster);
        let config = ExtractionConfig {
            use_cache: false,
            pages: Some(xberg::core::config::PageConfig {
                extract_pages: true,
                ..Default::default()
            }),
            ..Default::default()
        };
        assert!(!config.force_ocr && config.ocr.is_none());
        if angle != 0 {
            let mut enabled = config.clone();
            enabled.ocr = Some(serde_json::from_value(serde_json::json!({"auto_rotate": true})).unwrap());
            let output = xberg::extract(ExtractInput::from_bytes(pdf.clone(), "application/pdf", None), &enabled)
                .await
                .unwrap();
            assert!(output.errors.is_empty());
            let [document] = <[_; 1]>::try_from(output.results).unwrap();
            if let Some(root) = env::var_os("XBERG_2497_RECEIPTS") {
                fs::create_dir_all(&root).unwrap();
                fs::write(
                    std::path::PathBuf::from(root).join(format!("two-tables-heading-enabled-{angle}.json")),
                    serde_json::to_vec_pretty(&document).unwrap(),
                )
                .unwrap();
            }
            readings.push((format!("enabled-{angle}"), angle, document));
        }
        let output = xberg::extract(ExtractInput::from_bytes(pdf, "application/pdf", None), &config)
            .await
            .unwrap();
        assert!(output.errors.is_empty());
        let [document] = <[_; 1]>::try_from(output.results).unwrap();
        if let Some(root) = env::var_os("XBERG_2497_RECEIPTS") {
            fs::create_dir_all(&root).unwrap();
            fs::write(
                std::path::PathBuf::from(root).join(format!("two-tables-heading-{angle}.json")),
                serde_json::to_vec_pretty(&document).unwrap(),
            )
            .unwrap();
        }
        readings.push((angle.to_string(), angle, document));
    }
    assert_eq!(readings.len(), 5);
    for (route, angle, document) in readings {
        assert!(document.metadata.ocr_used);
        assert_eq!(
            document.content.split_whitespace().collect::<Vec<_>>(),
            words,
            "source order at {route}"
        );
        assert_eq!(document.tables.len(), 2, "two source tables at {route}");
        for ((table, cells), region) in document.tables.iter().zip(&expected).zip(source_table_regions(angle)) {
            assert_eq!(&table.cells, cells, "exact source cells at {route}");
            assert_eq!(table.page_number, 1);
            let bbox = table.bounding_box.expect("source table bounds must be present");
            assert!(bbox.x0 < bbox.x1 && bbox.y0 < bbox.y1);
            assert!(
                owns_source_table_bounds(bbox, region),
                "table bounds {bbox:?} escape independently transformed drawing region {region:?} at {route}"
            );
        }
        let [page] = document.pages.as_deref().unwrap() else {
            panic!("one source page");
        };
        assert_eq!(page.tables.len(), 2);
        for (actual, table) in page.tables.iter().zip(&document.tables) {
            assert_eq!(actual.cells, table.cells);
            assert_eq!(actual.bounding_box, table.bounding_box);
        }
        assert_eq!(
            page.content.trim(),
            format!(
                "{heading}\n\n{}\n\n{}",
                document.tables[0].markdown.trim(),
                document.tables[1].markdown.trim()
            )
        );
        assert!(document.ocr_elements.is_none());
    }
}

#[cfg(feature = "pdf-native")]
fn source_table_regions(angle: u16) -> [BoundingBox; 2] {
    let image = two_tables_with_source_heading();
    [(99, 731), (869, 1501)].map(|band| source_points(source_ink_bounds(&image, band), angle))
}

#[cfg(feature = "pdf-native")]
fn source_ink_bounds(image: &image::GrayImage, (top, bottom): (u32, u32)) -> [u32; 4] {
    let ink = image
        .enumerate_pixels()
        .filter(|(_, y, pixel)| *y >= top && *y < bottom && pixel[0] < 64);
    let (mut left, mut first, mut right, mut last) = (image.width(), bottom, 0, 0);
    let mut count = 0;
    for (x, y, _) in ink {
        left = left.min(x);
        first = first.min(y);
        right = right.max(x + 1);
        last = last.max(y + 1);
        count += 1;
    }
    assert!(
        count > 1000,
        "nonempty independent ink in the authored source table band"
    );
    [left, first, right, last]
}

#[cfg(feature = "pdf-native")]
fn source_points([left, top, right, bottom]: [u32; 4], angle: u16) -> BoundingBox {
    let transform = |x: u32, y: u32| {
        let (x, y, height) = match angle {
            0 => (x, y, 1650),
            90 => (1650 - y, x, 1276),
            270 => (y, 1276 - x, 1276),
            _ => panic!("unsupported source pose"),
        };
        (f64::from(x) * 72. / 150., f64::from(height - y) * 72. / 150.)
    };
    let (a, b) = (transform(left, top), transform(right, bottom));
    BoundingBox {
        x0: a.0.min(b.0),
        y0: a.1.min(b.1),
        x1: a.0.max(b.0),
        y1: a.1.max(b.1),
    }
}

#[cfg(feature = "pdf-native")]
#[tokio::test]
async fn automatic_upright_tables_stay_in_source_drawing_regions() {
    if !in_pinned_child("automatic_upright_tables_stay_in_source_drawing_regions") {
        return;
    }
    let pdf = grayscale_scan_pdf(&two_tables_with_source_heading());
    let config = ExtractionConfig {
        use_cache: false,
        pages: Some(xberg::core::config::PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    assert!(!config.force_ocr && config.ocr.is_none());
    let output = xberg::extract(ExtractInput::from_bytes(pdf, "application/pdf", None), &config)
        .await
        .unwrap();
    assert!(output.errors.is_empty());
    let [document] = <[_; 1]>::try_from(output.results).unwrap();
    assert!(document.metadata.ocr_used);
    let expected: Vec<Vec<Vec<String>>> = serde_json::from_str(include_str!(
        "fixtures/ocr/multitable_orientation_2497/strict-cells.json"
    ))
    .unwrap();
    assert_eq!(document.tables.len(), 2);
    for ((table, cells), source) in document.tables.iter().zip(expected).zip(source_table_regions(0)) {
        assert_eq!(table.cells, cells);
        let actual = table.bounding_box.expect("source-owned table point bounds");
        assert!(actual.x0 < actual.x1 && actual.y0 < actual.y1);
        assert!(
            owns_source_table_bounds(actual, source),
            "{actual:?} is not in source drawing region {source:?}"
        );
    }
}

#[cfg(all(feature = "pdf-native", feature = "paddle-ocr-ort"))]
#[tokio::test]
async fn detached_mixed_tables_keep_source_words_structure_and_frames() {
    if !in_pinned_child("detached_mixed_tables_keep_source_words_structure_and_frames") {
        return;
    }
    let upright = two_tables_with_source_heading();
    let cells: Vec<Vec<Vec<String>>> = serde_json::from_str(include_str!(
        "fixtures/ocr/multitable_orientation_2497/strict-cells.json"
    ))
    .unwrap();
    let expected: Vec<_> = "Cedar Company Ledger Summary For The March Quarter Report 2026"
        .split_whitespace()
        .chain(
            cells
                .iter()
                .flatten()
                .flatten()
                .flat_map(|cell| cell.split_whitespace()),
        )
        .collect();
    assert_eq!(expected.len(), 210);
    for (angle, raster) in [
        (0, upright.clone()),
        (90, image::imageops::rotate90(&upright)),
        (270, image::imageops::rotate270(&upright)),
    ] {
        let config = ExtractionConfig {
            force_ocr: true,
            use_cache: false,
            include_document_structure: true,
            pages: Some(xberg::core::config::PageConfig {
                extract_pages: true,
                ..Default::default()
            }),
            ..Default::default()
        };
        let output = xberg::extract(
            ExtractInput::from_bytes(grayscale_scan_pdf(&raster), "application/pdf", None),
            &config,
        )
        .await
        .unwrap();
        assert!(output.errors.is_empty(), "angle {angle}: {:?}", output.errors);
        let [document] = <[_; 1]>::try_from(output.results).unwrap();
        assert_eq!(
            document.content.split_whitespace().collect::<Vec<_>>(),
            expected,
            "angle {angle}"
        );
        assert_eq!(document.tables.len(), 2);
        for ((table, expected), source) in document.tables.iter().zip(&cells).zip(source_table_regions(angle)) {
            assert_eq!(&table.cells, expected);
            let actual = table.bounding_box.expect("source-owned PDF point bounds");
            assert!(
                owns_source_table_bounds(actual, source),
                "angle {angle}: {actual:?} outside {source:?}"
            );
        }
        let structure = serde_json::to_value(document.document.expect("requested structure")).unwrap();
        let nodes = structure["nodes"].as_array().unwrap();
        assert_eq!(nodes.len(), 3, "one heading and two tables at angle {angle}");
        assert_eq!(
            nodes
                .iter()
                .filter(|node| node["content"]["node_type"] == "table")
                .count(),
            2
        );
    }
}

#[cfg(feature = "pdf-native")]
fn source_raster(bytes: &[u8]) -> image::GrayImage {
    let pdf = xberg_native_pdf::PdfDocument::from_bytes(bytes.to_vec()).unwrap();
    assert_eq!(pdf.page_count().unwrap(), 1);
    assert!(
        pdf.extract_page_text_with_options(0, xberg_native_pdf::document::ReadingOrder::TopToBottom)
            .unwrap()
            .spans
            .is_empty()
    );
    let images = pdf.page_image_handles(0).unwrap();
    assert_eq!(images.len(), 1);
    let image = images[0].decode().unwrap().to_dynamic_image().unwrap().to_luma8();
    assert_eq!(image.dimensions(), (1276, 1650));
    image
}

#[cfg(feature = "pdf-native")]
fn two_tables_with_source_heading() -> image::GrayImage {
    let mut tables = source_raster(include_bytes!(
        "fixtures/ocr/multitable_orientation_2497/strict-two-tables-0.pdf"
    ));
    let heading = source_raster(include_bytes!(
        "fixtures/ocr/detached_heading_frame_2497/heading-table-0.pdf"
    ));
    // The source heading occupies y=90..110; move its isolated band above the first table at y=120. ~keep
    let band = image::imageops::crop_imm(&heading, 0, 70, 1276, 45).to_image();
    image::imageops::replace(&mut tables, &band, 0, 0);
    tables
}

#[cfg(feature = "pdf-native")]
fn grayscale_scan_pdf(image: &image::GrayImage) -> Vec<u8> {
    use lopdf::{Document, Object, Stream, dictionary};
    let mut pdf = Document::with_version("1.4");
    let pages = pdf.new_object_id();
    let (width, height) = (
        image.width() as f32 * 72.0 / 150.0,
        image.height() as f32 * 72.0 / 150.0,
    );
    let scan = pdf.add_object(Stream::new(
        dictionary! {
            "Type"=>"XObject", "Subtype"=>"Image", "Width"=>image.width(), "Height"=>image.height(),
            "ColorSpace"=>"DeviceGray", "BitsPerComponent"=>8,
        },
        image.as_raw().clone(),
    ));
    let content = pdf.add_object(Stream::new(
        dictionary! {},
        format!("q {width} 0 0 {height} 0 0 cm /Scan Do Q").into_bytes(),
    ));
    let page = pdf.add_object(dictionary! {
        "Type"=>"Page", "Parent"=>pages, "Contents"=>content,
        "MediaBox"=>vec![0.into(),0.into(),width.into(),height.into()],
        "Resources"=>dictionary! { "XObject"=>dictionary! { "Scan"=>scan } },
    });
    pdf.objects.insert(
        pages,
        Object::Dictionary(dictionary! { "Type"=>"Pages", "Kids"=>vec![page.into()], "Count"=>1 }),
    );
    let root = pdf.add_object(dictionary! { "Type"=>"Catalog", "Pages"=>pages });
    pdf.trailer.set("Root", root);
    pdf.compress();
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    bytes
}

#[cfg(feature = "pdf-native")]
fn owns_source_table_bounds(actual: BoundingBox, source: BoundingBox) -> bool {
    let area = |bbox: BoundingBox| (bbox.x1 - bbox.x0).max(0.) * (bbox.y1 - bbox.y0).max(0.);
    if ![actual.x0, actual.x1, actual.y0, actual.y1]
        .into_iter()
        .all(f64::is_finite)
    {
        return false;
    }
    let intersection = BoundingBox {
        x0: actual.x0.max(source.x0),
        y0: actual.y0.max(source.y0),
        x1: actual.x1.min(source.x1),
        y1: actual.y1.min(source.y1),
    };
    let overlap = area(intersection);
    overlap / (area(actual) + area(source) - overlap) >= 0.95
}

#[cfg(feature = "pdf-native")]
#[test]
fn independent_source_bounds_refuse_degenerate_subboxes() {
    let [source, _] = source_table_regions(0);
    assert!(owns_source_table_bounds(source, source));
    let tiny = BoundingBox {
        x0: source.x0 + 1.,
        y0: source.y0 + 1.,
        x1: source.x0 + 2.,
        y1: source.y0 + 2.,
    };
    assert!(
        !owns_source_table_bounds(tiny, source),
        "a tiny contained box owns no table"
    );
}

#[cfg(feature = "pdf-native")]
#[tokio::test]
async fn mixed_tables_keep_primary_words_outside_the_corrected_proof() {
    if !in_pinned_child("mixed_tables_keep_primary_words_outside_the_corrected_proof") {
        return;
    }
    let mut source = two_tables_with_source_heading();
    let heading = image::imageops::crop_imm(&source, 0, 0, 1276, 45).to_image();
    let sidebar = image::imageops::rotate90(&heading);
    image::imageops::replace(&mut source, &sidebar, 800, 200);
    let pdf = grayscale_scan_pdf(&image::imageops::rotate270(&source));
    let config = ExtractionConfig {
        use_cache: false,
        ..Default::default()
    };
    let mut enabled = config.clone();
    enabled.ocr = Some(serde_json::from_value(serde_json::json!({"auto_rotate": true})).unwrap());
    let candidate = read_pdf(&pdf, &enabled).await;
    let cells: Vec<Vec<Vec<String>>> = serde_json::from_str(include_str!(
        "fixtures/ocr/multitable_orientation_2497/strict-cells.json"
    ))
    .unwrap();
    assert_eq!(
        candidate.tables.len(),
        2,
        "the refused candidate really recovers both tables"
    );
    for (table, expected) in candidate.tables.iter().zip(cells) {
        assert_eq!(table.cells, expected);
    }
    let mut disabled = config.clone();
    disabled.ocr = Some(serde_json::from_value(serde_json::json!({"auto_rotate": false})).unwrap());
    let primary = read_pdf(&pdf, &disabled).await;
    assert!(
        primary.content.contains("Cedar Company Ledger Summary"),
        "independent upright sidebar must survive primary OCR: {}",
        primary.content
    );
    assert!(primary.tables.is_empty());
    assert_ne!(
        candidate.content, primary.content,
        "candidate and primary are independently different reads"
    );
    let implicit = read_pdf(&pdf, &config).await;
    assert_eq!(
        implicit.content, primary.content,
        "unproved primary sidebar must prevent replacement"
    );
    assert!(implicit.tables.is_empty());
    assert!(
        implicit
            .processing_warnings
            .iter()
            .any(|warning| warning.message.contains("recognition noise"))
    );
}

#[cfg(feature = "pdf-native")]
async fn read_pdf(pdf: &[u8], config: &ExtractionConfig) -> xberg::ExtractedDocument {
    let output = xberg::extract(ExtractInput::from_bytes(pdf.to_vec(), "application/pdf", None), config)
        .await
        .unwrap();
    assert!(output.errors.is_empty(), "{:?}", output.errors);
    let [document] = <[_; 1]>::try_from(output.results).unwrap();
    document
}

#[cfg(feature = "pdf-native")]
#[tokio::test]
async fn single_table_heading_recovers_both_sideways_source_poses() {
    if !in_pinned_child("single_table_heading_recovers_both_sideways_source_poses") {
        return;
    }
    let source = source_raster(include_bytes!(
        "fixtures/ocr/detached_heading_frame_2497/heading-table-0.pdf"
    ));
    let cells: Vec<Vec<String>> =
        serde_json::from_str(include_str!("fixtures/ocr/detached_heading_frame_2497/cells.json")).unwrap();
    let expected = "Cedar Company Ledger Summary For The March Quarter Report 2026"
        .split_whitespace()
        .chain(cells.iter().flatten().flat_map(|cell| cell.split_whitespace()))
        .collect::<Vec<_>>();
    assert_eq!(expected.len(), 470);
    for (angle, raster) in [
        (90, image::imageops::rotate90(&source)),
        (270, image::imageops::rotate270(&source)),
    ] {
        let pdf = grayscale_scan_pdf(&raster);
        let config = ExtractionConfig {
            use_cache: false,
            ..Default::default()
        };
        let document = read_pdf(&pdf, &config).await;
        assert_eq!(
            document.content.split_whitespace().collect::<Vec<_>>(),
            expected,
            "angle {angle}"
        );
        let [table] = document.tables.as_slice() else {
            panic!("one source table");
        };
        assert_eq!(table.cells, cells);
        let actual = table.bounding_box.unwrap();
        let ink = source_ink_bounds(&source, (190, 1540));
        assert!(
            owns_source_table_bounds(actual, source_points(ink, angle)),
            "angle {angle}: {actual:?}"
        );
    }
}
