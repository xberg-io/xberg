use super::budgeted_downscale::{self, SourceDpi};
use super::{
    PreparedOcrImage, SecurityLimits, TesseractConfig, dpi_workspace, log_ci_debug, native_image_budget,
    ocr_dpi_config, ocr_preprocessing_decision, prepare_decided_ocr_image, resolve_known_source_dpi,
};
use crate::core::config::ImageExtractionConfig;
use crate::extraction::image::{DecodedOcrImage, ReducedScale, load_image_for_ocr_with_processing};
use crate::image::preprocessing::{normalization_requires_resize, normalized_image_dimensions};
use crate::types::ImagePreprocessingConfig;

const JPEG_PREPARATION_CANDIDATES: usize = 4;

struct Preparation<'a> {
    bytes: &'a [u8],
    limits: &'a SecurityLimits,
    config: &'a TesseractConfig,
    images: Option<&'a ImageExtractionConfig>,
    ci_debug_enabled: bool,
    allow_bounded_inferred_upscale: bool,
    /// Whether the candidate being prepared is a genuinely gray raster on the gray contrast path, so the
    /// workspace model charges that path's own peak. Reset when a candidate is decoded. ~keep
    gray_input: std::cell::Cell<bool>,
}

struct Assessment {
    // None has no pixel-selected policy yet; Some(None) is confirmed raw ingestion. ~keep
    policy: Option<Option<ImagePreprocessingConfig>>,
    maximum_pixels: u64,
}

enum Candidate {
    Ready(PreparedOcrImage),
    Retry(Assessment),
}

#[cfg(test)]
pub(super) fn prepare_encoded_ocr_image(
    bytes: &[u8],
    limits: &SecurityLimits,
    config: &TesseractConfig,
    images: Option<&ImageExtractionConfig>,
    ci_debug_enabled: bool,
) -> crate::Result<PreparedOcrImage> {
    prepare_with_dpi_policy(bytes, limits, config, images, ci_debug_enabled, false)
}

pub(super) fn prepare_with_dpi_policy(
    bytes: &[u8],
    limits: &SecurityLimits,
    config: &TesseractConfig,
    images: Option<&ImageExtractionConfig>,
    ci_debug_enabled: bool,
    allow_bounded_inferred_upscale: bool,
) -> crate::Result<PreparedOcrImage> {
    let preparation = Preparation {
        bytes,
        limits,
        config,
        images,
        ci_debug_enabled,
        // A caller-selected preprocessing policy keeps its requested raster budget; a policy
        // the engine chose may fall back to the largest upscale that fits. ~keep
        allow_bounded_inferred_upscale: allow_bounded_inferred_upscale || config.preprocessing.is_none(),
        gray_input: std::cell::Cell::new(false),
    };
    let mut assessment = Assessment {
        policy: config
            .preprocessing
            .clone()
            .map(Some)
            .or_else(|| config.known_full_page_scan.then(|| Some(Default::default()))),
        maximum_pixels: u64::MAX,
    };
    // Full, half, quarter and eighth are the only JPEG output candidates. ~keep
    for _ in 0..JPEG_PREPARATION_CANDIDATES {
        let decoded = preparation.decode(&assessment)?;
        match preparation.prepare_candidate(decoded)? {
            Candidate::Ready(image) => return Ok(image),
            Candidate::Retry(next) => assessment = next,
        }
    }
    Err(crate::XbergError::validation(format!(
        "JPEG image workspace exceeds security_limits.max_content_size ({} bytes). Raise security_limits.max_content_size",
        limits.max_content_size,
    )))
}

