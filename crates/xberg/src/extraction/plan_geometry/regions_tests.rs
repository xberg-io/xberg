//! Closed-region recovery tests.
//!
//! Kept beside `regions.rs` so the module stays inside the file-size limit.

use super::*;
use crate::types::{VectorPath, VectorStroke, VectorSubpath};

fn point(x: f32, y: f32) -> VectorPoint {
    VectorPoint { x, y }
}

fn path_from_subpath(subpath: VectorSubpath) -> VectorPath {
    let mut bounds = BoundingBox {
        x0: f64::INFINITY,
        y0: f64::INFINITY,
        x1: f64::NEG_INFINITY,
        y1: f64::NEG_INFINITY,
    };
    let mut update = |point: VectorPoint| {
        bounds.x0 = bounds.x0.min(f64::from(point.x));
        bounds.y0 = bounds.y0.min(f64::from(point.y));
        bounds.x1 = bounds.x1.max(f64::from(point.x));
        bounds.y1 = bounds.y1.max(f64::from(point.y));
    };
    update(subpath.start);
    for segment in &subpath.segments {
        if let VectorSegment::Line { to } = segment {
            update(*to);
        }
    }
    VectorPath {
        subpaths: vec![subpath],
        bounding_box: bounds,
        fill_color: None,
        stroke: Some(VectorStroke::default()),
        layer: None,
    }
}

fn line(from: VectorPoint, to: VectorPoint) -> VectorPath {
    path_from_subpath(VectorSubpath {
        start: from,
        segments: vec![VectorSegment::Line { to }],
        closed: false,
    })
}

fn has_edge(graph: &Graph, from: VectorPoint, to: VectorPoint) -> bool {
    graph.edges.iter().any(|&(a, b)| {
        let (left, right) = (graph.coords[a], graph.coords[b]);
        (left == from && right == to) || (left == to && right == from)
    })
}

fn rectangle(x0: f32, y0: f32, x1: f32, y1: f32) -> VectorPath {
    path_from_subpath(VectorSubpath {
        start: point(x0, y0),
        segments: vec![
            VectorSegment::Line { to: point(x1, y0) },
            VectorSegment::Line { to: point(x1, y1) },
            VectorSegment::Line { to: point(x0, y1) },
        ],
        closed: true,
    })
}

fn config(snap: f32, gap: f32) -> PlanRegionConfig {
    PlanRegionConfig {
        enabled: true,
        snap_tolerance: snap,
        gap_tolerance: gap,
        max_segments: 20_000,
    }
}

fn span(text: &str, x: f32, y: f32, width: f32, height: f32) -> xberg_native_pdf::layout::TextSpan {
    xberg_native_pdf::layout::TextSpan {
        provenance: None,
        artifact_type: None,
        text: text.to_string(),
        bbox: xberg_native_pdf::geometry::Rect::new(x, y, width, height),
        font_name: Default::default(),
        font_size: 12.0,
        font_weight: xberg_native_pdf::layout::FontWeight::Normal,
        is_italic: false,
        is_monospace: false,
        color: xberg_native_pdf::layout::Color { r: 0.0, g: 0.0, b: 0.0 },
        mcid: None,
        mcid_scope: None,
        sequence: 0,
        split_boundary_before: false,
        offset_semantic: false,
        char_spacing: 0.0,
        word_spacing: 0.0,
        horizontal_scaling: 100.0,
        primary_detected: false,
        char_widths: vec![],
        char_x_offsets: Vec::new(),
        heading_level: None,
        rotation_degrees: 0.0,
        wmode: 0,
        text_rise: 0.0,
        rtl_draw_logical: false,
        ligature_glyph: false,
        mirrored: false,
        page_rotation_applied: 0,
    }
}

#[test]
fn a_closed_rectangle_becomes_one_face_with_its_text() {
    let paths = vec![rectangle(0.0, 0.0, 100.0, 50.0)];
    let spans = vec![span("ROOM 101", 40.0, 20.0, 20.0, 10.0)];
    let regions = close_regions(&paths, &spans, &config(0.5, 0.0));
    assert_eq!(regions.len(), 1);
    assert_eq!(regions[0].area, 5000.0);
    assert_eq!(regions[0].text, "ROOM 101");
    assert_eq!(
        regions[0].bounding_box,
        BoundingBox {
            x0: 0.0,
            y0: 0.0,
            x1: 100.0,
            y1: 50.0,
        }
    );
}

