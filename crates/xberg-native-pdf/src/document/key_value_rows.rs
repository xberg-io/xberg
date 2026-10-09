//! Reading-order repair for repeated left-to-right key/value form rows. ~keep

struct KeyValueBlock {
    labels: Vec<usize>,
    values: Vec<Vec<usize>>,
}

fn cluster_indices_by_x(spans: &[crate::layout::TextSpan], mut indices: Vec<usize>, tolerance: f32) -> Vec<Vec<usize>> {
    indices.sort_by(|&a, &b| crate::utils::safe_float_cmp(spans[a].bbox.x, spans[b].bbox.x));
    let mut clusters: Vec<Vec<usize>> = Vec::new();
    for index in indices {
        let belongs = clusters.last().is_some_and(|cluster| {
            let anchor = spans[cluster[0]].bbox.x;
            (spans[index].bbox.x - anchor).abs() <= tolerance
        });
        if belongs {
            if let Some(cluster) = clusters.last_mut() {
                cluster.push(index);
            }
        } else {
            clusters.push(vec![index]);
        }
    }
    clusters
}

fn is_horizontal_ltr(span: &crate::layout::TextSpan) -> bool {
    span.rotation_degrees == 0.0 && span.wmode == 0 && !span.mirrored && !crate::text::bidi::looks_rtl(&span.text)
}

fn assign_key_value_rows(spans: &[crate::layout::TextSpan], labels: &[usize], values: &[usize]) -> Vec<Vec<usize>> {
    let mut rows = vec![Vec::new(); labels.len()];
    for &value in values {
        let Some((row, distance)) = labels
            .iter()
            .enumerate()
            .map(|(row, &label)| (row, (spans[value].bbox.y - spans[label].bbox.y).abs()))
            .min_by(|a, b| crate::utils::safe_float_cmp(a.1, b.1))
        else {
            continue;
        };
        let adjacent_gap = if row == 0 {
            spans[labels[0]].bbox.y - spans[labels[1]].bbox.y
        } else if row + 1 == labels.len() {
            spans[labels[row - 1]].bbox.y - spans[labels[row]].bbox.y
        } else {
            (spans[labels[row - 1]].bbox.y - spans[labels[row]].bbox.y)
                .max(spans[labels[row]].bbox.y - spans[labels[row + 1]].bbox.y)
        };
        if distance <= (adjacent_gap / 2.0).max(spans[labels[row]].font_size) {
            rows[row].push(value);
        }
    }
    for row in &mut rows {
        row.sort_by(|&a, &b| crate::utils::safe_float_cmp(spans[b].bbox.y, spans[a].bbox.y));
    }
    rows
}

fn populated_runs(rows: &[Vec<usize>], minimum: usize) -> Vec<Vec<usize>> {
    let runs = rows.iter().enumerate().filter(|(_, row)| !row.is_empty()).fold(
        Vec::<Vec<usize>>::new(),
        |mut runs, (row, _)| {
            let extends_run = runs
                .last()
                .and_then(|run| run.last())
                .is_some_and(|last| *last + 1 == row);
            if extends_run {
                if let Some(run) = runs.last_mut() {
                    run.push(row);
                }
            } else {
                runs.push(vec![row]);
            }
            runs
        },
    );
    runs.into_iter().filter(|run| run.len() >= minimum).collect()
}

fn build_key_value_blocks(
    spans: &[crate::layout::TextSpan],
    labels: &[usize],
    values: &[usize],
    minimum: usize,
) -> Vec<KeyValueBlock> {
    let rows = assign_key_value_rows(spans, labels, values);
    populated_runs(&rows, minimum)
        .into_iter()
        .filter_map(|run| {
            let selected_labels: Vec<usize> = run.iter().map(|&row| labels[row]).collect();
            let selected_rows: Vec<Vec<usize>> = run.iter().map(|&row| rows[row].clone()).collect();
            let centered = selected_labels.iter().zip(&selected_rows).any(|(&label, row)| {
                let epsilon = spans[label].font_size * 0.25;
                row.iter()
                    .any(|&value| spans[value].bbox.y > spans[label].bbox.y + epsilon)
                    && row
                        .iter()
                        .any(|&value| spans[value].bbox.y < spans[label].bbox.y - epsilon)
            });
            centered.then_some(KeyValueBlock {
                labels: selected_labels,
                values: selected_rows,
            })
        })
        .collect()
}

