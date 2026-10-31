use super::*;

fn region(class: &str, bbox: [f64; 4]) -> Region {
    Region {
        class_name: class.into(),
        bbox,
    }
}

fn prediction(class: &str, bbox: [f64; 4], confidence: f64) -> Prediction {
    Prediction {
        region: region(class, bbox),
        confidence,
    }
}

fn page(predictions: Vec<Prediction>) -> Page {
    Page {
        image_id: 1,
        width: 100,
        height: 100,
        truth: vec![region("text", [0., 0., 10., 10.])],
        predictions,
    }
}

#[test]
fn perfect_boxes_score_one_at_all_ten_thresholds() {
    let metrics = score(
        &[page(vec![prediction("text", [0., 0., 10., 10.], 0.9)])],
        &BTreeSet::from(["text".into(), "table".into()]),
    )
    .unwrap();
    assert_eq!(metrics.mean_ap_50_95, Some(1.0));
    assert_eq!(metrics.micro.true_positives, 1);
    assert_eq!(metrics.classes["text"].f1, Some(1.0));
    assert_eq!(metrics.classes["table"].ap_50_95, None);
    assert_eq!(metrics.classes_with_ground_truth, 1);
}

#[test]
fn high_confidence_false_positive_reduces_interpolated_ap() {
    let metrics = score(
        &[page(vec![
            prediction("text", [50., 50., 60., 60.], 0.9),
            prediction("text", [0., 0., 10., 10.], 0.8),
        ])],
        &BTreeSet::new(),
    )
    .unwrap();
    assert_eq!(metrics.mean_ap_50_95, Some(0.5));
    assert_eq!(metrics.micro.precision, Some(0.5));
    assert_eq!(metrics.micro.recall, Some(1.0));
    assert_eq!(metrics.micro.f1, Some(2.0 / 3.0));
}

#[test]
fn duplicate_after_a_true_positive_is_false_positive_without_reducing_ap() {
    let metrics = score(
        &[page(vec![
            prediction("text", [0., 0., 10., 10.], 0.9),
            prediction("text", [0., 0., 10., 10.], 0.8),
        ])],
        &BTreeSet::new(),
    )
    .unwrap();
    assert_eq!(metrics.micro.false_positives, 1);
    assert_eq!(metrics.mean_ap_50_95, Some(1.0));
}

#[test]
fn class_mismatch_is_not_a_geometric_match() {
    let metrics = score(
        &[page(vec![prediction("table", [0., 0., 10., 10.], 0.9)])],
        &BTreeSet::new(),
    )
    .unwrap();
    assert_eq!(metrics.micro.true_positives, 0);
    assert_eq!(metrics.micro.false_positives, 1);
    assert_eq!(metrics.micro.false_negatives, 1);
    assert_eq!(metrics.mean_ap_50_95, Some(0.0));
    assert_eq!(metrics.classes["table"].ap_50_95, None);
}

#[test]
fn explicit_blank_page_has_unknown_ap_and_false_positives_remain_visible() {
    let page = Page {
        truth: Vec::new(),
        ..page(vec![prediction("text", [0., 0., 10., 10.], 0.9)])
    };
    let metrics = score(&[page], &BTreeSet::new()).unwrap();
    assert_eq!(metrics.mean_ap_50_95, None);
    assert_eq!(metrics.micro.false_positives, 1);
    assert_eq!(metrics.micro.f1, Some(0.0));
}

#[test]
fn missing_predictions_are_false_negatives_and_empty_corpus_is_rejected() {
    let metrics = score(&[page(Vec::new())], &BTreeSet::new()).unwrap();
    assert_eq!(metrics.micro.false_negatives, 1);
    assert_eq!(metrics.mean_ap_50_95, Some(0.0));
    assert_eq!(score(&[], &BTreeSet::new()).unwrap_err(), "geometry corpus is empty");
}

