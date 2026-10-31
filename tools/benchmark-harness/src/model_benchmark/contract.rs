use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use xberg::layout::{DetectionResult, LayoutDetection};

use super::{Detector, ModelBenchmarkConfig};
use crate::{Error, Result};

const REPORT_SCHEMA_VERSION: u32 = 2;
pub(super) const SCORED_REPORT_SCHEMA_VERSION: u32 = 3;
const MODEL_ARMS: usize = 2;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelIdentity {
    pub detector: Detector,
    pub sha256: String,
    pub artifact_bytes: u64,
    pub execution_provider: String,
    pub intra_op_threads: usize,
    pub inter_op_threads: usize,
    pub load_ms: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExpectedDocument {
    pub pages: usize,
    pub document_sha256: String,
    pub fixture_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DetectorMeasurement {
    pub inference_performed: bool,
    pub elapsed_ms: f64,
    pub page_width: u32,
    pub page_height: u32,
    pub canonical_class_counts: BTreeMap<String, usize>,
    pub detections: Vec<LayoutDetection>,
}

impl DetectorMeasurement {
    pub(super) fn from_detection(result: DetectionResult, elapsed_ms: f64) -> Result<Self> {
        let mut canonical_class_counts = BTreeMap::new();
        for detection in &result.detections {
            let class = serde_json::to_value(detection.class_name)?;
            let name = class
                .as_str()
                .ok_or_else(|| Error::Benchmark("invalid canonical class".to_string()))?;
            *canonical_class_counts.entry(name.to_string()).or_default() += 1;
        }
        let measurement = Self {
            inference_performed: true,
            elapsed_ms,
            page_width: result.page_width,
            page_height: result.page_height,
            canonical_class_counts,
            detections: result.detections,
        };
        measurement.validate()?;
        Ok(measurement)
    }

    fn validate(&self) -> Result<()> {
        if !self.inference_performed {
            return Err(Error::Benchmark("detector inference was skipped".to_string()));
        }
        if self.page_width == 0 || self.page_height == 0 || !valid_time(self.elapsed_ms) {
            return Err(Error::Benchmark("invalid detector dimensions or timing".to_string()));
        }
        let mut counts = BTreeMap::new();
        for detection in &self.detections {
            validate_detection(detection, self.page_width, self.page_height)?;
            let class = serde_json::to_value(detection.class_name)?;
            let name = class
                .as_str()
                .ok_or_else(|| Error::Benchmark("invalid canonical class".to_string()))?;
            *counts.entry(name.to_string()).or_insert(0usize) += 1;
        }
        if counts != self.canonical_class_counts {
            return Err(Error::Benchmark(
                "canonical class counts do not match actual detections".to_string(),
            ));
        }
        Ok(())
    }
}

fn validate_detection(detection: &LayoutDetection, width: u32, height: u32) -> Result<()> {
    let bbox = detection.bbox;
    let valid = detection.confidence.is_finite()
        && (0.0..=1.0).contains(&detection.confidence)
        && [bbox.x1, bbox.y1, bbox.x2, bbox.y2]
            .iter()
            .all(|value| value.is_finite())
        && bbox.x1 >= 0.0
        && bbox.y1 >= 0.0
        && bbox.x2 > bbox.x1
        && bbox.y2 > bbox.y1
        && bbox.x2 <= width as f32
        && bbox.y2 <= height as f32;
    if !valid {
        return Err(Error::Benchmark("invalid or off-page detection".to_string()));
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DetectorPageRow {
    pub fixture_sha256: String,
    pub document: String,
    pub document_sha256: String,
    pub page: usize,
    pub iteration: usize,
    pub render_sha256: String,
    pub a_first: bool,
    pub measurements: [DetectorMeasurement; MODEL_ARMS],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelBenchmarkReport {
    pub schema_version: u32,
    pub quality_assessed: bool,
    pub xberg_build_id: String,
    pub cohort_sha256: String,
    pub expected_documents: BTreeMap<String, ExpectedDocument>,
    pub expected_pages: usize,
    pub expected_inference_calls: usize,
    pub actual_inference_calls: usize,
    pub iterations: usize,
    pub max_pages: usize,
    pub dpi: i32,
    pub confidence_threshold: f32,
    pub apply_heuristics: bool,
    pub models: [ModelIdentity; MODEL_ARMS],
    pub rows: Vec<DetectorPageRow>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resources: Option<super::PairedProcessResources>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub geometry: Option<super::GeometryEvidence>,
}

impl ModelBenchmarkReport {
    pub(super) fn new(
        config: &ModelBenchmarkConfig,
        models: [ModelIdentity; MODEL_ARMS],
        expected_documents: BTreeMap<String, ExpectedDocument>,
        rows: Vec<DetectorPageRow>,
        cohort_sha256: String,
    ) -> Result<Self> {
        let expected_pages = total_pages(&expected_documents)?;
        let expected_inference_calls = expected_calls(expected_pages, config.iterations)?;
        let actual_inference_calls = rows
            .iter()
            .flat_map(|row| &row.measurements)
            .filter(|measurement| measurement.inference_performed)
            .count();
        Ok(Self {
            schema_version: REPORT_SCHEMA_VERSION,
            quality_assessed: false,
            xberg_build_id: xberg::embedded_build_id().to_string(),
            cohort_sha256,
            expected_documents,
            expected_pages,
            expected_inference_calls,
            actual_inference_calls,
            iterations: config.iterations,
            max_pages: config.max_pages,
            dpi: config.dpi,
            confidence_threshold: config.confidence_threshold,
            apply_heuristics: false,
            models,
            rows,
            resources: None,
            geometry: None,
        })
    }

    pub fn validate(&self) -> Result<()> {
        self.validate_header()?;
        self.resources
            .as_ref()
            .ok_or_else(|| Error::Benchmark("missing paired process resource evidence".to_string()))?
            .validate()?;
        validate_model_identities(&self.models[0], &self.models[1])?;
        let mut seen = BTreeSet::new();
        let mut render_hashes = BTreeMap::new();
        for (ordinal, row) in self.rows.iter().enumerate() {
            self.validate_row(row)?;
            if !seen.insert((&row.document, row.page, row.iteration)) {
                return Err(Error::Benchmark("duplicate detector page/iteration row".to_string()));
            }
            let key = (&row.document, row.page);
            if let Some(previous) = render_hashes.insert(key, &row.render_sha256)
                && previous != &row.render_sha256
            {
                return Err(Error::Benchmark("page render changed between iterations".to_string()));
            }
            if row.a_first != ordinal.is_multiple_of(2) {
                return Err(Error::Benchmark("detector paired execution order mismatch".to_string()));
            }
        }
        let actual = self
            .rows
            .iter()
            .flat_map(|row| &row.measurements)
            .filter(|measurement| measurement.inference_performed)
            .count();
        if actual != self.actual_inference_calls || actual != self.expected_inference_calls {
            return Err(Error::Benchmark(format!(
                "detector row count mismatch: expected {} calls, observed {actual}",
                self.expected_inference_calls
            )));
        }
        if let Some(geometry) = &self.geometry {
            geometry.validate(self)?;
        }
        Ok(())
    }

    fn validate_header(&self) -> Result<()> {
        let supported = matches!(
            (self.schema_version, self.quality_assessed, self.geometry.is_some()),
            (REPORT_SCHEMA_VERSION, false, false) | (SCORED_REPORT_SCHEMA_VERSION, true, true)
        );
        if !supported || self.apply_heuristics {
            return Err(Error::Benchmark("unsupported detector report contract".to_string()));
        }
        if self.expected_documents.is_empty() || self.expected_documents.values().any(|doc| doc.pages == 0) {
            return Err(Error::Benchmark("detector benchmark corpus is empty".to_string()));
        }
        if !(1..=super::MAX_RENDER_DPI).contains(&self.dpi)
            || !self.confidence_threshold.is_finite()
            || !(0.0..=1.0).contains(&self.confidence_threshold)
            || !valid_sha256(&self.cohort_sha256)
            || self
                .expected_documents
                .values()
                .any(|doc| !valid_sha256(&doc.document_sha256) || !valid_sha256(&doc.fixture_sha256))
        {
            return Err(Error::Benchmark(
                "invalid detector configuration or input identity".to_string(),
            ));
        }
        let pages = total_pages(&self.expected_documents)?;
        if pages != self.expected_pages || pages == 0 || self.iterations == 0 || self.max_pages == 0 {
            return Err(Error::Benchmark("detector page-count mismatch".to_string()));
        }
        if self.expected_documents.values().any(|doc| doc.pages > self.max_pages)
            || self.expected_inference_calls != expected_calls(pages, self.iterations)?
        {
            return Err(Error::Benchmark(
                "detector inference-count expectation mismatch".to_string(),
            ));
        }
        Ok(())
    }

    fn validate_row(&self, row: &DetectorPageRow) -> Result<()> {
        let expected = self
            .expected_documents
            .get(&row.document)
            .ok_or_else(|| Error::Benchmark("unexpected detector document row".to_string()))?;
        if row.page == 0 || row.page > expected.pages || row.iteration == 0 || row.iteration > self.iterations {
            return Err(Error::Benchmark(
                "detector page/iteration exceeds declared limits".to_string(),
            ));
        }
        if row.document_sha256 != expected.document_sha256
            || row.fixture_sha256 != expected.fixture_sha256
            || !valid_sha256(&row.render_sha256)
        {
            return Err(Error::Benchmark("detector input identity mismatch".to_string()));
        }
        for measurement in &row.measurements {
            measurement.validate()?;
            if measurement
                .detections
                .iter()
                .any(|detection| detection.confidence < self.confidence_threshold)
            {
                return Err(Error::Benchmark(
                    "detector emitted a result below its configured threshold".to_string(),
                ));
            }
        }
        if (row.measurements[0].page_width, row.measurements[0].page_height)
            != (row.measurements[1].page_width, row.measurements[1].page_height)
        {
            return Err(Error::Benchmark(
                "detector arms saw different page dimensions".to_string(),
            ));
        }
        Ok(())
    }
}

pub(super) fn expected_calls(pages: usize, iterations: usize) -> Result<usize> {
    pages
        .checked_mul(iterations)
        .and_then(|count| count.checked_mul(MODEL_ARMS))
        .ok_or_else(|| Error::Config("detector inference count overflow".to_string()))
}

fn valid_time(value: f64) -> bool {
    value.is_finite() && value >= 0.0
}
pub(super) fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub(super) fn validate_model_identities(a: &ModelIdentity, b: &ModelIdentity) -> Result<()> {
    if a.detector == b.detector || a.sha256.eq_ignore_ascii_case(&b.sha256) {
        return Err(Error::Config(
            "detector arms have identical model identities".to_string(),
        ));
    }
    for model in [a, b] {
        if !valid_sha256(&model.sha256)
            || model.artifact_bytes == 0
            || model.execution_provider != "cpu"
            || !valid_time(model.load_ms)
            || model.intra_op_threads != super::DETECTOR_INTRA_OP_THREADS
            || model.inter_op_threads != super::DETECTOR_INTER_OP_THREADS
        {
            return Err(Error::Benchmark("invalid detector model identity".to_string()));
        }
    }
    Ok(())
}

pub(super) fn total_pages(documents: &BTreeMap<String, ExpectedDocument>) -> Result<usize> {
    documents.values().try_fold(0usize, |total, doc| {
        total
            .checked_add(doc.pages)
            .ok_or_else(|| Error::Config("detector page count overflow".to_string()))
    })
}
