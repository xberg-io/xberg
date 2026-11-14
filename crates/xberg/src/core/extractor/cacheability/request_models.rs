use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use super::{current, invalidate_current, is_cacheable, ocr_stop_requested};
use crate::cache::blake3_hash_file;

const MAX_PENDING_OCR_WRITES: usize = 64;

type PendingWrites = Vec<Box<dyn FnOnce() + Send>>;

pub(super) struct RequestModels {
    digests: Mutex<HashMap<PathBuf, String>>,
    writes: Mutex<Option<PendingWrites>>,
}

impl Default for RequestModels {
    fn default() -> Self {
        Self {
            digests: Mutex::default(),
            writes: Mutex::new(Some(Vec::new())),
        }
    }
}

pub(crate) fn request_model_digest(path: &Path) -> crate::Result<String> {
    match current().and_then(|context| context.2) {
        Some(models) => models.digest(path),
        None => blake3_hash_file(path),
    }
}

impl RequestModels {
    // Freeze content identities for this request. Native engines with these keys load verified private copies. ~keep
    fn digest(&self, path: &Path) -> crate::Result<String> {
        let mut digests = self.digests.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(digest) = digests.get(path) {
            return Ok(digest.clone());
        }
        let digest = blake3_hash_file(path)?;
        digests.insert(path.to_owned(), digest.clone());
        Ok(digest)
    }

    pub(super) fn defer(&self, write: impl FnOnce() + Send + 'static) {
        let mut writes = self.writes.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(writes) = writes.as_mut().filter(|writes| writes.len() < MAX_PENDING_OCR_WRITES) {
            writes.push(Box::new(write));
        }
    }

    pub(super) fn finish(&self) {
        let Some(writes) = self
            .writes
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take()
        else {
            return;
        };
        if ocr_stop_requested() || !is_cacheable() {
            return;
        }
        let digests = self.digests.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        for (path, expected) in digests.iter() {
            match blake3_hash_file(path) {
                Ok(actual) if actual == *expected => {}
                Ok(_) => {
                    tracing::warn!(model = %path.display(), "Tesseract model changed during extraction");
                    invalidate_current();
                    return;
                }
                Err(error) => {
                    tracing::warn!(model = %path.display(), %error, "Could not validate Tesseract model at extraction boundary");
                    invalidate_current();
                    return;
                }
            }
        }
        drop(digests);
        for write in writes {
            if ocr_stop_requested() {
                break;
            }
            write();
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };

    #[tokio::test]
    #[allow(unsafe_code)]
    async fn mapped_model_edit_blocks_cache_publication_at_the_request_boundary() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("eng.traineddata");
        std::fs::write(&path, b"one").unwrap();
        let file = std::fs::OpenOptions::new().read(true).write(true).open(&path).unwrap();
        // SAFETY: the test owns the temporary file and keeps its size fixed while mapped.
        let mut mapping = unsafe { memmap2::MmapMut::map_mut(&file) }.unwrap();
        mapping.copy_from_slice(b"one");
        let published = Arc::new(AtomicBool::new(false));
        let original = super::super::scope(async {
            let original = super::super::with_ocr_cancellation(None, async {
                let digest = request_model_digest(&path).unwrap();
                let published = Arc::clone(&published);
                super::super::defer_ocr_cache_write(move || published.store(true, Ordering::Release));
                let before = file.metadata().unwrap();
                mapping.copy_from_slice(b"two");
                let after = file.metadata().unwrap();
                use std::os::unix::fs::MetadataExt;
                assert_eq!(before.ino(), after.ino());
                assert_eq!(before.len(), after.len());
                assert_eq!(request_model_digest(&path).unwrap(), digest);
                assert_ne!(blake3_hash_file(&path).unwrap(), digest);
                digest
            })
            .await;
            assert!(!super::super::is_cacheable());
            original
        })
        .await;
        assert!(!published.load(Ordering::Acquire));
        super::super::with_ocr_cancellation(None, async {
            assert_ne!(request_model_digest(&path).unwrap(), original);
            let published = Arc::clone(&published);
            super::super::defer_ocr_cache_write(move || published.store(true, Ordering::Release));
        })
        .await;
        assert!(published.load(Ordering::Acquire));
    }
}

#[cfg(test)]
#[tokio::test]
async fn timed_out_worker_cannot_publish_deferred_ocr_results() {
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };

    let token = crate::cancellation::CancellationToken::new();
    let early = Arc::new(AtomicBool::new(false));
    let late = Arc::new(AtomicBool::new(false));
    let (resume, wait) = std::sync::mpsc::channel();
    let worker = super::scope(async {
        let worker = super::with_ocr_cancellation(Some(&token), async {
            let early = Arc::clone(&early);
            super::defer_ocr_cache_write(move || early.store(true, Ordering::Relaxed));
            let late = Arc::clone(&late);
            let worker = tokio::task::spawn_blocking(super::blocking(move || {
                wait.recv().unwrap();
                assert!(super::ocr_stop_token().unwrap().stop_requested());
                let write_flag = Arc::clone(&late);
                super::defer_ocr_cache_write(move || write_flag.store(true, Ordering::Relaxed));
                assert_eq!(Arc::strong_count(&late), 2, "late write must never enter the queue");
            }));
            token.request_stop();
            worker
        })
        .await;
        assert!(!super::can_publish_outer_result());
        worker
    })
    .await;
    resume.send(()).unwrap();
    worker.await.unwrap();
    assert!(!early.load(Ordering::Relaxed));
    assert!(!late.load(Ordering::Relaxed));
}

#[cfg(test)]
#[tokio::test]
async fn worker_cannot_enqueue_after_deferred_ocr_queue_is_drained() {
    use std::sync::Arc;

    let (drained, wait) = std::sync::mpsc::channel();
    let value = Arc::new(());
    let worker = super::with_ocr_cancellation(None, async {
        super::defer_ocr_cache_write(move || drained.send(()).unwrap());
        let value = Arc::clone(&value);
        tokio::task::spawn_blocking(super::blocking(move || {
            wait.recv().unwrap();
            assert!(!super::ocr_stop_requested());
            let queued_value = Arc::clone(&value);
            super::defer_ocr_cache_write(move || drop(queued_value));
            assert_eq!(Arc::strong_count(&value), 2);
        }))
    })
    .await;
    worker.await.unwrap();
}
