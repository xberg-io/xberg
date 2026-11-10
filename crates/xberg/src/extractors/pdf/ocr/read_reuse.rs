//! Raw reads shared only between one extraction's ink probe and its final page route.

use std::io::Write;
use std::sync::Arc;

use crate::core::config::OcrConfig;
use crate::plugins::{OcrBackend, OcrBackendType};
use crate::types::ExtractedDocument;

pub(crate) struct ProbeReads {
    state: parking_lot::Mutex<State>,
    limit: usize,
}

#[derive(Default)]
struct State {
    ready: bool,
    bytes: usize,
    reads: ahash::AHashMap<u32, Entry>,
}

struct Entry {
    key: blake3::Hash,
    backend: Arc<dyn OcrBackend>,
    document: ExtractedDocument,
    bytes: usize,
}

pub(super) struct ReadIdentity<'a> {
    pub(super) page: u32,
    pub(super) backend: &'a Arc<dyn OcrBackend>,
    pub(super) configs: (&'a OcrConfig, &'a OcrConfig),
    pub(super) whole_page_raster: bool,
    pub(super) correction_degrees: u32,
}

struct ReadKey {
    digest: blake3::Hash,
    revision: (String, blake3::Hash),
}

impl ProbeReads {
    pub(in crate::extractors::pdf) fn new(limit: usize) -> Self {
        Self {
            state: Default::default(),
            limit,
        }
    }

    pub(in crate::extractors::pdf) fn finish(&self, contradicted: &[u32]) -> bool {
        let mut state = self.state.lock();
        state.reads.retain(|page, _| contradicted.contains(page));
        state.bytes = state.reads.values().map(|entry| entry.bytes).sum();
        state.ready = true;
        !state.reads.is_empty()
    }

    fn may_read(&self, page: u32) -> bool {
        let state = self.state.lock();
        !state.ready || state.reads.contains_key(&page)
    }

    fn collecting(&self) -> bool {
        !self.state.lock().ready
    }

    fn take(&self, page: u32, key: &ReadKey, backend: &Arc<dyn OcrBackend>) -> Option<ExtractedDocument> {
        let mut state = self.state.lock();
        if !state.ready {
            return None;
        }
        let entry = state.reads.get(&page)?;
        if entry.key != key.digest || !Arc::ptr_eq(&entry.backend, backend) {
            return None;
        }
        let entry = state.reads.remove(&page)?;
        state.bytes -= entry.bytes;
        Some(entry.document)
    }

    fn insert(&self, page: u32, backend: Arc<dyn OcrBackend>, key: ReadKey, read: &ExtractedDocument) {
        let mut state = self.state.lock();
        if state.ready {
            return;
        }
        if let Some(previous) = state.reads.remove(&page) {
            state.bytes -= previous.bytes;
        }
        let Some(bytes) = retained_size(read, self.limit.saturating_sub(state.bytes)) else {
            return;
        };
        state.bytes += bytes;
        state.reads.insert(
            page,
            Entry {
                key: key.digest,
                backend,
                document: read.clone(),
                bytes,
            },
        );
    }
}

pub(super) async fn read<F, Fut>(
    reuse: Option<&Arc<ProbeReads>>,
    identity: ReadIdentity<'_>,
    bytes: Arc<Vec<u8>>,
    cancel: Option<&crate::cancellation::CancellationToken>,
    recognize: F,
) -> crate::Result<ExtractedDocument>
where
    F: FnOnce(Arc<Vec<u8>>) -> Fut,
    Fut: std::future::Future<Output = crate::Result<ExtractedDocument>>,
{
    if reuse.is_none() {
        return recognize(bytes).await;
    }
    check_cancelled(cancel)?;
    let reuse = reuse.filter(|reads| reads.may_read(identity.page));
    let key = match reuse {
        Some(_) => key(&identity, Arc::clone(&bytes)).await,
        None => None,
    };
    if let (Some(reuse), Some(key)) = (reuse, key.as_ref())
        && let Some(document) = reuse.take(identity.page, key, identity.backend)
        && revision(&identity).await == Some(key.revision.clone())
    {
        check_cancelled(cancel)?;
        #[cfg(all(test, feature = "bundle-tessdata-eng"))]
        crate::ocr::tesseract_backend::read_counter::record_probe_reuse(identity.configs.1);
        tracing::trace!(page = identity.page, "reused ink-probe OCR read");
        return Ok(document);
    }
    let document = recognize(bytes).await?;
    check_cancelled(cancel)?;
    if let (Some(reuse), Some(key)) = (reuse, key)
        && reuse.collecting()
        && revision(&identity).await == Some(key.revision.clone())
    {
        let reuse = Arc::clone(reuse);
        let backend = Arc::clone(identity.backend);
        let page = identity.page;
        let document = super::super::spawn_pdf_blocking(move || {
            reuse.insert(page, backend, key, &document);
            Ok(document)
        })
        .await?;
        check_cancelled(cancel)?;
        return Ok(document);
    }
    Ok(document)
}

fn check_cancelled(cancel: Option<&crate::cancellation::CancellationToken>) -> crate::Result<()> {
    if cancel.is_some_and(|token| token.is_cancelled()) {
        return Err(crate::XbergError::Cancelled);
    }
    Ok(())
}

