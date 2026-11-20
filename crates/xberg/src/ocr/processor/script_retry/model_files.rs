use std::collections::HashMap;
use std::path::{Component, Path};
use std::sync::{Arc, Mutex, OnceLock, PoisonError, Weak};

use super::{ModelSet, engine_error, hash_language_models};
use crate::ocr::error::OcrError;

/// A verified private model directory shared by every engine that loads the same files.
///
/// The copy keeps the bytes a native engine reads identical to the revision the pool keyed
/// on, even when the source models change between the snapshot and initialization. The
/// directory lives as long as an engine references it and is removed when the last one is
/// dropped, or by the process-exit hook.
pub(in crate::ocr::processor) struct RetryModelFiles(Arc<SharedModelDirectory>);

impl RetryModelFiles {
    pub(in crate::ocr::processor) fn capture(models: &ModelSet) -> Result<Self, OcrError> {
        shared_model_directories().capture(models).map(Self)
    }

    pub(in crate::ocr::processor) fn directory(&self) -> Result<&str, OcrError> {
        self.0
            .directory
            .path()
            .to_str()
            .ok_or_else(|| engine_error("initialize private model directory", "path is not UTF-8"))
    }
}

struct SharedModelDirectory {
    directory: tempfile::TempDir,
}

impl SharedModelDirectory {
    fn capture(models: &ModelSet) -> Result<Self, OcrError> {
        let directory = tempfile::tempdir().map_err(|error| engine_error("create private model directory", error))?;
        let source = Path::new(&models.directory);
        let languages = models.languages.split('+');
        for language in languages.clone() {
            copy_model(directory.path(), source, language)?;
        }
        let mut hash = hash_language_models(directory.path(), languages, crate::cache::blake3_hash_file)?;
        if models.implicit_osd {
            hash.update(capture_osd(directory.path(), source)?.as_bytes());
        }
        if hash.finalize() != models.revision {
            return Err(engine_error(
                "capture traineddata",
                "models changed before initialization",
            ));
        }
        Ok(Self { directory })
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct ModelKey {
    languages: String,
    implicit_osd: bool,
    revision: blake3::Hash,
}

impl ModelKey {
    fn new(models: &ModelSet) -> Self {
        Self {
            languages: models.languages.clone(),
            implicit_osd: models.implicit_osd,
            revision: models.revision,
        }
    }
}

struct SharedEntry {
    directory: Weak<SharedModelDirectory>,
    path: std::path::PathBuf,
}

/// Process-wide private model directories, keyed by the files an engine loads.
///
/// An entry holds its directory weakly: while any engine uses it the `TempDir` stays alive,
/// and when the last engine drops it the directory is removed. Encoding the key from the
/// languages and the verified revision means engines that need identical bytes share one
/// copy instead of each materializing its own.
struct SharedModelDirectories {
    entries: Mutex<HashMap<ModelKey, SharedEntry>>,
}

impl SharedModelDirectories {
    fn new() -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
        }
    }

    fn capture(&self, models: &ModelSet) -> Result<Arc<SharedModelDirectory>, OcrError> {
        let key = ModelKey::new(models);
        if let Some(shared) = self.shared(&key) {
            verify_source(models)?;
            ensure_exit_cleanup_registered();
            return Ok(shared);
        }
        let shared = Arc::new(SharedModelDirectory::capture(models)?);
        let mut entries = self.entries.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(existing) = entries.get(&key).and_then(|entry| entry.directory.upgrade()) {
            return Ok(existing);
        }
        entries.insert(
            key,
            SharedEntry {
                directory: Arc::downgrade(&shared),
                path: shared.directory.path().to_owned(),
            },
        );
        drop(entries);
        ensure_exit_cleanup_registered();
        Ok(shared)
    }

    fn shared(&self, key: &ModelKey) -> Option<Arc<SharedModelDirectory>> {
        let entries = self.entries.lock().unwrap_or_else(PoisonError::into_inner);
        entries.get(key).and_then(|entry| entry.directory.upgrade())
    }

