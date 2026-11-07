use super::formula_regions_in_reading_order;
use crate::layout::{BBox, DetectionResult, LayoutClass, LayoutDetection};

fn det(class: LayoutClass, x1: f32, y1: f32) -> LayoutDetection {
    LayoutDetection {
        class_name: class,
        confidence: 0.9,
        bbox: BBox {
            x1,
            y1,
            x2: x1 + 10.0,
            y2: y1 + 10.0,
        },
    }
}

#[test]
fn orders_formulas_top_to_bottom_then_left_to_right() {
    let page = DetectionResult {
        page_width: 100,
        page_height: 100,
        detections: vec![
            det(LayoutClass::Formula, 50.0, 40.0),
            det(LayoutClass::Text, 0.0, 0.0),
            det(LayoutClass::Formula, 10.0, 40.0),
            det(LayoutClass::Formula, 10.0, 5.0),
        ],
    };
    let ordered = formula_regions_in_reading_order(&page);
    let coords: Vec<(f32, f32)> = ordered.iter().map(|d| (d.bbox.y1, d.bbox.x1)).collect();
    assert_eq!(coords, vec![(5.0, 10.0), (40.0, 10.0), (40.0, 50.0)]);
}

#[test]
fn pages_without_formulas_yield_no_regions() {
    let page = DetectionResult {
        page_width: 100,
        page_height: 100,
        detections: vec![det(LayoutClass::Text, 0.0, 0.0), det(LayoutClass::Table, 5.0, 5.0)],
    };
    assert!(formula_regions_in_reading_order(&page).is_empty());
}

fn pt_box(x0: f64, y0: f64, x1: f64, y1: f64) -> crate::types::BoundingBox {
    crate::types::BoundingBox { x0, y0, x1, y1 }
}

#[test]
fn matching_counts_without_geometry_pair_positionally() {
    let assigned = super::assign_formula_slots(2, &[None, None], None);
    assert_eq!(assigned, vec![Some(0), Some(1)]);
}

#[test]
fn matching_counts_without_slot_bboxes_pair_positionally() {
    let regions = [pt_box(10.0, 100.0, 200.0, 130.0), pt_box(10.0, 10.0, 200.0, 40.0)];
    let assigned = super::assign_formula_slots(2, &[None, None], Some(&regions));
    assert_eq!(assigned, vec![Some(0), Some(1)]);
}

#[test]
fn mismatched_counts_pair_by_overlap_and_leave_the_rest_unassigned() {
    // Three regions, two slots; slots overlap regions 0 and 2.
    let regions = [
        pt_box(10.0, 100.0, 200.0, 130.0),
        pt_box(10.0, 50.0, 200.0, 80.0),
        pt_box(10.0, 10.0, 200.0, 40.0),
    ];
    let slots = [
        Some(pt_box(12.0, 12.0, 190.0, 38.0)),
        Some(pt_box(12.0, 102.0, 190.0, 128.0)),
    ];
    let assigned = super::assign_formula_slots(3, &slots, Some(&regions));
    assert_eq!(assigned, vec![Some(1), None, Some(0)]);
}

#[test]
fn matching_counts_pair_by_geometry_not_by_position() {
    // Two formulas on one line; the right one sits slightly higher, so it sorts first.
    let regions = [pt_box(180.5, 339.8, 190.0, 351.0), pt_box(119.5, 340.0, 150.0, 350.5)];
    let slots = [
        Some(pt_box(120.2, 340.2, 149.4, 350.2)),
        Some(pt_box(181.6, 340.2, 188.7, 350.2)),
    ];
    let assigned = super::assign_formula_slots(2, &slots, Some(&regions));
    assert_eq!(assigned, vec![Some(1), Some(0)]);
}

#[test]
fn matching_counts_pair_positionally_when_a_region_overlaps_nothing() {
    // Region 0 overlaps slot 1; region 1 overlaps neither a missing box nor a distant one.
    let regions = [pt_box(10.0, 100.0, 200.0, 130.0), pt_box(10.0, 10.0, 200.0, 40.0)];
    let near = Some(pt_box(12.0, 102.0, 190.0, 128.0));
    for slots in [[None, near], [Some(pt_box(300.0, 300.0, 400.0, 330.0)), near]] {
        let assigned = super::assign_formula_slots(2, &slots, Some(&regions));
        assert_eq!(assigned, vec![Some(0), Some(1)]);
    }
}

#[test]
fn best_overlap_pairs_first_regardless_of_region_order() {
    // Region 0 overlaps slot 0 by 0.8 and slot 1 by 0.6; region 1 covers slot 0 exactly.
    let regions = [pt_box(2.0, 0.0, 16.0, 10.0), pt_box(0.0, 0.0, 10.0, 10.0)];
    let slots = [Some(pt_box(0.0, 0.0, 10.0, 10.0)), Some(pt_box(10.0, 0.0, 20.0, 10.0))];
    let assigned = super::assign_formula_slots(2, &slots, Some(&regions));
    assert_eq!(assigned, vec![Some(1), Some(0)]);
}

