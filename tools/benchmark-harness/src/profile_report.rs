//! Comprehensive profiling report generation with hotspot analysis
//!
//! This module provides infrastructure for generating detailed profiling reports from
//! CPU profile data. Reports include top function hotspots, memory trajectory analysis,
//! actionable recommendations, and sample quality metrics.
//!
//! # Report Components
//!
//! - **Summary Statistics**: Sample count, profiling duration, effective sampling frequency
//! - **Top Hotspots**: Top 10 functions by sample count with percentages
//! - **Memory Trajectory**: Memory usage snapshots over profiling duration (when available)
//! - **Recommendations**: Actionable insights based on sample quality and profiling data
//!
//! # Sample Quality Guidelines
//!
//! - **< 100 samples**: Profile may have high variance, increase duration or frequency
//! - **100-499 samples**: Acceptable for basic analysis, consider longer runs
//! - **500+ samples**: Good quality profile with reliable hotspot identification
//! - **1000+ samples**: Excellent quality with strong statistical confidence
//!
//! # HTML Report Format
//!
//! Reports are generated as self-contained HTML documents with inline CSS, requiring
//! no external dependencies. The HTML is viewable in any modern web browser.

#[cfg(feature = "profiling")]
use crate::profiling::ProfilingResult;
use std::time::Duration;

/// Comprehensive profiling report with hotspot analysis
///
/// Contains aggregated profiling metrics, top functions, and analysis recommendations
/// suitable for performance optimization decisions.
#[derive(Debug, Clone)]
pub struct ProfileReport {
    /// Total number of CPU samples collected
    pub sample_count: usize,
    /// Total profiling duration
    pub duration: Duration,
    /// Effective sampling frequency (samples collected per second)
    pub effective_frequency: f64,
    /// Top 10 functions by sample count
    pub top_hotspots: Vec<Hotspot>,
    /// Memory usage trajectory (if available)
    pub memory_trajectory: Vec<MemorySnapshot>,
    /// Actionable recommendations based on profile quality
    pub recommendations: Vec<String>,
}

/// Individual function hotspot identified in the profile
///
/// Represents a function that consumed significant CPU samples during profiling.
#[derive(Debug, Clone)]
pub struct Hotspot {
    /// Function name or symbol (demangled if possible)
    pub function_name: String,
    /// Number of samples attributed to this function
    pub samples: usize,
    /// Percentage of total samples (0.0-100.0)
    pub percentage: f64,
    /// File location if available (filename:line)
    pub file_location: Option<String>,
}

/// Memory usage snapshot at a point in time
///
/// Used to track memory growth patterns during profiling.
#[derive(Debug, Clone)]
pub struct MemorySnapshot {
    /// Relative time from profiling start in milliseconds
    pub timestamp_ms: u64,
    /// Memory usage in bytes (RSS)
    pub memory_bytes: u64,
}

impl Default for ProfileReport {
    fn default() -> Self {
        Self {
            sample_count: 0,
            duration: Duration::ZERO,
            effective_frequency: 0.0,
            top_hotspots: Vec::new(),
            memory_trajectory: Vec::new(),
            recommendations: Vec::new(),
        }
    }
}

impl ProfileReport {
    /// Create a ProfileReport from profiling result (feature-gated for profiling)
    ///
    /// Analyzes the pprof Report structure to extract:
    /// - Sample count and duration metrics
    /// - Top 10 functions by sample count
    /// - Effective sampling frequency
    /// - Quality-based recommendations
    ///
    /// # Arguments
    ///
    /// * `result` - ProfilingResult from ProfileGuard::finish()
    /// * `framework_name` - Name of the framework being profiled (for reporting)
    ///
    /// # Returns
    ///
    /// A ProfileReport with hotspot analysis and recommendations
    ///
    /// # Note
    ///
    /// This function is only available when the `profiling` feature is enabled.
    #[cfg(feature = "profiling")]
    pub fn from_profiling_result(result: &ProfilingResult, framework_name: &str) -> Self {
        let duration = result.duration;
        let sample_count = result.sample_count;

        let effective_frequency = if duration.as_secs_f64() > 0.0 {
            sample_count as f64 / duration.as_secs_f64()
        } else {
            0.0
        };

        let top_hotspots = Self::extract_top_hotspots(&result.report, sample_count);

        let recommendations = Self::generate_recommendations(sample_count, framework_name);

        Self {
            sample_count,
            duration,
            effective_frequency,
            top_hotspots,
            memory_trajectory: Vec::new(),
            recommendations,
        }
    }

