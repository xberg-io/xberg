#![cfg(all(feature = "pdf", feature = "ocr", feature = "tokio-runtime"))]

use xberg::{ExtractInput, ExtractionConfig, OcrConfig};

// Synthetic 12pt DejaVu Sans glyph outlines, placed as five rows on a Letter page. ~keep
const OUTLINED: &[u8] = include_bytes!("fixtures/ocr/outlined_prose_2313.pdf");
const SINGLE_OUTLINE: &[u8] = include_bytes!("fixtures/ocr/outlined_caption_2313.pdf");
const RASTER_PROSE: &[u8] = include_bytes!("fixtures/ocr/raster_prose_stubs_2313.pdf");
const RASTER_CAPTION: &[u8] = include_bytes!("fixtures/ocr/raster_caption_stubs_2313.pdf");
const HEADER: &str = "Minutes of the municipal library planning committee and public consultation working group";
const PROSE: [&str; 5] = [
    "The committee met on Tuesday to review the plans for the library.",
    "Members asked about the schedule, budget, and planned parking.",
    "The architect said work would start after the winter season ends.",
    "A vote is planned for the next regular meeting of the council.",
    "Residents may send written comments before the end of the month.",
];

async fn read(pdf: &[u8], force_ocr: bool, fallback: Option<bool>) -> xberg::ExtractedDocument {
    let output = xberg::extract(
        ExtractInput::from_bytes(pdf.to_vec(), "application/pdf", None),
        &ExtractionConfig {
            force_ocr,
            ocr_near_empty_fallback: fallback,
            use_cache: false,
            ocr: Some(OcrConfig::default()),
            ..Default::default()
        },
    )
    .await
    .expect("extract outlined-prose control");
    assert!(output.errors.is_empty(), "{:?}", output.errors);
    let [document] = <[_; 1]>::try_from(output.results).unwrap();
    document
}

#[tokio::test]
async fn auto_ocr_recovers_outline_only_paragraphs_between_native_stubs() {
    let expected = std::iter::once(HEADER)
        .chain(PROSE)
        .chain(std::iter::once("Page 7"))
        .flat_map(str::split_whitespace)
        .collect::<Vec<_>>();
    for force in [true, false] {
        let document = read(OUTLINED, force, None).await;
        let expected_method = if force {
            xberg::types::ExtractionMethod::Ocr
        } else {
            xberg::types::ExtractionMethod::Mixed
        };
        assert_eq!(document.extraction_method, Some(expected_method));
        assert_eq!(
            document.content.split_whitespace().collect::<Vec<_>>(),
            expected,
            "all five outlined lines and both native stubs with force_ocr={force}"
        );
    }
}

#[tokio::test]
async fn page_raster_prose_recovers_between_visible_native_stubs() {
    let expected: Vec<_> = std::iter::once(HEADER)
        .chain(PROSE)
        .chain(std::iter::once("Page 7"))
        .flat_map(str::split_whitespace)
        .collect();
    for (force, method) in [
        (true, xberg::types::ExtractionMethod::Ocr),
        (false, xberg::types::ExtractionMethod::Mixed),
    ] {
        let document = read(RASTER_PROSE, force, None).await;
        assert_eq!(
            document.content.split_whitespace().collect::<Vec<_>>(),
            expected,
            "five raster-only lines and both independently painted native stubs with force_ocr={force}"
        );
        assert_eq!(document.extraction_method, Some(method));
    }

    let disabled = read(RASTER_PROSE, false, Some(false)).await;
    assert_eq!(disabled.content.trim(), format!("{HEADER}\n\nPage 7"));
    assert!(!disabled.metadata.ocr_used);
}

#[tokio::test]
async fn page_raster_figure_and_caption_keep_visible_native_stubs() {
    let document = read(RASTER_CAPTION, false, None).await;
    assert_eq!(document.content.trim(), format!("{HEADER}\n\nPage 7"));
    assert_eq!(document.extraction_method, Some(xberg::types::ExtractionMethod::Native));
    assert!(!document.metadata.ocr_used);
}

#[tokio::test]
async fn explicit_fallback_opt_out_keeps_the_native_stubs() {
    let document = read(OUTLINED, false, Some(false)).await;
    assert_eq!(document.content.trim(), format!("{HEADER}\n\nPage 7"));
    assert!(!document.metadata.ocr_used);
}

#[tokio::test]
async fn a_single_outlined_caption_leaves_native_text_untouched() {
    let document = read(SINGLE_OUTLINE, false, None).await;
    assert_eq!(document.content.trim(), format!("{HEADER}\n\nPage 7"));
    assert!(!document.metadata.ocr_used);
}

