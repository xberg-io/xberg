use super::{BodyHyperlinks, Paragraph};

#[derive(Default)]
pub(super) struct ExplicitHyperlink {
    depth: usize,
    start: Option<(usize, usize, String)>,
}

impl ExplicitHyperlink {
    pub(super) fn open(&mut self, paragraph: Option<&Paragraph>, target: Option<&str>, index: usize) {
        self.depth += 1;
        self.start = match (self.depth, paragraph, target) {
            (1, Some(paragraph), Some(target)) => Some((index, paragraph.runs.len(), target.to_owned())),
            _ => None,
        };
    }

    pub(super) fn reject_unsupported(&mut self, name: &str) {
        if matches!(
            name,
            "w:fldSimple" | "w:fldChar" | "w:instrText" | "w:p" | "w:tbl" | "w:tc"
        ) {
            self.start = None;
        }
    }

    pub(super) fn close(&mut self, paragraph: Option<&Paragraph>, index: usize, links: &mut BodyHyperlinks) {
        self.depth = self.depth.saturating_sub(1);
        let (Some((start_index, start, target)), Some(paragraph)) = (self.start.take(), paragraph) else {
            return;
        };
        if start_index != index {
            return;
        }
        let Some(runs) = paragraph.runs.get(start..) else {
            return;
        };
        if !runs
            .iter()
            .all(|run| run.math_latex.is_none() && run.hyperlink_url.as_deref() == Some(target.as_str()))
        {
            return;
        }
        let mut text = runs.iter().enumerate().filter(|(_, run)| !run.text.is_empty());
        if let Some((first, _)) = text.next()
            && text.next().is_some()
        {
            links
                .entry(index)
                .or_default()
                .push(start + first..paragraph.runs.len());
        }
    }
}