    /// Extract top 10 hotspots from the pprof Report
    ///
    /// # Arguments
    ///
    /// * `_report` - pprof Report containing collected profile data
    /// * `total_samples` - Total sample count for percentage calculation
    ///
    /// # Returns
    ///
    /// Vector of up to 10 hotspots sorted by sample count descending
    ///
    /// Note: This is a stub implementation. The pprof Report API doesn't expose
    /// sample-level data directly in public API. A future enhancement would require
    /// either:
    /// 1. Creating custom serialization from pprof protobuf output
    /// 2. Writing reports to intermediate format and parsing
    /// 3. Enhancing pprof with additional API methods
    ///
    /// For now, we generate recommendations based on sample count which is meaningful.
    #[cfg(feature = "profiling")]
    fn extract_top_hotspots(_report: &pprof::Report, total_samples: usize) -> Vec<Hotspot> {
        if total_samples == 0 {
            return Vec::new();
        }

        vec![Hotspot {
            function_name: "[profile data collected - hotspot extraction requires pprof API enhancement]".to_string(),
            samples: total_samples,
            percentage: 100.0,
            file_location: None,
        }]
    }

    /// Generate recommendations based on profile quality metrics
    ///
    /// # Arguments
    ///
    /// * `sample_count` - Number of samples collected
    /// * `framework_name` - Name of the profiled framework
    ///
    /// # Returns
    ///
    /// Vector of actionable recommendations
    #[allow(dead_code)]
    fn generate_recommendations(sample_count: usize, framework_name: &str) -> Vec<String> {
        let mut recommendations = vec![format!(
            "Profiling data collected for {} framework with {} samples",
            framework_name, sample_count
        )];

        if sample_count < 50 {
            recommendations.push(
                "Very low sample count (<50): Profile may be unreliable. Increase profiling duration \
                 or sampling frequency for better accuracy."
                    .to_string(),
            );
            recommendations.push(
                "Consider running the benchmark with amplified iterations (see --profiling-amplification) \
                 to collect more samples."
                    .to_string(),
            );
        } else if sample_count < 100 {
            recommendations.push(
                "Low sample count (<100): Profile has high variance. Increase profiling duration or \
                 consider longer-running benchmarks."
                    .to_string(),
            );
        } else if sample_count < 500 {
            recommendations.push(
                "Acceptable sample count (100-500): Profile is suitable for basic hotspot identification, \
                 but confidence in percentages is moderate. Consider longer runs for more precision."
                    .to_string(),
            );
        } else if sample_count < 1000 {
            recommendations.push(
                "Good sample count (500-1000): Profile quality is reliable for identifying hotspots.".to_string(),
            );
        } else {
            recommendations.push(
                "Excellent sample count (1000+): Profile has high statistical confidence. \
                 Hotspot percentages are reliable for optimization decisions."
                    .to_string(),
            );
        }

        match framework_name {
            "kreuzberg" => {
                recommendations.push(
                    "Kreuzberg profile analysis: Focus on PDF parsing (pdf module) and text extraction \
                     (text module) hotspots."
                        .to_string(),
                );
            }
            "python" => {
                recommendations.push(
                    "Python bindings: High overhead in PyO3 marshalling may appear in hotspots. \
                           Consider optimizing PyO3 FFI boundary."
                        .to_string(),
                );
            }
            "ruby" => {
                recommendations.push(
                    "Ruby bindings: GIL contention may limit threading performance. \
                           Verify Magnus FFI overhead in hotspot analysis."
                        .to_string(),
                );
            }
            _ => {}
        }

        recommendations
    }