#[test]
fn text_outside_a_face_is_not_attached() {
    let paths = vec![rectangle(0.0, 0.0, 100.0, 50.0)];
    let spans = vec![span("OUTSIDE", 140.0, 20.0, 20.0, 10.0)];
    let regions = close_regions(&paths, &spans, &config(0.5, 0.0));
    assert_eq!(regions.len(), 1);
    assert_eq!(regions[0].text, "");
}

#[test]
fn a_door_sized_gap_is_closed_within_the_tolerance() {
    // The bottom wall is split by a 10-point door opening: (0,0)->(45,0)
    // and (55,0)->(100,0), with the other three walls closed.
    let mut paths = vec![
        path_from_subpath(VectorSubpath {
            start: point(0.0, 0.0),
            segments: vec![VectorSegment::Line { to: point(45.0, 0.0) }],
            closed: false,
        }),
        path_from_subpath(VectorSubpath {
            start: point(55.0, 0.0),
            segments: vec![VectorSegment::Line { to: point(100.0, 0.0) }],
            closed: false,
        }),
        path_from_subpath(VectorSubpath {
            start: point(0.0, 0.0),
            segments: vec![
                VectorSegment::Line { to: point(0.0, 50.0) },
                VectorSegment::Line { to: point(100.0, 50.0) },
                VectorSegment::Line { to: point(100.0, 0.0) },
            ],
            closed: false,
        }),
    ];
    paths.push(path_from_subpath(VectorSubpath {
        start: point(100.0, 0.0),
        segments: vec![VectorSegment::Line { to: point(55.0, 0.0) }],
        closed: false,
    }));

    let closed = close_regions(&paths, &[], &config(0.5, 20.0));
    assert_eq!(closed.len(), 1, "the 10-point gap must close at tolerance 20");
    assert!((closed[0].area - 5000.0).abs() < 1.0);

    let open = close_regions(&paths, &[], &config(0.5, 5.0));
    assert!(open.is_empty(), "a gap beyond the tolerance must stay open");
}

#[test]
fn crossing_rules_form_a_single_cell() {
    // Each rule extends past the crossing, so the face only exists when the
    // crossing pass splits both rules at their mid-span intersection.
    let horizontal_top = path_from_subpath(VectorSubpath {
        start: point(-10.0, 100.0),
        segments: vec![VectorSegment::Line {
            to: point(110.0, 100.0),
        }],
        closed: false,
    });
    let horizontal_bottom = path_from_subpath(VectorSubpath {
        start: point(-10.0, 0.0),
        segments: vec![VectorSegment::Line { to: point(110.0, 0.0) }],
        closed: false,
    });
    let vertical_left = path_from_subpath(VectorSubpath {
        start: point(0.0, -10.0),
        segments: vec![VectorSegment::Line { to: point(0.0, 110.0) }],
        closed: false,
    });
    let vertical_right = path_from_subpath(VectorSubpath {
        start: point(100.0, -10.0),
        segments: vec![VectorSegment::Line {
            to: point(100.0, 110.0),
        }],
        closed: false,
    });

    let regions = close_regions(
        &[horizontal_top, horizontal_bottom, vertical_left, vertical_right],
        &[],
        &config(0.5, 0.0),
    );
    assert_eq!(regions.len(), 1);
    assert!((regions[0].area - 10_000.0).abs() < 1.0);
}

#[test]
fn an_open_cross_has_no_face() {
    // A plus sign: one horizontal and one vertical rule crossing mid-span.
    let horizontal = path_from_subpath(VectorSubpath {
        start: point(0.0, 50.0),
        segments: vec![VectorSegment::Line { to: point(100.0, 50.0) }],
        closed: false,
    });
    let vertical = path_from_subpath(VectorSubpath {
        start: point(50.0, 0.0),
        segments: vec![VectorSegment::Line { to: point(50.0, 100.0) }],
        closed: false,
    });
    let regions = close_regions(&[horizontal, vertical], &[], &config(0.5, 0.0));
    assert!(regions.is_empty(), "an open cross has no closed face");
}

