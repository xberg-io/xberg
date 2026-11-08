use super::*;

fn scaled_ink(scale: u32, columns: &[Range<usize>], rows: Range<usize>) -> Source {
    let (width, height) = (200, 180);
    let mut base = vec![255; width * height];
    for y in rows {
        for column in columns {
            base[y * width + column.start..y * width + column.end].fill(0);
        }
    }
    let mut samples = Vec::new();
    for row in base.chunks_exact(width) {
        let scaled: Vec<_> = row
            .iter()
            .flat_map(|&sample| std::iter::repeat_n(sample, scale as usize))
            .collect();
        for _ in 0..scale {
            samples.extend_from_slice(&scaled);
        }
    }
    Source {
        width: width as u32 * scale,
        height: height as u32 * scale,
        dpi: 150.0 * f64::from(scale),
        retained_bytes: samples.len(),
        samples,
        certify_rows: true,
    }
}

#[test]
fn same_physical_ink_rows_are_certified_at_150_and_300_dpi() {
    for (columns, rows) in [(vec![20..40], 20..70), (vec![20..40, 70..110], 20..30)] {
        for scale in [1, 2] {
            let source = scaled_ink(scale, &columns, rows.clone());
            assert_eq!(
                source.blank_bands(|| false, 1),
                Some(vec![0..rows.start as u32 * scale, rows.end as u32 * scale..180 * scale]),
                "same physical line and inter-word gap at {} DPI",
                source.dpi
            );
        }
    }
}

#[test]
fn physically_wide_columns_and_tall_ink_remain_uncertified_at_each_density() {
    for scale in [1, 2] {
        let columns = scaled_ink(scale, &[20..40, 130..150], 20..30);
        assert_eq!(columns.blank_bands(|| false, 1), None);
        let tall_ink = scaled_ink(scale, &[20..40], 20..120);
        assert_eq!(tall_ink.blank_bands(|| false, 1), None);
    }
}

#[test]
fn two_columns_and_staggered_line_starts_refuse_ordering() {
    let mut line = LineFootprint::new(200, 150.0);
    let mut row = vec![255; 200];
    row[10..30].fill(0);
    row[150..170].fill(0);
    assert_eq!(line.add(&row), Some(()));
    assert_eq!(line.finish(), None);

    let mut line = LineFootprint::new(200, 150.0);
    row[150..170].fill(255);
    assert_eq!(line.add(&row), Some(()));
    assert_eq!(line.finish(), Some(()));
    row[10..30].fill(255);
    row[150..170].fill(0);
    assert_eq!(line.add(&row), Some(()));
    assert_eq!(line.finish(), None);
}
