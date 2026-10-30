//! Detector-only A/B measurements on identical PDF renders or original raster images.

use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Instant;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use xberg::core::config::acceleration::{AccelerationConfig, ExecutionProviderType};
use xberg::layout::{CustomModelVariant, LayoutEngine, LayoutEngineConfig, LayoutModelManager, ModelBackend};

use crate::corpus::CorpusDocument;
use crate::{CohortManifest, Error, Result};

mod contract;
mod geometry;
mod input;
mod resources;
#[cfg(test)]
mod tests;

pub use contract::{DetectorMeasurement, DetectorPageRow, ExpectedDocument, ModelBenchmarkReport, ModelIdentity};
pub use geometry::GeometryEvidence;
pub use resources::{PairedProcessResources, ResourceHostIdentity};

use input::{PreparedInput, read_document, validate_file_type};

const DEFAULT_RENDER_DPI: i32 = 150;
const DEFAULT_MAX_PAGES: usize = 3;
const DEFAULT_CONFIDENCE: f32 = 0.5;
const MAX_RENDER_DPI: i32 = 600;
const HASH_BUFFER_BYTES: usize = 64 * 1024;
const DETECTOR_INTRA_OP_THREADS: usize = 1;
const DETECTOR_INTER_OP_THREADS: usize = 1;

/// Detector benchmarking deliberately excludes OCR, table recognition and extraction caches.
#[derive(Clone)]
pub struct ModelBenchmarkConfig {
    pub fixtures_dir: PathBuf,
    pub cohort: PathBuf,
    pub model_a: String,
    pub model_b: String,
    pub max_pages: usize,
    pub iterations: usize,
    /// PDF rasterization DPI; image inputs retain their original pixel dimensions.
    pub dpi: i32,
    pub confidence_threshold: f32,
    pub expected_pages: Option<usize>,
    /// Hash-bound COCO geometry annotations; absent keeps timing-only reporting.
    pub geometry_annotations: Option<PathBuf>,
}

impl Default for ModelBenchmarkConfig {
    fn default() -> Self {
        Self {
            fixtures_dir: PathBuf::from("tools/benchmark-harness/fixtures"),
            cohort: PathBuf::from("tools/benchmark-harness/cohorts/layout-pdf-fast.json"),
            model_a: "rtdetr".to_string(),
            model_b: "pp_doclayout_v3".to_string(),
            max_pages: DEFAULT_MAX_PAGES,
            iterations: 1,
            dpi: DEFAULT_RENDER_DPI,
            confidence_threshold: DEFAULT_CONFIDENCE,
            expected_pages: None,
            geometry_annotations: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Detector {
    #[serde(rename = "rtdetr")]
    RtDetr,
    #[serde(rename = "pp_doclayout_v3")]
    PpDocLayoutV3,
}

impl Detector {
    fn parse(name: &str) -> Result<Self> {
        match name {
            "rtdetr" => Ok(Self::RtDetr),
            "pp_doclayout_v3" => Ok(Self::PpDocLayoutV3),
            _ => Err(Error::Config(format!("unknown layout detector: {name}"))),
        }
    }

    fn variant(self) -> CustomModelVariant {
        match self {
            Self::RtDetr => CustomModelVariant::RtDetr,
            Self::PpDocLayoutV3 => CustomModelVariant::PpDocLayoutV3,
        }
    }
}

struct PreparedDetector {
    identity: ModelIdentity,
    engine: LayoutEngine,
}

fn prepare_detector(detector: Detector, threshold: f32) -> Result<PreparedDetector> {
    let manager = LayoutModelManager::new(None);
    let path = match detector {
        Detector::RtDetr => manager.ensure_rtdetr_model(),
        Detector::PpDocLayoutV3 => manager.ensure_pp_doclayout_v3_model(),
    }
    .map_err(benchmark_error)?;
    let sha256 = hash_file(&path)?;
    let started = Instant::now();
    let engine = load_detector_engine(detector, &path, threshold)?;
    let load_ms = started.elapsed().as_secs_f64() * 1000.0;
    let intra_op_threads = engine.intra_op_thread_budget();
    if intra_op_threads != DETECTOR_INTRA_OP_THREADS {
        return Err(Error::Benchmark(
            "layout session did not retain the explicit thread budget".to_string(),
        ));
    }
    if hash_file(&path)? != sha256 {
        return Err(Error::Benchmark(
            "model artifact changed while its session was loaded".to_string(),
        ));
    }
    Ok(PreparedDetector {
        identity: ModelIdentity {
            detector,
            sha256,
            artifact_bytes: std::fs::metadata(path)?.len(),
            execution_provider: "cpu".to_string(),
            intra_op_threads,
            inter_op_threads: DETECTOR_INTER_OP_THREADS,
            load_ms,
        },
        engine,
    })
}

fn load_detector_engine(detector: Detector, path: &Path, threshold: f32) -> Result<LayoutEngine> {
    LayoutEngine::from_config_with_thread_budget(
        LayoutEngineConfig {
            backend: ModelBackend::Custom {
                path: path.to_path_buf(),
                variant: detector.variant(),
            },
            confidence_threshold: Some(threshold),
            apply_heuristics: false,
            cache_dir: None,
            acceleration: Some(AccelerationConfig {
                provider: ExecutionProviderType::Cpu,
                device_id: 0,
            }),
        },
        DETECTOR_INTRA_OP_THREADS,
    )
    .map_err(benchmark_error)
}

fn hash_file(path: &Path) -> Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; HASH_BUFFER_BYTES];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
    }
    Ok(hex_digest(&hash.finalize()))
}

