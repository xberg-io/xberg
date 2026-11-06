use std::ops::Range;

use xberg_native_pdf::object::Object;

use crate::engine::layout_region_text::PageRegionRanges;
use crate::extractors::pdf::rotation::TextSpan;
use crate::pdf::structure::types::LayoutHint;
use crate::types::{BoundingBox, LayoutRegion};

const MAX_REGION_SPAN_CHECKS: usize = 1_000_000;

pub(super) fn page_area(doc: &xberg_native_pdf::PdfDocument, page: usize, hints: &[LayoutHint]) -> Option<f64> {
    use xberg_native_pdf::PageRotation;

    if !matches!(
        doc.get_page_rotation_status(page).ok()?,
        PageRotation::Absent | PageRotation::Valid(0)
    ) {
        return None;
    }
    let object = doc.get_page(page).ok()?;
    let dictionary = object.as_dict()?;
    if dictionary.contains_key("UserUnit") {
        return None;
    }
    if let Some(crop) = dictionary.get("CropBox")
        && strict_page_box(doc, crop)? != strict_page_box(doc, dictionary.get("MediaBox")?)?
    {
        return None;
    }
    let (left, bottom, right, top) = doc.get_page_media_box(page).ok()?;
    (left == 0.0
        && bottom == 0.0
        && right.is_finite()
        && top.is_finite()
        && right > 0.0
        && top > 0.0
        && hints.iter().all(|hint| {
            hint.left >= left
                && hint.right <= right
                && hint.left < hint.right
                && hint.bottom >= bottom
                && hint.top <= top
                && hint.bottom < hint.top
                && (0.0..=1.0).contains(&hint.confidence)
        }))
    .then_some(f64::from(right) * f64::from(top))
}

fn strict_page_box(doc: &xberg_native_pdf::PdfDocument, object: &Object) -> Option<[f64; 4]> {
    let Object::Array(entries) = doc.resolve_object(object).ok()? else {
        return None;
    };
    let [left, bottom, right, top] = entries.as_slice() else {
        return None;
    };
    // A rounded match in the raster frame cannot prove that the PDF boxes are identical.
    let number = |entry: &Object| match doc.resolve_object(entry).ok()? {
        Object::Integer(value) if (value as f64) as i128 == i128::from(value) => Some(value as f64),
        Object::Real(value) if value.is_finite() => Some(value),
        _ => None,
    };
    let (left, bottom, right, top) = (number(left)?, number(bottom)?, number(right)?, number(top)?);
    (left == 0.0 && bottom == 0.0 && right > 0.0 && top > 0.0).then_some([left, bottom, right, top])
}

pub(super) fn assemble(
    page_number: u32,
    native_spans: &[xberg_native_pdf::layout::TextSpan],
    spans: &[TextSpan],
    order: &[usize],
    hints: &[LayoutHint],
    classes: &[crate::layout::LayoutClass],
    area: f64,
) -> (String, Option<PageRegionRanges>) {
    use crate::pdf::native::text::{assemble_upright_layout_text, assemble_upright_layout_text_observed};

    let Some(groups) = contiguous_groups(spans, order, hints) else {
        return (assemble_upright_layout_text(native_spans, order), None);
    };
    let mut offsets = Vec::with_capacity(order.len());
    let text = assemble_upright_layout_text_observed(native_spans, order, |start, end| offsets.push((start, end)));
    let regions: Vec<_> = groups
        .into_iter()
        .filter_map(|(owner, run)| {
            let range = offsets.get(run.start)?.0..offsets.get(run.end - 1)?.1;
            let range = whole_word_range(&text, range)?;
            let hint = &hints[owner];
            Some((region(hint, classes[owner], area), range))
        })
        .collect();
    let pending = (!regions.is_empty()).then(|| PageRegionRanges {
        page_number,
        fingerprint: blake3::hash(text.as_bytes()),
        regions,
    });
    (text, pending)
}

