use xberg::{BoundingBox, LayoutRegion};

#[test]
fn legacy_rust_layout_region_literal_and_wire_shape_still_work() {
    let region = LayoutRegion {
        class_name: "text".into(),
        confidence: 1.0,
        bounding_box: BoundingBox {
            x0: 0.0,
            y0: 0.0,
            x1: 2.0,
            y1: 2.0,
        },
        area_fraction: 0.5,
    };
    assert_eq!(
        serde_json::to_value(region).unwrap(),
        serde_json::json!({
            "class_name": "text",
            "confidence": 1.0,
            "bounding_box": {"x0": 0.0, "y0": 0.0, "x1": 2.0, "y1": 2.0},
            "area_fraction": 0.5
        })
    );
}