impl Preparation<'_> {
    fn is_jpeg(&self) -> bool {
        self.bytes.starts_with(&[0xff, 0xd8])
    }

    fn source_dpi(&self, width: u32, height: u32) -> Option<f64> {
        resolve_known_source_dpi(self.config.source_dpi, self.bytes, width, height)
    }

    fn decode(&self, assessment: &Assessment) -> crate::Result<DecodedOcrImage> {
        self.gray_input.set(false);
        let planned = self.config.preprocessing.clone().unwrap_or_default();
        let dpi_config = ocr_dpi_config(&planned, self.images);
        let rejected_policy = std::cell::Cell::new(false);
        let result = load_image_for_ocr_with_processing(
            self.bytes,
            self.limits,
            |scale| {
                if scale.denominator != 1
                    && assessment.policy.as_ref().is_some_and(|policy| {
                        native_image_budget::known_phase_peak(
                            scale.width,
                            scale.height,
                            self.bytes.len(),
                            policy.as_ref(),
                        )
                        .is_ok_and(|peak| peak.is_none())
                    })
                {
                    rejected_policy.set(true);
                    return false;
                }
                self.scale_fits(scale, assessment)
            },
            |scale| {
                self.source_dpi(scale.original_width, scale.original_height)
                    .is_some_and(|dpi| {
                        !normalization_requires_resize(
                            scale.width,
                            scale.height,
                            &dpi_config,
                            dpi / f64::from(scale.denominator),
                        )
                    })
            },
            |scale| self.prefer_reduced(scale, assessment),
        );
        if result.is_err() && rejected_policy.get() {
            return Err(self.unsupported_reduced_policy());
        }
        result
    }

    fn scale_fits(&self, scale: ReducedScale, assessment: &Assessment) -> bool {
        let pixels = u64::from(scale.width) * u64::from(scale.height);
        pixels <= assessment.maximum_pixels
            && assessment.policy.as_ref().is_none_or(|policy| {
                let dimensions = if scale.denominator == 1
                    && let Some(preprocessing) = policy.as_ref()
                {
                    self.normalized_dimensions(
                        (scale.width, scale.height),
                        self.source_dpi(scale.original_width, scale.original_height),
                        preprocessing,
                    )
                } else {
                    Some((scale.width, scale.height))
                };
                dimensions.is_none_or(|dimensions| self.known_native_fits(dimensions, policy.as_ref()).unwrap_or(false))
            })
    }

    fn prefer_reduced(&self, scale: ReducedScale, assessment: &Assessment) -> bool {
        assessment.policy.as_ref().is_some_and(|policy| {
            self.known_native_fits((scale.width, scale.height), policy.as_ref())
                .is_ok_and(|fits| !fits)
        })
    }

    fn normalized_dimensions(
        &self,
        dimensions: (u32, u32),
        dpi: Option<f64>,
        preprocessing: &ImagePreprocessingConfig,
    ) -> Option<(u32, u32)> {
        dpi.map(|dpi| {
            normalized_image_dimensions(
                dimensions.0,
                dimensions.1,
                &ocr_dpi_config(preprocessing, self.images),
                Some(dpi),
            )
        })
    }

    fn phase_peak(
        &self,
        width: u32,
        height: u32,
        preprocessing: Option<&ImagePreprocessingConfig>,
    ) -> crate::Result<Option<u64>> {
        if self.gray_input.get() {
            native_image_budget::known_gray_phase_peak(width, height, self.bytes.len(), preprocessing)
        } else {
            native_image_budget::known_phase_peak(width, height, self.bytes.len(), preprocessing)
        }
    }

    fn known_native_fits(
        &self,
        dimensions: (u32, u32),
        preprocessing: Option<&ImagePreprocessingConfig>,
    ) -> crate::Result<bool> {
        Ok(self
            .phase_peak(dimensions.0, dimensions.1, preprocessing)?
            .is_none_or(|peak| peak <= self.limits.max_content_size as u64))
    }

    fn prepare_candidate(&self, decoded: DecodedOcrImage) -> crate::Result<Candidate> {
        let (width, height) = decoded.image.dimensions();
        log_ci_debug(self.ci_debug_enabled, "image", || {
            format!("dimensions={width}x{height} color_type=RGB8")
        });
        let preprocessing = ocr_preprocessing_decision(
            decoded.image.as_raw(),
            width,
            height,
            self.config.preprocessing.as_ref(),
            self.config.known_full_page_scan,
        );
        let policy = preprocessing.as_deref();
        self.gray_input.set(
            policy.is_some_and(native_image_budget::releases_source_rgb)
                && super::super::super::preprocessing::rgb_is_gray(decoded.image.as_raw()),
        );
        self.check_reduced_policy(&decoded, policy)?;
        let dpi = match self.candidate_dpi(&decoded, policy) {
            Ok(dpi) => dpi,
            Err(error) => return self.recover_downscale(decoded, policy, None, error),
        };
        let native_dimensions = if decoded.scale_denominator == 1
            && let Some(preprocessing) = policy
        {
            self.normalized_dimensions((width, height), dpi, preprocessing)
                .unwrap_or((width, height))
        } else {
            (width, height)
        };
        if let (Some(dpi), Some(policy)) = (dpi, policy)
            && let Some(target_dpi) = self.budgeted_upscale(&decoded, native_dimensions, dpi, policy)?
        {
            return self
                .finish_budgeted_upscale(decoded, dpi, policy, target_dpi)
                .map(Candidate::Ready);
        }
        if !self.known_native_fits(native_dimensions, policy)? {
            if !self.is_jpeg() {
                let error = self.native_error("OCR native image workspace", false);
                // A source that fits may still require its explicit DPI normalization. ~keep
                if !self.known_native_fits((width, height), policy)? {
                    return self.recover_downscale(decoded, policy, dpi, error);
                }
                return Err(error);
            }
            self.check_unknown_dpi_upscale(&decoded, native_dimensions, dpi, policy)?;
            return Ok(retry_candidate(decoded, policy));
        }
        self.check_reduced_dpi(&decoded, dpi, policy)?;
        if self.resize_needs_retry(&decoded, dpi, policy)? {
            return Ok(retry_candidate(decoded, policy));
        }
        self.finish_candidate(decoded, dpi, policy).map(Candidate::Ready)
    }

    /// PNG and TIFF always may. A full-size JPEG may only when the engine chose the policy and the
    /// resolution is inferred: the caller's requested upscale keeps its refusal, and a reduced
    /// decode has no inferred resolution of the original to scale. ~keep
    fn can_shrink_upscale(&self, decoded: &DecodedOcrImage) -> bool {
        match image::guess_format(self.bytes) {
            Ok(image::ImageFormat::Png | image::ImageFormat::Tiff) => true,
            Ok(image::ImageFormat::Jpeg) => {
                self.allow_bounded_inferred_upscale
                    && decoded.scale_denominator == 1
                    && self
                        .source_dpi(decoded.original_width, decoded.original_height)
                        .is_none()
            }
            _ => false,
        }
    }

    fn budgeted_upscale(
        &self,
        decoded: &DecodedOcrImage,
        normalized: (u32, u32),
        source_dpi: f64,
        policy: &ImagePreprocessingConfig,
    ) -> crate::Result<Option<i32>> {
        let (width, height) = decoded.image.dimensions();
        if !source_dpi.is_finite()
            || source_dpi <= 0.0
            || normalized.0 <= width
            || normalized.1 <= height
            || !self.can_shrink_upscale(decoded)
            || (!self.allow_bounded_inferred_upscale && self.source_dpi(width, height).is_none())
            || self.known_native_fits(normalized, Some(policy))?
        {
            return Ok(None);
        }
        let mut dpi_config = ocr_dpi_config(policy, self.images);
        dpi_config.auto_adjust_dpi = false;
        let source = decoded.image.as_raw();
        let source_extra = source.capacity().saturating_sub(source.len());
        let resize_limit = self.limits.max_content_size.saturating_sub(source_extra);
        if self
            .phase_peak(width, height, Some(policy))?
            .is_none_or(|peak| peak > resize_limit as u64)
        {
            return Ok(None);
        }
        let lower = source_dpi.ceil() as i32;
        let Some(upper) = dpi_workspace::largest_unclamped_upscale(
            lower,
            policy.target_dpi,
            width.max(height),
            source_dpi,
            dpi_config.max_image_dimension,
        )?
        else {
            return Ok(None);
        };
        dpi_workspace::largest_feasible_upscale(lower, upper, |target| {
            dpi_config.target_dpi = target;
            let dimensions = normalized_image_dimensions(width, height, &dpi_config, Some(source_dpi));
            let native_peak = self.phase_peak(dimensions.0, dimensions.1, Some(policy))?;
            let native_limit = if dimensions == (width, height) {
                resize_limit
            } else {
                self.limits.max_content_size
            };
            Ok(dimensions.0 <= normalized.0
                && dimensions.1 <= normalized.1
                && native_peak.is_some_and(|peak| peak <= native_limit as u64)
                && dpi_workspace::check_known_dpi_resize(
                    width,
                    height,
                    source_dpi,
                    &dpi_config,
                    self.bytes.len(),
                    resize_limit,
                )
                .is_ok())
        })
    }

    fn finish_budgeted_upscale(
        &self,
        decoded: DecodedOcrImage,
        source_dpi: f64,
        policy: &ImagePreprocessingConfig,
        target_dpi: i32,
    ) -> crate::Result<PreparedOcrImage> {
        let images = ImageExtractionConfig {
            auto_adjust_dpi: false,
            ..self.images.cloned().unwrap_or_default()
        };
        let bounded_policy = ImagePreprocessingConfig {
            target_dpi,
            ..policy.clone()
        };
        let (width, height) = decoded.image.dimensions();
        let mut prepared = prepare_decided_ocr_image(
            decoded.image.into_raw(),
            width,
            height,
            Some(&bounded_policy),
            Some(&images),
            self.ci_debug_enabled,
            Some(source_dpi),
        );
        let extra_capacity = prepared.data.capacity().saturating_sub(prepared.data.len()) as u64;
        let peak = self.phase_peak(prepared.width, prepared.height, prepared.preprocessing.as_ref())?;
        if peak
            .and_then(|peak| peak.checked_add(extra_capacity))
            .is_none_or(|peak| peak > self.limits.max_content_size as u64)
        {
            return Err(self.native_error("OCR preprocessing and native image workspace", false));
        }
        if let Some(metadata) = &mut prepared.image_preprocessing {
            if metadata.skipped_resize {
                prepared.source_dpi = source_dpi.round() as i32;
                metadata.final_dpi = prepared.source_dpi;
                metadata.scale_factor = 1.0;
            }
            metadata.target_dpi = policy.target_dpi;
            metadata.calculated_dpi = Some(metadata.final_dpi);
            metadata.auto_adjusted = true;
        }
        Ok(prepared)
    }

    fn check_unknown_dpi_upscale(
        &self,
        decoded: &DecodedOcrImage,
        normalized: (u32, u32),
        dpi: Option<f64>,
        policy: Option<&ImagePreprocessingConfig>,
    ) -> crate::Result<()> {
        let (Some(dpi), Some(policy)) = (dpi, policy) else {
            return Ok(());
        };
        let (width, height) = decoded.image.dimensions();
        if decoded.scale_denominator != 1
            || (normalized.0 <= width && normalized.1 <= height)
            || self.source_dpi(width, height).is_some()
            || !self.known_native_fits((width, height), Some(policy))?
        {
            return Ok(());
        }
        // A reduced decode cannot reuse the inferred DPI of the original raster. ~keep
        dpi_workspace::check_known_dpi_resize(
            width,
            height,
            dpi,
            &ocr_dpi_config(policy, self.images),
            self.bytes.len(),
            self.limits.max_content_size,
        )?;
        Err(self.native_error("OCR native image workspace after DPI resize", true))
    }

    fn check_reduced_policy(
        &self,
        decoded: &DecodedOcrImage,
        preprocessing: Option<&ImagePreprocessingConfig>,
    ) -> crate::Result<()> {
        if decoded.scale_denominator != 1
            && native_image_budget::known_phase_peak(
                decoded.image.width(),
                decoded.image.height(),
                self.bytes.len(),
                preprocessing,
            )?
            .is_none()
        {
            return Err(self.unsupported_reduced_policy());
        }
        Ok(())
    }

    fn unsupported_reduced_policy(&self) -> crate::XbergError {
        crate::XbergError::validation(format!(
            "The requested preprocessing policy has no bounded native image workspace for reduced JPEG recovery \
             within security_limits.max_content_size ({} bytes). Use a supported preprocessing policy \
             or raise security_limits.max_content_size to process the original image",
            self.limits.max_content_size,
        ))
    }

    fn candidate_dpi(
        &self,
        decoded: &DecodedOcrImage,
        preprocessing: Option<&ImagePreprocessingConfig>,
    ) -> crate::Result<Option<f64>> {
        let dpi = self
            .source_dpi(decoded.original_width, decoded.original_height)
            .map(|dpi| dpi / f64::from(decoded.scale_denominator));
        if decoded.scale_denominator != 1 || dpi.is_some() {
            return Ok(dpi);
        }
        let Some(preprocessing) = preprocessing else {
            return Ok(dpi);
        };
        let retained = self
            .bytes
            .len()
            .checked_mul(2)
            .and_then(|encoded| encoded.checked_add(decoded.image.as_raw().capacity()));
        let available = retained
            .and_then(|retained| self.limits.max_content_size.checked_sub(retained))
            .ok_or_else(|| {
                crate::XbergError::validation(
                    "OCR image DPI estimation workspace exceeds security_limits.max_content_size",
                )
            })?;
        crate::image::preprocessing::bounded_unknown_raster_dpi(
            decoded.image.as_raw(),
            decoded.image.width() as usize,
            decoded.image.height() as usize,
            preprocessing.target_dpi,
            available as u64,
        )
        .map(Some)
    }

    fn recover_downscale(
        &self,
        decoded: DecodedOcrImage,
        preprocessing: Option<&ImagePreprocessingConfig>,
        dpi: Option<f64>,
        error: crate::XbergError,
    ) -> crate::Result<Candidate> {
        budgeted_downscale::recover(
            decoded.image,
            self.bytes,
            self.limits,
            preprocessing,
            self.images,
            dpi.map_or(SourceDpi::Unknown, SourceDpi::Known),
        )?
        .map(Candidate::Ready)
        .ok_or(error)
    }

    fn check_reduced_dpi(
        &self,
        decoded: &DecodedOcrImage,
        dpi: Option<f64>,
        preprocessing: Option<&ImagePreprocessingConfig>,
    ) -> crate::Result<()> {
        let Some(preprocessing) = preprocessing.filter(|_| decoded.scale_denominator != 1) else {
            return Ok(());
        };
        let (width, height) = decoded.image.dimensions();
        let dpi_config = ocr_dpi_config(preprocessing, self.images);
        if !dpi.is_none_or(|dpi| normalization_requires_resize(width, height, &dpi_config, dpi)) {
            return Ok(());
        }
        // An unknown source DPI leaves only the limit as a remedy for a standalone image. ~keep
        let (reason, remedy) = dpi.map_or_else(
            || (", and its source DPI is unknown, so", String::new()),
            |dpi| (", and", format!(", or set target_dpi to {}", dpi.round())),
        );
        Err(crate::XbergError::validation(format!(
            "The image exceeds security_limits.max_content_size ({} bytes) at its source resolution{reason} a \
             reduced-size image cannot be resized to target_dpi {} within that limit. Raise \
             security_limits.max_content_size{remedy}",
            self.limits.max_content_size, dpi_config.target_dpi,
        )))
    }

    fn resize_needs_retry(
        &self,
        decoded: &DecodedOcrImage,
        dpi: Option<f64>,
        preprocessing: Option<&ImagePreprocessingConfig>,
    ) -> crate::Result<bool> {
        let (Some(preprocessing), Some(dpi)) = (preprocessing, dpi) else {
            return Ok(false);
        };
        let (width, height) = decoded.image.dimensions();
        let dpi_config = ocr_dpi_config(preprocessing, self.images);
        let Err(error) = dpi_workspace::check_known_dpi_resize(
            width,
            height,
            dpi,
            &dpi_config,
            self.bytes.len(),
            self.limits.max_content_size,
        ) else {
            return Ok(false);
        };
        if self.is_jpeg()
            && dpi_workspace::has_no_resize_reduction(width, height, dpi, &dpi_config, decoded.scale_denominator)
        {
            Ok(true)
        } else {
            Err(error)
        }
    }

    fn finish_candidate(
        &self,
        decoded: DecodedOcrImage,
        dpi: Option<f64>,
        preprocessing: Option<&ImagePreprocessingConfig>,
    ) -> crate::Result<PreparedOcrImage> {
        let (width, height) = decoded.image.dimensions();
        let prepared = prepare_decided_ocr_image(
            decoded.image.into_raw(),
            width,
            height,
            preprocessing,
            self.images,
            self.ci_debug_enabled,
            dpi,
        );
        if !self.known_native_fits((prepared.width, prepared.height), prepared.preprocessing.as_ref())? {
            return Err(self.native_error("OCR preprocessing and native image workspace", false));
        }
        Ok(prepared)
    }

    fn native_error(&self, phase: &str, has_planned_resize: bool) -> crate::XbergError {
        let hint = if has_planned_resize { " or lower target_dpi" } else { "" };
        crate::XbergError::validation(format!(
            "{phase} exceeds security_limits.max_content_size ({} bytes). \
             Raise security_limits.max_content_size{hint}",
            self.limits.max_content_size,
        ))
    }
}

