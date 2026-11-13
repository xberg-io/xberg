use crate::content::{Matrix, Operator, parse_content_stream};
use crate::extractors::TextExtractor;
use crate::geometry::{Point, Rect};
use crate::layout::{TextChar, TextSpan};
use crate::structure::table_extractor::{Table, TableCell, TableRow};

const TOLERANCE: f32 = 0.3;
const MAX_CONTENT: usize = 128 * 1024;
const MAX_OPERATORS: usize = 8192;
const MAX_DIVIDERS: usize = 15;

#[derive(Clone, Copy)]
struct Rule {
    start: Point,
    end: Point,
}

#[derive(Clone)]
struct PaintGlyph {
    glyph: TextChar,
    run: usize,
}

impl std::ops::Deref for PaintGlyph {
    type Target = TextChar;

    fn deref(&self) -> &Self::Target {
        &self.glyph
    }
}

enum Paint {
    Text(Vec<PaintGlyph>),
    Rule(Rule),
    Other,
}

#[cfg(test)]
pub(super) fn recover(
    document: &crate::PdfDocument,
    page: usize,
    config: &crate::structure::spatial_table_detector::TableDetectionConfig,
) -> Option<Table> {
    let events = page_paint(document, page)?;
    let table = body_from_paint(&events)?;
    fits_config(&table, config).then_some(table)
}

fn fits_config(table: &Table, config: &crate::structure::spatial_table_detector::TableDetectionConfig) -> bool {
    config.enabled
        && table.col_count >= config.min_table_columns
        && table.col_count <= config.max_table_columns
        && table.rows.len() * table.col_count >= config.min_table_cells
}

fn page_paint(document: &crate::PdfDocument, page: usize) -> Option<Vec<Paint>> {
    if document.get_page_rotation(page).ok()? != 0 {
        return None;
    }
    let data = document.get_page_content_data(page).ok()?;
    if data.len() > MAX_CONTENT {
        return None;
    }
    let operators = parse_content_stream(&data).ok()?;
    if operators.len() > MAX_OPERATORS {
        return None;
    }
    let object = document.get_page(page).ok()?;
    let resources = object.as_dict()?.get("Resources")?;
    if !supported_graphics_states(document, resources, &operators) {
        return None;
    }
    let mut extractor = TextExtractor::new();
    extractor.set_page_index(page as u32);
    extractor.set_resources(resources.clone());
    extractor.set_document(document);
    document.load_fonts(resources, &mut extractor).ok()?;
    paint_events(operators, &mut extractor)
}

pub(super) fn replace_collapsed(
    document: &crate::PdfDocument,
    page: usize,
    config: &crate::structure::spatial_table_detector::TableDetectionConfig,
    ordinary: &Table,
) -> Option<Table> {
    if ordinary.rows.len() != 2
        || ordinary.col_count < 2
        || ordinary.rows.iter().any(|row| {
            row.cells.len() != ordinary.col_count || row.cells.iter().any(|cell| cell.colspan != 1 || cell.rowspan != 1)
        })
        || ordinary.rows[1].cells.iter().any(|cell| cell.text.lines().count() < 3)
    {
        return None;
    }
    let events = page_paint(document, page)?;
    replacement_from_paint(&events, config, ordinary)
}

fn replacement_from_paint(
    events: &[Paint],
    config: &crate::structure::spatial_table_detector::TableDetectionConfig,
    ordinary: &Table,
) -> Option<Table> {
    let (mut body, start) = locate_body(events)?;
    if !fits_config(&body, config) || body.col_count != ordinary.col_count + 2 {
        return None;
    }
    let (header, top) = recover_header(events, start, &body)?;
    let bounds = body.bbox?;
    if !same_bounds(
        ordinary.bbox?,
        Rect::new(bounds.x, bounds.y, bounds.width, top - bounds.y),
    ) || !header_matches(&ordinary.rows[0], &header)
        || !body_matches(&ordinary.rows[1], &body)
    {
        return None;
    }
    body.rows.insert(0, header);
    body.has_header = true;
    body.bbox = Some(ordinary.bbox?);
    Some(body)
}

fn same_bounds(a: Rect, b: Rect) -> bool {
    [
        (a.left(), b.left()),
        (a.right(), b.right()),
        (a.top(), b.top()),
        (a.bottom(), b.bottom()),
    ]
    .into_iter()
    .all(|(a, b)| (a - b).abs() <= TOLERANCE)
}