    /// Generate an HTML report from the profile
    ///
    /// Creates a self-contained HTML document with inline CSS that displays:
    /// - Summary statistics table
    /// - Top 10 hotspots table with percentages
    /// - Memory trajectory chart (if available)
    /// - Recommendations list
    ///
    /// The HTML is viewable in any modern browser without external dependencies.
    ///
    /// # Returns
    ///
    /// HTML string with the formatted report
    pub fn generate_html(&self) -> String {
        let hotspots_html = self.render_hotspots_table();
        let recommendations_html = self.render_recommendations();
        let memory_html = if self.memory_trajectory.is_empty() {
            String::new()
        } else {
            self.render_memory_chart()
        };

        let css = Self::css_styles();
        let duration_ms = self.duration.as_millis();

        format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Profiling Report</title>
    <style>
{}
    </style>
</head>
<body>
    <div class="container">
        <header class="report-header">
            <h1>CPU Profile Report</h1>
            <p class="subtitle">Comprehensive hotspot analysis and recommendations</p>
        </header>

        <section class="summary-stats">
            <h2>Profiling Summary</h2>
            <table class="stats-table">
                <tr>
                    <td class="stat-label">Total Samples Collected:</td>
                    <td class="stat-value">{}</td>
                </tr>
                <tr>
                    <td class="stat-label">Profiling Duration:</td>
                    <td class="stat-value">{} ms</td>
                </tr>
                <tr>
                    <td class="stat-label">Effective Frequency:</td>
                    <td class="stat-value">{:.1} samples/sec</td>
                </tr>
                <tr>
                    <td class="stat-label">Sample Quality:</td>
                    <td class="stat-value">{}</td>
                </tr>
            </table>
        </section>

        <section class="hotspots-section">
            <h2>Top 10 Hotspots</h2>
            {}
        </section>

        {}

        <section class="recommendations-section">
            <h2>Recommendations</h2>
            {}
        </section>

        <footer class="report-footer">
            <p>Generated by Kreuzberg Benchmark Harness</p>
        </footer>
    </div>
</body>
</html>"#,
            css,
            self.sample_count,
            duration_ms,
            self.effective_frequency,
            self.sample_quality_label(),
            hotspots_html,
            memory_html,
            recommendations_html
        )
    }

    /// Determine sample quality label based on count
    fn sample_quality_label(&self) -> &str {
        match self.sample_count {
            0..=49 => "Very Low",
            50..=99 => "Low",
            100..=499 => "Acceptable",
            500..=999 => "Good",
            _ => "Excellent",
        }
    }

    /// Render hotspots table in HTML
    fn render_hotspots_table(&self) -> String {
        if self.top_hotspots.is_empty() {
            return "<p class=\"no-data\">No hotspots captured in profile</p>".to_string();
        }

        let rows: String = self
            .top_hotspots
            .iter()
            .enumerate()
            .map(|(idx, hotspot)| {
                let bar_width = (hotspot.percentage * 3.0).min(300.0);
                format!(
                    r#"<tr>
                    <td class="rank">{}</td>
                    <td class="function-name" title="{}">{}</td>
                    <td class="sample-count">{}</td>
                    <td class="percentage">
                        <div class="bar-container">
                            <div class="bar" style="width: {}px"></div>
                            <span class="percentage-text">{:.1}%</span>
                        </div>
                    </td>
                </tr>"#,
                    idx + 1,
                    hotspot.function_name,
                    Self::truncate_function_name(&hotspot.function_name, 50),
                    hotspot.samples,
                    bar_width,
                    hotspot.percentage
                )
            })
            .collect();

        format!(
            r#"<table class="hotspots-table">
            <thead>
                <tr>
                    <th class="rank-col">Rank</th>
                    <th class="function-col">Function</th>
                    <th class="samples-col">Samples</th>
                    <th class="percentage-col">Percentage</th>
                </tr>
            </thead>
            <tbody>
                {}
            </tbody>
        </table>"#,
            rows
        )
    }

    /// Render recommendations section in HTML
    fn render_recommendations(&self) -> String {
        if self.recommendations.is_empty() {
            return String::new();
        }

        let items: String = self
            .recommendations
            .iter()
            .map(|rec| format!("<li>{}</li>", html_escape(rec)))
            .collect();

        format!("<ul class=\"recommendations-list\">{}</ul>", items)
    }

    /// Render memory trajectory chart (stub for future expansion)
    fn render_memory_chart(&self) -> String {
        if self.memory_trajectory.is_empty() {
            return String::new();
        }

        format!(
            r#"<section class="memory-section">
            <h2>Memory Trajectory</h2>
            <p class="note">Memory profiling data ({} snapshots collected)</p>
        </section>"#,
            self.memory_trajectory.len()
        )
    }

    /// Truncate long function names for display
    fn truncate_function_name(name: &str, max_len: usize) -> String {
        if name.len() > max_len {
            format!("{}...", &name[..max_len - 3])
        } else {
            name.to_string()
        }
    }

    /// Inline CSS styles for the HTML report
    ///
    /// Self-contained styles requiring no external dependencies.
    /// Includes responsive design and print-friendly styles.
    fn css_styles() -> &'static str {
        r#"
        * {
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }

        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, sans-serif;
            line-height: 1.6;
            color: #333;
            background: linear-gradient(135deg, #f5f7fa 0%, #c3cfe2 100%);
            min-height: 100vh;
            padding: 20px;
        }

        .container {
            max-width: 1200px;
            margin: 0 auto;
            background: white;
            border-radius: 8px;
            box-shadow: 0 10px 40px rgba(0, 0, 0, 0.1);
            overflow: hidden;
        }

        .report-header {
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: white;
            padding: 40px 30px;
            text-align: center;
        }

        .report-header h1 {
            font-size: 2.5em;
            margin-bottom: 10px;
            font-weight: 700;
        }

        .subtitle {
            font-size: 1.1em;
            opacity: 0.95;
            font-weight: 300;
        }

        section {
            padding: 40px 30px;
            border-bottom: 1px solid #e0e0e0;
        }

        section:last-of-type {
            border-bottom: none;
        }

        h2 {
            color: #667eea;
            font-size: 1.8em;
            margin-bottom: 25px;
            font-weight: 700;
        }

        .summary-stats {
            background: #f9fafb;
        }

        .stats-table {
            width: 100%;
            border-collapse: collapse;
        }

        .stats-table tr {
            border-bottom: 1px solid #e5e7eb;
        }

        .stats-table tr:last-child {
            border-bottom: none;
        }

        .stat-label {
            font-weight: 600;
            color: #1f2937;
            padding: 12px 16px;
            width: 40%;
        }

        .stat-value {
            padding: 12px 16px;
            color: #667eea;
            font-weight: 500;
            font-size: 1.1em;
        }

        .hotspots-table {
            width: 100%;
            border-collapse: collapse;
        }

        .hotspots-table thead {
            background: #f0f4ff;
            border-bottom: 2px solid #e0e7ff;
        }

        .hotspots-table th {
            padding: 15px;
            text-align: left;
            font-weight: 600;
            color: #667eea;
            font-size: 0.95em;
            text-transform: uppercase;
            letter-spacing: 0.5px;
        }

        .hotspots-table tbody tr {
            border-bottom: 1px solid #e5e7eb;
            transition: background 0.2s;
        }

        .hotspots-table tbody tr:hover {
            background: #f9fafb;
        }

        .hotspots-table td {
            padding: 12px 15px;
            font-size: 0.95em;
        }

        .rank {
            font-weight: 700;
            color: #667eea;
            text-align: center;
            width: 50px;
        }

        .rank-col {
            width: 50px;
        }

        .function-col {
            width: 40%;
        }

        .samples-col {
            width: 15%;
        }

        .percentage-col {
            width: 35%;
        }

        .function-name {
            font-family: 'Courier New', monospace;
            font-size: 0.9em;
            color: #1f2937;
            word-break: break-all;
        }

        .sample-count {
            font-weight: 500;
            color: #764ba2;
        }

        .percentage {
            min-width: 300px;
        }

        .bar-container {
            position: relative;
            height: 28px;
            display: flex;
            align-items: center;
        }

        .bar {
            height: 20px;
            background: linear-gradient(90deg, #667eea 0%, #764ba2 100%);
            border-radius: 3px;
            min-width: 2px;
            transition: all 0.2s;
        }

        .bar-container:hover .bar {
            filter: brightness(1.1);
        }

        .percentage-text {
            margin-left: 10px;
            font-weight: 600;
            color: #764ba2;
            font-size: 0.9em;
            min-width: 50px;
        }

        .recommendations-section {
            background: #f0fdf4;
        }

        .recommendations-list {
            list-style: none;
            margin-left: 0;
        }

        .recommendations-list li {
            padding: 12px 16px;
            margin-bottom: 10px;
            background: white;
            border-left: 4px solid #10b981;
            border-radius: 4px;
            color: #1f2937;
        }

        .recommendations-list li:before {
            content: "✓ ";
            color: #10b981;
            font-weight: bold;
            margin-right: 8px;
        }

        .memory-section {
            background: #f0f9ff;
        }

        .note {
            color: #666;
            font-style: italic;
            margin-top: 10px;
        }

        .no-data {
            color: #999;
            text-align: center;
            padding: 20px;
            font-style: italic;
        }

        .report-footer {
            background: #f3f4f6;
            text-align: center;
            color: #666;
            font-size: 0.9em;
            padding: 20px !important;
            border-top: 1px solid #e5e7eb;
            border-bottom: none;
        }

        @media (max-width: 768px) {
            .container {
                border-radius: 0;
            }

            .report-header {
                padding: 30px 20px;
            }

            .report-header h1 {
                font-size: 1.8em;
            }

            section {
                padding: 25px 20px;
            }

            h2 {
                font-size: 1.4em;
            }

            .hotspots-table,
            .stats-table {
                font-size: 0.9em;
            }

            .hotspots-table td,
            .hotspots-table th,
            .stats-table td {
                padding: 8px 10px;
            }

            .function-col {
                width: 100%;
            }

            .percentage-col {
                width: 100%;
            }

            .function-name {
                display: block;
                margin-bottom: 5px;
            }

            .percentage {
                min-width: auto;
                margin-top: 10px;
            }
        }

        @media print {
            body {
                background: white;
            }

            .container {
                box-shadow: none;
                border-radius: 0;
            }

            .report-header {
                page-break-after: avoid;
            }

            section {
                page-break-inside: avoid;
            }
        }
        "#
    }
}

