//! Aggregation and analysis functions for consolidating multiple benchmark runs

use crate::types::{BenchmarkResult, QualityMetrics};
use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Framework aggregation with per-run variance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameworkAggregation {
    pub framework: String,
    pub total_files: usize,
    pub runs: Vec<RunAggregation>,
    pub mean_duration_ms: f64,
    pub duration_variance_ms2: f64,
    pub duration_std_dev_ms: f64,
    pub mean_throughput_bps: f64,
    pub throughput_variance_bps2: f64,
    pub throughput_std_dev_bps: f64,
    pub mean_peak_memory: u64,
    pub peak_memory_variance: f64,
    pub peak_memory_std_dev: f64,
    pub success_rate: f64,
    pub avg_quality: Option<QualityMetrics>,
    pub by_extension: HashMap<String, ExtensionStats>,
}

/// Aggregation for a single run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunAggregation {
    pub run_index: usize,
    pub file_count: usize,
    pub mean_duration_ms: f64,
    pub mean_throughput_bps: f64,
    pub mean_peak_memory: u64,
    pub success_rate: f64,
}

/// Performance statistics for a specific file extension
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionStats {
    pub extension: String,
    pub file_count: usize,
    pub mean_duration_ms: f64,
    pub p95_duration_ms: f64,
    pub mean_throughput_bps: f64,
    pub success_rate: f64,
}

/// Cross-framework comparison with rankings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossFrameworkComparison {
    pub performance_ranking: Vec<FrameworkRanking>,
    pub throughput_ranking: Vec<FrameworkRanking>,
    pub memory_ranking: Vec<FrameworkRanking>,
    pub reliability_ranking: Vec<FrameworkRanking>,
    pub deltas_vs_baseline: HashMap<String, PerformanceDelta>,
}

/// Framework ranking entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameworkRanking {
    pub framework: String,
    pub rank: usize,
    pub value: f64,
    pub relative_performance: f64,
}

/// Performance delta relative to baseline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceDelta {
    pub framework: String,
    pub duration_delta_ms: f64,
    pub duration_delta_percent: f64,
    pub throughput_delta_bps: f64,
    pub throughput_delta_percent: f64,
    pub memory_delta_bytes: i64,
    pub memory_delta_percent: f64,
}

/// Quality analysis across frameworks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityAnalysis {
    pub by_framework: HashMap<String, FrameworkQuality>,
    pub quality_ranking: Vec<QualityRanking>,
    pub reliability: QualityReliability,
}

/// Quality metrics for a framework
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameworkQuality {
    pub framework: String,
    pub files_with_metrics: usize,
    pub mean_f1_text: f64,
    pub f1_text_std_dev: f64,
    pub mean_f1_numeric: f64,
    pub mean_f1_layout: f64,
    pub mean_quality_score: f64,
    pub quality_score_std_dev: f64,
}

/// Quality ranking entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityRanking {
    pub framework: String,
    pub rank: usize,
    pub mean_quality_score: f64,
    pub consistency_score: f64,
}

/// Quality reliability metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityReliability {
    pub consensus_success_rate: f64,
    pub perfect_frameworks: Vec<String>,
    pub quality_degradation_concerns: Vec<String>,
}

/// Consolidated results from multiple runs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidatedResults {
    pub by_framework: HashMap<String, FrameworkAggregation>,
    pub comparison: CrossFrameworkComparison,
    pub quality: QualityAnalysis,
    pub run_count: usize,
    pub total_files: usize,
    pub framework_count: usize,
}

/// Load benchmark results from results.json files in a directory
pub fn load_run_results(dir: &Path) -> Result<Vec<BenchmarkResult>> {
    let mut results = Vec::new();
    for entry in fs::read_dir(dir).map_err(Error::Io)? {
        let entry = entry.map_err(Error::Io)?;
        let path = entry.path();

        if path.is_file() && path.file_name().is_some_and(|n| n == "results.json") {
            eprintln!("Loading results from {}", path.display());
            let json_content = fs::read_to_string(&path).map_err(Error::Io)?;
            let run_results: Vec<BenchmarkResult> = serde_json::from_str(&json_content)
                .map_err(|e| Error::Benchmark(format!("Failed to parse {}: {}", path.display(), e)))?;
            results.extend(run_results);
        } else if path.is_dir() {
            match load_run_results(&path) {
                Ok(mut run_results) => results.append(&mut run_results),
                Err(e) => eprintln!("Warning: Failed to load results from {}: {}", path.display(), e),
            }
        }
    }
    Ok(results)
}

