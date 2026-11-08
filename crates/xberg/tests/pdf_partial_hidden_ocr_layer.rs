#![cfg(all(feature = "pdf", feature = "ocr", feature = "tokio-runtime"))]

use lopdf::{Dictionary, Document, Object, Stream, dictionary};
use std::fmt::Write;
use xberg::core::config::{ExtractInput, ExtractionConfig, OcrConfig};
use xberg::types::{ExtractedDocument, ExtractionMethod};
use xberg_native_pdf::{PdfDocument, rendering};

const PAGE_WIDTH: f32 = 612.0;
const PAGE_HEIGHT: f32 = 792.0;
const RASTER_WIDTH: u32 = 1275;
const RASTER_HEIGHT: u32 = 1650;
const RASTER_DPI: u32 = 150;
const PARTIAL: &[usize] = &[0, 1, 2, 3, 10];
const SPARSE: &[usize] = &[0, 10];
const COMPLETE: &[usize] = &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

// ~keep Avoid ambiguous adjacent r/n glyphs in the synthetic OCR text.
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

fn page_pdf(operators: String, raster: Option<Stream>) -> Vec<u8> {
    pages_pdf(vec![(operators, raster)])
}

fn pages_pdf(pages_content: Vec<(String, Option<Stream>)>) -> Vec<u8> {
    let mut document = Document::with_version("1.4");
    let pages = document.new_object_id();
    let font = document.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica", "Encoding" => "WinAnsiEncoding",
    });
    let mut children = Vec::new();
    for (operators, raster) in pages_content {
        let mut resources = dictionary! { "Font" => dictionary! { "F1" => font } };
        if let Some(raster) = raster {
            resources.set("XObject", dictionary! { "Scan" => document.add_object(raster) });
        }
        let content = document.add_object(Stream::new(Dictionary::new(), operators.into_bytes()));
        let page = document.add_object(dictionary! {
            "Type" => "Page", "Parent" => pages,
            "MediaBox" => vec![0.into(), 0.into(), PAGE_WIDTH.into(), PAGE_HEIGHT.into()],
            "Resources" => resources, "Contents" => content,
        });
        children.push(Object::Reference(page));
    }
    let count = children.len() as i64;
    document.objects.insert(
        pages,
        Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => children, "Count" => count }),
    );
    let root = document.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
    document.trailer.set("Root", root);
    document.compress();
    let mut bytes = Vec::new();
    document.save_to(&mut bytes).expect("serialize synthetic PDF");
    bytes
}

fn add_text(operators: &mut String, rows: &[usize], scale: f32, bottom: f32, render_mode: u8, extra: Option<&str>) {
    let font_size = 12.0 * scale;
    let horizontal_scale = 100.0 / scale;
    writeln!(
        operators,
        "BT /F1 {font_size} Tf {horizontal_scale} Tz {render_mode} Tr"
    )
    .expect("write text state");
    for &index in rows {
        let (y, text) = LINES[index];
        let y = bottom + scale * (PAGE_HEIGHT - y * 72.0 / RASTER_DPI as f32);
        writeln!(operators, "1 0 0 1 67.2 {y} Tm ({text}) Tj").expect("write text operation");
    }
    if let Some(extra) = extra {
        writeln!(operators, "1 0 0 1 67.2 143 Tm ({extra}) Tj").expect("write independent text");
    }
    operators.push_str("ET\n");
}

