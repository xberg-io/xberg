#![cfg(all(feature = "pdf", feature = "ocr", feature = "bundle-tessdata-eng"))]

mod helpers;

#[path = "ocr_explicit_sideways_2496/prose.rs"]
mod prose;

use image::RgbImage;
use lopdf::{Document, Object, Stream, dictionary};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, process::Command};
use xberg::core::config::PageConfig;
use xberg::types::{ExtractedDocument, ExtractionMethod};
use xberg::{ExtractionConfig, OcrConfig};
use xberg_native_pdf::PdfDocument;

#[cfg(feature = "auto-rotate-tract")]
const CASE: &str = "explicit_language_block_retains_the_sideways_source_table";
const CHILD: &str = "XBERG_EXPLICIT_SIDEWAYS_CHILD";
#[cfg(feature = "auto-rotate-tract")]
const MODEL_SHA256: &str = "6b742aebce6f0f7f71f747931ac7becfc7c96c51641e14943b291eeb334e7947";
const ENGLISH_SHA256: &str = "7d4322bd2a7749724879683fc3912cb542f19906c83bcc1a52132556427170b2";
// Official tessdata_fast@87416418657359cb625c412a48b6e1d6d41c29bd, blob 527457ca8f8fe1fda7c2f88bce3c0e4be12be9d0.
const OSD_SHA256: &str = "9cf5d576fcc47564f11265841e5ca839001e7e6f38ff7f7aacf46d15a96b00ff";
const PDF: &[u8] = include_bytes!("fixtures/ocr/rotated_financial_table.pdf");
const PDF_SHA256: &str = "516c699ca846a70118f5b75537d56a43e9d2ff303c3b013f2fe19561a9a5c3d4";
// Reconstructed from #2314's public drawing script (Random(3), column-major), not the OCR snapshot.
const SOURCE_ROWS: &str = r"Cash 100	31,290	77,778	71,433	Rent 101	60,446	68,267	50,719
Rent 107	17,194	48,590	79,257	Fuel 108	84,495	78,173	89,356
Fuel 114	62,235	82,114	76,233	Parts 115	73,403	13,544	81,381
Parts 121	8,688	79,477	1,825	Wages 122	66,557	35,659	56,617
Wages 128	61,603	34,094	72,292	Tax 129	83,237	94,512	93,897
Tax 135	30,814	25,232	94,098	Fees 136	31,247	39,564	57,436
Fees 142	61,738	71,006	72,141	Repairs 143	33,947	68,405	39,813
Repairs 149	62,536	52,153	83,863	Freight 150	71,983	44,521	1,601
Freight 156	19,841	30,498	83,312	Supplies 157	54,522	76,117	41,371
Supplies 163	19,973	68,674	51,209	Travel 164	2,728	49,450	80,813
Travel 170	97,257	2,085	88,103	Rates 171	77,328	82,961	17,567
Rates 177	8,492	20,992	99,482	Audit 178	7,974	83,138	82,326
Audit 184	77,576	5,708	39,587	Legal 185	43,674	61,212	46,357
Legal 191	4,164	35,414	62,064	Postage 192	89,134	46,312	79,905
Postage 198	78,055	94,317	50,904	Cash 199	92,761	36,659	96,835
Cash 205	93,702	56,059	51,868	Rent 206	64,260	3,007	77,350
Rent 212	95,536	75,716	58,377	Fuel 213	8,039	88,707	2,885
Fuel 219	17,683	48,009	12,873	Parts 220	48,487	33,015	82,409
Parts 226	4,803	17,921	64,965	Wages 227	59,909	39,242	77,782
Wages 233	28,540	33,914	88,185	Tax 234	78,933	42,046	23,354
Tax 240	57,268	82,236	39,556	Fees 241	47,804	24,380	41,082
Fees 247	55,300	66,585	50,676	Repairs 248	99,466	48,485	78,169
Repairs 254	75,338	46,094	70,105	Freight 255	34,720	39,474	49,535
Freight 261	76,786	53,521	76,679	Supplies 262	13,845	3,628	74,713
Supplies 268	30,559	44,240	89,488	Travel 269	89,718	96,460	17,325
Travel 275	3,856	36,758	79,505	Rates 276	40,734	65,637	29,268
Rates 282	88,085	91,269	21,477	Audit 283	85,784	35,407	31,386
Audit 289	91,668	42,880	71,110	Legal 290	43,063	24,662	88,975
Legal 296	75,067	74,694	13,741	Postage 297	57,147	85,236	91,596
Postage 303	93,661	86,019	27,772	Cash 304	12,816	13,448	78,838
Cash 310	83,065	75,274	35,107	Rent 311	42,300	43,845	88,564
Rent 317	37,449	16,409	8,417	Fuel 318	29,520	57,565	22,288
Fuel 324	63,276	83,823	63,474	Parts 325	10,578	44,233	97,364
Parts 331	11,702	45,199	8,830	Wages 332	85,327	28,676	74,601
Wages 338	53,900	19,861	2,737	Tax 339	59,226	35,568	29,594
Tax 345	38,620	56,086	54,520	Fees 346	15,949	4,544	69,519
Fees 352	15,686	5,892	79,397	Repairs 353	25,109	41,406	75,444
Repairs 359	80,648	99,928	5,990	Freight 360	24,163	36,615	44,678
Freight 366	49,619	94,267	76,957	Supplies 367	84,240	11,311	81,277
Supplies 373	43,478	72,301	36,678	Travel 374	45,356	77,364	17,098
Travel 380	66,346	31,026	4,820	Rates 381	55,317	38,365	68,047
Rates 387	40,689	1,048	10,188	Audit 388	35,643	61,011	45,497
Audit 394	14,271	78,712	70,299	Legal 395	83,223	54,744	38,163
Legal 401	4,212	25,971	53,569	Postage 402	55,120	74,592	53,776
Postage 408	38,322	80,115	34,620	Cash 409	4,758	54,267	20,546
Cash 415	20,573	90,504	5,662	Rent 416	26,258	711	62,667
Rent 422	44,640	41,234	47,311	Fuel 423	81,713	66,965	57,031
Fuel 429	18,230	49,617	49,481	Parts 430	73,352	94,112	29,209";

