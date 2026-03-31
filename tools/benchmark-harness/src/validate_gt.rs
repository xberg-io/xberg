//! Ground truth validation and HTML-to-GFM cleanup
//!
//! Replaces the Python scripts `validate_ground_truth.py` and `cleanup_html_in_gt.py`
//! with a single Rust module that can report HTML issues and optionally fix them in-place.

use crate::{Fixture, Result};
use regex::Regex;
use std::path::{Path, PathBuf};

/// Configuration for the validate-gt subcommand.
pub struct ValidateGtConfig {
    /// Directory containing fixture JSON files.
    pub fixtures_dir: PathBuf,
    /// When true, auto-convert HTML tags to GFM markdown in-place.
    pub fix: bool,
}

/// Summary report produced by [`validate_ground_truth`].
pub struct ValidateGtReport {
    pub total_fixtures: usize,
    pub with_text_gt: usize,
    pub with_markdown_gt: usize,
    pub missing_text_gt: usize,
    pub missing_markdown_gt: usize,
    /// Files smaller than 10 bytes: (relative path, size).
    pub small_gt_files: Vec<(String, u64)>,
    /// Markdown GT files containing HTML: (path, list of tags found).
    pub html_issues: Vec<(String, Vec<String>)>,
    /// Number of fixes applied (only non-zero when `--fix` is used).
    pub fixes_applied: usize,
    /// GT files containing noise issues (Warning or Error severity): (path, issue_count).
    pub noisy_gt_files: Vec<(String, usize)>,
    /// GT files with low block diversity (no headings for files > 100 bytes).
    pub low_diversity_gt: Vec<String>,
}

// ---------------------------------------------------------------------------
// HTML detection
// ---------------------------------------------------------------------------

/// Common HTML tags that should not appear in GFM ground truth.
const HTML_TAG_NAMES: &[&str] = &[
    "table", "tr", "td", "th", "b", "strong", "i", "em", "div", "span", "p", "br", "a ", "code", "pre", "img", "sup",
    "sub", "ul", "ol", "li", "h1", "h2", "h3", "h4", "h5", "h6",
];

/// Build a regex that matches opening or self-closing HTML tags for the names
/// listed in [`HTML_TAG_NAMES`].
fn html_tag_regex() -> Regex {
    // Build alternation: `table|tr|td|…|h[1-6]`
    // We handle the special "a " entry by converting it to `a\s` so it only
    // matches `<a ` (anchor with attributes) and not random words starting with "a".
    let alts: Vec<String> = HTML_TAG_NAMES
        .iter()
        .map(|t| {
            if *t == "a " {
                r"a\s".to_string()
            } else {
                regex::escape(t)
            }
        })
        .collect();

    let pattern = format!(r"(?i)</?(?:{})(?:\s[^>]*)?\s*/?>", alts.join("|"));
    Regex::new(&pattern).expect("invalid HTML tag regex")
}

/// Strip content inside fenced code blocks so we don't flag code examples.
///
/// Uses a line-by-line scanner because the `regex` crate does not support
/// backreferences needed to match opening/closing fences of the same length.
fn strip_fenced_code_blocks(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut in_fence = false;
    let mut fence_marker = String::new();

    for line in text.lines() {
        let trimmed = line.trim_start();
        if in_fence {
            // Check if this line closes the current fence
            if trimmed.starts_with(&fence_marker) && trimmed.trim() == fence_marker {
                in_fence = false;
                fence_marker.clear();
            }
            // Skip all lines inside fence (including open/close)
            continue;
        }

        // Check for opening fence: ``` or ~~~  (3+ chars)
        let opens_backtick = trimmed.starts_with("```");
        let opens_tilde = trimmed.starts_with("~~~");
        if opens_backtick || opens_tilde {
            let fence_char = if opens_backtick { '`' } else { '~' };
            let fence_len = trimmed.chars().take_while(|&c| c == fence_char).count();
            fence_marker = std::iter::repeat_n(fence_char, fence_len).collect();
            in_fence = true;
            continue;
        }

        result.push_str(line);
        result.push('\n');
    }

    result
}

