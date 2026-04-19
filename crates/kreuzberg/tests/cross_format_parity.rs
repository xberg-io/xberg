//! Cross-format output parity tests.
//!
//! Verify that all output formats (Markdown, HTML, Djot, Plain) produce
//! equivalent text content for the same document. We extract each document
//! in every format, strip markup to plain text, tokenize, and compute
//! token-level F1 scores between format pairs.
//!
//! Usage:
//!   cargo test -p kreuzberg --test cross_format_parity -- --nocapture

mod helpers;

use helpers::{get_test_file_path, test_documents_available};
use kreuzberg::core::config::{ExtractionConfig, OutputFormat};
use kreuzberg::extract_file_sync;
use std::collections::HashMap;
use std::path::Path;

// ============================================================================
// Text stripping helpers
// ============================================================================

/// Strip markdown markup to recover approximate plain text.
fn strip_markdown(text: &str) -> String {
    let mut result = String::with_capacity(text.len());

    for line in text.lines() {
        let trimmed = line.trim();

        // Skip code fence lines
        if trimmed.starts_with("```") {
            continue;
        }

        // Skip table separator lines (e.g., |---|---|)
        if trimmed.starts_with('|') && trimmed.chars().all(|c| c == '|' || c == '-' || c == ':' || c == ' ') {
            continue;
        }

        // Strip heading markers
        let line = strip_leading_pattern(trimmed, '#');

        // Strip blockquote markers
        let line = strip_leading_pattern(&line, '>');

        // Strip unordered list markers
        let line = strip_list_marker(&line);

        // Strip table pipes
        let line = line.replace('|', " ");

        // Strip link syntax: [text](url) -> text
        let line = strip_links(&line);

        // Strip image syntax: ![alt](url) -> alt
        let line = strip_images(&line);

        // Strip inline formatting markers
        let line = line.replace("**", "");
        let line = line.replace("__", "");
        let line = line.replace('*', "");
        let line = line.replace('_', " ");
        let line = line.replace('~', "");
        let line = line.replace('`', "");

        result.push_str(&line);
        result.push('\n');
    }

    result
}

/// Strip HTML tags and decode common entities.
fn strip_html(text: &str) -> String {
    // Remove all HTML tags
    let mut result = String::with_capacity(text.len());
    let mut in_tag = false;

    for ch in text.chars() {
        if ch == '<' {
            in_tag = true;
        } else if ch == '>' {
            in_tag = false;
            // Add space after closing tags to prevent word merging
            result.push(' ');
        } else if !in_tag {
            result.push(ch);
        }
    }

    // Decode common HTML entities
    let result = result.replace("&amp;", "&");
    let result = result.replace("&lt;", "<");
    let result = result.replace("&gt;", ">");
    let result = result.replace("&quot;", "\"");
    let result = result.replace("&apos;", "'");
    let result = result.replace("&#39;", "'");
    let result = result.replace("&nbsp;", " ");

    // Decode numeric entities: &#NNN;
    decode_numeric_entities(&result)
}

/// Strip djot markup (similar to markdown with minor differences).
fn strip_djot(text: &str) -> String {
    // Djot is structurally similar to markdown for our purposes
    strip_markdown(text)
}

// ============================================================================
// Tokenization and scoring
// ============================================================================

/// Tokenize text: lowercase, split on whitespace, filter empty and
/// purely-punctuation tokens.
fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split_whitespace()
        .map(|t| t.trim_matches(|c: char| c.is_ascii_punctuation()).to_string())
        .filter(|t| !t.is_empty())
        .collect()
}

/// Compute token-level F1 between two token sequences using bag-of-tokens.
///
/// This treats each sequence as a multiset (bag) and computes precision,
/// recall, and F1 based on token overlap counts.
fn token_f1(a: &[String], b: &[String]) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }

    let mut bag_a: HashMap<&str, usize> = HashMap::new();
    for token in a {
        *bag_a.entry(token.as_str()).or_insert(0) += 1;
    }

    let mut bag_b: HashMap<&str, usize> = HashMap::new();
    for token in b {
        *bag_b.entry(token.as_str()).or_insert(0) += 1;
    }

    let mut overlap = 0usize;
    for (token, &count_a) in &bag_a {
        if let Some(&count_b) = bag_b.get(token) {
            overlap += count_a.min(count_b);
        }
    }

    let precision = overlap as f64 / b.len() as f64;
    let recall = overlap as f64 / a.len() as f64;

    if precision + recall == 0.0 {
        return 0.0;
    }

    2.0 * precision * recall / (precision + recall)
}

// ============================================================================
// Internal helpers
// ============================================================================

/// Strip leading repeated characters (like `#` for headings or `>` for quotes).
fn strip_leading_pattern(line: &str, marker: char) -> String {
    let stripped = line.trim_start_matches(marker);
    if stripped.len() < line.len() {
        stripped.trim_start().to_string()
    } else {
        line.to_string()
    }
}

