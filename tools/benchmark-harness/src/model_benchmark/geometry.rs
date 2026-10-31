//! Opt-in, hash-bound COCO bbox scoring on original raster coordinates.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use xberg::layout::LayoutClass;

use super::{ExpectedDocument, ModelBenchmarkReport, benchmark_error, hash_bytes, hash_file};
use crate::Result;
use crate::corpus::CorpusDocument;
use crate::quality::geometry::{self as metrics, GeometryMetrics, Page, Prediction};

#[path = "geometry/input.rs"]
mod input;
use input::{AnnotatedPage, AnnotationManifest, RetainedAnnotations};

const MAX_ANNOTATION_BYTES: u64 = 64 * 1024 * 1024;
const METRIC_CONTRACT: &str = concat!(
    "bbox-only; all areas; crowd/ignore unsupported; ",
    "AP: IoU .50:.05:.95, 101 recall points, maxDets=100/image/category; ",
    "P/R/F1: IoU .50, uncapped; stable score ties, images by COCO ID, equal-IoU later GT; ",
    "repeats scored separately; no qualification or reading-order claim"
);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ArmScore {
    pub metrics: GeometryMetrics,
    pub projected_class_counts: BTreeMap<String, usize>,
    pub relabeled_class_counts: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ScoreRow {
    pub document: String,
    pub page: usize,
    pub iteration: usize,
    pub arms: [ArmScore; 2],
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct IterationScore {
    pub iteration: usize,
    pub arms: [GeometryMetrics; 2],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeometryEvidence {
    pub metric_contract: String,
    pub qualification_complete: bool,
    pub source: String,
    pub annotation_manifest_sha256: String,
    pub annotation_sources: RetainedAnnotations,
    pub coco_sha256: String,
    pub mapping_sha256: String,
    pub scorer_sha256: String,
    pub ground_truth_sha256: String,
    pub category_mapping: BTreeMap<u64, String>,
    pub prediction_projection: BTreeMap<String, String>,
    pub pages: Vec<AnnotatedPage>,
    pub rows: Vec<ScoreRow>,
    pub iterations: Vec<IterationScore>,
}

pub(super) struct Plan {
    manifest_path: PathBuf,
    coco_path: PathBuf,
    evidence: GeometryEvidence,
}

pub(super) fn load(
    path: &Path,
    documents: &[CorpusDocument],
    expected: &BTreeMap<String, ExpectedDocument>,
) -> Result<Plan> {
    let bytes = bounded_read(path)?;
    let manifest: AnnotationManifest = serde_json::from_slice(&bytes)?;
    let coco_path = path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(&manifest.coco_file);
    let coco_bytes = bounded_read(&coco_path)?;
    let annotation_manifest_sha256 = hash_bytes(&bytes);
    let annotation_sources = RetainedAnnotations {
        manifest_json: String::from_utf8(bytes).map_err(|error| benchmark_error(error.to_string()))?,
        coco_json: String::from_utf8(coco_bytes).map_err(|error| benchmark_error(error.to_string()))?,
    };
    let (manifest, pages) = annotation_sources
        .decode(&annotation_manifest_sha256, &manifest.coco_sha256)
        .map_err(benchmark_error)?;
    validate_mapping(&manifest.category_mapping, &manifest.prediction_projection)?;
    validate_raster_bindings(&pages, documents, expected)?;
    let evidence = GeometryEvidence {
        metric_contract: METRIC_CONTRACT.into(),
        qualification_complete: false,
        source: manifest.source,
        annotation_manifest_sha256,
        annotation_sources,
        coco_sha256: manifest.coco_sha256,
        mapping_sha256: mapping_hash(&manifest.category_mapping, &manifest.prediction_projection)?,
        scorer_sha256: scorer_hash(),
        ground_truth_sha256: hash_bytes(&serde_json::to_vec(&pages)?),
        category_mapping: manifest.category_mapping,
        prediction_projection: manifest.prediction_projection,
        pages,
        rows: Vec::new(),
        iterations: Vec::new(),
    };
    Ok(Plan {
        manifest_path: path.into(),
        coco_path,
        evidence,
    })
}

fn bounded_read(path: &Path) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(MAX_ANNOTATION_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_ANNOTATION_BYTES {
        return Err(benchmark_error("geometry annotation file exceeds 64 MiB limit"));
    }
    Ok(bytes)
}

fn validate_mapping(mapping: &BTreeMap<u64, String>, projection: &BTreeMap<String, String>) -> Result<()> {
    let targets: BTreeSet<_> = mapping.values().cloned().collect();
    for label in mapping.values().chain(projection.keys()).chain(projection.values()) {
        serde_json::from_value::<LayoutClass>(serde_json::Value::String(label.clone()))
            .map_err(|_| benchmark_error(format!("unknown canonical geometry label: {label}")))?;
    }
    if targets.is_empty()
        || targets.iter().any(|target| projection.get(target) != Some(target))
        || projection.values().any(|target| !targets.contains(target))
    {
        return Err(benchmark_error(
            "geometry projection requires declared identity labels and mapped targets",
        ));
    }
    Ok(())
}

fn validate_raster_bindings(
    pages: &[AnnotatedPage],
    documents: &[CorpusDocument],
    expected: &BTreeMap<String, ExpectedDocument>,
) -> Result<()> {
    if documents
        .iter()
        .any(|doc| !matches!(doc.file_type.as_str(), "png" | "jpg" | "jpeg"))
    {
        return Err(benchmark_error(
            "geometry scoring requires original raster fixtures; PDF transforms are undeclared",
        ));
    }
    let selected: BTreeSet<_> = pages
        .iter()
        .map(|page| (page.binding.document.clone(), page.binding.page))
        .collect();
    let planned: BTreeSet<_> = expected
        .iter()
        .flat_map(|(name, doc)| (1..=doc.pages).map(move |page| (name.clone(), page)))
        .collect();
    if selected != planned || selected.len() != pages.len() {
        return Err(benchmark_error(
            "geometry annotations must cover exactly every planned page",
        ));
    }
    for page in pages {
        if page.binding.render_sha256 != expected[&page.binding.document].document_sha256 {
            return Err(benchmark_error("geometry image hash mismatch"));
        }
        let doc = documents
            .iter()
            .find(|doc| doc.name == page.binding.document)
            .ok_or_else(|| benchmark_error("geometry document is absent"))?;
        let bytes = super::read_document(doc)?;
        if hash_bytes(&bytes) != page.binding.render_sha256 {
            return Err(benchmark_error("geometry image hash mismatch"));
        }
        let input = super::PreparedInput::open(&bytes, &doc.file_type)?;
        let prepared = input.page(0, super::DEFAULT_RENDER_DPI)?;
        if (page.binding.width, page.binding.height) != (prepared.width, prepared.height) {
            return Err(benchmark_error("geometry annotation dimensions mismatch"));
        }
    }
    Ok(())
}

impl Plan {
    pub(super) fn finish(mut self, report: &ModelBenchmarkReport) -> Result<GeometryEvidence> {
        if hash_file(&self.manifest_path)? != self.evidence.annotation_manifest_sha256
            || hash_file(&self.coco_path)? != self.evidence.coco_sha256
        {
            return Err(benchmark_error("geometry annotations changed during inference"));
        }
        (self.evidence.rows, self.evidence.iterations) = self.evidence.compute(report)?;
        Ok(self.evidence)
    }
}

impl GeometryEvidence {
    pub(super) fn validate(&self, report: &ModelBenchmarkReport) -> Result<()> {
        validate_mapping(&self.category_mapping, &self.prediction_projection)?;
        let hashes = [
            &self.annotation_manifest_sha256,
            &self.coco_sha256,
            &self.mapping_sha256,
            &self.scorer_sha256,
            &self.ground_truth_sha256,
        ];
        if self.metric_contract != METRIC_CONTRACT
            || self.qualification_complete
            || self.source.trim().is_empty()
            || hashes.iter().any(|hash| !super::contract::valid_sha256(hash))
            || self.scorer_sha256 != scorer_hash()
            || self.mapping_sha256 != mapping_hash(&self.category_mapping, &self.prediction_projection)?
            || self.ground_truth_sha256 != hash_bytes(&serde_json::to_vec(&self.pages)?)
        {
            return Err(benchmark_error(
                "geometry evidence identity or metric contract mismatch",
            ));
        }
        self.validate_sources()?;
        self.validate_pages(report)?;
        let (rows, iterations) = self.compute(report)?;
        if rows != self.rows || iterations != self.iterations {
            return Err(benchmark_error(
                "geometry scores do not match retained detections and annotations",
            ));
        }
        Ok(())
    }

    fn validate_sources(&self) -> Result<()> {
        let (manifest, pages) = self
            .annotation_sources
            .decode(&self.annotation_manifest_sha256, &self.coco_sha256)
            .map_err(benchmark_error)?;
        if self.source != manifest.source
            || self.category_mapping != manifest.category_mapping
            || self.prediction_projection != manifest.prediction_projection
            || self.pages != pages
        {
            return Err(benchmark_error(
                "geometry normalized annotations differ from retained sources",
            ));
        }
        Ok(())
    }

    fn validate_pages(&self, report: &ModelBenchmarkReport) -> Result<()> {
        let mut ids = BTreeSet::new();
        let mut seen = BTreeSet::new();
        let mut regions = BTreeSet::new();
        for page in &self.pages {
            let binding = &page.binding;
            if !seen.insert((&binding.document, binding.page))
                || !ids.insert(binding.image_id)
                || !super::contract::valid_sha256(&binding.render_sha256)
                || page.regions.len() != binding.expected_annotations
            {
                return Err(benchmark_error("invalid or duplicate geometry annotation page"));
            }
            for region in &page.regions {
                if !regions.insert(region.annotation_id)
                    || self.category_mapping.get(&region.source_category_id) != Some(&region.region.class_name)
                {
                    return Err(benchmark_error("invalid geometry annotation identity or mapping"));
                }
                metrics::validate_region(&region.region, binding.width, binding.height).map_err(benchmark_error)?;
            }
        }
        let planned: BTreeSet<_> = report
            .expected_documents
            .iter()
            .flat_map(|(name, doc)| (1..=doc.pages).map(move |page| (name, page)))
            .collect();
        if seen != planned || self.pages.len() != report.expected_pages {
            return Err(benchmark_error(
                "geometry annotations must cover exactly every planned page",
            ));
        }
        Ok(())
    }

    fn compute(&self, report: &ModelBenchmarkReport) -> Result<(Vec<ScoreRow>, Vec<IterationScore>)> {
        let classes = self.category_mapping.values().cloned().collect();
        let mut rows = Vec::new();
        let mut samples: BTreeMap<usize, [Vec<Page>; 2]> = BTreeMap::new();
        for row in &report.rows {
            let annotation = self
                .pages
                .iter()
                .find(|page| page.binding.document == row.document && page.binding.page == row.page)
                .ok_or_else(|| benchmark_error("missing geometry page annotations"))?;
            let mut scores = Vec::new();
            for (arm, measurement) in row.measurements.iter().enumerate() {
                let page = projected_page(annotation, measurement, &self.prediction_projection, &row.render_sha256)?;
                let score = arm_score(&page, measurement, &self.prediction_projection, &classes)?;
                samples.entry(row.iteration).or_default()[arm].push(page);
                scores.push(score);
            }
            rows.push(ScoreRow {
                document: row.document.clone(),
                page: row.page,
                iteration: row.iteration,
                arms: scores
                    .try_into()
                    .map_err(|_| benchmark_error("geometry arm count mismatch"))?,
            });
        }
        let iterations = samples
            .into_iter()
            .map(|(iteration, pages)| {
                Ok(IterationScore {
                    iteration,
                    arms: [
                        metrics::score(&pages[0], &classes).map_err(benchmark_error)?,
                        metrics::score(&pages[1], &classes).map_err(benchmark_error)?,
                    ],
                })
            })
            .collect::<Result<_>>()?;
        Ok((rows, iterations))
    }
}

fn projected_page(
    annotation: &AnnotatedPage,
    measurement: &super::DetectorMeasurement,
    projection: &BTreeMap<String, String>,
    render_sha256: &str,
) -> Result<Page> {
    let binding = &annotation.binding;
    if binding.render_sha256 != render_sha256
        || (binding.width, binding.height) != (measurement.page_width, measurement.page_height)
    {
        return Err(benchmark_error("geometry render identity or dimensions mismatch"));
    }
    let predictions = measurement
        .detections
        .iter()
        .map(|detection| {
            let original = detection.class_name.as_str();
            let class_name = projection
                .get(original)
                .ok_or_else(|| benchmark_error(format!("unprojected detector class: {original}")))?
                .clone();
            let bbox = detection.bbox;
            Ok(Prediction {
                region: metrics::Region {
                    class_name,
                    bbox: [bbox.x1.into(), bbox.y1.into(), bbox.x2.into(), bbox.y2.into()],
                },
                confidence: detection.confidence.into(),
            })
        })
        .collect::<Result<_>>()?;
    Ok(Page {
        image_id: binding.image_id,
        width: binding.width,
        height: binding.height,
        truth: annotation.regions.iter().map(|region| region.region.clone()).collect(),
        predictions,
    })
}

fn arm_score(
    page: &Page,
    measurement: &super::DetectorMeasurement,
    projection: &BTreeMap<String, String>,
    classes: &BTreeSet<String>,
) -> Result<ArmScore> {
    let mut counts = BTreeMap::new();
    for prediction in &page.predictions {
        *counts.entry(prediction.region.class_name.clone()).or_default() += 1;
    }
    let relabeled = measurement
        .canonical_class_counts
        .iter()
        .filter(|(name, _)| projection.get(*name) != Some(*name))
        .map(|(name, count)| (name.clone(), *count))
        .collect();
    Ok(ArmScore {
        metrics: metrics::score(std::slice::from_ref(page), classes).map_err(benchmark_error)?,
        projected_class_counts: counts,
        relabeled_class_counts: relabeled,
    })
}

fn mapping_hash(mapping: &BTreeMap<u64, String>, projection: &BTreeMap<String, String>) -> Result<String> {
    Ok(hash_bytes(&serde_json::to_vec(&(mapping, projection))?))
}

fn scorer_hash() -> String {
    let sources = [
        include_bytes!("../quality/geometry.rs").as_slice(),
        include_bytes!("geometry/input.rs").as_slice(),
        include_bytes!("geometry.rs").as_slice(),
    ];
    hash_bytes(&sources.concat())
}
