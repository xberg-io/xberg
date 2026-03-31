//! Noise and dirt detection for markdown extraction output.
//!
//! Detects common quality issues in extracted markdown such as HTML remnants,
//! garbled text, broken tables, page number artifacts, and other extraction
//! artifacts. All heuristics operate on the raw markdown string, line by line,
//! skipping content inside fenced code blocks.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A single noise issue found in the markdown.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoiseIssue {
    /// The kind of noise detected.
    pub kind: NoiseKind,
    /// 1-indexed line number where the issue was found.
    pub line: usize,
    /// ~80 char preview of the offending line.
    pub context: String,
    /// Severity of the issue.
    pub severity: Severity,
}

/// Categories of noise that can be detected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NoiseKind {
    /// HTML tags found outside code blocks.
    HtmlRemnant,
    /// Runs of 4+ consecutive blank lines.
    ExcessiveWhitespace,
    /// Lines with high non-ASCII ratio or consecutive punctuation.
    GarbledText,
    /// Heading markers with no content text.
    EmptyHeading,
    /// Pipe tables with inconsistent column counts.
    BrokenTable,
    /// List markers (`-`, `*`, `+`, `1.`) with no content.
    OrphanedListMarker,
    /// Standalone small numbers that look like page numbers.
    PageNumberArtifact,
    /// Lines repeated 5+ times in the document.
    HeaderFooterRepetition,
    /// Footnote references without matching definitions.
    DanglingReference,
    /// More headings than paragraphs (heading-heavy document).
    ExcessiveHeadingDensity,
}

impl NoiseKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::HtmlRemnant => "HtmlRemnant",
            Self::ExcessiveWhitespace => "ExcessiveWhitespace",
            Self::GarbledText => "GarbledText",
            Self::EmptyHeading => "EmptyHeading",
            Self::BrokenTable => "BrokenTable",
            Self::OrphanedListMarker => "OrphanedListMarker",
            Self::PageNumberArtifact => "PageNumberArtifact",
            Self::HeaderFooterRepetition => "HeaderFooterRepetition",
            Self::DanglingReference => "DanglingReference",
            Self::ExcessiveHeadingDensity => "ExcessiveHeadingDensity",
        }
    }
}

/// Severity levels for noise issues.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Severity {
    /// Informational — minor cosmetic issues.
    Info,
    /// Warning — likely extraction artifacts.
    Warning,
    /// Error — definite extraction failures.
    Error,
}

impl Severity {
    fn as_str(self) -> &'static str {
        match self {
            Self::Info => "Info",
            Self::Warning => "Warning",
            Self::Error => "Error",
        }
    }
}

/// Full diagnostic report for a markdown document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticReport {
    /// All noise issues found.
    pub issues: Vec<NoiseIssue>,
    /// Aggregated summary.
    pub summary: NoiseSummary,
}

/// Aggregated summary of noise issues.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoiseSummary {
    /// Total number of issues found.
    pub total_issues: usize,
    /// Issue counts grouped by kind.
    pub by_kind: HashMap<String, usize>,
    /// Issue counts grouped by severity.
    pub by_severity: HashMap<String, usize>,
    /// Overall noise score: 0.0 = clean, 1.0 = extremely noisy.
    pub noise_score: f64,
}

/// Represents a range of lines inside a fenced code block.
#[derive(Debug, Clone, Copy)]
struct CodeRange {
    start: usize, // inclusive, 0-indexed
    end: usize,   // inclusive, 0-indexed
}

/// Returns true if the given 0-indexed line is inside any code range.
fn in_code_block(line_idx: usize, code_ranges: &[CodeRange]) -> bool {
    code_ranges.iter().any(|r| line_idx >= r.start && line_idx <= r.end)
}

/// Identifies fenced code block ranges (``` or ~~~) using a simple state machine.
fn find_code_ranges(lines: &[&str]) -> Vec<CodeRange> {
    let mut ranges = Vec::new();
    let mut in_fence = false;
    let mut fence_start = 0;

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            if in_fence {
                ranges.push(CodeRange {
                    start: fence_start,
                    end: i,
                });
                in_fence = false;
            } else {
                fence_start = i;
                in_fence = true;
            }
        }
    }

    ranges
}

