//! Redaction engine: orchestrates pattern matching, optional NER, span merging,
//! and the destructive rewrite of every textual field on [`ExtractedDocument`].
//!
//! The engine is invoked from the Late-stage post-processor at
//! [`crate::plugins::processor::builtin::redaction`].

use std::collections::HashSet;

use crate::Result;
use crate::core::config::redaction::RedactionConfig;
use crate::types::ExtractedDocument;
use crate::types::redaction::{PiiCategory, RedactionFinding, RedactionReport};

use super::patterns::{PatternMatch, scan_text};
use super::strategy::{TokenCounter, apply_strategy};

/// Run pattern redaction (and optional NER-driven redaction) over `result` and
/// rewrite every textual field. Populates `result.redaction_report`.
pub async fn redact(result: &mut ExtractedDocument, config: &RedactionConfig) -> Result<()> {
    // Validate user-supplied terms/patterns up front so the engine never tries to
    // compile a malformed regex mid-pipeline.
    config.validate()?;
    let active_categories = active_categories(config);
    // Compile every user-supplied term + pattern ONCE here so chunk / formatted /
    // entity scans reuse the same regex objects — avoids O(chunks × terms)
    // compilations on long documents.
    let custom_regexes = compile_custom(config);

    // 1. Pattern-engine matches on the original content.
    let categories_vec: Vec<PiiCategory> = active_categories.iter().cloned().collect();
    let mut matches = scan_text(&result.content, &categories_vec);

    // 1b. User-supplied literal terms and regex patterns.
    matches.extend(scan_custom(&result.content, &custom_regexes));

    // 2. Optional NER matches for Person / Organization / Location.
    #[cfg(feature = "ner")]
    if let Some(ner_config) = &config.ner {
        let ner_matches = collect_ner_matches(&result.content, ner_config, &active_categories).await?;
        matches.extend(ner_matches);
    }
    // Suppress unused-binding warning when `ner` is off (we still read the field for offset
    // computations elsewhere on Late stage callers).
    #[cfg(not(feature = "ner"))]
    let _ = &active_categories;

    // 3. Filter to only the configured categories (if any were specified).
    //    Custom-category hits (`custom_terms` / `custom_patterns`) are always
    //    retained — the user added them explicitly, the category filter is for
    //    pruning the engine's built-in detectors.
    if !config.categories.is_empty() {
        matches.retain(|m| matches!(m.category, PiiCategory::Custom(_)) || config.categories.contains(&m.category));
    }

    // 4. Resolve overlaps: prefer earlier match; if equal start, prefer longer span.
    let matches = dedupe_overlaps(matches);

    // Build findings before rewriting (so offsets refer to the original content).
    let mut counter = TokenCounter::new();
    let mut findings: Vec<RedactionFinding> = Vec::with_capacity(matches.len());
    for m in &matches {
        let replacement = apply_strategy(config.strategy, &m.text, &m.category, &mut counter);
        findings.push(RedactionFinding {
            start: m.start as u32,
            end: m.end as u32,
            category: m.category.clone(),
            strategy: config.strategy,
            replacement_token: replacement,
        });
    }

    // 5. Rewrite content: apply replacements in reverse order so byte offsets
    // stay valid for earlier matches.
    let new_content = apply_replacements_reverse(&result.content, &matches, &findings);
    let original_content = std::mem::replace(&mut result.content, new_content);

    // 6. Rewrite formatted_content with the same content-substitution map.
    //    formatted_content uses different offsets from `content`, so we rescan it
    //    rather than reuse `matches`.
    if let Some(formatted) = result.formatted_content.as_ref() {
        let formatted_matches = build_matches_for(formatted, &categories_vec, config, &custom_regexes);
        let formatted_findings: Vec<RedactionFinding> = formatted_matches
            .iter()
            .map(|m| {
                let replacement = apply_strategy(config.strategy, &m.text, &m.category, &mut counter);
                RedactionFinding {
                    start: m.start as u32,
                    end: m.end as u32,
                    category: m.category.clone(),
                    strategy: config.strategy,
                    replacement_token: replacement,
                }
            })
            .collect();
        let rewritten = apply_replacements_reverse(formatted, &formatted_matches, &formatted_findings);
        result.formatted_content = Some(rewritten);
    }

    // 7. Rewrite each chunk.
    if let Some(chunks) = result.chunks.as_mut() {
        for chunk in chunks.iter_mut() {
            let chunk_matches = build_matches_for(&chunk.content, &categories_vec, config, &custom_regexes);
            if chunk_matches.is_empty() {
                continue;
            }
            let chunk_findings: Vec<RedactionFinding> = chunk_matches
                .iter()
                .map(|m| {
                    let replacement = apply_strategy(config.strategy, &m.text, &m.category, &mut counter);
                    RedactionFinding {
                        start: m.start as u32,
                        end: m.end as u32,
                        category: m.category.clone(),
                        strategy: config.strategy,
                        replacement_token: replacement,
                    }
                })
                .collect();
            let original_len = chunk.content.len();
            let rewritten = apply_replacements_reverse(&chunk.content, &chunk_matches, &chunk_findings);
            let new_len = rewritten.len();
            chunk.content = rewritten;

            if config.preserve_offsets {
                // Shift `byte_end` to track the rewrite. `byte_start` is the
                // anchor in the original document and stays put unless the
                // shift makes the range invalid.
                let delta = new_len as isize - original_len as isize;
                let new_end = (chunk.metadata.byte_end as isize + delta).max(chunk.metadata.byte_start as isize);
                chunk.metadata.byte_end = new_end as usize;
            }
        }
    }

    // 8. Rewrite NER entity text (if any).
    if let Some(entities) = result.entities.as_mut() {
        for entity in entities.iter_mut() {
            entity.text = redact_string(&entity.text, &categories_vec, config, &custom_regexes, &mut counter);
        }
    }

    // 9. Rewrite summary text.
    if let Some(summary) = result.summary.as_mut() {
        summary.text = redact_string(&summary.text, &categories_vec, config, &custom_regexes, &mut counter);
    }

    // 10. Rewrite translation body + formatted markup.
    if let Some(translation) = result.translation.as_mut() {
        translation.content = redact_string(
            &translation.content,
            &categories_vec,
            config,
            &custom_regexes,
            &mut counter,
        );
        if let Some(formatted) = translation.formatted_content.as_mut() {
            *formatted = redact_string(formatted, &categories_vec, config, &custom_regexes, &mut counter);
        }
    }

    // 11. Rewrite page classification labels — labels are configured strings, so
    // redacting them rarely fires, but Custom categories may match.
    if let Some(pages) = result.page_classifications.as_mut() {
        for page in pages.iter_mut() {
            for label in page.labels.iter_mut() {
                label.label = redact_string(&label.label, &categories_vec, config, &custom_regexes, &mut counter);
            }
        }
    }

    // 11b. Walk every remaining text-bearing field. Redaction previously
    // rewrote only content/formatted_content/chunks/entities/summary/translation
    // and left ~12 serialized fields carrying the original PII while the report
    // claimed success (xberg-io/xberg#1223). Every field a consumer can read
    // must be masked, or the report is a lie.
    redact_secondary_text_fields(result, &categories_vec, config, &custom_regexes, &mut counter);

    // 12. Populate redaction_report.
    let total = findings.len() as u32;
    result.redaction_report = Some(RedactionReport {
        findings,
        total_redacted: total,
    });

    // Drop the original_content explicitly so the compiler can't keep it alive.
    drop(original_content);

    Ok(())
}