/// Main consolidation orchestrator
pub fn consolidate_runs(runs: Vec<Vec<BenchmarkResult>>) -> Result<ConsolidatedResults> {
    if runs.is_empty() {
        return Err(Error::Benchmark("No runs provided".to_string()));
    }

    eprintln!("Consolidating {} runs", runs.len());
    let all_results: Vec<&BenchmarkResult> = runs.iter().flat_map(|r| r.iter()).collect();

    if all_results.is_empty() {
        return Err(Error::Benchmark("No benchmark results to consolidate".to_string()));
    }

    let total_files = all_results.len();
    let all_results_owned: Vec<BenchmarkResult> = runs.iter().flat_map(|r| r.iter().cloned()).collect();

    eprintln!("Aggregating results by framework");
    let by_framework = aggregate_by_framework_with_runs(&runs)?;

    eprintln!("Comparing frameworks");
    let comparison = compare_frameworks(&all_results_owned);

    eprintln!("Analyzing quality metrics");
    let quality = analyze_quality(&all_results_owned);

    let run_count = runs.len();
    let framework_count = by_framework.len();

    Ok(ConsolidatedResults {
        by_framework,
        comparison,
        quality,
        run_count,
        total_files,
        framework_count,
    })
}

/// Aggregate results by framework
pub fn aggregate_by_framework(results: &[BenchmarkResult]) -> HashMap<String, FrameworkAggregation> {
    let mut by_framework: HashMap<String, Vec<&BenchmarkResult>> = HashMap::new();
    for result in results {
        by_framework.entry(result.framework.clone()).or_default().push(result);
    }

    let mut aggregations = HashMap::new();
    for (framework, framework_results) in by_framework {
        let agg = create_framework_aggregation(&framework, &framework_results);
        aggregations.insert(framework, agg);
    }
    aggregations
}

fn aggregate_by_framework_with_runs(runs: &[Vec<BenchmarkResult>]) -> Result<HashMap<String, FrameworkAggregation>> {
    let mut by_framework_by_run: HashMap<String, Vec<Vec<&BenchmarkResult>>> = HashMap::new();

    for run_results in runs {
        for result in run_results {
            by_framework_by_run
                .entry(result.framework.clone())
                .or_insert_with(|| vec![Vec::new(); runs.len()]);
        }
    }

    for (run_index, run_results) in runs.iter().enumerate() {
        for result in run_results {
            if let Some(runs_vec) = by_framework_by_run.get_mut(&result.framework) {
                while runs_vec.len() <= run_index {
                    runs_vec.push(Vec::new());
                }
                runs_vec[run_index].push(result);
            }
        }
    }

    let mut final_aggregations = HashMap::new();

    for (framework, runs_for_framework) in by_framework_by_run {
        let mut run_aggregations = Vec::new();
        let mut all_durations = Vec::new();
        let mut all_throughputs = Vec::new();
        let mut all_memory = Vec::new();
        let mut total_files = 0;
        let mut total_successful = 0;

        for (run_index, run_results) in runs_for_framework.iter().enumerate() {
            if !run_results.is_empty() {
                let successful: Vec<_> = run_results.iter().filter(|r| r.success).collect();

                let mean_duration_ms = if !successful.is_empty() {
                    successful
                        .iter()
                        .map(|r| r.duration.as_secs_f64() * 1000.0)
                        .sum::<f64>()
                        / successful.len() as f64
                } else {
                    0.0
                };

                let mean_throughput_bps = if !successful.is_empty() {
                    successful
                        .iter()
                        .map(|r| r.metrics.throughput_bytes_per_sec)
                        .sum::<f64>()
                        / successful.len() as f64
                } else {
                    0.0
                };

                let mean_peak_memory = if !successful.is_empty() {
                    (successful
                        .iter()
                        .map(|r| r.metrics.peak_memory_bytes as f64)
                        .sum::<f64>()
                        / successful.len() as f64) as u64
                } else {
                    0
                };

                let success_rate = if !run_results.is_empty() {
                    successful.len() as f64 / run_results.len() as f64
                } else {
                    0.0
                };

                run_aggregations.push(RunAggregation {
                    run_index,
                    file_count: run_results.len(),
                    mean_duration_ms,
                    mean_throughput_bps,
                    mean_peak_memory,
                    success_rate,
                });

                all_durations.push(mean_duration_ms);
                all_throughputs.push(mean_throughput_bps);
                all_memory.push(mean_peak_memory as f64);
                total_files += run_results.len();
                total_successful += successful.len();
            }
        }

        let (mean_duration, duration_variance, duration_std_dev) = calculate_variance(&all_durations);
        let (mean_throughput, throughput_variance, throughput_std_dev) = calculate_variance(&all_throughputs);
        let (mean_memory, memory_variance, memory_std_dev) = calculate_variance(&all_memory);

        let success_rate = if total_files > 0 {
            total_successful as f64 / total_files as f64
        } else {
            0.0
        };

        let all_framework_results: Vec<_> = runs_for_framework.iter().flatten().copied().collect();
        let avg_quality = aggregate_quality_metrics(&all_framework_results);
        let by_extension = calculate_extension_stats(&all_framework_results);

        final_aggregations.insert(
            framework.clone(),
            FrameworkAggregation {
                framework: framework.clone(),
                total_files,
                runs: run_aggregations,
                mean_duration_ms: mean_duration,
                duration_variance_ms2: duration_variance,
                duration_std_dev_ms: duration_std_dev,
                mean_throughput_bps: mean_throughput,
                throughput_variance_bps2: throughput_variance,
                throughput_std_dev_bps: throughput_std_dev,
                mean_peak_memory: mean_memory as u64,
                peak_memory_variance: memory_variance,
                peak_memory_std_dev: memory_std_dev,
                success_rate,
                avg_quality,
                by_extension,
            },
        );
    }

    Ok(final_aggregations)
}

