//! Redaction engine: orchestrates pattern matching, optional NER, span merging,
//! and the destructive rewrite of every textual field on [`ExtractedDocument`].
//!
//! The engine is invoked from the Late-stage post-processor at
//! [`crate::plugins::processor::builtin::redaction`].
//!
//! # One pass, every field
//!
//! A single `RedactionPass` carries the whole matcher set — pattern engine,
//! user-supplied terms and patterns, *and* the terms derived from NER
//! detections — and every text-bearing field is rewritten through it. The NER
//! backend reads [`ExtractedDocument::content`] and each kept text layer, so its
//! byte spans are meaningless for a table cell or a metadata value; the detected mentions are
//! therefore also compiled into literal matchers that run against every field
//! and every occurrence (xberg-io/xberg#200, #202, #203).
//!
//! # Kept text layer
//!
//! `PageContent::native_content` holds the characters where the text layer of a
//! page and its OCR text differ, so a detection over `content` says nothing
//! about it. A pass returns a kept text layer only when every redaction source
//! of the pass was evaluated on that text: entity detection runs on each kept
//! text layer of the document's own pages, and its mentions join the matcher
//! set. A source that cannot be evaluated on the kept text (a finding given as
//! offsets into `content`, a caller-supplied entity stream, a failed detection,
//! a kept text layer in an embedded document) removes the kept text layer, and
//! the pass adds one processing warning.
//!
//! # Audit trail
//!
//! A [`RedactionFinding`] is recorded only once the replacement has actually
//! been applied to the field (xberg-io/xberg#201), and the report carries the
//! findings from *every* field, not just `content` (xberg-io/xberg#204).
//! `RedactionFinding::start`/`end` are byte offsets within the field the
//! finding came from — for `content` findings that is the original content, as
//! documented on the type; for a secondary field it is that field's own
//! pre-redaction text.

use std::collections::HashSet;

use crate::Result;
use crate::core::config::redaction::{ExternalRedactionFinding, RedactionConfig, RedactionOffsetEncoding};
use crate::extractors::security::SecurityLimits;
use crate::types::ExtractedDocument;
use crate::types::entity::{Entity, EntityCategory};
use crate::types::metadata::FormatMetadata;
use crate::types::redaction::{PiiCategory, RedactionFinding, RedactionReport};
use crate::types::revisions::{DiffLine, RevisionAnchor};

#[cfg(feature = "tokio-runtime")]
use super::external::compile_configured_findings_async;
use super::external::{
    ExternalRedactionRequest, any_offset_finding, compile_configured_findings, compile_external_findings,
};
use super::patterns::{PatternMatch, scan_text};
use super::strategy::{TokenCounter, apply_strategy};

/// Maximum nesting depth followed into embedded sub-documents (image OCR
/// results, archive members).
///
/// Redaction walks the same tree the extractor built; a hostile deeply-nested
/// archive must not be able to turn that walk into a stack overflow.
const MAX_NESTED_DOCUMENT_DEPTH: usize = 16;

/// Maximum nesting depth followed into djot block trees, for the same reason.
const MAX_BLOCK_NESTING_DEPTH: usize = 32;

/// Run pattern redaction (and optional NER-driven redaction) over `result` and
/// rewrite every textual field. Populates `result.redaction_report`.
pub async fn redact(result: &mut ExtractedDocument, config: &RedactionConfig) -> Result<()> {
    redact_with_security_limits(result, config, &SecurityLimits::default()).await
}

/// Redact an owned document using a JSON array or JSON Lines payload from an external inspection engine.
///
/// The payload is parsed in Rust so vendor aliases and nested fields remain intact across language bindings.
/// `offset_encoding` defaults to `unicode_code_points` and `max_findings` defaults to 10,000 when omitted.
/// Unknown encodings return a validation error.
#[cfg_attr(feature = "alef-meta", alef(since = "1.3.1"))]
pub async fn redact_external(
    document: ExtractedDocument,
    config: RedactionConfig,
    findings_json: &str,
    offset_encoding: Option<&str>,
    max_findings: Option<u32>,
) -> Result<ExtractedDocument> {
    let offset_encoding = offset_encoding.unwrap_or("unicode_code_points").parse()?;
    let requested_limit = max_findings.unwrap_or(super::external::DEFAULT_MAX_FINDINGS);
    let default_limits = SecurityLimits::default();
    let security_limit = super::external::security_finding_limit(&default_limits);
    let effective_limit = u32::try_from(security_limit.min(requested_limit as usize)).map_err(|_| {
        crate::XbergError::validation("effective redaction finding limit exceeds the supported u32 range".to_string())
    })?;
    let findings = super::external::parse_external_findings_bounded(findings_json, effective_limit)?;
    redact_external_with_findings(document, config, findings, offset_encoding, max_findings).await
}

pub(crate) async fn redact_external_with_findings(
    mut document: ExtractedDocument,
    config: RedactionConfig,
    findings: Vec<ExternalRedactionFinding>,
    offset_encoding: RedactionOffsetEncoding,
    max_findings: Option<u32>,
) -> Result<ExtractedDocument> {
    let requested_limit = max_findings.unwrap_or(super::external::DEFAULT_MAX_FINDINGS);
    let default_limits = SecurityLimits::default();
    let security_limit = super::external::security_finding_limit(&default_limits);
    let effective_limit = security_limit.min(requested_limit as usize);
    if findings.len() > effective_limit {
        let message = if requested_limit as usize <= security_limit {
            format!("redaction findings exceed maximum of {requested_limit}")
        } else {
            format!("redaction findings exceed the effective redaction finding limit ({effective_limit})")
        };
        return Err(crate::XbergError::validation(message));
    }
    config.validate()?;
    let external_terms = compile_external_findings(
        &document.content,
        &findings,
        offset_encoding,
        effective_limit as u32,
        config.min_score,
    )?;
    redact_counted(
        &mut document,
        &config,
        CompiledExternalFindings {
            terms: &external_terms,
            count: findings.len(),
            limit: Some(effective_limit),
            has_offsets: any_offset_finding(&findings),
        },
        true,
        &default_limits,
    )
    .await?;
    Ok(document)
}

pub(crate) async fn redact_with_security_limits(
    result: &mut ExtractedDocument,
    config: &RedactionConfig,
    limits: &SecurityLimits,
) -> Result<()> {
    redact_counted(result, config, CompiledExternalFindings::default(), true, limits)
        .await
        .map(|_counter| ())
}

pub(crate) async fn redact_with_external_findings(
    result: &mut ExtractedDocument,
    config: &RedactionConfig,
    request: &ExternalRedactionRequest,
    limits: &SecurityLimits,
) -> Result<()> {
    config.validate()?;
    let external_terms = compile_external_findings(
        &result.content,
        &request.findings,
        request.offset_encoding,
        request.max_findings,
        config.min_score,
    )?;
    redact_counted(
        result,
        config,
        CompiledExternalFindings {
            terms: &external_terms,
            count: request.findings.len(),
            limit: Some(request.max_findings as usize),
            has_offsets: any_offset_finding(&request.findings),
        },
        request.include_configured_sources,
        limits,
    )
    .await
    .map(|_counter| ())
}

/// Like [`redact`], additionally returning the token to original-text map for
/// later rehydration. Only `RedactionStrategy::TokenReplace` allocations
/// appear in the map; `Mask`, `Hash`, and `Drop` are not reversible. The map
/// never touches disk here; encrypt it with
/// [`super::rehydration::encrypt_map`] and persistence stays with the caller.
#[cfg(feature = "redaction-rehydrate")]
#[cfg_attr(alef, alef(skip))]
pub async fn redact_capturing_rehydration_map(
    result: &mut ExtractedDocument,
    config: &RedactionConfig,
) -> Result<super::rehydration::RehydrationMap> {
    let counter = redact_counted(
        result,
        config,
        CompiledExternalFindings::default(),
        true,
        &SecurityLimits::default(),
    )
    .await?;
    Ok(counter.rehydration_map())
}