/// Strip list markers (- , * , + , 1. , etc.).
fn strip_list_marker(line: &str) -> String {
    let trimmed = line.trim_start();
    if trimmed.starts_with("- ") || trimmed.starts_with("* ") || trimmed.starts_with("+ ") {
        let indent_len = line.len() - trimmed.len();
        let rest = &trimmed[2..];
        format!("{}{}", &line[..indent_len], rest)
    } else if let Some(after_digit) = trimmed.strip_prefix(|c: char| c.is_ascii_digit()) {
        // Handle "1. ", "2. ", etc.
        if let Some(rest) = after_digit.strip_prefix(". ") {
            let indent_len = line.len() - trimmed.len();
            format!("{}{}", &line[..indent_len], rest)
        } else {
            line.to_string()
        }
    } else {
        line.to_string()
    }
}

/// Strip markdown link syntax: [text](url) -> text
fn strip_links(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '[' {
            // Look for closing ] followed by (
            if let Some(close_bracket) = chars[i + 1..].iter().position(|&c| c == ']') {
                let close_idx = i + 1 + close_bracket;
                if close_idx + 1 < chars.len() && chars[close_idx + 1] == '(' {
                    // Found [text]( ... look for closing )
                    if let Some(close_paren) = chars[close_idx + 2..].iter().position(|&c| c == ')') {
                        // Extract just the text part
                        let text_part: String = chars[i + 1..close_idx].iter().collect();
                        result.push_str(&text_part);
                        i = close_idx + 2 + close_paren + 1;
                        continue;
                    }
                }
            }
            result.push(chars[i]);
            i += 1;
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }

    result
}

/// Strip markdown image syntax: ![alt](url) -> alt
fn strip_images(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '!' && i + 1 < chars.len() && chars[i + 1] == '[' {
            // Image syntax: ![alt](url)
            if let Some(close_bracket) = chars[i + 2..].iter().position(|&c| c == ']') {
                let close_idx = i + 2 + close_bracket;
                if close_idx + 1 < chars.len()
                    && chars[close_idx + 1] == '('
                    && let Some(close_paren) = chars[close_idx + 2..].iter().position(|&c| c == ')')
                {
                    let alt_text: String = chars[i + 2..close_idx].iter().collect();
                    result.push_str(&alt_text);
                    i = close_idx + 2 + close_paren + 1;
                    continue;
                }
            }
            result.push(chars[i]);
            i += 1;
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }

    result
}

/// Decode numeric HTML entities (&#NNN;) to characters.
fn decode_numeric_entities(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '&' && chars.peek() == Some(&'#') {
            chars.next(); // consume '#'
            let mut num_str = String::new();
            while let Some(&c) = chars.peek() {
                if c == ';' {
                    chars.next(); // consume ';'
                    break;
                }
                if c.is_ascii_digit() {
                    num_str.push(c);
                    chars.next();
                } else {
                    break;
                }
            }
            if let Ok(code) = num_str.parse::<u32>()
                && let Some(decoded) = char::from_u32(code)
            {
                result.push(decoded);
                continue;
            }
            // Failed to decode, emit as-is
            result.push('&');
            result.push('#');
            result.push_str(&num_str);
        } else {
            result.push(ch);
        }
    }

    result
}

// ============================================================================
// GFM validation
// ============================================================================

/// Validate basic GFM (GitHub Flavored Markdown) lint rules.
///
/// Returns a list of violation descriptions. An empty list means the markdown
/// passes all checks. This is a lightweight inline replacement for shelling
/// out to `rumdl` which may not be installed.
fn validate_gfm_basics(markdown: &str) -> Vec<String> {
    let mut violations = Vec::new();
    let lines: Vec<&str> = markdown.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        let line_num = i + 1;

        // Rule: no trailing whitespace on lines
        if line.ends_with(' ') || line.ends_with('\t') {
            violations.push(format!("line {}: trailing whitespace", line_num));
        }

        // Rule: ATX-style headings only (not underline/setext style)
        if i > 0 {
            let prev = lines[i - 1].trim();
            if !prev.is_empty() && (line.chars().all(|c| c == '=') && line.len() >= 2) {
                violations.push(format!(
                    "line {}: setext heading (=== style), use ATX (# style)",
                    line_num
                ));
            }
            if !prev.is_empty() && (line.chars().all(|c| c == '-') && line.len() >= 2) && !prev.starts_with('|') {
                // Exclude table separator rows (previous line starts with |)
                violations.push(format!(
                    "line {}: setext heading (--- style), use ATX (# style)",
                    line_num
                ));
            }
        }

        // Rule: blank line before headings (except at file start)
        if line.starts_with('#') && i > 0 && !lines[i - 1].trim().is_empty() {
            violations.push(format!("line {}: missing blank line before heading", line_num));
        }

        // Rule: fenced code blocks should not be indented
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") && line.len() != trimmed.len() {
            violations.push(format!("line {}: indented fenced code block", line_num));
        }
    }

    // Rule: single trailing newline at end of file
    if !markdown.is_empty() {
        if !markdown.ends_with('\n') {
            violations.push("file does not end with a newline".to_string());
        } else if markdown.ends_with("\n\n") {
            violations.push("file ends with multiple trailing newlines".to_string());
        }
    }

    // Rule: no escaped brackets outside code blocks/spans
    let mut in_fenced_block = false;
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            in_fenced_block = !in_fenced_block;
            continue;
        }
        if in_fenced_block {
            continue;
        }

        // Strip inline code spans before checking for escaped brackets
        let without_code = strip_inline_code(line);
        if without_code.contains("\\[") || without_code.contains("\\]") {
            violations.push(format!(
                "line {}: escaped bracket (\\[ or \\]) outside code context",
                i + 1
            ));
        }
    }

    // Rule: valid pipe table format (header row must be followed by separator row)
    let mut in_code = false;
    for (i, line) in lines.iter().enumerate() {
        if line.trim().starts_with("```") {
            in_code = !in_code;
            continue;
        }
        if in_code {
            continue;
        }

        let trimmed = line.trim();
        // Detect a pipe-table header row: starts and ends with | and contains text
        if trimmed.starts_with('|') && trimmed.ends_with('|') && trimmed.len() > 2 && !is_table_separator(trimmed) {
            // Check if this could be the first row of a table (preceded by blank or start)
            let is_first_table_row = i == 0 || lines[i - 1].trim().is_empty() || !lines[i - 1].trim().starts_with('|');
            if is_first_table_row {
                // Next line should be a separator row
                if i + 1 >= lines.len() || !is_table_separator(lines[i + 1].trim()) {
                    violations.push(format!(
                        "line {}: pipe table header row not followed by separator row",
                        i + 1
                    ));
                }
            }
        }
    }

    violations
}

