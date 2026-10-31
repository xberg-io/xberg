use std::io::Cursor;

use image::{ImageFormat, Rgb, RgbImage};
use tempfile::TempDir;

use super::super::input::{PreparedInput, read_document};
use super::*;

fn raster(format: ImageFormat) -> Vec<u8> {
    let image = RgbImage::from_pixel(7, 11, Rgb([24, 128, 200]));
    let mut bytes = Cursor::new(Vec::new());
    image.write_to(&mut bytes, format).unwrap();
    bytes.into_inner()
}

fn document(temp: &TempDir, name: &str, file_type: &str, bytes: &[u8]) -> CorpusDocument {
    let filename = format!("{name}.{file_type}");
    let document_path = temp.path().join(&filename);
    let fixture_path = temp.path().join(format!("{name}.json"));
    std::fs::write(&document_path, bytes).unwrap();
    let fixture = serde_json::json!({
        "document": filename,
        "file_type": file_type,
        "file_size": bytes.len(),
    });
    std::fs::write(&fixture_path, serde_json::to_vec(&fixture).unwrap()).unwrap();
    CorpusDocument {
        name: name.to_string(),
        document_path,
        file_type: file_type.to_string(),
        file_size: bytes.len() as u64,
        ground_truth_text: None,
        ground_truth_markdown: None,
        metadata: Default::default(),
        fixture_path,
    }
}

fn cohort(temp: &TempDir, documents: &[CorpusDocument]) -> ModelBenchmarkConfig {
    let fixtures: Vec<_> = documents
        .iter()
        .map(|document| document.fixture_path.file_name().unwrap().to_str().unwrap())
        .collect();
    let manifest = serde_json::json!({
        "schema_version": 1,
        "name": "detector-raster-contract",
        "batch_size": 1,
        "fixtures": fixtures,
    });
    let path = temp.path().join("cohort.json");
    std::fs::write(&path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    ModelBenchmarkConfig {
        fixtures_dir: temp.path().to_path_buf(),
        cohort: path,
        ..Default::default()
    }
}

#[test]
fn should_load_png_and_both_jpeg_extension_spellings_without_skipping() {
    let temp = tempfile::tempdir().unwrap();
    let documents = [
        document(&temp, "png", "png", &raster(ImageFormat::Png)),
        document(&temp, "jpg", "jpg", &raster(ImageFormat::Jpeg)),
        document(&temp, "jpeg", "jpeg", &raster(ImageFormat::Jpeg)),
    ];
    let loaded = load_documents(&cohort(&temp, &documents)).unwrap();
    assert_eq!(loaded.len(), 3);
    assert_eq!(
        loaded
            .iter()
            .map(|document| document.name.as_str())
            .collect::<std::collections::BTreeSet<_>>(),
        std::collections::BTreeSet::from(["png", "jpg", "jpeg"])
    );
}

#[test]
fn should_plan_mixed_inputs_with_exact_limited_page_and_inference_counts() {
    let temp = tempfile::tempdir().unwrap();
    let png = raster(ImageFormat::Png);
    let jpeg = raster(ImageFormat::Jpeg);
    let documents = [
        document(&temp, "paper", "pdf", &three_page_pdf()),
        document(&temp, "png", "png", &png),
        document(&temp, "jpeg", "jpeg", &jpeg),
    ];
    let config = ModelBenchmarkConfig {
        expected_pages: Some(4),
        max_pages: 2,
        iterations: 3,
        ..cohort(&temp, &documents)
    };
    let plan = plan_documents(&load_documents(&config).unwrap(), &config).unwrap();
    assert_eq!(plan.len(), 3);
    assert_eq!(plan["paper"].pages, 2);
    assert_eq!(plan["png"].pages, 1);
    assert_eq!(plan["jpeg"].pages, 1);
    assert_eq!(plan["png"].document_sha256, hash_bytes(&png));
    assert_eq!(plan["jpeg"].document_sha256, hash_bytes(&jpeg));
    assert_eq!(
        contract::expected_calls(contract::total_pages(&plan).unwrap(), 3).unwrap(),
        24
    );
    assert_eq!(
        plan["png"].fixture_sha256,
        hash_file(&documents[1].fixture_path).unwrap()
    );
}

#[test]
fn should_count_rasters_as_one_page_when_the_declared_total_is_wrong() {
    let temp = tempfile::tempdir().unwrap();
    let documents = [
        document(&temp, "png", "png", &raster(ImageFormat::Png)),
        document(&temp, "jpeg", "jpeg", &raster(ImageFormat::Jpeg)),
    ];
    let config = ModelBenchmarkConfig {
        expected_pages: Some(3),
        ..Default::default()
    };
    assert_eq!(
        plan_documents(&documents, &config).unwrap_err().to_string(),
        "Configuration error: detector page-count mismatch: planned 2 pages"
    );
}

#[test]
fn should_reject_corrupt_rasters_before_model_loading() {
    let temp = tempfile::tempdir().unwrap();
    let documents = [document(&temp, "corrupt", "png", b"not an image")];
    let error = plan_documents(&documents, &Default::default()).unwrap_err().to_string();
    assert!(
        error.starts_with("Benchmark error: invalid detector raster:"),
        "{error}"
    );
}

#[test]
fn should_reject_encoded_formats_that_disagree_with_the_fixture_type() {
    let temp = tempfile::tempdir().unwrap();
    let documents = [document(&temp, "mislabeled", "jpeg", &raster(ImageFormat::Png))];
    assert_eq!(
        plan_documents(&documents, &Default::default()).unwrap_err().to_string(),
        "Configuration error: detector raster format does not match fixture type"
    );
}

fn crc32(bytes: &[u8]) -> u32 {
    const POLYNOMIAL: u32 = 0xedb8_8320;
    let mut crc = u32::MAX;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ if crc & 1 == 0 { 0 } else { POLYNOMIAL };
        }
    }
    !crc
}

