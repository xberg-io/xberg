//! Unit tests for [`super`].
//!
//! Split out of `paths.rs` purely for file size, mirroring the split used for
//! `document.rs`/`document/tests.rs` and `extractors/text/mod.rs`/`extractors/text/tests.rs`. ~keep

use super::*;

#[test]
fn test_path_extractor_new() {
    let extractor = PathExtractor::new();
    assert_eq!(extractor.path_count(), 0);
    assert!(!extractor.has_current_path());
}

#[test]
fn test_simple_line_stroke() {
    let mut extractor = PathExtractor::new();

    extractor.move_to(10.0, 10.0);
    extractor.line_to(100.0, 10.0);
    extractor.stroke();

    let paths = extractor.finish();
    assert_eq!(paths.len(), 1);
    assert_eq!(paths[0].operations.len(), 2);
    assert!(paths[0].has_stroke());
    assert!(!paths[0].has_fill());
}

#[test]
fn filled_path_uses_black_before_any_color_operator() {
    let mut extractor = PathExtractor::new();
    extractor.rectangle(50.0, 50.0, 100.0, 80.0);
    extractor.fill(FillRule::NonZero);
    let [path] = <[_; 1]>::try_from(extractor.finish()).unwrap();
    assert_eq!(path.fill_color, Some(Color::black()));
    assert!(!path.has_stroke());
}

#[test]
fn test_rectangle_fill() {
    let mut extractor = PathExtractor::new();
    extractor.set_fill_color(Color::new(1.0, 0.0, 0.0));

    extractor.rectangle(50.0, 50.0, 100.0, 80.0);
    extractor.fill(FillRule::NonZero);

    let paths = extractor.finish();
    assert_eq!(paths.len(), 1);
    assert_eq!(paths[0].operations.len(), 1);
    assert!(!paths[0].has_stroke());
    assert!(paths[0].has_fill());

    assert_eq!(paths[0].bbox.x, 50.0);
    assert_eq!(paths[0].bbox.y, 50.0);
    assert_eq!(paths[0].bbox.width, 100.0);
    assert_eq!(paths[0].bbox.height, 80.0);
}

#[test]
fn test_closed_path() {
    let mut extractor = PathExtractor::new();
    extractor.set_fill_color(Color::new(0.5, 0.5, 0.5));

    extractor.move_to(0.0, 0.0);
    extractor.line_to(100.0, 0.0);
    extractor.line_to(100.0, 100.0);
    extractor.line_to(0.0, 100.0);
    extractor.close_path();
    extractor.fill_and_stroke(FillRule::NonZero);

    let paths = extractor.finish();
    assert_eq!(paths.len(), 1);
    assert_eq!(paths[0].operations.len(), 5); // MoveTo, 3x LineTo, ClosePath ~keep
    assert!(paths[0].has_stroke());
    assert!(paths[0].has_fill());
}

#[test]
fn test_bezier_curve() {
    let mut extractor = PathExtractor::new();

    extractor.move_to(0.0, 0.0);
    extractor.curve_to(25.0, 100.0, 75.0, 100.0, 100.0, 0.0);
    extractor.stroke();

    let paths = extractor.finish();
    assert_eq!(paths.len(), 1);
    assert_eq!(paths[0].operations.len(), 2);
    assert!(matches!(
        paths[0].operations[1],
        PathOperation::CurveTo(_, _, _, _, _, _)
    ));
}

#[test]
fn test_multiple_paths() {
    let mut extractor = PathExtractor::new();

    extractor.move_to(0.0, 0.0);
    extractor.line_to(100.0, 0.0);
    extractor.stroke();

    extractor.move_to(50.0, 0.0);
    extractor.line_to(50.0, 100.0);
    extractor.stroke();

    let paths = extractor.finish();
    assert_eq!(paths.len(), 2);
}

#[test]
fn test_end_path_clears_operations() {
    let mut extractor = PathExtractor::new();

    extractor.move_to(0.0, 0.0);
    extractor.line_to(100.0, 100.0);
    extractor.end_path();

    let paths = extractor.finish();
    assert_eq!(paths.len(), 0);
}

