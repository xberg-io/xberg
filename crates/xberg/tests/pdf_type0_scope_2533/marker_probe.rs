//! The hidden-layer probe route must read a raster whose invisible layer omits picture words,
//! while a scanned page whose invisible layer carries glyphs without a Unicode value gets no
//! marker coverage layer on that route (#2900): its layer is never offered as ink proof.

use lopdf::{Dictionary, Document, Object, Stream, dictionary};
use std::fmt::Write as _;
use xberg::core::config::{ExtractInput, ExtractionConfig, OcrConfig, PageConfig};
use xberg::types::{ExtractedDocument, ExtractionMethod};

const PAGE_WIDTH: f32 = 612.0;
const PAGE_HEIGHT: f32 = 792.0;
const RASTER_WIDTH: u32 = 1275;
const RASTER_HEIGHT: u32 = 1650;
const RASTER_DPI: u32 = 150;
const PARTIAL_ROWS: &[usize] = &[0, 1, 2, 3, 10];
const COMPLETE_ROWS: &[usize] = &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
/// A native-only word the raster does not hold: its survival pins the marker layer as the winner.
const LAYER_ONLY_WORD: &str = "Sealed voucher X512";

// ~keep Avoid ambiguous adjacent r/n glyphs in the synthetic text.
const LINES: &[(f32, &str)] = &[
    (135.0, "Heading"),
    (
        245.0,
        "The coastal archive opened its wooden shutters before sunrise. Volunteers",
    ),
    (
        279.0,
        "carried maps into the reading room and carefully recorded each original",
    ),
    (313.0, "label beside the central register."),
    (
        435.0,
        "Beyond the courtyard, a narrow pathway led toward the lighthouse. During",
    ),
    (
        469.0,
        "at midday, the keeper inspected a brass compass and repaired a cracked",
    ),
    (503.0, "window before supper."),
    (
        625.0,
        "Across the quiet harbor, fishing boats arrived with silver baskets.",
    ),
    (
        659.0,
        "Neighbors delivered warm bread to the dock and counted lamps while the",
    ),
    (693.0, "evening tide rose gently."),
    (1215.0, "Page 7"),
];

/// One full-page raster shared by both pages, carrying every word of `LINES` as ink.
fn raster_stream() -> Stream {
    let mut operators = String::new();
    operators.push_str("BT /F1 12 Tf\n");
    for &index in COMPLETE_ROWS {
        let (y, text) = LINES[index];
        let y = PAGE_HEIGHT - y * 72.0 / RASTER_DPI as f32;
        writeln!(operators, "1 0 0 1 67.2 {y} Tm ({text}) Tj").expect("write raster words");
    }
    operators.push_str("ET\n");
    let mut document = Document::with_version("1.4");
    let pages = document.new_object_id();
    let font = document.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica", "Encoding" => "WinAnsiEncoding",
    });
    let content = document.add_object(Stream::new(Dictionary::new(), operators.into_bytes()));
    let page = document.add_object(dictionary! {
        "Type" => "Page", "Parent" => pages,
        "MediaBox" => vec![0.into(), 0.into(), PAGE_WIDTH.into(), PAGE_HEIGHT.into()],
        "Resources" => dictionary! { "Font" => dictionary! { "F1" => font } }, "Contents" => content,
    });
    document.objects.insert(
        pages,
        Object::Dictionary(dictionary! {
            "Type" => "Pages", "Kids" => vec![Object::Reference(page)], "Count" => 1
        }),
    );
    let root = document.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
    document.trailer.set("Root", root);
    document.compress();
    let mut bytes = Vec::new();
    document.save_to(&mut bytes).expect("serialize synthetic PDF");
    let source = xberg_native_pdf::PdfDocument::from_bytes(bytes).expect("open synthetic source page");
    let rendered = xberg_native_pdf::rendering::render_page(
        &source,
        0,
        &xberg_native_pdf::rendering::RenderOptions::with_dpi(RASTER_DPI),
    )
    .expect("render independent vector source text");
    assert_eq!((rendered.width, rendered.height), (RASTER_WIDTH, RASTER_HEIGHT));
    let image = image::load_from_memory(&rendered.data)
        .expect("decode rendered PNG")
        .into_rgb8();
    Stream::new(
        dictionary! {
            "Type" => "XObject", "Subtype" => "Image", "ColorSpace" => "DeviceRGB", "BitsPerComponent" => 8,
            "Width" => RASTER_WIDTH, "Height" => RASTER_HEIGHT,
        },
        image.into_raw(),
    )
}

