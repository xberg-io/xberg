use super::{BoundingBox, PdfParagraph};
use crate::pdf::hierarchy::SegmentData;

pub(super) fn order(page: &mut [PdfParagraph], tracks: &[(f32, f32)], table: BoundingBox) {
    if tracks.len() < 3
        || page.len() < 2
        || page.len() > tracks.len() + 1
        || !tracks
            .iter()
            .all(|&(left, right)| left.is_finite() && right.is_finite() && left < right)
        || !tracks.windows(2).all(|pair| pair[0].1 < pair[1].0)
    {
        return;
    }
    if header_band(page, tracks, table).is_some() {
        page.sort_by(|left, right| {
            right.lines[0]
                .baseline_y
                .total_cmp(&left.lines[0].baseline_y)
                .then_with(|| left.lines[0].segments[0].x.total_cmp(&right.lines[0].segments[0].x))
        });
    }
}

fn header_band(page: &[PdfParagraph], tracks: &[(f32, f32)], table: BoundingBox) -> Option<()> {
    let mut labels = vec![false; tracks.len()];
    let mut label_geometry = None;
    let mut intro_bottom = None;
    for paragraph in page {
        let segments = source_line(paragraph, table)?;
        let first = segments.first()?;
        if segments.iter().all(|segment| segment.is_bold) {
            let geometry = (first.y, first.height, first.font_size);
            if *label_geometry.get_or_insert(geometry) != geometry
                || f64::from(first.y) - table.y1 > f64::from(first.font_size)
            {
                return None;
            }
            for segment in segments {
                if !claim_label(segment, geometry, tracks, &mut labels) {
                    return None;
                }
            }
        } else if !intro_line(segments, tracks) || intro_bottom.replace(first.y).is_some() {
            return None;
        }
    }
    let (bottom, height, _) = label_geometry?;
    (intro_bottom? > bottom + height && labels.into_iter().all(|present| present)).then_some(())
}

fn source_line(paragraph: &PdfParagraph, table: BoundingBox) -> Option<&[SegmentData]> {
    let [line] = paragraph.lines.as_slice() else {
        return None;
    };
    if !paragraph.text.is_empty()
        || paragraph.is_list_item
        || paragraph.is_code_block
        || paragraph.is_formula
        || paragraph.is_page_furniture
        || paragraph.layout_class.is_some()
        || paragraph.layout_region_path.is_some()
        || paragraph.caption_for.is_some()
        || !line.native_scripts.is_empty()
        || !line.baseline_y.is_finite()
    {
        return None;
    }
    let valid = line.segments.iter().all(|segment| {
        segment.rotation_degrees == 0.0
            && segment.assigned_role.is_none()
            && segment.y == line.baseline_y
            && segment.baseline_y == segment.y
            && segment.width.is_finite()
            && segment.width > 0.0
            && segment.height.is_finite()
            && segment.height > 0.0
            && segment.font_size.is_finite()
            && segment.font_size > 0.0
            && !segment.text.trim().is_empty()
            && f64::from(segment.x) >= table.x0
            && f64::from(segment.x + segment.width) <= table.x1
            && f64::from(segment.y) > table.y1
    });
    (valid && line.segments.windows(2).all(|pair| pair[0].x < pair[1].x)).then_some(&line.segments)
}

fn claim_label(segment: &SegmentData, geometry: (f32, f32, f32), tracks: &[(f32, f32)], labels: &mut [bool]) -> bool {
    if (segment.y, segment.height, segment.font_size) != geometry || segment.is_italic || segment.is_monospace {
        return false;
    }
    let right = segment.x + segment.width;
    let Some(index) = tracks.iter().position(|&(left, end)| segment.x >= left && right <= end) else {
        return false;
    };
    !std::mem::replace(&mut labels[index], true)
}

fn intro_line(segments: &[SegmentData], tracks: &[(f32, f32)]) -> bool {
    let first = &segments[0];
    let last = &segments[segments.len() - 1];
    let right = last.x + last.width;
    segments.iter().all(|segment| !segment.is_bold)
        && last.text.trim_end().ends_with(':')
        && tracks
            .iter()
            .filter(|&&(left, end)| first.x < end && right > left)
            .count()
            >= 2
}
