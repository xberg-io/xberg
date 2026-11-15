//! Replay the table-aware serializer with its selected word IDs before planning edits.

use super::{
    MAX_TRANSCRIPT_BYTES, OccurrenceEdit, RegionBox, SourceWord,
    projection::{MAX_STOCK_OCCURRENCES, ProjectionWriter, RawStockWord, VerifiedProjection},
};
use crate::ocr::processor::{render_inline_ocr_table_content_with_occurrences, serialize_hocr_occurrences};
use crate::table_core::HocrWord;
use crate::types::OcrTable;

struct InlineStock<'a> {
    words: &'a [HocrWord],
    /// Source-frame bounds in the same order as the serializer input.
    source_bounds: &'a [RegionBox],
    tables: &'a [OcrTable],
    /// Any suffix already attached outside the OCR serializer, such as annotations.
    unowned_suffix: &'a str,
    transcript: &'a str,
}

struct SelectedLine<'a, 'slice> {
    region: RegionBox,
    column: RegionBox,
    occurrences: &'slice [usize],
    sources: [&'slice [SourceWord<'a>]; 2],
    protected: &'slice [RegionBox],
}

fn materialize(transcript: &str, edits: &[OccurrenceEdit<'_>]) -> Option<String> {
    if edits.is_empty() {
        return None;
    }
    let length = edits.iter().try_fold(transcript.len(), |total, edit| {
        total
            .checked_sub(edit.expected.len())?
            .checked_add(edit.replacement.len())
    })?;
    if length > MAX_TRANSCRIPT_BYTES {
        return None;
    }
    let mut revised = String::with_capacity(length);
    let mut cursor = 0;
    for edit in edits {
        if transcript.get(edit.bytes.clone())? != edit.expected {
            return None;
        }
        revised.push_str(transcript.get(cursor..edit.bytes.start)?);
        revised.push_str(edit.replacement);
        cursor = edit.bytes.end;
    }
    revised.push_str(transcript.get(cursor..)?);
    Some(revised)
}

fn project_emitted<'stock, 'a>(
    raw: &'stock [RawStockWord<'a>],
    transcript: &'a str,
    emitted: impl IntoIterator<Item = (usize, std::ops::Range<usize>)>,
) -> Option<VerifiedProjection<'stock, 'a>> {
    let mut writer = ProjectionWriter::new(raw)?;
    let mut cursor = 0;
    for (id, bytes) in emitted {
        if transcript.get(bytes.clone())? != raw.get(id)?.text {
            return None;
        }
        writer.append_unowned(transcript.get(cursor..bytes.start)?)?;
        writer.append_stock(id)?;
        cursor = bytes.end;
    }
    writer.append_unowned(transcript.get(cursor..)?)?;
    writer.verify(transcript)
}

fn assign_selected_owners<'a>(
    mut edits: Vec<OccurrenceEdit<'a>>,
    selected: &[usize],
) -> Option<Vec<OccurrenceEdit<'a>>> {
    for edit in &mut edits {
        edit.selection_index = *selected.get(edit.selection_index)?;
    }
    Some(edits)
}