#[test]
fn omitted_false_and_true_rotation_are_three_distinct_public_configurations() {
    let parse = |value: serde_json::Value| -> OcrConfig {
        let mut block = serde_json::json!({"backend": "tesseract", "language": ["eng"]});
        block
            .as_object_mut()
            .unwrap()
            .extend(value.as_object().unwrap().clone());
        serde_json::from_value(block).unwrap()
    };
    let omitted = parse(serde_json::json!({}));
    let disabled = parse(serde_json::json!({"auto_rotate": false}));
    let enabled = parse(serde_json::json!({"auto_rotate": true}));
    assert_eq!(omitted.auto_rotate, None);
    assert_eq!(disabled.auto_rotate, Some(false));
    assert_eq!(enabled.auto_rotate, Some(true));
    assert!(serde_json::to_value(&omitted).unwrap().get("auto_rotate").is_none());
    assert_eq!(serde_json::to_value(&disabled).unwrap()["auto_rotate"], false);
    assert_eq!(serde_json::to_value(&enabled).unwrap()["auto_rotate"], true);
}

#[cfg(feature = "auto-rotate-tract")]
#[test]
#[ignore = "Run mise run rust:test:explicit-sideways with verified cached models"]
fn explicit_language_block_retains_the_sideways_source_table() {
    if std::env::var(CHILD).as_deref() != Ok(CASE) {
        run_private_child(CASE);
        return;
    }
    let (upright, config) = controls();
    let explicit: OcrConfig = serde_json::from_value(serde_json::json!({
        "backend": "tesseract", "language": ["eng"]
    }))
    .unwrap();
    let implicit = extract("implicit-sideways", PDF, &config);
    let explicit_config = ExtractionConfig {
        ocr: Some(explicit),
        ..config.clone()
    };
    let upright = extract("explicit-upright", &upright, &explicit_config);
    let enabled = extract(
        "enabled-sideways",
        PDF,
        &ExtractionConfig {
            ocr: Some(OcrConfig {
                auto_rotate: Some(true),
                ..Default::default()
            }),
            ..config.clone()
        },
    );
    let automatic = automatic_layout_control(&explicit_config);
    assert_eq!(automatic.content.split_whitespace().count(), 468);
    assert_eq!(source_matches(&automatic.content), 0);
    assert!(
        automatic.tables.is_empty(),
        "an explicit PSM3 must remain the requested route"
    );
    let osd = osd_recognition_control(&explicit_config);
    let actual = extract("explicit-sideways", PDF, &explicit_config);
    let mut evidence_config = explicit_config.clone();
    evidence_config.ocr.as_mut().unwrap().element_config = Some(xberg::types::OcrElementConfig {
        include_elements: true,
        min_level: xberg::types::OcrElementLevel::Word,
        ..Default::default()
    });
    let primary = extract("explicit-primary-evidence", PDF, &evidence_config);
    assert_eq!(primary.content, actual.content);
    assert_source_table(&primary);
    assert_osd_original_geometry(primary.ocr_elements.as_deref().unwrap());
    assert!(actual.ocr_elements.is_none(), "private retry evidence must not leak");
    assert!(actual.metadata.additional.get("auto_rotated").is_none());
    for (name, result) in [
        ("implicit", &implicit),
        ("upright", &upright),
        ("enabled", &enabled),
        ("psm3-no-rotation", &automatic),
        ("psm1-osd-no-rotation", &osd),
        ("explicit", &actual),
    ] {
        report(name, result);
    }
    for result in [&implicit, &upright, &enabled] {
        assert_source_table(result);
    }
    assert_source_table(&actual);
}

#[cfg(not(auto_rotate))]
#[test]
#[ignore = "Run mise run rust:test:explicit-sideways with verified cached models"]
fn explicit_language_block_recovers_without_classifier_features() {
    const NATIVE_CASE: &str = "explicit_language_block_recovers_without_classifier_features";
    if std::env::var(CHILD).as_deref() != Ok(NATIVE_CASE) {
        run_private_child(NATIVE_CASE);
        return;
    }
    let (upright, mut config) = controls();
    config.ocr = Some(OcrConfig::default());
    let upright = extract("native-only-upright", &upright, &config);
    assert_source_table(&upright);
    let custom = automatic_layout_control(&config);
    assert_eq!(custom.content.split_whitespace().count(), 468);
    assert_eq!(source_matches(&custom.content), 0);
    assert!(custom.tables.is_empty());
    let result = extract("native-only-explicit", PDF, &config);
    report("native-only-explicit", &result);
    assert_source_table(&result);
    assert!(result.ocr_elements.is_none());
    assert!(result.metadata.additional.get("auto_rotated").is_none());
}