/// Compute the set of categories the engine will consider during this run.
fn active_categories(config: &RedactionConfig) -> HashSet<PiiCategory> {
    if config.categories.is_empty() {
        let mut s: HashSet<PiiCategory> = [
            PiiCategory::Email,
            PiiCategory::Phone,
            PiiCategory::Ssn,
            PiiCategory::CreditCard,
            PiiCategory::PostalCode,
            PiiCategory::IpAddress,
            PiiCategory::Iban,
            PiiCategory::SwiftBic,
        ]
        .into_iter()
        .collect();
        if config.ner.is_some() {
            s.insert(PiiCategory::Person);
            s.insert(PiiCategory::Organization);
            s.insert(PiiCategory::Location);
        }
        s
    } else {
        config.categories.clone()
    }
}

/// Build matches for an arbitrary string by re-running the pattern engine.
/// (NER backends operate on the main `content`; secondary fields are
/// regex-only by design — re-running NER per field would be expensive and
/// the source field text is generally derived from the main content.)
fn build_matches_for(
    text: &str,
    categories: &[PiiCategory],
    config: &RedactionConfig,
    custom_regexes: &[(String, regex::Regex)],
) -> Vec<PatternMatch> {
    let mut matches = scan_text(text, categories);
    matches.extend(scan_custom(text, custom_regexes));
    if !config.categories.is_empty() {
        matches.retain(|m| matches!(m.category, PiiCategory::Custom(_)) || config.categories.contains(&m.category));
    }
    dedupe_overlaps(matches)
}

