use crate::ocr::error::OcrError;
use crate::ocr::shaded_rows::normalize_shaded_rows;
use crate::types::ImagePreprocessingConfig;
use xberg_tesseract::Pix;

pub(crate) const ADAPTIVE_TILE_SIZE: i32 = 32;
const CONTRAST_GAMMA: f32 = 0.5;
const CONTRAST_INPUT_MIN: i32 = 40;
const CONTRAST_INPUT_MAX: i32 = 220;
const DARK_BACKGROUND_MEAN_THRESHOLD: f64 = 100.0;
const DENOISE_KERNEL_SIZE: i32 = 3;
const LIGHT_PIXEL_VALUE_THRESHOLD: u8 = 180;
const MIN_LIGHT_PIXEL_FRACTION_FOR_INVERT: f64 = 0.01;
const POLARITY_SAMPLE_STRIDE: i32 = 4;
const SAUVOLA_FACTOR: f32 = 0.35;
const SAUVOLA_MAX_TILE_DIMENSION: i32 = 512;
const SAUVOLA_WINDOW_HALF_SIZE: i32 = 15;
const UNSHARP_FRACTION: f32 = 0.5;
const UNSHARP_HALF_WIDTH: i32 = 3;

pub(crate) fn should_invert_for_polarity(mean_gray: f64, light_fraction: f64, force_invert: bool) -> bool {
    force_invert
        || (mean_gray < DARK_BACKGROUND_MEAN_THRESHOLD && light_fraction >= MIN_LIGHT_PIXEL_FRACTION_FOR_INVERT)
}

/// Whether `normalize_shaded_rows` was asked for but cannot act.
///
/// ~keep A named predicate rather than an inline condition so the reporting path can be asserted
/// without a real raster, and so the two facts it depends on -- the config combination and the
/// `drop(gray)` in [`preprocess_pix`]'s non-binarized contrast arm -- stay visibly coupled.
pub(crate) fn shaded_row_normalization_is_inert(config: &ImagePreprocessingConfig, binarization_enabled: bool) -> bool {
    config.normalize_shaded_rows && !binarization_enabled && config.contrast_enhance
}

pub(crate) fn preprocess_pix(pix: Pix, config: &ImagePreprocessingConfig) -> Result<Pix, OcrError> {
    crate::core::config_validation::validate_image_preprocessing_config(config).map_err(|error| {
        if let crate::XbergError::Validation { message, .. } = error {
            OcrError::InvalidConfiguration(message)
        } else {
            OcrError::InvalidConfiguration(error.to_string())
        }
    })?;
    let binarization_method = config.binarization_method.to_ascii_lowercase();
    let binarization_enabled = !matches!(binarization_method.as_str(), "none" | "off");

    let gray = pix
        .to_grayscale()
        .map_err(preprocessing_error("convert to grayscale"))?;
    // Only non-binarized contrast re-reads the color source. Release it before
    // thresholding allocates full-page grayscale tiles and sampling copies. ~keep
    let contrast_source = if !binarization_enabled && config.contrast_enhance {
        Some(pix)
    } else {
        drop(pix);
        None
    };
    // ~keep normalize_shaded_rows only touches `gray`; the `contrast_enhance` path with
    // binarization disabled below reconverts from the original `pix` and does not see it
    // (GH#1785 scope: the fix targets the binarized path, which is the config default).
    // GH#1837: that combination made the option silently inert, so say so rather than letting a
    // caller believe their request took effect. The work stays skipped -- running it would change
    // the output of a config combination nothing has measured.
    if shaded_row_normalization_is_inert(config, binarization_enabled) {
        tracing::warn!(
            binarization_method = %config.binarization_method,
            "normalize_shaded_rows was requested but has no effect: with binarization disabled and \
             contrast_enhance on, the non-binarized contrast path re-reads the original image and \
             discards the per-band normalization"
        );
    }
    let gray = if config.normalize_shaded_rows {
        apply_optional(gray, "normalize shaded rows", normalize_shaded_rows)
    } else {
        gray
    };
    let polarity_stats = gray
        .grayscale_stats(LIGHT_PIXEL_VALUE_THRESHOLD, POLARITY_SAMPLE_STRIDE)
        .ok();
    let should_invert = match polarity_stats {
        Some((mean, light_fraction)) => should_invert_for_polarity(mean, light_fraction, config.invert_colors),
        None => config.invert_colors,
    };

    let mut processed = if binarization_enabled && should_invert {
        gray.invert().map_err(preprocessing_error("invert colors"))?
    } else if binarization_enabled {
        gray
    } else if let Some(pix) = contrast_source {
        drop(gray);
        enhance_non_binarized(pix, should_invert)?
    } else if should_invert {
        gray.invert().map_err(preprocessing_error("invert colors"))?
    } else {
        gray
    };
    if config.denoise {
        processed = apply_optional(processed, "denoise", |source| {
            source.median_filter(DENOISE_KERNEL_SIZE, DENOISE_KERNEL_SIZE)
        });
    }
    if config.contrast_enhance && binarization_enabled {
        processed = apply_optional(processed, "enhance contrast", enhance_contrast);
    }

    if binarization_enabled {
        processed = apply_optional(processed, "binarize", |source| {
            binarize(source, &config.binarization_method)
        });
    }
    Ok(deskew_if_enabled(processed, config.deskew))
}