fn make_raster() -> Stream {
    let mut operators = String::new();
    add_text(&mut operators, COMPLETE, 1.0, 0.0, 0, None);
    let vector = PdfDocument::from_bytes(page_pdf(operators, None)).expect("open synthetic source page");
    let rendered = rendering::render_page(&vector, 0, &rendering::RenderOptions::with_dpi(RASTER_DPI))
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

fn scanned_page(raster: &Stream, percent: u8, rows: &[usize], render_mode: u8, extra_native: Option<&str>) -> Vec<u8> {
    let scale = f64::from(percent) / 100.0;
    let image_height = f64::from(PAGE_HEIGHT) * scale;
    let bottom = (f64::from(PAGE_HEIGHT) - image_height) / 2.0;
    let mut operators = format!("q {PAGE_WIDTH} 0 0 {image_height} 0 {bottom} cm /Scan Do Q\n");
    add_text(
        &mut operators,
        rows,
        scale as f32,
        bottom as f32,
        render_mode,
        extra_native,
    );
    let bytes = page_pdf(operators, Some(raster.clone()));
    let native = PdfDocument::from_bytes(bytes.clone()).expect("open scan");
    let handles = native.page_image_handles(0).expect("read painted images");
    assert_eq!(handles.len(), 1);
    let image = &handles[0];
    assert_eq!((image.width, image.height), (RASTER_WIDTH, RASTER_HEIGHT));
    let area = f64::from(image.bbox.width) * f64::from(image.bbox.height);
    let coverage = area / f64::from(PAGE_WIDTH * PAGE_HEIGHT);
    assert!((coverage - f64::from(percent) / 100.0).abs() < 1e-6, "{coverage}");
    assert!(coverage >= 0.85);
    let signals = native.classify_page(0).expect("classify scan").signals;
    assert_eq!(signals.invisible_text_ratio, if render_mode == 3 { 1.0 } else { 0.0 },);
    bytes
}

fn default_tesseract() -> ExtractionConfig {
    ExtractionConfig {
        use_cache: false,
        ocr: Some(OcrConfig::default()),
        ..Default::default()
    }
}

async fn extract(bytes: &[u8], config: &ExtractionConfig) -> ExtractedDocument {
    let mut batch = xberg::extract(
        ExtractInput::from_bytes(bytes.to_vec(), "application/pdf", None),
        config,
    )
    .await
    .expect("extract synthetic scan");
    assert!(batch.errors.is_empty());
    assert_eq!(batch.results.len(), 1);
    batch.results.remove(0)
}

fn words(doc: &ExtractedDocument) -> Vec<&str> {
    let content = doc.content.split_whitespace().collect::<Vec<_>>();
    let pages = doc.pages.as_deref().expect("one extracted page");
    assert_eq!(pages.len(), 1);
    assert_eq!(pages[0].page_number, 1);
    assert_eq!(pages[0].content.split_whitespace().collect::<Vec<_>>(), content);
    content
}

fn expected(rows: &[usize]) -> Vec<&'static str> {
    rows.iter()
        .flat_map(|&index| LINES[index].1.split_whitespace())
        .collect()
}

fn assert_exact_word_inventory(doc: &ExtractedDocument) {
    let mut actual = doc.content.split_whitespace().collect::<Vec<_>>();
    let mut source = expected(COMPLETE);
    assert_eq!(source.len(), 81);
    actual.sort_unstable();
    source.sort_unstable();
    assert_eq!(actual, source);
}

#[tokio::test]
async fn plain_auto_recovers_words_missing_from_a_partial_hidden_layer() {
    let raster = make_raster();
    let mut missing = Vec::new();
    for percent in [85, 100] {
        let pdf = scanned_page(&raster, percent, PARTIAL, 3, None);
        let native = extract(
            &pdf,
            &ExtractionConfig {
                disable_ocr: true,
                ..default_tesseract()
            },
        )
        .await;
        assert_eq!(native.extraction_method, Some(ExtractionMethod::Native));
        assert_eq!(words(&native), expected(PARTIAL));
        let forced = extract(
            &pdf,
            &ExtractionConfig {
                force_ocr: true,
                ..default_tesseract()
            },
        )
        .await;
        assert_exact_word_inventory(&forced);
        let automatic = extract(&pdf, &default_tesseract()).await;
        if words(&automatic) != expected(COMPLETE) {
            missing.push((percent, words(&automatic).len()));
        } else {
            assert!(automatic.metadata.ocr_used);
            assert!(automatic.tables.is_empty());
        }
    }
    assert!(
        missing.is_empty(),
        "automatic OCR lost raster words at these coverage/word counts: {missing:?}"
    );
}