/// Compile every user-supplied term and pattern once. Returns `(label, regex)`
/// tuples in declaration order — terms first, then patterns.
///
/// Regex compilation has already been validated by
/// [`RedactionConfig::validate`]; this function silently skips malformed inputs
/// so a residual stray pattern can't crash the engine.
fn compile_custom(config: &RedactionConfig) -> Vec<(String, regex::Regex)> {
    let mut out: Vec<(String, regex::Regex)> =
        Vec::with_capacity(config.custom_terms.len() + config.custom_patterns.len());

    for term in &config.custom_terms {
        if term.value.is_empty() {
            continue;
        }
        let escaped = regex::escape(&term.value);
        let pattern_str = if term.case_sensitive {
            escaped
        } else {
            format!("(?i){escaped}")
        };
        if let Ok(regex) = regex::Regex::new(&pattern_str) {
            out.push((term.label.clone(), regex));
        }
    }

    for pattern in &config.custom_patterns {
        if pattern.pattern.is_empty() {
            continue;
        }
        let pattern_str = if pattern.case_sensitive {
            pattern.pattern.clone()
        } else {
            format!("(?i){}", pattern.pattern)
        };
        if let Ok(regex) = regex::Regex::new(&pattern_str) {
            out.push((pattern.label.clone(), regex));
        }
    }

    out
}

/// Scan `text` with pre-compiled custom regexes. Surfaces hits as
/// `PiiCategory::Custom(label)` matches.
fn scan_custom(text: &str, custom_regexes: &[(String, regex::Regex)]) -> Vec<PatternMatch> {
    let mut out = Vec::new();
    for (label, regex) in custom_regexes {
        for m in regex.find_iter(text) {
            out.push(PatternMatch {
                start: m.start(),
                end: m.end(),
                category: PiiCategory::Custom(label.clone()),
                text: m.as_str().to_string(),
            });
        }
    }
    out
}

/// Apply per-match replacements in reverse byte order so earlier offsets remain valid.
fn apply_replacements_reverse(text: &str, matches: &[PatternMatch], findings: &[RedactionFinding]) -> String {
    debug_assert_eq!(matches.len(), findings.len());
    let mut out = text.to_string();
    for (m, finding) in matches.iter().zip(findings.iter()).rev() {
        // Guard against out-of-range or non-UTF-8-boundary spans.
        if m.start <= m.end && m.end <= out.len() && out.is_char_boundary(m.start) && out.is_char_boundary(m.end) {
            out.replace_range(m.start..m.end, &finding.replacement_token);
        }
    }
    out
}

/// Pick the highest-priority match among overlapping spans.
///
/// Strategy: walk matches in (start, -length) order; keep a match only if its
/// start is at or after the previously-kept end. This is a standard interval
/// dedupe that prefers earlier and longer spans.
fn dedupe_overlaps(mut matches: Vec<PatternMatch>) -> Vec<PatternMatch> {
    if matches.is_empty() {
        return matches;
    }
    matches.sort_by(|a, b| a.start.cmp(&b.start).then((b.end - b.start).cmp(&(a.end - a.start))));
    let mut kept: Vec<PatternMatch> = Vec::with_capacity(matches.len());
    for m in matches {
        if let Some(last) = kept.last()
            && m.start < last.end
        {
            continue;
        }
        kept.push(m);
    }
    kept
}

/// Run redaction over a single string, returning the rewritten copy.
fn redact_string(
    text: &str,
    categories: &[PiiCategory],
    config: &RedactionConfig,
    custom_regexes: &[(String, regex::Regex)],
    counter: &mut TokenCounter,
) -> String {
    let matches = build_matches_for(text, categories, config, custom_regexes);
    if matches.is_empty() {
        return text.to_string();
    }
    let findings: Vec<RedactionFinding> = matches
        .iter()
        .map(|m| {
            let replacement = apply_strategy(config.strategy, &m.text, &m.category, counter);
            RedactionFinding {
                start: m.start as u32,
                end: m.end as u32,
                category: m.category.clone(),
                strategy: config.strategy,
                replacement_token: replacement,
            }
        })
        .collect();
    apply_replacements_reverse(text, &matches, &findings)
}