fn create_framework_aggregation(framework: &str, results: &[&BenchmarkResult]) -> FrameworkAggregation {
    let durations: Vec<f64> = results.iter().map(|r| r.duration.as_secs_f64() * 1000.0).collect();
    let (mean_duration, duration_variance, duration_std_dev) = calculate_variance(&durations);

    let throughputs: Vec<f64> = results.iter().map(|r| r.metrics.throughput_bytes_per_sec).collect();
    let (mean_throughput, throughput_variance, throughput_std_dev) = calculate_variance(&throughputs);

    let memories: Vec<f64> = results.iter().map(|r| r.metrics.peak_memory_bytes as f64).collect();
    let (mean_memory, memory_variance, memory_std_dev) = calculate_variance(&memories);

    let successful: Vec<_> = results.iter().filter(|r| r.success).collect();
    let success_rate = if !results.is_empty() {
        successful.len() as f64 / results.len() as f64
    } else {
        0.0
    };

    let avg_quality = aggregate_quality_metrics(results);
    let by_extension = calculate_extension_stats(results);

    FrameworkAggregation {
        framework: framework.to_string(),
        total_files: results.len(),
        runs: vec![],
        mean_duration_ms: mean_duration,
        duration_variance_ms2: duration_variance,
        duration_std_dev_ms: duration_std_dev,
        mean_throughput_bps: mean_throughput,
        throughput_variance_bps2: throughput_variance,
        throughput_std_dev_bps: throughput_std_dev,
        mean_peak_memory: mean_memory as u64,
        peak_memory_variance: memory_variance,
        peak_memory_std_dev: memory_std_dev,
        success_rate,
        avg_quality,
        by_extension,
    }
}