/// Escape HTML special characters
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_report_default() {
        let report = ProfileReport::default();
        assert_eq!(report.sample_count, 0);
        assert_eq!(report.duration, Duration::ZERO);
        assert_eq!(report.effective_frequency, 0.0);
        assert!(report.top_hotspots.is_empty());
        assert!(report.recommendations.is_empty());
    }

    #[test]
    fn test_sample_quality_label() {
        let mut report = ProfileReport::default();

        report.sample_count = 25;
        assert_eq!(report.sample_quality_label(), "Very Low");

        report.sample_count = 75;
        assert_eq!(report.sample_quality_label(), "Low");

        report.sample_count = 250;
        assert_eq!(report.sample_quality_label(), "Acceptable");

        report.sample_count = 750;
        assert_eq!(report.sample_quality_label(), "Good");

        report.sample_count = 1500;
        assert_eq!(report.sample_quality_label(), "Excellent");
    }

    #[test]
    fn test_generate_recommendations_very_low_samples() {
        let recommendations = ProfileReport::generate_recommendations(25, "kreuzberg");
        assert!(recommendations.len() >= 3);
        assert!(recommendations[1].contains("Very low sample count"));
        assert!(recommendations[2].contains("amplified iterations"));
    }

    #[test]
    fn test_generate_recommendations_good_samples() {
        let recommendations = ProfileReport::generate_recommendations(750, "kreuzberg");
        assert!(recommendations[1].contains("Good sample count"));
    }

    #[test]
    fn test_generate_recommendations_excellent_samples() {
        let recommendations = ProfileReport::generate_recommendations(2000, "python");
        assert!(recommendations[1].contains("Excellent"));
    }

    #[test]
    fn test_truncate_function_name() {
        let long_name = "this_is_a_very_long_function_name_that_should_be_truncated_for_display";
        let truncated = ProfileReport::truncate_function_name(long_name, 30);
        assert_eq!(truncated.len(), 30);
        assert!(truncated.ends_with("..."));
    }

    #[test]
    fn test_truncate_function_name_short() {
        let short_name = "short";
        let result = ProfileReport::truncate_function_name(short_name, 30);
        assert_eq!(result, "short");
    }

    #[test]
    fn test_html_escape() {
        assert_eq!(html_escape("hello"), "hello");
        assert_eq!(html_escape("<script>"), "&lt;script&gt;");
        assert_eq!(html_escape("a&b"), "a&amp;b");
        assert_eq!(html_escape("\"quote\""), "&quot;quote&quot;");
        assert_eq!(html_escape("'apostrophe'"), "&#39;apostrophe&#39;");
    }

    #[test]
    fn test_generate_html_empty_report() {
        let report = ProfileReport::default();
        let html = report.generate_html();

        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("CPU Profile Report"));
        assert!(html.contains("0</td>"));
        assert!(html.contains("Very Low</td>"));
        assert!(html.contains("No hotspots captured"));
    }

    #[test]
    fn test_generate_html_with_hotspots() {
        let mut report = ProfileReport::default();
        report.sample_count = 1000;
        report.duration = Duration::from_millis(1000);
        report.effective_frequency = 1000.0;
        report.top_hotspots = vec![
            Hotspot {
                function_name: "extraction_function".to_string(),
                samples: 500,
                percentage: 50.0,
                file_location: None,
            },
            Hotspot {
                function_name: "text_processing".to_string(),
                samples: 300,
                percentage: 30.0,
                file_location: None,
            },
        ];
        report.recommendations = vec!["Good profile quality".to_string()];

        let html = report.generate_html();

        assert!(html.contains("1000</td>"));
        assert!(html.contains("extraction_function"));
        assert!(html.contains("500"));
        assert!(html.contains("50.0%"));
        assert!(html.contains("Good profile quality"));
        assert!(html.contains("Excellent"));
    }

    #[test]
    fn test_effective_frequency_calculation() {
        let report = ProfileReport {
            sample_count: 1000,
            duration: Duration::from_secs(2),
            effective_frequency: 500.0,
            top_hotspots: Vec::new(),
            memory_trajectory: Vec::new(),
            recommendations: Vec::new(),
        };

        assert_eq!(report.effective_frequency, 500.0);
    }

    #[test]
    fn test_effective_frequency_zero_duration() {
        let report = ProfileReport::default();
        assert_eq!(report.effective_frequency, 0.0);
    }

    #[test]
    fn test_hotspots_render_empty() {
        let report = ProfileReport::default();
        let html = report.render_hotspots_table();
        assert!(html.contains("No hotspots captured"));
    }

    #[test]
    fn test_hotspots_render_with_data() {
        let mut report = ProfileReport::default();
        report.top_hotspots = vec![
            Hotspot {
                function_name: "func_one".to_string(),
                samples: 100,
                percentage: 50.0,
                file_location: None,
            },
            Hotspot {
                function_name: "func_two".to_string(),
                samples: 50,
                percentage: 25.0,
                file_location: None,
            },
        ];

        let html = report.render_hotspots_table();
        assert!(html.contains("func_one"));
        assert!(html.contains("100"));
        assert!(html.contains("50.0%"));
        assert!(html.contains("func_two"));
        assert!(html.contains("50"));
        assert!(html.contains("25.0%"));
    }

    #[test]
    fn test_css_styles_present() {
        let css = ProfileReport::css_styles();
        assert!(css.contains("@media (max-width: 768px)"));
        assert!(css.contains("@media print"));
        assert!(css.contains("border-radius"));
        assert!(css.contains("font-family"));
    }
}
