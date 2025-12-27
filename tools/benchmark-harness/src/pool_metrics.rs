//! Pool metrics collection and reporting
//!
//! This module provides infrastructure for collecting and reporting metrics
//! from pool operations during document extraction, helping to identify
//! allocation patterns and pool efficiency.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Aggregate metrics for a single file extraction
#[derive(Debug, Clone)]
pub struct FilePoolMetrics {
    pub file_name: String,
    pub mime_type: String,
    pub file_size: usize,
    pub string_pool_acquires: usize,
    pub string_pool_reuses: usize,
    pub string_pool_hit_rate: f64,
}

/// Aggregate metrics for all extractions
#[derive(Debug, Clone)]
pub struct PoolMetricsReport {
    pub total_files: usize,
    pub files: Vec<FilePoolMetrics>,
    pub average_hit_rate: f64,
    pub min_hit_rate: f64,
    pub max_hit_rate: f64,
}

impl PoolMetricsReport {
    /// Calculate overall statistics from individual file metrics
    pub fn from_files(files: Vec<FilePoolMetrics>) -> Self {
        let total_files = files.len();

        let hit_rates: Vec<f64> = files.iter().map(|f| f.string_pool_hit_rate).collect();
        let average_hit_rate = if !hit_rates.is_empty() {
            hit_rates.iter().sum::<f64>() / hit_rates.len() as f64
        } else {
            0.0
        };

        let min_hit_rate = hit_rates.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_hit_rate = hit_rates.iter().cloned().fold(0.0, f64::max);

        PoolMetricsReport {
            total_files,
            files,
            average_hit_rate,
            min_hit_rate,
            max_hit_rate,
        }
    }

    /// Serialize to JSON format
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&serde_json::json!({
            "metadata": {
                "version": "1.0",
                "timestamp": chrono::Local::now().to_rfc3339(),
            },
            "summary": {
                "total_files": self.total_files,
                "average_hit_rate": self.average_hit_rate,
                "min_hit_rate": self.min_hit_rate,
                "max_hit_rate": self.max_hit_rate,
            },
            "files": self.files.iter().map(|f| serde_json::json!({
                "file_name": f.file_name,
                "mime_type": f.mime_type,
                "file_size": f.file_size,
                "string_pool": {
                    "total_acquires": f.string_pool_acquires,
                    "total_reuses": f.string_pool_reuses,
                    "hit_rate_percent": f.string_pool_hit_rate,
                }
            })).collect::<Vec<_>>(),
        }))
    }

    /// Write report to file
    pub fn write_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error>> {
        let json = self.to_json()?;
        fs::write(path, json)?;
        Ok(())
    }

    /// Print human-readable summary
    pub fn print_summary(&self) {
        println!("\n=== Pool Metrics Report ===");
        println!("Total files analyzed: {}", self.total_files);
        println!(
            "Hit rate (avg): {:.2}% (min: {:.2}%, max: {:.2}%)",
            self.average_hit_rate, self.min_hit_rate, self.max_hit_rate
        );

        let mut ranges = HashMap::new();
        for file in &self.files {
            let range = if file.string_pool_hit_rate < 25.0 {
                "0-25%"
            } else if file.string_pool_hit_rate < 50.0 {
                "25-50%"
            } else if file.string_pool_hit_rate < 75.0 {
                "50-75%"
            } else if file.string_pool_hit_rate < 90.0 {
                "75-90%"
            } else {
                "90%+"
            };
            *ranges.entry(range).or_insert(0) += 1;
        }

        println!("\nHit rate distribution:");
        for range in &["0-25%", "25-50%", "50-75%", "75-90%", "90%+"] {
            let count = ranges.get(range).unwrap_or(&0);
            println!("  {}: {} files", range, count);
        }

        println!("\nBottom 5 performers (lowest hit rate):");
        let mut sorted = self.files.clone();
        sorted.sort_by(|a, b| a.string_pool_hit_rate.partial_cmp(&b.string_pool_hit_rate).unwrap());
        for file in sorted.iter().take(5) {
            println!(
                "  {} ({:.2}% hit rate, {} bytes)",
                file.file_name, file.string_pool_hit_rate, file.file_size
            );
        }
    }
}
