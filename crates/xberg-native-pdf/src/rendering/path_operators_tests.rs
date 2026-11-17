use super::tests::minimal_pdf_doc;
use super::{Operator, PageRenderer, Pixmap, RenderOptions, Transform};
use crate::object::Object;
use std::collections::HashMap;

fn path_pixmap(operators: &[Operator]) -> Pixmap {
    let mut pixmap = Pixmap::new(32, 32).unwrap();
    PageRenderer::new(RenderOptions::default())
        .execute_operators(
            &mut pixmap,
            Transform::identity(),
            operators,
            &minimal_pdf_doc(),
            0,
            &Object::Dictionary(HashMap::new()),
        )
        .expect("render path operators");
    pixmap
}

#[test]
fn test_negative_rect_normalization() {
    let rendered = path_pixmap(&[
        Operator::Rectangle {
            x: 8.0,
            y: 24.0,
            width: 16.0,
            height: -16.0,
        },
        Operator::Fill,
    ]);
    assert_eq!(rendered.pixel(12, 12).unwrap().alpha(), 255);
    assert_eq!(rendered.pixel(12, 4).unwrap().alpha(), 0);
    assert_eq!(rendered.pixel(12, 28).unwrap().alpha(), 0);
}

#[test]
fn test_negative_rect_both_negative() {
    let rendered = path_pixmap(&[
        Operator::Rectangle {
            x: 24.0,
            y: 24.0,
            width: -16.0,
            height: -16.0,
        },
        Operator::Fill,
    ]);
    assert_eq!(rendered.pixel(12, 12).unwrap().alpha(), 255);
    assert_eq!(rendered.pixel(4, 12).unwrap().alpha(), 0);
    assert_eq!(rendered.pixel(28, 12).unwrap().alpha(), 0);
}

#[test]
fn test_shorthand_curves_match_their_full_cubic_path() {
    let render = |curve_v, curve_y| {
        path_pixmap(&[
            Operator::MoveTo { x: 4.0, y: 4.0 },
            Operator::LineTo { x: 4.0, y: 24.0 },
            curve_v,
            curve_y,
            Operator::ClosePath,
            Operator::Stroke,
        ])
    };
    let shorthand = render(
        Operator::CurveToV {
            x2: 10.0,
            y2: 25.0,
            x3: 16.0,
            y3: 16.0,
        },
        Operator::CurveToY {
            x1: 22.0,
            y1: 14.0,
            x3: 26.0,
            y3: 4.0,
        },
    );
    let explicit = render(
        Operator::CurveTo {
            x1: 4.0,
            y1: 24.0,
            x2: 10.0,
            y2: 25.0,
            x3: 16.0,
            y3: 16.0,
        },
        Operator::CurveTo {
            x1: 22.0,
            y1: 14.0,
            x2: 26.0,
            y2: 4.0,
            x3: 26.0,
            y3: 4.0,
        },
    );
    assert!(
        shorthand.pixel(16, 4).unwrap().alpha() > 0,
        "closing edge must be stroked"
    );
    assert_eq!(shorthand.pixel(30, 12).unwrap().alpha(), 0);
    assert_eq!(shorthand.data(), explicit.data());
}