#[test]
#[ignore = "Run mise run rust:test:explicit-sideways with verified cached models"]
fn explicit_language_block_recovers_the_counterclockwise_source_grid() {
    const CASE: &str = "explicit_language_block_recovers_the_counterclockwise_source_grid";
    if std::env::var(CHILD).as_deref() != Ok(CASE) {
        run_private_child(CASE);
        return;
    }
    let (_, mut config) = controls();
    let source = PdfDocument::from_bytes(PDF.to_vec()).unwrap();
    let images = source.page_image_handles(0).unwrap();
    let raster = images[0].decode().unwrap().to_dynamic_image().unwrap().to_rgb8();
    let counterclockwise = image_pdf(&image::imageops::rotate180(&raster));
    config.ocr = Some(OcrConfig::default());
    let actual = extract("explicit-counterclockwise", &counterclockwise, &config);
    assert_source_table(&actual);
    assert!(actual.ocr_elements.is_none());
    assert!(!actual.metadata.additional.contains_key("auto_rotated"));
}

#[test]
#[ignore = "Run mise run rust:test:explicit-sideways with verified cached models"]
fn explicit_language_block_recovers_the_upside_down_source_grid() {
    const CASE: &str = "explicit_language_block_recovers_the_upside_down_source_grid";
    if std::env::var(CHILD).as_deref() != Ok(CASE) {
        run_private_child(CASE);
        return;
    }
    let (upright_pdf, mut config) = controls();
    let source = PdfDocument::from_bytes(PDF.to_vec()).unwrap();
    let images = source.page_image_handles(0).unwrap();
    let sideways = images[0].decode().unwrap().to_dynamic_image().unwrap().to_rgb8();
    let upside_down_pdf = image_pdf(&image::imageops::rotate270(&sideways));
    config.ocr = Some(
        serde_json::from_value(serde_json::json!({
            "backend": "tesseract", "language": ["eng"]
        }))
        .unwrap(),
    );

    let upright = extract("explicit-180-upright-control", &upright_pdf, &config);
    assert_source_table(&upright);
    let actual = extract("explicit-180-upside-down", &upside_down_pdf, &config);
    assert_source_table(&actual);
    let [page] = actual.pages.as_deref().unwrap() else {
        panic!("one physical page");
    };
    assert!(source_matches(&page.content) >= 456, "95% page token retention");
    assert_eq!(page.tables.len(), 1);
    assert_eq!(page.tables[0].cells, actual.tables[0].cells);
    assert!(actual.ocr_elements.is_none(), "private evidence must not leak");
    assert!(!actual.metadata.additional.contains_key("auto_rotated"));
    assert!(!actual.metadata.additional.contains_key("ocr_recognition_page_down"));
}

#[test]
#[ignore = "Run mise run rust:test:explicit-sideways with verified cached models"]
fn caller_psm1_word_elements_on_an_upside_down_page_carry_no_internal_marker() {
    const CASE: &str = "caller_psm1_word_elements_on_an_upside_down_page_carry_no_internal_marker";
    if std::env::var(CHILD).as_deref() != Ok(CASE) {
        run_private_child(CASE);
        return;
    }
    let (_, mut config) = controls();
    let source = PdfDocument::from_bytes(PDF.to_vec()).unwrap();
    let images = source.page_image_handles(0).unwrap();
    let sideways = images[0].decode().unwrap().to_dynamic_image().unwrap().to_rgb8();
    let upside_down_pdf = image_pdf(&image::imageops::rotate270(&sideways));
    config.ocr = Some(OcrConfig {
        element_config: Some(xberg::types::OcrElementConfig {
            include_elements: true,
            min_level: xberg::types::OcrElementLevel::Word,
            ..Default::default()
        }),
        ..Default::default()
    });
    config
        .ocr
        .as_mut()
        .unwrap()
        .tesseract_config
        .get_or_insert_default()
        .psm = Some(1);
    let result = extract("caller-psm1-elements", &upside_down_pdf, &config);
    assert_eq!(
        result
            .metadata
            .additional
            .get("psm")
            .and_then(serde_json::Value::as_str),
        Some("1")
    );
    assert!(
        result
            .ocr_elements
            .as_ref()
            .is_some_and(|elements| !elements.is_empty()),
        "the caller asked for word elements"
    );
    assert!(
        !result.metadata.additional.contains_key("ocr_recognition_page_down"),
        "internal orientation evidence must not reach a caller"
    );
}

#[test]
#[ignore = "Run mise run rust:test:explicit-sideways with verified cached models"]
fn explicit_osd_retains_the_original_source_grid_without_rotating_pixels() {
    const GRID_CASE: &str = "explicit_osd_retains_the_original_source_grid_without_rotating_pixels";
    if std::env::var(CHILD).as_deref() != Ok(GRID_CASE) {
        run_private_child(GRID_CASE);
        return;
    }
    let _ = tracing_subscriber::fmt()
        .with_env_filter("xberg::ocr::processor::execution::recognition_orientation=debug")
        .with_ansi(false)
        .try_init();
    let (_, mut config) = controls();
    config.ocr = Some(OcrConfig::default());
    let result = osd_recognition_control(&config);
    assert_source_table(&result);
}

