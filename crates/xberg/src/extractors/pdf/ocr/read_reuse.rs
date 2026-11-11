//! Raw reads shared only between one extraction's ink probe and its final page route.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use crate::core::config::OcrConfig;
use crate::plugins::OcrBackend;
use crate::types::ExtractedDocument;

const MAX_RETAINED_CHARGE: usize = 8 << 20;
// Allow headroom for owned allocations that take less space in JSON. ~keep
const SERIALIZED_BYTE_CHARGE: usize = 8;
#[cfg(unix)]
const MODEL_TIMESTAMP_MARGIN: std::time::Duration = std::time::Duration::from_secs(2);

pub(crate) struct ProbeReads {
    state: parking_lot::Mutex<State>,
    budget: Arc<tokio::sync::Semaphore>,
    models: once_cell::sync::OnceCell<Option<ModelSnapshot>>,
    #[cfg(test)]
    model_hashes: std::sync::atomic::AtomicUsize,
}

#[derive(Default)]
struct State {
    ready: bool,
    reads: ahash::AHashMap<u32, Entry>,
}

struct Entry {
    key: blake3::Hash,
    backend: Arc<dyn OcrBackend>,
    document: ExtractedDocument,
    _permit: tokio::sync::OwnedSemaphorePermit,
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
            budget: Arc::new(tokio::sync::Semaphore::new(limit.min(MAX_RETAINED_CHARGE))),
            models: Default::default(),
            #[cfg(test)]
            model_hashes: Default::default(),
        }
    }

    pub(in crate::extractors::pdf) fn finish(&self, contradicted: &[u32]) -> bool {
        let (has_reads, discarded) = {
            let mut state = self.state.lock();
            let mut discarded = std::mem::take(&mut state.reads);
            for page in contradicted {
                if let Some(entry) = discarded.remove(page) {
                    state.reads.insert(*page, entry);
                }
            }
            state.ready = true;
            (!state.reads.is_empty(), discarded)
        };
        drop(discarded);
        has_reads
    }

    pub(in crate::extractors::pdf) fn release_after_final_read(self: &Arc<Self>) -> Option<FinalRead> {
        self.state.lock().ready.then(|| FinalRead(Arc::clone(self)))
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
        drop(state);
        Some(entry.document)
    }

    fn insert(&self, page: u32, backend: Arc<dyn OcrBackend>, key: ReadKey, read: &ExtractedDocument) {
        let Some(charge) = retained_size(read, self.budget.available_permits()) else {
            return;
        };
        let Ok(permit) = Arc::clone(&self.budget).try_acquire_many_owned(charge as u32) else {
            return;
        };
        let entry = Entry {
            key: key.digest,
            backend,
            document: read.clone(),
            _permit: permit,
        };
        let mut state = self.state.lock();
        if !state.ready {
            let previous = state.reads.insert(page, entry);
            drop(state);
            drop(previous);
        }
    }
}

pub(in crate::extractors::pdf) struct FinalRead(Arc<ProbeReads>);