/// Two scanned pages. Page 1 holds an invisible partial layer, so the probe reads its raster.
/// Page 2 holds an invisible complete layer whose extra words are set beside glyphs without a
/// Unicode value; the marker layer covers its picture but the probe never reads it.
fn scanned_pages() -> Vec<u8> {
    let raster = raster_stream();
    let mut document = Document::with_version("1.4");
    let pages = document.new_object_id();
    let font = document.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica", "Encoding" => "WinAnsiEncoding",
    });
    let markers = super::marker_font(&mut document);
    let mut children = Vec::new();
    for (index, rows) in [(0, PARTIAL_ROWS), (1, COMPLETE_ROWS)] {
        let mut operators = format!("q {PAGE_WIDTH} 0 0 {PAGE_HEIGHT} 0 0 cm /Scan Do Q\n");
        operators.push_str("BT /F1 12 Tf 3 Tr\n");
        for &row in rows {
            let (y, text) = LINES[row];
            let y = PAGE_HEIGHT - y * 72.0 / RASTER_DPI as f32;
            writeln!(operators, "1 0 0 1 67.2 {y} Tm ({text}) Tj").expect("write layer words");
        }
        if index == 1 {
            writeln!(operators, "1 0 0 1 67.2 712 Tm ({LAYER_ONLY_WORD}) Tj").expect("write layer-only words");
            operators.push_str("ET\nBT /F2 12 Tf 3 Tr\n");
            writeln!(operators, "1 0 0 1 67.2 688 Tm <0044004400440044> Tj").expect("write markers");
        }
        operators.push_str("ET\n");
        let content = document.add_object(Stream::new(Dictionary::new(), operators.into_bytes()));
        let mut resources = dictionary! { "Font" => dictionary! { "F1" => font } };
        if index == 1 {
            resources
                .get_mut(b"Font")
                .unwrap()
                .as_dict_mut()
                .unwrap()
                .set("F2", markers);
        }
        resources.set("XObject", dictionary! { "Scan" => document.add_object(raster.clone()) });
        let page = document.add_object(dictionary! {
            "Type" => "Page", "Parent" => pages,
            "MediaBox" => vec![0.into(), 0.into(), PAGE_WIDTH.into(), PAGE_HEIGHT.into()],
            "Resources" => resources,
            "Contents" => content,
        });
        children.push(Object::Reference(page));
    }
    document.objects.insert(
        pages,
        Object::Dictionary(dictionary! {
            "Type" => "Pages", "Kids" => children, "Count" => 2
        }),
    );
    let root = document.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
    document.trailer.set("Root", root);
    // No `compress()`: it Flates the ToUnicode stream, which kept its bfchar mapping from loading
    // on this route. The audit reads raw content streams anyway. ~keep
    let mut bytes = Vec::new();
    document.save_to(&mut bytes).expect("serialize synthetic PDF");
    bytes
}

fn config() -> ExtractionConfig {
    ExtractionConfig {
        use_cache: false,
        ocr: Some(OcrConfig::default()),
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ..Default::default()
    }
}

async fn extract(bytes: &[u8], config: &ExtractionConfig) -> ExtractedDocument {
    let batch = xberg::extract(
        ExtractInput::from_bytes(bytes.to_vec(), "application/pdf", None),
        config,
    )
    .await
    .expect("extract the synthetic scans");
    assert!(batch.errors.is_empty(), "{:?}", batch.errors);
    let [document] = <[_; 1]>::try_from(batch.results).unwrap();
    document
}

fn page_words(document: &ExtractedDocument, page: usize) -> Vec<&str> {
    let pages = document.pages.as_deref().expect("page output requested");
    assert_eq!(pages.len(), 2);
    assert_eq!(pages[page].page_number as usize, page + 1);
    pages[page].content.split_whitespace().collect()
}

fn sorted(mut words: Vec<&str>) -> Vec<&str> {
    words.sort_unstable();
    words
}

fn expected(rows: &[usize]) -> Vec<&'static str> {
    rows.iter()
        .flat_map(|&index| LINES[index].1.split_whitespace())
        .collect()
}