#[test]
#[ignore = "Run mise run rust:test:explicit-sideways with verified cached models"]
fn upright_osd_preserves_its_existing_cells_and_content() {
    const UPRIGHT_CASE: &str = "upright_osd_preserves_its_existing_cells_and_content";
    if std::env::var(CHILD).as_deref() != Ok(UPRIGHT_CASE) {
        run_private_child(UPRIGHT_CASE);
        return;
    }
    let (upright, mut config) = controls();
    config.ocr = Some(OcrConfig {
        auto_rotate: None,
        tesseract_config: Some(xberg::types::TesseractConfig {
            psm: Some(1),
            ..Default::default()
        }),
        ..Default::default()
    });
    let result = extract("upright-osd", &upright, &config);
    // This pre-existing PSM1 read duplicates prose; preserve it until that separate defect is repaired.
    assert_eq!(result.content.split_whitespace().count(), 942);
    assert_eq!(source_matches(&result.content), 473);
    assert_eq!(
        hex::encode(Sha256::digest(result.content.as_bytes())),
        "0a1bb659d01c2253893cfcb8727c442d0c7b4146ac1d86aa7413783e2b0e17df"
    );
    assert_eq!(
        hex::encode(Sha256::digest(serde_json::to_vec(&result.tables[0].cells).unwrap())),
        "b1b2893d3d99f08a6262c04e071b353f96fd28a36520833789c7d58474f6d260"
    );
    assert!(result.metadata.additional.get("auto_rotated").is_none());
}

const NO_OSD_ORIGINAL_SHA256: &str = "ecd2f9eadd62662614e11e74d43ebac8286a993c812cb5fc3a2a4ca1f93259f9";

fn no_osd_config(models: &std::path::Path, auto_rotate: Option<bool>) -> ExtractionConfig {
    let (_, mut config) = controls();
    seed_english(models);
    config.ocr = Some(OcrConfig {
        tessdata_path: Some(models.into()),
        auto_rotate,
        ..Default::default()
    });
    config
}

fn assert_original_read(result: &ExtractedDocument) {
    assert_eq!(result.content.split_whitespace().count(), 147);
    assert_eq!(source_matches(&result.content), 0);
    assert!(result.tables.is_empty());
    assert!(result.ocr_elements.is_none(), "private primary evidence must not leak");
    assert_eq!(
        hex::encode(Sha256::digest(result.content.as_bytes())),
        NO_OSD_ORIGINAL_SHA256
    );
}

#[cfg(feature = "auto-rotate-tract")]
#[test]
#[ignore = "Run mise run rust:test:explicit-sideways with verified cached models"]
fn language_only_without_cached_osd_recovers_through_the_classifier() {
    const CASE: &str = "language_only_without_cached_osd_recovers_through_the_classifier";
    if std::env::var(CHILD).as_deref() != Ok(CASE) {
        run_private_child(CASE);
        return;
    }
    let models = tempfile::tempdir().unwrap();
    let config = no_osd_config(models.path(), None);
    let result = extract("unset-no-osd", PDF, &config);
    assert_source_table(&result);
    assert!(result.ocr_elements.is_none(), "private retry evidence must not leak");
    assert_eq!(
        std::fs::read_dir(models.path()).unwrap().count(),
        1,
        "no model download"
    );
}

#[cfg(feature = "auto-rotate-tract")]
#[test]
#[ignore = "Run mise run rust:test:explicit-sideways with verified cached models"]
fn language_only_without_cached_osd_recovers_the_counterclockwise_and_upside_down_grids() {
    const CASE: &str = "language_only_without_cached_osd_recovers_the_counterclockwise_and_upside_down_grids";
    if std::env::var(CHILD).as_deref() != Ok(CASE) {
        run_private_child(CASE);
        return;
    }
    let source = PdfDocument::from_bytes(PDF.to_vec()).unwrap();
    let images = source.page_image_handles(0).unwrap();
    let sideways = images[0].decode().unwrap().to_dynamic_image().unwrap().to_rgb8();
    let models = tempfile::tempdir().unwrap();
    let config = no_osd_config(models.path(), None);
    for (name, raster) in [
        ("unset-no-osd-270", image::imageops::rotate180(&sideways)),
        ("unset-no-osd-180", image::imageops::rotate270(&sideways)),
    ] {
        let result = extract(name, &image_pdf(&raster), &config);
        assert_source_table(&result);
        assert!(result.ocr_elements.is_none(), "{name}: private evidence must not leak");
    }
}

#[test]
#[ignore = "Run mise run rust:test:explicit-sideways with verified cached models"]
fn explicit_false_keeps_the_original_read_with_and_without_cached_osd() {
    const CASE: &str = "explicit_false_keeps_the_original_read_with_and_without_cached_osd";
    if std::env::var(CHILD).as_deref() != Ok(CASE) {
        run_private_child(CASE);
        return;
    }
    let models = tempfile::tempdir().unwrap();
    let missing = no_osd_config(models.path(), Some(false));
    assert_original_read(&extract("false-no-osd", PDF, &missing));
    assert_eq!(
        std::fs::read_dir(models.path()).unwrap().count(),
        1,
        "no model download"
    );
    // The child's TESSDATA_PREFIX carries the OSD pack, so this is the cached-OSD path.
    let (_, mut cached) = controls();
    cached.ocr = Some(OcrConfig {
        auto_rotate: Some(false),
        ..Default::default()
    });
    assert_original_read(&extract("false-cached-osd", PDF, &cached));
}