fn header_matches(ordinary: &TableRow, recovered: &TableRow) -> bool {
    ordinary
        .cells
        .iter()
        .zip(recovered.cells.iter().skip(1))
        .all(|(old, new)| compact(&old.text) == compact(&new.text))
}

fn body_matches(ordinary: &TableRow, recovered: &Table) -> bool {
    ordinary.cells.iter().enumerate().all(|(column, old)| {
        if old.spans.is_empty() {
            return old
                .text
                .lines()
                .map(compact)
                .eq(recovered.rows.iter().map(|row| compact(&row.cells[column + 1].text)));
        }
        let mut remaining: Vec<_> = old.spans.iter().collect();
        for row in &recovered.rows {
            let cell = &row.cells[column + 1];
            let Some(bounds) = cell.bbox else { return false };
            let mut matching = Vec::new();
            remaining.retain(|span| {
                if (bounds.top() - TOLERANCE..=bounds.bottom() + TOLERANCE).contains(&span.bbox.center().y) {
                    matching.push(*span);
                    false
                } else {
                    true
                }
            });
            matching.sort_by(|a, b| a.bbox.x.total_cmp(&b.bbox.x));
            let original: String = matching
                .iter()
                .flat_map(|span| span.text.chars())
                .filter(|c| !c.is_whitespace())
                .collect();
            if original != compact(&cell.text) {
                return false;
            }
        }
        remaining.is_empty()
    })
}

fn compact(text: &str) -> String {
    text.chars().filter(|c| !c.is_whitespace()).collect()
}

fn recover_header(events: &[Paint], start: usize, body: &Table) -> Option<(TableRow, f32)> {
    let columns = body.col_count - 1;
    let first = start.checked_sub(columns + 3)?;
    let [Paint::Text(chars), Paint::Rule(top), Paint::Rule(bottom), dividers @ ..] = events.get(first..start)? else {
        return None;
    };
    let bounds = body.bbox?;
    if ![top, bottom].iter().all(|rule| {
        (rule.start.y - rule.end.y).abs() <= TOLERANCE
            && (rule.start.x - bounds.left()).abs() <= TOLERANCE
            && (rule.end.x - bounds.right()).abs() <= TOLERANCE
    }) || (bottom.start.y - bounds.bottom()).abs() > TOLERANCE
        || top.start.y <= bottom.start.y
    {
        return None;
    }
    let axes = body_axes(events.get(start + 1..)?)?;
    let (low, high) = divider_band(dividers, &axes)?;
    if (low - bottom.start.y).abs() > TOLERANCE || (high - top.start.y).abs() > TOLERANCE {
        return None;
    }
    let header_chars: Vec<_> = chars
        .iter()
        .filter(|glyph| (bottom.start.y..top.start.y).contains(&glyph.origin_y))
        .cloned()
        .collect();
    if events.iter().enumerate().any(|(index, event)| {
        let body_end = start + body.rows.len() * body.col_count;
        index != first
            && !(start..body_end).contains(&index)
            && matches!(event, Paint::Text(glyphs) if glyphs.iter().any(|glyph| {
                !glyph.char.is_whitespace() && (bounds.left()..=bounds.right()).contains(&glyph.origin_x)
                    && (bounds.top()..top.start.y).contains(&glyph.origin_y)
            }))
    }) {
        return None;
    }
    let mut header = row_from_chars(&header_chars, &axes, bottom.start.y, top.start.y)?;
    header.is_header = true;
    for cell in &mut header.cells {
        cell.is_header = true;
    }
    Some((header, top.start.y))
}

fn supported_graphics_states(
    document: &crate::PdfDocument,
    resources: &crate::object::Object,
    operators: &[Operator],
) -> bool {
    let resolve = |value: &crate::object::Object| {
        if let Some(reference) = value.as_reference() {
            document.load_object(reference).ok()
        } else {
            Some(value.clone())
        }
    };
    let states = resolve(resources).and_then(|resources| resources.as_dict()?.get("ExtGState").and_then(resolve));
    operators.iter().all(|operator| {
        let Operator::SetExtGState { dict_name } = operator else {
            return true;
        };
        states
            .as_ref()
            .and_then(|states| states.as_dict()?.get(dict_name))
            .and_then(resolve)
            .is_some_and(|state| {
                state.as_dict().is_some_and(|dict| {
                    dict.keys().all(|key| {
                        matches!(
                            key.as_str(),
                            "Type" | "BG2" | "OP" | "op" | "OPM" | "SA" | "SM" | "UCR2"
                        )
                    })
                })
            })
    })
}

