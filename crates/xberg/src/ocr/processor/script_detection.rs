//! Script detection for a page that a caller may read again in another language (GH#2300).
//!
//! Detection runs on its own small engine pool. A detection engine never takes the place of a
//! recognition engine in the processor pool, and a recognition engine never takes the place of
//! a detection engine. ~keep

use std::path::Path;
use std::sync::{Arc, LazyLock};

use super::api_pool::TesseractApiPool;
use super::validation::{OSD_LANGUAGE, TessdataEnv, existing_tessdata_dir_for};
use crate::extractors::security::SecurityLimits;
use xberg_tesseract::TessPageSegMode;

/// The script name that Tesseract's orientation and script detection reports for Chinese text.
const OSD_HAN_SCRIPT: &str = "Han";

/// Engines kept for script detection. Detection is rare, so the pool is small.
const OSD_POOL_CAPACITY: usize = 2;

static OSD_POOL: LazyLock<Arc<TesseractApiPool>> = LazyLock::new(|| TesseractApiPool::with_capacity(OSD_POOL_CAPACITY));

/// `true` when the page's script is Han by Tesseract's orientation and script detection.
///
/// `read_again_languages` is the whole language list of the second pass. Nothing is downloaded:
/// when no directory of the search chain holds every one of those languages, or no directory
/// holds the `osd` model, the answer is `false` and the caller keeps its first pass. A failure
/// of the detection itself also answers `false`.
pub(crate) fn page_is_han_script(
    image_bytes: &[u8],
    read_again_languages: &[String],
    tessdata_override: Option<&Path>,
    security_limits: &SecurityLimits,
) -> bool {
    page_is_han_script_in(
        read_again_languages,
        tessdata_override,
        &TessdataEnv::from_process(),
        OSD_LANGUAGE,
        |osd_dir| detect_script(image_bytes, osd_dir, security_limits),
    )
}

/// [`page_is_han_script`] with the environment, the model name and the detection given as
/// arguments. `detect` runs only when the whole language list and the model are on disk.
fn page_is_han_script_in(
    read_again_languages: &[String],
    tessdata_override: Option<&Path>,
    env: &TessdataEnv,
    osd_model: &str,
    detect: impl FnOnce(&str) -> Result<bool, crate::ocr::error::OcrError>,
) -> bool {
    if existing_tessdata_dir_for(read_again_languages, tessdata_override, env).is_none() {
        return false;
    }
    let Some(osd_dir) = existing_tessdata_dir_for(&[osd_model.to_string()], tessdata_override, env) else {
        return false;
    };
    match detect(&osd_dir) {
        Ok(is_han) => is_han,
        Err(error) => {
            tracing::debug!(%error, "script detection failed; keeping the first OCR pass");
            false
        }
    }
}

fn detect_script(
    image_bytes: &[u8],
    osd_dir: &str,
    security_limits: &SecurityLimits,
) -> Result<bool, crate::ocr::error::OcrError> {
    use crate::ocr::error::OcrError;
    let rgb_image = crate::extraction::image::load_image_for_ocr(image_bytes, security_limits)
        .map_err(|e| OcrError::ImageProcessingFailed(e.to_string()))?
        .into_rgb8();
    let (width, height) = rgb_image.dimensions();
    let api = OSD_POOL.checkout(osd_dir, OSD_LANGUAGE, 3)?;
    let _ = api.set_page_seg_mode(TessPageSegMode::PSM_OSD_ONLY);
    api.set_image(rgb_image.as_raw(), width as i32, height as i32, 3, (width * 3) as i32)
        .map_err(|error| OcrError::ProcessingFailed(format!("Failed to set image for script detection: {error}")))?;
    let detected = api.detect_os();
    let _ = api.clear();
    Ok(matches!(detected, Ok((_, _, ref script, _)) if script == OSD_HAN_SCRIPT))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir_with(files: &[&str]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for file in files {
            std::fs::write(dir.path().join(format!("{file}.traineddata")), b"").unwrap();
        }
        dir
    }

    fn env() -> TessdataEnv {
        TessdataEnv {
            tessdata_prefix: None,
            cache_dir: None,
        }
    }

    fn languages(list: &[&str]) -> Vec<String> {
        list.iter().map(|l| l.to_string()).collect()
    }

    #[test]
    fn detection_runs_when_the_whole_list_and_the_model_are_in_one_chain() {
        let dir = dir_with(&["eng", "han-test-lang", "osd"]);
        let mut ran_in = None;
        let answer = page_is_han_script_in(
            &languages(&["eng", "han-test-lang"]),
            Some(dir.path()),
            &env(),
            "osd",
            |osd_dir| {
                ran_in = Some(osd_dir.to_string());
                Ok(true)
            },
        );
        assert!(answer);
        assert_eq!(ran_in.as_deref(), dir.path().to_str());
    }

    /// The added language is not on disk: no detection, and the probe never downloads.
    #[test]
    fn a_language_that_is_not_on_disk_skips_detection() {
        let dir = dir_with(&["eng", "osd"]);
        let answer = page_is_han_script_in(
            &languages(&["eng", "absent-test-lang"]),
            Some(dir.path()),
            &env(),
            "osd",
            |_| panic!("detection must not run when the list is not on disk"),
        );
        assert!(!answer);
        assert!(
            !dir.path().join("absent-test-lang.traineddata").exists(),
            "nothing is downloaded"
        );
    }

    #[test]
    fn a_missing_script_model_skips_detection() {
        let dir = dir_with(&["eng", "han-test-lang"]);
        let answer = page_is_han_script_in(
            &languages(&["eng", "han-test-lang"]),
            Some(dir.path()),
            &env(),
            "absent-test-osd",
            |_| panic!("detection must not run without the model"),
        );
        assert!(!answer);
    }

    #[test]
    fn a_failed_detection_keeps_the_first_pass() {
        let dir = dir_with(&["eng", "han-test-lang", "osd"]);
        let answer = page_is_han_script_in(
            &languages(&["eng", "han-test-lang"]),
            Some(dir.path()),
            &env(),
            "osd",
            |_| Err(crate::ocr::error::OcrError::ProcessingFailed("boom".to_string())),
        );
        assert!(!answer);
    }
}