impl Drop for FinalRead {
    fn drop(&mut self) {
        let reads = std::mem::take(&mut self.0.state.lock().reads);
        drop(reads);
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
        Some(reuse) => key(reuse, &identity, Arc::clone(&bytes)).await,
        None => None,
    };
    if let (Some(reuse), Some(key)) = (reuse, key.as_ref())
        && let Some(document) = reuse.take(identity.page, key, identity.backend)
        && revision(reuse, &identity).await == Some(key.revision.clone())
    {
        check_cancelled(cancel)?;
        #[cfg(all(test, feature = "bundle-tessdata-eng"))]
        crate::ocr::tesseract_backend::read_counter::record_probe_reuse(identity.configs.1);
        tracing::trace!(page = identity.page, "reused ink-probe OCR read");
        return Ok(document);
    }
    check_cancelled(cancel)?;
    let document = recognize(bytes).await?;
    check_cancelled(cancel)?;
    if let (Some(reuse), Some(key)) = (reuse, key)
        && reuse.collecting()
        && revision(reuse, &identity).await == Some(key.revision.clone())
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

async fn key(reuse: &Arc<ProbeReads>, identity: &ReadIdentity<'_>, bytes: Arc<Vec<u8>>) -> Option<ReadKey> {
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
        || !crate::plugins::registry::get_ocr_backend_registry()
            .read()
            .is_builtin_tesseract(identity.backend)
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
    let revision = revision(reuse, identity).await?;
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
        hash_config(&mut hash, config).ok()?;
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

fn hash_config(hash: &mut blake3::Hasher, config: &OcrConfig) -> serde_json::Result<()> {
    let mut config = config.clone();
    // Stock Tesseract leaves this discard policy to PDF scoring after the raw read. ~keep
    let mut thresholds = config.effective_thresholds();
    thresholds.discard_suspected_ocr_noise = false;
    config.quality_thresholds = Some(thresholds);
    hash_json(hash, &(&config, &config.security_limits, &config.acceleration))
}

#[derive(Clone, PartialEq, Eq)]
struct ModelFiles {
    root: PathBuf,
    languages: Vec<String>,
    stamps: Vec<Option<FileStamp>>,
}

#[derive(Clone, PartialEq, Eq)]
struct FileStamp {
    path: PathBuf,
    len: u64,
    modified: std::time::SystemTime,
    #[cfg(unix)]
    identity: (u64, u64, i64, i64),
}

impl FileStamp {
    fn read(path: &Path) -> std::io::Result<Self> {
        let path = path.canonicalize()?;
        let metadata = path.metadata()?;
        if !metadata.is_file() {
            return Err(std::io::Error::other("not a traineddata file"));
        }
        #[cfg(unix)]
        use std::os::unix::fs::MetadataExt;
        Ok(Self {
            path,
            len: metadata.len(),
            modified: metadata.modified()?,
            #[cfg(unix)]
            identity: (metadata.dev(), metadata.ino(), metadata.ctime(), metadata.ctime_nsec()),
        })
    }
}

impl ModelFiles {
    fn resolve(languages: Vec<String>, directory: Option<&Path>) -> Option<Self> {
        let env = crate::ocr::processor::validation::TessdataEnv::from_process();
        let root = crate::ocr::processor::validation::existing_tessdata_dir_for(&languages, directory, &env)?;
        let root = Path::new(&root).canonicalize().ok()?;
        let stamps = Self::stamps(&root, &languages)?;
        Some(Self {
            root,
            languages,
            stamps,
        })
    }

    fn stamps(root: &Path, languages: &[String]) -> Option<Vec<Option<FileStamp>>> {
        languages
            .iter()
            .map(|lang| (lang.as_str(), false))
            .chain([("osd", true)])
            .map(|(lang, optional)| {
                let path = root.join(format!("{lang}.traineddata"));
                match FileStamp::read(&path) {
                    Ok(stamp) => Some(Some(stamp)),
                    Err(error) if optional && error.kind() == std::io::ErrorKind::NotFound => Some(None),
                    Err(_) => None,
                }
            })
            .collect()
    }

    fn requires_rehash(&self, checked_at: SystemTime) -> bool {
        // A recent same-length write can land in the same filesystem clock tick. ~keep
        #[cfg(unix)]
        {
            let Some(cutoff) = checked_at.checked_sub(MODEL_TIMESTAMP_MARGIN) else {
                return true;
            };
            self.stamps.iter().flatten().any(|stamp| {
                let (.., changed_sec, changed_nsec) = stamp.identity;
                let changed = u64::try_from(changed_sec)
                    .ok()
                    .zip(u32::try_from(changed_nsec).ok())
                    .and_then(|(seconds, nanos)| {
                        SystemTime::UNIX_EPOCH.checked_add(std::time::Duration::new(seconds, nanos))
                    });
                stamp.modified >= cutoff || changed.is_none_or(|changed| changed >= cutoff)
            })
        }
        // Portable metadata has no change time to detect writes that restore mtime. ~keep
        #[cfg(not(unix))]
        {
            let _ = checked_at;
            true
        }
    }
}

struct ModelSnapshot {
    checked_at: SystemTime,
    files: ModelFiles,
    digest: blake3::Hash,
}

async fn revision(reuse: &Arc<ProbeReads>, identity: &ReadIdentity<'_>) -> Option<(String, blake3::Hash)> {
    if observer_active() {
        return None;
    }
    let config = identity.configs.1;
    let languages = config.effective_tesseract_language();
    let directory = config.tessdata_path.clone();
    let reuse = Arc::clone(reuse);
    let model = super::super::spawn_pdf_blocking(move || {
        let Some(files) = ModelFiles::resolve(languages, directory.as_deref()) else {
            return Ok(None);
        };
        Ok(model_revision(&reuse, &files))
    })
    .await
    .ok()??;
    Some((identity.backend.version(), model))
}

fn model_revision(reuse: &ProbeReads, files: &ModelFiles) -> Option<blake3::Hash> {
    let mut initialized = false;
    // Initialize inside the blocking worker so cancelling its waiter cannot restart the hash. ~keep
    let snapshot = reuse
        .models
        .get_or_init(|| {
            initialized = true;
            let checked_at = SystemTime::now();
            let digest = hash_stamped_models(reuse, files)?;
            Some(ModelSnapshot {
                checked_at,
                files: files.clone(),
                digest,
            })
        })
        .as_ref()?;
    if &snapshot.files != files {
        return None;
    }
    if !initialized
        && files.requires_rehash(snapshot.checked_at)
        && hash_stamped_models(reuse, files)? != snapshot.digest
    {
        return None;
    }
    Some(snapshot.digest)
}

fn hash_stamped_models(_reuse: &ProbeReads, files: &ModelFiles) -> Option<blake3::Hash> {
    #[cfg(test)]
    _reuse.model_hashes.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let digest = model_identity(files)?;
    (ModelFiles::stamps(&files.root, &files.languages)? == files.stamps).then_some(digest)
}

fn model_identity(files: &ModelFiles) -> Option<blake3::Hash> {
    let mut hash = blake3::Hasher::new();
    hash_json(&mut hash, &(&files.root, &files.languages)).ok()?;
    for stamp in &files.stamps {
        if let Some(stamp) = stamp {
            hash.update(&[1]);
            hash.update(&stamp.len.to_le_bytes());
            let mut file = std::fs::File::open(&stamp.path).ok()?;
            if std::io::copy(&mut file, &mut HashWriter(&mut hash)).ok()? != stamp.len {
                return None;
            }
        } else {
            hash.update(&[0]);
        }
    }
    Some(hash.finalize())
}

fn retained_size(document: &ExtractedDocument, limit: usize) -> Option<usize> {
    if document.raw_ocr_source_frame.is_some() || document.llm_usage.as_ref().is_some_and(|usage| !usage.is_empty()) {
        return None;
    }
    if has_nested_read(
        document.children.as_deref(),
        document.images.as_deref().unwrap_or_default(),
    ) || [&document.ocr_internal_document, &document.internal_document]
        .into_iter()
        .flatten()
        .any(|internal| has_nested_read(internal.children.as_deref(), &internal.images))
    {
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
    (writer.bytes <= limit).then_some(writer.bytes)
}

fn has_nested_read(children: Option<&[crate::types::ArchiveEntry]>, images: &[crate::types::ExtractedImage]) -> bool {
    // Nested documents may own data omitted from their serialized form. ~keep
    children.is_some_and(|children| !children.is_empty()) || images.iter().any(|image| image.ocr_result.is_some())
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
            .checked_add(bytes.len().saturating_mul(SERIALIZED_BYTE_CHARGE))
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