#[tokio::test(flavor = "current_thread")]
async fn probe_read_replaces_the_partial_scan_and_keeps_no_marker_coverage_layer() {
    let pdf = scanned_pages();

    let native = extract(
        &pdf,
        &ExtractionConfig {
            disable_ocr: true,
            ..config()
        },
    )
    .await;
    assert_eq!(native.extraction_method, Some(ExtractionMethod::Native));
    assert_eq!(
        sorted(page_words(&native, 0)),
        sorted(expected(PARTIAL_ROWS)),
        "page 1 native layer words"
    );
    let marker_page = page_words(&native, 1);
    let mut marker_words = expected(COMPLETE_ROWS);
    marker_words.extend(LAYER_ONLY_WORD.split_whitespace());
    assert_eq!(
        sorted(marker_page),
        sorted(marker_words),
        "page 2 native marker layer words"
    );
    assert!(
        native
            .processing_warnings
            .iter()
            .any(|warning| warning.message.contains("no Unicode value")),
        "page 2's invisible layer must carry glyphs without a Unicode value: {:?}",
        native.processing_warnings
    );

    let (automatic, decisions) = Box::pin(capture_with_sibling(&pdf)).await;
    assert_eq!(
        decisions,
        vec![
            ("1".to_owned(), "Some(true)".to_owned(), "true".to_owned()),
            ("2".to_owned(), "None".to_owned(), "false".to_owned()),
        ],
        "the probe reads page 1 only; page 2's marker layer is never probed"
    );

    assert_eq!(automatic.extraction_method, Some(ExtractionMethod::Mixed));
    assert!(automatic.metadata.ocr_used);
    assert_eq!(
        sorted(page_words(&automatic, 0)),
        sorted(expected(COMPLETE_ROWS)),
        "the probe read must own every word of page 1's raster"
    );
    let kept = page_words(&automatic, 1);
    assert!(
        kept.contains(&"X512"),
        "page 2 keeps its marker layer's own words: {kept:?}"
    );
    for page in 0..2 {
        assert_eq!(
            automatic.pages.as_deref().unwrap()[page]
                .content
                .matches(char::REPLACEMENT_CHARACTER)
                .count(),
            0,
            "page {} must get no marker coverage layer",
            page + 1
        );
    }
    assert!(
        !automatic
            .processing_warnings
            .iter()
            .any(|warning| warning.message.contains("retained native text")),
        "no page may be certified for marker retention: {:?}",
        automatic.processing_warnings
    );
}

async fn capture_with_sibling(pdf: &[u8]) -> (ExtractedDocument, Vec<super::probe_capture::Decision>) {
    use std::sync::mpsc;
    use std::time::Duration;

    const PROBE_WAIT: Duration = Duration::from_secs(60);
    let (captured_at_probe, wait_for_captured_probe) = mpsc::channel();
    let (sibling_at_probe, wait_for_sibling_probe) = mpsc::channel();
    let sibling_pdf = pdf.to_vec();
    let sibling = tokio::task::spawn_blocking(move || {
        wait_for_captured_probe
            .recv_timeout(PROBE_WAIT)
            .expect("captured extraction reached its first probe");
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("build sibling PDF runtime")
            .block_on(Box::pin(check_sibling_probe(&sibling_pdf, sibling_at_probe)));
    });
    let settings = config();
    let captured = super::probe_capture::capture(
        extract(pdf, &settings),
        "needed",
        Some(Box::new(move || {
            captured_at_probe
                .send(())
                .expect("sibling is waiting at the probe gate");
            // ~keep The captured extraction cannot leave its first probe event until the sibling emits its own.
            wait_for_sibling_probe
                .recv_timeout(PROBE_WAIT)
                .expect("sibling reached its first probe while captured extraction was at its probe");
        })),
    );
    let (captured, sibling) = tokio::time::timeout(2 * PROBE_WAIT, async { tokio::join!(Box::pin(captured), sibling) })
        .await
        .expect("both overlapping extractions must complete within 120 seconds");
    sibling.expect("sibling probe extraction completed");
    captured
}

async fn check_sibling_probe(pdf: &[u8], at_probe: std::sync::mpsc::Sender<()>) {
    let settings = ExtractionConfig {
        use_cache: false,
        ocr: Some(OcrConfig::default()),
        ..Default::default()
    };
    let (document, decisions) = super::probe_capture::capture(
        extract(pdf, &settings),
        "needed",
        Some(Box::new(move || {
            at_probe.send(()).expect("captured extraction is still at its probe")
        })),
    )
    .await;
    assert_eq!(decisions.len(), 2, "sibling must emit both probe decisions");
    assert_eq!(document.extraction_method, Some(ExtractionMethod::Mixed));
    assert!(document.content.contains("Across the quiet harbor"));
    assert!(document.content.contains(LAYER_ONLY_WORD));
}