#[test]
fn nonfinite_off_page_and_invalid_confidence_inputs_fail_instead_of_disappearing() {
    for (bbox, confidence) in [
        ([0., 0., 101., 10.], 0.9),
        ([0., 0., f64::NAN, 10.], 0.9),
        ([0., 0., 10., 10.], 2.0),
    ] {
        assert!(score(&[page(vec![prediction("text", bbox, confidence)])], &BTreeSet::new()).is_err());
    }
}

#[test]
fn ap_cap_is_reported_without_hiding_predictions_from_precision() {
    let mut predictions: Vec<_> = (0..100)
        .map(|index| prediction("text", [50., 50., 60., 60.], 1.0 - f64::from(index) * 0.001))
        .collect();
    predictions.push(prediction("text", [0., 0., 10., 10.], 0.5));
    let metrics = score(&[page(predictions)], &BTreeSet::new()).unwrap();
    assert_eq!(metrics.mean_ap_50_95, Some(0.0));
    assert_eq!(metrics.micro.true_positives, 1);
    assert_eq!(metrics.micro.predictions, 101);
    assert_eq!(metrics.micro.ap_truncated_predictions, 1);
}

#[test]
fn cross_image_score_ties_follow_original_coco_image_ids_not_input_order() {
    let false_positive = Page {
        image_id: 2,
        ..page(vec![prediction("text", [50., 50., 60., 60.], 0.9)])
    };
    let true_positive = page(vec![prediction("text", [0., 0., 10., 10.], 0.9)]);
    let a = score(&[false_positive.clone(), true_positive.clone()], &BTreeSet::new()).unwrap();
    let b = score(&[true_positive, false_positive], &BTreeSet::new()).unwrap();
    assert_eq!(a, b);
    assert_eq!(a.classes["text"].true_positives, 1);
}

#[test]
fn scores_roundtrip_through_json_without_changing_the_metric_values() {
    let metrics = score(
        &[page(vec![
            prediction("text", [0., 0., 10., 10.], 0.9),
            prediction("text", [20., 20., 30., 30.], 0.8),
        ])],
        &BTreeSet::new(),
    )
    .unwrap();
    let decoded: GeometryMetrics = serde_json::from_slice(&serde_json::to_vec(&metrics).unwrap()).unwrap();
    assert_eq!(metrics, decoded);
}

#[test]
fn signed_zero_scores_are_stable_ties_instead_of_a_confidence_order() {
    let metrics = score(
        &[page(vec![
            prediction("text", [50., 50., 60., 60.], -0.0),
            prediction("text", [0., 0., 10., 10.], 0.0),
        ])],
        &BTreeSet::new(),
    )
    .unwrap();
    assert_eq!(metrics.mean_ap_50_95, Some(0.5));
}

#[test]
fn recall_boundary_seven_of_twenty_matches_official_grid() {
    let matched = vec![(1.0, true); 7];
    assert_eq!(average_precision(&matched, 20), Some(35.0 / 101.0));
}

#[test]
fn all_recall_boundaries_use_multiply_then_compare() {
    for count in 1..=100 {
        let matched = vec![(1.0, true); count];
        let achieved = count as f64 / 100.0;
        let points = (0..=100).filter(|step| *step as f64 * 0.01 <= achieved).count();
        assert_eq!(average_precision(&matched, 100), Some(points as f64 / 101.0), "{count}");
    }
}

#[test]
fn iou_ninety_percent_uses_official_linspace_binary_boundary() {
    let threshold = IOU_THRESHOLDS[8];
    assert_eq!(threshold.to_bits(), (0.50_f64 + 8.0 * ((0.95 - 0.50) / 9.0)).to_bits());
    let boundary = Page {
        truth: vec![region("text", [0., 0., 1., 1.])],
        ..page(vec![prediction("text", [0., 0., 0.8999999999999999, 1.], 0.9)])
    };
    assert!(page_matches(&boundary, "text", threshold, 100)[0].1);
}