/// Truncates a string to approximately `max_len` characters for context previews.
/// Uses char boundaries to avoid panicking on multi-byte UTF-8 sequences.
fn truncate_context(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        // Find a valid char boundary at or before max_len
        let end = s.floor_char_boundary(max_len);
        format!("{}...", &s[..end])
    }
}

/// Detects HTML tags outside code blocks.
fn detect_html_remnants(lines: &[&str], code_ranges: &[CodeRange]) -> Vec<NoiseIssue> {
    let html_tags = [
        "<table", "</table", "<tr", "</tr", "<td", "</td", "<th", "</th", "<div", "</div", "<span", "</span", "<p>",
        "</p>", "<p ", "<br", "<b>", "</b>", "<strong", "</strong", "<i>", "</i>", "<em", "</em", "<a ", "</a>", "<a>",
        "<img", "<pre", "</pre", "<code", "</code", "<ul", "</ul", "<ol", "</ol", "<li", "</li", "<h1", "</h1", "<h2",
        "</h2", "<h3", "</h3", "<h4", "</h4", "<h5", "</h5", "<h6", "</h6", "<sup", "</sup", "<sub", "</sub",
    ];

    let mut issues = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        if in_code_block(i, code_ranges) {
            continue;
        }
        let lower = line.to_lowercase();
        for tag in &html_tags {
            if lower.contains(tag) {
                issues.push(NoiseIssue {
                    kind: NoiseKind::HtmlRemnant,
                    line: i + 1,
                    context: truncate_context(line, 80),
                    severity: Severity::Warning,
                });
                break; // one issue per line
            }
        }
    }
    issues
}

/// Detects runs of 4+ consecutive blank lines.
fn detect_excessive_whitespace(lines: &[&str], code_ranges: &[CodeRange]) -> Vec<NoiseIssue> {
    let mut issues = Vec::new();
    let mut blank_run_start: Option<usize> = None;
    let mut blank_count = 0;

    let flush_blank_run = |issues: &mut Vec<NoiseIssue>, count: usize, run_start: Option<usize>| {
        if let Some(start) = run_start
            && count >= 4
        {
            issues.push(NoiseIssue {
                kind: NoiseKind::ExcessiveWhitespace,
                line: start + 1,
                context: format!("{count} consecutive blank lines"),
                severity: Severity::Info,
            });
        }
    };

    for (i, line) in lines.iter().enumerate() {
        if in_code_block(i, code_ranges) {
            flush_blank_run(&mut issues, blank_count, blank_run_start);
            blank_count = 0;
            blank_run_start = None;
            continue;
        }

        if line.trim().is_empty() {
            if blank_run_start.is_none() {
                blank_run_start = Some(i);
            }
            blank_count += 1;
        } else {
            flush_blank_run(&mut issues, blank_count, blank_run_start);
            blank_count = 0;
            blank_run_start = None;
        }
    }

    // Handle trailing blank lines
    flush_blank_run(&mut issues, blank_count, blank_run_start);

    issues
}

/// Returns true if the line is a markdown table separator row (e.g., `|---|---|`).
fn is_table_separator_row(trimmed: &str) -> bool {
    trimmed.starts_with('|') && trimmed.chars().all(|c| c == '|' || c == '-' || c == ':' || c == ' ')
}

/// Returns true if the line is a markdown horizontal rule (`---`, `***`, `===`, `___`).
fn is_horizontal_rule(trimmed: &str) -> bool {
    if trimmed.len() < 3 {
        return false;
    }
    let first = trimmed.chars().next().unwrap_or(' ');
    matches!(first, '-' | '*' | '=' | '_') && trimmed.chars().all(|c| c == first || c == ' ')
}

/// Characters that commonly appear in markdown structural punctuation and should
/// NOT trigger the consecutive-punctuation garbled-text heuristic.
const MARKDOWN_STRUCTURAL_PUNCT: &[char] = &['-', '|', '*', '_', '=', '~', ':', '#', '>'];

