//! Source geometry from Gutenberg #38769 p319, public PDF SHA-256
//! 0ce1933e43fd03bb095a03f1007df9bf40decdb7924bf7b81f2743eaa071d16a.
//! Independent source readers place P₁ in Fig.44, below the running header. The
//! inferred grid has an empty header anchor above that separate label. ~keep

use super::*;

fn source_spans(label: &str) -> Vec<TextSpan> {
    let source = [
        ("145]", [32.398716, 558.367, 20.811619, 11.9552]),
        ("DERIVATIVES", [119.233, 558.367, 68.9133, 9.962585]),
        ("P", [129.80977, 481.86285, 6.919525, 10.942352]),
        (label, [92.7416, 457.57288, 10.763481, 12.589844]),
    ];
    source
        .into_iter()
        .map(|(text, [x, y, w, h])| {
            let mut span = tests::create_test_span(text, x, y, w, h);
            span.font_size = h;
            span
        })
        .collect()
}

fn source_grid(label: &str) -> (GridStructure, Vec<TextSpan>, Vec<Vec<CellMergeInfo>>) {
    let spans = source_spans(label);
    let columns = [
        [35.31392, 63.115913, 90.91791],
        [90.91791, 109.45242, 127.98693],
        [127.98693, 146.5218, 165.05669],
    ]
    .into_iter()
    .map(|[x_min, x_center, x_max]| ColumnCluster {
        x_min,
        x_center,
        x_max,
        span_indices: Vec::new(),
    })
    .collect();
    let rows = [[492.97855, 532.1504, 571.3222], [380.61255, 436.79553, 492.97855]]
        .into_iter()
        .map(|[y_min, y_center, y_max]| RowCluster {
            y_min,
            y_center,
            y_max,
            span_indices: Vec::new(),
        })
        .collect();
    let grid = GridStructure {
        columns,
        rows,
        cells: vec![vec![vec![0], vec![], vec![1]], vec![vec![], vec![3], vec![2]]],
    };
    let mut merges = vec![
        vec![
            CellMergeInfo {
                rowspan: 1,
                colspan: 1,
                covered: false
            };
            3
        ];
        2
    ];
    for column in [0, 1] {
        merges[0][column].rowspan = 2;
        merges[1][column].covered = true;
    }
    (grid, spans, merges)
}

#[test]
fn an_empty_inferred_header_anchor_does_not_claim_a_separate_figure_label() {
    let (grid, spans, merges) = source_grid("P1");
    let table = grid_to_table(&grid, &spans, Some(merges));
    assert_eq!(table.rows[0].cells[1].text, "");
    let actual: Vec<_> = table
        .rows
        .iter()
        .flat_map(|r| &r.cells)
        .flat_map(|c| &c.spans)
        .map(|s| (s.text.as_str(), s.bbox))
        .collect();
    let expected: Vec<_> = spans[..3].iter().map(|s| (s.text.as_str(), s.bbox)).collect();
    assert_eq!(
        actual, expected,
        "independent anchored source occurrences remain unchanged"
    );
}

#[test]
fn unanchored_identical_digits_and_labels_are_not_borrowed_from_covered_slots() {
    for text in ["1", "Label"] {
        let (grid, mut spans, merges) = source_grid(text);
        spans[0].text = text.into();
        let table = grid_to_table(&grid, &spans, Some(merges));
        let actual: Vec<_> = table
            .rows
            .iter()
            .flat_map(|r| &r.cells)
            .flat_map(|c| &c.spans)
            .filter(|s| s.text == text)
            .collect();
        assert_eq!(
            actual.len(),
            1,
            "only the independently anchored {text} occurrence belongs to this grid"
        );
        assert_eq!(actual[0].bbox, spans[0].bbox);
    }
}

#[test]
fn a_nonempty_merge_anchor_retains_repeated_occurrences_by_source_identity() {
    for text in ["1", "Label"] {
        let (mut grid, mut spans, merges) = source_grid(text);
        spans[1].text = text.into();
        grid.cells[0][1] = vec![1];
        grid.cells[0][2].clear();
        let table = grid_to_table(&grid, &spans, Some(merges));
        let cell = &table.rows[0].cells[1];
        assert_eq!(cell.spans.len(), 2);
        assert_eq!(
            cell.spans.iter().map(|s| s.text.as_str()).collect::<Vec<_>>(),
            vec![text, text]
        );
        assert_eq!(
            cell.spans.iter().map(|s| s.bbox).collect::<Vec<_>>(),
            vec![spans[1].bbox, spans[3].bbox]
        );
    }
}

#[test]
fn an_off_center_colspan_keeps_its_right_hand_header_source_once() {
    let (mut grid, spans, mut merges) = source_grid("P1");
    grid.cells[0][0].clear();
    grid.cells[0][1] = vec![1];
    grid.cells[0][2].clear();
    for row in &mut merges {
        row.fill(CellMergeInfo {
            rowspan: 1,
            colspan: 1,
            covered: false,
        });
    }
    merges[0][0].colspan = 2;
    merges[0][1].covered = true;
    let table = grid_to_table(&grid, &spans, Some(merges));
    assert_eq!(table.rows[0].cells[0].text, "DERIVATIVES");
    assert_eq!(table.rows[0].cells[0].spans.len(), 1);
    assert_eq!(table.rows[0].cells[0].spans[0].bbox, spans[1].bbox);
}
