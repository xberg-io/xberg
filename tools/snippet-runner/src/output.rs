use crate::error::Result;
use crate::types::{RunSummary, SnippetStatus, ValidationResult};
use std::path::Path;

/// Print a terminal summary table of validation results.
pub fn print_summary(summary: &RunSummary, show_code: bool) {
    println!();
    println!(
        "{:<60} {:<12} {:<10} {:<8} TIME",
        "SNIPPET", "LANGUAGE", "STATUS", "LEVEL"
    );
    println!("{}", "-".repeat(100));

    for result in &summary.results {
        let path_display = result.snippet.path.file_name().and_then(|n| n.to_str()).unwrap_or("?");

        let status_str = match result.status {
            SnippetStatus::Pass => "PASS",
            SnippetStatus::Fail => "FAIL",
            SnippetStatus::Skip => "SKIP",
            SnippetStatus::Error => "ERROR",
            SnippetStatus::Unavailable => "N/A",
        };

        println!(
            "{:<60} {:<12} {:<10} {:<8} {}ms",
            truncate(path_display, 58),
            result.snippet.language,
            status_str,
            result.level,
            result.duration_ms,
        );

        if result.status == SnippetStatus::Fail || result.status == SnippetStatus::Error {
            // Show source location
            let title_info = result
                .snippet
                .title
                .as_deref()
                .map(|t| format!(" (title: {t})"))
                .unwrap_or_default();
            println!(
                "  Source: {}:{}{}",
                result.snippet.path.display(),
                result.snippet.start_line,
                title_info,
            );

            // Show full error output
            if let Some(msg) = &result.message {
                let trimmed = msg.trim();
                if !trimmed.is_empty() {
                    println!("  Error:");
                    for line in trimmed.lines() {
                        println!("    {line}");
                    }
                }
            }

            // Optionally show the snippet source code
            if show_code {
                println!("  Code:");
                for (i, line) in result.snippet.code.lines().enumerate() {
                    println!("    {:>3} | {line}", i + 1);
                }
            }

            println!();
        }
    }

    println!("{}", "-".repeat(100));
    println!(
        "Total: {}  Passed: {}  Failed: {}  Skipped: {}  Errors: {}  Unavailable: {}",
        summary.total, summary.passed, summary.failed, summary.skipped, summary.errors, summary.unavailable,
    );
    println!();
}

/// Write results as JSON to a file.
pub fn write_json(results: &[ValidationResult], path: &Path) -> Result<()> {
    let json = serde_json::to_string_pretty(results)?;
    std::fs::write(path, json)?;
    Ok(())
}

/// Print snippet listing in a table format.
pub fn print_snippet_list(snippets: &[crate::types::Snippet]) {
    println!("{:<60} {:<12} {:<8} TITLE", "FILE", "LANGUAGE", "LINE");
    println!("{}", "-".repeat(95));

    for s in snippets {
        let path_display = s.path.file_name().and_then(|n| n.to_str()).unwrap_or("?");

        println!(
            "{:<60} {:<12} {:<8} {}",
            truncate(path_display, 58),
            s.language,
            s.start_line,
            s.title.as_deref().unwrap_or("-"),
        );
    }

    println!("{}", "-".repeat(95));
    println!("Total: {} snippets", snippets.len());
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max.saturating_sub(3)])
    }
}
