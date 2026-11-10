use super::pipeline::{ProvenRead, judge_proven_read};
use super::rendering::{MIN_LEGIBLE_PROVEN_DPI, read_may_be_illegible};
use crate::core::config::OcrQualityThresholds;
use crate::types::ImagePreprocessingMetadata;

const CAUSE: &str = "the retry read was rejected as noise";

/// Twenty two-letter words: every one is fragmented, and the sample is above the word minimum.
fn fragmented_text() -> String {
    "ab ".repeat(20).trim().to_owned()
}

const PROSE: &str = "The river runs past the old mill and into the lake below the town where the farmers \
wait for the fields to dry each spring before the planting begins again";

fn thresholds(discard: bool) -> OcrQualityThresholds {
    OcrQualityThresholds {
        discard_suspected_ocr_noise: discard,
        ..Default::default()
    }
}

fn read(used_proven_source_dpi: bool, possibly_illegible: bool, warn_when_kept: bool) -> ProvenRead {
    ProvenRead {
        used_proven_source_dpi,
        possibly_illegible,
        page_idx: 0,
        warn_when_kept,
    }
}

struct Judged {
    text: String,
    warnings: Vec<String>,
    causes: Vec<String>,
}

fn judge(text: &str, read: ProvenRead, discard: bool) -> Judged {
    let mut warnings = Vec::new();
    let mut causes = Vec::new();
    let text = judge_proven_read(text.to_owned(), read, &thresholds(discard), &mut warnings, &mut causes);
    Judged {
        text,
        warnings: warnings
            .into_iter()
            .map(|warning| warning.message.into_owned())
            .collect(),
        causes,
    }
}

#[test]
fn a_noisy_read_of_a_possibly_illegible_image_is_dropped_whatever_the_policy() {
    for discard in [false, true] {
        let judged = judge(&fragmented_text(), read(true, true, true), discard);
        assert!(judged.text.is_empty(), "discard={discard}");
        assert_eq!(judged.causes, [CAUSE], "discard={discard}");
        assert!(judged.warnings.is_empty(), "discard={discard}");
    }
}

#[test]
fn a_clean_read_of_a_possibly_illegible_image_is_kept() {
    let judged = judge(PROSE, read(true, true, true), true);
    assert_eq!(judged.text, PROSE);
    assert!(judged.causes.is_empty() && judged.warnings.is_empty());
}

#[test]
fn a_noisy_read_of_an_unshrunk_image_is_kept_with_a_warning_unless_the_caller_discards() {
    let kept = judge(&fragmented_text(), read(true, false, true), false);
    assert_eq!(kept.text, fragmented_text());
    assert!(kept.causes.is_empty());
    assert_eq!(kept.warnings.len(), 1, "{:?}", kept.warnings);
    assert!(
        kept.warnings[0].contains("its text was retained"),
        "{:?}",
        kept.warnings
    );

    let discarded = judge(&fragmented_text(), read(true, false, false), true);
    assert!(discarded.text.is_empty());
    assert_eq!(discarded.causes, [CAUSE]);
    assert_eq!(discarded.warnings.len(), 1, "{:?}", discarded.warnings);
    assert!(
        discarded.warnings[0].contains("its text was discarded"),
        "{:?}",
        discarded.warnings
    );
}

#[test]
fn a_kept_read_warns_only_on_the_route_that_does_not_assess_afterwards() {
    let judged = judge(&fragmented_text(), read(true, false, false), false);
    assert_eq!(judged.text, fragmented_text());
    assert!(judged.warnings.is_empty(), "{:?}", judged.warnings);
}

#[test]
fn a_clean_read_of_an_unshrunk_image_is_kept_silently() {
    let judged = judge(PROSE, read(true, false, true), true);
    assert_eq!(judged.text, PROSE);
    assert!(judged.causes.is_empty() && judged.warnings.is_empty());
}

#[test]
fn an_empty_read_stays_empty_without_a_cause_or_warning() {
    for shrunk in [false, true] {
        let judged = judge("", read(true, shrunk, true), true);
        assert!(judged.text.is_empty());
        assert!(judged.causes.is_empty() && judged.warnings.is_empty());
    }
}

#[test]
fn a_read_without_a_proven_resolution_is_left_to_the_page_policy() {
    for shrunk in [false, true] {
        let judged = judge(&fragmented_text(), read(false, shrunk, true), true);
        assert_eq!(judged.text, fragmented_text());
        assert!(judged.causes.is_empty() && judged.warnings.is_empty());
    }
}

fn metadata(scale_factor: f64, final_dpi: i32, dimension_clamped: bool) -> ImagePreprocessingMetadata {
    ImagePreprocessingMetadata {
        original_dimensions: (10200, 7200).into(),
        original_dpi: (300.0, 300.0).into(),
        target_dpi: 300,
        scale_factor,
        auto_adjusted: true,
        final_dpi,
        new_dimensions: None,
        resample_method: "test".to_owned(),
        dimension_clamped,
        calculated_dpi: None,
        skipped_resize: scale_factor >= 1.0,
        resize_error: None,
    }
}

#[test]
fn an_unshrunk_image_at_a_low_resolution_is_not_suspect() {
    assert!(
        !read_may_be_illegible(Some(&metadata(1.0, 90, false))),
        "an unshrunk image keeps its own low resolution"
    );
}

#[test]
fn a_plain_downscale_to_the_issue_2832_resolution_is_not_suspect() {
    assert!(!read_may_be_illegible(Some(&metadata(1.0, 300, false))));
    assert!(
        !read_may_be_illegible(Some(&metadata(0.4, 120, false))),
        "10200 px read at 120 dpi"
    );
    assert!(
        !read_may_be_illegible(Some(&metadata(0.34, MIN_LEGIBLE_PROVEN_DPI, false))),
        "at the floor"
    );
}

#[test]
fn a_clamp_a_resolution_below_the_floor_or_no_record_is_suspect() {
    assert!(
        read_may_be_illegible(Some(&metadata(0.34, 72, true))),
        "ad9c80bf0b's clamped tall scan"
    );
    assert!(read_may_be_illegible(Some(&metadata(1.0, 300, true))), "clamp alone");
    assert!(
        read_may_be_illegible(Some(&metadata(0.3, MIN_LEGIBLE_PROVEN_DPI - 1, false))),
        "below the floor without a clamp"
    );
    assert!(read_may_be_illegible(None), "no record of what was done");
}