    #[cfg(test)]
    fn directory(&self, key: &ModelKey) -> Option<std::path::PathBuf> {
        let entries = self.entries.lock().unwrap_or_else(PoisonError::into_inner);
        entries.get(key).map(|entry| entry.path.clone())
    }

    /// Removes every registered directory, including one whose engine is still alive at
    /// process exit.
    fn remove_all(&self) {
        let mut entries = self.entries.lock().unwrap_or_else(PoisonError::into_inner);
        for (_, entry) in entries.drain() {
            // A dead entry's `TempDir` already removed its path; removing it again is harmless.
            let _ = std::fs::remove_dir_all(&entry.path);
        }
    }

    #[cfg(test)]
    fn live_count(&self) -> usize {
        let mut entries = self.entries.lock().unwrap_or_else(PoisonError::into_inner);
        entries.retain(|_, entry| entry.directory.strong_count() > 0);
        entries.len()
    }
}

static SHARED_MODEL_DIRECTORIES: OnceLock<SharedModelDirectories> = OnceLock::new();

fn shared_model_directories() -> &'static SharedModelDirectories {
    SHARED_MODEL_DIRECTORIES.get_or_init(SharedModelDirectories::new)
}

/// The registered path for `models`, live or already removed, for tests that must observe
/// whether a failed construction left a directory behind.
#[cfg(test)]
pub(in crate::ocr::processor) fn live_directory(models: &ModelSet) -> Option<std::path::PathBuf> {
    shared_model_directories().directory(&ModelKey::new(models))
}

/// Removes every shared model directory. The process-exit hook calls this; tests may call it
/// to observe the same cleanup without exiting.
pub(in crate::ocr::processor) fn remove_shared_model_directories() {
    shared_model_directories().remove_all();
}

fn verify_source(models: &ModelSet) -> Result<(), OcrError> {
    let source = Path::new(&models.directory);
    let languages = models.languages.split('+');
    let mut hash = hash_language_models(source, languages.clone(), crate::cache::blake3_hash_file)?;
    if models.implicit_osd {
        let revision = crate::core::extractor::cacheability::fingerprint_osd_file(
            &source.join("osd.traineddata"),
            crate::cache::blake3_hash_file,
        )
        .map_err(|error| engine_error("fingerprint implicit OSD model", error))?;
        hash.update(revision.as_bytes());
    }
    if hash.finalize() != models.revision {
        return Err(engine_error(
            "capture traineddata",
            "models changed before initialization",
        ));
    }
    Ok(())
}

fn copy_model(destination_root: &Path, source: &Path, language: &str) -> Result<(), OcrError> {
    let name = format!("{language}.traineddata");
    let relative = Path::new(&name);
    if !relative.components().all(|part| matches!(part, Component::Normal(_))) {
        return Err(engine_error("copy traineddata", "expected a relative model name"));
    }
    let destination = destination_root.join(relative);
    let input = source.join(relative);
    let copy = || -> std::io::Result<()> {
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(&input, &destination)?;
        Ok(())
    };
    copy().map_err(|error| engine_error(&format!("copy model '{}'", input.display()), error))
}

fn capture_osd(destination_root: &Path, source: &Path) -> Result<blake3::Hash, OcrError> {
    let private_path = destination_root.join("osd.traineddata");
    crate::core::extractor::cacheability::fingerprint_osd_file(&source.join("osd.traineddata"), |resolved| {
        if !private_path.is_file() {
            std::fs::copy(resolved, &private_path)?;
        }
        crate::cache::blake3_hash_file(&private_path)
    })
    .map_err(|error| engine_error("capture implicit OSD model", error))
}

#[cfg(not(target_arch = "wasm32"))]
static EXIT_CLEANUP: OnceLock<()> = OnceLock::new();

#[cfg(not(target_arch = "wasm32"))]
#[allow(unsafe_code)] // ~keep: declaring the C `atexit` this crate registers
unsafe extern "C" {
    fn atexit(handler: extern "C" fn()) -> std::ffi::c_int;
}