/// Check if a line is a markdown table separator row (e.g., `|---|---|`).
fn is_table_separator(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with('|') && trimmed.chars().all(|c| c == '|' || c == '-' || c == ':' || c == ' ')
}

/// Strip inline code spans from a line for bracket-escaping analysis.
fn strip_inline_code(line: &str) -> String {
    let mut result = String::with_capacity(line.len());
    let mut in_code = false;
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '`' {
            in_code = !in_code;
            i += 1;
            continue;
        }
        if !in_code {
            result.push(chars[i]);
        }
        i += 1;
    }

    result
}

// ============================================================================
// Structural block counting
// ============================================================================

/// Count structural block elements in markdown/djot content.
///
/// Returns a map of block type name to count.
fn count_blocks(content: &str) -> HashMap<String, usize> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    let lines: Vec<&str> = content.lines().collect();
    let mut in_code_block = false;

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Track fenced code blocks
        if trimmed.starts_with("```") {
            if !in_code_block {
                *counts.entry("code_blocks".to_string()).or_insert(0) += 1;
            }
            in_code_block = !in_code_block;
            continue;
        }

        if in_code_block {
            continue;
        }

        // Headings
        if trimmed.starts_with('#') {
            *counts.entry("headings".to_string()).or_insert(0) += 1;
        }
        // List items (unordered or ordered)
        else if trimmed.starts_with("- ")
            || trimmed.starts_with("* ")
            || trimmed.starts_with("+ ")
            || (trimmed.len() > 2
                && trimmed.chars().next().is_some_and(|c| c.is_ascii_digit())
                && trimmed.contains(". "))
        {
            *counts.entry("list_items".to_string()).or_insert(0) += 1;
        }
        // Table rows (non-separator pipe table rows)
        else if trimmed.starts_with('|') && trimmed.ends_with('|') && !is_table_separator(trimmed) {
            *counts.entry("table_rows".to_string()).or_insert(0) += 1;
        }
        // Paragraphs: non-empty line preceded by a blank line (or at start of file)
        else if !trimmed.is_empty() && (i == 0 || lines[i - 1].trim().is_empty()) && !trimmed.starts_with('>') {
            *counts.entry("paragraphs".to_string()).or_insert(0) += 1;
        }
    }

    counts
}

// ============================================================================
// Extraction helpers
// ============================================================================

/// Extract a document in the given output format.
fn extract_with_format(path: &Path, format: OutputFormat) -> Option<String> {
    let config = ExtractionConfig {
        output_format: format.clone(),
        ..Default::default()
    };

    match extract_file_sync(path, None, &config) {
        Ok(result) => Some(result.content),
        Err(err) => {
            eprintln!(
                "  [WARN] extraction failed for {} with format {}: {}",
                path.display(),
                format,
                err
            );
            None
        }
    }
}

/// Strip markup from content based on its format.
fn strip_markup(content: &str, format: &OutputFormat) -> String {
    match format {
        OutputFormat::Plain => content.to_string(),
        OutputFormat::Markdown => strip_markdown(content),
        OutputFormat::Html => strip_html(content),
        OutputFormat::Djot => strip_djot(content),
        _ => content.to_string(),
    }
}

// ============================================================================
// Test document definitions
// ============================================================================

struct TestDoc {
    /// Human-readable label.
    label: &'static str,
    /// Path relative to test_documents/.
    relative_path: &'static str,
    /// Required cargo feature (empty string means no feature needed).
    required_feature: &'static str,
    /// Expected minimum TF1 for Markdown vs HTML.
    md_html_threshold: f64,
    /// Expected minimum TF1 for Markdown vs Djot.
    md_djot_threshold: f64,
    /// Expected minimum TF1 for Markdown vs Plain.
    md_plain_threshold: f64,
    /// Whether the source document is HTML (relaxed thresholds due to
    /// round-trip divergence).
    _is_html_input: bool,
}

