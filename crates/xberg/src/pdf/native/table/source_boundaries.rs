use super::{RuleWork, VerticalRule, rules_in_gap, separates_pair};
use crate::pdf::table_reconstruct::HocrWord;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn preserves_source_boundaries(
    original: &[Vec<String>],
    corrected: &[Vec<String>],
    words: &[HocrWord],
    rules: &[VerticalRule],
    page_height: f32,
    work: &mut RuleWork,
) -> Option<bool> {
    work.charge(words.len())?;
    let rows = source_rows(words);
    if rows.len() != original.len() {
        return Some(false);
    }
    let recurring = recurring_boundaries(&rows, rules, page_height, work)?;
    for ((old, new), row) in original.iter().zip(corrected).zip(&rows) {
        work.charge(old.len().saturating_add(new.len()).saturating_add(row.len()))?;
        let matches = row
            .iter()
            .flat_map(|word| word.text.split_whitespace())
            .eq(old.iter().flat_map(|cell| cell.split_whitespace()));
        if !matches || !row_splits_are_ruled((old, new), row, rules, page_height, &recurring, work)? {
            return Some(false);
        }
    }
    all_columns_are_ruled(corrected, &rows, rules, page_height, &recurring, work)
}

fn source_rows(words: &[HocrWord]) -> Vec<Vec<&HocrWord>> {
    let mut rows = BTreeMap::<u32, Vec<&HocrWord>>::new();
    for word in words {
        rows.entry(word.top).or_default().push(word);
    }
    rows.into_values()
        .map(|mut row| {
            row.sort_by_key(|word| word.left);
            row
        })
        .collect()
}

fn recurring_boundaries(
    rows: &[Vec<&HocrWord>],
    rules: &[VerticalRule],
    page_height: f32,
    work: &mut RuleWork,
) -> Option<BTreeSet<u32>> {
    let mut supported = BTreeMap::new();
    for row in rows {
        for pair in row.windows(2) {
            work.charge(1)?;
            for rule in rules_in_gap(pair[0], pair[1], rules) {
                work.charge(1)?;
                if separates_pair(rule, pair[0], pair[1], page_height) {
                    let (first_row, repeated) = supported.entry(rule.x.to_bits()).or_insert((pair[0].top, false));
                    *repeated |= *first_row != pair[0].top;
                }
            }
        }
    }
    work.charge(supported.len())?;
    Some(
        supported
            .into_iter()
            .filter_map(|(boundary, (_, repeated))| repeated.then_some(boundary))
            .collect(),
    )
}

fn cell_boundaries(cells: &[String]) -> Vec<usize> {
    let mut count = 0;
    cells
        .iter()
        .take(cells.len().saturating_sub(1))
        .map(|cell| {
            count += cell.split_whitespace().count();
            count
        })
        .collect()
}

type SourceBoundaries<'a> = BTreeMap<usize, (&'a HocrWord, &'a HocrWord)>;

fn source_word_boundaries<'a>(row: &[&'a HocrWord]) -> SourceBoundaries<'a> {
    let mut boundaries = BTreeMap::new();
    let mut count = 0;
    for pair in row.windows(2) {
        count += pair[0].text.split_whitespace().count();
        boundaries.insert(count, (pair[0], pair[1]));
    }
    boundaries
}

fn all_columns_are_ruled(
    corrected: &[Vec<String>],
    rows: &[Vec<&HocrWord>],
    rules: &[VerticalRule],
    page_height: f32,
    recurring: &BTreeSet<u32>,
    work: &mut RuleWork,
) -> Option<bool> {
    let columns = corrected.first().map_or(0, |row| row.len().saturating_sub(1));
    work.charge(columns)?;
    let mut supported = vec![BTreeSet::new(); columns];
    for (row, cells) in rows.iter().zip(corrected) {
        work.charge(row.len().saturating_add(cells.len()))?;
        if cells.len() != columns + 1 {
            return Some(false);
        }
        let source = source_word_boundaries(row);
        for (column, boundary) in cell_boundaries(cells).into_iter().enumerate() {
            let Some(&(left, right)) = source.get(&boundary) else {
                continue;
            };
            for rule in rules_in_gap(left, right, rules) {
                work.charge(1)?;
                if recurring.contains(&rule.x.to_bits()) && separates_pair(rule, left, right, page_height) {
                    supported[column].insert(rule.x.to_bits());
                }
            }
        }
    }
    // ~keep A scalar clustering limit can affect every column, so each resulting boundary needs its own ordered rule.
    let mut previous = None;
    Some(supported.iter().all(|candidates| {
        let next = candidates
            .iter()
            .copied()
            .find(|candidate| previous.is_none_or(|prior| prior < *candidate));
        previous = next;
        next.is_some()
    }))
}

fn row_splits_are_ruled(
    cells: (&[String], &[String]),
    row: &[&HocrWord],
    rules: &[VerticalRule],
    page_height: f32,
    recurring: &BTreeSet<u32>,
    work: &mut RuleWork,
) -> Option<bool> {
    let (original, corrected) = cells;
    let original_boundaries = cell_boundaries(original);
    let source_boundaries = source_word_boundaries(row);
    let total = row
        .iter()
        .map(|word| word.text.split_whitespace().count())
        .sum::<usize>();
    for boundary in cell_boundaries(corrected) {
        work.charge(1)?;
        if boundary == 0 || boundary == total || original_boundaries.contains(&boundary) {
            continue;
        }
        let Some(&(left, right)) = source_boundaries.get(&boundary) else {
            return Some(false);
        };
        let mut supported = false;
        for rule in rules_in_gap(left, right, rules) {
            work.charge(1)?;
            if recurring.contains(&rule.x.to_bits()) && separates_pair(rule, left, right, page_height) {
                supported = true;
                break;
            }
        }
        if !supported {
            return Some(false);
        }
    }
    Some(true)
}