fn paint_events(operators: Vec<Operator>, extractor: &mut TextExtractor<'_>) -> Option<Vec<Paint>> {
    let mut events = Vec::new();
    let mut matrices = Vec::new();
    let mut ctm = Matrix::identity();
    let mut path = Vec::new();
    let mut in_text = false;
    let mut restarts = Vec::new();
    extractor.take_body_paint_chars();
    for operator in operators {
        match &operator {
            Operator::BeginText if !in_text => in_text = true,
            Operator::EndText if in_text => {
                events.push(Paint::Text(paint_glyphs(extractor.take_body_paint_chars(), &restarts)));
                restarts.clear();
                in_text = false;
            }
            Operator::BeginText
            | Operator::EndText
            | Operator::Do { .. }
            | Operator::InlineImage { .. }
            | Operator::Tr { .. }
            | Operator::ClipNonZero
            | Operator::ClipEvenOdd
            | Operator::BeginMarkedContentDict { .. } => return None,
            Operator::SaveState => matrices.push(ctm),
            Operator::RestoreState => ctm = matrices.pop()?,
            Operator::Cm { a, b, c, d, e, f } => {
                ctm = Matrix {
                    a: *a,
                    b: *b,
                    c: *c,
                    d: *d,
                    e: *e,
                    f: *f,
                }
                .multiply(&ctm);
            }
            Operator::MoveTo { x, y } | Operator::LineTo { x, y } => {
                path.push(ctm.transform_point(*x, *y));
            }
            Operator::Stroke => {
                let paint = match path.as_slice() {
                    [start, end] => Paint::Rule(Rule {
                        start: *start,
                        end: *end,
                    }),
                    _ => Paint::Other,
                };
                events.push(paint);
                path.clear();
            }
            Operator::EndPath => path.clear(),
            Operator::Tf { .. }
            | Operator::Tm { .. }
            | Operator::Td { .. }
            | Operator::TD { .. }
            | Operator::Tj { .. }
            | Operator::TJ { .. }
            | Operator::Tw { .. }
            | Operator::Tc { .. }
            | Operator::TL { .. }
            | Operator::TStar
            | Operator::Quote { .. }
            | Operator::DoubleQuote { .. }
            | Operator::Ts { .. }
            | Operator::Tz { .. }
            | Operator::SetLineWidth { .. }
            | Operator::SetExtGState { .. } => {}
            _ => {
                events.push(Paint::Other);
                path.clear();
            }
        }
        execute_paint_operator(extractor, operator, in_text.then_some(&mut restarts))?;
    }
    (!in_text && matrices.is_empty()).then_some(events)
}

fn execute_paint_operator(
    extractor: &mut TextExtractor<'_>,
    operator: Operator,
    restarts: Option<&mut Vec<usize>>,
) -> Option<()> {
    let Some(restarts) = restarts else {
        return extractor.execute_operator_public(operator).ok();
    };
    if restarts_text_run(&operator) {
        restarts.push(extractor.char_count());
    }
    extractor
        .execute_operator_with_paint_restarts(operator, Some(restarts))
        .ok()
}

fn restarts_text_run(operator: &Operator) -> bool {
    matches!(
        operator,
        Operator::Tm { .. }
            | Operator::Td { .. }
            | Operator::TD { .. }
            | Operator::TStar
            | Operator::Quote { .. }
            | Operator::DoubleQuote { .. }
            | Operator::Ts { .. }
            | Operator::Cm { .. }
            | Operator::RestoreState
    )
}

fn paint_glyphs(chars: Vec<TextChar>, restarts: &[usize]) -> Vec<PaintGlyph> {
    chars
        .into_iter()
        .enumerate()
        .map(|(index, glyph)| PaintGlyph {
            glyph,
            run: restarts.partition_point(|start| *start <= index),
        })
        .collect()
}

#[cfg(test)]
fn body_from_paint(events: &[Paint]) -> Option<Table> {
    locate_body(events).map(|(body, _)| body)
}