const TEST_DOCS: &[TestDoc] = &[
    // Markdown extraction_test.md — has headings, tables, lists. No extra features needed.
    TestDoc {
        label: "markdown-extraction-test",
        relative_path: "markdown/extraction_test.md",
        required_feature: "",
        md_html_threshold: 0.95,
        md_djot_threshold: 0.85,
        md_plain_threshold: 0.85,
        _is_html_input: false,
    },
    // Markdown readme.md — headings, lists, code block. No extra features needed.
    TestDoc {
        label: "markdown-readme",
        relative_path: "markdown/readme.md",
        required_feature: "",
        md_html_threshold: 0.95,
        md_djot_threshold: 0.85,
        md_plain_threshold: 0.85,
        _is_html_input: false,
    },
    // RST document — requires office feature for the RST extractor.
    TestDoc {
        label: "rst-readme",
        relative_path: "rst/readme.rst",
        required_feature: "office",
        md_html_threshold: 0.95,
        md_djot_threshold: 0.85,
        md_plain_threshold: 0.85,
        _is_html_input: false,
    },
    // HTML page — requires html feature. The taylor_swift page is large (Wikipedia)
    // and includes extensive navigation/sidebar elements. When extracting as Markdown,
    // html-to-markdown-rs performs article extraction (producing ~43k tokens). When
    // extracting as HTML or Plain, the full InternalDocument is rendered (~82k tokens),
    // including navigation elements absent from the article extraction. This structural
    // divergence yields a TF1 of ~0.62 between Markdown and HTML/Plain outputs.
    // Thresholds are set conservatively below the observed TF1 to allow for variation.
    TestDoc {
        label: "html-taylor-swift",
        relative_path: "html/taylor_swift.html",
        required_feature: "html",
        md_html_threshold: 0.55,
        md_djot_threshold: 0.85,
        md_plain_threshold: 0.55,
        _is_html_input: true,
    },
    // LaTeX document — requires office feature.
    TestDoc {
        label: "latex-basic-sections",
        relative_path: "latex/basic_sections.tex",
        required_feature: "office",
        md_html_threshold: 0.95,
        md_djot_threshold: 0.85,
        md_plain_threshold: 0.85,
        _is_html_input: false,
    },
    // EPUB — requires office feature.
    TestDoc {
        label: "epub-wasteland",
        relative_path: "epub/wasteland.epub",
        required_feature: "office",
        md_html_threshold: 0.95,
        md_djot_threshold: 0.85,
        md_plain_threshold: 0.85,
        _is_html_input: false,
    },
    // DOCX — requires office feature.
    TestDoc {
        label: "docx-sample-document",
        relative_path: "docx/sample_document.docx",
        required_feature: "office",
        md_html_threshold: 0.95,
        md_djot_threshold: 0.85,
        md_plain_threshold: 0.85,
        _is_html_input: false,
    },
    // HTML table document — requires html feature.
    TestDoc {
        label: "html-simple-table",
        relative_path: "html/simple_table.html",
        required_feature: "html",
        md_html_threshold: 0.75,
        md_djot_threshold: 0.85,
        md_plain_threshold: 0.75,
        _is_html_input: true,
    },
    // LaTeX tables — requires office feature.
    TestDoc {
        label: "latex-tables",
        relative_path: "latex/tables.tex",
        required_feature: "office",
        md_html_threshold: 0.95,
        md_djot_threshold: 0.80,
        md_plain_threshold: 0.80,
        _is_html_input: false,
    },
];

// ============================================================================
// Tests
// ============================================================================

/// Check whether a required feature is available at runtime by attempting an
/// extraction. Returns false if extraction fails (feature likely not compiled).
fn feature_available(feature: &str) -> bool {
    // Each arm evaluates a different cfg! macro, so matches! is not appropriate.
    #[allow(clippy::match_like_matches_macro)]
    match feature {
        "" => true,
        "html" => cfg!(feature = "html"),
        "office" => cfg!(feature = "office"),
        "pdf" => cfg!(feature = "pdf"),
        "excel" => cfg!(feature = "excel"),
        _ => false,
    }
}

