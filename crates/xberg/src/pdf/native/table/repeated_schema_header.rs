//! Header evidence for repeated unruled numeric schemas.

use crate::types::{BoundingBox, Table};
use xberg_native_pdf::document::{PageRotation, PdfDocument};
use xberg_native_pdf::layout::{Color, TextSpan};

const MIN_SCHEMA_COPIES: usize = 3;
const MIN_SCHEMA_WIDTH: usize = 3;
const BODY_WITNESS_ROWS: usize = 3;
const BASELINE_TOLERANCE: f32 = 0.5;
const MAX_SOURCE_SPANS: usize = 4096;
const MAX_SCHEMA_COLUMNS: usize = 64;

// ~keep Run after layout arbitration so the source proof changes only header roles.
pub(crate) fn retain(doc: &PdfDocument, native_tables: &mut [Table], layout_tables: &mut [Table]) {
    let mut candidates: Vec<_> = native_tables
        .iter_mut()
        .filter_map(|table| repeated_numeric_schema(table).map(|width| (table, width)))
        .collect();
    candidates.sort_by_key(|(table, _)| table.page_number);
    let mut page_spans = None;
    for (table, width) in candidates {
        let Some(page) = table.page_number.checked_sub(1).map(|page| page as usize) else {
            continue;
        };
        if page_spans.as_ref().is_none_or(|(last, _)| *last != page) {
            page_spans = Some((page, painting_spans(doc, page)));
        }
        if page_spans
            .as_ref()
            .and_then(|(_, spans)| spans.as_deref())
            .is_none_or(|spans| !has_painted_header(table, width, spans))
        {
            continue;
        }
        table.columns = table.cells.first().cloned();
        for layout_table in layout_tables.iter_mut() {
            if same_records(table, layout_table) {
                layout_table.columns.clone_from(&table.columns);
            }
        }
    }
}

fn painting_spans(doc: &PdfDocument, page: usize) -> Option<Vec<TextSpan>> {
    if !matches!(
        doc.get_page_rotation_status(page).ok()?,
        PageRotation::Absent | PageRotation::Valid(0)
    ) {
        return None;
    }
    let (_, spans) = doc.extract_painting_text_and_spans(page).ok()?;
    (spans.len() <= MAX_SOURCE_SPANS).then_some(spans)
}

fn same_records(source: &Table, layout: &Table) -> bool {
    source.page_number == layout.page_number
        && layout.columns.is_none()
        && source.cells.first() == layout.cells.first()
        && layout
            .cells
            .get(1..)
            .is_some_and(|body| body.ends_with(&source.cells[1..]))
        && matches!((source.bounding_box, layout.bounding_box), (Some(left), Some(right))
            if super::finite_table_bounds(right)
                && (left.x0 - right.x0).abs() <= 1.0
                && (left.x1 - right.x1).abs() <= 1.0
                && (left.y0 - right.y0).abs() <= 1.0)
}

fn repeated_numeric_schema(table: &Table) -> Option<usize> {
    let labels = table.cells.first()?;
    if table.columns.is_some()
        || labels.len() > MAX_SCHEMA_COLUMNS
        || table.cells.len() <= BODY_WITNESS_ROWS
        || !labels
            .iter()
            .all(|label| label.chars().any(char::is_alphabetic) && !label.chars().any(char::is_numeric))
    {
        return None;
    }
    let width = (MIN_SCHEMA_WIDTH..=labels.len() / MIN_SCHEMA_COPIES).find(|&width| {
        labels.len().is_multiple_of(width)
            && labels[..width]
                .iter()
                .all(|label| labels[..width].iter().filter(|&other| other == label).count() == 1)
            && labels.chunks_exact(width).all(|group| group == &labels[..width])
    })?;
    table.cells[1..=BODY_WITNESS_ROWS]
        .iter()
        .all(|row| row.len() == labels.len() && row.iter().all(|value| is_numeric(value)))
        .then_some(width)
}

fn is_numeric(value: &str) -> bool {
    value.chars().any(|c| c.is_ascii_digit())
        && value
            .chars()
            .all(|c| c.is_ascii_digit() || matches!(c, '.' | ',' | '-' | '+') || c.is_whitespace())
}