fn contiguous_groups(spans: &[TextSpan], order: &[usize], hints: &[LayoutHint]) -> Option<Vec<(usize, Range<usize>)>> {
    if hints.is_empty() || spans.len().checked_mul(hints.len())? > MAX_REGION_SPAN_CHECKS || order.len() != spans.len()
    {
        return None;
    }
    let mut seen = vec![false; spans.len()];
    let mut groups: Vec<Option<Range<usize>>> = vec![None; hints.len()];
    for (position, &index) in order.iter().enumerate() {
        let span = spans.get(index)?;
        if std::mem::replace(&mut seen[index], true) {
            return None;
        }
        let Some(owner) = unique_owner(span, hints)? else {
            continue;
        };
        match &mut groups[owner] {
            run @ None => *run = Some(position..position + 1),
            Some(run) if run.end == position => run.end += 1,
            Some(_) => return None,
        }
    }
    let mut owned: Vec<_> = groups
        .into_iter()
        .enumerate()
        .filter_map(|(index, run)| Some((index, run?)))
        .collect();
    owned.sort_by_key(|(_, range)| range.start);
    (!owned.is_empty()).then_some(owned)
}

fn unique_owner(span: &TextSpan, hints: &[LayoutHint]) -> Option<Option<usize>> {
    let (left, bottom, right, top) = (span.x, span.y, span.x + span.width, span.y + span.height);
    if ![left, bottom, right, top].into_iter().all(f32::is_finite) || left >= right || bottom >= top {
        return None;
    }
    let mut owner = None;
    for (index, hint) in hints.iter().enumerate() {
        if left < hint.right && right > hint.left && bottom < hint.top && top > hint.bottom {
            if owner.is_some() || left < hint.left || right > hint.right || bottom < hint.bottom || top > hint.top {
                return None;
            }
            owner = Some(index);
        }
    }
    Some(owner)
}

fn whole_word_range(text: &str, range: Range<usize>) -> Option<Range<usize>> {
    if text
        .get(..range.start)?
        .chars()
        .next_back()
        .is_some_and(|ch| !ch.is_whitespace())
        || text
            .get(range.end..)?
            .chars()
            .next()
            .is_some_and(|ch| !ch.is_whitespace())
    {
        return None;
    }
    let raw = text.get(range.clone())?;
    let start = range.start + raw.len() - raw.trim_start().len();
    let end = range.start + raw.trim_end().len();
    (start < end).then_some(start..end)
}

fn region(hint: &LayoutHint, class: crate::layout::LayoutClass, area: f64) -> LayoutRegion {
    let [left, bottom, right, top] = [hint.left, hint.bottom, hint.right, hint.top].map(f64::from);
    LayoutRegion {
        class_name: class.as_str().to_owned(),
        confidence: f64::from(hint.confidence),
        bounding_box: BoundingBox {
            x0: left,
            y0: bottom,
            x1: right,
            y1: top,
        },
        area_fraction: (right - left) * (top - bottom) / area,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pdf::structure::types::LayoutHintClass;

    #[test]
    fn region_evidence_rejects_partial_overlapping_and_interleaved_ownership() {
        let span = |x| TextSpan {
            text: "word".into(),
            x,
            y: 10.,
            width: 20.,
            height: 10.,
            rotation_degrees: 0.,
        };
        let hint = |left, right| LayoutHint {
            class_name: LayoutHintClass::Text,
            confidence: 0.75,
            left,
            right,
            bottom: 0.,
            top: 30.,
        };
        let spans = [span(10.), span(110.), span(40.)];
        let hints = [hint(0., 70.), hint(100., 150.)];
        assert_eq!(
            contiguous_groups(&spans, &[1, 2, 0], &hints),
            Some(vec![(1, 0..1), (0, 1..3)])
        );
        assert_eq!(contiguous_groups(&spans, &[0, 1, 2], &hints), None);
        assert_eq!(contiguous_groups(&spans, &[0, 0, 1], &hints), None);
        assert_eq!(contiguous_groups(&spans[..1], &[0], &[hint(0., 25.)]), None);
        assert_eq!(
            contiguous_groups(&spans[..1], &[0], &[hint(0., 70.), hint(20., 70.)]),
            None
        );
        assert_eq!(whole_word_range("é lan", 1..5), None);
        assert_eq!(whole_word_range("élan", 0..2), None);
        assert_eq!(whole_word_range("élan 東京", 0..5), Some(0..5));
    }
}
