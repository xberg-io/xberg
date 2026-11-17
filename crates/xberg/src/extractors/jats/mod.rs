//! JATS (Journal Article Tag Suite) document extractor.
//!
//! This extractor handles JATS XML documents, the standard format for scientific journal articles
//! used by PubMed Central and major academic publishers.
//!
//! It extracts:
//! - Rich metadata (title, subtitle, authors with affiliations, DOI, PII, keywords, dates)
//! - Article abstract (regular and graphical)
//! - Section hierarchy and content (intro, methods, results, discussion)
//! - Paragraphs and text content
//! - Tables with captions
//! - Figures with captions
//! - Citations and references
//! - Supplementary material information

mod elements;
mod metadata;
mod parser;

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::extractors::security::SecurityBudget;
use crate::plugins::{InternalDocumentExtractor, Plugin};
use crate::text::utf8_validation;
use crate::types::Metadata;
use crate::types::internal::InternalDocument;
use crate::types::internal_builder::InternalDocumentBuilder;
use crate::types::metadata::{ContributorRole, FormatMetadata, JatsMetadata};
use crate::types::uri::ExtractedUri;
use async_trait::async_trait;
use quick_xml::events::Event;
#[cfg(feature = "tokio-runtime")]
use std::path::Path;

use crate::utils::xml_utils::EntityReader;

use elements::extract_jats_all_in_one;
use parser::extract_citation_text as jats_extract_citation;
use parser::extract_fig_content as jats_extract_fig;
use parser::extract_formula_latex as jats_extract_formula;
use parser::extract_text_content as jats_extract_text;

/// `ProcessingWarning::source` for every warning this extractor emits (#171).
const JATS_WARNING_SOURCE: &str = "jats";