#[test]
fn matching_counts_find_a_complete_geometric_assignment_before_falling_back() {
    let regions = [pt_box(10.0, 0.0, 25.0, 10.0), pt_box(5.0, 0.0, 15.0, 10.0)];
    let slots = [Some(pt_box(10.0, 0.0, 20.0, 10.0)), Some(pt_box(20.0, 0.0, 30.0, 10.0))];
    let assigned = super::assign_formula_slots(2, &slots, Some(&regions));
    assert_eq!(assigned, vec![Some(1), Some(0)]);
}

#[test]
fn matching_counts_complete_a_chain_of_contested_slots() {
    let regions = [
        pt_box(10.0, 0.0, 25.0, 10.0),
        pt_box(20.0, 0.0, 35.0, 10.0),
        pt_box(5.0, 0.0, 15.0, 10.0),
    ];
    let slots = [
        Some(pt_box(10.0, 0.0, 20.0, 10.0)),
        Some(pt_box(20.0, 0.0, 30.0, 10.0)),
        Some(pt_box(30.0, 0.0, 40.0, 10.0)),
    ];
    let assigned = super::assign_formula_slots(3, &slots, Some(&regions));
    assert_eq!(assigned, vec![Some(1), Some(2), Some(0)]);
}

#[test]
fn mismatched_counts_give_a_contested_slot_to_the_best_overlap() {
    // Region 0 overlaps the slot by 0.6 (IoU 0.43); region 1 contains it (IoU 0.25).
    let regions = [pt_box(4.0, 0.0, 14.0, 10.0), pt_box(0.0, 0.0, 40.0, 10.0)];
    let slots = [Some(pt_box(0.0, 0.0, 10.0, 10.0))];
    let assigned = super::assign_formula_slots(2, &slots, Some(&regions));
    assert_eq!(assigned, vec![None, Some(0)]);
}

#[test]
fn a_region_keeps_the_slot_it_overlaps_most() {
    let regions = [pt_box(0.0, 0.0, 10.0, 10.0)];
    let slots = [Some(pt_box(0.0, 0.0, 10.0, 10.0)), Some(pt_box(3.0, 0.0, 13.0, 10.0))];
    let assigned = super::assign_formula_slots(1, &slots, Some(&regions));
    assert_eq!(assigned, vec![Some(0)]);
}

#[test]
fn nested_boxes_tied_on_overlap_pair_by_iou() {
    // Each region contains or sits inside both slots, so every overlap is 1.0.
    let regions = [pt_box(0.0, 0.0, 100.0, 20.0), pt_box(10.0, 5.0, 20.0, 15.0)];
    let slots = [Some(pt_box(10.0, 5.0, 20.0, 15.0)), Some(pt_box(0.0, 0.0, 98.0, 20.0))];
    let assigned = super::assign_formula_slots(2, &slots, Some(&regions));
    assert_eq!(assigned, vec![Some(1), Some(0)]);
}

#[test]
fn mismatched_counts_without_geometry_assign_nothing() {
    let slots = [Some(pt_box(0.0, 0.0, 10.0, 10.0))];
    let assigned = super::assign_formula_slots(3, &slots, None);
    assert_eq!(assigned, vec![None, None, None]);
}

#[test]
fn slots_without_bboxes_stay_unassigned_on_mismatch() {
    let regions = [pt_box(0.0, 0.0, 100.0, 30.0)];
    let assigned = super::assign_formula_slots(1, &[None, None], Some(&regions));
    assert_eq!(assigned, vec![None]);
}

#[test]
fn synthesis_needs_geometry_when_the_page_carries_formula_text() {
    // No geometry, no text: a scanned page OCR'd to plain text synthesizes.
    assert!(super::formula_synthesis_allowed(false, 0, 0));
    // No geometry, text present: synthesis would duplicate it.
    assert!(!super::formula_synthesis_allowed(false, 1, 0));
    assert!(!super::formula_synthesis_allowed(false, 0, 2));
    // With geometry, unpaired regions synthesize regardless of text.
    assert!(super::formula_synthesis_allowed(true, 3, 1));
}

#[test]
fn overlap_ratio_is_intersection_over_smaller_area() {
    let big = pt_box(0.0, 0.0, 100.0, 100.0);
    let small = pt_box(50.0, 50.0, 150.0, 150.0);
    assert!((super::bbox_overlap_ratio(&big, &small) - 0.25).abs() < 1e-9);
    let contained = pt_box(10.0, 10.0, 20.0, 20.0);
    assert!((super::bbox_overlap_ratio(&big, &contained) - 1.0).abs() < 1e-9);
    assert_eq!(
        super::bbox_overlap_ratio(&big, &pt_box(200.0, 200.0, 300.0, 300.0)),
        0.0
    );
}