fn locate_body(events: &[Paint]) -> Option<(Table, usize)> {
    let mut candidate = None;
    let mut skip = 0;
    let mut attempts = 0;
    for (index, event) in events.iter().enumerate() {
        if index < skip
            || !matches!(event, Paint::Text(_))
            || !matches!(events.get(index + 1), Some(Paint::Rule(rule)) if vertical(rule))
        {
            continue;
        }
        attempts += 1;
        if attempts > 32 {
            return None;
        }
        let Some(table) = body_at(&events[index..]) else {
            continue;
        };
        let Some(bounds) = table.bbox else { continue };
        let preceding = events[..index].iter().rev();
        let mut preceding = preceding.skip_while(|event| matches!(event, Paint::Rule(rule) if vertical(rule)));
        if !matches!(preceding.next(), Some(Paint::Rule(rule))
            if (rule.start.y - bounds.bottom()).abs() <= TOLERANCE
                && (rule.end.y - rule.start.y).abs() <= TOLERANCE
                && (rule.start.x - bounds.left()).abs() <= TOLERANCE
                && (rule.end.x - bounds.right()).abs() <= TOLERANCE)
        {
            continue;
        }
        if candidate.is_some() {
            return None;
        }
        skip = index + table.rows.len() * table.col_count;
        candidate = Some((table, index));
    }
    candidate
}

fn body_axes(events: &[Paint]) -> Option<Vec<f32>> {
    let axes: Vec<_> = events
        .iter()
        .map_while(|event| match event {
            Paint::Rule(rule) if vertical(rule) => Some(rule.start.x),
            _ => None,
        })
        .take(MAX_DIVIDERS + 1)
        .collect();
    if !(2..=MAX_DIVIDERS).contains(&axes.len()) || !axes.windows(2).all(|pair| pair[1] - pair[0] > TOLERANCE) {
        return None;
    }
    Some(axes)
}

fn body_at(events: &[Paint]) -> Option<Table> {
    let axes = body_axes(events.get(1..)?)?;
    let mut rows = Vec::new();
    let mut top = None;
    let mut bottom = None;
    let mut remaining = events;
    while let [Paint::Text(chars), rest @ ..] = remaining {
        let Some((low, high)) = divider_band(rest, &axes) else {
            break;
        };
        if bottom.is_some_and(|previous: f32| (previous - high).abs() > TOLERANCE) {
            return None;
        }
        rows.push(row_from_chars(chars, &axes, low, high)?);
        top.get_or_insert(high);
        bottom = Some(low);
        remaining = &rest[axes.len()..];
    }
    if rows.len() < 3 {
        return None;
    }
    let [Paint::Rule(end), rest @ ..] = remaining else {
        return None;
    };
    let bottom = bottom?;
    if (end.start.y - end.end.y).abs() > TOLERANCE
        || end.start.x >= axes[0]
        || end.end.x <= *axes.last()?
        || end.start.y >= bottom
        || bottom - end.start.y > 5.0
        || !divider_band(rest, &axes)
            .is_some_and(|(low, high)| (low - end.start.y).abs() <= TOLERANCE && (high - bottom).abs() <= TOLERANCE)
    {
        return None;
    }
    let bounds = Rect::new(end.start.x, end.start.y, end.end.x - end.start.x, top? - end.start.y);
    if rows.iter().flat_map(|row| &row.cells).any(|cell| {
        cell.bbox
            .is_none_or(|bbox| bbox.left() < bounds.left() - TOLERANCE || bbox.right() > bounds.right() + 3.0)
    }) {
        return None;
    }
    Some(Table {
        rows,
        has_header: false,
        col_count: axes.len() + 1,
        bbox: Some(bounds),
    })
}

fn vertical(rule: &Rule) -> bool {
    (rule.start.x - rule.end.x).abs() <= TOLERANCE && rule.end.y - rule.start.y > TOLERANCE
}

fn divider_band(events: &[Paint], axes: &[f32]) -> Option<(f32, f32)> {
    let [Paint::Rule(first), ..] = events else { return None };
    let band = (first.start.y, first.end.y);
    events
        .get(..axes.len())?
        .iter()
        .zip(axes)
        .all(|(event, x)| {
            matches!(event, Paint::Rule(rule) if vertical(rule)
            && (rule.start.x - x).abs() <= TOLERANCE
            && (rule.start.y - band.0).abs() <= TOLERANCE
            && (rule.end.y - band.1).abs() <= TOLERANCE)
        })
        .then_some(band)
}