/// Extract text and inline annotations from a JATS `<p>` element.
///
/// Recognizes:
/// - `<italic>` → italic
/// - `<bold>` → bold
/// - `<underline>` → underline
/// - `<sub>` → subscript
/// - `<sup>` → superscript
/// - `<ext-link>` → link (with xlink:href)
fn extract_para_with_annotations_jats(
    reader: &mut EntityReader<'_>,
    budget: &mut SecurityBudget,
) -> crate::Result<(
    String,
    Vec<crate::types::document_structure::TextAnnotation>,
    Vec<String>,
)> {
    use crate::types::builder;

    let mut text = String::new();
    let mut annotations = Vec::new();
    let mut formulas: Vec<String> = Vec::new();
    let mut depth: u32 = 0;

    let mut inline_stack: Vec<(&'static str, u32, u32, Option<String>)> = Vec::new();

    loop {
        budget.step()?;
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                budget.enter()?;
                depth += 1;

                let name = e.name();
                let tag = crate::utils::xml_tag_name(name.as_ref());
                // A nested block (a list in the paragraph) is set apart from the text before it ~keep
                if parser::BLOCK_ELEMENTS.contains(&e.local_name().as_ref()) {
                    push_collapsed(&mut text, " ");
                }

                match tag.as_ref() {
                    // An article writes its equation inside the sentence that
                    // refers to it. The equation belongs in the formula list, so
                    // it is captured rather than flattened into the prose.
                    "disp-formula" | "inline-formula" => {
                        let latex = jats_extract_formula(reader, budget)?;
                        depth = depth.saturating_sub(1);
                        if !latex.trim().is_empty() {
                            formulas.push(latex.trim().to_string());
                        }
                    }
                    "italic" => {
                        inline_stack.push(("italic", depth, text.len() as u32, None));
                    }
                    "bold" => {
                        inline_stack.push(("bold", depth, text.len() as u32, None));
                    }
                    "underline" => {
                        inline_stack.push(("underline", depth, text.len() as u32, None));
                    }
                    "sub" => {
                        inline_stack.push(("subscript", depth, text.len() as u32, None));
                    }
                    "sup" => {
                        inline_stack.push(("superscript", depth, text.len() as u32, None));
                    }
                    "ext-link" => {
                        let mut href = None;
                        for attr in e.attributes().flatten() {
                            let key = std::borrow::Cow::Borrowed(attr.key.as_ref());
                            let val = std::borrow::Cow::Borrowed(attr.value.as_ref());
                            budget.check_attr(&key, &val)?;
                            if key == "xlink:href" || key.ends_with(":href") || key == "href" {
                                href = Some(val.to_string());
                            }
                        }
                        inline_stack.push(("link", depth, text.len() as u32, href));
                    }
                    _ => {}
                }
            }
            // An explicit line break (`<bold>first<break/>second</bold>`) separates the words ~keep
            Ok(Event::Empty(e)) if parser::BLOCK_ELEMENTS.contains(&e.local_name().as_ref()) => {
                push_collapsed(&mut text, " ");
            }
            Ok(Event::End(e)) => {
                budget.leave();
                if depth == 0 {
                    break;
                }
                if parser::BLOCK_ELEMENTS.contains(&e.local_name().as_ref()) {
                    push_collapsed(&mut text, " ");
                }

                if let Some(&(kind, open_depth, start, ref href)) = inline_stack.last()
                    && open_depth == depth
                {
                    let text_end = text.len() as u32;
                    // The span without the whitespace around it ~keep
                    let (actual_start, end) = if (start as usize) < text.len() {
                        let span = &text[start as usize..text_end as usize];
                        let leading = (span.len() - span.trim_start().len()) as u32;
                        let trailing = (span.len() - span.trim_end().len()) as u32;
                        (start + leading, text_end.saturating_sub(trailing).max(start + leading))
                    } else {
                        (start, text_end)
                    };
                    if end > actual_start {
                        let href_clone = href.clone();
                        let annotation = match kind {
                            "bold" => builder::bold(actual_start, end),
                            "italic" => builder::italic(actual_start, end),
                            "underline" => builder::underline(actual_start, end),
                            "subscript" => builder::subscript(actual_start, end),
                            "superscript" => builder::superscript(actual_start, end),
                            "link" => {
                                let url = href_clone.as_deref().unwrap_or("");
                                builder::link(actual_start, end, url, None)
                            }
                            // `inline_stack` is only ever pushed to with "italic", "bold",
                            // "underline", "subscript" (from "sub"), "superscript" (from
                            // "sup"), or "link" (from "ext-link") — every one of those six
                            // kinds is handled above, so no other value can appear.
                            _ => unreachable!("inline_stack only ever holds the six kinds handled above"),
                        };
                        annotations.push(annotation);
                    }
                    inline_stack.pop();
                }

                depth -= 1;
            }
            Ok(Event::Text(t)) => {
                // Text joins the text around it as written, so `H<sub>2</sub>O` reads `H2O`; ~keep
                // whitespace from the XML layout collapses to one space ~keep
                let decoded = t.as_ref().to_string();
                if !decoded.trim().is_empty() {
                    budget.check_entity(decoded.trim())?;
                    budget.account_text(decoded.trim().len())?;
                }
                push_collapsed(&mut text, &decoded);
            }
            Ok(Event::CData(t)) => {
                let decoded = t.as_ref().to_string();
                if !decoded.trim().is_empty() {
                    budget.check_entity(decoded.trim())?;
                    budget.account_text(decoded.trim().len())?;
                    if !text.is_empty() {
                        text.push(' ');
                    }
                    text.push_str(decoded.trim());
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(crate::error::XbergError::parsing(format!("XML parsing error: {}", e)));
            }
            _ => {}
        }
    }

    Ok((text.trim().to_string(), annotations, formulas))
}

/// Append `fragment` to `text` with each run of whitespace collapsed to one space, keeping a
/// space at either end only where `fragment` has whitespace there.
fn push_collapsed(text: &mut String, fragment: &str) {
    let words: Vec<&str> = fragment.split_whitespace().collect();
    let separated = |text: &String| text.is_empty() || text.ends_with([' ', '\n']);
    if words.is_empty() {
        if !fragment.is_empty() && !separated(text) {
            text.push(' ');
        }
        return;
    }
    if fragment.starts_with(char::is_whitespace) && !separated(text) {
        text.push(' ');
    }
    text.push_str(&words.join(" "));
    if fragment.ends_with(char::is_whitespace) {
        text.push(' ');
    }
}

