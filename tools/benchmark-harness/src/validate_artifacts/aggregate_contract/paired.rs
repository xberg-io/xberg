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

/// Every xberg layout variant that ran next to its baseline variant (same execution mode and a
/// file type layout can affect) must have a `layout_impact` entry for that file type, even when no
/// fixture paired: the entry is what makes a layout pass that failed everywhere visible instead of
/// absent.
pub(super) fn validate_layout_impact_coverage(aggregate: &NewConsolidatedResults, path: &Path) -> Result<()> {
    let rows = &aggregate.per_fixture_results;
    let frameworks: BTreeSet<&str> = rows.iter().map(|row| row.framework.as_str()).collect();
    let ran: BTreeSet<(&str, &str, &str)> = rows
        .iter()
        .filter(|row| crate::aggregate::layout_applies_to(&row.file_type))
        .map(|row| {
            (
                row.framework.as_str(),
                row.execution_mode.as_str(),
                row.file_type.as_str(),
            )
        })
        .collect();
    for &(framework, mode, file_type) in &ran {
        if !framework.starts_with("xberg-") {
            continue;
        }
        let Some(baseline) = crate::aggregate::baseline_framework_for(framework, &frameworks) else {
            continue;
        };
        if !ran.contains(&(baseline.as_str(), mode, file_type)) {
            continue;
        }
        require(
            aggregate.layout_impact.iter().any(|impact| {
                impact.layout_framework == framework
                    && impact.baseline_framework == baseline
                    && impact.mode == mode
                    && impact.file_type == file_type
            }),
            format!(
                "{}: layout_impact has no entry for {framework} vs {baseline} ({mode}, {file_type})",
                path.display()
            ),
        )?;
    }
    Ok(())
}

/// Each `head_to_head` only-list and ambiguous list holds at most 50 ids and exactly `min(count, 50)` of them, so a
/// truncated list is always visible through its count.
pub(super) fn validate_head_to_head_lists(aggregate: &NewConsolidatedResults, path: &Path) -> Result<()> {
    const MAX_LISTED: usize = 50;
    for entry in &aggregate.head_to_head {
        for (list, count) in [
            (&entry.xberg_only_fixture_ids, entry.xberg_only_fixture_count),
            (&entry.competitor_only_fixture_ids, entry.competitor_only_fixture_count),
            (&entry.ambiguous_fixture_ids, entry.ambiguous_fixture_count),
        ] {
            require(
                list.len() == count.min(MAX_LISTED),
                format!(
                    "{}: head_to_head {} vs {} ({}, {}) lists {} of {count} fixture ids",
                    path.display(),
                    entry.xberg_framework,
                    entry.competitor_framework,
                    entry.mode,
                    entry.file_type,
                    list.len()
                ),
            )?;
        }
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

fn paired_delta_is_consistent(delta: &crate::aggregate::PairedDelta) -> bool {
    let medians = [delta.baseline_p50, delta.layout_p50, delta.delta_p50];
    if delta.sample_count == 0 {
        medians.iter().all(Option::is_none)
    } else {
        medians.iter().all(|median| median.is_some_and(f64::is_finite))
    }
}

/// Every `PairedDelta` in the paired sections carries its three medians exactly when
/// `sample_count > 0`, so an unmeasured metric is `null` rather than a `0.0` that reads as "no
/// difference".
pub(super) fn validate_paired_delta_medians(aggregate: &NewConsolidatedResults, path: &Path) -> Result<()> {
    for impact in &aggregate.layout_impact {
        let deltas = [
            impact.sf1.as_ref(),
            Some(&impact.tf1),
            impact.reading_order.as_ref(),
            Some(&impact.quality_score),
            Some(&impact.duration_ms),
            Some(&impact.peak_memory_mb),
        ];
        require(
            deltas.into_iter().flatten().all(paired_delta_is_consistent),
            format!(
                "{}: layout_impact {} vs {} ({}, {}) has medians inconsistent with sample_count",
                path.display(),
                impact.layout_framework,
                impact.baseline_framework,
                impact.mode,
                impact.file_type
            ),
        )?;
    }
    for entry in &aggregate.head_to_head {
        let deltas = [
            Some(&entry.duration_ms),
            Some(&entry.extraction_duration_ms),
            Some(&entry.peak_memory_mb),
            Some(&entry.tf1),
            entry.sf1.as_ref(),
            entry.reading_order.as_ref(),
            Some(&entry.quality_score),
        ];
        require(
            deltas.into_iter().flatten().all(paired_delta_is_consistent),
            format!(
                "{}: head_to_head {} vs {} ({}, {}) has medians inconsistent with sample_count",
                path.display(),
                entry.xberg_framework,
                entry.competitor_framework,
                entry.mode,
                entry.file_type
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
            ocr_status: if success {
                OcrStatus::NotUsed
            } else {
                OcrStatus::Unknown
            },
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
    fn paired_delta_medians_must_be_null_exactly_when_unsampled() {
        let path = Path::new("aggregated.json");
        let mut aggregate = aggregate_new_format(&[
            result("xberg-markdown-baseline", "a", true),
            result("xberg-markdown-layout", "a", true),
            result("liteparse", "a", true),
        ]);
        validate_paired_delta_medians(&aggregate, path).expect("built aggregate is consistent");

        let unsampled = &mut aggregate.layout_impact[0].tf1;
        unsampled.sample_count = 0;
        validate_paired_delta_medians(&aggregate, path).expect_err("medians without samples must fail");

        let mut aggregate = baseline_and_layout(false);
        assert_eq!(aggregate.layout_impact[0].tf1.sample_count, 0);
        validate_paired_delta_medians(&aggregate, path).expect("all-null zero-pair entry is consistent");
        aggregate.layout_impact[0].tf1.delta_p50 = Some(0.0);
        let error = validate_paired_delta_medians(&aggregate, path)
            .expect_err("a 0.0 median without samples must fail")
            .to_string();
        assert!(error.contains("inconsistent with sample_count"), "{error}");
    }

    #[test]
    fn layout_coverage_ignores_file_types_layout_cannot_affect() {
        let mut docx_baseline = result("xberg-markdown-baseline", "a", true);
        let mut docx_layout = result("xberg-markdown-layout", "a", true);
        for docx in [&mut docx_baseline, &mut docx_layout] {
            docx.file_path = PathBuf::from("a.docx");
            docx.file_extension = "docx".to_string();
        }
        let aggregate = aggregate_new_format(&[docx_baseline, docx_layout]);
        assert!(aggregate.layout_impact.is_empty());
        validate_layout_impact_coverage(&aggregate, Path::new("aggregated.json")).expect("nothing required");
    }

    #[test]
    fn head_to_head_lists_must_match_their_counts() {
        let path = Path::new("aggregated.json");
        let mut aggregate = aggregate_new_format(&[
            result("xberg-markdown-baseline", "a", true),
            result("xberg-markdown-baseline", "b", true),
            result("liteparse", "a", true),
        ]);
        let entry = &aggregate.head_to_head[0];
        assert_eq!(entry.xberg_only_fixture_count, 1);
        validate_head_to_head_lists(&aggregate, path).expect("built aggregate is consistent");

        aggregate.head_to_head[0].xberg_only_fixture_count = 2;
        let error = validate_head_to_head_lists(&aggregate, path)
            .expect_err("a list shorter than its uncapped count must fail")
            .to_string();
        assert!(error.contains("lists 1 of 2"), "{error}");
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
