use super::{Edge, IntersectionCell, Table, TableRow};
use serde_json::{Value, json};
use std::cell::RefCell;

thread_local! {
    static JOURNAL: RefCell<Option<Vec<Value>>> = const { RefCell::new(None) };
}

struct Reset;

impl Drop for Reset {
    fn drop(&mut self) {
        drop(JOURNAL.with(|journal| journal.borrow_mut().take()));
    }
}

pub(crate) fn capture<T>(run: impl FnOnce() -> T) -> (T, Vec<Value>) {
    JOURNAL.with(|journal| {
        let mut journal = journal.borrow_mut();
        assert!(journal.is_none(), "diagnostic capture must not nest");
        *journal = Some(Vec::new());
    });
    let reset = Reset;
    let output = run();
    let events = JOURNAL.with(|journal| journal.borrow_mut().take().expect("active diagnostic"));
    drop(reset);
    (output, events)
}

pub(crate) fn event(stage: &str, payload: impl FnOnce() -> Value) {
    JOURNAL.with(|journal| {
        if let Some(events) = journal.borrow_mut().as_mut() {
            events.push(json!({"stage": stage, "payload": payload()}));
        }
    });
}

pub(crate) fn rows(rows: &[TableRow]) -> Value {
    json!(
        rows.iter()
            .map(|row| json!({
                "is_header": row.is_header,
                "cells": row.cells.iter().map(|cell| json!({
                    "text": cell.text, "bbox": cell.bbox, "rowspan": cell.rowspan, "colspan": cell.colspan,
                    "is_header": cell.is_header, "mcids": cell.mcids, "spans": cell.spans,
                    "source_span_ids": Value::Null,
                })).collect::<Vec<_>>(),
            }))
            .collect::<Vec<_>>()
    )
}

pub(crate) fn tables(tables: &[Table]) -> Value {
    json!(
        tables
            .iter()
            .map(|table| json!({
                "bbox": table.bbox, "col_count": table.col_count, "has_header": table.has_header,
                "rows": rows(&table.rows),
            }))
            .collect::<Vec<_>>()
    )
}

pub(super) fn grid(cells: &[IntersectionCell], xs: &[f32], ys: &[f32], edges: &[Edge], intersections: bool) {
    event("intersection_grid", || {
        json!({
            "xs": xs, "ys": ys, "cells_are_intersections": intersections,
            "cells": cells.iter().map(|cell| [cell.x1, cell.y1, cell.x2, cell.y2]).collect::<Vec<_>>(),
            "vertical_edges": edges.iter().map(|edge| [edge.coord, edge.start, edge.end]).collect::<Vec<_>>(),
        })
    });
}