#[tokio::test]
async fn complete_hidden_layer_keeps_its_exact_native_text() {
    let raster = make_raster();
    for percent in [85, 100] {
        let pdf = scanned_page(&raster, percent, COMPLETE, 3, None);
        let native = extract(
            &pdf,
            &ExtractionConfig {
                disable_ocr: true,
                ..default_tesseract()
            },
        )
        .await;
        let automatic = extract(&pdf, &default_tesseract()).await;
        assert_eq!(automatic.extraction_method, Some(ExtractionMethod::Native));
        assert!(!automatic.metadata.ocr_used);
        assert_eq!(automatic.content, native.content);
        assert_eq!(words(&automatic), expected(COMPLETE));
    }
}

#[tokio::test]
async fn sparse_visible_or_hidden_layers_still_use_page_ocr_without_duplication() {
    let raster = make_raster();
    for render_mode in [0, 3] {
        let pdf = scanned_page(&raster, 100, SPARSE, render_mode, None);
        let automatic = extract(&pdf, &default_tesseract()).await;
        assert!(automatic.metadata.ocr_used);
        assert_exact_word_inventory(&automatic);
    }
}

#[tokio::test]
async fn automatic_ocr_keeps_hidden_words_that_the_raster_does_not_contain() {
    let raster = make_raster();
    let pdf = scanned_page(&raster, 100, PARTIAL, 3, Some("Sealed voucher X512"));
    let native = extract(
        &pdf,
        &ExtractionConfig {
            disable_ocr: true,
            ..default_tesseract()
        },
    )
    .await;
    let automatic = extract(&pdf, &default_tesseract()).await;
    assert!(words(&native).contains(&"X512"));
    assert_eq!(automatic.extraction_method, Some(ExtractionMethod::Native));
    assert!(!automatic.metadata.ocr_used);
    assert_eq!(words(&automatic), words(&native));
}

#[tokio::test]
async fn explicit_page_ocr_opt_out_keeps_the_partial_native_layer() {
    let raster = make_raster();
    let pdf = scanned_page(&raster, 100, PARTIAL, 3, None);
    let automatic = extract(
        &pdf,
        &ExtractionConfig {
            ocr_near_empty_fallback: Some(false),
            ..default_tesseract()
        },
    )
    .await;
    assert_eq!(automatic.extraction_method, Some(ExtractionMethod::Native));
    assert!(!automatic.metadata.ocr_used);
    assert_eq!(words(&automatic), expected(PARTIAL));
}

#[tokio::test]
async fn unknown_hidden_font_keeps_the_existing_native_result() {
    let mut operators = "q 612 0 0 792 0 0 cm /Scan Do Q\n".to_owned();
    add_text(&mut operators, PARTIAL, 1.0, 0.0, 3, None);
    assert_eq!(operators.matches("/F1 ").count(), 1);
    let pdf = page_pdf(operators.replace("/F1 ", "/Z9 "), Some(make_raster()));
    let native = extract(
        &pdf,
        &ExtractionConfig {
            disable_ocr: true,
            ..default_tesseract()
        },
    )
    .await;
    assert_eq!(words(&native), expected(PARTIAL));
    let automatic = extract(&pdf, &default_tesseract()).await;
    assert_eq!(automatic.extraction_method, Some(ExtractionMethod::Native));
    assert!(!automatic.metadata.ocr_used);
    assert_eq!(automatic.content, native.content);
    assert_eq!(words(&automatic), words(&native));
}

