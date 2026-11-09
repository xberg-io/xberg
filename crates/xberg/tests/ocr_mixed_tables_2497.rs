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
                bbox.x0 >= region.x0 - 0.01
                    && bbox.x1 <= region.x1 + 0.01
                    && bbox.y0 >= region.y0 - 0.01
                    && bbox.y1 <= region.y1 + 0.01,
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
    // The source README pins font=21, row pitch=31 and first rows at y=120/890; these are drawing bounds. ~keep
    [120., 890.].map(|first| {
        let transform = |x: f64, y: f64| {
            let (x, y, height) = match angle {
                0 => (x, y, 1650.),
                90 => (1650. - y, x, 1276.),
                270 => (y, 1276. - x, 1276.),
                _ => panic!("unsupported source pose"),
            };
            (x * 72. / 150., (height - y) * 72. / 150.)
        };
        let (a, b) = (transform(0., first - 21.), transform(1276., first + 19. * 31. + 21.));
        BoundingBox {
            x0: a.0.min(b.0),
            y0: a.1.min(b.1),
            x1: a.0.max(b.0),
            y1: a.1.max(b.1),
        }
    })
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
            actual.x0 >= source.x0 - 0.01
                && actual.x1 <= source.x1 + 0.01
                && actual.y0 >= source.y0 - 0.01
                && actual.y1 <= source.y1 + 0.01,
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
                actual.x0 >= source.x0 - 0.01
                    && actual.x1 <= source.x1 + 0.01
                    && actual.y0 >= source.y0 - 0.01
                    && actual.y1 <= source.y1 + 0.01,
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
fn two_tables_with_source_heading() -> image::GrayImage {
    let decode = |bytes: &[u8]| {
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
    };
    let mut tables = decode(include_bytes!(
        "fixtures/ocr/multitable_orientation_2497/strict-two-tables-0.pdf"
    ));
    let heading = decode(include_bytes!(
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