/// Compare frameworks with cross-framework analysis
pub fn compare_frameworks(results: &[BenchmarkResult]) -> CrossFrameworkComparison {
    let aggregations = aggregate_by_framework(results);

    let mut performance_ranking: Vec<_> = aggregations
        .values()
        .map(|agg| (agg.framework.clone(), agg.mean_duration_ms))
        .collect();
    performance_ranking.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    let baseline_duration = performance_ranking.first().map(|r| r.1).unwrap_or(0.0);

    let performance_ranking: Vec<_> = performance_ranking
        .into_iter()
        .enumerate()
        .map(|(idx, (framework, value))| FrameworkRanking {
            framework,
            rank: idx + 1,
            value,
            relative_performance: if baseline_duration > 0.0 {
                value / baseline_duration
            } else {
                1.0
            },
        })
        .collect();

    let mut throughput_ranking: Vec<_> = aggregations
        .values()
        .map(|agg| (agg.framework.clone(), agg.mean_throughput_bps))
        .collect();
    throughput_ranking.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let baseline_throughput = throughput_ranking.first().map(|r| r.1).unwrap_or(1.0);

    let throughput_ranking: Vec<_> = throughput_ranking
        .into_iter()
        .enumerate()
        .map(|(idx, (framework, value))| FrameworkRanking {
            framework,
            rank: idx + 1,
            value,
            relative_performance: if baseline_throughput > 0.0 {
                value / baseline_throughput
            } else {
                1.0
            },
        })
        .collect();

    let mut memory_ranking: Vec<_> = aggregations
        .values()
        .map(|agg| (agg.framework.clone(), agg.mean_peak_memory as f64))
        .collect();
    memory_ranking.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    let baseline_memory = memory_ranking.first().map(|r| r.1).unwrap_or(1.0);

    let memory_ranking: Vec<_> = memory_ranking
        .into_iter()
        .enumerate()
        .map(|(idx, (framework, value))| FrameworkRanking {
            framework,
            rank: idx + 1,
            value,
            relative_performance: if baseline_memory > 0.0 {
                value / baseline_memory
            } else {
                1.0
            },
        })
        .collect();

    let mut reliability_ranking: Vec<_> = aggregations
        .values()
        .map(|agg| (agg.framework.clone(), agg.success_rate))
        .collect();
    reliability_ranking.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let reliability_ranking: Vec<_> = reliability_ranking
        .into_iter()
        .enumerate()
        .map(|(idx, (framework, value))| FrameworkRanking {
            framework,
            rank: idx + 1,
            value: value * 100.0,
            relative_performance: if value > 0.0 { value } else { 0.0 },
        })
        .collect();

    let baseline_framework = performance_ranking.first().map(|r| r.framework.as_str());
    let baseline_agg = baseline_framework.and_then(|f| aggregations.get(f));

    let mut deltas_vs_baseline = HashMap::new();
    if let Some(baseline) = baseline_agg {
        for (framework, agg) in &aggregations {
            if framework != &baseline.framework {
                let duration_delta_ms = agg.mean_duration_ms - baseline.mean_duration_ms;
                let duration_delta_percent = if baseline.mean_duration_ms > 0.0 {
                    (duration_delta_ms / baseline.mean_duration_ms) * 100.0
                } else {
                    0.0
                };

                let throughput_delta_bps = agg.mean_throughput_bps - baseline.mean_throughput_bps;
                let throughput_delta_percent = if baseline.mean_throughput_bps > 0.0 {
                    (throughput_delta_bps / baseline.mean_throughput_bps) * 100.0
                } else {
                    0.0
                };

                let memory_delta_bytes = agg.mean_peak_memory as i64 - baseline.mean_peak_memory as i64;
                let memory_delta_percent = if baseline.mean_peak_memory > 0 {
                    (memory_delta_bytes as f64 / baseline.mean_peak_memory as f64) * 100.0
                } else {
                    0.0
                };

                deltas_vs_baseline.insert(
                    framework.clone(),
                    PerformanceDelta {
                        framework: framework.clone(),
                        duration_delta_ms,
                        duration_delta_percent,
                        throughput_delta_bps,
                        throughput_delta_percent,
                        memory_delta_bytes,
                        memory_delta_percent,
                    },
                );
            }
        }
    }

    CrossFrameworkComparison {
        performance_ranking,
        throughput_ranking,
        memory_ranking,
        reliability_ranking,
        deltas_vs_baseline,
    }
}