#[cfg(feature = "auto-rotate-tract")]
#[test]
#[ignore = "Run mise run rust:test:explicit-sideways with verified cached models"]
fn explicit_true_rotates_without_cached_osd() {
    const CASE: &str = "explicit_true_rotates_without_cached_osd";
    if std::env::var(CHILD).as_deref() != Ok(CASE) {
        run_private_child(CASE);
        return;
    }
    let models = tempfile::tempdir().unwrap();
    let config = no_osd_config(models.path(), Some(true));
    let result = extract("true-no-osd", PDF, &config);
    assert_source_table(&result);
    assert_eq!(
        std::fs::read_dir(models.path()).unwrap().count(),
        1,
        "no model download"
    );
}

#[test]
#[ignore = "Run mise run rust:test:explicit-sideways with verified cached models"]
fn cached_file_recovers_when_osd_is_installed_without_changing_the_configuration() {
    const CACHE_CASE: &str = "cached_file_recovers_when_osd_is_installed_without_changing_the_configuration";
    if std::env::var(CHILD).as_deref() != Ok(CACHE_CASE) {
        run_private_child(CACHE_CASE);
        return;
    }
    let (_, mut config) = controls();
    let root = tempfile::tempdir().unwrap();
    let models = root.path().join("models");
    seed_english(&models);
    let source = root.path().join("sideways.pdf");
    std::fs::write(&source, PDF).unwrap();
    config.ocr = Some(OcrConfig {
        tessdata_path: Some(models.clone()),
        ..Default::default()
    });
    config.use_cache = true;
    config.cache_namespace = Some("explicit-osd-transition".into());
    let read = |config: &ExtractionConfig| {
        let result = helpers::extract_uri_document_blocking(&source, Some("application/pdf"), config).unwrap();
        assert_eq!(result.pages.as_ref().unwrap().len(), 1);
        assert_eq!(result.extraction_method, Some(ExtractionMethod::Ocr));
        result
    };
    let before = read(&config);
    assert_before_osd(&before);
    assert_eq!(
        persisted_extraction_entries("explicit-osd-transition"),
        1,
        "the real file-extraction cache must contain the first result"
    );
    let osd = osd_model();
    std::fs::write(models.join("osd.traineddata"), osd).unwrap();
    let fresh = read(&ExtractionConfig {
        use_cache: false,
        ..config.clone()
    });
    assert_source_table(&fresh);
    assert_route("fresh", &fresh, Route::Osd);
    let after = read(&config);
    record_result("cached-after-osd", &after);
    assert_source_table(&after);
    assert_route("after", &after, Route::Osd);
}

#[derive(Default)]
struct DocumentResultsCache {
    entries: std::sync::Mutex<HashMap<String, Vec<u8>>>,
    hits: std::sync::atomic::AtomicUsize,
    writes: std::sync::atomic::AtomicUsize,
    install_osd_on_lookup: std::sync::Mutex<Option<(std::path::PathBuf, Vec<u8>)>>,
}