#[test]
fn test_line_style_properties() {
    let mut extractor = PathExtractor::new();
    extractor.set_line_width(3.0);
    extractor.set_line_cap(LineCap::Round);
    extractor.set_line_join(LineJoin::Bevel);
    extractor.set_stroke_color(Color::new(0.0, 0.0, 1.0));

    extractor.move_to(0.0, 0.0);
    extractor.line_to(100.0, 100.0);
    extractor.stroke();

    let paths = extractor.finish();
    assert_eq!(paths.len(), 1);
    assert_eq!(paths[0].stroke_width, 3.0);
    assert_eq!(paths[0].line_cap, LineCap::Round);
    assert_eq!(paths[0].line_join, LineJoin::Bevel);
}

#[test]
fn test_ctm_transformation() {
    let mut extractor = PathExtractor::new();

    extractor.set_ctm(Matrix::translation(50.0, 50.0));

    extractor.move_to(0.0, 0.0);
    extractor.line_to(100.0, 0.0);
    extractor.stroke();

    let paths = extractor.finish();
    assert_eq!(paths.len(), 1);

    if let PathOperation::MoveTo(x, y) = paths[0].operations[0] {
        assert_eq!(x, 50.0);
        assert_eq!(y, 50.0);
    } else {
        panic!("Expected MoveTo operation");
    }
}

#[test]
fn test_bbox_calculation() {
    let mut extractor = PathExtractor::new();

    extractor.move_to(10.0, 20.0);
    extractor.line_to(110.0, 20.0);
    extractor.line_to(110.0, 120.0);
    extractor.line_to(10.0, 120.0);
    extractor.close_path();
    extractor.stroke();

    let paths = extractor.finish();
    assert_eq!(paths.len(), 1);

    let bbox = &paths[0].bbox;
    assert_eq!(bbox.x, 10.0);
    assert_eq!(bbox.y, 20.0);
    assert_eq!(bbox.width, 100.0);
    assert_eq!(bbox.height, 100.0);
}

#[test]
fn test_curve_to_v() {
    let mut extractor = PathExtractor::new();

    extractor.move_to(0.0, 0.0);
    extractor.curve_to_v(50.0, 100.0, 100.0, 0.0);
    extractor.stroke();

    let paths = extractor.finish();
    assert_eq!(paths.len(), 1);

    if let PathOperation::CurveTo(x1, y1, _, _, _, _) = paths[0].operations[1] {
        assert_eq!(x1, 0.0);
        assert_eq!(y1, 0.0);
    }
}

#[test]
fn test_curve_to_y() {
    let mut extractor = PathExtractor::new();

    extractor.move_to(0.0, 0.0);
    extractor.curve_to_y(50.0, 100.0, 100.0, 0.0);
    extractor.stroke();

    let paths = extractor.finish();
    assert_eq!(paths.len(), 1);

    if let PathOperation::CurveTo(_, _, x2, y2, x3, y3) = paths[0].operations[1] {
        assert_eq!(x2, x3);
        assert_eq!(y2, y3);
    }
}

#[test]
fn test_fill_even_odd() {
    let mut extractor = PathExtractor::new();
    extractor.set_fill_color(Color::new(0.0, 1.0, 0.0));

    extractor.rectangle(0.0, 0.0, 100.0, 100.0);
    extractor.fill(FillRule::EvenOdd);

    let paths = extractor.finish();
    assert_eq!(paths.len(), 1);
    assert!(paths[0].has_fill());
}

#[test]
fn test_close_and_stroke() {
    let mut extractor = PathExtractor::new();

    extractor.move_to(0.0, 0.0);
    extractor.line_to(100.0, 0.0);
    extractor.line_to(50.0, 100.0);
    extractor.close_and_stroke();

    let paths = extractor.finish();
    assert_eq!(paths.len(), 1);
    // Operations: MoveTo, LineTo, LineTo, ClosePath ~keep
    assert_eq!(paths[0].operations.len(), 4);
    assert!(matches!(paths[0].operations[3], PathOperation::ClosePath));
}

#[test]
fn test_update_from_state() {
    let mut extractor = PathExtractor::new();

    let mut state = GraphicsState::new();
    state.line_width = 5.0;
    state.line_cap = 1; // Round ~keep
    state.line_join = 2; // Bevel ~keep
    state.stroke_color_rgb = (1.0, 0.0, 0.0);
    state.fill_color_rgb = (0.0, 1.0, 0.0);

    extractor.update_from_state(&state);

    extractor.rectangle(0.0, 0.0, 100.0, 100.0);
    extractor.fill_and_stroke(FillRule::NonZero);

    let paths = extractor.finish();
    assert_eq!(paths.len(), 1);
    assert_eq!(paths[0].stroke_width, 5.0);
    assert_eq!(paths[0].line_cap, LineCap::Round);
    assert_eq!(paths[0].line_join, LineJoin::Bevel);
}