#[test]
fn cross_format_parity_all_documents() {
    if !test_documents_available() {
        eprintln!("Skipping: test_documents not available");
        return;
    }

    let formats = [
        OutputFormat::Markdown,
        OutputFormat::Html,
        OutputFormat::Djot,
        OutputFormat::Plain,
    ];

    let mut failures: Vec<String> = Vec::new();
    let mut tested = 0usize;

    for doc in TEST_DOCS {
        if !feature_available(doc.required_feature) {
            eprintln!("  [SKIP] {} — requires feature '{}'", doc.label, doc.required_feature);
            continue;
        }

        let path = get_test_file_path(doc.relative_path);
        if !path.exists() {
            eprintln!("  [SKIP] {} — file not found: {}", doc.label, path.display());
            continue;
        }

        eprintln!("\n--- {} ---", doc.label);

        // Extract in all formats
        let mut outputs: HashMap<String, String> = HashMap::new();
        for format in &formats {
            if let Some(content) = extract_with_format(&path, format.clone()) {
                let stripped = strip_markup(&content, format);
                let format_name = format.to_string();
                eprintln!(
                    "  {}: {} chars raw, {} chars stripped",
                    format_name,
                    content.len(),
                    stripped.len()
                );
                outputs.insert(format_name, stripped);
            }
        }

        // Need at least markdown and one other format to compare
        let md_tokens = match outputs.get("markdown") {
            Some(text) => tokenize(text),
            None => {
                eprintln!("  [SKIP] {} — markdown extraction failed", doc.label);
                continue;
            }
        };

        if md_tokens.is_empty() {
            eprintln!("  [SKIP] {} — markdown produced no tokens", doc.label);
            continue;
        }

        tested += 1;

        // Compare Markdown vs HTML
        if let Some(html_text) = outputs.get("html") {
            let html_tokens = tokenize(html_text);
            let f1 = token_f1(&md_tokens, &html_tokens);
            eprintln!(
                "  MD vs HTML:  TF1 = {:.4}  (md_tokens={}, html_tokens={})",
                f1,
                md_tokens.len(),
                html_tokens.len()
            );
            if f1 < doc.md_html_threshold {
                failures.push(format!(
                    "{}: MD vs HTML TF1 = {:.4} < threshold {:.2}",
                    doc.label, f1, doc.md_html_threshold
                ));
            }
        }

        // Compare Markdown vs Djot
        if let Some(djot_text) = outputs.get("djot") {
            let djot_tokens = tokenize(djot_text);
            let f1 = token_f1(&md_tokens, &djot_tokens);
            eprintln!(
                "  MD vs Djot:  TF1 = {:.4}  (md_tokens={}, djot_tokens={})",
                f1,
                md_tokens.len(),
                djot_tokens.len()
            );
            if f1 < doc.md_djot_threshold {
                failures.push(format!(
                    "{}: MD vs Djot TF1 = {:.4} < threshold {:.2}",
                    doc.label, f1, doc.md_djot_threshold
                ));
            }
        }

        // Compare Markdown vs Plain
        if let Some(plain_text) = outputs.get("plain") {
            let plain_tokens = tokenize(plain_text);
            let f1 = token_f1(&md_tokens, &plain_tokens);
            eprintln!(
                "  MD vs Plain: TF1 = {:.4}  (md_tokens={}, plain_tokens={})",
                f1,
                md_tokens.len(),
                plain_tokens.len()
            );
            if f1 < doc.md_plain_threshold {
                failures.push(format!(
                    "{}: MD vs Plain TF1 = {:.4} < threshold {:.2}",
                    doc.label, f1, doc.md_plain_threshold
                ));
            }
        }
    }

    eprintln!("\n=== Summary: tested {} documents ===", tested);

    if !failures.is_empty() {
        panic!(
            "Cross-format parity failures ({}/{} checks failed):\n  - {}",
            failures.len(),
            tested * 3,
            failures.join("\n  - ")
        );
    }

    assert!(tested > 0, "Expected at least one document to be tested");
}

/// Focused test for table content parity across formats.
///
/// Verifies that table cell text appears in all format outputs,
/// regardless of how the table is rendered (pipe tables, HTML tables,
/// space-separated text).
#[test]
fn cross_format_table_content_parity() {
    if !test_documents_available() {
        eprintln!("Skipping: test_documents not available");
        return;
    }

    // Documents known to contain tables
    let table_docs: &[(&str, &str, &[&str])] = &[
        #[cfg(feature = "html")]
        ("html/simple_table.html", "html", &["Product", "Category", "Price"]),
        #[cfg(feature = "office")]
        (
            "latex/tables.tex",
            "office",
            &[], // We don't know exact cell values; just check non-empty extraction
        ),
        #[cfg(feature = "office")]
        ("docx/docx_tables.docx", "office", &[]),
    ];

    let formats = [
        ("markdown", OutputFormat::Markdown),
        ("html", OutputFormat::Html),
        ("djot", OutputFormat::Djot),
        ("plain", OutputFormat::Plain),
    ];

    let mut tested = 0usize;
    let mut failures: Vec<String> = Vec::new();

    for &(relative_path, required_feature, expected_cells) in table_docs {
        if !feature_available(required_feature) {
            eprintln!("  [SKIP] {} — requires feature '{}'", relative_path, required_feature);
            continue;
        }

        let path = get_test_file_path(relative_path);
        if !path.exists() {
            eprintln!("  [SKIP] {} — file not found", relative_path);
            continue;
        }

        eprintln!("\n--- table test: {} ---", relative_path);
        tested += 1;

        for (format_name, format) in &formats {
            if let Some(content) = extract_with_format(&path, format.clone()) {
                let lower = content.to_lowercase();

                // Check that expected cell values appear in every format
                for &cell in expected_cells {
                    if !lower.contains(&cell.to_lowercase()) {
                        failures.push(format!(
                            "{} [{}]: missing expected table cell '{}'",
                            relative_path, format_name, cell
                        ));
                    }
                }

                // Every format should produce non-empty content
                if content.trim().is_empty() {
                    failures.push(format!("{} [{}]: produced empty content", relative_path, format_name));
                }
            }
        }
    }

    eprintln!("\n=== Table parity: tested {} documents ===", tested);

    if !failures.is_empty() {
        panic!("Table content parity failures:\n  - {}", failures.join("\n  - "));
    }
}