/// Redact `result` using `config` plus a caller-supplied entity stream, without
/// invoking a NER backend.
///
/// [`redact`] is this function with the entities the backend configured in
/// [`RedactionConfig::ner`] produced. Callers that already have an entity
/// stream — from `crate::enrich`, from the NER post-processor, or from their
/// own model — pass it here and skip the second inference pass. Entities of a
/// category the pattern engine already covers (email, phone, URL) are ignored:
/// the regex engine is the more reliable detector for those.
///
/// The entity stream covers `content` only, and this function runs no
/// detection. It therefore removes every kept text layer
/// (`PageContent::native_content`) and adds one processing warning. [`redact`]
/// runs the detection on each kept text layer and returns it redacted.
#[cfg_attr(alef, alef(skip))]
pub fn redact_with_entities(
    result: &mut ExtractedDocument,
    config: &RedactionConfig,
    entities: &[Entity],
) -> Result<()> {
    config.validate()?;
    let configured = compile_configured_findings(&result.content, config, &SecurityLimits::default())?;
    redact_pass(
        result,
        config,
        entities,
        &configured.terms,
        true,
        KeptTextRule::WITHHOLD,
        0,
    );
    Ok(())
}

/// Shared body for [`redact`] and the map-capturing variant: runs the full
/// pass and hands back the token counter it used.
#[derive(Default)]
struct CompiledExternalFindings<'a> {
    terms: &'a [(PiiCategory, regex::Regex)],
    count: usize,
    limit: Option<usize>,
    /// At least one finding gives a span of the content and no text.
    has_offsets: bool,
}

async fn redact_counted(
    result: &mut ExtractedDocument,
    config: &RedactionConfig,
    external: CompiledExternalFindings<'_>,
    include_configured_sources: bool,
    limits: &SecurityLimits,
) -> Result<TokenCounter> {
    config.validate()?;
    #[cfg(feature = "tokio-runtime")]
    let configured = compile_configured_findings_async(&result.content, config, limits).await?;
    #[cfg(not(feature = "tokio-runtime"))]
    let configured = compile_configured_findings(&result.content, config, limits)?;
    let total_findings = configured.count.saturating_add(external.count);
    let security_limit = super::external::security_finding_limit(limits);
    let limit = external
        .limit
        .map_or(security_limit, |request_limit| security_limit.min(request_limit));
    if total_findings > limit {
        return Err(crate::XbergError::validation(format!(
            "RedactionConfig: {total_findings} findings exceed the effective redaction finding limit ({limit})"
        )));
    }
    let has_offsets = configured.has_offsets || external.has_offsets;
    let mut all_external_terms = configured.terms;
    all_external_terms.extend_from_slice(external.terms);

    #[cfg(feature = "ner")]
    let (entities, kept_text, withheld_kept_text) = match (include_configured_sources, &config.ner) {
        (true, Some(ner_config)) => {
            let active = active_categories(config);
            let mut entities = collect_ner_entities(&result.content, ner_config, &active).await?;
            if has_offsets {
                (entities, KeptTextRule::WITHHOLD, 0)
            } else {
                let withheld = collect_kept_text_entities(result, ner_config, &active, &mut entities).await;
                (entities, KeptTextRule::REDACT_OWN, withheld)
            }
        }
        _ => (Vec::new(), KeptTextRule::for_offsets(has_offsets), 0),
    };
    #[cfg(not(feature = "ner"))]
    let (entities, kept_text, withheld_kept_text): (Vec<Entity>, KeptTextRule, usize) =
        (Vec::new(), KeptTextRule::for_offsets(has_offsets), 0);

    Ok(redact_pass(
        result,
        config,
        &entities,
        &all_external_terms,
        include_configured_sources,
        kept_text,
        withheld_kept_text,
    ))
}

/// Rewrite every text-bearing field on `result` and populate its audit report.
fn redact_pass(
    result: &mut ExtractedDocument,
    config: &RedactionConfig,
    entities: &[Entity],
    external_terms: &[(PiiCategory, regex::Regex)],
    include_configured_sources: bool,
    kept_text: KeptTextRule,
    withheld_kept_text: usize,
) -> TokenCounter {
    let active = if include_configured_sources {
        active_categories(config)
    } else {
        HashSet::new()
    };
    let categories: Vec<PiiCategory> = active.iter().cloned().collect();
    let custom_regexes = if include_configured_sources {
        compile_custom(config)
    } else {
        Vec::new()
    };
    let ner_terms = if include_configured_sources {
        compile_ner_terms(entities, config)
    } else {
        Vec::new()
    };

    let mut pass = RedactionPass {
        categories: &categories,
        config,
        custom_regexes: &custom_regexes,
        ner_terms: &ner_terms,
        external_terms,
        include_configured_sources,
        kept_text,
        withheld_kept_text,
        counter: TokenCounter::new(),
        findings: Vec::new(),
    };

    pass.redact_document(result, 0);

    if pass.withheld_kept_text > 0 {
        result.processing_warnings.push(crate::core::diagnostics::warning(
            "redaction",
            format!(
                "Redaction withheld {} kept text layer(s): a redaction source could not be applied to that text.",
                pass.withheld_kept_text
            ),
        ));
    }

    let findings = std::mem::take(&mut pass.findings);
    let total_redacted = findings.len() as u32;
    result.redaction_report = Some(RedactionReport {
        findings,
        total_redacted,
    });

    pass.counter
}

/// Which kept text layers (`PageContent::native_content`) a pass removes instead of rewriting.
///
/// A pass returns a kept text layer only when every redaction source of the pass was evaluated on
/// that text. Unknown is not "no findings".
#[derive(Clone, Copy)]
struct KeptTextRule {
    /// Remove the kept text layers of the document's own pages.
    withhold_own: bool,
    /// Remove the kept text layers of embedded documents (archive members, image OCR results).
    withhold_embedded: bool,
}

impl KeptTextRule {
    /// Every source of the pass scans each field: patterns, custom terms, findings given as text.
    const REDACT: Self = Self {
        withhold_own: false,
        withhold_embedded: false,
    };
    /// A source refers to positions in `content`, or to a detection that did not read the kept text.
    const WITHHOLD: Self = Self {
        withhold_own: true,
        withhold_embedded: true,
    };
    /// Entity detection ran on the kept text layers of the document's own pages only.
    #[cfg(feature = "ner")]
    const REDACT_OWN: Self = Self {
        withhold_own: false,
        withhold_embedded: true,
    };

    fn for_offsets(has_offsets: bool) -> Self {
        if has_offsets { Self::WITHHOLD } else { Self::REDACT }
    }

    fn withholds(self, depth: usize) -> bool {
        if depth == 0 {
            self.withhold_own
        } else {
            self.withhold_embedded
        }
    }
}

/// One document-wide redaction pass.
///
/// Owns the compiled matcher set, the strategy counter, and the audit findings
/// so that every field is rewritten by exactly the same matchers. Holding the
/// matchers here is what makes NER-derived terms reach fields other than
/// `content` (xberg-io/xberg#202).
struct RedactionPass<'a> {
    categories: &'a [PiiCategory],
    config: &'a RedactionConfig,
    custom_regexes: &'a [(String, regex::Regex)],
    ner_terms: &'a [(PiiCategory, regex::Regex)],
    external_terms: &'a [(PiiCategory, regex::Regex)],
    include_configured_sources: bool,
    kept_text: KeptTextRule,
    /// Count of the kept text layers that the pass, or the detection before it, removed.
    withheld_kept_text: usize,
    counter: TokenCounter,
    findings: Vec<RedactionFinding>,
}

