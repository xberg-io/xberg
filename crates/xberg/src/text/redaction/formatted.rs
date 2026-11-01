use super::patterns::PatternMatch;
use std::collections::BTreeMap;

pub(super) enum FormattedPlan {
    Preserve(Vec<PatternMatch>),
    Discard,
}

pub(super) enum PagePlan {
    Preserve(Vec<PatternMatch>),
    Plain { source: String, matches: Vec<PatternMatch> },
}

#[derive(Default)]
pub(super) struct SurfacePlans {
    pub formatted: Option<FormattedPlan>,
    pub pages: BTreeMap<usize, PagePlan>,
}

pub(super) fn content_matches(text: &str, matches: Vec<PatternMatch>) -> Option<Vec<PatternMatch>> {
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
            if finding.start >= range.start && finding.end <= range.end {
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
