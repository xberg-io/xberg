//! Metadata extraction from RTF documents.

use crate::extractors::rtf::encoding::parse_rtf_control_word;
use serde_json::Value;
use std::collections::HashMap;

/// Parse a `{\\creatim ...}` or `{\\revtim ...}` RTF info block into ISO 8601 format.
pub fn parse_rtf_datetime(segment: &str) -> Option<String> {
    let mut year: Option<i32> = None;
    let mut month: Option<i32> = None;
    let mut day: Option<i32> = None;
    let mut hour: Option<i32> = None;
    let mut minute: Option<i32> = None;

    let mut chars = segment.chars().peekable();
    while let Some(&ch) = chars.peek() {
        if ch != '\\' {
            chars.next();
            continue;
        }
        chars.next();
        let (word, value) = parse_rtf_control_word(&mut chars);
        if let Some(v) = value {
            match word.as_str() {
                "yr" => year = Some(v),
                "mo" => month = Some(v),
                "dy" => day = Some(v),
                "hr" => hour = Some(v),
                "min" => minute = Some(v),
                _ => {}
            }
        }
    }

    let year = year?;
    let month = month.unwrap_or(1).max(1) as u32;
    let day = day.unwrap_or(1).max(1) as u32;
    let hour = hour.unwrap_or(0).max(0) as u32;
    let minute = minute.unwrap_or(0).max(0) as u32;

    Some(format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:00Z",
        year, month, day, hour, minute
    ))
}

/// Extract metadata from the RTF `\\info` block and augment with computed statistics.
pub fn extract_rtf_metadata(rtf_content: &str, extracted_text: &str) -> HashMap<String, Value> {
    let mut metadata: HashMap<String, Value> = HashMap::new();

    if let Some(start) = rtf_content.find("{\\info") {
        let slice = &rtf_content[start..];
        let mut depth = 0usize;
        let mut end_offset: Option<usize> = None;

        for (idx, ch) in slice.char_indices() {
            match ch {
                '{' => depth += 1,
                '}' => {
                    if depth == 0 {
                        break;
                    }
                    depth -= 1;
                    if depth == 0 {
                        end_offset = Some(idx + 1);
                        break;
                    }
                }
                _ => {}
            }
        }

        let info_block = end_offset.map(|end| &slice[..end]).unwrap_or(slice);

        let mut segments: Vec<String> = Vec::new();
        let mut seg_depth = 0usize;
        let mut current = String::new();
        let mut in_segment = false;

        for ch in info_block.chars() {
            if ch == '{' {
                seg_depth += 1;
                if seg_depth == 2 {
                    in_segment = true;
                    current.clear();
                    continue;
                }
            } else if ch == '}' {
                if seg_depth == 2 && in_segment {
                    segments.push(current.clone());
                    in_segment = false;
                }
                seg_depth = seg_depth.saturating_sub(1);
                continue;
            }

            if in_segment {
                current.push(ch);
            }
        }

        for segment in segments {
            if !segment.starts_with('\\') {
                continue;
            }

            let cleaned_segment = if segment.starts_with("\\*\\") {
                segment.replacen("\\*\\", "\\", 1)
            } else {
                segment.clone()
            };

            let mut chars = cleaned_segment.chars().peekable();
            chars.next();
            let (keyword, numeric) = parse_rtf_control_word(&mut chars);
            let remaining: String = chars.collect();
            let trimmed = remaining.trim();

            match keyword.as_str() {
                "author" => {
                    if !trimmed.is_empty() {
                        let author = trimmed.to_string();
                        metadata.insert("created_by".to_string(), Value::String(author.clone()));
                        metadata.insert("authors".to_string(), Value::Array(vec![Value::String(author)]));
                    }
                }
                "operator" => {
                    if !trimmed.is_empty() {
                        metadata.insert("modified_by".to_string(), Value::String(trimmed.to_string()));
                    }
                }
                "title" => {
                    if !trimmed.is_empty() {
                        metadata.insert("title".to_string(), Value::String(trimmed.to_string()));
                    }
                }
                "subject" => {
                    if !trimmed.is_empty() {
                        metadata.insert("subject".to_string(), Value::String(trimmed.to_string()));
                    }
                }
                "generator" => {
                    if !trimmed.is_empty() {
                        metadata.insert("generator".to_string(), Value::String(trimmed.to_string()));
                    }
                }
                "creatim" => {
                    if let Some(dt) = parse_rtf_datetime(trimmed) {
                        metadata.insert("created_at".to_string(), Value::String(dt));
                    }
                }
                "revtim" => {
                    if let Some(dt) = parse_rtf_datetime(trimmed) {
                        metadata.insert("modified_at".to_string(), Value::String(dt));
                    }
                }
                "version" => {
                    if let Some(val) = numeric.or_else(|| trimmed.parse::<i32>().ok()) {
                        metadata.insert("revision".to_string(), Value::String(val.to_string()));
                    }
                }
                "nofpages" => {
                    if let Some(val) = numeric.or_else(|| trimmed.parse::<i32>().ok()) {
                        metadata.insert("page_count".to_string(), Value::Number(val.into()));
                    }
                }
                "nofwords" => {
                    if let Some(val) = numeric.or_else(|| trimmed.parse::<i32>().ok()) {
                        metadata.insert("word_count".to_string(), Value::Number(val.into()));
                    }
                }
                "nofchars" => {
                    if let Some(val) = numeric.or_else(|| trimmed.parse::<i32>().ok()) {
                        metadata.insert("character_count".to_string(), Value::Number(val.into()));
                    }
                }
                "lines" => {
                    if let Some(val) = numeric.or_else(|| trimmed.parse::<i32>().ok()) {
                        metadata.insert("line_count".to_string(), Value::Number(val.into()));
                    }
                }
                "paragraphs" => {
                    if let Some(val) = numeric.or_else(|| trimmed.parse::<i32>().ok()) {
                        metadata.insert("paragraph_count".to_string(), Value::Number(val.into()));
                    }
                }
                _ => {}
            }
        }
    }

    let cleaned_text = extracted_text.trim();
    if !cleaned_text.is_empty() {
        let word_count = cleaned_text.split_whitespace().count() as i64;
        metadata
            .entry("word_count".to_string())
            .or_insert(Value::Number(word_count.into()));

        let character_count = cleaned_text.chars().count() as i64;
        metadata
            .entry("character_count".to_string())
            .or_insert(Value::Number(character_count.into()));

        let line_count = cleaned_text.lines().count() as i64;
        metadata
            .entry("line_count".to_string())
            .or_insert(Value::Number(line_count.into()));

        let paragraph_count = cleaned_text.split("\n\n").filter(|p| !p.trim().is_empty()).count() as i64;
        metadata
            .entry("paragraph_count".to_string())
            .or_insert(Value::Number(paragraph_count.into()));
    }

    metadata
}
