use crate::pdf::table_reconstruct::HocrWord;
use crate::types::Table;
use xberg_native_pdf::elements::PathContent;

pub(super) fn consolidate(table: &mut Table, words: &[HocrWord], paths: &[PathContent], page_height: f32) {
    let Some(bounds) = table.bounding_box.as_ref() else {
        return;
    };
    let height = super::median_region_word_height(words);
    let mut rows = super::numeric_rows(words, (height / 2).max(3));
    for row in &mut rows {
        row.sort_by_key(|word| word.left);
    }
    let rules: Vec<_> = paths
        .iter()
        .filter(|path| path.has_stroke() && path.is_horizontal_line(0.5))
        .map(PathContent::rendered_bbox)
        .filter(|rule| f64::from(rule.x) <= bounds.x0 + 1.0 && f64::from(rule.right()) >= bounds.x1 - 1.0)
        .map(|rule| page_height - rule.center().y)
        .collect();
    let mut matches = table.cells.windows(2).enumerate().filter(|(_, pair)| {
        let [first, second] = pair else { return false };
        is_wrapped_label(first, second)
            && rows
                .windows(2)
                .filter(|band| {
                    let [upper, lower] = band else { return false };
                    row_matches(upper, first)
                        && row_matches(lower, second)
                        && lower[0]
                            .left
                            .checked_sub(upper[0].left)
                            .is_some_and(|indent| indent >= height / 2 && indent <= height.saturating_mul(2))
                        && has_enclosing_rules(upper, lower, &rules, height)
                })
                .count()
                == 1
    });
    let Some((index, _)) = matches.next() else { return };
    if matches.next().is_some() {
        return;
    }
    let first = table.cells.remove(index);
    table.cells[index][0] = format!("{} {}", first[0], table.cells[index][0]);
    table.markdown = crate::pdf::table_reconstruct::table_to_markdown(&table.cells);
}

fn is_wrapped_label(first: &[String], second: &[String]) -> bool {
    first
        .first()
        .is_some_and(|label| label.split_whitespace().count() >= 3 && !label.ends_with(':'))
        && first.iter().skip(1).all(|cell| cell.trim().is_empty())
        && second
            .first()
            .and_then(|label| label.chars().next())
            .is_some_and(char::is_lowercase)
        && second
            .iter()
            .skip(1)
            .filter(|cell| crate::table_core::text_is_value(cell))
            .count()
            >= 2
}

fn row_matches(words: &[&HocrWord], cells: &[String]) -> bool {
    words
        .iter()
        .flat_map(|word| word.text.split_whitespace())
        .eq(cells.iter().flat_map(|cell| cell.split_whitespace()))
}

fn has_enclosing_rules(upper: &[&HocrWord], lower: &[&HocrWord], rules: &[f32], height: u32) -> bool {
    let upper_top = upper.iter().map(|word| word.top).min().unwrap_or(0) as f32;
    let lower_bottom = lower
        .iter()
        .map(|word| word.top.saturating_add(word.height))
        .max()
        .unwrap_or(0) as f32;
    let tolerance = height as f32;
    rules.iter().any(|&y| (upper_top - tolerance..=upper_top).contains(&y))
        && rules
            .iter()
            .any(|&y| (lower_bottom..=lower_bottom + tolerance).contains(&y))
        && !rules.iter().any(|&y| y > upper_top && y < lower_bottom)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrapped_label_requires_enclosing_rules_and_rejects_a_separator() {
        let word = |top| HocrWord {
            top,
            height: 8,
            text: "label".into(),
            left: 0,
            width: 10,
            confidence: 100.0,
        };
        assert!(has_enclosing_rules(&[&word(12)], &[&word(24)], &[10.0, 34.0], 8));
        assert!(!has_enclosing_rules(&[&word(12)], &[&word(24)], &[10.0, 22.0, 34.0], 8));
        assert!(!has_enclosing_rules(&[&word(12)], &[&word(24)], &[10.0], 8));
    }
}
