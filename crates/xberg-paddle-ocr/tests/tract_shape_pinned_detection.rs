//! End-to-end check that DBNet loads and detects under `tract`, which can only run a plan built
//! for one concrete input shape.
//!
//! The property under test is that the plan is pinned to **the page's own resized extent**, so a
//! page is never padded into a larger canvas before detection. That is not a style preference:
//! both PaddleOCR detection backbones carry `GlobalAveragePool` squeeze-and-excitation blocks
//! that reduce over the whole input, so a padded canvas rescales every channel gate and moves
//! the probability map across the entire page. Cross-engine proof of the resulting equality with
//! ONNX Runtime lives in `xberg`'s `paddle_ocr::tract_parity`, which needs both engines linked;
//! what is checkable here, with tract alone, is that detection runs at each page's own extent and
//! that a second extent gets its own plan rather than being folded onto the first.
//!
//! `#[ignore]`d because it needs a real PaddleOCR detection model on disk. It reads the model
//! from the local Hugging Face cache only — it never downloads — and fails with a clear message
//! when the cache is not populated. Run it with:
//!
//! ```text
//! cargo test --release -p xberg-paddle-ocr --no-default-features --features tract \
//!     --test tract_shape_pinned_detection -- --ignored --nocapture
//! ```

#![allow(clippy::print_stdout, clippy::print_stderr)]
#![cfg(feature = "tract")] // ~keep: exercises the tract-only shape-pinned detection path

use std::path::PathBuf;

use xberg_paddle_ocr::InferenceBackend;
use xberg_paddle_ocr::base_net::BaseNet;
use xberg_paddle_ocr::db_net::DbNet;
use xberg_paddle_ocr::scale_param::ScaleParam;

// Must track `HF_REPO_REVISION` in `xberg`'s paddle_ocr::model_manager; this crate cannot
// reference that constant, so the revision is repeated here. ~keep
const MODEL_REPO_SNAPSHOT: &str =
    ".cache/huggingface/hub/models--xberg-io--paddleocr-onnx-models/snapshots/bc5ec866cf0e798e667808dfa51b0ba8ad0dafc8";
const DETECTION_MODEL: &str = "v2/det/mobile.onnx";
const DETECTION_TARGET_SIDE: u32 = 640;
const PAGE_WIDTH: u32 = 640;
const PAGE_HEIGHT: u32 = 320;

const BOX_SCORE_THRESHOLD: f32 = 0.5;
const BOX_THRESHOLD: f32 = 0.3;
const UN_CLIP_RATIO: f32 = 1.6;

fn cached_detection_model() -> PathBuf {
    let home = std::env::var_os("HOME").expect("HOME is set");
    let path = PathBuf::from(home).join(MODEL_REPO_SNAPSHOT).join(DETECTION_MODEL);
    assert!(
        path.exists(),
        "model not in the local Hugging Face cache: {}",
        path.display()
    );
    path
}

/// Draw a row of blocky `E` glyphs — thin strokes at a text-like scale and stroke density,
/// which DBNet responds to far more reliably than one solid bar.
fn draw_text_row(page: &mut image::RgbImage, top: u32, glyph_count: u32) {
    const GLYPH_WIDTH: u32 = 14;
    const GLYPH_HEIGHT: u32 = 22;
    const STROKE: u32 = 3;
    const ADVANCE: u32 = 20;
    let black = image::Rgb([0, 0, 0]);

    for glyph in 0..glyph_count {
        let left = 30 + glyph * ADVANCE;
        for y in top..top + GLYPH_HEIGHT {
            for x in left..left + STROKE {
                page.put_pixel(x, y, black);
            }
        }
        for bar in 0..3 {
            let bar_top = top + bar * (GLYPH_HEIGHT - STROKE) / 2;
            for y in bar_top..bar_top + STROKE {
                for x in left..left + GLYPH_WIDTH {
                    page.put_pixel(x, y, black);
                }
            }
        }
    }
}