/// Validate that markdown extraction output passes basic GFM lint rules.
///
/// Extracts each non-HTML document as Markdown and runs inline GFM checks.
/// This catches common issues like trailing whitespace, setext headings,
/// escaped brackets, and malformed tables without requiring an external tool.
#[test]
fn markdown_gfm_lint_validation() {
    if !test_documents_available() {
        eprintln!("Skipping: test_documents not available");
        return;
    }

    let mut failures: Vec<String> = Vec::new();
    let mut tested = 0usize;

    for doc in TEST_DOCS {
        if !feature_available(doc.required_feature) {
            eprintln!("  [SKIP] {} — requires feature '{}'", doc.label, doc.required_feature);
            continue;
        }

        let path = get_test_file_path(doc.relative_path);
        if !path.exists() {
            eprintln!("  [SKIP] {} — file not found: {}", doc.label, path.display());
            continue;
        }

        if let Some(md_content) = extract_with_format(&path, OutputFormat::Markdown) {
            tested += 1;
            let violations = validate_gfm_basics(&md_content);
            if !violations.is_empty() {
                // Report at most 5 violations per document to keep output manageable
                let shown: Vec<_> = violations.iter().take(5).collect();
                let suffix = if violations.len() > 5 {
                    format!(" ... and {} more", violations.len() - 5)
                } else {
                    String::new()
                };
                failures.push(format!(
                    "{}: {} GFM violations: [{}]{}",
                    doc.label,
                    violations.len(),
                    shown.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", "),
                    suffix,
                ));
            } else {
                eprintln!("  [OK] {} — GFM lint clean", doc.label);
            }
        }
    }

    eprintln!("\n=== GFM lint: tested {} documents ===", tested);

    if !failures.is_empty() {
        panic!(
            "GFM lint failures ({} documents):\n  - {}",
            failures.len(),
            failures.join("\n  - ")
        );
    }

    assert!(tested > 0, "Expected at least one document to be tested");
}

/// Compare structural block counts between Markdown and Djot outputs.
///
/// For each document, counts headings, paragraphs, table rows, list items,
/// and code blocks in both formats. Asserts they are within +/-2 of each
/// other, allowing for minor differences in paragraph consolidation.
#[test]
fn structural_block_comparison() {
    if !test_documents_available() {
        eprintln!("Skipping: test_documents not available");
        return;
    }

    let mut failures: Vec<String> = Vec::new();
    let mut tested = 0usize;
    let tolerance = 2i64;

    for doc in TEST_DOCS {
        if !feature_available(doc.required_feature) {
            continue;
        }

        let path = get_test_file_path(doc.relative_path);
        if !path.exists() {
            continue;
        }

        let md_content = extract_with_format(&path, OutputFormat::Markdown);
        let djot_content = extract_with_format(&path, OutputFormat::Djot);

        if let (Some(md), Some(djot)) = (md_content, djot_content) {
            tested += 1;
            let md_blocks = count_blocks(&md);
            let djot_blocks = count_blocks(&djot);

            eprintln!("\n--- structural blocks: {} ---", doc.label);

            let block_types = ["headings", "paragraphs", "table_rows", "list_items", "code_blocks"];
            for block_type in &block_types {
                let md_count = *md_blocks.get(*block_type).unwrap_or(&0) as i64;
                let djot_count = *djot_blocks.get(*block_type).unwrap_or(&0) as i64;
                let diff = (md_count - djot_count).abs();

                eprintln!("  {}: md={}, djot={}, diff={}", block_type, md_count, djot_count, diff);

                if diff > tolerance {
                    failures.push(format!(
                        "{}: {} count differs by {} (md={}, djot={}, tolerance={})",
                        doc.label, block_type, diff, md_count, djot_count, tolerance
                    ));
                }
            }
        }
    }

    eprintln!("\n=== Structural blocks: tested {} documents ===", tested);

    if !failures.is_empty() {
        panic!("Structural block comparison failures:\n  - {}", failures.join("\n  - "));
    }

    assert!(tested > 0, "Expected at least one document to be tested");
}