#[async_trait::async_trait]
impl xberg::engine::seams::CacheBackend for DocumentResultsCache {
    async fn get(&self, key: &str) -> Option<Vec<u8>> {
        if let Some((path, bytes)) = self.install_osd_on_lookup.lock().unwrap().take() {
            std::fs::write(path, bytes).unwrap();
        }
        let value = self.entries.lock().unwrap().get(key).cloned();
        if value.is_some() {
            self.hits.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
        value
    }

    async fn put(&self, key: &str, value: Vec<u8>, _ttl: Option<std::time::Duration>) {
        self.entries.lock().unwrap().insert(key.into(), value);
        self.writes.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
}

#[test]
#[ignore = "Run mise run rust:test:explicit-sideways with verified cached models"]
fn cached_engine_bytes_recover_when_osd_is_installed_without_changing_the_configuration() {
    const ENGINE_CASE: &str = "cached_engine_bytes_recover_when_osd_is_installed_without_changing_the_configuration";
    if std::env::var(CHILD).as_deref() != Ok(ENGINE_CASE) {
        run_private_child(ENGINE_CASE);
        return;
    }
    let (_, mut config) = controls();
    let models = tempfile::tempdir().unwrap();
    seed_english(models.path());
    config.ocr = Some(OcrConfig {
        tessdata_path: Some(models.path().into()),
        ..Default::default()
    });
    config.use_cache = true;
    let cache = std::sync::Arc::new(DocumentResultsCache::default());
    let engine = xberg::engine::Engine::builder()
        .with_cache_backend(cache.clone())
        .build();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let read = |config: &ExtractionConfig| read_engine(&engine, &runtime, config);
    let before = read(&config);
    assert_before_osd(&before);
    assert_eq!(cache.writes.load(std::sync::atomic::Ordering::SeqCst), 1);
    let osd = osd_model();
    std::fs::write(models.path().join("osd.traineddata"), osd).unwrap();
    let fresh = read(&ExtractionConfig {
        use_cache: false,
        ..config.clone()
    });
    assert_source_table(&fresh);
    assert_route("fresh", &fresh, Route::Osd);
    let after = read(&config);
    assert_eq!(
        cache.hits.load(std::sync::atomic::Ordering::SeqCst),
        0,
        "changed OSD capability must miss the outer cache"
    );
    assert_source_table(&after);
    assert_route("after", &after, Route::Osd);
}

#[test]
#[ignore = "Run mise run rust:test:explicit-sideways with verified cached models"]
fn cached_context_changed_during_extraction_is_not_written() {
    const CHANGED_CASE: &str = "cached_context_changed_during_extraction_is_not_written";
    if std::env::var(CHILD).as_deref() != Ok(CHANGED_CASE) {
        run_private_child(CHANGED_CASE);
        return;
    }
    let (_, mut config) = controls();
    let models = tempfile::tempdir().unwrap();
    seed_english(models.path());
    config.ocr = Some(OcrConfig {
        tessdata_path: Some(models.path().into()),
        ..Default::default()
    });
    config.use_cache = true;
    let osd = osd_model();
    let cache = std::sync::Arc::new(DocumentResultsCache {
        install_osd_on_lookup: std::sync::Mutex::new(Some((models.path().join("osd.traineddata"), osd))),
        ..Default::default()
    });
    let engine = xberg::engine::Engine::builder()
        .with_cache_backend(cache.clone())
        .build();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let read = read_engine(&engine, &runtime, &config);
    assert_source_table(&read);
    assert_route("read after install during lookup", &read, Route::Osd);
    assert!(
        cache.install_osd_on_lookup.lock().unwrap().is_none(),
        "lookup installed the verified OSD pack"
    );
    assert_eq!(
        cache.writes.load(std::sync::atomic::Ordering::SeqCst),
        0,
        "a capability change while extracting must not populate the earlier cache identity"
    );
}

#[cfg(any(feature = "auto-rotate-tract", not(auto_rotate)))]
fn automatic_layout_control(base: &ExtractionConfig) -> ExtractedDocument {
    let mut config = base.clone();
    let ocr = config.ocr.as_mut().unwrap();
    ocr.auto_rotate = None;
    ocr.tesseract_config.get_or_insert_default().psm = Some(3);
    assert_eq!(ocr.auto_rotate, None);
    let result = extract("psm3-no-rotation", PDF, &config);
    assert_eq!(
        result
            .metadata
            .additional
            .get("psm")
            .and_then(serde_json::Value::as_str),
        Some("3")
    );
    assert!(
        base.ocr.as_ref().unwrap().tesseract_config.is_none(),
        "keep the primary config unchanged"
    );
    result
}

fn osd_recognition_control(base: &ExtractionConfig) -> ExtractedDocument {
    let mut config = base.clone();
    let ocr = config.ocr.as_mut().unwrap();
    ocr.auto_rotate = None;
    ocr.tesseract_config.get_or_insert_default().psm = Some(1);
    ocr.element_config = Some(xberg::types::OcrElementConfig {
        include_elements: true,
        min_level: xberg::types::OcrElementLevel::Word,
        ..Default::default()
    });
    assert_eq!(ocr.auto_rotate, None);
    let result = extract("psm1-osd-no-rotation", PDF, &config);
    assert_eq!(
        result
            .metadata
            .additional
            .get("psm")
            .and_then(serde_json::Value::as_str),
        Some("1")
    );
    let words = result
        .ocr_elements
        .as_ref()
        .expect("private word geometry requested for qualification");
    let word_count = words
        .iter()
        .filter(|word| word.level == xberg::types::OcrElementLevel::Word)
        .count();
    assert_osd_original_geometry(words);
    assert!(
        source_matches(&result.content) >= 456,
        "OSD recognition retains the independent source text"
    );
    tracing::info!(
        private_words = word_count,
        processed_width = ?result.metadata.additional.get("ocr_processed_image_width"),
        processed_height = ?result.metadata.additional.get("ocr_processed_image_height"),
        "Qualified PSM1 source geometry"
    );
    for word in words
        .iter()
        .filter(|word| ["100", "101", "Cash", "Rent"].contains(&word.text.as_str()))
        .take(8)
    {
        tracing::debug!(anchor = %word.text, geometry = ?word.geometry, "OSD source anchor");
    }
    result
}

fn assert_osd_original_geometry(elements: &[xberg::types::OcrElement]) {
    let words: Vec<_> = elements
        .iter()
        .filter(|word| word.level == xberg::types::OcrElementLevel::Word)
        .collect();
    assert!(!words.is_empty());
    for word in &words {
        let xberg::types::OcrBoundingGeometry::Rectangle {
            left,
            top,
            width,
            height,
        } = word.geometry
        else {
            panic!("Tesseract must return rectangular original-raster geometry");
        };
        assert!(width > 0 && height > 0 && left + width <= 3300 && top + height <= 2552);
    }
    let first = source_anchor(&words, "100");
    let second = source_anchor(&words, "101");
    // The public drawing has both values on row y=60, separated by 620 upright pixels.
    assert!((60.0..=81.0).contains(&first.0) && (60.0..=81.0).contains(&second.0));
    assert!((first.0 - second.0).abs() <= 5.0);
    assert!(
        (first.1 - second.1 - 620.0).abs() <= 40.0,
        "original sideways axes must be preserved"
    );
}

fn source_anchor(words: &[&xberg::types::OcrElement], token: &str) -> (f64, f64) {
    let found: Vec<_> = words.iter().filter(|word| word.text == token).collect();
    assert_eq!(found.len(), 1, "unique source value {token}");
    let xberg::types::OcrBoundingGeometry::Rectangle {
        left,
        top,
        width,
        height,
    } = found[0].geometry
    else {
        unreachable!("rectangles checked before source anchors");
    };
    assert!(height > width, "OSD word boxes remain sideways in the original raster");
    (
        (f64::from(left) + f64::from(width) / 2.0) / 2.0,
        (f64::from(top) + f64::from(height) / 2.0) / 2.0,
    )
}

#[cfg(any(feature = "auto-rotate-tract", not(auto_rotate)))]
fn report(name: &str, result: &ExtractedDocument) {
    tracing::info!(
        route = name,
        words = result.content.split_whitespace().count(),
        source_matches = source_matches(&result.content),
        ordered_cells = source_cell_matches(result),
        tables = result.tables.len(),
        pages = result.pages.as_ref().unwrap().len(),
        method = ?result.extraction_method,
        "Explicit sideways OCR qualification"
    );
}

fn run_private_child(case: &str) {
    let root = tempfile::tempdir().unwrap();
    let hub = root.path().join("hub");
    #[cfg(feature = "auto-rotate-tract")]
    {
        let model = std::fs::read(
            std::env::var_os("XBERG_ORIENTATION_TEST_MODEL")
                .expect("set the pinned orientation model path; this test must not silently skip"),
        )
        .unwrap();
        assert_eq!(model.len(), 6_785_465);
        assert_eq!(hex::encode(Sha256::digest(&model)), MODEL_SHA256);
        let destination = hub.join(concat!(
            "models--xberg-io--paddleocr-onnx-models/snapshots/bc5ec866cf0e798e667808dfa51b0ba8ad0dafc8/",
            "v2/classifiers/PP-LCNet_x1_0_doc_ori.onnx"
        ));
        std::fs::create_dir_all(destination.parent().unwrap()).unwrap();
        std::fs::write(destination, model).unwrap();
    }
    let tessdata = private_tessdata(root.path());
    let output = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", case, "--ignored", "--nocapture", "--test-threads=1"])
        .env(CHILD, case)
        .env("HF_HOME", root.path())
        .env("HF_HUB_CACHE", hub)
        .env("HF_HUB_OFFLINE", "1")
        .env("HUGGINGFACE_HUB_OFFLINE", "1")
        .env("TESSDATA_PREFIX", tessdata)
        .env("XBERG_CACHE_DIR", root.path().join("xberg"))
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success() && stdout.contains("test result: ok. 1 passed; 0 failed; 0 ignored;"),
        "2496 child {}\n{stdout}\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
}

fn seed_english(directory: &std::path::Path) {
    std::fs::create_dir_all(directory).unwrap();
    let english = xberg_tesseract::bundled_eng_traineddata().expect("bundled real English model");
    assert_eq!(hex::encode(Sha256::digest(english)), ENGLISH_SHA256);
    std::fs::write(directory.join("eng.traineddata"), english).unwrap();
}

fn osd_model() -> Vec<u8> {
    let osd = std::fs::read(std::env::var_os("XBERG_OSD_TEST_MODEL").expect("set the pinned OSD model path")).unwrap();
    assert_eq!(osd.len(), 10_562_727);
    assert_eq!(hex::encode(Sha256::digest(&osd)), OSD_SHA256);
    osd
}

fn persisted_extraction_entries(namespace: &str) -> usize {
    let cache = std::path::PathBuf::from(std::env::var_os("XBERG_CACHE_DIR").unwrap())
        .join("extraction")
        .join(namespace);
    std::fs::read_dir(cache)
        .unwrap()
        .map(Result::unwrap)
        .filter(|entry| entry.path().extension().is_some_and(|extension| extension == "msgpack"))
        .count()
}

fn read_engine(
    engine: &xberg::engine::Engine,
    runtime: &tokio::runtime::Runtime,
    config: &ExtractionConfig,
) -> ExtractedDocument {
    let result = runtime
        .block_on(engine.extract(
            xberg::ExtractInput::from_bytes(PDF.to_vec(), "application/pdf", None),
            config,
        ))
        .unwrap();
    assert!(result.errors.is_empty());
    assert_eq!(result.results.len(), 1);
    result.results.into_iter().next().unwrap()
}

fn private_tessdata(root: &std::path::Path) -> std::path::PathBuf {
    assert_eq!(xberg_tesseract::TesseractAPI::version(), "5.5.4");
    let tessdata = root.join("tessdata");
    seed_english(&tessdata);
    std::fs::write(tessdata.join("osd.traineddata"), osd_model()).unwrap();
    tessdata
}

fn controls() -> (Vec<u8>, ExtractionConfig) {
    assert_eq!(hex::encode(Sha256::digest(PDF)), PDF_SHA256);
    let source = PdfDocument::from_bytes(PDF.to_vec()).unwrap();
    assert_eq!(source.page_count().unwrap(), 1);
    assert!(
        source
            .extract_page_text_with_options(0, xberg_native_pdf::document::ReadingOrder::TopToBottom)
            .unwrap()
            .spans
            .is_empty(),
        "the source must have no native text layer"
    );
    let images = source.page_image_handles(0).unwrap();
    assert_eq!(images.len(), 1);
    assert_eq!((images[0].width, images[0].height), (1650, 1276));
    let raster = images[0].decode().unwrap().to_dynamic_image().unwrap().to_rgb8();
    assert!(
        raster
            .pixels()
            .filter(|pixel| pixel.0.iter().all(|value| *value < 128))
            .count()
            > 10_000
    );
    let upright = image::imageops::rotate90(&raster);
    assert_eq!(upright.dimensions(), (1276, 1650));
    let config = ExtractionConfig {
        force_ocr: true,
        use_cache: false,
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    (image_pdf(&upright), config)
}

fn image_pdf(image: &RgbImage) -> Vec<u8> {
    let mut pdf = Document::with_version("1.4");
    let pages = pdf.new_object_id();
    let (width, height) = (
        image.width() as f32 * 72.0 / 150.0,
        image.height() as f32 * 72.0 / 150.0,
    );
    let scan = pdf.add_object(Stream::new(
        dictionary! {
            "Type"=>"XObject", "Subtype"=>"Image", "Width"=>image.width(), "Height"=>image.height(),
            "ColorSpace"=>"DeviceRGB", "BitsPerComponent"=>8,
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

fn extract(name: &str, bytes: &[u8], config: &ExtractionConfig) -> ExtractedDocument {
    let result = helpers::extract_bytes_document_blocking(bytes, "application/pdf", config)
        .expect("actual public PDF extraction");
    assert_eq!(result.pages.as_ref().expect("requested pages").len(), 1);
    assert!(
        !result.content.trim().is_empty(),
        "{name}: actual text must be nonempty"
    );
    assert_eq!(result.extraction_method, Some(ExtractionMethod::Ocr));
    record_result(name, &result);
    result
}

fn record_result(name: &str, result: &ExtractedDocument) {
    if let Some(root) = std::env::var_os("XBERG_2496_RECEIPTS") {
        let root = std::path::PathBuf::from(root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join(format!("{name}.json")),
            serde_json::to_vec_pretty(result).unwrap(),
        )
        .unwrap();
    }
}

fn source_matches(text: &str) -> usize {
    let mut actual = HashMap::new();
    for word in text.split_whitespace() {
        *actual.entry(word.trim_matches(['.', '°'])).or_insert(0_usize) += 1;
    }
    SOURCE_ROWS
        .split_whitespace()
        .filter(|word| {
            let count = actual.entry(word).or_default();
            if *count == 0 {
                false
            } else {
                *count -= 1;
                true
            }
        })
        .count()
}

fn source_cell_matches(result: &ExtractedDocument) -> usize {
    let Some(table) = result.tables.first() else {
        return 0;
    };
    SOURCE_ROWS
        .lines()
        .zip(&table.cells)
        .map(|(expected, actual)| {
            expected
                .split('\t')
                .zip(actual)
                .filter(|(source, cell)| {
                    let normalized = cell
                        .split_whitespace()
                        .map(|word| word.trim_matches(['.', '°']))
                        .collect::<Vec<_>>()
                        .join(" ");
                    *source == normalized
                })
                .count()
        })
        .sum()
}

#[derive(Debug, PartialEq)]
enum Route {
    Osd,
    Classifier,
    Original,
}

/// The OSD recognition read is forced to PSM 1 and never rotates pixels; the classifier route
/// rotates the raster and records it, so a stale entry from the other route is visible. ~keep
fn assert_route(name: &str, result: &ExtractedDocument, expected: Route) {
    let psm1 = result
        .metadata
        .additional
        .get("psm")
        .and_then(serde_json::Value::as_str)
        == Some("1");
    let rotated = result.metadata.additional.contains_key("auto_rotated");
    let observed = match (psm1, rotated) {
        (true, false) => Route::Osd,
        (false, true) => Route::Classifier,
        (false, false) => Route::Original,
        _ => panic!("{name}: ambiguous route psm1={psm1} auto_rotated={rotated}"),
    };
    assert_eq!(observed, expected, "{name}: served from the wrong orientation route");
}

/// The unset policy's read before the OSD pack exists: the classifier recovers the table when one
/// is compiled; otherwise the original read is kept. ~keep
fn assert_before_osd(before: &ExtractedDocument) {
    #[cfg(auto_rotate)]
    {
        assert_source_table(before);
        assert_route("before", before, Route::Classifier);
    }
    #[cfg(not(auto_rotate))]
    {
        assert_original_read(before);
        assert_route("before", before, Route::Original);
    }
}

fn assert_source_table(result: &ExtractedDocument) {
    assert_eq!(SOURCE_ROWS.split_whitespace().count(), 480);
    assert!(
        source_matches(&result.content) >= 456,
        "at least 95% of 480 independent source tokens; got {}",
        source_matches(&result.content)
    );
    assert_eq!(result.tables.len(), 1, "retain the complete physical table");
    assert_eq!(result.tables[0].cells.len(), 48);
    assert!(result.tables[0].cells.iter().all(|row| row.len() == 8));
    assert!(
        source_cell_matches(result) >= 365,
        "at least 95% of 384 source cells stay on their row"
    );
}
