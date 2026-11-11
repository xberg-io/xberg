//! The hidden-layer probe route must read a raster whose invisible layer omits picture words,
//! while a scanned page whose invisible layer carries glyphs without a Unicode value gets no
//! marker coverage layer on that route (#2900): its layer is never offered as ink proof.

use lopdf::{Dictionary, Document, Object, Stream, dictionary};
use std::fmt::Write as _;
use std::sync::{Arc, Mutex};
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

/// A Type0 font whose `ToUnicode` leaves CID 0x44 without a Unicode value, the marker source the
/// covered-layer tests use; each shown code becomes one replacement character.
fn marker_font(document: &mut Document) -> lopdf::ObjectId {
    let program = include_bytes!("../../../xberg-native-pdf/tests/fixtures/fonts/DejaVuSansMono.ttf");
    let file = document.add_object(Stream::new(
        dictionary! { "Length1" => program.len() as i64 },
        program.to_vec(),
    ));
    let descriptor = document.add_object(dictionary! {
        "Type" => "FontDescriptor", "FontName" => "ABCDEF+Probe", "Flags" => 4, "FontFile2" => file,
        "FontBBox" => vec![0.into(), (-200).into(), 1000.into(), 800.into()],
        "ItalicAngle" => 0, "Ascent" => 800, "Descent" => -200, "CapHeight" => 700, "StemV" => 80,
    });
    let cid = document.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "CIDFontType2", "BaseFont" => "ABCDEF+Probe",
        "CIDSystemInfo" => dictionary! { "Registry" => Object::string_literal("Adobe"),
            "Ordering" => Object::string_literal("Identity"), "Supplement" => 0 },
        "FontDescriptor" => descriptor, "DW" => 600, "CIDToGIDMap" => "Identity",
    });
    let cmap = b"begincmap /CMapName /Probe def /CMapType 2 def \
        1 begincodespacerange <0000> <FFFF> endcodespacerange \
        1 beginbfchar <0044> <0000> endbfchar endcmap";
    // The shown marker code stays out of the map: a miss against a present CMap is what keeps a
    // glyph without a Unicode value on this extraction route.
    let unicode = document.add_object(Stream::new(dictionary! {}, cmap.to_vec()));
    document.add_object(
        dictionary! { "Type" => "Font", "Subtype" => "Type0", "BaseFont" => "ABCDEF+Probe",
        "Encoding" => "Identity-H", "DescendantFonts" => vec![cid.into()], "ToUnicode" => unicode },
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
    let markers = marker_font(&mut document);
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

#[tokio::test]
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

    let decisions = probe_decisions(&pdf).await;
    assert_eq!(
        decisions,
        vec![
            ("1".to_owned(), "Some(true)".to_owned(), "true".to_owned()),
            ("2".to_owned(), "None".to_owned(), "false".to_owned()),
        ],
        "the probe reads page 1 only; page 2's marker layer is never probed"
    );

    let automatic = extract(&pdf, &config()).await;
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

#[derive(Default)]
struct Fields(std::collections::BTreeMap<String, String>);

impl tracing::field::Visit for Fields {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.0.insert(field.name().to_owned(), format!("{value:?}"));
    }
}

/// Probe decisions as `(page, needed, probed)` from the `Hidden PDF text probe decision` events.
#[derive(Clone, Default)]
struct ProbeDecisions(Arc<Mutex<Vec<(String, String, String)>>>);

impl<S> tracing_subscriber::Layer<S> for ProbeDecisions
where
    S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    fn on_event(&self, event: &tracing::Event<'_>, _context: tracing_subscriber::layer::Context<'_, S>) {
        let mut fields = Fields::default();
        event.record(&mut fields);
        if fields.0.get("message").map(String::as_str) != Some("Hidden PDF text probe decision") {
            return;
        }
        self.0.lock().expect("decision lock").push((
            fields.0["page"].clone(),
            fields.0["needed"].clone(),
            fields.0["probed"].clone(),
        ));
    }
}

async fn probe_decisions(pdf: &[u8]) -> Vec<(String, String, String)> {
    use tracing_subscriber::layer::SubscriberExt as _;
    static LOG: std::sync::OnceLock<ProbeDecisions> = std::sync::OnceLock::new();
    let log = LOG.get_or_init(|| {
        let decisions = ProbeDecisions::default();
        let subscriber = tracing_subscriber::registry().with(decisions.clone());
        tracing::subscriber::set_global_default(subscriber).expect("the only subscriber in this test binary");
        // Tests that ran earlier cached "no subscriber" as every call site's interest.
        tracing::callsite::rebuild_interest_cache();
        decisions
    });
    log.0.lock().expect("decision lock").clear();
    extract(pdf, &config()).await;
    log.0.lock().expect("decision lock").clone()
}