/// Strip inline code spans.
fn strip_inline_code(text: &str) -> String {
    let inline_re = Regex::new(r"`[^`]+`").expect("inline code regex");
    inline_re.replace_all(text, "").into_owned()
}

/// Detect HTML tags in a markdown string, returning the list of matched tags.
pub fn detect_html_tags(content: &str) -> Vec<String> {
    let cleaned = strip_inline_code(&strip_fenced_code_blocks(content));
    let re = html_tag_regex();
    re.find_iter(&cleaned).map(|m| m.as_str().to_string()).collect()
}

// ---------------------------------------------------------------------------
// HTML-to-GFM conversion
// ---------------------------------------------------------------------------

/// Convert common HTML tags to their GFM equivalents.
///
/// This intentionally does **not** attempt to convert `<table>` blocks — those
/// are complex and should be flagged in report mode instead.
pub fn convert_html_to_gfm(content: &str) -> (String, usize) {
    let mut text = content.to_string();
    let mut count: usize = 0;

    /// Helper: apply a regex substitution and accumulate the replacement count.
    macro_rules! apply {
        ($re:expr, $rep:expr) => {{
            let re = Regex::new($re).expect("regex");
            let before_len = text.len();
            let new = re.replace_all(&text, $rep);
            // Count by number of matches (cheaper than diffing strings)
            let n = re.find_iter(&text).count();
            if n > 0 {
                text = new.into_owned();
                count += n;
            }
            let _ = before_len; // suppress unused warning
        }};
    }

    // <b>text</b> or <strong>text</strong> → **text**
    apply!(r"(?is)<(?:b|strong)>(.*?)</(?:b|strong)>", "**$1**");

    // <i>text</i> or <em>text</em> → *text*
    apply!(r"(?is)<(?:i|em)>(.*?)</(?:i|em)>", "*$1*");

    // <code>text</code> → `text`
    apply!(r"(?is)<code>(.*?)</code>", "`$1`");

    // <a href="url">text</a> → [text](url)
    apply!(
        r#"(?is)<a\s+(?:[^>]*\s+)?href=["']([^"']*)["'][^>]*>(.*?)</a>"#,
        "[$2]($1)"
    );

    // <br>, <br/>, <br /> → newline
    apply!(r"(?i)<br\s*/?>", "\n");

    // <hr>, <hr/>, <hr /> → ---
    apply!(r"(?i)<hr\s*/?>", "---");

    // <sup>text</sup> → text (no GFM equivalent)
    apply!(r"(?is)<sup>(.*?)</sup>", "$1");

    // <sub>text</sub> → text
    apply!(r"(?is)<sub>(.*?)</sub>", "$1");

    // <pre>text</pre> → fenced code block
    {
        let re = Regex::new(r"(?is)<pre>(.*?)</pre>").expect("pre regex");
        let n = re.find_iter(&text).count();
        if n > 0 {
            text = re
                .replace_all(&text, |caps: &regex::Captures| {
                    let inner = caps[1].trim();
                    format!("```\n{}\n```", inner)
                })
                .into_owned();
            count += n;
        }
    }

    // Strip <div>, </div>, <span>, </span>, <p>, </p> keeping content
    apply!(r"(?i)</?div(?:\s[^>]*)?>", "");
    apply!(r"(?i)</?span(?:\s[^>]*)?>", "");
    apply!(r"(?i)</?p(?:\s[^>]*)?>", "");

    (text, count)
}

// ---------------------------------------------------------------------------
// Main validation entry point
// ---------------------------------------------------------------------------

