use std::path::{Component, Path};

use super::{ModelSet, engine_error, hash_language_models};
use crate::ocr::error::OcrError;

pub(in crate::ocr::processor) struct RetryModelFiles(tempfile::TempDir);

impl RetryModelFiles {
    pub(in crate::ocr::processor) fn capture(models: &ModelSet) -> Result<Self, OcrError> {
        let files = Self(tempfile::tempdir().map_err(|error| engine_error("create private model directory", error))?);
        let source = Path::new(&models.directory);
        let languages = models.languages.split('+');
        for language in languages.clone() {
            files.copy_model(source, language)?;
        }
        let mut hash = hash_language_models(files.0.path(), languages)?;
        if models.implicit_osd {
            hash.update(files.capture_osd(source)?.as_bytes());
        }
        if hash.finalize() != models.revision {
            return Err(engine_error(
                "capture traineddata",
                "models changed before initialization",
            ));
        }
        Ok(files)
    }

    pub(in crate::ocr::processor) fn directory(&self) -> Result<&str, OcrError> {
        self.0
            .path()
            .to_str()
            .ok_or_else(|| engine_error("initialize private model directory", "path is not UTF-8"))
    }

    fn copy_model(&self, source: &Path, language: &str) -> Result<(), OcrError> {
        let name = format!("{language}.traineddata");
        let relative = Path::new(&name);
        if !relative.components().all(|part| matches!(part, Component::Normal(_))) {
            return Err(engine_error("copy traineddata", "expected a relative model name"));
        }
        let destination = self.0.path().join(relative);
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

    fn capture_osd(&self, source: &Path) -> Result<blake3::Hash, OcrError> {
        let private_path = self.0.path().join("osd.traineddata");
        crate::core::extractor::cacheability::fingerprint_osd_file(&source.join("osd.traineddata"), |resolved| {
            if !private_path.is_file() {
                std::fs::copy(resolved, &private_path)?;
            }
            crate::cache::blake3_hash_file(&private_path)
        })
        .map_err(|error| engine_error("capture implicit OSD model", error))
    }
}