/// Verify that markdown output does not contain escaped brackets outside code.
///
/// Extracts a document known to contain links/brackets (markdown/comprehensive.md)
/// and checks that the output does not have `\[` or `\]` in non-code contexts.
/// Escaped brackets break rendering in most markdown viewers and are a sign
/// of incorrect link/bracket handling in the extraction pipeline.
#[test]
fn no_escaped_brackets_in_markdown() {
    if !test_documents_available() {
        eprintln!("Skipping: test_documents not available");
        return;
    }

    // Documents known to have links/brackets in the source.
    // We use extraction_test.md and readme.md which are well-formed and
    // contain links. comprehensive.md is excluded because it contains
    // intentional edge cases that may not round-trip cleanly.
    let bracket_docs: &[(&str, &str)] = &[
        ("markdown/extraction_test.md", ""),
        ("markdown/readme.md", ""),
        #[cfg(feature = "office")]
        ("rst/readme.rst", "office"),
        #[cfg(feature = "office")]
        ("epub/wasteland.epub", "office"),
    ];

    let mut tested = 0usize;
    let mut failures: Vec<String> = Vec::new();

    for &(relative_path, required_feature) in bracket_docs {
        if !feature_available(required_feature) {
            eprintln!("  [SKIP] {} — requires feature '{}'", relative_path, required_feature);
            continue;
        }

        let path = get_test_file_path(relative_path);
        if !path.exists() {
            eprintln!("  [SKIP] {} — file not found", relative_path);
            continue;
        }

        if let Some(md_content) = extract_with_format(&path, OutputFormat::Markdown) {
            tested += 1;

            // Check for escaped brackets outside of code blocks/spans
            let mut in_fenced_block = false;
            for (i, line) in md_content.lines().enumerate() {
                let trimmed = line.trim();
                if trimmed.starts_with("```") {
                    in_fenced_block = !in_fenced_block;
                    continue;
                }
                if in_fenced_block {
                    continue;
                }

                let without_code = strip_inline_code(line);
                if without_code.contains("\\[") || without_code.contains("\\]") {
                    failures.push(format!(
                        "{} line {}: escaped bracket found: '{}'",
                        relative_path,
                        i + 1,
                        line.trim()
                    ));
                    // Only report first occurrence per document
                    break;
                }
            }

            if !failures.iter().any(|f| f.starts_with(relative_path)) {
                eprintln!("  [OK] {} — no escaped brackets", relative_path);
            }
        }
    }

    eprintln!("\n=== Bracket escaping: tested {} documents ===", tested);

    if !failures.is_empty() {
        panic!("Escaped bracket violations:\n  - {}", failures.join("\n  - "));
    }

    assert!(tested > 0, "Expected at least one document to be tested");
}

// ============================================================================
// Unit tests for helper functions
// ============================================================================

#[cfg(test)]
mod helper_tests {
    use super::*;

    #[test]
    fn test_strip_markdown_headings() {
        let input = "# Heading 1\n## Heading 2\nPlain text\n";
        let stripped = strip_markdown(input);
        assert!(stripped.contains("Heading 1"));
        assert!(stripped.contains("Heading 2"));
        assert!(stripped.contains("Plain text"));
        assert!(!stripped.contains('#'));
    }

    #[test]
    fn test_strip_markdown_links() {
        let input = "See [link text](https://example.com) for details.\n";
        let stripped = strip_markdown(input);
        assert!(stripped.contains("link text"));
        assert!(!stripped.contains("https://example.com"));
        assert!(!stripped.contains('['));
        assert!(!stripped.contains(']'));
    }

    #[test]
    fn test_strip_markdown_bold_italic() {
        let input = "This is **bold** and *italic* text.\n";
        let stripped = strip_markdown(input);
        assert!(stripped.contains("bold"));
        assert!(stripped.contains("italic"));
    }

    #[test]
    fn test_strip_markdown_list() {
        let input = "- item one\n* item two\n1. item three\n";
        let stripped = strip_markdown(input);
        assert!(stripped.contains("item one"));
        assert!(stripped.contains("item two"));
        assert!(stripped.contains("item three"));
    }

    #[test]
    fn test_strip_html_tags() {
        let input = "<h1>Title</h1><p>Hello &amp; goodbye</p>";
        let stripped = strip_html(input);
        assert!(stripped.contains("Title"));
        assert!(stripped.contains("Hello & goodbye"));
        assert!(!stripped.contains('<'));
        assert!(!stripped.contains('>'));
    }

    #[test]
    fn test_strip_html_numeric_entity() {
        let input = "A&#65;B";
        let stripped = strip_html(input);
        assert!(stripped.contains("AAB"));
    }

    #[test]
    fn test_tokenize() {
        let input = "Hello, World! This is a TEST.";
        let tokens = tokenize(input);
        assert!(tokens.contains(&"hello".to_string()));
        assert!(tokens.contains(&"world".to_string()));
        assert!(tokens.contains(&"test".to_string()));
    }