fn has_painted_header(table: &Table, width: usize, spans: &[TextSpan]) -> bool {
    let Some(bounds) = table.bounding_box.filter(|bounds| super::finite_table_bounds(*bounds)) else {
        return false;
    };
    let local: Vec<_> = spans.iter().filter(|span| inside_table(span, bounds)).collect();
    let mut bands = Vec::with_capacity(BODY_WITNESS_ROWS);
    for row in &table.cells[1..=BODY_WITNESS_ROWS] {
        let Some(band) = painted_record(&local, row, width) else {
            return false;
        };
        bands.push(band);
    }
    let [first, second, third] = bands.as_slice() else {
        return false;
    };
    let pitch = first.y - second.y;
    if pitch <= 0.0
        || (second.y - third.y - pitch).abs() > BASELINE_TOLERANCE
        || first.color != second.color
        || first.color != third.color
        || ![second, third].into_iter().all(|band| {
            band.keys
                .iter()
                .zip(&first.keys)
                .all(|(a, b)| (a - b).abs() <= BASELINE_TOLERANCE)
        })
    {
        return false;
    }
    painted_label_bands(&local, first, pitch, &table.cells[0]) && painted_cells_match(table, &local)
}

// ~keep Reconstruct from the same painting spans that supplied the two label bands.
fn painted_cells_match(table: &Table, local: &[&TextSpan]) -> bool {
    use crate::pdf::native::hierarchy::project_flat_painted_span;
    use crate::pdf::table_reconstruct::project_segment_words;

    let segments: Vec<_> = local.iter().map(|&span| project_flat_painted_span(span)).collect();
    let page_height = segments.iter().map(|span| span.y + span.height).fold(792.0, f32::max);
    let words = project_segment_words(&segments, page_height, None);
    super::reconstruct_region_table(&words, page_height, table.page_number, false, 0)
        .is_some_and(|painted| painted.cells == table.cells)
}

struct RecordBand {
    y: f32,
    color: Color,
    keys: Vec<f32>,
}

fn painted_record(spans: &[&TextSpan], row: &[String], width: usize) -> Option<RecordBand> {
    let keys: Option<Vec<_>> = row
        .chunks_exact(width)
        .map(|group| {
            let mut matches = spans
                .iter()
                .copied()
                .filter(|span| span.text.trim() == group[0] && span.font_weight.is_bold());
            let key = matches.next()?;
            matches.next().is_none().then_some(key)
        })
        .collect();
    let keys = keys?;
    let first = keys.first()?;
    if keys
        .iter()
        .any(|key| key.color != first.color || (key.bbox.y - first.bbox.y).abs() > BASELINE_TOLERANCE)
        || keys.windows(2).any(|pair| pair[0].bbox.x >= pair[1].bbox.x)
    {
        return None;
    }
    for (index, (key, group)) in keys.iter().zip(row.chunks_exact(width)).enumerate() {
        let next_key = keys.get(index + 1).map_or(f32::INFINITY, |span| span.bbox.x);
        let values = spans.iter().copied().filter(|span| {
            span.bbox.x >= key.bbox.right()
                && span.bbox.x < next_key
                && (span.bbox.y - first.bbox.y).abs() <= BASELINE_TOLERANCE
        });
        let mut values: Vec<_> = values.collect();
        values.sort_by(|a, b| a.bbox.x.total_cmp(&b.bbox.x));
        if values.len() != width - 1
            || !values.iter().zip(&group[1..]).all(|(span, value)| {
                span.text.trim() == value && span.color != first.color && !span.font_weight.is_bold()
            })
        {
            return None;
        }
    }
    Some(RecordBand {
        y: first.bbox.y,
        color: first.color,
        keys: keys.iter().map(|span| span.bbox.x).collect(),
    })
}

fn inside_table(span: &TextSpan, bounds: BoundingBox) -> bool {
    span.rotation_degrees == 0.0
        && span.wmode == 0
        && !span.text.trim().is_empty()
        && f64::from(span.bbox.x + span.bbox.width * 0.5) >= bounds.x0
        && f64::from(span.bbox.x + span.bbox.width * 0.5) <= bounds.x1
        && f64::from(span.bbox.y + BASELINE_TOLERANCE) >= bounds.y0
        && f64::from(span.bbox.y) <= bounds.y1
}