#[tokio::test]
async fn plain_auto_recovers_trailing_picture_words_with_no_missing_rows() {
    let mut operators = "q 612 0 0 792 0 0 cm /Scan Do Q\n".to_owned();
    add_text(&mut operators, COMPLETE, 1.0, 0.0, 3, None);
    let prefix = "The coastal archive opened its wooden shutters";
    assert!(LINES[1].1.starts_with(prefix));
    assert_eq!(operators.matches(LINES[1].1).count(), 1);
    let pdf = page_pdf(operators.replace(LINES[1].1, prefix), Some(make_raster()));
    let native = extract(
        &pdf,
        &ExtractionConfig {
            disable_ocr: true,
            ..default_tesseract()
        },
    )
    .await;
    let mut truncated = expected(COMPLETE);
    let start = 1 + prefix.split_whitespace().count();
    let end = 1 + LINES[1].1.split_whitespace().count();
    truncated.drain(start..end);
    assert_eq!(words(&native), truncated);
    let forced = extract(
        &pdf,
        &ExtractionConfig {
            force_ocr: true,
            ..default_tesseract()
        },
    )
    .await;
    assert_exact_word_inventory(&forced);

    let automatic = extract(&pdf, &default_tesseract()).await;
    assert_eq!(words(&automatic), expected(COMPLETE));
    assert!(automatic.metadata.ocr_used);
    assert!(automatic.tables.is_empty());
}

fn trailing_cut_scan(raster: &Stream, cut: bool, extra: Option<&str>) -> (String, Option<Stream>) {
    let mut operators = "q 612 0 0 792 0 0 cm /Scan Do Q\n".to_owned();
    add_text(&mut operators, COMPLETE, 1.0, 0.0, 3, extra);
    if cut {
        let prefix = "The coastal archive opened its wooden shutters";
        assert!(LINES[1].1.starts_with(prefix));
        operators = operators.replace(LINES[1].1, prefix);
    }
    (operators, Some(raster.clone()))
}

fn assert_page_words(document: &ExtractedDocument, page: usize, expected: &[&str]) {
    let pages = document.pages.as_ref().expect("page content");
    assert_eq!(pages.len(), 2);
    assert_eq!(pages[page].page_number as usize, page + 1);
    let actual = pages[page].content.split_whitespace().collect::<Vec<_>>();
    assert_eq!(actual, expected, "page {}", page + 1);
}

fn cut_words() -> Vec<&'static str> {
    let mut words = expected(COMPLETE);
    let prefix = "The coastal archive opened its wooden shutters";
    words.drain(1 + prefix.split_whitespace().count()..1 + LINES[1].1.split_whitespace().count());
    assert_eq!(words.len(), 78);
    words
}

#[tokio::test]
async fn multipage_hidden_layers_recover_only_the_pages_missing_picture_words() {
    let raster = make_raster();
    for (first_cut, second_cut) in [(true, false), (true, true), (false, true)] {
        let pdf = pages_pdf(vec![
            trailing_cut_scan(&raster, first_cut, None),
            trailing_cut_scan(&raster, second_cut, None),
        ]);
        let native = extract(
            &pdf,
            &ExtractionConfig {
                disable_ocr: true,
                ..default_tesseract()
            },
        )
        .await;
        for (page, cut) in [first_cut, second_cut].into_iter().enumerate() {
            assert_page_words(&native, page, &if cut { cut_words() } else { expected(COMPLETE) });
        }
        assert!(!native.metadata.ocr_used);

        let automatic = extract(&pdf, &default_tesseract()).await;
        assert!(automatic.metadata.ocr_used, "cuts {first_cut}/{second_cut}");
        assert_eq!(automatic.content.split_whitespace().count(), 162);
        for (page, cut) in [first_cut, second_cut].into_iter().enumerate() {
            assert_page_words(&automatic, page, &expected(COMPLETE));
            let output = &automatic.pages.as_ref().expect("page content")[page];
            let native_page = &native.pages.as_ref().expect("native pages")[page];
            if cut {
                assert_ne!(
                    output.content,
                    native_page.content,
                    "page {} must recover words",
                    page + 1
                );
            } else {
                assert_eq!(output.content, native_page.content);
            }
        }
    }
}

