//! 2.12.0 paired-section checks: the arrays must be present in the file, and a layout variant that
//! ran next to its baseline must have a `layout_impact` entry.

use super::super::*;
use std::collections::BTreeSet;

/// The 2.12.0 paired sections must be present as arrays in the file itself: both carry
/// `#[serde(default)]`, so a typed load alone cannot tell an omitted key from an empty list.
pub(super) fn validate_paired_sections_present(raw: &serde_json::Value, path: &Path) -> Result<()> {
    for section in ["layout_impact", "head_to_head"] {
        require(
            raw.get(section).is_some_and(serde_json::Value::is_array),
            format!("{}: missing {section} array", path.display()),
        )?;
    }
    Ok(())
}

/// Every xberg layout variant that ran next to its baseline variant (same execution mode) must
/// have at least one `layout_impact` entry, even when no fixture paired: the entry is what makes
/// a layout pass that failed everywhere visible instead of absent.
pub(super) fn validate_layout_impact_coverage(aggregate: &NewConsolidatedResults, path: &Path) -> Result<()> {
    let ran: BTreeSet<(&str, &str)> = aggregate
        .per_fixture_results
        .iter()
        .map(|row| (row.framework.as_str(), row.execution_mode.as_str()))
        .collect();
    let frameworks: BTreeSet<&str> = ran.iter().map(|(framework, _)| *framework).collect();
    for &(framework, mode) in &ran {
        if !framework.starts_with("xberg-") {
            continue;
        }
        let Some(baseline) = crate::aggregate::baseline_framework_for(framework, &frameworks) else {
            continue;
        };
        if !ran.contains(&(baseline.as_str(), mode)) {
            continue;
        }
        require(
            aggregate.layout_impact.iter().any(|impact| {
                impact.layout_framework == framework && impact.baseline_framework == baseline && impact.mode == mode
            }),
            format!(
                "{}: layout_impact has no entry for {framework} vs {baseline} ({mode})",
                path.display()
            ),
        )?;
    }
    Ok(())
}

