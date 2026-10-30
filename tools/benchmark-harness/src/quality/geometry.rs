//! Class-aware bounding-box metrics; no text, polygon or reading-order score is inferred.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

// Match NumPy linspace operation order at binary threshold boundaries.
const IOU_THRESHOLDS: [f64; 10] = {
    let mut thresholds = [0.0; 10];
    let mut index = 0;
    while index < 9 {
        thresholds[index] = 0.50 + index as f64 * ((0.95 - 0.50) / 9.0);
        index += 1;
    }
    thresholds[9] = 0.95;
    thresholds
};
const RECALL_INTERVALS: usize = 100;
const AP_MAX_DETECTIONS: usize = 100;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Region {
    pub class_name: String,
    pub bbox: [f64; 4],
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Prediction {
    pub region: Region,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Page {
    pub image_id: u64,
    pub width: u32,
    pub height: u32,
    pub truth: Vec<Region>,
    pub predictions: Vec<Prediction>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ClassMetrics {
    pub ground_truth: usize,
    pub ap_truncated_predictions: usize,
    pub predictions: usize,
    pub true_positives: usize,
    pub false_positives: usize,
    pub false_negatives: usize,
    pub precision: Option<f64>,
    pub recall: Option<f64>,
    pub f1: Option<f64>,
    pub ap_50: Option<f64>,
    pub ap_50_95: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GeometryMetrics {
    pub classes: BTreeMap<String, ClassMetrics>,
    pub micro: ClassMetrics,
    pub mean_ap_50_95: Option<f64>,
    pub classes_with_ground_truth: usize,
    pub ap_max_detections_per_image_class: usize,
}

pub fn score(pages: &[Page], declared_classes: &BTreeSet<String>) -> Result<GeometryMetrics, String> {
    validate_pages(pages)?;
    let mut names = declared_classes.clone();
    for page in pages {
        names.extend(page.truth.iter().map(|region| region.class_name.clone()));
        names.extend(
            page.predictions
                .iter()
                .map(|prediction| prediction.region.class_name.clone()),
        );
    }
    let classes: BTreeMap<_, _> = names
        .into_iter()
        .map(|name| {
            let metrics = class_metrics(pages, &name);
            (name, metrics)
        })
        .collect();
    let known: Vec<_> = classes.values().filter_map(|metrics| metrics.ap_50_95).collect();
    Ok(GeometryMetrics {
        micro: micro_metrics(classes.values()),
        mean_ap_50_95: mean(&known),
        classes_with_ground_truth: known.len(),
        ap_max_detections_per_image_class: AP_MAX_DETECTIONS,
        classes,
    })
}

fn validate_pages(pages: &[Page]) -> Result<(), String> {
    if pages.is_empty() {
        return Err("geometry corpus is empty".into());
    }
    let mut seen = BTreeSet::new();
    for page in pages {
        if page.width == 0 || page.height == 0 || !seen.insert(page.image_id) {
            return Err("invalid or duplicate geometry page".into());
        }
        for region in &page.truth {
            validate_region(region, page.width, page.height)?;
        }
        for prediction in &page.predictions {
            validate_region(&prediction.region, page.width, page.height)?;
            if !prediction.confidence.is_finite() || !(0.0..=1.0).contains(&prediction.confidence) {
                return Err("invalid geometry confidence".into());
            }
        }
    }
    Ok(())
}

pub fn validate_region(region: &Region, width: u32, height: u32) -> Result<(), String> {
    let [x1, y1, x2, y2] = region.bbox;
    if region.class_name.is_empty()
        || !region.bbox.iter().all(|value| value.is_finite())
        || x1 < 0.0
        || y1 < 0.0
        || x2 <= x1
        || y2 <= y1
        || x2 > f64::from(width)
        || y2 > f64::from(height)
    {
        return Err("invalid or off-page geometry box".into());
    }
    Ok(())
}

fn intersection_over_union(a: &[f64; 4], b: &[f64; 4]) -> f64 {
    let intersection = (a[2].min(b[2]) - a[0].max(b[0])).max(0.0) * (a[3].min(b[3]) - a[1].max(b[1])).max(0.0);
    let area = |bbox: &[f64; 4]| (bbox[2] - bbox[0]) * (bbox[3] - bbox[1]);
    intersection / (area(a) + area(b) - intersection)
}

fn page_matches(page: &Page, class: &str, threshold: f64, maximum: usize) -> Vec<(f64, bool)> {
    let truth: Vec<_> = page.truth.iter().filter(|region| region.class_name == class).collect();
    let mut predictions: Vec<_> = page
        .predictions
        .iter()
        .filter(|prediction| prediction.region.class_name == class)
        .collect();
    predictions.sort_by(|a, b| {
        b.confidence
            .partial_cmp(&a.confidence)
            .expect("validated finite confidence")
    });
    let mut matched = vec![false; truth.len()];
    predictions
        .into_iter()
        .take(maximum)
        .map(|prediction| {
            let mut best = None;
            let mut best_overlap = threshold;
            for (index, region) in truth.iter().enumerate() {
                let overlap = intersection_over_union(&prediction.region.bbox, &region.bbox);
                if !matched[index] && overlap >= best_overlap {
                    best = Some(index);
                    best_overlap = overlap;
                }
            }
            if let Some(index) = best {
                matched[index] = true;
            }
            (prediction.confidence, best.is_some())
        })
        .collect()
}

fn matches(pages: &[Page], class: &str, threshold: f64, maximum: usize) -> Vec<(f64, bool)> {
    let mut pages: Vec<_> = pages.iter().collect();
    pages.sort_by_key(|page| page.image_id);
    let mut matches: Vec<_> = pages
        .into_iter()
        .flat_map(|page| page_matches(page, class, threshold, maximum))
        .collect();
    matches.sort_by(|a, b| b.0.partial_cmp(&a.0).expect("validated finite confidence"));
    matches
}

fn average_precision(matches: &[(f64, bool)], truth_count: usize) -> Option<f64> {
    if truth_count == 0 {
        return None;
    }
    let mut true_positives = 0usize;
    let points: Vec<_> = matches
        .iter()
        .enumerate()
        .map(|(index, (_, matched))| {
            true_positives += usize::from(*matched);
            (
                true_positives as f64 / truth_count as f64,
                true_positives as f64 / (index + 1) as f64,
            )
        })
        .collect();
    let precision: f64 = (0..=RECALL_INTERVALS)
        .map(|step| {
            let recall = step as f64 * (1.0 / RECALL_INTERVALS as f64);
            points
                .iter()
                .filter(|(achieved, _)| *achieved >= recall)
                .map(|(_, precision)| *precision)
                .fold(0.0, f64::max)
        })
        .sum();
    Some(precision / (RECALL_INTERVALS + 1) as f64)
}

fn class_metrics(pages: &[Page], class: &str) -> ClassMetrics {
    let truth = pages
        .iter()
        .map(|page| page.truth.iter().filter(|region| region.class_name == class).count())
        .sum();
    let observed = matches(pages, class, IOU_THRESHOLDS[0], usize::MAX);
    let true_positives = observed.iter().filter(|(_, matched)| *matched).count();
    let ap: Vec<_> = IOU_THRESHOLDS
        .iter()
        .filter_map(|threshold| average_precision(&matches(pages, class, *threshold, AP_MAX_DETECTIONS), truth))
        .collect();
    let truncated = observed.len() - matches(pages, class, IOU_THRESHOLDS[0], AP_MAX_DETECTIONS).len();
    let mut metrics = counts(truth, observed.len(), true_positives, truncated);
    metrics.ap_50 = ap.first().copied();
    metrics.ap_50_95 = mean(&ap);
    metrics
}

fn ratio(numerator: usize, denominator: usize) -> Option<f64> {
    (denominator != 0).then(|| numerator as f64 / denominator as f64)
}

fn mean(values: &[f64]) -> Option<f64> {
    (!values.is_empty()).then(|| values.iter().sum::<f64>() / values.len() as f64)
}

fn counts(truth: usize, predictions: usize, true_positives: usize, truncated: usize) -> ClassMetrics {
    let false_positives = predictions - true_positives;
    let false_negatives = truth - true_positives;
    ClassMetrics {
        ground_truth: truth,
        predictions,
        true_positives,
        false_positives,
        false_negatives,
        ap_truncated_predictions: truncated,
        precision: ratio(true_positives, predictions),
        recall: ratio(true_positives, truth),
        f1: ratio(
            2 * true_positives,
            2 * true_positives + false_positives + false_negatives,
        ),
        ap_50: None,
        ap_50_95: None,
    }
}

fn micro_metrics<'a>(classes: impl Iterator<Item = &'a ClassMetrics>) -> ClassMetrics {
    let (mut truth, mut predictions, mut positives, mut truncated) = (0, 0, 0, 0);
    for metrics in classes {
        truth += metrics.ground_truth;
        predictions += metrics.predictions;
        positives += metrics.true_positives;
        truncated += metrics.ap_truncated_predictions;
    }
    counts(truth, predictions, positives, truncated)
}

#[cfg(test)]
#[path = "geometry/tests.rs"]
mod tests;