fn hash_bytes(bytes: &[u8]) -> String {
    hex_digest(&Sha256::digest(bytes))
}

fn hex_digest(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn benchmark_error(error: impl std::fmt::Display) -> Error {
    Error::Benchmark(error.to_string())
}

fn validate_config(config: &ModelBenchmarkConfig) -> Result<[Detector; 2]> {
    let detectors = [Detector::parse(&config.model_a)?, Detector::parse(&config.model_b)?];
    if detectors[0] == detectors[1] {
        return Err(Error::Config("detector arms must be different".to_string()));
    }
    if config.max_pages == 0 || config.iterations == 0 {
        return Err(Error::Config(
            "max_pages and iterations must be greater than zero".to_string(),
        ));
    }
    if !(1..=MAX_RENDER_DPI).contains(&config.dpi)
        || !config.confidence_threshold.is_finite()
        || !(0.0..=1.0).contains(&config.confidence_threshold)
    {
        return Err(Error::Config("invalid render DPI or confidence threshold".to_string()));
    }
    Ok(detectors)
}

fn load_documents(config: &ModelBenchmarkConfig) -> Result<Vec<CorpusDocument>> {
    let manifest = CohortManifest::from_file(&config.cohort)?;
    let documents = manifest.load_corpus(&config.fixtures_dir, &config.cohort)?;
    if documents.is_empty() {
        return Err(Error::Config("detector benchmark corpus is empty".to_string()));
    }
    if documents.len() != manifest.fixtures.len() {
        return Err(Error::Config(
            "detector cohort must load every declared fixture".to_string(),
        ));
    }
    for document in &documents {
        validate_file_type(&document.file_type)?;
    }
    Ok(documents)
}

fn measure(detector: &mut PreparedDetector, image_bytes: &[u8]) -> Result<DetectorMeasurement> {
    let started = Instant::now();
    let result = detector
        .engine
        .detect_image_bytes(image_bytes)
        .map_err(benchmark_error)?;
    let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
    DetectorMeasurement::from_detection(result, elapsed_ms)
}

fn measure_pair(
    engines: &mut [PreparedDetector; 2],
    image_bytes: &[u8],
    ordinal: usize,
) -> Result<([DetectorMeasurement; 2], bool)> {
    // ~keep Alternate paired order; both arms consume the same rendered or original page bytes.
    let a_first = ordinal.is_multiple_of(2);
    let (a, b) = if a_first {
        let a = measure(&mut engines[0], image_bytes)?;
        let b = measure(&mut engines[1], image_bytes)?;
        (a, b)
    } else {
        let b = measure(&mut engines[1], image_bytes)?;
        let a = measure(&mut engines[0], image_bytes)?;
        (a, b)
    };
    Ok(([a, b], a_first))
}

fn plan_documents(
    documents: &[CorpusDocument],
    config: &ModelBenchmarkConfig,
) -> Result<BTreeMap<String, ExpectedDocument>> {
    let mut expected = BTreeMap::new();
    for doc in documents {
        let bytes = read_document(doc)?;
        let input = PreparedInput::open(&bytes, &doc.file_type)?;
        let pages = input.page_count().min(config.max_pages);
        if pages == 0 || expected.contains_key(&doc.name) {
            return Err(Error::Config(
                "empty PDF or duplicate detector document name".to_string(),
            ));
        }
        expected.insert(
            doc.name.clone(),
            ExpectedDocument {
                pages,
                document_sha256: hash_bytes(&bytes),
                fixture_sha256: hash_file(&doc.fixture_path)?,
            },
        );
    }
    let pages = contract::total_pages(&expected)?;
    if config.expected_pages.is_some_and(|declared| declared != pages) {
        return Err(Error::Config(format!(
            "detector page-count mismatch: planned {pages} pages"
        )));
    }
    eprintln!(
        "Planned {} documents, {pages} pages, {} detector calls",
        expected.len(),
        contract::expected_calls(pages, config.iterations)?
    );
    Ok(expected)
}

fn measure_document(
    doc: &CorpusDocument,
    expected: &ExpectedDocument,
    config: &ModelBenchmarkConfig,
    engines: &mut [PreparedDetector; 2],
    rows: &mut Vec<DetectorPageRow>,
) -> Result<()> {
    let bytes = read_document(doc)?;
    if hash_bytes(&bytes) != expected.document_sha256 || hash_file(&doc.fixture_path)? != expected.fixture_sha256 {
        return Err(Error::Benchmark("detector input changed after planning".to_string()));
    }
    let input = PreparedInput::open(&bytes, &doc.file_type)?;
    for page in 0..expected.pages {
        let prepared = input.page(page, config.dpi)?;
        let render_sha256 = hash_bytes(&prepared.bytes);
        for iteration in 0..config.iterations {
            let (measurements, a_first) = measure_pair(engines, &prepared.bytes, rows.len())?;
            prepared.validate_measurements(&measurements)?;
            rows.push(DetectorPageRow {
                fixture_sha256: expected.fixture_sha256.clone(),
                document: doc.name.clone(),
                document_sha256: expected.document_sha256.clone(),
                page: page + 1,
                iteration: iteration + 1,
                render_sha256: render_sha256.clone(),
                a_first,
                measurements,
            });
        }
    }
    Ok(())
}

/// Run actual detectors; optional bbox metrics do not constitute quality qualification.
pub async fn run_model_benchmark(config: &ModelBenchmarkConfig) -> Result<ModelBenchmarkReport> {
    validate_config(config)?;
    let config = config.clone();
    let monitor = crate::monitoring::ResourceMonitor::new();
    let (mut report, resources) = resources::measure_operation(&monitor, move || run_detectors(&config)).await?;
    report.resources = Some(resources);
    report.validate()?;
    Ok(report)
}

fn run_detectors(config: &ModelBenchmarkConfig) -> Result<ModelBenchmarkReport> {
    let detectors = validate_config(config)?;
    let cohort_sha256 = hash_file(&config.cohort)?;
    let documents = load_documents(config)?;
    let expected = plan_documents(&documents, config)?;
    let geometry = config
        .geometry_annotations
        .as_ref()
        .map(|path| geometry::load(path, &documents, &expected))
        .transpose()?;
    let mut engines = [
        prepare_detector(detectors[0], config.confidence_threshold)?,
        prepare_detector(detectors[1], config.confidence_threshold)?,
    ];
    contract::validate_model_identities(&engines[0].identity, &engines[1].identity)?;
    let mut rows = Vec::new();
    for doc in &documents {
        let planned = expected
            .get(&doc.name)
            .ok_or_else(|| Error::Benchmark("missing planned document".to_string()))?;
        measure_document(doc, planned, config, &mut engines, &mut rows)?;
    }
    if hash_file(&config.cohort)? != cohort_sha256 {
        return Err(Error::Benchmark("cohort manifest changed during inference".to_string()));
    }
    let mut report = ModelBenchmarkReport::new(
        config,
        engines.map(|engine| engine.identity),
        expected,
        rows,
        cohort_sha256,
    )?;
    if let Some(plan) = geometry {
        report.geometry = Some(plan.finish(&report)?);
        report.schema_version = contract::SCORED_REPORT_SCHEMA_VERSION;
        report.quality_assessed = true;
    }
    Ok(report)
}

pub fn print_model_table(report: &ModelBenchmarkReport) {
    eprintln!(
        "Detector benchmark: {} documents, {} pages, {} actual inference calls",
        report.expected_documents.len(),
        report.expected_pages,
        report.actual_inference_calls
    );
    eprintln!(
        "CPU, {} DPI, confidence {}, generic heuristics disabled; quality_assessed={}",
        report.dpi, report.confidence_threshold, report.quality_assessed
    );
    for model in &report.models {
        eprintln!(
            "{:?}: SHA256 {}, {} bytes, session load {:.1} ms",
            model.detector, model.sha256, model.artifact_bytes, model.load_ms
        );
        eprintln!(
            "  ORT session intra-op budget {}, inter-op budget {}",
            model.intra_op_threads, model.inter_op_threads
        );
    }
    if let Some(resources) = &report.resources {
        eprintln!(
            "Paired-process scope: {}; {} resource samples, peak RSS {} bytes, CPU {:.3} core-seconds",
            resources.scope, resources.sample_count, resources.peak_rss_bytes, resources.process_cpu_core_seconds
        );
        eprintln!("Timing contended: {}", resources.timing_contended);
    }
}