/// Detects garbled text: lines with >70% non-ASCII or 4+ consecutive non-structural punctuation.
fn detect_garbled_text(lines: &[&str], code_ranges: &[CodeRange]) -> Vec<NoiseIssue> {
    let mut issues = Vec::new();

    for (i, line) in lines.iter().enumerate() {
        if in_code_block(i, code_ranges) {
            continue;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Skip table separator rows and horizontal rules — they are legitimate markdown
        if is_table_separator_row(trimmed) || is_horizontal_rule(trimmed) {
            continue;
        }

        let non_ws_chars: Vec<char> = trimmed.chars().filter(|c| !c.is_whitespace()).collect();
        if non_ws_chars.is_empty() {
            continue;
        }

        // Check non-ASCII ratio (raised from 40% to 70% to avoid flagging multilingual text)
        let non_ascii_count = non_ws_chars.iter().filter(|c| !c.is_ascii()).count();
        let ratio = non_ascii_count as f64 / non_ws_chars.len() as f64;
        if ratio > 0.7 {
            issues.push(NoiseIssue {
                kind: NoiseKind::GarbledText,
                line: i + 1,
                context: truncate_context(line, 80),
                severity: Severity::Warning,
            });
            continue;
        }

        // Check for 4+ consecutive punctuation, excluding markdown structural characters.
        // Common markdown patterns like `---`, `***`, `|||`, `[^`, `$$` use structural
        // punctuation and should not be flagged.
        let mut consecutive_punct = 0;
        let mut has_punct_run = false;
        for ch in trimmed.chars() {
            if ch.is_ascii_punctuation() && !MARKDOWN_STRUCTURAL_PUNCT.contains(&ch) {
                consecutive_punct += 1;
                if consecutive_punct >= 4 {
                    has_punct_run = true;
                    break;
                }
            } else {
                consecutive_punct = 0;
            }
        }
        if has_punct_run {
            issues.push(NoiseIssue {
                kind: NoiseKind::GarbledText,
                line: i + 1,
                context: truncate_context(line, 80),
                severity: Severity::Warning,
            });
        }
    }

    issues
}

/// Detects empty headings (e.g., `# ` with no content).
fn detect_empty_headings(lines: &[&str], code_ranges: &[CodeRange]) -> Vec<NoiseIssue> {
    let mut issues = Vec::new();

    for (i, line) in lines.iter().enumerate() {
        if in_code_block(i, code_ranges) {
            continue;
        }
        let trimmed = line.trim();
        // Match ^#{1,6}\s*$
        if trimmed.starts_with('#') {
            let hash_count = trimmed.chars().take_while(|&c| c == '#').count();
            if (1..=6).contains(&hash_count) {
                let rest = &trimmed[hash_count..];
                if rest.trim().is_empty() {
                    issues.push(NoiseIssue {
                        kind: NoiseKind::EmptyHeading,
                        line: i + 1,
                        context: truncate_context(line, 80),
                        severity: Severity::Error,
                    });
                }
            }
        }
    }

    issues
}

/// Detects broken pipe tables with inconsistent column counts.
fn detect_broken_tables(lines: &[&str], code_ranges: &[CodeRange]) -> Vec<NoiseIssue> {
    let mut issues = Vec::new();

    let mut table_start: Option<usize> = None;
    let mut header_col_count: Option<usize> = None;

    for (i, line) in lines.iter().enumerate() {
        if in_code_block(i, code_ranges) {
            // End any open table
            table_start = None;
            header_col_count = None;
            continue;
        }

        let trimmed = line.trim();
        if trimmed.starts_with('|') {
            let col_count = trimmed.matches('|').count();
            if table_start.is_none() {
                // Start of a new table
                table_start = Some(i);
                header_col_count = Some(col_count);
            } else if let Some(expected) = header_col_count {
                // Skip separator rows (e.g., |---|---|)
                let is_separator = trimmed.chars().all(|c| c == '|' || c == '-' || c == ':' || c == ' ');
                if !is_separator && col_count != expected {
                    issues.push(NoiseIssue {
                        kind: NoiseKind::BrokenTable,
                        line: i + 1,
                        context: truncate_context(line, 80),
                        severity: Severity::Warning,
                    });
                }
            }
        } else {
            // Non-table line ends the current table
            table_start = None;
            header_col_count = None;
        }
    }

    issues
}

/// Detects orphaned list markers with no content.
fn detect_orphaned_list_markers(lines: &[&str], code_ranges: &[CodeRange]) -> Vec<NoiseIssue> {
    let mut issues = Vec::new();

    for (i, line) in lines.iter().enumerate() {
        if in_code_block(i, code_ranges) {
            continue;
        }
        let trimmed = line.trim();

        // Unordered: -, *, + with nothing after
        let is_orphaned_unordered = (trimmed == "-" || trimmed == "*" || trimmed == "+")
            || (trimmed.len() >= 2
                && (trimmed.starts_with("- ") || trimmed.starts_with("* ") || trimmed.starts_with("+ "))
                && trimmed[2..].trim().is_empty());

        // Ordered: digits followed by . and nothing else
        let is_orphaned_ordered = if let Some(dot_pos) = trimmed.find('.') {
            let before_dot = &trimmed[..dot_pos];
            let after_dot = &trimmed[dot_pos + 1..];
            !before_dot.is_empty() && before_dot.chars().all(|c| c.is_ascii_digit()) && after_dot.trim().is_empty()
        } else {
            false
        };

        if is_orphaned_unordered || is_orphaned_ordered {
            issues.push(NoiseIssue {
                kind: NoiseKind::OrphanedListMarker,
                line: i + 1,
                context: truncate_context(line, 80),
                severity: Severity::Warning,
            });
        }
    }

    issues
}

/// Detects standalone small numbers that look like page number artifacts.
///
/// Only flags when at least 3 such lines exist with sequential or near-sequential values.
fn detect_page_number_artifacts(lines: &[&str], code_ranges: &[CodeRange]) -> Vec<NoiseIssue> {
    // Collect candidate lines: standalone numbers 1-9999
    let mut candidates: Vec<(usize, u32)> = Vec::new();

    for (i, line) in lines.iter().enumerate() {
        if in_code_block(i, code_ranges) {
            continue;
        }
        let trimmed = line.trim();
        if let Ok(num) = trimmed.parse::<u32>()
            && (1..=9999).contains(&num)
            && trimmed.len() <= 4
        {
            candidates.push((i, num));
        }
    }

    if candidates.len() < 3 {
        return Vec::new();
    }

    // Check for sequential/near-sequential values
    let values: Vec<u32> = candidates.iter().map(|(_, v)| *v).collect();
    let mut sequential_count = 0;
    for window in values.windows(2) {
        let diff = window[1].saturating_sub(window[0]);
        if (1..=3).contains(&diff) {
            sequential_count += 1;
        }
    }

    // Need at least 2 sequential pairs (3 sequential numbers)
    if sequential_count < 2 {
        return Vec::new();
    }

    candidates
        .iter()
        .map(|(i, _)| NoiseIssue {
            kind: NoiseKind::PageNumberArtifact,
            line: i + 1,
            context: truncate_context(lines[*i], 80),
            severity: Severity::Info,
        })
        .collect()
}

/// Returns true if the line is a pipe table row (starts with `|`).
fn is_pipe_table_row(trimmed: &str) -> bool {
    trimmed.starts_with('|')
}

/// Returns true if the line is only an image placeholder like `![]()`.
fn is_image_placeholder(trimmed: &str) -> bool {
    trimmed.starts_with("![")
}

/// Detects lines that repeat 5+ times in the document (header/footer repetition).
///
/// Skips pipe table rows, image placeholders, table separator rows, and lines
/// with fewer than 15 non-whitespace characters (too short to be a meaningful
/// header/footer candidate).
fn detect_header_footer_repetition(lines: &[&str], code_ranges: &[CodeRange]) -> Vec<NoiseIssue> {
    let mut line_counts: HashMap<&str, Vec<usize>> = HashMap::new();

    for (i, line) in lines.iter().enumerate() {
        if in_code_block(i, code_ranges) {
            continue;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Skip pipe table rows (including separator rows like |---|---|)
        if is_pipe_table_row(trimmed) {
            continue;
        }

        // Skip image placeholders
        if is_image_placeholder(trimmed) {
            continue;
        }

        // Require minimum 15 non-whitespace characters to be a header/footer candidate
        let non_ws_count = trimmed.chars().filter(|c| !c.is_whitespace()).count();
        if non_ws_count < 15 {
            continue;
        }

        line_counts.entry(trimmed).or_default().push(i);
    }

    let mut issues = Vec::new();
    for (content, positions) in &line_counts {
        if positions.len() >= 5 {
            for &pos in positions {
                issues.push(NoiseIssue {
                    kind: NoiseKind::HeaderFooterRepetition,
                    line: pos + 1,
                    context: truncate_context(content, 80),
                    severity: Severity::Warning,
                });
            }
        }
    }

    // Sort by line number for deterministic output
    issues.sort_by_key(|issue| issue.line);
    issues
}

/// Detects footnote references `[^N]` without corresponding `[^N]:` definitions.
fn detect_dangling_references(lines: &[&str], code_ranges: &[CodeRange]) -> Vec<NoiseIssue> {
    let mut references: Vec<(usize, String)> = Vec::new(); // (line_idx, label)
    let mut definitions: std::collections::HashSet<String> = std::collections::HashSet::new();

    for (i, line) in lines.iter().enumerate() {
        if in_code_block(i, code_ranges) {
            continue;
        }

        let mut start = 0;
        while let Some(pos) = line[start..].find("[^") {
            let abs_pos = start + pos;
            let after = &line[abs_pos + 2..];
            if let Some(close) = after.find(']') {
                let label = after[..close].to_string();
                let after_close = &after[close + 1..];
                if after_close.starts_with(':') {
                    definitions.insert(label);
                } else {
                    references.push((i, label));
                }
                start = abs_pos + 2 + close + 1;
            } else {
                break;
            }
        }
    }

    references
        .into_iter()
        .filter(|(_, label)| !definitions.contains(label))
        .map(|(i, _)| NoiseIssue {
            kind: NoiseKind::DanglingReference,
            line: i + 1,
            context: truncate_context(lines[i], 80),
            severity: Severity::Warning,
        })
        .collect()
}

/// Detects excessive heading density (more headings than paragraphs when heading count > 5).
fn detect_excessive_heading_density(lines: &[&str], code_ranges: &[CodeRange]) -> Vec<NoiseIssue> {
    let mut heading_count = 0usize;
    let mut paragraph_count = 0usize;

    for (i, line) in lines.iter().enumerate() {
        if in_code_block(i, code_ranges) {
            continue;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if trimmed.starts_with('#') {
            let hash_count = trimmed.chars().take_while(|&c| c == '#').count();
            if (1..=6).contains(&hash_count) {
                heading_count += 1;
                continue;
            }
        }

        // Skip list markers, table rows
        if trimmed.starts_with('|')
            || trimmed.starts_with("- ")
            || trimmed.starts_with("* ")
            || trimmed.starts_with("+ ")
            || (trimmed.len() >= 2 && trimmed.as_bytes()[0].is_ascii_digit() && trimmed.contains(". "))
        {
            continue;
        }

        paragraph_count += 1;
    }

    if heading_count > paragraph_count && heading_count > 5 {
        vec![NoiseIssue {
            kind: NoiseKind::ExcessiveHeadingDensity,
            line: 1,
            context: format!("{heading_count} headings vs {paragraph_count} paragraphs"),
            severity: Severity::Warning,
        }]
    } else {
        Vec::new()
    }
}

/// Runs all noise detection heuristics and produces a diagnostic report.
pub fn detect_noise(markdown: &str) -> DiagnosticReport {
    let lines: Vec<&str> = markdown.lines().collect();
    let code_ranges = find_code_ranges(&lines);

    let mut issues = Vec::new();
    issues.extend(detect_html_remnants(&lines, &code_ranges));
    issues.extend(detect_excessive_whitespace(&lines, &code_ranges));
    issues.extend(detect_garbled_text(&lines, &code_ranges));
    issues.extend(detect_empty_headings(&lines, &code_ranges));
    issues.extend(detect_broken_tables(&lines, &code_ranges));
    issues.extend(detect_orphaned_list_markers(&lines, &code_ranges));
    issues.extend(detect_page_number_artifacts(&lines, &code_ranges));
    issues.extend(detect_header_footer_repetition(&lines, &code_ranges));
    issues.extend(detect_dangling_references(&lines, &code_ranges));
    issues.extend(detect_excessive_heading_density(&lines, &code_ranges));

    let total_lines = lines.len();
    let summary = build_summary(&issues, total_lines);

    DiagnosticReport { issues, summary }
}

/// Builds an aggregated summary from a list of issues.
fn build_summary(issues: &[NoiseIssue], total_lines: usize) -> NoiseSummary {
    let mut by_kind: HashMap<String, usize> = HashMap::new();
    let mut by_severity: HashMap<String, usize> = HashMap::new();

    let mut error_count = 0usize;
    let mut warning_count = 0usize;
    let mut info_count = 0usize;

    for issue in issues {
        *by_kind.entry(issue.kind.as_str().to_string()).or_insert(0) += 1;
        *by_severity.entry(issue.severity.as_str().to_string()).or_insert(0) += 1;

        match issue.severity {
            Severity::Error => error_count += 1,
            Severity::Warning => warning_count += 1,
            Severity::Info => info_count += 1,
        }
    }

    let weighted = error_count as f64 * 0.3 + warning_count as f64 * 0.1 + info_count as f64 * 0.02;
    let denominator = (total_lines / 50).max(1) as f64;
    let noise_score = (weighted / denominator).min(1.0);

    NoiseSummary {
        total_issues: issues.len(),
        by_kind,
        by_severity,
        noise_score,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_markdown() {
        let md = "\
# Hello World

This is a paragraph with some text.

## Section Two

Another paragraph here with more content.

- Item one
- Item two
- Item three
";
        let report = detect_noise(md);
        assert!(
            report.issues.is_empty(),
            "Expected 0 issues for clean markdown, got: {:?}",
            report.issues
        );
        assert_eq!(report.summary.noise_score, 0.0);
    }

    #[test]
    fn test_html_remnant_detection() {
        let md = "\
# Title

<div class=\"content\">Some text</div>

More text here.
";
        let report = detect_noise(md);
        let html_issues: Vec<_> = report
            .issues
            .iter()
            .filter(|i| i.kind == NoiseKind::HtmlRemnant)
            .collect();
        assert!(!html_issues.is_empty(), "Expected HTML remnant issues");
        assert_eq!(html_issues[0].severity, Severity::Warning);
    }

    #[test]
    fn test_empty_heading() {
        let md = "\
#

Some content here.
";
        let report = detect_noise(md);
        let heading_issues: Vec<_> = report
            .issues
            .iter()
            .filter(|i| i.kind == NoiseKind::EmptyHeading)
            .collect();
        assert_eq!(heading_issues.len(), 1);
        assert_eq!(heading_issues[0].severity, Severity::Error);
        assert_eq!(heading_issues[0].line, 1);
    }

    #[test]
    fn test_broken_table() {
        let md = "\
| Col1 | Col2 | Col3 |
|------|------|------|
| a | b | c |
| d | e |
";
        let report = detect_noise(md);
        let table_issues: Vec<_> = report
            .issues
            .iter()
            .filter(|i| i.kind == NoiseKind::BrokenTable)
            .collect();
        assert!(!table_issues.is_empty(), "Expected broken table issues");
        assert_eq!(table_issues[0].severity, Severity::Warning);
    }

    #[test]
    fn test_code_block_skipped() {
        let md = "\
# Title

```html
<div>This should not be flagged</div>
<table><tr><td>Also not flagged</td></tr></table>
```

Normal paragraph.
";
        let report = detect_noise(md);
        let html_issues: Vec<_> = report
            .issues
            .iter()
            .filter(|i| i.kind == NoiseKind::HtmlRemnant)
            .collect();
        assert!(
            html_issues.is_empty(),
            "HTML inside code blocks should not be flagged, got: {:?}",
            html_issues
        );
    }

    #[test]
    fn test_garbled_text() {
        let md = "\
# Title

\u{00e4}\u{00f6}\u{00fc}\u{00e4}\u{00f6}\u{00fc}\u{00e4}\u{00f6}\u{00fc}\u{00e4}\u{00f6}\u{00fc}\u{00e4}\u{00f6}\u{00fc}\u{00e4}\u{00f6}\u{00fc}\u{00e4}\u{00f6}\u{00fc}

Normal text here.
";
        let report = detect_noise(md);
        let garbled: Vec<_> = report
            .issues
            .iter()
            .filter(|i| i.kind == NoiseKind::GarbledText)
            .collect();
        assert!(
            !garbled.is_empty(),
            "Expected garbled text detection for high non-ASCII line"
        );
    }

    #[test]
    fn test_page_numbers() {
        let md = "\
# Title

Some text.

1

More text.

2

Even more text.

3

Final text.
";
        let report = detect_noise(md);
        let page_issues: Vec<_> = report
            .issues
            .iter()
            .filter(|i| i.kind == NoiseKind::PageNumberArtifact)
            .collect();
        assert!(
            !page_issues.is_empty(),
            "Expected page number artifact detection for sequential standalone numbers"
        );
        assert_eq!(page_issues.len(), 3);
    }

    #[test]
    fn test_dangling_footnote() {
        let md = "\
# Title

This has a reference[^1] and another[^2].

[^1]: This is defined.
";
        let report = detect_noise(md);
        let dangling: Vec<_> = report
            .issues
            .iter()
            .filter(|i| i.kind == NoiseKind::DanglingReference)
            .collect();
        assert!(!dangling.is_empty(), "Expected dangling reference for [^2]");
        // [^1] should not be flagged since it has a definition
        assert!(
            dangling.iter().all(|i| {
                let line = &md.lines().collect::<Vec<_>>()[i.line - 1];
                line.contains("[^2]")
            }),
            "Only [^2] should be flagged as dangling"
        );
    }

    #[test]
    fn test_empty_input() {
        let report = detect_noise("");
        assert!(report.issues.is_empty());
        assert_eq!(report.summary.total_issues, 0);
        assert_eq!(report.summary.noise_score, 0.0);
    }

    // ---- False-positive regression tests ----

    #[test]
    fn test_pipe_table_rows_not_flagged_as_header_footer() {
        // Pipe table rows like `|  |  |  |` should NOT be flagged as repetition
        let md = "\
| Col1 | Col2 | Col3 |
|------|------|------|
|  |  |  |
|  |  |  |
|  |  |  |
|  |  |  |
|  |  |  |
|  |  |  |
";
        let report = detect_noise(md);
        let rep_issues: Vec<_> = report
            .issues
            .iter()
            .filter(|i| i.kind == NoiseKind::HeaderFooterRepetition)
            .collect();
        assert!(
            rep_issues.is_empty(),
            "Pipe table rows should not be flagged as header/footer repetition, got: {:?}",
            rep_issues
        );
    }

    #[test]
    fn test_image_placeholders_not_flagged_as_header_footer() {
        let md = "\
# Gallery

![](image1.png)
![](image2.png)
![](image3.png)
![](image4.png)
![](image5.png)
![](image6.png)
";
        let report = detect_noise(md);
        let rep_issues: Vec<_> = report
            .issues
            .iter()
            .filter(|i| i.kind == NoiseKind::HeaderFooterRepetition)
            .collect();
        assert!(
            rep_issues.is_empty(),
            "Image placeholders should not be flagged as header/footer repetition, got: {:?}",
            rep_issues
        );
    }

    #[test]
    fn test_table_separator_not_flagged_as_garbled() {
        let md = "\
| Col1 | Col2 |
|------|------|
| a    | b    |
";
        let report = detect_noise(md);
        let garbled: Vec<_> = report
            .issues
            .iter()
            .filter(|i| i.kind == NoiseKind::GarbledText)
            .collect();
        assert!(
            garbled.is_empty(),
            "Table separator rows should not be flagged as garbled text, got: {:?}",
            garbled
        );
    }

    #[test]
    fn test_horizontal_rule_not_flagged_as_garbled() {
        let md = "\
# Section

---

More text.

***

Even more text.
";
        let report = detect_noise(md);
        let garbled: Vec<_> = report
            .issues
            .iter()
            .filter(|i| i.kind == NoiseKind::GarbledText)
            .collect();
        assert!(
            garbled.is_empty(),
            "Horizontal rules should not be flagged as garbled text, got: {:?}",
            garbled
        );
    }

    #[test]
    fn test_arabic_text_not_flagged_as_garbled() {
        // Arabic text is 100% non-ASCII but is legitimate multilingual content.
        // At 70% threshold it would be flagged, but we want to verify that
        // mixed content below 70% is NOT flagged.
        let md = "\
# Document

Some English text mixed with \u{0645}\u{0631}\u{062d}\u{0628}\u{0627} Arabic words in a sentence.
";
        let report = detect_noise(md);
        let garbled: Vec<_> = report
            .issues
            .iter()
            .filter(|i| i.kind == NoiseKind::GarbledText)
            .collect();
        assert!(
            garbled.is_empty(),
            "Mixed Arabic/English text below 70% non-ASCII should not be flagged, got: {:?}",
            garbled
        );
    }

    #[test]
    fn test_cjk_text_below_threshold_not_flagged() {
        // CJK text mixed with enough ASCII to stay below 70%
        let md = "\
# Document

This line has some CJK chars \u{4f60}\u{597d} mixed with English text here.
";
        let report = detect_noise(md);
        let garbled: Vec<_> = report
            .issues
            .iter()
            .filter(|i| i.kind == NoiseKind::GarbledText)
            .collect();
        assert!(
            garbled.is_empty(),
            "Mixed CJK/English text below 70% non-ASCII should not be flagged, got: {:?}",
            garbled
        );
    }

    #[test]
    fn test_truly_garbled_text_still_flagged() {
        // Mojibake / truly garbled text with high non-ASCII ratio (>70%)
        let md = "\
# Title

\u{00c3}\u{00a9}\u{00c3}\u{00a8}\u{00c3}\u{00aa}\u{00c3}\u{00ab}\u{00c3}\u{00a0}\u{00c3}\u{00a2}\u{00c3}\u{00a4}\u{00c3}\u{00a6}\u{00c3}\u{00b9}\u{00c3}\u{00bb}\u{00c3}\u{00bc}\u{00c3}\u{00b1}\u{00c3}\u{00a7}\u{00c3}\u{00b0}\u{00c3}\u{00be}

Normal text here.
";
        let report = detect_noise(md);
        let garbled: Vec<_> = report
            .issues
            .iter()
            .filter(|i| i.kind == NoiseKind::GarbledText)
            .collect();
        assert!(
            !garbled.is_empty(),
            "Truly garbled text (high non-ASCII ratio) should still be flagged"
        );
    }

    #[test]
    fn test_markdown_structural_punct_not_flagged_as_garbled() {
        // Lines with markdown structural punctuation like `---`, `***`, `|||`, `$$`
        let md = "\
# Title

Some text with --- dashes in it.

Text with **bold** and ~~strike~~ formatting.
";
        let report = detect_noise(md);
        let garbled: Vec<_> = report
            .issues
            .iter()
            .filter(|i| i.kind == NoiseKind::GarbledText)
            .collect();
        assert!(
            garbled.is_empty(),
            "Markdown structural punctuation should not be flagged as garbled text, got: {:?}",
            garbled
        );
    }

    #[test]
    fn test_short_repeated_lines_not_flagged_as_header_footer() {
        // Short lines (< 15 non-ws chars) repeated many times should not be flagged
        let md = "\
Hello world
Hello world
Hello world
Hello world
Hello world
Hello world

Some other text here.
";
        let report = detect_noise(md);
        let rep_issues: Vec<_> = report
            .issues
            .iter()
            .filter(|i| i.kind == NoiseKind::HeaderFooterRepetition)
            .collect();
        assert!(
            rep_issues.is_empty(),
            "Short repeated lines (< 15 non-ws chars) should not be flagged, got: {:?}",
            rep_issues
        );
    }

    #[test]
    fn test_genuine_header_footer_repetition_still_flagged() {
        // Genuine header/footer text repeated 5+ times should still be flagged
        let md = "\
Copyright 2024 Acme Corporation All Rights Reserved
# Chapter 1
Some content here.
Copyright 2024 Acme Corporation All Rights Reserved
# Chapter 2
More content here.
Copyright 2024 Acme Corporation All Rights Reserved
# Chapter 3
Even more content.
Copyright 2024 Acme Corporation All Rights Reserved
# Chapter 4
Still more content.
Copyright 2024 Acme Corporation All Rights Reserved
";
        let report = detect_noise(md);
        let rep_issues: Vec<_> = report
            .issues
            .iter()
            .filter(|i| i.kind == NoiseKind::HeaderFooterRepetition)
            .collect();
        assert!(
            !rep_issues.is_empty(),
            "Genuine header/footer repetition (5+ times, 15+ chars) should be flagged"
        );
        assert_eq!(rep_issues.len(), 5);
    }
}