#[cfg(not(target_arch = "wasm32"))]
/// Whether every pixel of packed RGB has equal channels, i.e. the raster carries only gray.
pub(crate) fn rgb_is_gray(rgb: &[u8]) -> bool {
    rgb.as_chunks::<3>()
        .0
        .iter()
        .all(|pixel| pixel[0] == pixel[1] && pixel[1] == pixel[2])
}

#[cfg(not(target_arch = "wasm32"))]
/// [`preprocess_pix`] for the non-binarized contrast policy on a gray `rgb` raster, without a 32-bit Pix:
/// the plane becomes an 8-bit Pix and `rgb` is freed before background normalization and sharpening.
/// For equal channels the colour path's per-channel normalization and sharpening are the same operation on
/// each channel, so the result is pixel-identical. Until the 8-bit Pix exists `rgb` is untouched and an
/// error leaves the caller its raw fallback; afterwards every step keeps the best Pix already built.
pub(crate) fn preprocess_gray_contrast(
    rgb: &mut Vec<u8>,
    width: u32,
    height: u32,
    config: &ImagePreprocessingConfig,
) -> Result<Pix, OcrError> {
    crate::core::config_validation::validate_image_preprocessing_config(config)
        .map_err(|error| OcrError::InvalidConfiguration(error.to_string()))?;
    let plane: Vec<u8> = rgb.as_chunks::<3>().0.iter().map(|pixel| pixel[0]).collect();
    let gray = Pix::from_grayscale_bytes(&plane, width, height).map_err(preprocessing_error("create gray raster"))?;
    drop(plane);
    drop(std::mem::take(rgb));
    Ok(enhance_gray_contrast(gray, config.invert_colors))
}