/// A white page carrying two rows of text-like glyphs, sized to fit `width`.
fn page_with_text_rows(width: u32, height: u32) -> image::RgbImage {
    const LEFT_MARGIN: u32 = 30;
    const ADVANCE: u32 = 20;
    const GLYPH_WIDTH: u32 = 14;
    let fitting_glyphs = (width - LEFT_MARGIN - GLYPH_WIDTH) / ADVANCE + 1;

    let mut page = image::RgbImage::from_pixel(width, height, image::Rgb([255, 255, 255]));
    draw_text_row(&mut page, 60, fitting_glyphs.min(24));
    draw_text_row(&mut page, 160, fitting_glyphs.min(16));
    page
}

fn load_detector() -> DbNet {
    let model_path = cached_detection_model();
    let mut db_net = DbNet::new();
    // Named explicitly rather than via `init_model`: that entry point takes the compile-time
    // default backend, which prefers `ort` whenever `ort` is also compiled in. This file is
    // gated on `tract` alone, so under `--features ort,tract` the plain call would build a
    // shape-agnostic ORT detector and every assertion below about resident plans would be
    // testing the wrong engine.
    db_net
        .init_model_on(
            InferenceBackend::Tract,
            model_path.to_str().expect("model path is valid UTF-8"),
            1,
        )
        .expect("DBNet must load under tract");
    db_net
}

fn detect(db_net: &DbNet, page: &image::RgbImage) -> Vec<xberg_paddle_ocr::TextBox> {
    db_net
        .get_text_boxes(
            page,
            &ScaleParam::get_scale_param(page, DETECTION_TARGET_SIDE),
            BOX_SCORE_THRESHOLD,
            BOX_THRESHOLD,
            UN_CLIP_RATIO,
        )
        .expect("shape-pinned tract detection must run")
}

#[test]
#[ignore = "requires the PaddleOCR detection model in the local Hugging Face cache"]
fn should_detect_text_boxes_under_tract_at_the_page_extent() {
    let db_net = load_detector();
    let page = page_with_text_rows(PAGE_WIDTH, PAGE_HEIGHT);
    let scale = ScaleParam::get_scale_param(&page, DETECTION_TARGET_SIDE);
    assert_eq!(
        (scale.dst_width, scale.dst_height),
        (PAGE_WIDTH, PAGE_HEIGHT),
        "this page is already 32-aligned, so detection must run at its own extent"
    );

    let text_boxes = detect(&db_net, &page);

    println!("detections at the page extent: {}", text_boxes.len());
    assert!(
        !text_boxes.is_empty(),
        "two rows of text-like glyphs must produce at least one detection"
    );
    for text_box in &text_boxes {
        for point in &text_box.points {
            assert!(
                point.x <= PAGE_WIDTH && point.y <= PAGE_HEIGHT,
                "detection {point:?} escaped the page"
            );
        }
    }
}

/// A second page shape must get its own plan, not be folded onto the first one's.
///
/// This is the invariant the fixed-canvas design broke: one plan served every page because every
/// page was padded to one shape. `Debug` reports how many plans are resident, which is the only
/// external evidence of which shape a page actually ran at.
#[test]
#[ignore = "requires the PaddleOCR detection model in the local Hugging Face cache"]
fn should_build_one_detection_plan_per_page_extent() {
    let db_net = load_detector();
    assert!(
        format!("{db_net:?}").contains("resident_plans: Some(0)"),
        "no plan may be built before the first page's extent is known: {db_net:?}"
    );

    let landscape = page_with_text_rows(PAGE_WIDTH, PAGE_HEIGHT);
    assert!(!detect(&db_net, &landscape).is_empty(), "landscape page must detect");
    assert!(
        format!("{db_net:?}").contains("resident_plans: Some(1)"),
        "the first page must build exactly one plan: {db_net:?}"
    );

    // Same content, taller page: `ScaleParam` produces a different extent, which cannot run on
    // the first plan.
    let portrait = page_with_text_rows(PAGE_HEIGHT, PAGE_WIDTH);
    assert!(!detect(&db_net, &portrait).is_empty(), "portrait page must detect");
    assert!(
        format!("{db_net:?}").contains("resident_plans: Some(2)"),
        "a second page extent must build its own plan: {db_net:?}"
    );

    // Back to the first extent: served from the cache, no third plan.
    assert!(
        !detect(&db_net, &landscape).is_empty(),
        "landscape page must detect again"
    );
    assert!(
        format!("{db_net:?}").contains("resident_plans: Some(2)"),
        "a repeated extent must reuse its cached plan: {db_net:?}"
    );
}