#[tokio::test]
async fn an_ocr_read_that_loses_the_native_heading_keeps_the_native_page() {
    let mut pdf = lopdf::Document::load_mem(OUTLINED).expect("load outlined prose fixture");
    let page = pdf.get_pages()[&1];
    let [stream] = <[_; 1]>::try_from(pdf.get_page_contents(page)).expect("one content stream");
    let content = String::from_utf8(pdf.get_page_content(page)).expect("ASCII PDF drawing commands");
    let edited = content
        .replacen("f\nBT", "f\n1 g BT", 1)
        .replace("(Page 7) Tj", "0 g (Page 7) Tj");
    assert_ne!(content, edited);
    pdf.get_object_mut(stream)
        .unwrap()
        .as_stream_mut()
        .unwrap()
        .set_plain_content(edited.into_bytes());
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).expect("serialize invisible-heading control");

    let forced = read(&bytes, true, None).await;
    assert!(!forced.content.contains(HEADER));
    assert!(PROSE.iter().all(|line| forced.content.contains(line)));

    let automatic = read(&bytes, false, None).await;
    assert_eq!(automatic.content.trim(), format!("{HEADER}\n\nPage 7"));
    assert!(!automatic.metadata.ocr_used);
}

/// The five outlined rows painted at three heights: fifteen drawn rows between the same native stubs.
fn fifteen_outlined_rows() -> Vec<u8> {
    let mut pdf = lopdf::Document::load_mem(OUTLINED).expect("load outlined prose fixture");
    let page = pdf.get_pages()[&1];
    let [stream] = <[_; 1]>::try_from(pdf.get_page_contents(page)).expect("one content stream");
    let content = String::from_utf8(pdf.get_page_content(page)).expect("ASCII PDF drawing commands");
    let (outline, stubs) = content.split_once("BT").expect("outline paint before the native stubs");
    let rows: String = [0, -165, -330]
        .iter()
        .map(|dy| format!("q 1 0 0 1 0 {dy} cm\n{outline}Q\n"))
        .collect();
    pdf.get_object_mut(stream)
        .unwrap()
        .as_stream_mut()
        .unwrap()
        .set_plain_content(format!("{rows}BT{stubs}").into_bytes());
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).expect("serialize fifteen-row page");
    bytes
}

#[tokio::test]
async fn auto_ocr_recovers_outlined_rows_past_the_seven_row_paint_budget() {
    let bytes = fifteen_outlined_rows();
    let pdf = xberg_native_pdf::PdfDocument::from_bytes(bytes.clone()).expect("load fifteen-row page");
    assert!(
        pdf.extract_bounded_page_local_paths(0, 512 * 1024, 12_000)
            .unwrap()
            .is_empty(),
        "the page paints more than the seven-row budget allowed"
    );
    assert_eq!(
        pdf.extract_bounded_page_local_paths(0, usize::MAX, usize::MAX)
            .unwrap()
            .len(),
        3
    );

    let document = read(&bytes, false, None).await;
    assert_eq!(document.extraction_method, Some(xberg::types::ExtractionMethod::Mixed));
    for line in std::iter::once(HEADER).chain(PROSE).chain(std::iter::once("Page 7")) {
        assert!(
            document.content.contains(line),
            "missing {line:?} in {:?}",
            document.content
        );
    }
}

/// The five outlined rows with a picture XObject of the given subtype painted below them.
fn outlined_rows_beside_xobject(subtype: &str) -> Vec<u8> {
    use lopdf::{Object, Stream, dictionary};

    let mut pdf = lopdf::Document::load_mem(OUTLINED).expect("load outlined prose fixture");
    let page = pdf.get_pages()[&1];
    let mut dict = dictionary! { "Type" => "XObject", "Subtype" => subtype };
    let body = if subtype == "Image" {
        dict.set("Width", 2);
        dict.set("Height", 2);
        dict.set("ColorSpace", "DeviceGray");
        dict.set("BitsPerComponent", 8);
        vec![90, 160, 160, 90]
    } else {
        dict.set("BBox", vec![0.into(), 0.into(), 1.into(), 1.into()]);
        b"0 0 1 1 re f".to_vec()
    };
    let picture = Stream::new(dict, body);
    let picture = pdf.add_object(picture);
    let resources = pdf
        .get_object_mut(page)
        .and_then(Object::as_dict_mut)
        .and_then(|page| page.get_mut(b"Resources"))
        .and_then(Object::as_dict_mut)
        .expect("inline page resources");
    resources.set("XObject", dictionary! { "Im1" => picture });
    let [stream] = <[_; 1]>::try_from(pdf.get_page_contents(page)).expect("one content stream");
    let content = String::from_utf8(pdf.get_page_content(page)).expect("ASCII PDF drawing commands");
    pdf.get_object_mut(stream)
        .unwrap()
        .as_stream_mut()
        .unwrap()
        .set_plain_content(format!("q 300 0 0 150 156 300 cm /Im1 Do Q\n{content}").into_bytes());
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).expect("serialize outlined page with a picture");
    bytes
}