#[test]
fn nested_rectangles_are_ordered_by_area() {
    let paths = vec![rectangle(0.0, 0.0, 100.0, 100.0), rectangle(40.0, 40.0, 60.0, 60.0)];
    let regions = close_regions(&paths, &[], &config(0.5, 0.0));
    assert_eq!(regions.len(), 2);
    assert!((regions[0].area - 10_000.0).abs() < 1.0);
    assert!((regions[1].area - 400.0).abs() < 1.0);
}

#[test]
fn the_result_is_deterministic_for_reversed_input() {
    let forward = vec![rectangle(0.0, 0.0, 100.0, 100.0), rectangle(200.0, 0.0, 260.0, 60.0)];
    let mut reversed = forward.clone();
    reversed.reverse();
    let first = close_regions(&forward, &[], &config(0.5, 0.0));
    let second = close_regions(&reversed, &[], &config(0.5, 0.0));
    assert_eq!(first, second);
}

#[test]
fn degeneracies_do_not_panic() {
    let zero_length = path_from_subpath(VectorSubpath {
        start: point(5.0, 5.0),
        segments: vec![VectorSegment::Line { to: point(5.0, 5.0) }],
        closed: false,
    });
    let collinear = vec![
        path_from_subpath(VectorSubpath {
            start: point(0.0, 0.0),
            segments: vec![VectorSegment::Line { to: point(10.0, 0.0) }],
            closed: false,
        }),
        path_from_subpath(VectorSubpath {
            start: point(10.0, 0.0),
            segments: vec![VectorSegment::Line { to: point(20.0, 0.0) }],
            closed: false,
        }),
    ];
    assert!(close_regions(&[zero_length], &[], &config(0.5, 0.0)).is_empty());
    assert!(close_regions(&collinear, &[], &config(0.5, 0.0)).is_empty());
}

#[test]
fn a_page_over_the_segment_limit_is_skipped() {
    let paths: Vec<VectorPath> = (0..10)
        .map(|index| rectangle(index as f32 * 10.0, 0.0, index as f32 * 10.0 + 5.0, 5.0))
        .collect();
    let mut settings = config(0.5, 0.0);
    settings.max_segments = 5;
    assert!(close_regions(&paths, &[], &settings).is_empty());
}

#[test]
fn a_crossing_near_an_endpoint_keeps_the_whole_wall() {
    // The diagonal crosses the wall at x = 0.125, inside the first 0.5-wide
    // cell, so snapping moves the split point onto the wall's start: one piece
    // is degenerate and the other is the original edge. Removing the original
    // after re-adding it would erase the wall.
    let mut graph = Graph::new(0.5);
    graph.add_segment(point(0.0, 0.0), point(100.0, 0.0));
    graph.add_segment(point(0.0, 0.5), point(0.5, -1.5));
    let _ = graph.split_crossings();
    assert!(
        has_edge(&graph, point(0.0, 0.0), point(100.0, 0.0)),
        "a near-endpoint crossing must not delete the wall"
    );
}

#[test]
fn endpoints_within_the_snap_distance_merge_across_a_cell_boundary() {
    // 0.24 and 0.26 straddle the 0.25 boundary between two 0.5-wide cells.
    let mut graph = Graph::new(0.5);
    let left = graph.node(point(0.24, 0.0));
    let right = graph.node(point(0.26, 0.0));
    assert_eq!(left, right, "ends 0.02 pt apart must merge even across a cell edge");

    let far = graph.node(point(1.2, 0.0));
    assert_ne!(far, left, "ends beyond the snap tolerance must stay apart");
}

#[test]
fn a_sheet_of_disjoint_segments_stops_at_the_pair_limit() {
    // 1200 stacked rules share an x span, so the sweep examines 719,400 pairs
    // even though no two rules touch: more than the 500,000 budget.
    let mut graph = Graph::new(0.5);
    for index in 0..1_200 {
        let y = index as f32 * 10.0;
        graph.add_segment(point(0.0, y), point(100.0, y));
    }
    let before = graph.edges.len();
    assert!(graph.split_crossings(), "the pair-test budget must be exhausted");
    assert_eq!(graph.edges.len(), before, "disjoint rules cannot be split");
}