/// Redact every string in a JSON value tree in place (keys are left alone;
/// only values are masked). Used for `structured_output` and
/// `code_intelligence`, which hold LLM-distilled / derived text.
fn redact_json_value(
    value: &mut serde_json::Value,
    categories: &[PiiCategory],
    config: &RedactionConfig,
    custom_regexes: &[(String, regex::Regex)],
    counter: &mut TokenCounter,
) {
    match value {
        serde_json::Value::String(s) => {
            *s = redact_string(s, categories, config, custom_regexes, counter);
        }
        serde_json::Value::Array(items) => {
            for item in items.iter_mut() {
                redact_json_value(item, categories, config, custom_regexes, counter);
            }
        }
        serde_json::Value::Object(map) => {
            for v in map.values_mut() {
                redact_json_value(v, categories, config, custom_regexes, counter);
            }
        }
        _ => {}
    }
}

/// Mask PII in every text-bearing output field beyond the primary
/// content/chunk/entity set. Recurses into image OCR sub-documents.
///
/// This is an allowlist of *fields*, but it is meant to be exhaustive over the
/// text surfaces of [`ExtractedDocument`]; when a new text field is added there,
/// it must be added here too (see the field-coverage test in this module).
fn redact_secondary_text_fields(
    doc: &mut ExtractedDocument,
    categories: &[PiiCategory],
    config: &RedactionConfig,
    custom_regexes: &[(String, regex::Regex)],
    counter: &mut TokenCounter,
) {
    macro_rules! rd {
        ($s:expr) => {{
            let redacted = redact_string(&$s, categories, config, custom_regexes, counter);
            $s = redacted;
        }};
    }
    macro_rules! rd_opt {
        ($o:expr) => {
            if let Some(s) = $o.as_mut() {
                rd!(*s);
            }
        };
    }

    // Top-level tables.
    for table in doc.tables.iter_mut() {
        for row in table.cells.iter_mut() {
            for cell in row.iter_mut() {
                rd!(*cell);
            }
        }
        rd!(table.markdown);
    }

    // Per-page content and per-page tables (Arc-shared, so make_mut to write).
    if let Some(pages) = doc.pages.as_mut() {
        for page in pages.iter_mut() {
            rd!(page.content);
            for table in page.tables.iter_mut() {
                let table = std::sync::Arc::make_mut(table);
                for row in table.cells.iter_mut() {
                    for cell in row.iter_mut() {
                        rd!(*cell);
                    }
                }
                rd!(table.markdown);
            }
        }
    }

    // Element stream.
    if let Some(elements) = doc.elements.as_mut() {
        for el in elements.iter_mut() {
            rd!(el.text);
        }
    }

    // OCR elements.
    if let Some(ocr_elements) = doc.ocr_elements.as_mut() {
        for el in ocr_elements.iter_mut() {
            rd!(el.text);
        }
    }

    // Djot plain-text projection.
    if let Some(djot) = doc.djot_content.as_mut() {
        rd!(djot.plain_text);
    }

    // Images: captions, descriptions, and nested OCR sub-documents.
    if let Some(images) = doc.images.as_mut() {
        for image in images.iter_mut() {
            rd_opt!(image.caption);
            rd_opt!(image.description);
            if let Some(ocr_doc) = image.ocr_result.as_mut() {
                rd!(ocr_doc.content);
                redact_secondary_text_fields(ocr_doc, categories, config, custom_regexes, counter);
            }
        }
    }

    // Hyperlinks and their labels — Email is an always-on category, so the very
    // addresses redaction claims to remove were being returned here verbatim.
    if let Some(uris) = doc.uris.as_mut() {
        for uri in uris.iter_mut() {
            rd!(uri.url);
            rd_opt!(uri.label);
        }
    }

    // PDF annotation / comment text.
    if let Some(annotations) = doc.annotations.as_mut() {
        for annotation in annotations.iter_mut() {
            rd_opt!(annotation.content);
        }
    }

    // Interactive form field values — the canonical PII surface on filled
    // tax / medical / loan forms.
    for field in doc.form_fields.iter_mut() {
        rd!(field.name);
        rd_opt!(field.value);
        rd_opt!(field.default_value);
        rd_opt!(field.tooltip);
    }

    // Derived keyword text.
    #[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
    if let Some(keywords) = doc.extracted_keywords.as_mut() {
        for kw in keywords.iter_mut() {
            rd!(kw.text);
        }
    }

    // Document metadata.
    rd_opt!(doc.metadata.title);
    rd_opt!(doc.metadata.subject);
    if let Some(authors) = doc.metadata.authors.as_mut() {
        for author in authors.iter_mut() {
            rd!(*author);
        }
    }
    if let Some(keywords) = doc.metadata.keywords.as_mut() {
        for kw in keywords.iter_mut() {
            rd!(*kw);
        }
    }

    // LLM-distilled structured output and code-intelligence JSON.
    if let Some(structured) = doc.structured_output.as_mut() {
        redact_json_value(structured, categories, config, custom_regexes, counter);
    }
    #[cfg(feature = "tree-sitter")]
    if let Some(code) = doc.code_intelligence.as_mut() {
        redact_json_value(code, categories, config, custom_regexes, counter);
    }
}