/// Build an `InternalDocument` from JATS XML content.
fn build_jats_internal_document(content: &str, budget: &mut SecurityBudget) -> crate::Result<InternalDocument> {
    let mut reader = EntityReader::from_str(content);
    let mut builder = InternalDocumentBuilder::new("jats");

    let mut in_article_meta = false;
    let mut in_abstract = false;
    let mut in_body = false;
    let mut in_back = false;
    let mut in_ref_list = false;
    let mut in_table = false;
    let mut in_thead = false;
    let mut in_tbody = false;
    let mut in_row = false;
    let mut current_table: Vec<Vec<String>> = Vec::new();
    let mut current_row: Vec<String> = Vec::new();
    let mut sec_depth: u32 = 0;
    let mut ref_list_opened = false;
    let mut back_list_opened = false;

    loop {
        budget.step()?;
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                budget.enter()?;
                let name = e.name();
                let tag = crate::utils::xml_tag_name(name.as_ref());

                match tag.as_ref() {
                    "article-meta" => {
                        in_article_meta = true;
                    }
                    "article-title" if in_article_meta && !in_abstract => {
                        let text = jats_extract_text(&mut reader, budget)?;
                        if !text.is_empty() {
                            builder.push_heading(1, &text, None, None);
                        }
                        continue;
                    }
                    "abstract" if in_article_meta => {
                        in_abstract = true;
                        builder.push_heading(2, "Abstract", None, None);
                    }
                    "sec" if in_abstract => {}
                    "title" if in_abstract => {
                        let text = jats_extract_text(&mut reader, budget)?;
                        if !text.is_empty() {
                            builder.push_heading(3, &text, None, None);
                        }
                        continue;
                    }
                    "p" if in_abstract => {
                        let (text, annotations, para_formulas) =
                            extract_para_with_annotations_jats(&mut reader, budget)?;
                        if !text.is_empty() {
                            builder.push_paragraph(&text, annotations, None, None);
                        }
                        // The equation belongs where the sentence put it, so it
                        // follows the block that holds it. A paragraph that is only
                        // an equation has no text, and still has its formula.
                        for latex in &para_formulas {
                            builder.push_formula(latex, None, None);
                        }
                        continue;
                    }
                    "body" => {
                        in_body = true;
                        sec_depth = 0;
                    }
                    "sec" if in_body => {
                        sec_depth += 1;
                    }
                    "title" if in_body && !in_article_meta && !in_ref_list => {
                        let text = jats_extract_text(&mut reader, budget)?;
                        if !text.is_empty() {
                            let level = (sec_depth + 1).min(6) as u8;
                            builder.push_heading(level, &text, None, None);
                        }
                        continue;
                    }
                    "p" if in_body => {
                        let (text, annotations, para_formulas) =
                            extract_para_with_annotations_jats(&mut reader, budget)?;
                        if !text.is_empty() {
                            for ann in &annotations {
                                if let crate::types::document_structure::AnnotationKind::Link { url, .. } = &ann.kind
                                    && !url.is_empty()
                                {
                                    let label = text.get(ann.start as usize..ann.end as usize).map(|s| s.to_string());
                                    builder.push_uri(ExtractedUri::hyperlink(url, label));
                                }
                            }
                            builder.push_paragraph(&text, annotations, None, None);
                        }
                        // The equation belongs where the sentence put it, so it
                        // follows the block that holds it. A paragraph that is only
                        // an equation has no text, and still has its formula.
                        for latex in &para_formulas {
                            builder.push_formula(latex, None, None);
                        }
                        continue;
                    }
                    "fig" if in_body => {
                        let (label, caption, href) = jats_extract_fig(&mut reader, budget)?;
                        let caption_full = match (&label, &caption) {
                            (Some(l), Some(c)) => format!("{}: {}", l, c),
                            (Some(l), None) => l.clone(),
                            (None, Some(c)) => c.clone(),
                            (None, None) => String::new(),
                        };
                        if !caption_full.is_empty() || href.is_some() {
                            let display = match (&href, caption_full.is_empty()) {
                                (Some(h), false) => format!("![{}]({})", caption_full, h),
                                (Some(h), true) => format!("![]({})", h),
                                (None, false) => caption_full.clone(),
                                (None, true) => String::new(),
                            };
                            if !display.is_empty() {
                                builder.push_paragraph(&display, Vec::new(), None, None);
                            }
                            if let Some(href) = &href {
                                let label_opt = if caption_full.is_empty() {
                                    None
                                } else {
                                    Some(caption_full.clone())
                                };
                                builder.push_uri(ExtractedUri::image(href, label_opt));
                            }
                        }
                        continue;
                    }
                    "disp-formula" | "inline-formula" if in_body => {
                        let latex = jats_extract_formula(&mut reader, budget)?;
                        if !latex.is_empty() {
                            builder.push_formula(&latex, None, None);
                        }
                        continue;
                    }
                    "back" => {
                        in_back = true;
                    }
                    "ack" if in_back => {}
                    "supplementary-material" if in_back => {
                        let _ = jats_extract_text(&mut reader, budget)?;
                        continue;
                    }
                    "title" if in_back && !in_ref_list => {
                        let text = jats_extract_text(&mut reader, budget)?;
                        if !text.is_empty() {
                            builder.push_heading(2, &text, None, None);
                        }
                        continue;
                    }
                    "p" if in_back && !in_ref_list => {
                        let (text, annotations, para_formulas) =
                            extract_para_with_annotations_jats(&mut reader, budget)?;
                        if !text.is_empty() {
                            builder.push_paragraph(&text, annotations, None, None);
                        }
                        // The equation belongs where the sentence put it, so it
                        // follows the block that holds it. A paragraph that is only
                        // an equation has no text, and still has its formula.
                        for latex in &para_formulas {
                            builder.push_formula(latex, None, None);
                        }
                        continue;
                    }
                    "term" if in_back && !in_ref_list => {
                        let text = jats_extract_text(&mut reader, budget)?;
                        if !text.is_empty() {
                            builder.push_definition_term(&text, None);
                        }
                        continue;
                    }
                    "list-item" if in_back && !in_ref_list => {
                        let (text, annotations, para_formulas) =
                            extract_para_with_annotations_jats(&mut reader, budget)?;
                        if !text.is_empty() {
                            if !back_list_opened {
                                builder.push_list(false);
                                back_list_opened = true;
                            }
                            builder.push_list_item(&text, false, annotations, None, None);
                        }
                        // The equation belongs where the sentence put it, so it
                        // follows the block that holds it. A paragraph that is only
                        // an equation has no text, and still has its formula.
                        for latex in &para_formulas {
                            builder.push_formula(latex, None, None);
                        }
                        continue;
                    }
                    "table" => {
                        in_table = true;
                        current_table.clear();
                    }
                    "thead" if in_table => {
                        in_thead = true;
                    }
                    "tbody" if in_table => {
                        in_tbody = true;
                    }
                    // A row can sit in `thead`, `tbody` or `tfoot`, or directly in `table` ~keep
                    "tr" if in_table => {
                        in_row = true;
                        current_row.clear();
                    }
                    "td" | "th" if in_row => {
                        let text = jats_extract_text(&mut reader, budget)?;
                        budget.add_cells(1)?;
                        current_row.push(text);
                        continue;
                    }
                    "ref-list" => {
                        in_ref_list = true;
                    }
                    "title" if in_ref_list => {
                        let text = jats_extract_text(&mut reader, budget)?;
                        if !text.is_empty() {
                            builder.push_heading(2, &text, None, None);
                        }
                        continue;
                    }
                    "ref" if in_ref_list => {
                        let text = jats_extract_citation(&mut reader, budget)?;
                        if !text.is_empty() {
                            if !ref_list_opened {
                                builder.push_list(true);
                                ref_list_opened = true;
                            }
                            builder.push_list_item(&text, true, Vec::new(), None, None);
                        }
                        continue;
                    }
                    _ => {}
                }
            }
            Ok(Event::End(e)) => {
                budget.leave();
                let name = e.name();
                let tag = crate::utils::xml_tag_name(name.as_ref());

                match tag.as_ref() {
                    "article-meta" => {
                        in_article_meta = false;
                    }
                    "abstract" if in_abstract => {
                        in_abstract = false;
                    }
                    "body" => {
                        in_body = false;
                    }
                    "sec" if in_body && sec_depth > 0 => {
                        sec_depth -= 1;
                    }
                    "back" => {
                        in_back = false;
                    }
                    "list" if back_list_opened => {
                        builder.end_list();
                        back_list_opened = false;
                    }
                    "ref-list" => {
                        if ref_list_opened {
                            builder.end_list();
                            ref_list_opened = false;
                        }
                        in_ref_list = false;
                    }
                    "table" if in_table => {
                        if !current_table.is_empty() {
                            builder.push_table_from_cells(&current_table, None, None);
                            current_table.clear();
                        }
                        in_table = false;
                    }
                    "thead" if in_thead => {
                        in_thead = false;
                    }
                    "tbody" if in_tbody => {
                        in_tbody = false;
                    }
                    "tr" if in_row => {
                        if !current_row.is_empty() {
                            current_table.push(std::mem::take(&mut current_row));
                        }
                        in_row = false;
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(crate::error::XbergError::parsing(format!("XML parsing error: {}", e)));
            }
            _ => {}
        }
    }

    Ok(builder.build())
}