/// The improved / regressed / unchanged / unscored lists of every `layout_impact` entry must
/// partition its paired fixtures: disjoint, and together exactly `paired_fixture_count` long.
pub(super) fn validate_layout_impact_classification(aggregate: &NewConsolidatedResults, path: &Path) -> Result<()> {
    for impact in &aggregate.layout_impact {
        let lists = [
            &impact.improved_fixture_ids,
            &impact.regressed_fixture_ids,
            &impact.unchanged_fixture_ids,
            &impact.unscored_fixture_ids,
        ];
        let listed: usize = lists.iter().map(|ids| ids.len()).sum();
        let distinct: BTreeSet<&str> = lists.iter().flat_map(|ids| ids.iter().map(String::as_str)).collect();
        require(
            listed == impact.paired_fixture_count && distinct.len() == listed,
            format!(
                "{}: layout_impact {} vs {} ({}, {}) classifies {listed} fixture ids ({} distinct) for {} paired fixtures",
                path.display(),
                impact.layout_framework,
                impact.baseline_framework,
                impact.mode,
                impact.file_type,
                distinct.len(),
                impact.paired_fixture_count
            ),
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aggregate::aggregate_new_format;
    use crate::types::FrameworkCapabilities;

    fn result(framework: &str, fixture: &str, success: bool) -> BenchmarkResult {
        BenchmarkResult {
            framework: framework.to_string(),
            output_format: OutputFormat::Markdown,
            file_path: PathBuf::from(format!("{fixture}.pdf")),
            file_size: 1024,
            success,
            error_message: (!success).then(|| "layout pass failed".to_string()),
            error_kind: if success {
                ErrorKind::None
            } else {
                ErrorKind::FrameworkError
            },
            duration: Duration::from_millis(100),
            extraction_duration: None,
            subprocess_overhead: None,
            metrics: PerformanceMetrics {
                baseline_memory_bytes: 0,
                peak_memory_bytes: 1,
                peak_memory_delta_bytes: 1,
                avg_cpu_percent: 0.0,
                cpu_seconds: 0.0,
                throughput_bytes_per_sec: 1.0,
                p50_memory_bytes: 1,
                p95_memory_bytes: 1,
                p99_memory_bytes: 1,
                ..Default::default()
            },
            quality: success.then(|| QualityMetrics {
                f1_score_text: 0.5,
                f1_score_numeric: 0.5,
                f1_score_layout: Some(0.5),
                quality_score: 0.5,
                missing_tokens: vec![],
                extra_tokens: vec![],
                correct: false,
                reading_order_score: None,
            }),
            iterations: vec![],
            statistics: None,
            cold_start_duration: None,
            file_extension: "pdf".to_string(),
            framework_capabilities: FrameworkCapabilities::default(),
            pdf_metadata: None,
            // Every production failure path reports Unknown: extraction never said whether it OCRed.
            ocr_status: if success { OcrStatus::NotUsed } else { OcrStatus::Unknown },
            extracted_text: None,
            system_load: None,
        }
    }

    fn baseline_and_layout(layout_succeeds: bool) -> NewConsolidatedResults {
        aggregate_new_format(&[
            result("xberg-markdown-baseline", "a", true),
            result("xberg-markdown-layout", "a", layout_succeeds),
        ])
    }

    #[test]
    fn layout_coverage_accepts_an_aggregate_with_layout_impact() {
        let aggregate = baseline_and_layout(true);
        assert_eq!(aggregate.layout_impact.len(), 1);
        validate_layout_impact_coverage(&aggregate, Path::new("aggregated.json")).expect("entry present");
    }

    #[test]
    fn layout_coverage_accepts_an_entry_with_no_paired_fixture() {
        let aggregate = baseline_and_layout(false);
        assert_eq!(aggregate.layout_impact[0].paired_fixture_count, 0);
        validate_layout_impact_coverage(&aggregate, Path::new("aggregated.json")).expect("entry present");
    }

    #[test]
    fn layout_coverage_accepts_a_layout_variant_whose_rows_all_failed_with_unknown_ocr() {
        let aggregate = aggregate_new_format(&[
            result("xberg-markdown-baseline", "a", true),
            result("xberg-markdown-baseline", "b", true),
            result("xberg-markdown-layout", "a", false),
            result("xberg-markdown-layout", "b", false),
        ]);
        let layout_ocr: Vec<Option<bool>> = aggregate
            .per_fixture_results
            .iter()
            .filter(|row| row.framework == "xberg-markdown-layout")
            .map(|row| row.ocr)
            .collect();
        assert_eq!(layout_ocr, [None, None]);

        assert_eq!(aggregate.layout_impact.len(), 1);
        let impact = &aggregate.layout_impact[0];
        assert_eq!(impact.paired_fixture_count, 0);
        assert_eq!(impact.ocr, Some(false));
        validate_layout_impact_coverage(&aggregate, Path::new("aggregated.json")).expect("entry present");
    }

    #[test]
    fn layout_coverage_rejects_empty_layout_impact_when_layout_variants_ran() {
        let mut aggregate = baseline_and_layout(true);
        aggregate.layout_impact.clear();

        let error = validate_layout_impact_coverage(&aggregate, Path::new("aggregated.json"))
            .expect_err("layout variants without layout_impact must fail")
            .to_string();
        assert!(error.contains("layout_impact has no entry"), "{error}");
    }

    #[test]
    fn layout_coverage_ignores_runs_without_layout_variants() {
        let aggregate = aggregate_new_format(&[result("xberg-markdown-baseline", "a", true)]);
        assert!(aggregate.layout_impact.is_empty());
        validate_layout_impact_coverage(&aggregate, Path::new("aggregated.json")).expect("nothing required");
    }

    #[test]
    fn classification_must_partition_the_paired_fixtures() {
        let path = Path::new("aggregated.json");
        let mut aggregate = aggregate_new_format(&[
            result("xberg-markdown-baseline", "a", true),
            result("xberg-markdown-baseline", "b", true),
            result("xberg-markdown-layout", "a", true),
            result("xberg-markdown-layout", "b", true),
        ]);
        assert_eq!(aggregate.layout_impact[0].paired_fixture_count, 2);
        validate_layout_impact_classification(&aggregate, path).expect("built aggregate partitions");

        let impact = &mut aggregate.layout_impact[0];
        let moved = impact.unchanged_fixture_ids.pop().expect("equal scores are unchanged");
        validate_layout_impact_classification(&aggregate, path).expect_err("a dropped id must fail");

        let impact = &mut aggregate.layout_impact[0];
        impact.unchanged_fixture_ids.push(moved.clone());
        impact.improved_fixture_ids.push(moved);
        let error = validate_layout_impact_classification(&aggregate, path)
            .expect_err("a double-listed id must fail")
            .to_string();
        assert!(error.contains("3 fixture ids (2 distinct)"), "{error}");
    }

    #[test]
    fn paired_sections_present_rejects_a_file_without_the_arrays() {
        let path = Path::new("aggregated.json");
        let mut raw = serde_json::to_value(baseline_and_layout(true)).unwrap();
        validate_paired_sections_present(&raw, path).expect("serialized aggregate carries both arrays");

        for section in ["layout_impact", "head_to_head"] {
            let mut missing = raw.clone();
            missing.as_object_mut().unwrap().remove(section);
            let error = validate_paired_sections_present(&missing, path)
                .expect_err("missing section must fail")
                .to_string();
            assert!(error.contains(section), "{error}");
        }
        raw["layout_impact"] = serde_json::Value::Null;
        assert!(validate_paired_sections_present(&raw, path).is_err());
    }
}
