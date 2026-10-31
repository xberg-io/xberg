//! Recover cell scripts from original typography before the table word fusion discards it. ~keep

use super::super::span_geometry::page_advance_midpoint;
use super::{NativeDocument, is_script_run_of, sort_spans_in_reading_order};
use crate::rendering::table_scripts::CellScriptAnnotation;
use crate::types::{AnnotationKind, ProcessingWarning, TextAnnotation};
use xberg_native_pdf::layout::TextSpan;
use xberg_native_pdf::structure::table_extractor::{Table, TableCell};

#[derive(Default)]
pub(super) struct PageSpans {
    spans: Vec<TextSpan>,
    margin: f32,
}

#[derive(Debug)]
pub(crate) struct TableScripts {
    pub cells: Vec<Vec<String>>,
    pub annotations: Vec<CellScriptAnnotation>,
}

pub(super) fn page_spans(doc: &NativeDocument, page: usize, warnings: &mut Vec<ProcessingWarning>) -> PageSpans {
    match doc
        .doc
        .extract_page_text_with_script_geometry(page, xberg_native_pdf::document::ReadingOrder::TopToBottom)
    {
        Ok(text) => {
            let mut spans: Vec<_> = text
                .spans
                .into_iter()
                .map(with_effective_baseline)
                .filter(|span| page_advance_midpoint(span).1.is_finite() && span.font_size.is_finite())
                .collect();
            spans.sort_by(|a, b| page_advance_midpoint(a).1.total_cmp(&page_advance_midpoint(b).1));
            let margin = spans.iter().map(|span| span.font_size).fold(0.0f32, f32::max)
                * crate::script_run::SCRIPT_RUN_MAX_BASELINE_FONT_FACTOR;
            PageSpans { spans, margin }
        }
        Err(error) => {
            warnings.push(ProcessingWarning {
                source: "pdf_tables".into(),
                message: format!(
                    "table script geometry unavailable on page {}: {error}; cell text was preserved",
                    page + 1
                )
                .into(),
            });
            PageSpans::default()
        }
    }
}

fn with_effective_baseline(mut span: TextSpan) -> TextSpan {
    let rise = span.text_rise * span.font_size;
    let (sin, cos) = span.rotation_degrees.to_radians().sin_cos();
    span.bbox.x -= sin * rise;
    span.bbox.y += cos * rise;
    span.text_rise = 0.0;
    span
}

pub(super) fn capture(table: &Table, cells: &[Vec<String>], spans: &PageSpans) -> Option<TableScripts> {
    let rows: Vec<Vec<_>> = table.rows.iter().map(|row| row.cells.iter().collect()).collect();
    let mut annotations = Vec::new();
    crate::extraction::grid_flatten::resolve_span_grid(
        &rows,
        |cell| cell.colspan,
        |cell| cell.rowspan,
        |row, col, cell| {
            let Some(text) = cells.get(row as usize).and_then(|cells| cells.get(col as usize)) else {
                return;
            };
            for annotation in cell_annotations(cell, text, spans) {
                annotations.push(CellScriptAnnotation {
                    row: row as usize,
                    col: col as usize,
                    annotation,
                });
            }
        },
    );
    (!annotations.is_empty()).then(|| TableScripts {
        cells: cells.to_vec(),
        annotations,
    })
}

fn cell_annotations(cell: &TableCell, text: &str, spans: &PageSpans) -> Vec<TextAnnotation> {
    let Some(bounds) = cell.bbox else { return Vec::new() };
    let start = spans
        .spans
        .partition_point(|span| page_advance_midpoint(span).1 < bounds.y - spans.margin);
    let end = spans
        .spans
        .partition_point(|span| page_advance_midpoint(span).1 <= bounds.y + bounds.height + spans.margin);
    let selected: Vec<_> = spans.spans[start..end]
        .iter()
        .filter(|span| {
            let (x, y) = page_advance_midpoint(span);
            let margin = span.font_size * crate::script_run::SCRIPT_RUN_MAX_BASELINE_FONT_FACTOR;
            x >= bounds.x
                && x <= bounds.x + bounds.width
                && y >= bounds.y - margin
                && y <= bounds.y + bounds.height + margin
        })
        .collect();
    let selected = sort_spans_in_reading_order(selected);
    let mut source = Vec::new();
    for (index, span) in selected.iter().enumerate() {
        let kind = script_kind(&selected, index);
        source.extend(
            span.text
                .chars()
                .filter(|c| !c.is_whitespace())
                .map(|c| (c, kind.clone())),
        );
    }
    project_annotations(text, source)
}

fn project_annotations(text: &str, source: Vec<(char, Option<AnnotationKind>)>) -> Vec<TextAnnotation> {
    // Projection requires identical characters; nearby unrelated text cannot invent cell formatting. ~keep
    let target: Vec<_> = text.char_indices().filter(|(_, c)| !c.is_whitespace()).collect();
    if source.len() != target.len() || source.iter().zip(&target).any(|((a, _), (_, b))| a != b) {
        return Vec::new();
    }
    let mut annotations: Vec<TextAnnotation> = Vec::new();
    for ((_, kind), (offset, character)) in source.into_iter().zip(target) {
        let Some(kind) = kind else { continue };
        let Some((start, end)) = u32::try_from(offset)
            .ok()
            .zip(u32::try_from(offset + character.len_utf8()).ok())
        else {
            return Vec::new();
        };
        if let Some(last) = annotations.last_mut()
            && last.kind == kind
            && last.end == start
        {
            last.end = end;
        } else {
            annotations.push(TextAnnotation { start, end, kind });
        }
    }
    annotations
}

fn script_kind(spans: &[&TextSpan], index: usize) -> Option<AnnotationKind> {
    let script = spans[index];
    spans[..index]
        .iter()
        .rev()
        .take(super::TABLE_SCRIPT_RUN_BASE_LOOKBACK)
        .find_map(|base| {
            if script.font_size >= base.font_size || !is_script_run_of(base, script) {
                return None;
            }
            let (_, base_y) = super::super::span_geometry::upright_origin(base);
            let (_, script_y) = super::super::span_geometry::upright_origin(script);
            Some(if script_y > base_y {
                AnnotationKind::Superscript
            } else {
                AnnotationKind::Subscript
            })
        })
}

#[cfg(test)]
#[path = "script_tests.rs"]
mod tests;