#[tokio::test]
async fn complete_scans_and_native_neighbors_preserve_their_original_page_content() {
    let raster = make_raster();
    let mut native_operators = String::new();
    add_text(&mut native_operators, COMPLETE, 1.0, 0.0, 0, None);
    for native_first in [false, true] {
        let mut pages = vec![trailing_cut_scan(&raster, true, None), (native_operators.clone(), None)];
        if native_first {
            pages.reverse();
        }
        let pdf = pages_pdf(pages);
        let automatic = extract(&pdf, &default_tesseract()).await;
        for page in 0..2 {
            assert_page_words(&automatic, page, &expected(COMPLETE));
        }
        let native_page = if native_first { 0 } else { 1 };
        assert!(
            automatic.pages.as_ref().expect("pages")[native_page]
                .ocr_confidence
                .is_none()
        );
    }

    let pdf = pages_pdf(vec![
        trailing_cut_scan(&raster, false, None),
        trailing_cut_scan(&raster, false, None),
    ]);
    let automatic = extract(&pdf, &default_tesseract()).await;
    assert!(!automatic.metadata.ocr_used);
    for page in 0..2 {
        assert_page_words(&automatic, page, &expected(COMPLETE));
    }
}

#[tokio::test]
async fn multipage_recovery_keeps_unmatched_hidden_words_on_their_own_page() {
    let raster = make_raster();
    let pdf = pages_pdf(vec![
        trailing_cut_scan(&raster, true, Some("Sealed voucher X512")),
        trailing_cut_scan(&raster, true, None),
    ]);
    let native = extract(
        &pdf,
        &ExtractionConfig {
            disable_ocr: true,
            ..default_tesseract()
        },
    )
    .await;
    let automatic = extract(&pdf, &default_tesseract()).await;
    let native_first = &native.pages.as_ref().expect("native pages")[0].content;
    assert!(native_first.contains("X512"));
    assert_eq!(automatic.pages.as_ref().expect("pages")[0].content, *native_first);
    assert_page_words(&automatic, 1, &expected(COMPLETE));
}

