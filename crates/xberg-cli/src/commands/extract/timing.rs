//! Optional per-stage cold-start timing for `xberg extract --format json`.

use std::time::Instant;
use xberg::ExtractionConfig;

use crate::output::StageTimings;

/// Environment variable that enables per-stage cold-start timing in `xberg extract --format json`.
///
/// Set to `1` (or any non-empty value) to include a `stage_timings` object in the JSON output
/// envelope. Disabled by default so the timing path costs nothing (no extra `Instant::now()`
/// calls, no allocation) when not requested.
pub const STAGE_TIMING_ENV_VAR: &str = "XBERG_EMIT_STAGE_TIMING";

/// Returns `true` when [`STAGE_TIMING_ENV_VAR`] is set to a non-empty value.
///
/// Checked once per invocation; callers should cache the result rather than re-reading the
/// environment on every stage boundary.
pub fn stage_timing_requested() -> bool {
    std::env::var(STAGE_TIMING_ENV_VAR).is_ok_and(|v| !v.is_empty())
}

/// Builds the [`StageTimings`] breakdown for a completed extraction.
///
/// `process_start` is the [`Instant`] captured in `main()` (or `None` if unavailable);
/// `extraction_start` is the [`Instant`] captured immediately before the extraction call;
/// `extraction_time_ms` is the already-computed wall-clock duration of that call.
///
/// `ort_session_and_inference_ms` is populated (as a coarse approximation — see the field's doc
/// comment on [`StageTimings`]) whenever the extraction config has layout or OCR enabled, since
/// both may invoke ONNX Runtime.
pub(super) fn build_stage_timings(
    process_start: Option<Instant>,
    extraction_start: Instant,
    extraction_time_ms: f64,
    config: &ExtractionConfig,
) -> StageTimings {
    let process_init_ms = process_start.map(|start| extraction_start.duration_since(start).as_secs_f64() * 1000.0);
    #[cfg(feature = "layout-detection")]
    let layout_active = config.layout.is_some();
    #[cfg(not(feature = "layout-detection"))]
    let layout_active = false;
    let ort_active = layout_active || config.ocr.is_some();
    StageTimings {
        process_init_ms: process_init_ms.unwrap_or(0.0),
        first_parse_ms: extraction_time_ms,
        ort_session_and_inference_ms: ort_active.then_some(extraction_time_ms),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::commands::env_test;

    const CHILD: &str = "commands::extract::timing::tests::stage_timing_env_child";
    const LLM_API_KEY_VAR: &str = "XBERG_LLM_API_KEY";

    fn run_timing_case(case: &str, value: Option<&str>) {
        env_test::run_child_test(CHILD, env_test::CASE, case, |command| {
            command.env_remove(LLM_API_KEY_VAR).env_remove(STAGE_TIMING_ENV_VAR);
            if let Some(value) = value {
                command.env(STAGE_TIMING_ENV_VAR, value);
            }
        });
    }

    #[test]
    fn stage_timing_requested_is_false_when_env_var_unset() {
        run_timing_case("unset", None);
    }

    #[test]
    fn stage_timing_requested_is_false_when_env_var_empty() {
        run_timing_case("empty", Some(""));
    }

    #[test]
    fn stage_timing_requested_is_true_when_env_var_set() {
        run_timing_case("enabled", Some("1"));
    }

    #[test]
    #[should_panic(expected = "stage timing expectation failed")]
    fn stage_timing_child_reports_a_wrong_expectation() {
        run_timing_case("wrong-expectation", Some("1"));
    }

    #[test]
    #[should_panic(expected = "returned early because it did not read XBERG_CLI_ENV_TEST_CASE")]
    fn stage_timing_child_without_case_fails_the_parent() {
        env_test::run_child_test(CHILD, env_test::CASE, "unset", |command| {
            command.env_remove(LLM_API_KEY_VAR).env_remove(env_test::CASE);
        });
    }

    #[test]
    #[should_panic(expected = "did not run exactly one passing test")]
    fn stage_timing_child_with_wrong_test_name_fails_the_parent() {
        env_test::run_child_test(
            "commands::extract::timing::tests::no_such_child",
            env_test::CASE,
            "unset",
            |command| {
                command.env_remove(LLM_API_KEY_VAR);
            },
        );
    }

    #[test]
    #[ignore = "run with an isolated environment by its parent test"]
    fn stage_timing_env_child() {
        let Some(case) = env_test::child_test_root(env_test::CASE) else {
            return;
        };
        let (expected_input, expected_timing) = match case.to_str().expect("non-UTF-8 case selector") {
            "unset" => (None, false),
            "empty" => (Some(""), false),
            "enabled" => (Some("1"), true),
            "wrong-expectation" => (Some("1"), false),
            _ => panic!("unknown stage timing test case"),
        };
        let input = std::env::var_os(STAGE_TIMING_ENV_VAR);
        assert_eq!(input.as_deref(), expected_input.map(std::ffi::OsStr::new));
        assert_eq!(
            stage_timing_requested(),
            expected_timing,
            "stage timing expectation failed"
        );
    }

    #[test]
    fn build_stage_timings_reports_process_init_and_first_parse() {
        let process_start = Instant::now();
        std::thread::sleep(std::time::Duration::from_millis(5));
        let extraction_start = Instant::now();
        let config = ExtractionConfig::default();

        let timings = build_stage_timings(Some(process_start), extraction_start, 42.0, &config);

        assert!(
            timings.process_init_ms >= 5.0,
            "expected process_init_ms >= 5.0 (slept 5ms before extraction_start), got {}",
            timings.process_init_ms
        );
        assert_eq!(timings.first_parse_ms, 42.0);
        assert_eq!(
            timings.ort_session_and_inference_ms, None,
            "default ExtractionConfig has no layout/ocr, so ORT sub-stage should be absent"
        );
    }

    #[test]
    fn build_stage_timings_reports_zero_process_init_when_process_start_missing() {
        let extraction_start = Instant::now();
        let config = ExtractionConfig::default();

        let timings = build_stage_timings(None, extraction_start, 10.0, &config);

        assert_eq!(timings.process_init_ms, 0.0);
        assert_eq!(timings.first_parse_ms, 10.0);
    }

    #[cfg(feature = "layout-detection")]
    #[test]
    fn build_stage_timings_populates_ort_field_when_layout_active() {
        let extraction_start = Instant::now();
        let config = ExtractionConfig {
            layout: Some(xberg::LayoutDetectionConfig::default()),
            ..ExtractionConfig::default()
        };

        let timings = build_stage_timings(None, extraction_start, 1171.0, &config);

        assert_eq!(timings.ort_session_and_inference_ms, Some(1171.0));
    }

    #[test]
    fn build_stage_timings_populates_ort_field_when_ocr_active() {
        let extraction_start = Instant::now();
        let config = ExtractionConfig {
            ocr: Some(xberg::OcrConfig::default()),
            ..ExtractionConfig::default()
        };

        let timings = build_stage_timings(None, extraction_start, 500.0, &config);

        assert_eq!(timings.ort_session_and_inference_ms, Some(500.0));
    }
}