fn retry_candidate(decoded: DecodedOcrImage, preprocessing: Option<&ImagePreprocessingConfig>) -> Candidate {
    let maximum_pixels = (u64::from(decoded.image.width()) * u64::from(decoded.image.height())).saturating_sub(1);
    let policy = Some(preprocessing.cloned());
    // Release the rejected raster before the caller starts another decode. ~keep
    drop(decoded);
    Candidate::Retry(Assessment { policy, maximum_pixels })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ocr::preprocessing::rgb_is_gray;

    enum ExpectedImage {
        Original,
        Upscaled,
        WorkspaceError,
    }

    fn png_fixture(pixel: [u8; 3], expected_gray: bool) -> Vec<u8> {
        let image = image::RgbImage::from_pixel(200, 100, image::Rgb(pixel));
        assert_eq!(rgb_is_gray(image.as_raw()), expected_gray, "fixture {pixel:?}");
        let mut encoded = std::io::Cursor::new(Vec::new());
        image.write_to(&mut encoded, image::ImageFormat::Png).unwrap();
        encoded.into_inner()
    }

    fn contrast_policy() -> ImagePreprocessingConfig {
        ImagePreprocessingConfig {
            contrast_enhance: true,
            binarization_method: "none".into(),
            deskew: false,
            ..Default::default()
        }
    }

    fn prepare_png(bytes: &[u8], policy: &ImagePreprocessingConfig, budget: usize) -> crate::Result<PreparedOcrImage> {
        prepare_encoded_ocr_image(
            bytes,
            &SecurityLimits {
                max_content_size: budget,
                ..Default::default()
            },
            &TesseractConfig {
                source_dpi: Some(100.0),
                preprocessing: Some(policy.clone()),
                ..Default::default()
            },
            Some(&ImageExtractionConfig {
                auto_adjust_dpi: false,
                ..Default::default()
            }),
            false,
        )
    }

    fn assert_budget_cases(bytes: &[u8], policy: &ImagePreprocessingConfig, source_peak: usize, target_peak: usize) {
        for (name, budget, expected) in [
            ("source_at_peak", source_peak, ExpectedImage::Original),
            ("source_below_peak", source_peak - 1, ExpectedImage::WorkspaceError),
            ("target_at_peak", target_peak, ExpectedImage::Upscaled),
        ] {
            let result = prepare_png(bytes, policy, budget);
            let (dimensions, scale, skipped) = match expected {
                ExpectedImage::Original => ((200, 100, 100), 1.0, true),
                ExpectedImage::Upscaled => ((600, 300, 300), 3.0, false),
                ExpectedImage::WorkspaceError => {
                    let error = result
                        .err()
                        .unwrap_or_else(|| panic!("{name}: expected a workspace refusal"));
                    assert!(
                        matches!(&error, crate::XbergError::Validation { message, .. }
                            if message == &format!("OCR native image workspace exceeds security_limits.max_content_size \
                                ({budget} bytes). Raise security_limits.max_content_size")),
                        "{name}: {error}"
                    );
                    continue;
                }
            };
            let image = result.unwrap_or_else(|error| panic!("{name}: {error}"));
            assert_eq!((image.width, image.height, image.source_dpi), dimensions, "{name}");
            let metadata = image
                .image_preprocessing
                .unwrap_or_else(|| panic!("{name}: missing DPI metadata"));
            assert_eq!(metadata.target_dpi, 300, "{name}");
            assert_eq!(metadata.final_dpi, dimensions.2, "{name}");
            assert_eq!(metadata.scale_factor, scale, "{name}");
            assert_eq!(metadata.skipped_resize, skipped, "{name}");
        }
    }

    #[test]
    fn budgeted_png_without_resize_keeps_the_real_source_resolution() {
        let bytes = png_fixture([245; 3], true);
        let policy = contrast_policy();
        let source_peak = native_image_budget::known_gray_phase_peak(200, 100, bytes.len(), Some(&policy))
            .unwrap()
            .unwrap() as usize;
        let target_peak = native_image_budget::known_gray_phase_peak(600, 300, bytes.len(), Some(&policy))
            .unwrap()
            .unwrap() as usize;
        assert_budget_cases(&bytes, &policy, source_peak, target_peak);
    }

    #[test]
    fn budgeted_colour_png_uses_the_colour_workspace_boundary() {
        let bytes = png_fixture([245, 245, 244], false);
        let policy = contrast_policy();
        let source_peak = native_image_budget::known_phase_peak(200, 100, bytes.len(), Some(&policy))
            .unwrap()
            .unwrap() as usize;
        let target_peak = native_image_budget::known_phase_peak(600, 300, bytes.len(), Some(&policy))
            .unwrap()
            .unwrap() as usize;
        assert_budget_cases(&bytes, &policy, source_peak, target_peak);
    }
}