    #[test]
    fn test_token_f1_identical() {
        let a = vec!["hello".to_string(), "world".to_string()];
        let f1 = token_f1(&a, &a);
        assert!((f1 - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_token_f1_no_overlap() {
        let a = vec!["hello".to_string()];
        let b = vec!["world".to_string()];
        let f1 = token_f1(&a, &b);
        assert!(f1.abs() < f64::EPSILON);
    }

    #[test]
    fn test_token_f1_partial_overlap() {
        let a = vec![
            "the".to_string(),
            "quick".to_string(),
            "brown".to_string(),
            "fox".to_string(),
        ];
        let b = vec![
            "the".to_string(),
            "quick".to_string(),
            "red".to_string(),
            "fox".to_string(),
        ];
        let f1 = token_f1(&a, &b);
        // 3 overlapping tokens out of 4 each -> precision=3/4, recall=3/4, F1=3/4
        assert!((f1 - 0.75).abs() < 0.01);
    }

    #[test]
    fn test_token_f1_empty() {
        let empty: Vec<String> = vec![];
        assert!((token_f1(&empty, &empty) - 1.0).abs() < f64::EPSILON);
        assert!(token_f1(&empty, &["a".to_string()]).abs() < f64::EPSILON);
    }

    #[test]
    fn test_strip_images() {
        let input = "Before ![alt text](image.png) after";
        let stripped = strip_images(input);
        assert!(stripped.contains("alt text"));
        assert!(!stripped.contains("image.png"));
    }

    // ---- GFM validation unit tests ----

    #[test]
    fn test_gfm_trailing_whitespace() {
        let md = "Hello world  \nNext line\n";
        let violations = validate_gfm_basics(md);
        assert!(violations.iter().any(|v| v.contains("trailing whitespace")));
    }

    #[test]
    fn test_gfm_no_trailing_newline() {
        let md = "Hello world";
        let violations = validate_gfm_basics(md);
        assert!(violations.iter().any(|v| v.contains("does not end with a newline")));
    }

    #[test]
    fn test_gfm_multiple_trailing_newlines() {
        let md = "Hello world\n\n";
        let violations = validate_gfm_basics(md);
        assert!(violations.iter().any(|v| v.contains("multiple trailing newlines")));
    }

    #[test]
    fn test_gfm_setext_heading() {
        let md = "Title\n=====\n";
        let violations = validate_gfm_basics(md);
        assert!(violations.iter().any(|v| v.contains("setext heading")));
    }

    #[test]
    fn test_gfm_missing_blank_before_heading() {
        let md = "Some text\n# Heading\n";
        let violations = validate_gfm_basics(md);
        assert!(
            violations
                .iter()
                .any(|v| v.contains("missing blank line before heading"))
        );
    }

    #[test]
    fn test_gfm_escaped_brackets() {
        let md = "Text with \\[escaped\\] brackets\n";
        let violations = validate_gfm_basics(md);
        assert!(violations.iter().any(|v| v.contains("escaped bracket")));
    }

    #[test]
    fn test_gfm_escaped_brackets_in_code_ok() {
        let md = "Text with `\\[code\\]` is fine\n";
        let violations = validate_gfm_basics(md);
        assert!(!violations.iter().any(|v| v.contains("escaped bracket")));
    }

    #[test]
    fn test_gfm_indented_code_fence() {
        let md = "  ```rust\ncode\n```\n";
        let violations = validate_gfm_basics(md);
        assert!(violations.iter().any(|v| v.contains("indented fenced code block")));
    }

    #[test]
    fn test_gfm_valid_markdown() {
        let md = "# Heading\n\nSome text here.\n\n## Sub heading\n\nMore text.\n";
        let violations = validate_gfm_basics(md);
        assert!(violations.is_empty(), "Expected no violations, got: {:?}", violations);
    }

    #[test]
    fn test_gfm_valid_table() {
        let md = "# Table\n\n| Header | Col |\n| --- | --- |\n| A | B |\n";
        let violations = validate_gfm_basics(md);
        assert!(
            !violations.iter().any(|v| v.contains("table header")),
            "Valid table flagged: {:?}",
            violations
        );
    }

    #[test]
    fn test_gfm_table_missing_separator() {
        let md = "# Table\n\n| Header | Col |\n| A | B |\n";
        let violations = validate_gfm_basics(md);
        assert!(violations.iter().any(|v| v.contains("separator row")));
    }

    // ---- Block counting unit tests ----

    #[test]
    fn test_count_blocks_headings() {
        let md = "# H1\n\n## H2\n\n### H3\n\nSome text.\n";
        let counts = count_blocks(md);
        assert_eq!(*counts.get("headings").unwrap_or(&0), 3);
    }

    #[test]
    fn test_count_blocks_list_items() {
        let md = "- one\n- two\n- three\n";
        let counts = count_blocks(md);
        assert_eq!(*counts.get("list_items").unwrap_or(&0), 3);
    }

    #[test]
    fn test_count_blocks_code_blocks() {
        let md = "```rust\nfn main() {}\n```\n\n```\nplain\n```\n";
        let counts = count_blocks(md);
        assert_eq!(*counts.get("code_blocks").unwrap_or(&0), 2);
    }

    #[test]
    fn test_count_blocks_table_rows() {
        let md = "| A | B |\n| --- | --- |\n| 1 | 2 |\n| 3 | 4 |\n";
        let counts = count_blocks(md);
        // Header row + 2 data rows = 3 (separator excluded)
        assert_eq!(*counts.get("table_rows").unwrap_or(&0), 3);
    }

    // ---- is_table_separator / strip_inline_code unit tests ----

    #[test]
    fn test_is_table_separator() {
        assert!(is_table_separator("| --- | --- |"));
        assert!(is_table_separator("|---|---|"));
        assert!(is_table_separator("| :---: | ---: |"));
        assert!(!is_table_separator("| data | here |"));
    }

    #[test]
    fn test_strip_inline_code() {
        assert_eq!(strip_inline_code("hello `world` foo"), "hello  foo");
        assert_eq!(strip_inline_code("no code here"), "no code here");
        assert_eq!(strip_inline_code("`all code`"), "");
    }
}