/// Walk fixture JSON files, resolve GT paths, and produce a validation report.
///
/// When `config.fix` is true, HTML tags in markdown GT files are auto-converted
/// to GFM equivalents in-place.
pub fn validate_ground_truth(config: &ValidateGtConfig) -> Result<ValidateGtReport> {
    let mut report = ValidateGtReport {
        total_fixtures: 0,
        with_text_gt: 0,
        with_markdown_gt: 0,
        missing_text_gt: 0,
        missing_markdown_gt: 0,
        small_gt_files: Vec::new(),
        html_issues: Vec::new(),
        fixes_applied: 0,
        noisy_gt_files: Vec::new(),
        low_diversity_gt: Vec::new(),
    };

    let fixture_files = collect_json_files(&config.fixtures_dir)?;

    for fixture_path in &fixture_files {
        let fixture = match Fixture::from_file(fixture_path) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("Warning: failed to load fixture {}: {}", fixture_path.display(), e);
                continue;
            }
        };

        report.total_fixtures += 1;

        let Some(gt) = &fixture.ground_truth else {
            report.missing_text_gt += 1;
            report.missing_markdown_gt += 1;
            continue;
        };

        // Resolve paths relative to the fixture file's parent directory.
        let fixture_dir = fixture_path.parent().unwrap_or(Path::new("."));

        // --- text GT ---
        let text_path = fixture_dir.join(&gt.text_file);
        if text_path.exists() {
            report.with_text_gt += 1;
            check_small_file(&text_path, &config.fixtures_dir, &mut report);
        } else {
            report.missing_text_gt += 1;
        }

        // --- markdown GT ---
        if let Some(md_rel) = &gt.markdown_file {
            let md_path = fixture_dir.join(md_rel);
            if md_path.exists() {
                report.with_markdown_gt += 1;
                check_small_file(&md_path, &config.fixtures_dir, &mut report);
                check_html_in_markdown(&md_path, config.fix, &mut report);
                check_noise_in_markdown(&md_path, &config.fixtures_dir, &mut report);
                check_block_diversity(&md_path, &config.fixtures_dir, &mut report);
            } else {
                report.missing_markdown_gt += 1;
            }
        }
    }

    Ok(report)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Recursively collect `*.json` files under `dir`.