#[test]
fn test_pop_xobject_marks_as_processed() {
    let mut ext = PathExtractor::new();
    let r = crate::object::ObjectRef::new(42, 0);

    assert!(ext.can_process_xobject(r));
    ext.push_xobject(r);
    ext.pop_xobject();
    assert!(
        !ext.can_process_xobject(r),
        "Successfully processed XObject should be permanently skipped"
    );
}

#[test]
fn test_pop_xobject_failed_allows_retry() {
    let mut ext = PathExtractor::new();
    let r = crate::object::ObjectRef::new(42, 0);

    assert!(ext.can_process_xobject(r));
    ext.push_xobject(r);
    ext.pop_xobject_failed();
    assert!(ext.can_process_xobject(r), "Failed XObject should be retryable");
}

#[test]
fn test_oc_layer_truncate_isolates_unbalanced_xobject() {
    // A page-level `/OC` region is active; we then descend into a Form
    // XObject that opens a `/OC` marker but (malformed) never closes it.
    // `truncate_oc_layers` back to the entry depth must drop the leaked
    // entry so the caller's next path keeps the page-level layer rather
    // than inheriting the XObject's. ~keep
    let mut ext = PathExtractor::new();
    ext.set_stroke_color(Color::black());

    ext.push_oc_layer(Some("A-GRID".to_string()));
    let base = ext.oc_layer_depth();

    ext.push_oc_layer(Some("S-COLS".to_string()));
    ext.move_to(0.0, 0.0);
    ext.line_to(10.0, 0.0);
    ext.stroke();
    // (no matching EMC — simulates a malformed XObject stream) ~keep
    ext.truncate_oc_layers(base);

    ext.move_to(0.0, 20.0);
    ext.line_to(10.0, 20.0);
    ext.stroke();

    let paths = ext.finish();
    assert_eq!(paths.len(), 2);
    assert_eq!(paths[0].layer.as_deref(), Some("S-COLS"));
    assert_eq!(paths[1].layer.as_deref(), Some("A-GRID"));
}

#[test]
fn test_oc_layer_effective_is_o1_top_of_stack() {
    // push_oc_layer pre-resolves the effective layer, so a non-OC `BMC`
    // nested inside an `/OC` region inherits the OCG name and the top of
    // the stack is always the active layer. ~keep
    let mut ext = PathExtractor::new();
    ext.push_oc_layer(Some("A-WALL".to_string()));
    ext.push_oc_layer(None);
    assert_eq!(ext.current_layer().as_deref(), Some("A-WALL"));
    ext.pop_oc_layer();
    ext.pop_oc_layer();
    assert_eq!(ext.current_layer(), None);
}

#[test]
fn trailing_move_to_does_not_extend_the_bounding_box_to_the_page() {
    // GH#1759 carrier, as it arrives from the content stream:
    // `0 42.63 595.28 -28.35 re  0 842 m  f*`. The negative-height `re`
    // must be normalised and the lone trailing `m` — a subpath no segment
    // follows, which paints nothing — must not stretch the box to the
    // whole page. ~keep
    let mut extractor = PathExtractor::new();
    extractor.rectangle(0.0, 42.63, 595.28, -28.35);
    extractor.move_to(0.0, 842.0);
    extractor.fill(FillRule::EvenOdd);

    let paths = extractor.finish();
    assert_eq!(paths.len(), 1);

    let bbox = &paths[0].bbox;
    assert!(
        (bbox.y - 14.28).abs() < 1e-2,
        "expected the footer band's own y (14.28), got {}",
        bbox.y
    );
    assert!(
        (bbox.height - 28.35).abs() < 1e-2,
        "expected the footer band's own height (28.35), got {}",
        bbox.height
    );
}

#[test]
fn move_to_followed_by_a_segment_still_contributes_both_points() {
    // Control for GH#1759: a `MoveTo` that begins a real subpath is
    // painted and must keep contributing its point. ~keep
    let mut extractor = PathExtractor::new();
    extractor.move_to(10.0, 20.0);
    extractor.line_to(110.0, 120.0);
    extractor.stroke();

    let paths = extractor.finish();
    assert_eq!(paths.len(), 1);

    let bbox = &paths[0].bbox;
    assert_eq!(bbox.x, 10.0);
    assert_eq!(bbox.y, 20.0);
    assert_eq!(bbox.width, 100.0);
    assert_eq!(bbox.height, 100.0);
}