#[cfg(not(target_arch = "wasm32"))]
fn enhance_gray_contrast(gray: Pix, invert_colors: bool) -> Pix {
    let should_invert = match gray.grayscale_stats(LIGHT_PIXEL_VALUE_THRESHOLD, POLARITY_SAMPLE_STRIDE) {
        Ok((mean, light_fraction)) => should_invert_for_polarity(mean, light_fraction, invert_colors),
        Err(_) => invert_colors,
    };
    enhance_gray_steps(gray, should_invert, Pix::background_normalize, |pix| {
        pix.unsharp_mask(UNSHARP_HALF_WIDTH, UNSHARP_FRACTION)
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn enhance_gray_steps(
    gray: Pix,
    invert: bool,
    normalize: impl FnOnce(&Pix) -> xberg_tesseract::Result<Pix>,
    sharpen: impl FnOnce(&Pix) -> xberg_tesseract::Result<Pix>,
) -> Pix {
    let source = if invert {
        apply_optional(gray, "invert colors", Pix::invert)
    } else {
        gray
    };
    let normalized = apply_optional(source, "normalize background", normalize);
    apply_optional(normalized, "sharpen", sharpen)
}

fn deskew_if_enabled(pix: Pix, enabled: bool) -> Pix {
    if !enabled || pix.depth() != 1 {
        return pix;
    }
    apply_optional(pix, "deskew", |source| {
        let output = source.deskew();
        #[cfg(all(feature = "pdf", feature = "tokio-runtime", not(target_arch = "wasm32")))]
        geometry_evidence::record(output.as_ref().is_ok_and(|result| result.as_ptr() == source.as_ptr()));
        output
    })
}

fn enhance_non_binarized(pix: Pix, invert: bool) -> Result<Pix, OcrError> {
    let source = if invert {
        let inverted = pix.invert().map_err(preprocessing_error("invert colors"))?;
        drop(pix);
        inverted
    } else {
        pix
    };
    let normalized = source
        .background_normalize()
        .map_err(preprocessing_error("normalize background"))?;
    let sharpened = normalized
        .unsharp_mask(UNSHARP_HALF_WIDTH, UNSHARP_FRACTION)
        .map_err(preprocessing_error("sharpen"))?;
    sharpened
        .to_grayscale()
        .map_err(preprocessing_error("convert to grayscale"))
}

fn enhance_contrast(source: &Pix) -> xberg_tesseract::Result<Pix> {
    let normalized = source.background_normalize()?;
    normalized.contrast_stretch(CONTRAST_GAMMA, CONTRAST_INPUT_MIN, CONTRAST_INPUT_MAX)
}

fn binarize(pix: &Pix, method: &str) -> xberg_tesseract::Result<Pix> {
    match method.to_ascii_lowercase().as_str() {
        "otsu" => pix.otsu_threshold(),
        "adaptive" if pix.width().min(pix.height()) >= ADAPTIVE_TILE_SIZE => {
            pix.adaptive_threshold(ADAPTIVE_TILE_SIZE, ADAPTIVE_TILE_SIZE)
        }
        "adaptive" => {
            tracing::warn!(
                requested_binarization_method = "adaptive",
                width = pix.width(),
                height = pix.height(),
                minimum_dimension = ADAPTIVE_TILE_SIZE,
                fallback_binarization_method = "otsu",
                "image too small for adaptive binarization; falling back to Otsu threshold"
            );
            pix.otsu_threshold()
        }
        "sauvola" if pix.width().min(pix.height()) > SAUVOLA_WINDOW_HALF_SIZE * 2 + 2 => {
            let tile_columns = tile_count(pix.width());
            let tile_rows = tile_count(pix.height());
            pix.sauvola_threshold(SAUVOLA_WINDOW_HALF_SIZE, SAUVOLA_FACTOR, tile_columns, tile_rows)
        }
        "sauvola" => {
            tracing::warn!(
                requested_binarization_method = "sauvola",
                width = pix.width(),
                height = pix.height(),
                minimum_dimension = SAUVOLA_WINDOW_HALF_SIZE * 2 + 2,
                fallback_binarization_method = "otsu",
                "image too small for Sauvola binarization; falling back to Otsu threshold"
            );
            pix.otsu_threshold()
        }
        _ => Err(xberg_tesseract::TesseractError::InvalidParameterError),
    }
}

fn tile_count(dimension: i32) -> i32 {
    dimension
        .saturating_add(SAUVOLA_MAX_TILE_DIMENSION - 1)
        .checked_div(SAUVOLA_MAX_TILE_DIMENSION)
        .unwrap_or(1)
        .max(1)
}

fn apply_optional(
    source: Pix,
    operation: &'static str,
    transform: impl FnOnce(&Pix) -> xberg_tesseract::Result<Pix>,
) -> Pix {
    match transform(&source) {
        Ok(transformed) => transformed,
        Err(error) => {
            tracing::warn!(operation, %error, "OCR image preprocessing step failed; retaining prior raster");
            source
        }
    }
}

fn preprocessing_error(operation: &'static str) -> impl FnOnce(xberg_tesseract::TesseractError) -> OcrError {
    move |error| OcrError::ProcessingFailed(format!("Failed to {operation}: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_preserve_grayscale_when_binarization_is_disabled() {
        let config = ImagePreprocessingConfig {
            deskew: false,
            binarization_method: "none".to_string(),
            ..Default::default()
        };
        let mut rgb = Vec::with_capacity(64 * 64 * 3);
        for value in 0..(64 * 64) {
            let gray = (value % 256) as u8;
            rgb.extend_from_slice(&[gray, gray, gray]);
        }
        let expected = Pix::from_raw_rgb(&rgb, 64, 64).unwrap().to_grayscale().unwrap();
        let pix = Pix::from_raw_rgb(&rgb, 64, 64).unwrap();

        let processed = preprocess_pix(pix, &config).unwrap();
        let expected_stats = expected.grayscale_stats(192, 1).unwrap();
        let processed_stats = processed.grayscale_stats(192, 1).unwrap();
        let (_, low_threshold_fraction) = processed.grayscale_stats(64, 1).unwrap();
        let (_, high_threshold_fraction) = processed_stats;

        assert_eq!(
            processed.depth(),
            8,
            "disabled binarization must retain grayscale samples"
        );
        assert_eq!(
            processed_stats, expected_stats,
            "none must not enhance grayscale pixels"
        );
        assert!(
            low_threshold_fraction > high_threshold_fraction,
            "none must retain intermediate grayscale levels"
        );
    }

    #[test]
    fn should_accept_off_as_disabled_binarization_alias() {
        let off_config = ImagePreprocessingConfig {
            deskew: false,
            binarization_method: "off".to_string(),
            ..Default::default()
        };
        let none_config = ImagePreprocessingConfig {
            deskew: false,
            binarization_method: "none".to_string(),
            ..Default::default()
        };
        let mut rgb = Vec::with_capacity(64 * 64 * 3);
        for value in 0..(64 * 64) {
            let gray = (value % 256) as u8;
            rgb.extend_from_slice(&[gray, gray, gray]);
        }

        let off = preprocess_pix(Pix::from_raw_rgb(&rgb, 64, 64).unwrap(), &off_config).unwrap();
        let none = preprocess_pix(Pix::from_raw_rgb(&rgb, 64, 64).unwrap(), &none_config).unwrap();

        assert_eq!(off.depth(), 8);
        assert_eq!(
            off.grayscale_stats(64, 1).unwrap(),
            none.grayscale_stats(64, 1).unwrap()
        );
        assert_eq!(
            off.grayscale_stats(192, 1).unwrap(),
            none.grayscale_stats(192, 1).unwrap()
        );
    }

    #[test]
    fn should_reject_deskew_without_binarization() {
        let config = ImagePreprocessingConfig {
            deskew: true,
            binarization_method: "none".to_string(),
            ..Default::default()
        };
        let pix = Pix::from_raw_rgb(&vec![200; 64 * 64 * 3], 64, 64).unwrap();

        let result = preprocess_pix(pix, &config);

        assert!(matches!(
            result,
            Err(OcrError::InvalidConfiguration(message))
                if message == "deskew must be false when binarization_method is none or off"
        ));
    }

    #[test]
    fn should_reject_unknown_binarization_method() {
        let config = ImagePreprocessingConfig {
            binarization_method: "unknown".to_string(),
            ..Default::default()
        };
        let pix = Pix::from_raw_rgb(&vec![200; 64 * 64 * 3], 64, 64).unwrap();

        let result = preprocess_pix(pix, &config);

        assert!(matches!(result, Err(OcrError::InvalidConfiguration(_))));
    }

    /// A `tracing` `Layer` that records every emitted event's level and formatted
    /// `message` field, for GH#1785's silent-adaptive/Sauvola-fallback regression tests.
    #[derive(Clone, Default)]
    struct EventCapture {
        events: std::sync::Arc<std::sync::Mutex<Vec<(tracing::Level, String)>>>,
    }

    impl<S> tracing_subscriber::Layer<S> for EventCapture
    where
        S: tracing::Subscriber,
    {
        fn on_event(&self, event: &tracing::Event<'_>, _ctx: tracing_subscriber::layer::Context<'_, S>) {
            struct MessageVisitor(String);
            impl tracing::field::Visit for MessageVisitor {
                fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
                    if field.name() == "message" {
                        self.0 = format!("{value:?}");
                    }
                }
            }
            let mut visitor = MessageVisitor(String::new());
            event.record(&mut visitor);
            self.events.lock().unwrap().push((*event.metadata().level(), visitor.0));
        }
    }

    fn warn_messages(capture: &EventCapture) -> Vec<String> {
        capture
            .events
            .lock()
            .unwrap()
            .iter()
            .filter(|(level, _)| *level == tracing::Level::WARN)
            .map(|(_, message)| message.clone())
            .collect()
    }

    /// GH#1785: an image too small for `adaptive` binarization silently fell back to
    /// Otsu with no observable signal, so a caller who asked for `adaptive` had no way
    /// to tell their request was downgraded. `preprocessing.rs:107-121`'s fallback arm
    /// must now emit a WARN naming both the requested and the fallback method.
    #[test]
    fn should_warn_when_adaptive_binarization_falls_back_to_otsu_for_a_small_image() {
        use tracing_subscriber::layer::SubscriberExt as _;

        let capture = EventCapture::default();
        let subscriber = tracing_subscriber::registry().with(capture.clone());

        let small_dimension = ADAPTIVE_TILE_SIZE - 1;
        let config = ImagePreprocessingConfig {
            deskew: false,
            binarization_method: "adaptive".to_string(),
            ..Default::default()
        };
        let pix = Pix::from_raw_rgb(
            &vec![200u8; (small_dimension * small_dimension * 3) as usize],
            small_dimension as u32,
            small_dimension as u32,
        )
        .unwrap();

        let result = tracing::subscriber::with_default(subscriber, || preprocess_pix(pix, &config));

        assert!(result.is_ok(), "the fallback must still produce a usable image");
        let messages = warn_messages(&capture);
        assert!(
            messages
                .iter()
                .any(|message| message.contains("adaptive binarization") && message.contains("Otsu")),
            "expected a WARN naming the adaptive-to-Otsu fallback, got: {messages:?}"
        );
    }

    /// GH#1785: the same silent downgrade for `sauvola` on a small image.
    #[test]
    fn should_warn_when_sauvola_binarization_falls_back_to_otsu_for_a_small_image() {
        use tracing_subscriber::layer::SubscriberExt as _;

        let capture = EventCapture::default();
        let subscriber = tracing_subscriber::registry().with(capture.clone());

        let small_dimension = SAUVOLA_WINDOW_HALF_SIZE * 2 + 2;
        let config = ImagePreprocessingConfig {
            deskew: false,
            binarization_method: "sauvola".to_string(),
            ..Default::default()
        };
        let pix = Pix::from_raw_rgb(
            &vec![200u8; (small_dimension * small_dimension * 3) as usize],
            small_dimension as u32,
            small_dimension as u32,
        )
        .unwrap();

        let result = tracing::subscriber::with_default(subscriber, || preprocess_pix(pix, &config));

        assert!(result.is_ok(), "the fallback must still produce a usable image");
        let messages = warn_messages(&capture);
        assert!(
            messages
                .iter()
                .any(|message| message.contains("Sauvola binarization") && message.contains("Otsu")),
            "expected a WARN naming the Sauvola-to-Otsu fallback, got: {messages:?}"
        );
    }

    /// Negative control for the two tests above: an image large enough for adaptive
    /// binarization must NOT emit the fallback warning, proving the assertion is
    /// actually sensitive to image size rather than passing unconditionally.
    #[test]
    fn should_not_warn_when_adaptive_binarization_succeeds_for_a_large_enough_image() {
        use tracing_subscriber::layer::SubscriberExt as _;

        let capture = EventCapture::default();
        let subscriber = tracing_subscriber::registry().with(capture.clone());

        let large_dimension = ADAPTIVE_TILE_SIZE * 4;
        let config = ImagePreprocessingConfig {
            deskew: false,
            binarization_method: "adaptive".to_string(),
            ..Default::default()
        };
        let pix = Pix::from_raw_rgb(
            &vec![200u8; (large_dimension * large_dimension * 3) as usize],
            large_dimension as u32,
            large_dimension as u32,
        )
        .unwrap();

        let result = tracing::subscriber::with_default(subscriber, || preprocess_pix(pix, &config));

        assert!(result.is_ok());
        let messages = warn_messages(&capture);
        assert!(
            !messages.iter().any(|message| message.contains("falling back to Otsu")),
            "a large-enough image must not trigger the small-image fallback warning: {messages:?}"
        );
    }

    /// GH#1837: `normalize_shaded_rows` is silently inert when binarization is off and
    /// `contrast_enhance` is on -- `preprocess_pix`'s non-binarized contrast arm drops the
    /// normalized grayscale and reconverts from the original `pix`. The option must say so rather
    /// than letting a caller believe it took effect.
    #[test]
    fn should_warn_when_normalize_shaded_rows_cannot_act_on_the_non_binarized_contrast_path() {
        use tracing_subscriber::layer::SubscriberExt as _;

        let capture = EventCapture::default();
        let subscriber = tracing_subscriber::registry().with(capture.clone());

        let config = ImagePreprocessingConfig {
            deskew: false,
            binarization_method: "none".to_string(),
            contrast_enhance: true,
            normalize_shaded_rows: true,
            ..Default::default()
        };
        let pix = Pix::from_raw_rgb(&vec![200u8; 64 * 64 * 3], 64, 64).unwrap();

        let result = tracing::subscriber::with_default(subscriber, || preprocess_pix(pix, &config));

        assert!(
            result.is_ok(),
            "the inert combination must still produce a usable image"
        );
        let messages = warn_messages(&capture);
        assert!(
            messages
                .iter()
                .any(|message| message.contains("normalize_shaded_rows") && message.contains("no effect")),
            "expected a WARN naming the inert normalize_shaded_rows request, got: {messages:?}"
        );
    }

    /// Negative control for the test above, so a WARN emitted unconditionally would be caught: the
    /// same option on the default (Otsu) path does act, and must stay quiet.
    #[test]
    fn should_not_warn_when_normalize_shaded_rows_runs_on_the_binarized_path() {
        use tracing_subscriber::layer::SubscriberExt as _;

        let capture = EventCapture::default();
        let subscriber = tracing_subscriber::registry().with(capture.clone());

        let config = ImagePreprocessingConfig {
            deskew: false,
            normalize_shaded_rows: true,
            ..Default::default()
        };
        let pix = Pix::from_raw_rgb(&vec![200u8; 64 * 64 * 3], 64, 64).unwrap();

        let result = tracing::subscriber::with_default(subscriber, || preprocess_pix(pix, &config));

        assert!(result.is_ok());
        let messages = warn_messages(&capture);
        assert!(
            !messages.iter().any(|message| message.contains("normalize_shaded_rows")),
            "the binarized path does apply the option and must not report it as inert: {messages:?}"
        );
    }

    /// The predicate the WARN is driven by, across the four combinations that matter. Asserted
    /// directly so the reporting condition is pinned independently of whether a given raster
    /// happens to route through the arm in question.
    #[test]
    fn shaded_row_normalization_is_inert_only_without_binarization_and_with_contrast_enhance() {
        let inert = ImagePreprocessingConfig {
            binarization_method: "none".to_string(),
            contrast_enhance: true,
            normalize_shaded_rows: true,
            ..Default::default()
        };
        assert!(shaded_row_normalization_is_inert(&inert, false));
        // Binarized: the normalized grayscale survives into `processed`.
        assert!(!shaded_row_normalization_is_inert(&inert, true));
        // No contrast enhancement: the `should_invert`/passthrough arms keep `gray`.
        assert!(!shaded_row_normalization_is_inert(
            &ImagePreprocessingConfig {
                contrast_enhance: false,
                ..inert.clone()
            },
            false
        ));
        // Never requested at all: nothing to report.
        assert!(!shaded_row_normalization_is_inert(
            &ImagePreprocessingConfig {
                normalize_shaded_rows: false,
                ..inert
            },
            false
        ));
    }
}

#[cfg(all(feature = "pdf", feature = "tokio-runtime", not(target_arch = "wasm32")))]
pub(crate) mod geometry_evidence {
    use std::sync::Arc;

    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Observation {
        Absent,
        ScaleOnly,
        Refused,
    }

    #[derive(Clone, Copy, Debug)]
    pub(crate) struct WordInventory {
        pub(crate) count: usize,
        pub(crate) skipped: usize,
    }

    enum InventoryObservation {
        NotRequested,
        Absent,
        Recorded(WordInventory),
        Refused,
    }

    struct ReadEvidence {
        geometry: Observation,
        inventory: InventoryObservation,
    }

    #[derive(Clone)]
    pub(crate) struct GeometryEvidence(Arc<parking_lot::Mutex<ReadEvidence>>);

    tokio::task_local! {
        static REQUEST: GeometryEvidence;
    }

    pub(crate) async fn observe<F: std::future::Future>(read: F) -> (F::Output, bool) {
        let (result, scale_only, _) = observe_read(read, InventoryObservation::NotRequested).await;
        (result, scale_only)
    }

    pub(crate) async fn observe_with_word_inventory<F: std::future::Future>(
        read: F,
    ) -> (F::Output, bool, Option<WordInventory>) {
        observe_read(read, InventoryObservation::Absent).await
    }

    async fn observe_read<F: std::future::Future>(
        read: F,
        inventory: InventoryObservation,
    ) -> (F::Output, bool, Option<WordInventory>) {
        let request = GeometryEvidence(Arc::new(parking_lot::Mutex::new(ReadEvidence {
            geometry: Observation::Absent,
            inventory,
        })));
        let result = REQUEST.scope(request.clone(), read).await;
        let state = request.0.lock();
        let inventory = match state.inventory {
            InventoryObservation::Recorded(inventory) => Some(inventory),
            _ => None,
        };
        (result, state.geometry == Observation::ScaleOnly, inventory)
    }

    pub(crate) fn record_word_inventory(inventory: WordInventory) {
        let _ = REQUEST.try_with(|request| {
            let mut state = request.0.lock();
            state.inventory = match state.inventory {
                InventoryObservation::Absent => InventoryObservation::Recorded(inventory),
                InventoryObservation::Recorded(_) => InventoryObservation::Refused,
                InventoryObservation::NotRequested => InventoryObservation::NotRequested,
                InventoryObservation::Refused => InventoryObservation::Refused,
            };
        });
    }

    impl GeometryEvidence {
        pub(crate) fn requested() -> Option<Self> {
            REQUEST.try_with(Clone::clone).ok()
        }

        pub(crate) fn on_worker<T>(self, run: impl FnOnce() -> T) -> T {
            REQUEST.sync_scope(self, run)
        }
    }

    // Leptonica pixDeskew returns a reference-counted clone for a no-op.
    // Any distinct Pix, error, or repeated observation is insufficient evidence.
    pub(super) fn record(same_pix: bool) {
        let _ = REQUEST.try_with(|request| {
            let mut current = request.0.lock();
            current.geometry = if same_pix && current.geometry == Observation::Absent {
                Observation::ScaleOnly
            } else {
                Observation::Refused
            };
        });
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[tokio::test]
        async fn inventory_is_kept_only_for_one_authoritative_worker_read() {
            for reads in 0..=2 {
                let (_, scale_only, inventory) = observe_with_word_inventory(async {
                    let request = GeometryEvidence::requested().unwrap();
                    tokio::task::spawn_blocking(move || {
                        request.on_worker(|| {
                            record(true);
                            for _ in 0..reads {
                                record_word_inventory(WordInventory { count: 513, skipped: 0 });
                            }
                        });
                    })
                    .await
                    .unwrap();
                })
                .await;
                assert!(scale_only);
                assert_eq!(
                    inventory.map(|inventory| (inventory.count, inventory.skipped)),
                    (reads == 1).then_some((513, 0))
                );
                assert!(GeometryEvidence::requested().is_none());
            }
        }

        #[tokio::test]
        async fn only_one_positive_deskew_observation_can_certify_the_selected_worker_read() {
            for (observations, expected) in [
                (&[][..], false),
                (&[true][..], true),
                (&[false][..], false),
                (&[true, true][..], false),
                (&[false, true][..], false),
            ] {
                let (_, certified) = observe(async {
                    let request = GeometryEvidence::requested().unwrap();
                    tokio::task::spawn_blocking(move || {
                        request.on_worker(|| observations.iter().copied().for_each(record));
                    })
                    .await
                    .unwrap();
                })
                .await;
                assert_eq!(certified, expected);
                assert!(GeometryEvidence::requested().is_none());
            }
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod gray_route_tests {
    use super::*;

    fn contrast_config(invert_colors: bool) -> ImagePreprocessingConfig {
        ImagePreprocessingConfig {
            binarization_method: "none".to_owned(),
            contrast_enhance: true,
            deskew: false,
            denoise: false,
            normalize_shaded_rows: false,
            invert_colors,
            ..Default::default()
        }
    }

    /// Gray text-like strokes over a gradient, 160x120.
    fn gray_page(dark_background: bool) -> Vec<u8> {
        let mut rgb = Vec::new();
        for y in 0..120u32 {
            for x in 0..160u32 {
                let ink = (x / 6 + y / 5) % 4 == 0 && (x % 6) < 3;
                let base = 150 + (x / 4) as u8;
                let value = if ink { 40 + (y % 7) as u8 } else { base };
                let value = if dark_background { 255 - value } else { value };
                rgb.extend([value; 3]);
            }
        }
        rgb
    }

    #[test]
    fn gray_route_is_pixel_identical_to_the_colour_path_for_gray_input() {
        for (dark, invert) in [(false, false), (true, false), (false, true)] {
            let rgb = gray_page(dark);
            assert!(rgb_is_gray(&rgb));
            let config = contrast_config(invert);
            let coloured = preprocess_pix(Pix::from_raw_rgb(&rgb, 160, 120).unwrap(), &config).expect("colour path");
            let mut owned = rgb.clone();
            let gray = preprocess_gray_contrast(&mut owned, 160, 120, &config).expect("gray route");
            assert!(owned.is_empty(), "the RGB is freed");
            assert_eq!(
                gray.grayscale_bytes().unwrap(),
                coloured.grayscale_bytes().unwrap(),
                "dark={dark} invert={invert}"
            );
        }
    }

    #[test]
    fn a_tinted_raster_is_not_gray_and_keeps_the_colour_path() {
        let mut rgb = gray_page(false);
        rgb[1] = rgb[1].wrapping_add(1);
        assert!(!rgb_is_gray(&rgb));
    }

    #[test]
    fn a_failed_step_keeps_the_best_raster_already_built() {
        let rgb = gray_page(false);
        let plane: Vec<u8> = rgb.as_chunks::<3>().0.iter().map(|pixel| pixel[0]).collect();
        let build = || Pix::from_grayscale_bytes(&plane, 160, 120).unwrap();
        let failing = || -> xberg_tesseract::Result<Pix> { Err(xberg_tesseract::TesseractError::NullPointerError) };
        let kept = enhance_gray_steps(build(), false, |_| failing(), |_| failing());
        assert_eq!(
            kept.grayscale_bytes().unwrap(),
            plane,
            "both steps failed: the gray Pix survives"
        );
        let normalized = build().background_normalize().unwrap().grayscale_bytes().unwrap();
        let after_sharpen_failure = enhance_gray_steps(build(), false, Pix::background_normalize, |_| failing());
        assert_eq!(after_sharpen_failure.grayscale_bytes().unwrap(), normalized);
        assert_ne!(normalized, plane, "the fixture must exercise normalization");
    }
}