impl RedactionPass<'_> {
    /// Every match in `text`, deduped and in ascending byte order.
    fn matches_for(&self, text: &str) -> Vec<PatternMatch> {
        let mut matches = if self.include_configured_sources {
            scan_text(text, self.categories)
        } else {
            Vec::new()
        };

        let custom = self
            .custom_regexes
            .iter()
            .map(|(label, regex)| (PiiCategory::Custom(label.clone()), regex));
        matches.extend(scan_regexes(text, custom));

        let detected = self.ner_terms.iter().map(|(category, regex)| (category.clone(), regex));
        matches.extend(scan_regexes(text, detected));

        let external = self
            .external_terms
            .iter()
            .map(|(category, regex)| (category.clone(), regex));
        matches.extend(scan_regexes(text, external));

        if !self.config.categories.is_empty() {
            let requested = &self.config.categories;
            matches.retain(|m| matches!(m.category, PiiCategory::Custom(_)) || requested.contains(&m.category));
        }
        // A zero-length or inverted span carries no PII and would either insert a
        // stray token or underflow the interval dedupe. ~keep
        matches.retain(|m| m.start < m.end);
        dedupe_overlaps(matches)
    }

    /// Rewrite `text`, recording one finding per replacement that was actually
    /// applied.
    ///
    /// A match whose span is not a valid slice of `text`, or whose slice no
    /// longer holds the matched text, is skipped and produces no finding. The
    /// audit trail must never claim a redaction that did not happen — a false
    /// audit trail is worse than a visibly missing one (xberg-io/xberg#201).
    fn redact(&mut self, text: &str) -> String {
        let matches = self.matches_for(text);
        if matches.is_empty() {
            return text.to_string();
        }

        // Forward pass allocates replacements in ascending order, so TokenReplace
        // numbering still follows reading order; the rewrite then runs in reverse
        // so earlier byte offsets stay valid. ~keep
        let mut applied: Vec<(usize, usize, String)> = Vec::with_capacity(matches.len());
        for m in &matches {
            if !is_applicable(text, m) {
                continue;
            }
            let replacement = apply_strategy(self.config.strategy, &m.text, &m.category, &mut self.counter);
            self.findings.push(RedactionFinding {
                start: m.start as u32,
                end: m.end as u32,
                category: m.category.clone(),
                strategy: self.config.strategy,
                replacement_token: replacement.clone(),
            });
            applied.push((m.start, m.end, replacement));
        }

        let mut out = text.to_string();
        for (start, end, replacement) in applied.iter().rev() {
            out.replace_range(*start..*end, replacement);
        }
        out
    }

    /// Rewrite a string field in place.
    fn redact_in_place(&mut self, text: &mut String) {
        let redacted = self.redact(text);
        *text = redacted;
    }

    /// Rewrite an optional string field in place.
    fn redact_optional(&mut self, text: &mut Option<String>) {
        if let Some(text) = text.as_mut() {
            self.redact_in_place(text);
        }
    }

    /// Redact every string in a JSON value tree in place. Keys are left alone;
    /// only values are masked.
    fn redact_json_value(&mut self, value: &mut serde_json::Value) {
        match value {
            serde_json::Value::String(text) => self.redact_in_place(text),
            serde_json::Value::Array(items) => {
                for item in items.iter_mut() {
                    self.redact_json_value(item);
                }
            }
            serde_json::Value::Object(map) => {
                for value in map.values_mut() {
                    self.redact_json_value(value);
                }
            }
            _ => {}
        }
    }

    /// Rewrite every text surface of `doc`, recursing into embedded
    /// sub-documents up to [`MAX_NESTED_DOCUMENT_DEPTH`].
    fn redact_document(&mut self, doc: &mut ExtractedDocument, depth: usize) {
        // A nested result that its own pipeline already packaged (DOCX, PDF) holds base64 in
        // `content`. That pipeline ran with the same redaction config and redacted the
        // text before packaging it; masking a stretch of the encoding would corrupt it.
        let packaged = depth > 0
            && crate::plugins::registry::holds_encoded_package(doc.metadata.output_format.as_deref(), &doc.content);
        if !packaged {
            let content = std::mem::take(&mut doc.content);
            doc.content = self.redact(&content);
        }

        if let Some(formatted) = doc.formatted_content.take() {
            doc.formatted_content = Some(self.redact(&formatted));
        }

        self.redact_chunks(doc);

        if let Some(entities) = doc.entities.as_mut() {
            for entity in entities.iter_mut() {
                self.redact_in_place(&mut entity.text);
            }
        }

        if let Some(summary) = doc.summary.as_mut() {
            self.redact_in_place(&mut summary.text);
        }

        if let Some(translation) = doc.translation.as_mut() {
            self.redact_in_place(&mut translation.content);
            self.redact_optional(&mut translation.formatted_content);
        }

        if let Some(pages) = doc.page_classifications.as_mut() {
            for page in pages.iter_mut() {
                for label in page.labels.iter_mut() {
                    self.redact_in_place(&mut label.label);
                }
            }
        }

        self.redact_secondary_text_fields(doc, depth);
    }

    /// Rewrite chunk text, keeping chunk byte ranges consistent when the
    /// caller asked for it.
    fn redact_chunks(&mut self, doc: &mut ExtractedDocument) {
        let Some(chunks) = doc.chunks.as_mut() else {
            return;
        };
        for chunk in chunks.iter_mut() {
            let original_len = chunk.content.len();
            self.redact_in_place(&mut chunk.content);
            let new_len = chunk.content.len();

            if self.config.preserve_offsets && new_len != original_len {
                let delta = new_len as isize - original_len as isize;
                let new_end = (chunk.metadata.byte_end as isize + delta).max(chunk.metadata.byte_start as isize);
                chunk.metadata.byte_end = new_end as usize;
            }

            for heading in chunk.metadata.heading_path.iter_mut() {
                self.redact_in_place(heading);
            }
            if let Some(context) = chunk.metadata.heading_context.as_mut() {
                for heading in context.headings.iter_mut() {
                    self.redact_in_place(&mut heading.text);
                }
            }
        }
    }

    /// Mask PII in every text-bearing output field beyond the primary
    /// content/chunk/entity set.
    ///
    /// This is an allowlist of *fields*, but it is meant to be exhaustive over
    /// the text surfaces of [`ExtractedDocument`]; when a new text field is
    /// added there it must be added here too.
    fn redact_secondary_text_fields(&mut self, doc: &mut ExtractedDocument, depth: usize) {
        self.redact_tables(doc);
        self.redact_pages(doc, depth);
        self.redact_elements(doc);
        self.redact_djot(doc);
        self.redact_document_structure(doc);
        self.redact_revisions(doc);
        self.redact_nested_documents(doc, depth);
        self.redact_references(doc);
        self.redact_metadata(doc);
        self.redact_processing_warnings(doc);

        if let Some(structured) = doc.structured_output.as_mut() {
            self.redact_json_value(structured);
        }
        #[cfg(feature = "tree-sitter")]
        if let Some(code) = doc.code_intelligence.as_mut() {
            self.redact_json_value(code);
        }
    }

    /// Rewrite the document-level table cells and their rendered markdown.
    fn redact_tables(&mut self, doc: &mut ExtractedDocument) {
        for table in doc.tables.iter_mut() {
            for row in table.cells.iter_mut() {
                for cell in row.iter_mut() {
                    self.redact_in_place(cell);
                }
            }
            if let Some(columns) = table.columns.as_mut() {
                for column in columns.iter_mut() {
                    self.redact_in_place(column);
                }
            }
            self.redact_in_place(&mut table.markdown);
        }
    }

    /// Rewrite per-page text, slide/sheet names, hierarchy blocks, and page tables.
    ///
    /// The kept text layer of a page is rewritten, or removed when [`KeptTextRule`] says that a
    /// source of this pass was not evaluated on it.
    fn redact_pages(&mut self, doc: &mut ExtractedDocument, depth: usize) {
        let Some(pages) = doc.pages.as_mut() else {
            return;
        };
        let withhold_kept_text = self.kept_text.withholds(depth);
        for page in pages.iter_mut() {
            self.redact_in_place(&mut page.content);
            if withhold_kept_text {
                if page.native_content.take().is_some() {
                    self.withheld_kept_text += 1;
                }
            } else {
                self.redact_optional(&mut page.native_content);
            }
            self.redact_optional(&mut page.speaker_notes);
            self.redact_optional(&mut page.section_name);
            self.redact_optional(&mut page.sheet_name);
            if let Some(hierarchy) = page.hierarchy.as_mut() {
                for block in hierarchy.blocks.iter_mut() {
                    self.redact_in_place(&mut block.text);
                }
            }
            for table in page.tables.iter_mut() {
                let table = std::sync::Arc::make_mut(table);
                for row in table.cells.iter_mut() {
                    for cell in row.iter_mut() {
                        self.redact_in_place(cell);
                    }
                }
                if let Some(columns) = table.columns.as_mut() {
                    for column in columns.iter_mut() {
                        self.redact_in_place(column);
                    }
                }
                self.redact_in_place(&mut table.markdown);
            }
        }
    }

    /// Rewrite semantic elements, OCR elements, and formula source.
    fn redact_elements(&mut self, doc: &mut ExtractedDocument) {
        if let Some(elements) = doc.elements.as_mut() {
            for element in elements.iter_mut() {
                self.redact_in_place(&mut element.text);
                self.redact_optional(&mut element.metadata.filename);
                for value in element.metadata.additional.values_mut() {
                    self.redact_in_place(value);
                }
            }
        }
        if let Some(ocr_elements) = doc.ocr_elements.as_mut() {
            for element in ocr_elements.iter_mut() {
                self.redact_in_place(&mut element.text);
            }
        }
        for formula in doc.formulas.iter_mut() {
            self.redact_in_place(&mut formula.latex);
        }
    }

    /// Rewrite the structured document tree (`ExtractedDocument::document`).
    ///
    /// `NodeContent` carries the document body verbatim across every text-bearing
    /// node type; without this, a redaction pass leaves the structured tree
    /// holding everything the caller just asked to have hidden from `content`
    /// (xberg-io/xberg#298).
    fn redact_document_structure(&mut self, doc: &mut ExtractedDocument) {
        let Some(structure) = doc.document.as_mut() else {
            return;
        };
        for node in structure.nodes.iter_mut() {
            node.content.for_each_text_field_mut(|text| self.redact_in_place(text));
        }
    }

    /// Rewrite the djot representation: plain text, block tree, links, images,
    /// and footnotes.
    fn redact_djot(&mut self, doc: &mut ExtractedDocument) {
        let Some(djot) = doc.djot_content.as_mut() else {
            return;
        };
        self.redact_in_place(&mut djot.plain_text);
        for block in djot.blocks.iter_mut() {
            self.redact_djot_block(block, 0);
        }
        for link in djot.links.iter_mut() {
            self.redact_in_place(&mut link.url);
            self.redact_in_place(&mut link.text);
            self.redact_optional(&mut link.title);
        }
        for image in djot.images.iter_mut() {
            self.redact_in_place(&mut image.src);
            self.redact_in_place(&mut image.alt);
            self.redact_optional(&mut image.title);
        }
        for footnote in djot.footnotes.iter_mut() {
            self.redact_in_place(&mut footnote.label);
            for block in footnote.content.iter_mut() {
                self.redact_djot_block(block, 0);
            }
        }
    }

    /// Rewrite tracked-change text: the diff lines carry the document body
    /// verbatim, so an unredacted revision hands back what `content` just hid.
    fn redact_revisions(&mut self, doc: &mut ExtractedDocument) {
        let Some(revisions) = doc.revisions.as_mut() else {
            return;
        };
        for revision in revisions.iter_mut() {
            self.redact_optional(&mut revision.author);
            if let Some(RevisionAnchor::Sheet { name, .. }) = revision.anchor.as_mut() {
                self.redact_optional(name);
            }
            for line in revision.delta.content.iter_mut() {
                match line {
                    DiffLine::Context(text) | DiffLine::Added(text) | DiffLine::Removed(text) => {
                        self.redact_in_place(text);
                    }
                }
            }
            for change in revision.delta.table_changes.iter_mut() {
                self.redact_in_place(&mut change.from);
                self.redact_in_place(&mut change.to);
            }
            for change in revision.delta.property_changes.iter_mut() {
                self.redact_optional(&mut change.from);
                self.redact_optional(&mut change.to);
            }
        }
    }

    /// Rewrite embedded sub-documents: image OCR results and archive members.
    fn redact_nested_documents(&mut self, doc: &mut ExtractedDocument, depth: usize) {
        if let Some(images) = doc.images.as_mut() {
            for image in images.iter_mut() {
                self.redact_optional(&mut image.caption);
                self.redact_optional(&mut image.description);
                if let Some(ocr_doc) = image.ocr_result.as_mut()
                    && depth < MAX_NESTED_DOCUMENT_DEPTH
                {
                    self.redact_document(ocr_doc, depth + 1);
                }
            }
        }

        if let Some(children) = doc.children.as_mut() {
            for child in children.iter_mut() {
                self.redact_in_place(&mut child.path);
                if depth < MAX_NESTED_DOCUMENT_DEPTH {
                    self.redact_document(&mut child.result, depth + 1);
                }
            }
        }
    }

    /// Rewrite URIs, annotations, form fields, and extracted keywords.
    fn redact_references(&mut self, doc: &mut ExtractedDocument) {
        if let Some(uris) = doc.uris.as_mut() {
            for uri in uris.iter_mut() {
                self.redact_in_place(&mut uri.url);
                self.redact_optional(&mut uri.label);
            }
        }

        if let Some(annotations) = doc.annotations.as_mut() {
            for annotation in annotations.iter_mut() {
                self.redact_optional(&mut annotation.content);
            }
        }

        for field in doc.form_fields.iter_mut() {
            self.redact_in_place(&mut field.name);
            self.redact_optional(&mut field.value);
            self.redact_optional(&mut field.default_value);
            self.redact_optional(&mut field.tooltip);
        }

        #[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
        if let Some(keywords) = doc.extracted_keywords.as_mut() {
            for keyword in keywords.iter_mut() {
                self.redact_in_place(&mut keyword.text);
            }
        }
    }

    /// Rewrite the free-text metadata surfaces.
    fn redact_metadata(&mut self, doc: &mut ExtractedDocument) {
        self.redact_optional(&mut doc.metadata.title);
        self.redact_optional(&mut doc.metadata.subject);
        self.redact_optional(&mut doc.metadata.created_by);
        self.redact_optional(&mut doc.metadata.modified_by);
        self.redact_optional(&mut doc.metadata.category);
        self.redact_optional(&mut doc.metadata.abstract_text);

        if let Some(authors) = doc.metadata.authors.as_mut() {
            for author in authors.iter_mut() {
                self.redact_in_place(author);
            }
        }
        if let Some(keywords) = doc.metadata.keywords.as_mut() {
            for keyword in keywords.iter_mut() {
                self.redact_in_place(keyword);
            }
        }
        if let Some(tags) = doc.metadata.tags.as_mut() {
            for tag in tags.iter_mut() {
                self.redact_in_place(tag);
            }
        }

        if let Some(format) = doc.metadata.format.as_mut() {
            self.redact_format_metadata(format);
        }

        // Format-specific metadata lands here as untyped JSON for several
        // extractors, so it has to be walked as a value tree. ~keep
        for value in doc.metadata.additional.values_mut() {
            self.redact_json_value(value);
        }
    }

    /// Rewrite the free-text surfaces of `Metadata::format`.
    ///
    /// `FormatMetadata` is a ~20-variant discriminated union and was not visited by
    /// any redaction matcher at all: `EmailMetadata` alone carries sender/recipient
    /// addresses and names verbatim (`from_email`, `from_name`, `to_emails`,
    /// `cc_emails`, `bcc_emails`), and several other variants carry free text or
    /// names (sheet names, archive file paths, HTML page metadata, bibliographic
    /// author lists, source code chunks) (xberg-io/xberg#299). Variants with no
    /// free-text field (page/row counts, codecs, PDF page geometry, etc.) are
    /// intentionally left as no-ops.
    fn redact_format_metadata(&mut self, format: &mut FormatMetadata) {
        match format {
            FormatMetadata::Excel(excel) => {
                if let Some(sheet_names) = excel.sheet_names.as_mut() {
                    for name in sheet_names.iter_mut() {
                        self.redact_in_place(name);
                    }
                }
            }
            FormatMetadata::Email(email) => {
                self.redact_optional(&mut email.from_email);
                self.redact_optional(&mut email.from_name);
                self.redact_optional(&mut email.message_id);
                for address in email.to_emails.iter_mut() {
                    self.redact_in_place(address);
                }
                for address in email.cc_emails.iter_mut() {
                    self.redact_in_place(address);
                }
                for address in email.bcc_emails.iter_mut() {
                    self.redact_in_place(address);
                }
                for attachment in email.attachments.iter_mut() {
                    self.redact_in_place(attachment);
                }
            }
            FormatMetadata::Archive(archive) => {
                for path in archive.file_list.iter_mut() {
                    self.redact_in_place(path);
                }
            }
            FormatMetadata::Text(text) => {
                self.redact_text_metadata_fields(text);
            }
            #[cfg(feature = "office")]
            FormatMetadata::Docx(docx) => {
                if let Some(core) = docx.core_properties.as_mut() {
                    self.redact_optional(&mut core.title);
                    self.redact_optional(&mut core.subject);
                    self.redact_optional(&mut core.creator);
                    self.redact_optional(&mut core.keywords);
                    self.redact_optional(&mut core.description);
                    self.redact_optional(&mut core.last_modified_by);
                }
                if let Some(app) = docx.app_properties.as_mut() {
                    self.redact_optional(&mut app.company);
                }
                if let Some(custom) = docx.custom_properties.as_mut() {
                    for value in custom.values_mut() {
                        self.redact_json_value(value);
                    }
                }
            }
            #[cfg(feature = "office")]
            FormatMetadata::Bibtex(bibtex) => {
                for author in bibtex.authors.iter_mut() {
                    self.redact_in_place(author);
                }
            }
            #[cfg(feature = "office")]
            FormatMetadata::Citation(citation) => {
                for author in citation.authors.iter_mut() {
                    self.redact_in_place(author);
                }
                for keyword in citation.keywords.iter_mut() {
                    self.redact_in_place(keyword);
                }
            }
            #[cfg(feature = "office")]
            FormatMetadata::FictionBook(fiction_book) => {
                self.redact_optional(&mut fiction_book.annotation);
            }
            #[cfg(feature = "xml")]
            FormatMetadata::Jats(jats) => {
                self.redact_optional(&mut jats.copyright);
                for contributor in jats.contributor_roles.iter_mut() {
                    self.redact_in_place(&mut contributor.name);
                }
            }
            FormatMetadata::Html(html) => {
                self.redact_html_metadata_fields(html);
            }
            #[cfg(feature = "tree-sitter")]
            FormatMetadata::Code(code) => {
                for chunk in code.chunks.iter_mut() {
                    self.redact_in_place(&mut chunk.text);
                }
                if let Some(data) = code.data.as_mut() {
                    self.redact_code_data_node(data);
                }
            }
            #[cfg(feature = "pdf")]
            FormatMetadata::Pdf(_) => {}
            // Slide titles are the direct analogue of `ExcelMetadata::sheet_names`
            // handled above — a deck routinely names people in them
            // ("Performance review — J. Smith"). ~keep
            FormatMetadata::Pptx(pptx) => {
                for name in pptx.slide_names.iter_mut() {
                    self.redact_in_place(name);
                }
            }
            // EXIF values carry Artist, Copyright, camera-owner and GPS tags. Only the
            // values are redacted: the keys are EXIF tag names from a fixed vocabulary,
            // and rewriting them would corrupt the map without hiding anything.
            FormatMetadata::Image(image) => {
                for value in image.exif.values_mut() {
                    self.redact_in_place(value);
                }
            }
            // No string-bearing fields, or only format descriptors (delimiter, column
            // types, codec) that cannot carry document content.
            FormatMetadata::Xml(_) | FormatMetadata::Ocr(_) | FormatMetadata::Csv(_) | FormatMetadata::Pst(_) => {}
            #[cfg(feature = "office")]
            FormatMetadata::Dbf(_) | FormatMetadata::Epub(_) => {}
            #[cfg(feature = "transcription-types")]
            FormatMetadata::Audio(_) => {}
        }
    }

    /// Rewrite the free-text surfaces of `TextMetadata` (Markdown headers/links/code).
    fn redact_text_metadata_fields(&mut self, text: &mut crate::types::metadata::TextMetadata) {
        if let Some(headers) = text.headers.as_mut() {
            for header in headers.iter_mut() {
                self.redact_in_place(header);
            }
        }
        if let Some(links) = text.links.as_mut() {
            for link in links.iter_mut() {
                self.redact_in_place(&mut link.text);
                self.redact_in_place(&mut link.url);
            }
        }
        if let Some(code_blocks) = text.code_blocks.as_mut() {
            for code_block in code_blocks.iter_mut() {
                self.redact_in_place(&mut code_block.code);
            }
        }
    }

    /// Rewrite the free-text surfaces of `HtmlMetadata`: page-level metadata,
    /// social-card metadata, and every extracted header/link/image/structured-data
    /// element.
    fn redact_html_metadata_fields(&mut self, html: &mut crate::types::metadata::HtmlMetadata) {
        self.redact_optional(&mut html.title);
        self.redact_optional(&mut html.description);
        self.redact_optional(&mut html.author);
        self.redact_optional(&mut html.canonical_url);
        self.redact_optional(&mut html.base_href);
        for keyword in html.keywords.iter_mut() {
            self.redact_in_place(keyword);
        }
        for value in html.meta_tags.values_mut() {
            self.redact_in_place(value);
        }
        for value in html.open_graph.values_mut() {
            self.redact_in_place(value);
        }
        for value in html.twitter_card.values_mut() {
            self.redact_in_place(value);
        }
        for header in html.headers.iter_mut() {
            self.redact_in_place(&mut header.text);
        }
        for link in html.links.iter_mut() {
            self.redact_in_place(&mut link.href);
            self.redact_in_place(&mut link.text);
            self.redact_optional(&mut link.title);
        }
        for image in html.images.iter_mut() {
            self.redact_in_place(&mut image.src);
            self.redact_optional(&mut image.alt);
            self.redact_optional(&mut image.title);
        }
        for structured in html.structured_data.iter_mut() {
            self.redact_in_place(&mut structured.raw_json);
        }
    }

    /// Rewrite a code-format data-tree node (JSON/YAML/TOML/XML/CSV structural
    /// tree) and its children, bounded by [`MAX_BLOCK_NESTING_DEPTH`].
    #[cfg(feature = "tree-sitter")]
    fn redact_code_data_node(&mut self, node: &mut crate::types::metadata::CodeDataNode) {
        self.redact_code_data_node_at_depth(node, 0);
    }

    #[cfg(feature = "tree-sitter")]
    fn redact_code_data_node_at_depth(&mut self, node: &mut crate::types::metadata::CodeDataNode, depth: usize) {
        self.redact_optional(&mut node.value);
        if depth >= MAX_BLOCK_NESTING_DEPTH {
            return;
        }
        for child in node.children.iter_mut() {
            self.redact_code_data_node_at_depth(child, depth + 1);
        }
    }

    /// Rewrite processing-warning messages: they can embed source paths or other
    /// diagnostic detail carrying a person's name (e.g. a home-directory path).
    fn redact_processing_warnings(&mut self, doc: &mut ExtractedDocument) {
        for warning in doc.processing_warnings.iter_mut() {
            let redacted = self.redact(&warning.message);
            warning.message = std::borrow::Cow::Owned(redacted);
        }
    }

    /// Rewrite a djot block and its children, bounded by
    /// [`MAX_BLOCK_NESTING_DEPTH`].
    fn redact_djot_block(&mut self, block: &mut crate::types::djot::FormattedBlock, depth: usize) {
        for inline in block.inline_content.iter_mut() {
            self.redact_in_place(&mut inline.content);
        }
        self.redact_optional(&mut block.code);
        if depth >= MAX_BLOCK_NESTING_DEPTH {
            return;
        }
        for child in block.children.iter_mut() {
            self.redact_djot_block(child, depth + 1);
        }
    }
}

