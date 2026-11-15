use crate::pdf::table_reconstruct::HocrWord;
use crate::types::Table;
use xberg_native_pdf::elements::{PathContent, PathOperation};

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
        .filter(|path| path.is_horizontal_line(0.5) && (path.has_stroke() || has_divider_fill(path)))
        .map(|path| (path.rendered_bbox(), path.has_stroke()))
        .filter(|(rule, _)| f64::from(rule.x) < bounds.x1 && f64::from(rule.right()) > bounds.x0)
        .map(|(rule, stroked)| {
            let full_width =
                stroked && f64::from(rule.x) <= bounds.x0 + 1.0 && f64::from(rule.right()) >= bounds.x1 - 1.0;
            (page_height - rule.center().y, full_width)
        })
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

// ~keep A filled sliver can veto joining two rows. It supplies no enclosing stroke.
fn has_divider_fill(path: &PathContent) -> bool {
    path.matrix.is_none()
        && is_axis_aligned_rectangle(&path.operations)
        && path.bbox.height > 0.0
        && path.fill_color.is_some_and(|color| {
            let channels = [color.r, color.g, color.b];
            channels.iter().all(|channel| (0.0..=1.0).contains(channel))
                && channels.iter().any(|channel| *channel < 0.9)
        })
}

fn is_axis_aligned_rectangle(operations: &[PathOperation]) -> bool {
    match operations {
        [PathOperation::Rectangle(..)] | [PathOperation::Rectangle(..), PathOperation::ClosePath] => true,
        [
            PathOperation::MoveTo(x0, y0),
            PathOperation::LineTo(x1, y1),
            PathOperation::LineTo(x2, y2),
            PathOperation::LineTo(x3, y3),
            tail @ ..,
        ] if matches!(tail, [] | [PathOperation::ClosePath]) => {
            (y0 == y1 && x1 == x2 && y2 == y3 && x3 == x0 && x0 != x1 && y1 != y2)
                || (x0 == x1 && y1 == y2 && x2 == x3 && y3 == y0 && y0 != y1 && x1 != x2)
        }
        _ => false,
    }
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

fn has_enclosing_rules(upper: &[&HocrWord], lower: &[&HocrWord], rules: &[(f32, bool)], height: u32) -> bool {
    let upper_top = upper.iter().map(|word| word.top).min().unwrap_or(0) as f32;
    let lower_bottom = lower
        .iter()
        .map(|word| word.top.saturating_add(word.height))
        .max()
        .unwrap_or(0) as f32;
    let tolerance = height as f32;
    rules
        .iter()
        .any(|&(y, full_width)| full_width && (upper_top - tolerance..=upper_top).contains(&y))
        && rules
            .iter()
            .any(|&(y, full_width)| full_width && (lower_bottom..=lower_bottom + tolerance).contains(&y))
        && !rules.iter().any(|&(y, _)| y > upper_top && y < lower_bottom)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_legacy_near_rectangle_is_not_exact_filled_divider_evidence() {
        let mut path = PathContent::from_operations(vec![
            PathOperation::MoveTo(0.0, 0.0),
            PathOperation::LineTo(42.0, 0.05),
            PathOperation::LineTo(42.0, 0.45),
            PathOperation::LineTo(0.0, 0.4),
            PathOperation::ClosePath,
        ]);
        path.stroke_color = None;
        path.fill_color = Some(xberg_native_pdf::layout::Color::new(0.0, 0.0, 0.0));
        assert!(path.is_horizontal_line(0.5));
        assert!(!has_divider_fill(&path));
    }

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
        assert!(has_enclosing_rules(
            &[&word(12)],
            &[&word(24)],
            &[(10.0, true), (34.0, true)],
            8
        ));
        assert!(!has_enclosing_rules(
            &[&word(12)],
            &[&word(24)],
            &[(10.0, true), (22.0, true), (34.0, true)],
            8
        ));
        assert!(!has_enclosing_rules(&[&word(12)], &[&word(24)], &[(10.0, true)], 8));
    }
    #[test]
    fn consolidation_respects_a_label_only_divider() {
        let word = |text: &str, left, top| HocrWord {
            text: text.into(),
            left,
            top,
            width: 10,
            height: 8,
            confidence: 100.0,
        };
        let words = [
            word("Operating", 10, 12),
            word("income", 22, 12),
            word("before", 34, 12),
            word("tax", 18, 24),
            word("100", 75, 24),
            word("200", 100, 24),
        ];
        let rows = vec![
            vec!["Operating income before".into(), String::new(), String::new()],
            vec!["tax".into(), "100".into(), "200".into()],
        ];
        let table = || Table {
            cells: rows.clone(),
            bounding_box: Some(crate::types::BoundingBox {
                x0: 10.0,
                x1: 110.0,
                y0: 68.0,
                y1: 88.0,
            }),
            ..Table::default()
        };
        let rule = |left, right, top| {
            let y = 100.0_f32 - top;
            PathContent::from_operations(vec![PathOperation::MoveTo(left, y), PathOperation::LineTo(right, y)])
        };
        let rules = vec![rule(8.0, 112.0, 10.0), rule(8.0, 112.0, 34.0)];
        let mut united = table();
        consolidate(&mut united, &words, &rules, 100.0);
        assert_eq!(united.cells.len(), 1);
        assert_eq!(united.cells[0][0], "Operating income before tax");
        let mut divided = table();
        let mut stroked = rules.clone();
        stroked.push(rule(8.0, 50.0, 22.0));
        consolidate(&mut divided, &words, &stroked, 100.0);
        assert_eq!(divided.cells, rows, "stroked label-only interior divider");

        let filled = |height, color| {
            let mut path = PathContent::rect(8.0, 78.0 - height / 2.0, 42.0, height);
            path.stroke_color = None;
            path.fill_color = color;
            path
        };
        let black = xberg_native_pdf::layout::Color::new(0.0, 0.0, 0.0);
        let mut rule_fill = rules.clone();
        rule_fill.push(filled(0.4, Some(black)));
        let mut divided = table();
        consolidate(&mut divided, &words, &rule_fill, 100.0);
        assert_eq!(divided.cells, rows, "filled label-only interior divider");
        let white = xberg_native_pdf::layout::Color::new(1.0, 1.0, 1.0);
        for path in [filled(0.4, None), filled(0.4, Some(white)), filled(2.0, Some(black))] {
            let mut paths = rules.clone();
            paths.push(path);
            let mut wrapped = table();
            consolidate(&mut wrapped, &words, &paths, 100.0);
            assert_eq!(
                wrapped.cells, united.cells,
                "paintless, white, and thick paths do not veto"
            );
        }
        let mut partial_boundary = table();
        consolidate(
            &mut partial_boundary,
            &words,
            &[rule(8.0, 50.0, 10.0), rule(8.0, 112.0, 34.0)],
            100.0,
        );
        assert_eq!(partial_boundary.cells, rows, "partial outer rule");
    }
}
