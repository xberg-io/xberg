use super::{
    InternalDocument, OcrBoundingGeometry, OcrElement, PdfParagraph, is_prunable_element, paragraph_segments,
    strip_matching_tokens,
};

fn paragraph_rows(
    evidence: &PcbRow,
    paragraphs: &[PdfParagraph],
) -> Option<(std::ops::Range<usize>, std::ops::Range<usize>)> {
    let texts = evidence.reconcile(paragraphs.iter().map(|paragraph| paragraph.text.as_str()))?;
    let segments = evidence.reconcile(
        paragraphs
            .iter()
            .flat_map(|paragraph| &paragraph.lines)
            .flat_map(|line| &line.segments)
            .map(|segment| segment.text.as_str()),
    )?;
    let (mut text_offset, mut segment_offset) = (0, 0);
    for paragraph in paragraphs.iter() {
        let (text_start, segment_start) = (text_offset, segment_offset);
        text_offset += paragraph.text.split_whitespace().count();
        segment_offset += paragraph_segments(paragraph).count();
        let touches_text = text_start < texts.end && text_offset > texts.start;
        let touches_segments = segment_start < segments.end && segment_offset > segments.start;
        if (touches_text || touches_segments) && (paragraph.is_code_block || paragraph.is_formula) {
            return None;
        }
    }
    Some((texts, segments))
}

const PCB_ROW: [&str; 11] = [
    "cece'", "cocece", "ceeces", "coecee", "wecece", "cceece", "ccccee", "cocece", "eeeece", "eeecce", "ececee",
];

fn pcb_rows<'a>(text: impl Iterator<Item = &'a str>) -> Vec<std::ops::Range<usize>> {
    let tokens: Vec<_> = text.flat_map(str::split_whitespace).collect();
    tokens
        .windows(PCB_ROW.len())
        .enumerate()
        .filter(|(_, row)| matches!(row[0], "cece'" | "cece’") && row[1..] == PCB_ROW[1..])
        .map(|(start, _)| start..start + PCB_ROW.len())
        .collect()
}

pub(in crate::extractors::pdf::ocr) struct PcbRow {
    source: std::ops::Range<usize>,
    unique_source: bool,
}

impl PcbRow {
    pub(super) fn capture<'a>(transcript: &str, lines: impl Iterator<Item = &'a OcrElement>) -> Option<Self> {
        let lines: Vec<_> = lines.collect();
        let transcript_rows = pcb_rows(std::iter::once(transcript));
        let line_rows = pcb_rows(lines.iter().map(|line| line.text.as_str()));
        if transcript_rows.len() != line_rows.len() {
            return None;
        }
        let words: Vec<_> = lines
            .iter()
            .flat_map(|line| line.text.split_whitespace().map(move |_| *line))
            .collect();
        let mut certified = line_rows.iter().enumerate().filter(|(_, range)| {
            let mut right = 0;
            words[range.start..range.end].iter().all(|line| {
                let OcrBoundingGeometry::Rectangle { left, width, .. } = line.geometry else {
                    return false;
                };
                let valid = left >= right
                    && line.text.split_whitespace().count() == 1
                    && crate::extractors::pdf::ocr::symbol_noise_background::contains_pcb_strip(&line.geometry);
                right = left.saturating_add(width);
                valid
            })
        });
        // OCR order identifies the occurrence in the unchanged transcript. Structured views
        // can reorder matching rows and must resolve a unique match independently. ~keep
        let (ordinal, _) = certified.next()?;
        certified.next().is_none().then(|| Self {
            source: transcript_rows[ordinal].clone(),
            unique_source: transcript_rows.len() == 1,
        })
    }

    fn reconcile<'a>(&self, text: impl Iterator<Item = &'a str>) -> Option<std::ops::Range<usize>> {
        let rows = pcb_rows(text);
        (self.unique_source && rows.len() == 1).then(|| rows[0].clone())
    }
}

fn strip_pcb_row(text: &str, start: &mut usize, row: &std::ops::Range<usize>) -> String {
    strip_matching_tokens(text, |_| {
        let remove = row.contains(start);
        *start += 1;
        remove
    })
    .0
}