/// A match is applied only when its span is a valid slice of `text` that still
/// holds exactly the matched text.
///
/// NER backends report offsets the engine did not compute itself; a stale or
/// shifted span must be skipped rather than rewriting an unrelated region of
/// the document — and must not be reported as redacted either.
fn is_applicable(text: &str, m: &PatternMatch) -> bool {
    m.start < m.end
        && m.end <= text.len()
        && text.is_char_boundary(m.start)
        && text.is_char_boundary(m.end)
        && &text[m.start..m.end] == m.text.as_str()
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

/// Compile NER detections into literal matchers applied to every field.
///
/// The backend only sees `ExtractedDocument::content`, so its byte spans cannot
/// address a table cell, a chunk, or a metadata value, and matching by span
/// alone also stops at the *first* mention inside `content` itself. Matching
/// the detected mention as a word-boundary-anchored literal instead redacts it
/// wherever it appears (xberg-io/xberg#200, #202).
///
/// Custom NER labels are honoured (xberg-io/xberg#203), but only when the
/// caller actually asked for that label: a backend is free to invent a category
/// string, and an allowlist keeps an invented one from silently widening
/// redaction.
fn compile_ner_terms(entities: &[Entity], config: &RedactionConfig) -> Vec<(PiiCategory, regex::Regex)> {
    let mut allowed_custom: HashSet<String> = config
        .ner
        .as_ref()
        .map(|ner| {
            ner.custom_labels
                .iter()
                .map(|l| l.trim().to_ascii_lowercase())
                .collect()
        })
        .unwrap_or_default();
    for category in &config.categories {
        if let PiiCategory::Custom(label) = category {
            allowed_custom.insert(label.trim().to_ascii_lowercase());
        }
    }

    let mut seen: HashSet<(PiiCategory, String)> = HashSet::new();
    let mut out: Vec<(PiiCategory, regex::Regex)> = Vec::new();
    for entity in entities {
        let Some(category) = redactable_category(&entity.category, &allowed_custom) else {
            continue;
        };
        let mention = entity.text.trim();
        if mention.is_empty() {
            continue;
        }
        if !seen.insert((category.clone(), mention.to_string())) {
            continue;
        }
        if let Some(regex) = literal_regex(mention) {
            out.push((category, regex));
        }
    }
    out
}

/// Map an NER category onto the PII category it redacts as, or `None` when the
/// category is not NER-redactable.
///
/// Email / Phone / Url deliberately return `None`: the pattern engine detects
/// those more reliably and already covers every field.
fn redactable_category(category: &EntityCategory, allowed_custom: &HashSet<String>) -> Option<PiiCategory> {
    match category {
        EntityCategory::Person => Some(PiiCategory::Person),
        EntityCategory::Organization => Some(PiiCategory::Organization),
        EntityCategory::Location => Some(PiiCategory::Location),
        EntityCategory::Custom(label) => {
            let label = label.trim();
            if label.is_empty() || !allowed_custom.contains(&label.to_ascii_lowercase()) {
                return None;
            }
            Some(PiiCategory::Custom(label.to_string()))
        }
        EntityCategory::Date
        | EntityCategory::Time
        | EntityCategory::Money
        | EntityCategory::Percent
        | EntityCategory::Email
        | EntityCategory::Phone
        | EntityCategory::Url => None,
    }
}

/// Word-boundary-anchored literal matcher for a detected mention.
///
/// Anchors are only added where the mention actually starts or ends on a word
/// character, so mentions wrapped in punctuation still match. Matching is
/// case-sensitive on purpose: a case-insensitive match on a short name would
/// redact ordinary words ("Bill" would eat every "bill"), and destroying the
/// document is not an acceptable price for redacting it.
pub(super) fn literal_regex(mention: &str) -> Option<regex::Regex> {
    let escaped = regex::escape(mention);
    let prefix = if mention.chars().next().is_some_and(is_word_char) {
        r"\b"
    } else {
        ""
    };
    let suffix = if mention.chars().next_back().is_some_and(is_word_char) {
        r"\b"
    } else {
        ""
    };
    regex::Regex::new(&format!("{prefix}{escaped}{suffix}")).ok()
}

fn is_word_char(character: char) -> bool {
    character.is_alphanumeric() || character == '_'
}

/// Scan `text` with pre-compiled `(category, regex)` pairs.
fn scan_regexes<'a>(text: &str, matchers: impl Iterator<Item = (PiiCategory, &'a regex::Regex)>) -> Vec<PatternMatch> {
    let mut out = Vec::new();
    for (category, regex) in matchers {
        for m in regex.find_iter(text) {
            out.push(PatternMatch {
                start: m.start(),
                end: m.end(),
                category: category.clone(),
                text: m.as_str().to_string(),
            });
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

/// Detect entities in `text` with the configured NER backend.
///
/// Custom labels alone are enough to justify the call: a caller can ask for
/// `custom_labels` without asking for PERSON / ORGANIZATION / LOCATION, and
/// skipping the backend in that case meant custom labels were never redacted
/// (xberg-io/xberg#203).
#[cfg(feature = "ner")]
async fn collect_ner_entities(
    text: &str,
    ner_config: &crate::core::config::ner::NerConfig,
    active: &HashSet<PiiCategory>,
) -> Result<Vec<Entity>> {
    let want_person = active.contains(&PiiCategory::Person);
    let want_organization = active.contains(&PiiCategory::Organization);
    let want_location = active.contains(&PiiCategory::Location);
    let want_custom = ner_config.custom_labels.iter().any(|label| !label.trim().is_empty());
    if !(want_person || want_organization || want_location || want_custom) {
        return Ok(Vec::new());
    }

    let mut categories: Vec<EntityCategory> = Vec::new();
    if want_person {
        categories.push(EntityCategory::Person);
    }
    if want_organization {
        categories.push(EntityCategory::Organization);
    }
    if want_location {
        categories.push(EntityCategory::Location);
    }

    let backend = make_ner_backend(ner_config)?;
    backend
        .detect_with_custom(text, &categories, &ner_config.custom_labels)
        .await
}

/// Run entity detection on the kept text layer of each of the document's own pages, and add the
/// mentions to `entities`.
///
/// A kept text layer whose detection fails is removed: the pass cannot know its entities. Returns
/// the count of the removed kept text layers.
#[cfg(feature = "ner")]
async fn collect_kept_text_entities(
    result: &mut ExtractedDocument,
    ner_config: &crate::core::config::ner::NerConfig,
    active: &HashSet<PiiCategory>,
    entities: &mut Vec<Entity>,
) -> usize {
    let Some(pages) = result.pages.as_mut() else {
        return 0;
    };
    let mut withheld = 0;
    for page in pages.iter_mut() {
        let Some(kept_text) = page.native_content.as_deref() else {
            continue;
        };
        match collect_ner_entities(kept_text, ner_config, active).await {
            Ok(found) => entities.extend(found),
            Err(_) => {
                page.native_content = None;
                withheld += 1;
            }
        }
    }
    withheld
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
                Ok(crate::text::ner::gline::get_or_init_backend_blocking(
                    config.model.as_deref(),
                )?)
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

    fn entity(category: EntityCategory, text: &str, start: u32, end: u32) -> Entity {
        Entity {
            category,
            text: text.to_string(),
            start,
            end,
            confidence: Some(0.99),
        }
    }

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
    fn is_applicable_rejects_a_span_that_no_longer_holds_its_text() {
        let text = "Contact Ada Lovelace today";
        let stale = PatternMatch {
            start: 0,
            end: 7,
            category: PiiCategory::Person,
            text: "Ada Lovelace".into(),
        };
        assert!(!is_applicable(text, &stale));

        let out_of_range = PatternMatch {
            start: 100,
            end: 112,
            category: PiiCategory::Person,
            text: "Ada Lovelace".into(),
        };
        assert!(!is_applicable(text, &out_of_range));

        let good = PatternMatch {
            start: 8,
            end: 20,
            category: PiiCategory::Person,
            text: "Ada Lovelace".into(),
        };
        assert!(is_applicable(text, &good));
    }

    #[test]
    fn literal_regex_anchors_on_word_boundaries() {
        let regex = literal_regex("Bill").expect("compiles");
        assert!(regex.is_match("Bill signed"));
        assert!(!regex.is_match("Billingsgate"));
        assert!(!regex.is_match("bill"), "matching must stay case-sensitive");
    }

    #[test]
    fn redactable_category_requires_an_allowlisted_custom_label() {
        let allowed: HashSet<String> = ["treatment".to_string()].into_iter().collect();
        assert_eq!(
            redactable_category(&EntityCategory::Custom("Treatment".into()), &allowed),
            Some(PiiCategory::Custom("Treatment".into()))
        );
        assert_eq!(
            redactable_category(&EntityCategory::Custom("Invented".into()), &allowed),
            None
        );
        assert_eq!(
            redactable_category(&EntityCategory::Person, &allowed),
            Some(PiiCategory::Person)
        );
        assert_eq!(redactable_category(&EntityCategory::Email, &allowed), None);
    }

    /// The capturing variant returns exactly the TokenReplace substitutions:
    /// every token in the rewritten content maps back to the original PII.
    #[cfg(feature = "redaction-rehydrate")]
    #[tokio::test]
    async fn capture_returns_token_to_original_map() {
        let email = "alice@example.com";
        let phone = "+1-555-123-4567";
        let mut doc = ExtractedDocument {
            content: format!("Contact {email} or call {phone}. Again: {email}."),
            ..Default::default()
        };
        let config = RedactionConfig {
            strategy: crate::types::redaction::RedactionStrategy::TokenReplace,
            ..Default::default()
        };

        let map = redact_capturing_rehydration_map(&mut doc, &config)
            .await
            .expect("capture must succeed");

        assert!(
            !doc.content.contains(email),
            "content still holds the email: {}",
            doc.content
        );
        assert_eq!(
            map.values().filter(|v| v.as_str() == email).count(),
            1,
            "repeated originals must dedupe to one token: {map:?}"
        );
        let mut rehydrated = doc.content.clone();
        for (token, original) in &map {
            rehydrated = rehydrated.replace(token, original);
        }
        assert!(
            rehydrated.contains(email) && rehydrated.contains(phone),
            "rehydrated: {rehydrated}"
        );
    }

    /// The spelling of a name in the OCR text, and its spelling in the text layer of the same page.
    const OCR_NAME: &str = "Zamak Quorlim";
    const LAYER_NAME: &str = "Zarnak Quorlim";

    /// A document with one page. `content` is the OCR text of the page, and `kept_text` is the
    /// text layer that the OCR text replaced.
    fn document_with_kept_text(content: &str, kept_text: &str) -> ExtractedDocument {
        ExtractedDocument {
            content: content.to_string(),
            pages: Some(vec![crate::types::PageContent {
                page_number: 1,
                content: content.to_string(),
                tables: Vec::new(),
                image_indices: Vec::new(),
                image_preprocessing: None,
                hierarchy: None,
                is_blank: None,
                layout_regions: None,
                speaker_notes: None,
                section_name: None,
                sheet_name: None,
                ocr_confidence: None,
                native_content: Some(kept_text.to_string()),
            }]),
            ..Default::default()
        }
    }

    /// The kept text layer of the one page of `doc`.
    fn kept_text(doc: &ExtractedDocument) -> Option<&str> {
        let page = &doc.pages.as_ref().expect("the page is kept")[0];
        page.native_content.as_deref()
    }

    /// The count of the warnings that redaction added to `doc`.
    fn redaction_warnings(doc: &ExtractedDocument) -> usize {
        doc.processing_warnings
            .iter()
            .filter(|warning| warning.source == "redaction")
            .count()
    }

    /// A redaction config whose entity backend is a local loopback HTTP stub. The stub answers a
    /// request with the reply of the first needle that the request body holds, and with an empty
    /// entity list when the body holds no needle.
    #[cfg(all(feature = "api", feature = "ner-llm"))]
    async fn stub_entity_redaction_config(replies: &'static [(&'static str, &'static str)]) -> RedactionConfig {
        let app = axum::Router::new().fallback(axum::routing::post(move |body: String| async move {
            let reply = replies
                .iter()
                .find(|entry| body.contains(entry.0))
                .map_or(r#"{"entities":[]}"#, |entry| entry.1);
            axum::response::Json(serde_json::json!({
                "id": "test",
                "object": "chat.completion",
                "created": 0,
                "model": "test",
                "choices": [{
                    "index": 0,
                    "message": { "role": "assistant", "content": reply },
                    "finish_reason": "stop"
                }]
            }))
        }));

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        RedactionConfig {
            ner: Some(crate::core::config::ner::NerConfig {
                backend: crate::core::config::ner::NerBackendKind::Llm,
                llm: Some(crate::core::config::llm::LlmConfig {
                    model: "openai/gpt-4o-mini".to_string(),
                    api_key: Some("test-key".to_string()),
                    base_url: Some(format!("http://{addr}/v1/")),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    /// The reply of the entity stub that reports `OCR_NAME` as a person.
    #[cfg(all(feature = "api", feature = "ner-llm"))]
    const OCR_NAME_REPLY: &str = r#"{"entities":[{"text":"Zamak Quorlim","category":"person"}]}"#;

    /// The reply of the entity stub that reports `LAYER_NAME` as a person.
    #[cfg(all(feature = "api", feature = "ner-llm"))]
    const LAYER_NAME_REPLY: &str = r#"{"entities":[{"text":"Zarnak Quorlim","category":"person"}]}"#;

    #[tokio::test]
    async fn redaction_rewrites_the_kept_text_layer() {
        let email = "alice@example.com";
        let mut doc = document_with_kept_text(
            &format!("OCR reading of {email}."),
            &format!("Text layer with {email}."),
        );

        redact(&mut doc, &RedactionConfig::default())
            .await
            .expect("redaction must succeed");

        let native_content = kept_text(&doc).expect("the kept text layer stays present");
        assert!(
            native_content.starts_with("Text layer with ") && !native_content.contains(email),
            "the kept text layer must be redacted in place: {native_content:?}"
        );
        let page = &doc.pages.as_ref().expect("the page is kept")[0];
        assert!(
            page.content.starts_with("OCR reading of ") && !page.content.contains(email),
            "the page content must be redacted too: {:?}",
            page.content
        );
        assert_eq!(redaction_warnings(&doc), 0, "pattern redaction withholds nothing");
    }

    /// The entity backend reads the OCR text and the kept text layer in separate requests. The
    /// name in the kept text layer has a spelling that the OCR text does not hold.
    #[cfg(all(feature = "api", feature = "ner-llm"))]
    #[tokio::test]
    async fn an_entity_spelled_only_in_the_kept_text_layer_is_redacted_there() {
        let config = stub_entity_redaction_config(&[(LAYER_NAME, LAYER_NAME_REPLY), (OCR_NAME, OCR_NAME_REPLY)]).await;
        let mut doc = document_with_kept_text(
            &format!("Report signed by {OCR_NAME} today"),
            &format!("Report signed by {LAYER_NAME} today"),
        );

        redact(&mut doc, &config).await.expect("redaction must succeed");

        let native_content = kept_text(&doc).expect("the kept text layer stays present");
        assert!(
            !native_content.contains(LAYER_NAME),
            "the name must not stay in the kept text layer: {native_content:?}"
        );
        assert!(
            native_content.starts_with("Report signed by ") && native_content.ends_with(" today"),
            "the text around the name must stay: {native_content:?}"
        );
        assert!(
            !doc.content.contains(OCR_NAME),
            "the content must be redacted too: {:?}",
            doc.content
        );
        assert_eq!(redaction_warnings(&doc), 0, "nothing is withheld");
    }

    /// The entity backend answers the request for the kept text layer with a reply that is not
    /// JSON, so the detection for that text fails.
    #[cfg(all(feature = "api", feature = "ner-llm"))]
    #[tokio::test]
    async fn a_kept_text_layer_whose_entity_detection_fails_is_withheld() {
        let config = stub_entity_redaction_config(&[(LAYER_NAME, "not json")]).await;
        let mut doc = document_with_kept_text(
            &format!("Report signed by {OCR_NAME} today"),
            &format!("Report signed by {LAYER_NAME} today"),
        );

        redact(&mut doc, &config).await.expect("redaction must succeed");

        assert_eq!(kept_text(&doc), None, "an unknown result is not an empty one");
        assert!(
            doc.content.starts_with("Report signed by "),
            "the content stays: {:?}",
            doc.content
        );
        assert_eq!(redaction_warnings(&doc), 1);
    }

    /// With entity detection on, the pass does not run the detection on the kept text layer of an
    /// archive member, so it must not return that text.
    #[cfg(all(feature = "api", feature = "ner-llm"))]
    #[tokio::test]
    async fn entity_redaction_withholds_the_kept_text_layer_of_an_embedded_document() {
        let config = stub_entity_redaction_config(&[]).await;
        let mut doc = ExtractedDocument {
            content: "Archive with one member".to_string(),
            children: Some(vec![crate::types::ArchiveEntry {
                path: "member.pdf".to_string(),
                mime_type: "application/pdf".to_string(),
                result: Box::new(document_with_kept_text("OCR reading", "Text layer")),
            }]),
            ..Default::default()
        };

        redact(&mut doc, &config).await.expect("redaction must succeed");

        let member = &doc.children.as_ref().expect("the member is kept")[0].result;
        assert_eq!(kept_text(member), None);
        assert_eq!(member.content, "OCR reading");
        assert_eq!(redaction_warnings(&doc), 1);
    }

    /// The document of the offset tests: the OCR text and the text layer spell the name differently.
    fn signed_report() -> ExtractedDocument {
        document_with_kept_text(
            &format!("Report signed by {OCR_NAME} today"),
            &format!("Report signed by {LAYER_NAME} today"),
        )
    }

    /// A finding that gives the offsets of `OCR_NAME` in the content of `signed_report`.
    fn offset_finding_of_the_ocr_name() -> ExternalRedactionFinding {
        let start = "Report signed by ".len() as u32;
        ExternalRedactionFinding {
            label: "PERSON".to_string(),
            start: Some(start),
            end: Some(start + OCR_NAME.len() as u32),
            ..Default::default()
        }
    }

    /// The kept text layer is absent, the name is not in `content`, and redaction added one warning.
    fn assert_kept_text_withheld(doc: &ExtractedDocument) {
        assert_eq!(kept_text(doc), None, "the kept text layer must be absent");
        assert!(
            doc.content.starts_with("Report signed by ") && !doc.content.contains(OCR_NAME),
            "the content must be redacted: {:?}",
            doc.content
        );
        assert_eq!(redaction_warnings(doc), 1);
    }

    /// A finding given as offsets names a stretch of `content`. The pass cannot evaluate it on the
    /// kept text layer, which holds another spelling.
    #[tokio::test]
    async fn a_finding_given_as_offsets_withholds_the_kept_text_layer() {
        let mut doc = signed_report();
        let config = RedactionConfig {
            findings: vec![offset_finding_of_the_ocr_name()],
            ..Default::default()
        };

        redact(&mut doc, &config).await.expect("redaction must succeed");

        assert_kept_text_withheld(&doc);
    }

    /// The offset finding comes in the JSON payload of `redact_external`, the route of the bindings.
    #[tokio::test]
    async fn an_offset_finding_in_an_external_redaction_withholds_the_kept_text_layer() {
        let findings = serde_json::to_string(&[offset_finding_of_the_ocr_name()]).expect("the finding serializes");

        let doc = redact_external(signed_report(), RedactionConfig::default(), &findings, None, None)
            .await
            .expect("redaction must succeed");

        assert_kept_text_withheld(&doc);
    }

    /// The offset finding comes with an extraction request, the route of the redaction post-processor.
    #[cfg(feature = "tokio-runtime")]
    #[tokio::test]
    async fn an_offset_finding_in_an_extraction_request_withholds_the_kept_text_layer() {
        let mut doc = signed_report();
        let config = RedactionConfig::default();
        let limits = SecurityLimits::default();
        let request = ExternalRedactionRequest::new(
            vec![offset_finding_of_the_ocr_name()],
            RedactionOffsetEncoding::UnicodeCodePoints,
            10,
            true,
        );

        redact_with_external_findings(&mut doc, &config, &request, &limits)
            .await
            .expect("redaction must succeed");

        assert_kept_text_withheld(&doc);
    }

    /// Entity detection could read the kept text layer, but the offset finding cannot be evaluated
    /// on it, so the pass must not return the kept text with the entity terms only.
    #[cfg(all(feature = "api", feature = "ner-llm"))]
    #[tokio::test]
    async fn an_offset_finding_with_entity_detection_on_withholds_the_kept_text_layer() {
        let mut doc = signed_report();
        let stub = stub_entity_redaction_config(&[(LAYER_NAME, LAYER_NAME_REPLY), (OCR_NAME, OCR_NAME_REPLY)]).await;
        let config = RedactionConfig {
            findings: vec![offset_finding_of_the_ocr_name()],
            ..stub
        };

        redact(&mut doc, &config).await.expect("redaction must succeed");

        assert_kept_text_withheld(&doc);
    }

    /// A finding given as text is a literal that the pass looks for in every field.
    #[tokio::test]
    async fn a_finding_given_as_text_is_applied_to_the_kept_text_layer() {
        let mut doc = document_with_kept_text(
            &format!("Report signed by {OCR_NAME} today"),
            &format!("Report signed by {LAYER_NAME} today"),
        );
        let config = RedactionConfig {
            findings: vec![ExternalRedactionFinding {
                label: "PERSON".to_string(),
                text: Some(LAYER_NAME.to_string()),
                ..Default::default()
            }],
            ..Default::default()
        };

        redact(&mut doc, &config).await.expect("redaction must succeed");

        let native_content = kept_text(&doc).expect("the kept text layer stays present");
        assert!(
            native_content.starts_with("Report signed by ") && !native_content.contains(LAYER_NAME),
            "the kept text layer must be redacted in place: {native_content:?}"
        );
        assert_eq!(redaction_warnings(&doc), 0, "nothing is withheld");
    }

    /// A caller-supplied entity stream covers `content` only, and the function runs no detection.
    #[test]
    fn redaction_with_a_caller_entity_stream_withholds_the_kept_text_layer() {
        let mut doc = document_with_kept_text(
            &format!("Report signed by {OCR_NAME} today"),
            &format!("Report signed by {LAYER_NAME} today"),
        );
        let entities = vec![entity(EntityCategory::Person, OCR_NAME, 17, 30)];

        redact_with_entities(&mut doc, &RedactionConfig::default(), &entities).expect("redaction must succeed");

        assert_eq!(kept_text(&doc), None, "the kept text layer must be absent");
        assert!(
            !doc.content.contains(OCR_NAME),
            "the content must be redacted: {:?}",
            doc.content
        );
        assert_eq!(redaction_warnings(&doc), 1);
    }

    /// Regression for xberg-io/xberg#1223: redaction must mask PII on every
    /// structured surface, not just `content`.
    #[tokio::test]
    async fn redacts_every_text_bearing_field() {
        use crate::types::form_field::PdfFormField;
        use crate::types::uri::{ExtractedUri, UriKind};

        let email = "alice@example.com";
        let mut doc = ExtractedDocument {
            content: format!("Contact {email} for details."),
            tables: vec![crate::types::tables::Table {
                cells: vec![vec!["Name".into(), email.into()]],
                markdown: format!("| Name | {email} |"),
                columns: Some(vec!["Name".into(), email.into()]),
                page_number: 1,
                bounding_box: None,
                ..Default::default()
            }],
            pages: Some(vec![crate::types::PageContent {
                page_number: 1,
                content: format!("Page mentions {email}."),
                tables: vec![std::sync::Arc::new(crate::types::tables::Table {
                    cells: vec![vec!["Name".into(), email.into()]],
                    markdown: format!("| Name | {email} |"),
                    columns: Some(vec!["Name".into(), email.into()]),
                    ..Default::default()
                })],
                image_indices: Vec::new(),
                image_preprocessing: None,
                hierarchy: None,
                is_blank: None,
                layout_regions: None,
                speaker_notes: None,
                section_name: None,
                sheet_name: None,
                ocr_confidence: None,
                native_content: None,
            }]),
            uris: Some(vec![ExtractedUri {
                url: format!("mailto:{email}"),
                label: Some(email.into()),
                page: None,
                kind: UriKind::Email,
            }]),
            form_fields: vec![PdfFormField {
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
            }],
            structured_output: Some(serde_json::json!({ "email": email })),
            ..Default::default()
        };
        doc.metadata.subject = Some(format!("Re: {email}"));
        doc.metadata.created_by = Some(email.to_string());

        let config = RedactionConfig::default();
        redact(&mut doc, &config).await.expect("redaction must succeed");

        let mut leaks: Vec<&str> = Vec::new();
        if doc.content.contains(email) {
            leaks.push("content");
        }
        if doc.tables[0].cells.iter().flatten().any(|c| c.contains(email))
            || doc.tables[0].markdown.contains(email)
            || doc.tables[0]
                .columns
                .as_ref()
                .is_some_and(|columns| columns.iter().any(|column| column.contains(email)))
        {
            leaks.push("tables");
        }
        let page = &doc.pages.as_ref().unwrap()[0];
        if page.content.contains(email) {
            leaks.push("pages");
        }
        if page.tables[0]
            .columns
            .as_ref()
            .is_some_and(|columns| columns.iter().any(|column| column.contains(email)))
        {
            leaks.push("page.tables");
        }
        let uri = &doc.uris.as_ref().unwrap()[0];
        if uri.url.contains(email) || uri.label.as_deref().unwrap_or("").contains(email) {
            leaks.push("uris");
        }
        if doc.form_fields[0].value.as_deref().unwrap_or("").contains(email) {
            leaks.push("form_fields");
        }
        if doc.metadata.subject.as_deref().unwrap_or("").contains(email) {
            leaks.push("metadata.subject");
        }
        if doc.metadata.created_by.as_deref().unwrap_or("").contains(email) {
            leaks.push("metadata.created_by");
        }
        if doc.structured_output.as_ref().unwrap().to_string().contains(email) {
            leaks.push("structured_output");
        }
        assert!(leaks.is_empty(), "PII leaked on fields: {leaks:?}");
    }

    /// xberg-io/xberg#200 — every occurrence of an NER mention must be redacted,
    /// not just the first.
    #[test]
    fn redacts_every_occurrence_of_an_ner_mention() {
        let mut doc = ExtractedDocument {
            content: "Zarnak Quorlim signed. Later Zarnak Quorlim paid. Zarnak Quorlim left.".to_string(),
            ..Default::default()
        };
        let entities = vec![entity(EntityCategory::Person, "Zarnak Quorlim", 0, 14)];

        redact_with_entities(&mut doc, &RedactionConfig::default(), &entities).expect("redaction must succeed");

        assert_eq!(doc.content.matches("[REDACTED]").count(), 3, "content: {}", doc.content);
        assert!(!doc.content.contains("Zarnak Quorlim"), "content: {}", doc.content);
    }

    /// xberg-io/xberg#204 — the report must count findings from every field.
    #[test]
    fn report_counts_findings_from_secondary_fields() {
        let mut doc = ExtractedDocument {
            content: "Zarnak Quorlim signed.".to_string(),
            ..Default::default()
        };
        doc.metadata.title = Some("Zarnak Quorlim".to_string());
        let entities = vec![entity(EntityCategory::Person, "Zarnak Quorlim", 0, 14)];

        redact_with_entities(&mut doc, &RedactionConfig::default(), &entities).expect("redaction must succeed");

        let report = doc.redaction_report.expect("report present");
        assert_eq!(report.total_redacted, 2);
        assert_eq!(report.findings.len(), 2);
    }
}