#[tokio::test]
async fn auto_ocr_recovers_outlined_rows_beside_a_picture() {
    let bytes = outlined_rows_beside_xobject("Image");
    let document = read(&bytes, false, None).await;
    assert_eq!(document.extraction_method, Some(xberg::types::ExtractionMethod::Mixed));
    for line in std::iter::once(HEADER).chain(PROSE).chain(std::iter::once("Page 7")) {
        assert!(
            document.content.contains(line),
            "missing {line:?} in {:?}",
            document.content
        );
    }
}

#[test]
fn vector_inventory_abstains_at_a_form_xobject() {
    let paths = |subtype| {
        xberg_native_pdf::PdfDocument::from_bytes(outlined_rows_beside_xobject(subtype))
            .expect("load outlined page with an XObject")
            .extract_bounded_page_local_paths(0, usize::MAX, usize::MAX)
            .unwrap()
            .len()
    };
    assert_eq!(paths("Image"), 1, "an image XObject leaves the page-local outline readable");
    assert_eq!(paths("Form"), 0, "a form XObject can hold paths the page stream does not show");
}

#[test]
fn vector_inventory_abstains_before_materializing_out_of_budget_paths() {
    let pdf = xberg_native_pdf::PdfDocument::from_bytes(OUTLINED.to_vec()).expect("load outlined prose fixture");
    let paths = pdf.extract_bounded_page_local_paths(0, 512 * 1024, 12_000).unwrap();
    let [outline] = paths.as_slice() else {
        panic!("fixture contains one painted outline path");
    };
    assert!(outline.operations.len() > 3000);
    for (byte_limit, operator_limit) in [(1024, 12_000), (512 * 1024, 1000)] {
        assert!(
            pdf.extract_bounded_page_local_paths(0, byte_limit, operator_limit)
                .unwrap()
                .is_empty()
        );
    }
}

#[tokio::test]
async fn a_raster_figure_keeps_its_native_text_extraction_method() {
    let bytes = include_bytes!("fixtures/pdf/regressions/scan_detect/gh1793-control-native-text-with-figure.pdf");
    let document = read(bytes, false, None).await;
    for line in 1..=12 {
        assert!(document.content.contains(&format!(
            "Line {line}: the fund supports public safety, parks and library services."
        )));
    }
    assert_eq!(document.extraction_method, Some(xberg::types::ExtractionMethod::Native));
}

#[derive(Default)]
struct DocumentCache {
    entry: std::sync::Mutex<Option<(String, Vec<u8>)>>,
    hits: std::sync::atomic::AtomicUsize,
}

#[async_trait::async_trait]
impl xberg::engine::seams::CacheBackend for DocumentCache {
    async fn get(&self, key: &str) -> Option<Vec<u8>> {
        let guard = self.entry.lock().unwrap();
        let (_, bytes) = guard.as_ref().filter(|(stored_key, _)| stored_key == key)?;
        self.hits.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Some(bytes.clone())
    }

    async fn put(&self, key: &str, value: Vec<u8>, _ttl: Option<std::time::Duration>) {
        *self.entry.lock().unwrap() = Some((key.to_owned(), value));
    }
}

#[tokio::test]
async fn cached_auto_ocr_keeps_the_recovered_outline_text() {
    use std::sync::{Arc, atomic::Ordering};

    let cache = Arc::new(DocumentCache::default());
    let engine = xberg::engine::Engine::builder()
        .with_cache_backend(cache.clone())
        .build();
    let config = ExtractionConfig {
        ocr: Some(OcrConfig::default()),
        use_cache: true,
        ..Default::default()
    };
    let extract = || {
        engine.extract(
            ExtractInput::from_bytes(OUTLINED.to_vec(), "application/pdf", None),
            &config,
        )
    };
    let cold = extract().await.unwrap();
    assert!(cold.errors.is_empty(), "{:?}", cold.errors);
    let [cold] = <[_; 1]>::try_from(cold.results).unwrap();
    assert!(PROSE.iter().all(|line| cold.content.contains(line)));
    assert_eq!(cold.extraction_method, Some(xberg::types::ExtractionMethod::Mixed));
    assert_eq!(cache.hits.load(Ordering::Relaxed), 0);

    let warm = extract().await.unwrap();
    assert!(warm.errors.is_empty(), "{:?}", warm.errors);
    let [warm] = <[_; 1]>::try_from(warm.results).unwrap();
    assert_eq!(cache.hits.load(Ordering::Relaxed), 1);
    assert_eq!(serde_json::to_value(cold).unwrap(), serde_json::to_value(warm).unwrap());
}