fn painted_label_bands(spans: &[&TextSpan], first: &RecordBand, pitch: f32, cells: &[String]) -> bool {
    let mut labels: Vec<_> = spans
        .iter()
        .copied()
        .filter(|span| span.bbox.y > first.y + BASELINE_TOLERANCE)
        .collect();
    if labels.iter().any(|label| {
        !label.font_weight.is_bold() || label.color == first.color || !label.text.chars().any(char::is_alphabetic)
    }) {
        return false;
    }
    let mut painted_words: Vec<_> = labels.iter().flat_map(|label| label.text.split_whitespace()).collect();
    let mut cell_words: Vec<_> = cells.iter().flat_map(|cell| cell.split_whitespace()).collect();
    painted_words.sort_unstable();
    cell_words.sort_unstable();
    if painted_words != cell_words {
        return false;
    }
    labels.sort_by(|a, b| b.bbox.y.total_cmp(&a.bbox.y));
    let Some(upper) = labels.first().map(|span| span.bbox.y) else {
        return false;
    };
    let split = labels.partition_point(|span| (span.bbox.y - upper).abs() <= BASELINE_TOLERANCE);
    let (upper_labels, lower_labels) = labels.split_at(split);
    let Some(lower) = lower_labels.first().map(|span| span.bbox.y) else {
        return false;
    };
    (upper - lower - pitch).abs() <= BASELINE_TOLERANCE
        && lower - first.y > pitch + BASELINE_TOLERANCE
        && lower_labels
            .iter()
            .all(|span| (span.bbox.y - lower).abs() <= BASELINE_TOLERANCE)
        && repeated_label_band(upper_labels, &first.keys)
        && repeated_label_band(lower_labels, &first.keys)
}

fn repeated_label_band(labels: &[&TextSpan], keys: &[f32]) -> bool {
    let mut groups = vec![Vec::new(); keys.len()];
    for span in labels {
        let index = keys.partition_point(|key| *key - BASELINE_TOLERANCE <= span.bbox.x);
        let Some(group) = index.checked_sub(1).and_then(|index| groups.get_mut(index)) else {
            return false;
        };
        group.push(*span);
    }
    for (group, key) in groups.iter_mut().zip(keys) {
        group.sort_by(|a, b| a.bbox.x.total_cmp(&b.bbox.x));
        if group
            .first()
            .is_none_or(|span| (span.bbox.x - key).abs() > BASELINE_TOLERANCE)
        {
            return false;
        }
    }
    groups[1..].iter().all(|group| {
        group
            .iter()
            .map(|span| span.text.trim())
            .eq(groups[0].iter().map(|span| span.text.trim()))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires the pinned ParseBench insurance PDF"]
    fn painted_header_rejects_labels_reassigned_to_neighbor_cells() {
        use super::super::{TablePagePaths, extract_tables_heuristic};
        use crate::pdf::native::NativeDocument;
        use sha2::{Digest, Sha256};

        let input = std::env::var_os("XBERG_2332_PARSEBENCH_INPUTS").expect("set the pinned PDF directory");
        let bytes = std::fs::read(std::path::Path::new(&input).join("f48db8574acac2e2744d.pdf")).unwrap();
        assert_eq!(
            hex::encode(Sha256::digest(&bytes)),
            "0085e250b078694c7a6887245e52108e511401b90db99052fe1ee2b4c0d099a3"
        );
        let mut doc = NativeDocument::open_bytes(&bytes).unwrap();
        let mut tables = extract_tables_heuristic(
            &mut doc,
            false,
            &Default::default(),
            Default::default(),
            &mut TablePagePaths::default(),
        )
        .unwrap()
        .tables;
        assert_eq!(tables.len(), 1);
        let mut table = tables.pop().unwrap();
        let spans = painting_spans(&doc.doc, 0).unwrap();
        let width = repeated_numeric_schema(&table).unwrap();
        assert!(has_painted_header(&table, width, &spans));

        let mut mismatched = table.clone();
        for labels in mismatched.cells[0].chunks_exact_mut(width) {
            labels[1] = "PRICE".into();
        }
        assert_eq!(repeated_numeric_schema(&mismatched), Some(width));
        assert!(!has_painted_header(&mismatched, width, &spans));

        for labels in table.cells[0].chunks_exact_mut(width) {
            assert_eq!(&labels[2..], ["MIN. PREM.", "HAZ L/C CODE"]);
            labels[2] = "MIN. PREM. L/C".into();
            labels[3] = "HAZ CODE".into();
        }
        assert_eq!(repeated_numeric_schema(&table), Some(width));
        assert!(!has_painted_header(&table, width, &spans));
    }
}