async fn key(identity: &ReadIdentity<'_>, bytes: Arc<Vec<u8>>) -> Option<ReadKey> {
    if identity.whole_page_raster
        || observer_active()
        || [identity.configs.0, identity.configs.1].iter().any(|config| {
            config.auto_rotate == Some(true)
                || (config.auto_rotate != Some(false)
                    && config
                        .tesseract_config
                        .as_ref()
                        .and_then(|config| config.preprocessing.as_ref())
                        .is_some_and(|preprocessing| preprocessing.auto_rotate))
        })
        || !matches!(identity.backend.backend_type(), OcrBackendType::Tesseract)
        || identity.configs.0.tessdata_bytes.is_some()
        || identity.configs.1.tessdata_bytes.is_some()
        || !identity
            .configs
            .1
            .backend_options
            .as_ref()?
            .get("source_dpi")?
            .as_f64()?
            .is_finite()
    {
        return None;
    }
    let builtin = crate::plugins::registry::get_ocr_backend_registry()
        .read()
        .is_builtin_tesseract(identity.backend);
    let revision = revision(identity).await?;
    let mut hash = blake3::Hasher::new();
    hash.update(b"ink-probe-raw-whole-image-sparse-no-standard-retry");
    hash.update(&identity.page.to_le_bytes());
    hash.update(&identity.correction_degrees.to_le_bytes());
    hash_json(
        &mut hash,
        &(identity.backend.name(), &revision.0, revision.1.as_bytes()),
    )
    .ok()?;
    for config in [identity.configs.0, identity.configs.1] {
        hash_config(&mut hash, config, builtin).ok()?;
    }
    hash.update(&(bytes.len() as u64).to_le_bytes());
    hash.update(blake3::hash(bytes.as_slice()).as_bytes());
    Some(ReadKey {
        digest: hash.finalize(),
        revision,
    })
}

fn observer_active() -> bool {
    if !crate::core::extractor::cacheability::is_cacheable()
        || crate::ocr::preprocessing::geometry_evidence::GeometryEvidence::requested().is_some()
    {
        return true;
    }
    #[cfg(auto_rotate)]
    if crate::ocr::tesseract_backend::orientation_evidence::CAPTURE
        .try_with(|_| ())
        .is_ok()
    {
        return true;
    }
    false
}

fn hash_config(hash: &mut blake3::Hasher, config: &OcrConfig, builtin: bool) -> serde_json::Result<()> {
    let mut config = config.clone();
    if builtin {
        // Stock Tesseract leaves this discard policy to PDF scoring after the raw read. ~keep
        let mut thresholds = config.effective_thresholds();
        thresholds.discard_suspected_ocr_noise = false;
        config.quality_thresholds = Some(thresholds);
    }
    hash_json(hash, &(&config, &config.security_limits, &config.acceleration))
}

async fn revision(identity: &ReadIdentity<'_>) -> Option<(String, blake3::Hash)> {
    if observer_active() {
        return None;
    }
    let version = identity.backend.version();
    let config = identity.configs.1;
    let languages = config.effective_tesseract_language();
    let directory = config.tessdata_path.clone();
    let environment = crate::ocr::processor::validation::TessdataEnv::from_process();
    let model =
        super::super::spawn_pdf_blocking(move || Ok(model_identity(&languages, directory.as_deref(), &environment)))
            .await
            .ok()??;
    Some((version, model))
}

fn model_identity(
    languages: &[String],
    directory: Option<&std::path::Path>,
    environment: &crate::ocr::processor::validation::TessdataEnv,
) -> Option<blake3::Hash> {
    let root = crate::ocr::processor::validation::existing_tessdata_dir_for(languages, directory, environment)?;
    let root = std::fs::canonicalize(root).ok()?;
    let mut hash = blake3::Hasher::new();
    hash_json(&mut hash, &root).ok()?;
    for language in languages.iter().map(String::as_str).chain(std::iter::once("osd")) {
        hash_json(&mut hash, &language).ok()?;
        match std::fs::File::open(root.join(format!("{language}.traineddata"))) {
            Ok(mut file) => {
                hash.update(&[1]);
                let length = file.metadata().ok()?.len();
                hash.update(&length.to_le_bytes());
                if std::io::copy(&mut file, &mut HashWriter(&mut hash)).ok()? != length {
                    return None;
                }
            }
            Err(error) if language == "osd" && error.kind() == std::io::ErrorKind::NotFound => {
                hash.update(&[0]);
            }
            Err(_) => return None,
        }
    }
    Some(hash.finalize())
}

fn retained_size(document: &ExtractedDocument, limit: usize) -> Option<usize> {
    if document.raw_ocr_source_frame.is_some() || document.llm_usage.as_ref().is_some_and(|usage| !usage.is_empty()) {
        return None;
    }
    let mut writer = SizeWriter {
        bytes: std::mem::size_of::<Entry>(),
        limit,
    };
    serde_json::to_writer(
        &mut writer,
        &(
            document,
            &document.formatted_content,
            &document.ocr_internal_document,
            &document.internal_document,
            &document.raw_ocr_source_frame,
            document
                .ocr_internal_document
                .as_ref()
                .map(|document| (&document.recorded_formulas, &document.page_marker_format)),
            document
                .internal_document
                .as_ref()
                .map(|document| (&document.recorded_formulas, &document.page_marker_format)),
        ),
    )
    .ok()?;
    Some(writer.bytes)
}

struct HashWriter<'a>(&'a mut blake3::Hasher);
impl Write for HashWriter<'_> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.update(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn hash_json(hash: &mut blake3::Hasher, value: &impl serde::Serialize) -> serde_json::Result<()> {
    serde_json::to_writer(HashWriter(hash), value)?;
    hash.update(&[0]);
    Ok(())
}

struct SizeWriter {
    bytes: usize,
    limit: usize,
}
impl Write for SizeWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.bytes = self
            .bytes
            .checked_add(bytes.len())
            .filter(|total| *total <= self.limit)
            .ok_or_else(|| std::io::Error::other("ink probe retention limit"))?;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
#[path = "read_reuse_tests.rs"]
mod tests;

#[cfg(all(test, feature = "bundle-tessdata-eng"))]
#[path = "stock_tests.rs"]
mod stock_tests;