#[cfg(not(target_arch = "wasm32"))]
extern "C" fn exit_cleanup() {
    remove_shared_model_directories();
}

/// Registers [`remove_shared_model_directories`] once, after the first directory exists.
///
/// The hook is registered before the first engine initializes, which is before
/// `xberg-tesseract` registers its own handler after a successful recognition. C runs exit
/// handlers in reverse registration order, so the native engines are finalized first and
/// the directories they read still exist until then.
#[cfg(not(target_arch = "wasm32"))]
#[allow(unsafe_code)] // ~keep: C `atexit` is the only process-exit hook safe to use here
fn ensure_exit_cleanup_registered() {
    let _ = EXIT_CLEANUP.get_or_init(|| unsafe {
        atexit(exit_cleanup);
    });
}

#[cfg(target_arch = "wasm32")]
fn ensure_exit_cleanup_registered() {}

#[cfg(all(test, feature = "bundle-tessdata-eng"))]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn english_source() -> TempDir {
        let source = tempfile::tempdir().expect("source model directory");
        std::fs::write(
            source.path().join("eng.traineddata"),
            xberg_tesseract::bundled_eng_traineddata().expect("bundled English model"),
        )
        .expect("write the English model");
        source
    }

    fn snapshot(source: &TempDir, languages: &[&str]) -> ModelSet {
        ModelSet::snapshot(
            source.path().to_str().expect("UTF-8 source path").to_owned(),
            &languages
                .iter()
                .map(|language| (*language).to_owned())
                .collect::<Vec<_>>(),
            false,
        )
        .expect("model snapshot")
    }

    #[test]
    fn engines_loading_the_same_models_share_one_directory() {
        let source = english_source();
        let models = snapshot(&source, &["eng"]);
        let registry = SharedModelDirectories::new();

        let first = registry.capture(&models).expect("first capture");
        let second = registry.capture(&models).expect("second capture");

        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(first.directory.path(), second.directory.path());
        assert_eq!(registry.live_count(), 1);

        std::fs::write(source.path().join("fra.traineddata"), b"not a real model").unwrap();
        let distinct = snapshot(&source, &["eng", "fra"]);
        let third = registry.capture(&distinct).expect("distinct capture");

        assert_ne!(first.directory.path(), third.directory.path());
        assert_eq!(registry.live_count(), 2);
    }

    #[test]
    fn the_exit_cleanup_removes_directories_that_engines_still_reference() {
        let source = english_source();
        let models = snapshot(&source, &["eng"]);
        let registry = SharedModelDirectories::new();
        let files = registry.capture(&models).expect("capture");
        let path = files.directory.path().to_owned();

        assert!(path.is_dir());
        registry.remove_all();

        assert!(!path.exists());
        assert_eq!(registry.live_count(), 0);
    }

    #[test]
    fn the_last_engine_dropping_removes_its_directory() {
        let source = english_source();
        let models = snapshot(&source, &["eng"]);
        let registry = SharedModelDirectories::new();
        let files = registry.capture(&models).expect("capture");
        let path = files.directory.path().to_owned();

        drop(files);

        assert!(!path.exists());
        assert_eq!(registry.live_count(), 0);
    }

    #[test]
    fn changed_source_models_are_rejected_without_a_second_directory() {
        let source = english_source();
        let models = snapshot(&source, &["eng"]);
        let registry = SharedModelDirectories::new();
        let first = registry.capture(&models).expect("capture");

        std::fs::write(source.path().join("eng.traineddata"), b"changed after the snapshot").unwrap();
        let result = registry.capture(&models);

        assert!(matches!(result, Err(OcrError::ProcessingFailed(message)) if message.contains("models changed")));
        assert_eq!(registry.live_count(), 1);
        assert_eq!(
            first.directory.path(),
            registry.shared(&ModelKey::new(&models)).unwrap().directory.path()
        );
    }
}