fn publish_plain_pair(
    document: &mut String,
    page: &mut String,
    document_plan: (&str, &[OccurrenceEdit<'_>]),
    page_plan: (&str, &[OccurrenceEdit<'_>]),
) -> Option<()> {
    let ((stock_document, document_edits), (stock_page, page_edits)) = (document_plan, page_plan);
    if document.as_str() != stock_document || page.as_str() != stock_page {
        return None;
    }
    if !document_edits
        .iter()
        .map(|edit| (edit.selection_index, edit.expected, edit.replacement))
        .eq(page_edits
            .iter()
            .map(|edit| (edit.selection_index, edit.expected, edit.replacement)))
    {
        return None;
    }
    let revised_document = materialize(stock_document, document_edits)?;
    let revised_page = materialize(stock_page, page_edits)?;
    *document = revised_document;
    *page = revised_page;
    Some(())
}

impl<'a> InlineStock<'a> {
    /// Update both plain-text views together, only while they still equal the replayed stock.
    /// The caller retains the existing table, annotation and raw OCR structures. ~keep
    fn reconcile_plain(&self, document: &mut String, page: &mut String, selected: SelectedLine<'a, '_>) -> bool {
        if document.as_str() != self.transcript || page.as_str() != self.transcript {
            return false;
        }
        let Some(edits) = self.plan(
            selected.region,
            selected.column,
            selected.occurrences,
            selected.sources,
            selected.protected,
        ) else {
            return false;
        };
        publish_plain_pair(document, page, (self.transcript, &edits), (self.transcript, &edits)).is_some()
    }

    fn plan(
        &self,
        region: RegionBox,
        column: RegionBox,
        selected: &[usize],
        sources: [&[SourceWord<'a>]; 2],
        protected: &[RegionBox],
    ) -> Option<Vec<OccurrenceEdit<'a>>> {
        if self.words.len() != self.source_bounds.len()
            || self.words.len() > MAX_STOCK_OCCURRENCES
            || self.tables.len() > MAX_STOCK_OCCURRENCES
            || self.transcript.len() > MAX_TRANSCRIPT_BYTES
            || self.words.iter().try_fold(0usize, |n, w| n.checked_add(w.text.len()))? > MAX_TRANSCRIPT_BYTES
            || self
                .tables
                .iter()
                .try_fold(0usize, |n, t| n.checked_add(t.markdown.len()))?
                > MAX_TRANSCRIPT_BYTES
        {
            return None;
        }
        self.words.iter().try_fold(0u32, |height, word| {
            word.left.checked_add(word.width)?;
            word.top.checked_add(word.height)?;
            height.checked_add(word.height)
        })?;
        let expected_body = self.transcript.strip_suffix(self.unowned_suffix)?;
        let words: &'a [HocrWord] = self.words;
        let raw: Vec<_> = words
            .iter()
            .zip(self.source_bounds)
            .map(|(word, &bounds)| RawStockWord {
                text: &word.text,
                bounds,
            })
            .collect();
        let mut emitted = Vec::new();
        let replay = render_inline_ocr_table_content_with_occurrences(self.words, self.tables, &mut |id, bytes| {
            emitted.push((id, bytes));
        });
        if replay != expected_body {
            return None;
        }
        let edits =
            project_emitted(&raw, self.transcript, emitted)?.plan(region, column, selected, sources, protected)?;
        assign_selected_owners(edits, selected)
    }
}

struct HocrStock<'a> {
    html: &'a str,
    unowned_suffix: &'a str,
    transcript: &'a str,
}

fn raw_bounds(word: &HocrWord) -> Option<[u32; 4]> {
    Some([
        word.left,
        word.top,
        word.left.checked_add(word.width)?,
        word.top.checked_add(word.height)?,
    ])
}

impl<'a> HocrStock<'a> {
    fn plan(&self, page: &InlineStock<'a>, selected: &SelectedLine<'a, '_>) -> Option<Vec<OccurrenceEdit<'a>>> {
        if self.transcript.len() > MAX_TRANSCRIPT_BYTES || self.html.len() > 1024 * 1024 {
            return None;
        }
        let body = self.transcript.strip_suffix(self.unowned_suffix)?;
        let receipts = serialize_hocr_occurrences(self.html, None, 1, body)?;
        let mut raw: Vec<_> = receipts
            .iter()
            .map(|receipt| {
                let [left, top, right, bottom] = receipt.bounds;
                Some(RawStockWord {
                    text: body.get(receipt.bytes.clone())?,
                    bounds: RegionBox {
                        left,
                        top,
                        right,
                        bottom,
                    },
                })
            })
            .collect::<Option<_>>()?;
        let mut ids = Vec::new();
        for &id in selected.occurrences {
            let stock = page.words.get(id)?;
            let bounds = raw_bounds(stock)?;
            let same_raw_word = |word: &HocrWord| word.text == stock.text && raw_bounds(word) == Some(bounds);
            if page.words.iter().filter(|word| same_raw_word(word)).count() != 1 {
                return None;
            }
            let mut matches = receipts
                .iter()
                .enumerate()
                .filter(|(index, receipt)| receipt.bounds == bounds && raw[*index].text == stock.text);
            let (index, _) = matches.next()?;
            if matches.next().is_some() {
                return None;
            }
            raw[index].bounds = *page.source_bounds.get(id)?;
            ids.push(index);
        }
        let emitted = receipts
            .into_iter()
            .enumerate()
            .map(|(index, receipt)| (index, receipt.bytes));
        let edits = project_emitted(&raw, self.transcript, emitted)?.plan(
            selected.region,
            selected.column,
            &ids,
            selected.sources,
            selected.protected,
        )?;
        assign_selected_owners(edits, selected.occurrences)
    }

    fn reconcile_with_inline(
        &self,
        stock_page: &InlineStock<'a>,
        document: &mut String,
        page: &mut String,
        selected: SelectedLine<'a, '_>,
    ) -> Option<()> {
        if document.as_str() != self.transcript || page.as_str() != stock_page.transcript {
            return None;
        }
        let page_edits = stock_page.plan(
            selected.region,
            selected.column,
            selected.occurrences,
            selected.sources,
            selected.protected,
        )?;
        let document_edits = self.plan(stock_page, &selected)?;
        publish_plain_pair(
            document,
            page,
            (self.transcript, &document_edits),
            (stock_page.transcript, &page_edits),
        )
    }
}

#[path = "region_serializer_tests.rs"]
mod tests;