fn value_tracks(
    spans: &[crate::layout::TextSpan],
    labels: &[usize],
    tolerance: f32,
    minimum: usize,
) -> Vec<Vec<usize>> {
    let label_x = spans[labels[labels.len() / 2]].bbox.x;
    let font_size = spans[labels[0]].font_size.max(1.0);
    let min_value_x = label_x + font_size * 4.0;
    let max_value_x = label_x + font_size * 20.0;
    let top_gap = spans[labels[0]].bbox.y - spans[labels[1]].bbox.y;
    let bottom_gap = spans[labels[labels.len() - 2]].bbox.y - spans[labels[labels.len() - 1]].bbox.y;
    let top_y = spans[labels[0]].bbox.y + (top_gap / 2.0).max(font_size);
    let bottom_y = spans[labels[labels.len() - 1]].bbox.y - (bottom_gap / 2.0).max(font_size);
    let candidates = spans
        .iter()
        .enumerate()
        .filter(|(index, span)| {
            !labels.contains(index)
                && is_horizontal_ltr(span)
                && !span.text.trim_end().ends_with(':')
                && (min_value_x..=max_value_x).contains(&span.bbox.x)
                && (bottom_y..=top_y).contains(&span.bbox.y)
        })
        .map(|(index, _)| index)
        .collect();
    let mut tracks = cluster_indices_by_x(spans, candidates, tolerance);
    tracks.retain(|track| track.len() >= minimum);
    tracks.sort_by(|a, b| crate::utils::safe_float_cmp(spans[a[0]].bbox.x, spans[b[0]].bbox.x));
    tracks
}

/// Find only repeated LTR form rows whose wrapped values bracket a centred label.
/// The centred-line requirement keeps ordinary aligned tables and columns byte-identical. ~keep
fn find_key_value_blocks(spans: &[crate::layout::TextSpan]) -> Vec<KeyValueBlock> {
    const TRACK_TOLERANCE: f32 = 8.0;
    const MIN_LABELS: usize = 4;

    let label_indices = spans
        .iter()
        .enumerate()
        .filter(|(_, span)| is_horizontal_ltr(span) && span.text.trim_end().ends_with(':'))
        .map(|(index, _)| index)
        .collect();
    let mut label_clusters = cluster_indices_by_x(spans, label_indices, TRACK_TOLERANCE);
    label_clusters.sort_by_key(|cluster| std::cmp::Reverse(cluster.len()));
    let mut blocks = Vec::new();
    let mut used = std::collections::HashSet::new();

    for mut labels in label_clusters.into_iter().filter(|cluster| cluster.len() >= MIN_LABELS) {
        labels.sort_by(|&a, &b| crate::utils::safe_float_cmp(spans[b].bbox.y, spans[a].bbox.y));
        for values in value_tracks(spans, &labels, TRACK_TOLERANCE, MIN_LABELS) {
            let candidates = build_key_value_blocks(spans, &labels, &values, MIN_LABELS);
            let mut accepted = false;
            for block in candidates {
                let members = block.labels.iter().chain(block.values.iter().flatten());
                if members.clone().any(|index| used.contains(index)) {
                    continue;
                }
                used.extend(members.copied());
                blocks.push(block);
                accepted = true;
            }
            if accepted {
                break;
            }
        }
    }
    blocks
}

/// Repair reading order in a repeated left-to-right key/value form band.
///
/// This is an explicit opt-in for callers that want vertically centred labels
/// emitted before every line of their wrapped values. Other reading-order
/// modes remain unchanged unless the caller applies this helper.
pub fn reorder_ltr_key_value_rows(spans: &mut [crate::layout::TextSpan]) {
    for block in find_key_value_blocks(spans) {
        let mut slots: Vec<usize> = block
            .labels
            .iter()
            .copied()
            .chain(block.values.iter().flatten().copied())
            .collect();
        slots.sort_unstable();
        let ordered: Vec<crate::layout::TextSpan> = block
            .labels
            .iter()
            .zip(block.values)
            .flat_map(|(&label, values)| std::iter::once(label).chain(values))
            .map(|index| spans[index].clone())
            .collect();
        for (slot, span) in slots.into_iter().zip(ordered) {
            spans[slot] = span;
        }
    }
}