fn collect_json_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    if !dir.is_dir() {
        return Err(crate::Error::Config(format!(
            "Fixtures directory does not exist: {}",
            dir.display()
        )));
    }
    collect_json_recursive(dir, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_json_recursive(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in std::fs::read_dir(dir).map_err(crate::Error::Io)? {
        let entry = entry.map_err(crate::Error::Io)?;
        let path = entry.path();
        if path.is_dir() {
            collect_json_recursive(&path, out)?;
        } else if path.extension().is_some_and(|ext| ext == "json") {
            out.push(path);
        }
    }
    Ok(())
}

/// Warn if a GT file is suspiciously small (<10 bytes).
fn check_small_file(path: &Path, base: &Path, report: &mut ValidateGtReport) {
    if let Ok(meta) = std::fs::metadata(path)
        && meta.len() < 10
    {
        let display = path.strip_prefix(base).unwrap_or(path).display().to_string();
        report.small_gt_files.push((display, meta.len()));
    }
}

/// Check a markdown GT file for noise issues (Warning or Error severity).
fn check_noise_in_markdown(path: &Path, base: &Path, report: &mut ValidateGtReport) {
    let Ok(content) = std::fs::read_to_string(path) else {
        return;
    };

    let diagnostic = crate::noise_detection::detect_noise(&content);
    let serious_count = diagnostic
        .issues
        .iter()
        .filter(|issue| {
            matches!(
                issue.severity,
                crate::noise_detection::Severity::Warning | crate::noise_detection::Severity::Error
            )
        })
        .count();

    if serious_count > 0 {
        let display = path.strip_prefix(base).unwrap_or(path).display().to_string();
        report.noisy_gt_files.push((display, serious_count));
    }
}

/// Check if a markdown GT file has at least one heading for files > 100 bytes.
fn check_block_diversity(path: &Path, base: &Path, report: &mut ValidateGtReport) {
    let Ok(meta) = std::fs::metadata(path) else {
        return;
    };

    if meta.len() <= 100 {
        return;
    }

    let Ok(content) = std::fs::read_to_string(path) else {
        return;
    };

    let blocks = crate::markdown_quality::parse_markdown_blocks(&content);
    let has_heading = blocks.iter().any(|b| b.block_type.is_heading());

    if !has_heading {
        let display = path.strip_prefix(base).unwrap_or(path).display().to_string();
        report.low_diversity_gt.push(display);
    }
}

/// Check a markdown GT file for HTML tags; optionally fix in-place.
fn check_html_in_markdown(path: &Path, fix: bool, report: &mut ValidateGtReport) {
    let Ok(content) = std::fs::read_to_string(path) else {
        return;
    };

    let tags = detect_html_tags(&content);
    if tags.is_empty() {
        return;
    }

    report.html_issues.push((path.display().to_string(), tags));

    if fix {
        let (converted, n) = convert_html_to_gfm(&content);
        if n > 0 && converted != content && std::fs::write(path, &converted).is_ok() {
            report.fixes_applied += n;
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_html_tag_detection() {
        let tags = detect_html_tags("<b>bold</b> and <i>italic</i> and <table><tr><td>cell</td></tr></table>");
        assert!(!tags.is_empty(), "should detect HTML tags");
        // Should find <b>, </b>, <i>, </i>, <table>, <tr>, <td>, </td>, </tr>, </table>
        assert!(tags.iter().any(|t| t.contains("b>")), "should detect <b>");
        assert!(tags.iter().any(|t| t.contains("table")), "should detect <table>");
    }

    #[test]
    fn test_html_tag_detection_skips_code_blocks() {
        let input = "```\n<b>not a tag</b>\n```\noutside `<i>also not</i>` here";
        let tags = detect_html_tags(input);
        assert!(
            tags.is_empty(),
            "should not detect tags inside code blocks or inline code"
        );
    }

    #[test]
    fn test_html_to_gfm_bold() {
        let (result, n) = convert_html_to_gfm("<b>text</b>");
        assert_eq!(result, "**text**");
        assert!(n > 0);

        let (result, _) = convert_html_to_gfm("<strong>text</strong>");
        assert_eq!(result, "**text**");
    }

    #[test]
    fn test_html_to_gfm_italic() {
        let (result, n) = convert_html_to_gfm("<i>text</i>");
        assert_eq!(result, "*text*");
        assert!(n > 0);

        let (result, _) = convert_html_to_gfm("<em>text</em>");
        assert_eq!(result, "*text*");
    }

    #[test]
    fn test_html_to_gfm_link() {
        let (result, n) = convert_html_to_gfm(r#"<a href="https://example.com">text</a>"#);
        assert_eq!(result, "[text](https://example.com)");
        assert!(n > 0);
    }

    #[test]
    fn test_html_to_gfm_code() {
        let (result, n) = convert_html_to_gfm("<code>text</code>");
        assert_eq!(result, "`text`");
        assert!(n > 0);
    }

    #[test]
    fn test_html_to_gfm_br() {
        let (result, n) = convert_html_to_gfm("line1<br>line2");
        assert_eq!(result, "line1\nline2");
        assert!(n > 0);

        let (result, _) = convert_html_to_gfm("line1<br/>line2");
        assert_eq!(result, "line1\nline2");

        let (result, _) = convert_html_to_gfm("line1<br />line2");
        assert_eq!(result, "line1\nline2");
    }

    #[test]
    fn test_strip_div_span() {
        let (result, n) = convert_html_to_gfm("<div>text</div>");
        assert_eq!(result, "text");
        assert!(n > 0);

        let (result, _) = convert_html_to_gfm("<span>text</span>");
        assert_eq!(result, "text");
    }

    #[test]
    fn test_html_to_gfm_pre() {
        let (result, n) = convert_html_to_gfm("<pre>some code</pre>");
        assert_eq!(result, "```\nsome code\n```");
        assert!(n > 0);
    }

    #[test]
    fn test_html_to_gfm_hr() {
        let (result, n) = convert_html_to_gfm("<hr>");
        assert_eq!(result, "---");
        assert!(n > 0);
    }

    #[test]
    fn test_html_to_gfm_sup_sub() {
        let (result, _) = convert_html_to_gfm("<sup>text</sup>");
        assert_eq!(result, "text");

        let (result, _) = convert_html_to_gfm("<sub>text</sub>");
        assert_eq!(result, "text");
    }
}