/// Convert NER-detected entities into pattern matches so the same offset
/// machinery can rewrite them. Only Person / Organization / Location are
/// considered redactable — Email / Phone / Url etc. flow through the pattern
/// engine which is more reliable for structured PII.
#[cfg(feature = "ner")]
async fn collect_ner_matches(
    text: &str,
    ner_config: &crate::core::config::ner::NerConfig,
    active: &HashSet<PiiCategory>,
) -> Result<Vec<PatternMatch>> {
    use crate::types::entity::EntityCategory;

    let want_person = active.contains(&PiiCategory::Person);
    let want_org = active.contains(&PiiCategory::Organization);
    let want_loc = active.contains(&PiiCategory::Location);
    if !(want_person || want_org || want_loc) {
        return Ok(Vec::new());
    }

    let mut categories: Vec<EntityCategory> = Vec::new();
    if want_person {
        categories.push(EntityCategory::Person);
    }
    if want_org {
        categories.push(EntityCategory::Organization);
    }
    if want_loc {
        categories.push(EntityCategory::Location);
    }

    let backend = make_ner_backend(ner_config)?;
    let entities = backend
        .detect_with_custom(text, &categories, &ner_config.custom_labels)
        .await?;

    Ok(entities
        .into_iter()
        .filter_map(|e| {
            let category = match e.category {
                EntityCategory::Person => PiiCategory::Person,
                EntityCategory::Organization => PiiCategory::Organization,
                EntityCategory::Location => PiiCategory::Location,
                _ => return None,
            };
            Some(PatternMatch {
                start: e.start as usize,
                end: e.end as usize,
                category,
                text: e.text,
            })
        })
        .collect())
}

