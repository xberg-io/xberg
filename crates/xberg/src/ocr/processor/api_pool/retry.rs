use std::sync::Arc;

use super::super::script_retry::{ModelSet, RetryModelFiles};
use super::{ResourceLease, ResourcePool, lease_failure_message, new_engine};
use crate::ocr::error::OcrError;
use xberg_tesseract::TesseractAPI;

pub(in crate::ocr::processor) enum RetryStage {
    Osd,
    Han,
    Khmer,
}

type RetryResources = Arc<ResourcePool<(ModelSet, u8), RetryEngine>>;

struct RetryEngine {
    api: TesseractAPI,
    // Tesseract can load implicit OSD after initialization. Keep the private files for the engine lifetime. ~keep
    _files: RetryModelFiles,
}

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
        self.with_initializer(stage, models, oem, new_engine, run)
    }

    fn with_initializer<T>(
        &self,
        stage: RetryStage,
        models: &ModelSet,
        oem: u8,
        initialize: impl FnOnce(&str, &str, u8) -> Result<TesseractAPI, OcrError>,
        run: impl FnOnce(&TesseractAPI) -> Result<T, OcrError>,
    ) -> Result<T, OcrError> {
        let resources = match stage {
            RetryStage::Osd => &self.osd,
            RetryStage::Han => &self.han,
            RetryStage::Khmer => &self.khmer,
        };
        let mut lease = RetryLease(resources.checkout((models.clone(), oem), || {
            let files = RetryModelFiles::capture(models)?;
            let api = initialize(files.directory()?, &models.languages, oem)?;
            Ok::<_, OcrError>(RetryEngine { api, _files: files })
        })?);
        lease
            .0
            .api
            .mark_in_use()
            .map_err(|error| OcrError::ProcessingFailed(lease_failure_message(&error).into()))?;
        let result = run(&lease.0.api);
        // OSD also reports failure for ordinary images with too few usable characters. ~keep
        if result.is_err() && !matches!(stage, RetryStage::Osd) {
            return result;
        }
        models.validate()?;
        lease.0.release_with(|engine| reset(&engine.api));
        result
    }
}

struct RetryLease(ResourceLease<(ModelSet, u8), RetryEngine>);

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

    #[test]
    fn source_model_rollback_during_native_initialization_cannot_change_the_loaded_bytes() {
        let source = bundled_tessdata();
        let path = source.path().to_str().unwrap().to_owned();
        let model_file = source.path().join("eng.traineddata");
        let original = std::fs::read(&model_file).unwrap();
        let osd_file = source.path().join("osd.traineddata");
        std::fs::write(&osd_file, b"OSD revision A").unwrap();
        let models = ModelSet::snapshot(path.clone(), &["eng".into()], true).unwrap();
        let pools = RetryApiPools::new(1);
        pools
            .with_initializer(
                RetryStage::Han,
                &models,
                3,
                |directory, language, oem| {
                    std::fs::write(&model_file, b"revision B").unwrap();
                    std::fs::write(&osd_file, b"OSD revision B").unwrap();
                    let loaded = new_engine(directory, language, oem);
                    std::fs::write(&model_file, &original).unwrap();
                    std::fs::write(&osd_file, b"OSD revision A").unwrap();
                    assert_ne!(directory, path);
                    assert_eq!(
                        std::fs::read(std::path::Path::new(directory).join("eng.traineddata")).unwrap(),
                        original
                    );
                    assert_eq!(
                        std::fs::read(std::path::Path::new(directory).join("osd.traineddata")).unwrap(),
                        b"OSD revision A"
                    );
                    loaded
                },
                |api| {
                    api.set_variable(WHITELIST, "A").unwrap();
                    Ok(())
                },
            )
            .unwrap();
        assert_eq!(pools.han.counts(), (0, 1));
        pools
            .with_initializer(
                RetryStage::Han,
                &models,
                3,
                |_, _, _| panic!("verified A should be reused"),
                |api| {
                    assert_eq!(api.get_string_variable(WHITELIST).unwrap(), "A");
                    Ok(())
                },
            )
            .unwrap();
    }

    #[test]
    fn different_captured_bytes_never_reach_native_initialization() {
        let source = bundled_tessdata();
        let path = source.path().to_str().unwrap().to_owned();
        let models = ModelSet::snapshot(path, &["eng".into()], false).unwrap();
        std::fs::write(source.path().join("eng.traineddata"), b"revision B").unwrap();
        let pools = RetryApiPools::new(1);
        let result: Result<(), OcrError> = pools.with_initializer(
            RetryStage::Han,
            &models,
            3,
            |_, _, _| panic!("mismatched bytes reached the initializer"),
            |_| panic!("mismatched bytes reached recognition"),
        );
        assert!(matches!(result, Err(OcrError::ProcessingFailed(message)) if message.contains("models changed")));
        assert_eq!(pools.han.counts(), (0, 0));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    #[ignore = "requires local Tesseract eng/osd packs and the verified Chinese fixture"]
    fn warm_retry_loads_deferred_osd_from_its_private_models() {
        use xberg_tesseract::{Pix, TessPageSegMode};

        let tessdata = std::env::var_os("XBERG_OSD_TESSDATA").expect("set XBERG_OSD_TESSDATA");
        let fixture = std::env::var_os("XBERG_OSD_FIXTURE").expect("set XBERG_OSD_FIXTURE");
        let image = image::open(fixture)
            .expect("decode the verified Chinese fixture")
            .into_rgb8();
        let pixels = Pix::from_raw_rgb(image.as_raw(), image.width(), image.height()).unwrap();
        let source = tempfile::tempdir().unwrap();
        let path = source.path().to_str().unwrap().to_owned();
        let tessdata = std::path::Path::new(&tessdata);
        std::fs::copy(tessdata.join("eng.traineddata"), source.path().join("eng.traineddata")).unwrap();
        let read = |api: &TesseractAPI, mode| {
            api.set_page_seg_mode(mode).unwrap();
            api.set_image_2(pixels.as_ptr()).unwrap();
            api.set_source_resolution(300).unwrap();
            api.recognize().unwrap();
            api.get_utf8_text()
        };
        let baseline = |mode| read(&new_engine(&path, "eng", 3).unwrap(), mode).unwrap();
        let without_osd = baseline(TessPageSegMode::PSM_AUTO_OSD);
        let osd_file = source.path().join("osd.traineddata");
        std::fs::copy(tessdata.join("osd.traineddata"), &osd_file).unwrap();
        let with_osd = baseline(TessPageSegMode::PSM_AUTO_OSD);
        assert!(!with_osd.trim().is_empty());
        assert_ne!(
            with_osd, without_osd,
            "the fixture must distinguish actual implicit OSD"
        );
        let models = ModelSet::snapshot(path, &["eng".into()], true).unwrap();
        let pools = RetryApiPools::new(1);
        pools
            .with_engine(RetryStage::Han, &models, 3, |api| {
                read(api, TessPageSegMode::PSM_AUTO).unwrap();
                Ok(())
            })
            .unwrap();
        let parked_model = source.path().join("parked-osd");
        let warm = pools
            .with_initializer(
                RetryStage::Han,
                &models,
                3,
                |_, _, _| panic!("the deferred OSD read must use the warm engine"),
                |api| {
                    std::fs::rename(&osd_file, &parked_model).unwrap();
                    let text = read(api, TessPageSegMode::PSM_AUTO_OSD);
                    std::fs::rename(&parked_model, &osd_file).unwrap();
                    Ok(text.unwrap())
                },
            )
            .unwrap();
        assert_eq!(warm, with_osd);
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