/// Analyze quality metrics across frameworks
pub fn analyze_quality(results: &[BenchmarkResult]) -> QualityAnalysis {
    let mut by_framework: HashMap<String, Vec<&BenchmarkResult>> = HashMap::new();

    for result in results {
        by_framework.entry(result.framework.clone()).or_default().push(result);
    }

    let mut by_framework_quality = HashMap::new();
    let mut quality_ranking_data = Vec::new();

    for (framework, framework_results) in by_framework {
        let with_metrics: Vec<_> = framework_results
            .iter()
            .filter(|r| r.quality.is_some())
            .copied()
            .collect();

        if !with_metrics.is_empty() {
            let f1_texts: Vec<f64> = with_metrics
                .iter()
                .filter_map(|r| r.quality.as_ref().map(|q| q.f1_score_text))
                .collect();
            let f1_numerics: Vec<f64> = with_metrics
                .iter()
                .filter_map(|r| r.quality.as_ref().map(|q| q.f1_score_numeric))
                .collect();
            let f1_layouts: Vec<f64> = with_metrics
                .iter()
                .filter_map(|r| r.quality.as_ref().map(|q| q.f1_score_layout))
                .collect();
            let quality_scores: Vec<f64> = with_metrics
                .iter()
                .filter_map(|r| r.quality.as_ref().map(|q| q.quality_score))
                .collect();

            let mean_f1_text = if !f1_texts.is_empty() {
                f1_texts.iter().sum::<f64>() / f1_texts.len() as f64
            } else {
                0.0
            };
            let mean_f1_numeric = if !f1_numerics.is_empty() {
                f1_numerics.iter().sum::<f64>() / f1_numerics.len() as f64
            } else {
                0.0
            };
            let mean_f1_layout = if !f1_layouts.is_empty() {
                f1_layouts.iter().sum::<f64>() / f1_layouts.len() as f64
            } else {
                0.0
            };

            let (mean_quality, _, quality_std_dev) = calculate_variance(&quality_scores);
            let (_, _, f1_text_std_dev) = calculate_variance(&f1_texts);

            let framework_quality = FrameworkQuality {
                framework: framework.clone(),
                files_with_metrics: with_metrics.len(),
                mean_f1_text,
                f1_text_std_dev,
                mean_f1_numeric,
                mean_f1_layout,
                mean_quality_score: mean_quality,
                quality_score_std_dev: quality_std_dev,
            };

            quality_ranking_data.push((framework.clone(), mean_quality, quality_std_dev));
            by_framework_quality.insert(framework, framework_quality);
        }
    }

    quality_ranking_data.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let quality_ranking: Vec<_> = quality_ranking_data
        .into_iter()
        .enumerate()
        .map(|(idx, (framework, score, consistency))| QualityRanking {
            framework,
            rank: idx + 1,
            mean_quality_score: score,
            consistency_score: (1.0 - consistency).clamp(0.0, 1.0),
        })
        .collect();

    let mut by_file: HashMap<String, Vec<bool>> = HashMap::new();
    for result in results {
        let file_key = result.file_path.to_string_lossy().to_string();
        by_file.entry(file_key).or_default().push(result.success);
    }

    let total_files = by_file.len();
    let consensus_success = by_file
        .values()
        .filter(|successes| successes.iter().all(|s| *s))
        .count();
    let consensus_success_rate = if total_files > 0 {
        consensus_success as f64 / total_files as f64
    } else {
        0.0
    };

    let mut framework_results: HashMap<String, (usize, usize)> = HashMap::new();
    for result in results {
        let (success_count, total_count) = framework_results.entry(result.framework.clone()).or_insert((0, 0));
        *total_count += 1;
        if result.success {
            *success_count += 1;
        }
    }

    let perfect_frameworks: Vec<String> = framework_results
        .into_iter()
        .filter(|(_, (success, total))| success == total && total > &0)
        .map(|(f, _)| f)
        .collect();

    let quality_degradation_concerns = by_framework_quality
        .values()
        .filter(|q| q.quality_score_std_dev > 0.2)
        .map(|q| q.framework.clone())
        .collect();

    let reliability = QualityReliability {
        consensus_success_rate,
        perfect_frameworks,
        quality_degradation_concerns,
    };

    QualityAnalysis {
        by_framework: by_framework_quality,
        quality_ranking,
        reliability,
    }
}

/// Write consolidated results to JSON
pub fn write_consolidated_json(results: &ConsolidatedResults, path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(Error::Io)?;
    }

    let json = serde_json::to_string_pretty(results)
        .map_err(|e| Error::Benchmark(format!("Failed to serialize results: {}", e)))?;

    fs::write(path, json).map_err(Error::Io)?;
    Ok(())
}

fn calculate_variance(values: &[f64]) -> (f64, f64, f64) {
    if values.is_empty() {
        return (0.0, 0.0, 0.0);
    }
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64;
    let std_dev = variance.sqrt();
    (mean, variance, std_dev)
}

fn aggregate_quality_metrics(results: &[&BenchmarkResult]) -> Option<QualityMetrics> {
    let with_quality: Vec<_> = results.iter().filter_map(|r| r.quality.as_ref()).collect();
    if with_quality.is_empty() {
        return None;
    }

    let f1_text = with_quality.iter().map(|q| q.f1_score_text).sum::<f64>() / with_quality.len() as f64;
    let f1_numeric = with_quality.iter().map(|q| q.f1_score_numeric).sum::<f64>() / with_quality.len() as f64;
    let f1_layout = with_quality.iter().map(|q| q.f1_score_layout).sum::<f64>() / with_quality.len() as f64;
    let quality_score = with_quality.iter().map(|q| q.quality_score).sum::<f64>() / with_quality.len() as f64;

    Some(QualityMetrics {
        f1_score_text: f1_text,
        f1_score_numeric: f1_numeric,
        f1_score_layout: f1_layout,
        quality_score,
    })
}