#[test]
fn a_dense_hatch_stops_collecting_gap_candidates() {
    // 400 strokes in a 4-by-6 point patch: all 800 ends lie within the
    // 24-point gap of each other, so an unbounded pass would hold 319,600
    // candidate pairs.
    let mut graph = Graph::new(0.0);
    for index in 0..400 {
        let x = index as f32 * 0.01;
        graph.add_segment(point(x, 0.0), point(x, 6.0));
    }
    let before = graph.edges.len();
    assert!(
        graph.close_gaps(24.0),
        "the candidate budget must fire on a dense hatch"
    );
    let added = graph.edges.len() - before;
    assert!(added > 0, "the pass still closes some gaps");
    assert!(added <= 400, "only greedy bridges are added, got {added}");
}

#[test]
fn a_bridge_that_crosses_a_wall_is_split_before_the_face_walk() {
    // A 10-point door opening in the bottom wall and a wall crossing the
    // bridge at x = 50. Without re-splitting after the bridge is added the
    // crossing has no node and the two halves are never separated.
    let paths = vec![
        line(point(0.0, 0.0), point(45.0, 0.0)),
        line(point(55.0, 0.0), point(100.0, 0.0)),
        path_from_subpath(VectorSubpath {
            start: point(0.0, 0.0),
            segments: vec![
                VectorSegment::Line { to: point(0.0, 50.0) },
                VectorSegment::Line { to: point(100.0, 50.0) },
                VectorSegment::Line { to: point(100.0, 0.0) },
            ],
            closed: false,
        }),
        line(point(100.0, 0.0), point(55.0, 0.0)),
        line(point(50.0, -10.0), point(50.0, 60.0)),
    ];

    let regions = close_regions(&paths, &[], &config(0.5, 10.5));
    assert_eq!(regions.len(), 2, "the crossing wall must split the closed room");
    for region in &regions {
        assert!(
            (region.area - 2_500.0).abs() < 1.0,
            "each half is 50 by 50, got {}",
            region.area
        );
    }
}

#[test]
fn the_text_pass_is_skipped_when_there_are_no_segments_to_close() {
    let mut paths = Vec::new();
    let mut read = false;
    let regions = close_page_regions(&mut paths, &config(0.5, 20.0), false, || {
        read = true;
        Ok(Vec::new())
    });
    assert!(regions.is_empty());
    assert!(!read, "a page without geometry must not pay for a second text pass");
}

#[test]
fn a_page_over_the_segment_limit_skips_the_text_pass_and_drops_paths() {
    let mut paths = vec![rectangle(0.0, 0.0, 10.0, 10.0)];
    let mut settings = config(0.5, 0.0);
    settings.max_segments = 1;
    let mut read = false;
    let regions = close_page_regions(&mut paths, &settings, false, || {
        read = true;
        Ok(Vec::new())
    });
    assert!(regions.is_empty());
    assert!(!read);
    assert!(
        paths.is_empty(),
        "unrequested paths are dropped when the page is skipped"
    );
}

#[test]
fn an_unreadable_text_pass_still_honours_the_keep_paths_contract() {
    let mut dropped = vec![rectangle(0.0, 0.0, 10.0, 10.0)];
    let regions = close_page_regions(&mut dropped, &config(0.5, 0.0), false, || {
        Err(xberg_native_pdf::Error::InvalidPdf("no text".to_string()))
    });
    assert!(regions.is_empty());
    assert!(dropped.is_empty(), "the error path must drop unrequested paths");

    let mut kept = vec![rectangle(0.0, 0.0, 10.0, 10.0)];
    let regions = close_page_regions(&mut kept, &config(0.5, 0.0), true, || {
        Err(xberg_native_pdf::Error::InvalidPdf("no text".to_string()))
    });
    assert!(regions.is_empty());
    assert_eq!(kept.len(), 1, "paths the caller asked for survive the error path");
}
