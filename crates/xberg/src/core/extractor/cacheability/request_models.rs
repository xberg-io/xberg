use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use super::{current, invalidate_current, is_cacheable, ocr_stop_requested};
use crate::cache::blake3_hash_file;

const MAX_PENDING_OCR_WRITES: usize = 64;

#[derive(Default)]
pub(super) struct RequestModels {
    digests: Mutex<HashMap<PathBuf, String>>,
    writes: Mutex<Vec<Box<dyn FnOnce() + Send>>>,
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
        if writes.len() < MAX_PENDING_OCR_WRITES {
            writes.push(Box::new(write));
        }
    }

    pub(super) fn finish(&self) {
        let writes = std::mem::take(&mut *self.writes.lock().unwrap_or_else(std::sync::PoisonError::into_inner));
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