#[cfg(feature = "ner")]
fn make_ner_backend(
    config: &crate::core::config::ner::NerConfig,
) -> Result<std::sync::Arc<dyn crate::text::ner::NerBackend>> {
    use crate::core::config::ner::NerBackendKind;

    match config.backend {
        NerBackendKind::Onnx => {
            #[cfg(feature = "ner-onnx")]
            {
                Ok(crate::text::ner::gline::get_or_init_backend(config.model.as_deref())?)
            }
            #[cfg(not(feature = "ner-onnx"))]
            {
                Err(crate::XbergError::MissingDependency(
                    "ner-onnx feature is not enabled — rebuild xberg with --features ner-onnx".to_string(),
                ))
            }
        }
        NerBackendKind::Llm => {
            #[cfg(all(feature = "ner-llm", not(all(target_os = "android", target_arch = "x86_64"))))]
            {
                let llm = config.llm.clone().ok_or_else(|| {
                    crate::XbergError::validation("Llm NER backend selected but NerConfig.llm is None".to_string())
                })?;
                let backend = crate::text::ner::llm::LlmBackend::new(llm);
                Ok(std::sync::Arc::new(backend))
            }
            #[cfg(not(all(feature = "ner-llm", not(all(target_os = "android", target_arch = "x86_64")))))]
            {
                Err(crate::XbergError::MissingDependency(
                    "ner-llm feature is not enabled — rebuild xberg with --features ner-llm".to_string(),
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dedupe_overlaps_keeps_longer_first() {
        let matches = vec![
            PatternMatch {
                start: 0,
                end: 10,
                category: PiiCategory::Email,
                text: "long@x.com".into(),
            },
            PatternMatch {
                start: 5,
                end: 8,
                category: PiiCategory::Phone,
                text: "555".into(),
            },
        ];
        let kept = dedupe_overlaps(matches);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].category, PiiCategory::Email);
    }

    #[test]
    fn test_apply_replacements_reverse() {
        let text = "Email me at alice@example.com or bob@test.io.";
        let matches = vec![
            PatternMatch {
                start: 12,
                end: 29,
                category: PiiCategory::Email,
                text: "alice@example.com".into(),
            },
            PatternMatch {
                start: 33,
                end: 44,
                category: PiiCategory::Email,
                text: "bob@test.io".into(),
            },
        ];
        let findings = vec![
            RedactionFinding {
                start: 12,
                end: 29,
                category: PiiCategory::Email,
                strategy: crate::types::redaction::RedactionStrategy::Mask,
                replacement_token: "[REDACTED]".into(),
            },
            RedactionFinding {
                start: 33,
                end: 44,
                category: PiiCategory::Email,
                strategy: crate::types::redaction::RedactionStrategy::Mask,
                replacement_token: "[REDACTED]".into(),
            },
        ];
        let out = apply_replacements_reverse(text, &matches, &findings);
        assert_eq!(out, "Email me at [REDACTED] or [REDACTED].");
    }

    /// Regression for xberg-io/xberg#1223: redaction must mask PII on every
    /// structured surface, not just `content`. Builds a document that carries
    /// the same email in a table cell, a page, an element, a URI, a form field,
    /// metadata, and structured_output, then asserts none survive.
    #[tokio::test]
    async fn redacts_every_text_bearing_field() {
        use crate::types::form_field::PdfFormField;
        use crate::types::uri::{ExtractedUri, UriKind};

        let email = "alice@example.com";
        let mut doc = ExtractedDocument::default();
        doc.content = format!("Contact {email} for details.");
        doc.tables = vec![crate::types::tables::Table {
            cells: vec![vec!["Name".into(), email.into()]],
            markdown: format!("| Name | {email} |"),
            page_number: 1,
            bounding_box: None,
        }];
        doc.pages = Some(vec![crate::types::PageContent {
            page_number: 1,
            content: format!("Page mentions {email}."),
            tables: Vec::new(),
            image_indices: Vec::new(),
            hierarchy: None,
            is_blank: None,
            layout_regions: None,
            speaker_notes: None,
            section_name: None,
            sheet_name: None,
        }]);
        doc.uris = Some(vec![ExtractedUri {
            url: format!("mailto:{email}"),
            label: Some(email.into()),
            page: None,
            kind: UriKind::Email,
        }]);
        doc.form_fields = vec![PdfFormField {
            name: "applicant_email".into(),
            full_name: "form.applicant_email".into(),
            field_type: crate::types::form_field::FormFieldType::Text,
            value: Some(email.into()),
            default_value: None,
            flags: 0,
            page: None,
            bbox: None,
            max_length: None,
            tooltip: None,
        }];
        doc.metadata.subject = Some(format!("Re: {email}"));
        doc.structured_output = Some(serde_json::json!({ "email": email }));

        let config = RedactionConfig::default();
        redact(&mut doc, &config).await.expect("redaction must succeed");

        // Everything the extractor returns must be clean.
        let mut leaks: Vec<&str> = Vec::new();
        if doc.content.contains(email) {
            leaks.push("content");
        }
        if doc.tables[0].cells.iter().flatten().any(|c| c.contains(email)) || doc.tables[0].markdown.contains(email) {
            leaks.push("tables");
        }
        if doc.pages.as_ref().unwrap()[0].content.contains(email) {
            leaks.push("pages");
        }
        let uri = &doc.uris.as_ref().unwrap()[0];
        if uri.url.contains(email) || uri.label.as_deref().unwrap_or("").contains(email) {
            leaks.push("uris");
        }
        if doc.form_fields[0].value.as_deref().unwrap_or("").contains(email) {
            leaks.push("form_fields");
        }
        if doc.metadata.subject.as_deref().unwrap_or("").contains(email) {
            leaks.push("metadata");
        }
        if doc.structured_output.as_ref().unwrap().to_string().contains(email) {
            leaks.push("structured_output");
        }
        assert!(leaks.is_empty(), "PII leaked on fields: {leaks:?}");
    }
}