#[test]
fn should_reject_decoded_rasters_over_the_existing_security_budget() {
    let temp = tempfile::tempdir().unwrap();
    let mut bytes = raster(ImageFormat::Png);
    bytes[16..20].copy_from_slice(&100_000u32.to_be_bytes());
    bytes[20..24].copy_from_slice(&100_000u32.to_be_bytes());
    let crc = crc32(&bytes[12..29]);
    bytes[29..33].copy_from_slice(&crc.to_be_bytes());
    let documents = [document(&temp, "oversized", "png", &bytes)];
    assert_eq!(
        plan_documents(&documents, &Default::default()).unwrap_err().to_string(),
        "Benchmark error: detector raster exceeds security_limits.max_content_size"
    );
}

#[test]
fn should_reject_unsupported_input_types_without_dropping_their_document() {
    let temp = tempfile::tempdir().unwrap();
    for kind in ["svg", "tif", "tiff", "gif"] {
        let documents = [document(&temp, kind, kind, b"unsupported input")];
        assert_eq!(
            load_documents(&cohort(&temp, &documents)).unwrap_err().to_string(),
            "Configuration error: detector cohort supports only PDF, PNG and JPEG fixtures"
        );
    }
}

#[test]
fn should_bound_encoded_raster_reads_by_the_existing_security_budget() {
    let temp = tempfile::tempdir().unwrap();
    let document = document(&temp, "oversized-encoded", "png", b"PNG");
    let limit = xberg::extractors::security::SecurityLimits::default().max_content_size as u64;
    std::fs::OpenOptions::new()
        .write(true)
        .open(&document.document_path)
        .unwrap()
        .set_len(limit + 1)
        .unwrap();
    assert_eq!(
        plan_documents(&[document], &Default::default())
            .unwrap_err()
            .to_string(),
        "Benchmark error: detector raster exceeds security_limits.max_content_size"
    );
}

#[test]
fn should_preserve_original_raster_bytes_dimensions_and_hashes_at_every_dpi() {
    let temp = tempfile::tempdir().unwrap();
    for (kind, format) in [("png", ImageFormat::Png), ("jpeg", ImageFormat::Jpeg)] {
        let bytes = raster(format);
        let document = document(&temp, kind, kind, &bytes);
        let actual = read_document(&document).unwrap();
        let input = PreparedInput::open(&actual, kind).unwrap();
        assert_eq!(input.page_count(), 1);
        for dpi in [1, 600] {
            let page = input.page(0, dpi).unwrap();
            assert_eq!((page.width, page.height), (7, 11));
            assert_eq!(page.bytes.as_ref(), bytes.as_slice());
            assert_eq!(hash_bytes(&page.bytes), hash_bytes(&bytes));
        }
        assert_eq!(
            input.page(1, 150).err().unwrap().to_string(),
            "Benchmark error: detector page index exceeds prepared input"
        );
    }
}

#[test]
fn should_check_both_detector_dimensions_against_the_decoded_input() {
    let bytes = raster(ImageFormat::Png);
    let input = PreparedInput::open(&bytes, "png").unwrap();
    let page = input.page(0, 150).unwrap();
    let measurement = DetectorMeasurement::from_detection(
        DetectionResult {
            page_width: 7,
            page_height: 11,
            detections: Vec::new(),
        },
        1.0,
    )
    .unwrap();
    let mut measurements = [measurement.clone(), measurement];
    page.validate_measurements(&measurements).unwrap();
    measurements[1].page_height = 12;
    assert_eq!(
        page.validate_measurements(&measurements).unwrap_err().to_string(),
        "Benchmark error: detector result dimensions do not match input pixels"
    );
}