fn disconnected_cell_glyphs(glyphs: &[&PaintGlyph]) -> bool {
    glyphs.iter().enumerate().any(|(index, glyph)| {
        let Some((prior_index, prior)) = glyphs[..index]
            .iter()
            .enumerate()
            .rev()
            .find(|(_, prior)| (prior.origin_y - glyph.origin_y).abs() <= 0.5)
        else {
            return index > 0 && !attaches_across_baselines(glyphs, index);
        };
        if (glyph.origin_y - glyphs[0].origin_y).abs() > 0.5 {
            let pen_end = prior.origin_x + prior.rendered_advance;
            let half_em = 0.5 * glyph.font_size.max(prior.font_size);
            let continuous = glyph.run == prior.run && (glyph.origin_x - pen_end).abs() <= half_em;
            return !continuous && !attaches_across_baselines(glyphs, index);
        }
        let allowance = glyph.font_size.max(prior.font_size) * 1.5;
        let mut end = prior.bbox.right();
        // Interleaved glyphs can bridge a gap only if they extend the run continuously. ~keep
        for bridge in &glyphs[prior_index + 1..index] {
            if bridge.origin_x >= prior.origin_x && bridge.origin_x <= end + allowance {
                end = end.max(bridge.bbox.right());
            }
        }
        glyph.origin_x - end > allowance || glyph.origin_x - prior.bbox.right() < -glyph.font_size
    })
}

fn attaches_across_baselines(glyphs: &[&PaintGlyph], index: usize) -> bool {
    let glyph = glyphs[index];
    let base = glyphs[0];
    // A shifted run must reach the cell's original baseline. An accepted
    // raised marker or wrapped line cannot serve as a new anchor. ~keep
    glyphs[..index].iter().rev().any(|prior| {
        if (prior.origin_y - base.origin_y).abs() > 0.5 {
            return false;
        }
        let em = glyph.font_size.max(prior.font_size);
        let gap = (glyph.origin_x - prior.bbox.right()).max(prior.origin_x - glyph.bbox.right());
        let rise = prior.origin_y - glyph.origin_y;
        let vertical_reach = if rise > 0.0 && glyph.font_size == prior.font_size {
            2.0
        } else {
            1.5
        };
        rise.abs() <= vertical_reach * em && gap <= 1.5 * em
    })
}

fn row_from_chars(chars: &[PaintGlyph], axes: &[f32], low: f32, high: f32) -> Option<TableRow> {
    let mut columns = vec![Vec::new(); axes.len() + 1];
    for glyph in chars {
        if glyph.char.is_whitespace() {
            continue;
        }
        if glyph.rotation_degrees != 0.0
            || !glyph.origin_x.is_finite()
            || !(low - TOLERANCE..high).contains(&glyph.origin_y)
        {
            return None;
        }
        columns[axes.partition_point(|x| *x <= glyph.origin_x)].push(glyph);
    }
    let mut row = TableRow::new(false);
    for glyphs in columns {
        let first = *glyphs.first()?;
        if disconnected_cell_glyphs(&glyphs) {
            return None;
        }
        let mut bbox = first.bbox;
        let mut text = String::new();
        let mut previous: Option<&TextChar> = None;
        for glyph in glyphs {
            if previous.is_some_and(|prior| {
                (prior.origin_y - glyph.origin_y).abs() > 0.5
                    || glyph.origin_x - prior.bbox.right() > glyph.font_size * 0.15
            }) {
                text.push(' ');
            }
            text.push(glyph.char);
            bbox = bbox.union(&glyph.bbox);
            previous = Some(glyph);
        }
        let mut cell = TableCell::new(text.clone(), false);
        cell.bbox = Some(bbox);
        cell.spans.push(TextSpan {
            text,
            bbox,
            font_name: first.font_name.clone(),
            font_size: first.font_size,
            ..Default::default()
        });
        row.cells.push(cell);
    }
    Some(row)
}

#[cfg(test)]
fn associate_body_rows(ruled: &Table, body: &Table) -> Option<Table> {
    // Keep the experiment out of selection until the real ruled grid supplies
    // the same body rows and columns. ~keep
    let header_count = ruled.rows.iter().take_while(|row| row.is_header).count();
    let existing = &ruled.rows[header_count..];
    if ruled.col_count != body.col_count || existing.len() != body.rows.len() {
        return None;
    }
    let mut result = ruled.clone();
    for (target, painted) in result.rows[header_count..].iter_mut().zip(&body.rows) {
        if target.is_header || target.cells.len() != painted.cells.len() {
            return None;
        }
        for (cell, source) in target.cells.iter_mut().zip(&painted.cells) {
            if cell.is_header || cell.colspan != 1 || cell.rowspan != 1 || !cell.bbox?.intersects(&source.bbox?) {
                return None;
            }
            cell.text.clone_from(&source.text);
            cell.spans.clone_from(&source.spans);
        }
    }
    Some(result)
}

#[cfg(test)]
#[path = "body_paint_tests.rs"]
mod tests;
