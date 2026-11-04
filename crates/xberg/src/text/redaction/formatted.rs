use super::patterns::PatternMatch;
use std::{cell::Cell, collections::BTreeMap, mem::size_of};

pub(super) enum FormattedPlan {
    Preserve(Vec<PatternMatch>),
    Discard,
    ProcessorDiscard,
    Literal(super::literal_leaf::plan::LiteralPlan),
    Html(HtmlPlan),
}

pub(super) enum PagePlan<'s> {
    Preserve(Vec<PatternMatch>),
    Plain { source: String, matches: Vec<PatternMatch> },
    Html { source: Option<&'s str>, plan: HtmlPlan },
}

#[derive(Default)]
pub(super) struct SurfacePlans<'s> {
    pub formatted: Option<FormattedPlan>,
    pub pages: BTreeMap<usize, PagePlan<'s>>,
}

pub(super) fn content_matches(
    text: &str,
    matches: Vec<PatternMatch>,
    tag_exemptions: bool,
) -> Option<Vec<PatternMatch>> {
    if matches.is_empty() {
        return Some(matches);
    }
    let ranges = crate::rendering::standard_inline_html_ranges(text)?;
    let mut index = 0;
    let mut content = Vec::new();
    for finding in matches {
        while ranges.get(index).is_some_and(|range| range.end <= finding.start) {
            index += 1;
        }
        if let Some(range) = ranges.get(index).filter(|range| range.start < finding.end) {
            if tag_exemptions && finding.start >= range.start && finding.end <= range.end {
                continue;
            }
            return None;
        }
        content.push(finding);
    }
    Some(content)
}

pub(super) fn escaped_plain_markdown(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    for character in text.chars() {
        if character.is_ascii_punctuation() {
            output.push('\\');
        }
        output.push(character);
    }
    output
}

pub(super) struct HtmlBudget {
    work: Cell<usize>,
    bytes: Cell<usize>,
}

pub(super) fn html_budget_error() -> crate::XbergError {
    crate::XbergError::validation("HTML redaction exceeds the verified work or output size limit")
}

impl HtmlBudget {
    pub(super) fn new(limits: &crate::extractors::security::SecurityLimits) -> Self {
        Self {
            work: Cell::new(limits.max_iterations),
            bytes: Cell::new(limits.max_content_size),
        }
    }

    pub(super) fn charge(&self, operations: usize) -> crate::Result<()> {
        self.work
            .set(self.work.get().checked_sub(operations).ok_or_else(html_budget_error)?);
        Ok(())
    }

    fn reserve(&self, bytes: usize) -> crate::Result<()> {
        self.bytes
            .set(self.bytes.get().checked_sub(bytes).ok_or_else(html_budget_error)?);
        Ok(())
    }

    pub(super) fn record(
        &self,
        matches: &mut Vec<PatternMatch>,
        item: super::patterns::BorrowedMatch<'_>,
        category: &crate::types::redaction::PiiCategory,
    ) -> crate::Result<()> {
        let label = match category {
            crate::types::redaction::PiiCategory::Custom(label) => label.len(),
            _ => 0,
        };
        // Bound the retained record, its sort scratch and the selected result. ~keep
        let bytes = (size_of::<PatternMatch>() * 4)
            .checked_add(item.text.len())
            .and_then(|n| n.checked_add(label))
            .ok_or_else(html_budget_error)?;
        self.reserve(bytes)?;
        matches.try_reserve(1).map_err(|_| html_budget_error())?;
        matches.push(PatternMatch {
            start: item.start,
            end: item.end,
            category: category.clone(),
            text: item.text.to_owned(),
        });
        Ok(())
    }
}

pub(super) struct HtmlPlan {
    pub(super) matches: Vec<PatternMatch>,
    pub(super) plain: String,
    output: Option<String>,
}

impl HtmlPlan {
    pub(super) fn new(
        source: &str,
        matches: Vec<PatternMatch>,
        strategy: crate::types::redaction::RedactionStrategy,
        budget: &HtmlBudget,
        render_html: bool,
    ) -> crate::Result<Self> {
        use crate::types::redaction::{PiiCategory, RedactionStrategy};
        if render_html && source.contains('\0') {
            return Err(crate::XbergError::validation(
                "HTML redaction cannot preserve a NUL in plain text",
            ));
        }
        let mut upper = source.len();
        for item in &matches {
            let size = match (strategy, &item.category) {
                (RedactionStrategy::Drop, _) => 0,
                (RedactionStrategy::Mask, _) => "[REDACTED]".len(),
                (RedactionStrategy::Hash, _) => "[HASH:0123456789abcdef]".len(),
                (RedactionStrategy::TokenReplace, PiiCategory::Custom(label)) => {
                    if render_html && label.contains('\0') {
                        return Err(crate::XbergError::validation(
                            "HTML redaction cannot preserve a NUL in a token label",
                        ));
                    }
                    label
                        .len()
                        .checked_add("[_4294967295]".len())
                        .ok_or_else(html_budget_error)?
                }
                (RedactionStrategy::TokenReplace, _) => "[DATE_OF_BIRTH_4294967295]".len(),
            };
            upper = upper
                .checked_sub(item.end - item.start)
                .and_then(|n| n.checked_add(size))
                .ok_or_else(html_budget_error)?;
        }
        let plain = reserve_html_buffer(upper, budget)?;
        let output = if render_html {
            let capacity = upper
                .checked_mul("&amp;".len())
                .and_then(|n| n.checked_add("<pre><span></span></pre>".len()))
                .ok_or_else(html_budget_error)?;
            Some(reserve_html_buffer(capacity, budget)?)
        } else {
            None
        };
        Ok(Self { matches, plain, output })
    }

    pub(super) fn finish(mut self) -> (String, Option<String>) {
        if let Some(output) = &mut self.output {
            output.push_str("<pre><span>");
            for (index, part) in self.plain.split('\r').enumerate() {
                if index != 0 {
                    output.push_str("&#13;");
                }
                crate::rendering::common::push_html_text(output, part);
            }
            output.push_str("</span></pre>");
        }
        (self.plain, self.output)
    }
}

fn reserve_html_buffer(capacity: usize, budget: &HtmlBudget) -> crate::Result<String> {
    budget.reserve(capacity)?;
    let mut output = String::new();
    output.try_reserve_exact(capacity).map_err(|_| html_budget_error())?;
    Ok(output)
}