fn calculate_extension_stats(results: &[&BenchmarkResult]) -> HashMap<String, ExtensionStats> {
    let mut by_ext: HashMap<String, Vec<&BenchmarkResult>> = HashMap::new();
    for result in results {
        by_ext.entry(result.file_extension.clone()).or_default().push(result);
    }

    let mut stats = HashMap::new();
    for (ext, ext_results) in by_ext {
        let successful: Vec<_> = ext_results.iter().filter(|r| r.success).collect();
        let durations: Vec<f64> = ext_results.iter().map(|r| r.duration.as_secs_f64() * 1000.0).collect();

        let (mean_duration, _, _) = calculate_variance(&durations);
        let p95_duration = calculate_percentile(&durations, 0.95);

        let mean_throughput = if !successful.is_empty() {
            successful
                .iter()
                .map(|r| r.metrics.throughput_bytes_per_sec)
                .sum::<f64>()
                / successful.len() as f64
        } else {
            0.0
        };

        let success_rate = if !ext_results.is_empty() {
            successful.len() as f64 / ext_results.len() as f64
        } else {
            0.0
        };

        stats.insert(
            ext.clone(),
            ExtensionStats {
                extension: ext,
                file_count: ext_results.len(),
                mean_duration_ms: mean_duration,
                p95_duration_ms: p95_duration,
                mean_throughput_bps: mean_throughput,
                success_rate,
            },
        );
    }
    stats
}

fn calculate_percentile(values: &[f64], percentile: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let idx = ((sorted.len() as f64 * percentile) as usize).min(sorted.len() - 1);
    sorted[idx]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{FrameworkCapabilities, PerformanceMetrics};
    use std::path::PathBuf;
    use std::time::Duration;

    fn create_test_result(framework: &str, file: &str, success: bool, duration_ms: u64) -> BenchmarkResult {
        BenchmarkResult {
            framework: framework.to_string(),
            file_path: PathBuf::from(file),
            file_size: 1024,
            success,
            error_message: None,
            duration: Duration::from_millis(duration_ms),
            extraction_duration: None,
            subprocess_overhead: None,
            metrics: PerformanceMetrics {
                peak_memory_bytes: 10_000_000,
                avg_cpu_percent: 50.0,
                throughput_bytes_per_sec: 1024.0,
                p50_memory_bytes: 8_000_000,
                p95_memory_bytes: 9_500_000,
                p99_memory_bytes: 9_900_000,
            },
            quality: None,
            iterations: vec![],
            statistics: None,
            cold_start_duration: None,
            file_extension: "pdf".to_string(),
            framework_capabilities: FrameworkCapabilities::default(),
            pdf_metadata: None,
        }
    }

    #[test]
    fn test_aggregate_by_framework() {
        let results = vec![
            create_test_result("Framework A", "file1.pdf", true, 100),
            create_test_result("Framework A", "file2.pdf", true, 200),
            create_test_result("Framework B", "file1.pdf", true, 150),
        ];
        let agg = aggregate_by_framework(&results);
        assert_eq!(agg.len(), 2);
        assert!(agg.contains_key("Framework A"));
        assert!(agg.contains_key("Framework B"));
        assert_eq!(agg["Framework A"].total_files, 2);
        assert_eq!(agg["Framework A"].success_rate, 1.0);
    }

    #[test]
    fn test_calculate_variance() {
        let values = vec![1.0, 2.0, 3.0];
        let (mean, variance, std_dev) = calculate_variance(&values);
        assert!((mean - 2.0).abs() < 0.01);
        assert!(variance > 0.0);
        assert!(std_dev > 0.0);
    }

    #[test]
    fn test_compare_frameworks() {
        let results = vec![
            create_test_result("Framework A", "file1.pdf", true, 100),
            create_test_result("Framework A", "file2.pdf", true, 100),
            create_test_result("Framework B", "file1.pdf", true, 200),
            create_test_result("Framework B", "file2.pdf", true, 200),
        ];
        let comparison = compare_frameworks(&results);
        assert_eq!(comparison.performance_ranking.len(), 2);
        assert_eq!(comparison.performance_ranking[0].framework, "Framework A");
        assert_eq!(comparison.performance_ranking[0].rank, 1);
    }
}