/// JATS document extractor.
///
/// Supports JATS (Journal Article Tag Suite) XML documents in various versions,
/// handling both the full article structure and minimal JATS subsets.
#[cfg_attr(alef, alef(skip))]
pub struct JatsExtractor;

impl Default for JatsExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl JatsExtractor {
    pub(crate) fn new() -> Self {
        Self
    }
}

impl Plugin for JatsExtractor {
    fn name(&self) -> &str {
        "jats-extractor"
    }

    fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    fn initialize(&self) -> Result<()> {
        Ok(())
    }

    fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl InternalDocumentExtractor for JatsExtractor {
    #[cfg_attr(
        feature = "otel",
        tracing::instrument(
            skip(self, content, config),
            fields(
                extractor.name = self.name(),
                content.size_bytes = content.len(),
            )
        )
    )]
    async fn extract_content(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<InternalDocument> {
        tracing::debug!(format = "jats", size_bytes = content.len(), "extraction starting");
        // Track the fallback: a non-UTF-8 article is decoded lossily and every byte the
        // decoder could not read is already U+FFFD before parsing starts (#171).
        let (jats_content, decoded_lossily) = match utf8_validation::from_utf8(content) {
            Ok(valid) => (valid.to_string(), false),
            Err(_) => (String::from_utf8_lossy(content).to_string(), true),
        };

        let (jats_metadata, _extracted_content, _title, _tables) = extract_jats_all_in_one(&jats_content)?;

        let mut metadata = Metadata::default();
        let mut subject_parts = Vec::new();

        if !jats_metadata.title.is_empty() {
            metadata.title = Some(jats_metadata.title.clone());
            metadata.subject = Some(jats_metadata.title.clone());
            subject_parts.push(format!("Title: {}", jats_metadata.title));
        }

        if let Some(subtitle) = &jats_metadata.subtitle {
            subject_parts.push(format!("Subtitle: {}", subtitle));
        }

        if !jats_metadata.authors.is_empty() {
            metadata.authors = Some(jats_metadata.authors.clone());
            subject_parts.push(format!("Authors: {}", jats_metadata.authors.join("; ")));
        }

        if !jats_metadata.affiliations.is_empty() {
            subject_parts.push(format!("Affiliations: {}", jats_metadata.affiliations.join("; ")));
        }

        if let Some(doi) = &jats_metadata.doi {
            subject_parts.push(format!("DOI: {}", doi));
        }

        if let Some(pii) = &jats_metadata.pii {
            subject_parts.push(format!("PII: {}", pii));
        }

        if !jats_metadata.keywords.is_empty() {
            metadata.keywords = Some(jats_metadata.keywords.clone());
            subject_parts.push(format!("Keywords: {}", jats_metadata.keywords.join("; ")));
        }

        if let Some(date) = &jats_metadata.publication_date {
            metadata.created_at = Some(date.clone());
            subject_parts.push(format!("Publication Date: {}", date));
        }

        if let Some(volume) = &jats_metadata.volume {
            subject_parts.push(format!("Volume: {}", volume));
        }

        if let Some(issue) = &jats_metadata.issue {
            subject_parts.push(format!("Issue: {}", issue));
        }

        if let Some(pages) = &jats_metadata.pages {
            subject_parts.push(format!("Pages: {}", pages));
        }

        if let Some(journal_title) = &jats_metadata.journal_title {
            subject_parts.push(format!("Journal: {}", journal_title));
        }

        if let Some(article_type) = &jats_metadata.article_type {
            subject_parts.push(format!("Article Type: {}", article_type));
        }

        if let Some(abstract_text) = &jats_metadata.abstract_text {
            subject_parts.push(format!("Abstract: {}", abstract_text));
        }

        if let Some(corresp_author) = &jats_metadata.corresponding_author {
            subject_parts.push(format!("Corresponding Author: {}", corresp_author));
        }

        let mut history_dates = std::collections::BTreeMap::new();
        if !jats_metadata.history_dates.is_empty() {
            for (date_type, date_val) in &jats_metadata.history_dates {
                let mut chars = date_type.chars();
                let capitalized_date_type = match chars.next() {
                    Some(first_char) => first_char.to_uppercase().collect::<String>() + chars.as_str(),
                    None => String::new(),
                };
                subject_parts.push(format!("{}: {}", capitalized_date_type, date_val));
                history_dates.insert(date_type.clone(), date_val.clone());
            }
        }

        let copyright = if let Some(copyright) = &jats_metadata.copyright_statement {
            subject_parts.push(format!("Copyright: {}", copyright));
            Some(copyright.clone())
        } else {
            None
        };

        let license = jats_metadata.license.clone();

        let contributor_roles: Vec<ContributorRole> = jats_metadata
            .contributor_roles
            .iter()
            .map(|(name, role)| ContributorRole {
                name: name.clone(),
                role: if role.is_empty() { None } else { Some(role.clone()) },
            })
            .collect();

        let jats_typed_metadata = JatsMetadata {
            copyright,
            license,
            history_dates,
            contributor_roles,
        };
        metadata.format = Some(FormatMetadata::Jats(jats_typed_metadata));

        if !subject_parts.is_empty() {
            metadata.subject = Some(subject_parts.join(" | "));
        }

        let mut budget = SecurityBudget::from_config(config);
        let mut doc = build_jats_internal_document(&jats_content, &mut budget)?;
        doc.mime_type = mime_type.to_string();
        doc.metadata = metadata;

        if decoded_lossily {
            crate::core::diagnostics::push_lossy_decode_warning(
                &mut doc.processing_warnings,
                JATS_WARNING_SOURCE,
                "JATS source",
            );
        }

        if let Some(doi) = &jats_metadata.doi {
            doc.push_uri(ExtractedUri::citation(
                format!("https://doi.org/{}", doi),
                Some(format!("DOI: {}", doi)),
            ));
        }

        tracing::debug!(
            element_count = doc.elements.len(),
            format = "jats",
            "extraction complete"
        );
        Ok(doc)
    }

    #[cfg(feature = "tokio-runtime")]
    #[cfg_attr(
        feature = "otel",
        tracing::instrument(
            skip(self, path, config),
            fields(
                extractor.name = self.name(),
            )
        )
    )]
    async fn extract_path(&self, path: &Path, mime_type: &str, config: &ExtractionConfig) -> Result<InternalDocument> {
        let bytes = crate::core::io::read_file_async(path).await?;
        self.extract_content(&bytes, mime_type, config).await
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["application/x-jats+xml", "text/jats"]
    }

    fn priority(&self) -> i32 {
        50
    }
}

#[cfg(test)]
mod tests;