#[tokio::test]
async fn zero_hidden_layer_probe_iterations_preserve_both_partial_pages() {
    let raster = make_raster();
    let pdf = pages_pdf(vec![
        trailing_cut_scan(&raster, true, None),
        trailing_cut_scan(&raster, true, None),
    ]);
    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            security_limits: Some(xberg::SecurityLimits {
                max_iterations: 0,
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..default_tesseract()
    };
    let document = extract(&pdf, &config).await;
    assert!(!document.metadata.ocr_used);
    for page in 0..2 {
        assert_page_words(&document, page, &cut_words());
    }
}

#[tokio::test]
async fn tighter_global_or_ocr_probe_limit_recovers_only_the_first_partial_page() {
    let raster = make_raster();
    let pdf = pages_pdf(vec![
        trailing_cut_scan(&raster, true, None),
        trailing_cut_scan(&raster, true, None),
    ]);
    for (global_limit, ocr_limit) in [(1, 2), (2, 1)] {
        let config = ExtractionConfig {
            security_limits: Some(xberg::SecurityLimits {
                max_iterations: global_limit,
                ..Default::default()
            }),
            ocr: Some(OcrConfig {
                security_limits: Some(xberg::SecurityLimits {
                    max_iterations: ocr_limit,
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..default_tesseract()
        };
        let document = extract(&pdf, &config).await;
        assert!(document.metadata.ocr_used, "limits {global_limit}/{ocr_limit}");
        assert_eq!(document.content.split_whitespace().count(), 159);
        assert_page_words(&document, 0, &expected(COMPLETE));
        assert_page_words(&document, 1, &cut_words());
    }
}

fn rot(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            'a'..='z' => (b'a' + (c as u8 - b'a' + 13) % 26) as char,
            'A'..='Z' => (b'A' + (c as u8 - b'A' + 13) % 26) as char,
            '0'..='9' => (b'0' + (c as u8 - b'0' + 5) % 10) as char,
            other => other,
        })
        .collect()
}

fn junk_layer_scan(raster: &Stream, rows: &[usize], render_mode: u8) -> Vec<u8> {
    let mut operators = "q 612 0 0 792 0 0 cm /Scan Do Q\n".to_owned();
    add_text(&mut operators, rows, 1.0, 0.0, render_mode, None);
    for &index in rows {
        let text = LINES[index].1;
        assert_eq!(operators.matches(&format!("({text})")).count(), 1);
        operators = operators.replace(&format!("({text})"), &format!("({})", rot(text)));
    }
    page_pdf(operators, Some(raster.clone()))
}

fn junk_words(rows: &[usize]) -> Vec<String> {
    expected(rows).into_iter().map(rot).collect()
}

#[tokio::test]
async fn confident_ocr_replaces_an_invisible_layer_that_shares_no_words_with_the_picture() {
    let raster = make_raster();
    let pdf = junk_layer_scan(&raster, PARTIAL, 3);
    let native = extract(
        &pdf,
        &ExtractionConfig {
            disable_ocr: true,
            ..default_tesseract()
        },
    )
    .await;
    assert_eq!(words(&native), junk_words(PARTIAL));
    let automatic = extract(&pdf, &default_tesseract()).await;
    assert!(automatic.metadata.ocr_used);
    assert_eq!(words(&automatic), expected(COMPLETE));
    assert!(automatic.tables.is_empty());
}

#[tokio::test]
async fn a_visible_layer_that_disagrees_with_the_picture_is_never_replaced() {
    let raster = make_raster();
    let pdf = junk_layer_scan(&raster, PARTIAL, 0);
    let automatic = extract(&pdf, &default_tesseract()).await;
    let kept = words(&automatic);
    for word in junk_words(PARTIAL) {
        assert!(kept.contains(&word.as_str()), "visible native word {word} was dropped");
    }
}

#[tokio::test]
async fn an_invisible_layer_that_mostly_matches_the_picture_keeps_its_unmatched_words() {
    let raster = make_raster();
    let mut operators = "q 612 0 0 792 0 0 cm /Scan Do Q\n".to_owned();
    add_text(&mut operators, PARTIAL, 1.0, 0.0, 3, Some("Sealed voucher X512"));
    let pdf = page_pdf(operators, Some(raster));
    let automatic = extract(&pdf, &default_tesseract()).await;
    assert!(!automatic.metadata.ocr_used);
    assert!(words(&automatic).contains(&"X512"));
}

// Averaging 4x4 blocks leaves the lines readable at mean confidence 0.30, below the 0.75 gate.
fn blocky(raster: &Stream, factor: usize) -> Stream {
    let (width, height) = (RASTER_WIDTH as usize, RASTER_HEIGHT as usize);
    let mut blurred = raster.clone();
    for top in (0..height).step_by(factor) {
        for left in (0..width).step_by(factor) {
            let (rows, columns) = (top..(top + factor).min(height), left..(left + factor).min(width));
            for channel in 0..3 {
                let at = |y: usize, x: usize| (y * width + x) * 3 + channel;
                let sum: usize = rows
                    .clone()
                    .flat_map(|y| columns.clone().map(move |x| (y, x)))
                    .map(|(y, x)| usize::from(raster.content[at(y, x)]))
                    .sum();
                let mean = (sum / (rows.len() * columns.len())) as u8;
                for y in rows.clone() {
                    for x in columns.clone() {
                        blurred.content[at(y, x)] = mean;
                    }
                }
            }
        }
    }
    blurred
}

#[tokio::test]
async fn low_confidence_ocr_does_not_replace_an_invisible_layer() {
    let pdf = junk_layer_scan(&blocky(&make_raster(), 4), PARTIAL, 3);
    let forced = extract(
        &pdf,
        &ExtractionConfig {
            force_ocr: true,
            pages: Some(xberg::core::config::PageConfig {
                extract_pages: true,
                ..Default::default()
            }),
            ..default_tesseract()
        },
    )
    .await;
    let confidence = forced.pages.as_ref().and_then(|pages| pages[0].ocr_confidence.as_ref());
    let score = confidence
        .and_then(|confidence| confidence.score)
        .expect("calibrated score");
    assert!(score < 0.75, "{score}");
    assert!(
        forced.content.split_whitespace().count() > 60,
        "the picture is still readable"
    );

    let automatic = extract(&pdf, &default_tesseract()).await;
    assert!(!automatic.metadata.ocr_used);
    assert_eq!(words(&automatic), junk_words(PARTIAL));
}
