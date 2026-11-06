//! Placement of two table-only OCR results from one detached PDF render.

use crate::types::internal::InternalDocument;
use crate::types::internal_builder::InternalDocumentBuilder;
use crate::types::{BoundingBox, Table};

pub(super) fn normalize(tables: &mut [Table], processed: (u32, u32), render: (u32, u32)) -> bool {
    let [first, second] = tables else { return false };
    let (Some(a), Some(b)) = (first.bounding_box, second.bounding_box) else {
        return false;
    };
    if ![a, b].into_iter().all(|bbox| within(bbox, processed)) || render.0 == 0 || render.1 == 0 {
        return false;
    }
    let scale_x = f64::from(render.0) / f64::from(processed.0);
    let scale_y = f64::from(render.1) / f64::from(processed.1);
    for (table, bbox) in [(first, a), (second, b)] {
        table.bounding_box = Some(BoundingBox {
            x0: bbox.x0 * scale_x,
            y0: bbox.y0 * scale_y,
            x1: bbox.x1 * scale_x,
            y1: bbox.y1 * scale_y,
        });
    }
    true
}

fn within(bbox: BoundingBox, frame: (u32, u32)) -> bool {
    0.0 <= bbox.x0
        && bbox.x0 < bbox.x1
        && bbox.x1 <= f64::from(frame.0)
        && 0.0 <= bbox.y0
        && bbox.y0 < bbox.y1
        && bbox.y1 <= f64::from(frame.1)
}

pub(super) fn assemble(tables: &[Table]) -> Option<InternalDocument> {
    let [first, second] = tables else { return None };
    let (a, b) = (first.bounding_box?, second.bounding_box?);
    let ordered = if a.y1 <= b.y0 {
        [first, second]
    } else if b.y1 <= a.y0 {
        [second, first]
    } else {
        return None;
    };
    if ordered.iter().any(|table| table.markdown.trim().is_empty()) || first.page_number != second.page_number {
        return None;
    }
    // The detached page owner converts these top-left render rectangles to page points. ~keep
    let mut builder = InternalDocumentBuilder::new("pdf");
    for table in ordered {
        builder.push_table(table.clone(), Some(table.page_number), table.bounding_box);
    }
    let mut document = builder.build();
    document.page_numbers = vec![first.page_number];
    Some(document)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(name: &str, y0: f64, y1: f64) -> Table {
        Table {
            cells: vec![vec![name.to_owned()]],
            markdown: name.to_owned(),
            page_number: 2,
            bounding_box: Some(BoundingBox {
                x0: 20.,
                y0,
                x1: 100.,
                y1,
            }),
            ..Default::default()
        }
    }

    #[test]
    fn normalized_table_pair_keeps_render_rectangles_and_reads_top_before_bottom() {
        let mut tables = [table("lower", 100., 140.), table("upper", 20., 70.)];
        assert!(normalize(&mut tables, (200, 160), (100, 80)));
        let doc = assemble(&tables).expect("distinct vertical bands");
        assert_eq!(doc.page_numbers, [2]);
        assert_eq!(doc.tables[0].cells[0][0], "upper");
        assert_eq!(doc.tables[1].cells[0][0], "lower");
        let upper = BoundingBox {
            x0: 10.,
            y0: 10.,
            x1: 50.,
            y1: 35.,
        };
        assert_eq!(doc.tables[0].bounding_box, Some(upper));
        assert_eq!(doc.elements[0].bbox, Some(upper));
    }

    #[test]
    fn ambiguous_table_pair_does_not_guess_order_or_partially_normalize() {
        let mut tables = [table("upper", 20., 70.), table("overlap", 50., 100.)];
        assert!(assemble(&tables).is_none());
        let first = tables[0].bounding_box;
        tables[1].bounding_box.as_mut().unwrap().x1 = f64::INFINITY;
        assert!(!normalize(&mut tables, (200, 160), (100, 80)));
        assert_eq!(tables[0].bounding_box, first);
        assert!(!normalize(&mut tables[..1], (200, 160), (100, 80)));
    }
}
