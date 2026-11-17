//! Path construction that precedes clipping and painting in the page operator loop.

use crate::content::operators::Operator;
use tiny_skia::PathBuilder;

pub(super) fn construct(current_path: &mut PathBuilder, op: &Operator) -> bool {
    match op {
        Operator::MoveTo { x, y } => {
            current_path.move_to(*x, *y);
        }
        Operator::LineTo { x, y } => {
            current_path.line_to(*x, *y);
        }
        Operator::CurveTo { x1, y1, x2, y2, x3, y3 } => {
            current_path.cubic_to(*x1, *y1, *x2, *y2, *x3, *y3);
        }
        Operator::CurveToV { x2, y2, x3, y3 } => {
            if let Some(last) = current_path.last_point() {
                current_path.cubic_to(last.x, last.y, *x2, *y2, *x3, *y3);
            }
        }
        Operator::CurveToY { x1, y1, x3, y3 } => {
            current_path.cubic_to(*x1, *y1, *x3, *y3, *x3, *y3);
        }
        Operator::Rectangle { x, y, width, height } => {
            // Normalize negative width/height per PDF spec:
            // re with negative dimensions means the rect extends in the opposite direction
            // ~keep
            let (nx, nw) = if *width < 0.0 {
                (x + width, -width)
            } else {
                (*x, *width)
            };
            let (ny, nh) = if *height < 0.0 {
                (y + height, -height)
            } else {
                (*y, *height)
            };
            if let Some(rect) = tiny_skia::Rect::from_xywh(nx, ny, nw, nh) {
                current_path.push_rect(rect);
            }
        }
        Operator::ClosePath => {
            current_path.close();
        }
        _ => return false,
    }
    true
}
