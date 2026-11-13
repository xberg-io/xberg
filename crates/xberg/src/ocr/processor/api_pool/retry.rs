use std::sync::Arc;

use super::super::script_retry::ModelSet;
use super::{ResourceLease, ResourcePool, lease_failure_message, new_engine};
use crate::ocr::error::OcrError;
use xberg_tesseract::TesseractAPI;

pub(in crate::ocr::processor) enum RetryStage {
    Osd,
    Han,
    Khmer,
}

type RetryResources = Arc<ResourcePool<(ModelSet, u8), TesseractAPI>>;

/// A retry holds its primary lease. Separate pools let it proceed when all primary slots are
/// occupied and let each stage retain its engine even when the configured capacity is one. ~keep
pub(in crate::ocr::processor) struct RetryApiPools {
    osd: RetryResources,
    han: RetryResources,
    khmer: RetryResources,
}

impl RetryApiPools {
    pub(super) fn new(capacity: usize) -> Self {
        Self {
            osd: ResourcePool::new(capacity),
            han: ResourcePool::new(capacity),
            khmer: ResourcePool::new(capacity),
        }
    }

    pub(in crate::ocr::processor) fn with_engine<T>(
        &self,
        stage: RetryStage,
        models: &ModelSet,
        oem: u8,
        run: impl FnOnce(&TesseractAPI) -> Result<T, OcrError>,
    ) -> Result<T, OcrError> {
        let resources = match stage {
            RetryStage::Osd => &self.osd,
            RetryStage::Han => &self.han,
            RetryStage::Khmer => &self.khmer,
        };
        let mut lease = RetryLease(resources.checkout((models.clone(), oem), || {
            new_engine(&models.directory, &models.languages, oem)
        })?);
        lease
            .0
            .mark_in_use()
            .map_err(|error| OcrError::ProcessingFailed(lease_failure_message(&error).into()))?;
        let result = run(&lease.0);
        // OSD also reports failure for ordinary images with too few usable characters. ~keep
        if result.is_err() && !matches!(stage, RetryStage::Osd) {
            return result;
        }
        models.validate()?;
        lease.0.release_with(reset);
        result
    }
}

struct RetryLease(ResourceLease<(ModelSet, u8), TesseractAPI>);

impl Drop for RetryLease {
    fn drop(&mut self) {
        self.0.release_with(|_| false);
    }
}

fn reset(api: &TesseractAPI) -> bool {
    if let Err(error) = api.clear().and_then(|()| api.clear_adaptive_classifier()) {
        tracing::warn!(%error, "Failed to reset pooled Tesseract retry engine; discarding handle");
        return false;
    }
    // SAFETY: the stage has finished using the handle. The pool marks it in use before reuse.
    #[allow(unsafe_code)]
    unsafe {
        api.mark_idle();
    }
    true
}

#[cfg(all(test, feature = "bundle-tessdata-eng"))]
mod tests {
    use super::*;
    use crate::ocr::processor::api_pool::{TesseractApiPool, tests::bundled_tessdata};
    use crate::ocr::processor::model_context::ModelContext;
    use std::sync::mpsc;
    use std::time::Duration;

    const WHITELIST: &str = "tessedit_char_whitelist";

    #[test]
    fn script_retry_reuses_each_stage_engine_while_the_primary_pool_is_full() {
        let (finished, verified) = mpsc::channel();
        let worker = std::thread::spawn(move || {
            let directory = bundled_tessdata();
            let path = directory.path().to_str().unwrap().to_owned();
            let models = ModelContext::for_path(path.clone(), 6).unwrap();
            let retry_models = ModelSet::snapshot(path, &["eng".into()], false).unwrap();
            let pool = TesseractApiPool::with_capacity(1);
            let primary = pool.checkout(&models, "eng", 3).unwrap();
            for expected in ["", "A"] {
                for stage in [RetryStage::Osd, RetryStage::Han, RetryStage::Khmer] {
                    pool.retry()
                        .with_engine(stage, &retry_models, 3, |api| {
                            assert_eq!(api.get_string_variable(WHITELIST).unwrap(), expected);
                            api.set_variable(WHITELIST, "A").unwrap();
                            Ok(())
                        })
                        .unwrap();
                }
            }
            assert_eq!(primary.get_string_variable(WHITELIST).unwrap(), "");
            finished.send(()).unwrap();
        });
        verified
            .recv_timeout(Duration::from_secs(10))
            .expect("all retry engines should be reused while the primary lease is held");
        worker.join().unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn a_same_length_model_change_with_restored_mtime_discards_the_handle() {
        let directory = bundled_tessdata();
        let path = directory.path().to_str().unwrap().to_owned();
        let model_file = directory.path().join("eng.traineddata");
        let modified = model_file.metadata().unwrap().modified().unwrap();
        let models = ModelSet::snapshot(path, &["eng".into()], false).unwrap();
        let pools = RetryApiPools::new(1);
        let result = pools.with_engine(RetryStage::Han, &models, 3, |_| {
            use std::io::Write;
            let mut file = std::fs::OpenOptions::new().write(true).open(&model_file).unwrap();
            file.write_all(b"XXXX").unwrap();
            file.set_modified(modified).unwrap();
            Ok(())
        });
        assert!(matches!(result, Err(OcrError::ProcessingFailed(message)) if message.contains("models changed")));
        assert_eq!(pools.han.counts(), (0, 0));
    }

    #[test]
    fn a_model_change_during_retry_discards_the_handle_even_after_rollback() {
        let directory = bundled_tessdata();
        let path = directory.path().to_str().unwrap().to_owned();
        let model_file = directory.path().join("eng.traineddata");
        let models = ModelSet::snapshot(path, &["eng".into()], false).unwrap();
        let pools = RetryApiPools::new(1);
        let result = pools.with_engine(RetryStage::Han, &models, 3, |api| {
            api.set_variable(WHITELIST, "A").unwrap();
            std::fs::write(&model_file, b"replacement").unwrap();
            Ok(())
        });
        assert!(matches!(result, Err(OcrError::ProcessingFailed(message)) if message.contains("models changed")));
        assert_eq!(pools.han.counts(), (0, 0));
        std::fs::write(&model_file, xberg_tesseract::bundled_eng_traineddata().unwrap()).unwrap();
        let whitelist = pools
            .with_engine(RetryStage::Han, &models, 3, |api| {
                Ok(api.get_string_variable(WHITELIST).unwrap())
            })
            .unwrap();
        assert_eq!(whitelist, "");
        assert_eq!(pools.han.counts(), (0, 1));
    }
}