pub(in crate::extractors::pdf::ocr) fn prune_pcb_row(
    evidence: &PcbRow,
    text: &mut String,
    read: Option<&mut String>,
    paragraphs: Option<&mut Vec<PdfParagraph>>,
    document: Option<&mut InternalDocument>,
) -> usize {
    let row = &evidence.source;
    let paragraph_rows = paragraphs
        .as_ref()
        .map(|paragraphs| paragraph_rows(evidence, paragraphs));
    let document_row = document
        .as_ref()
        .map(|document| evidence.reconcile(document.elements.iter().map(|element| element.text.as_ref())));
    if paragraph_rows == Some(None)
        || document_row == Some(None)
        || document.as_ref().is_some_and(|document| {
            let Some(Some(row)) = document_row.as_ref() else {
                return false;
            };
            let mut offset = 0;
            document.elements.iter().any(|element| {
                let start = offset;
                offset += element.text.split_whitespace().count();
                start < row.end
                    && offset > row.start
                    && (!is_prunable_element(element) || !element.annotations.is_empty())
            })
        })
    {
        return 0;
    }
    #[cfg(test)]
    let _ = super::REPLAY.try_with(|replay| {
        if let Some(tokens) = replay {
            tokens.lock().extend(
                text.split_whitespace()
                    .skip(row.start)
                    .take(row.len())
                    .map(str::to_owned),
            );
        }
    });
    *text = strip_pcb_row(text, &mut 0, row);
    if let Some(read) = read {
        *read = strip_pcb_row(read, &mut 0, row);
    }
    if let (Some(paragraphs), Some(Some((texts, segments)))) = (paragraphs, paragraph_rows) {
        let (mut text_index, mut segment_index) = (0, 0);
        paragraphs.retain_mut(|paragraph| {
            paragraph.text = strip_pcb_row(&paragraph.text, &mut text_index, &texts);
            paragraph.lines.retain_mut(|line| {
                line.segments.retain_mut(|segment| {
                    segment.text = strip_pcb_row(&segment.text, &mut segment_index, &segments);
                    !segment.text.is_empty()
                });
                !line.segments.is_empty()
            });
            paragraph.word_count = PdfParagraph::compute_word_count(&paragraph.text, &paragraph.lines);
            paragraph.word_count != 0
        });
    }
    if let (Some(document), Some(Some(row))) = (document, document_row) {
        let mut index = 0;
        document.elements.retain_mut(|element| {
            let before = index;
            let text = strip_pcb_row(&element.text, &mut index, &row);
            let touched = before < row.end && index > row.start;
            if touched {
                element.text = text;
            }
            !touched || !element.text.is_empty()
        });
    }
    PCB_ROW.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::OcrElementLevel;

    fn lines_for(top: u32) -> impl Iterator<Item = OcrElement> {
        PCB_ROW.into_iter().enumerate().map(move |(index, token)| {
            let mut line = OcrElement::new(
                token,
                OcrBoundingGeometry::Rectangle {
                    left: 423 + index as u32 * 110,
                    top,
                    width: 85,
                    height: 8,
                },
                Default::default(),
            );
            line.level = OcrElementLevel::Line;
            line
        })
    }

    #[test]
    fn pcb_certificate_removes_only_the_row_owned_by_boxes_inside_the_strip() {
        use crate::types::internal::{ElementKind, InternalElement};
        let row = PCB_ROW.join(" ");
        let original = format!("{row}\n33\n{row}\ncocece");
        let outside: Vec<_> = lines_for(700).collect();
        assert!(PcbRow::capture(&row, outside.iter()).is_none());

        let lines: Vec<_> = lines_for(700).chain(lines_for(555)).collect();
        let evidence = PcbRow::capture(&original, lines.iter()).expect("second row has certified boxes");
        let mut text = original.clone();
        let mut read = original.clone();
        let mut document = InternalDocument::new("pdf");
        let reordered = format!("{row}\ncocece\n{row}\n33");
        document
            .elements
            .push(InternalElement::text(ElementKind::Paragraph, &reordered, 0));
        assert_eq!(
            prune_pcb_row(&evidence, &mut text, Some(&mut read), None, Some(&mut document)),
            0
        );
        assert_eq!(text, original);
        assert_eq!(read, original);
        assert_eq!(document.elements[0].text, reordered);
        assert_eq!(prune_pcb_row(&evidence, &mut text, Some(&mut read), None, None), 11);
        let survivor = format!("{row}\n33\ncocece");
        assert_eq!(text, survivor);
        assert_eq!(read, survivor);
    }
    #[test]
    fn mixed_pruning_preserves_a_duplicate_of_a_removed_row_token() {
        use crate::types::internal::{ElementKind, InternalElement};
        let row = PCB_ROW.join(" ");
        let mut text = format!("{row}\n33\nceeces\ncocece\nlonger");
        let lines: Vec<_> = lines_for(555).collect();
        let evidence = PcbRow::capture(&text, lines.iter()).unwrap();
        let mut read = text.clone();
        let mut document = InternalDocument::new("pdf");
        document
            .elements
            .push(InternalElement::text(ElementKind::Paragraph, &text, 0));
        assert_eq!(
            super::super::prune_mixed(
                &mut text,
                Some(&mut read),
                None,
                Some(&mut document),
                super::super::MixedPruningEvidence {
                    candidates: &["ceeces", "longer"],
                    retained: None,
                    pcb_row: Some(&evidence),
                }
            ),
            (0, 1, 11)
        );
        assert_eq!(text, "33\nceeces\ncocece");
        assert_eq!(read, text);
        assert_eq!(document.elements[0].text, text);
    }
}
