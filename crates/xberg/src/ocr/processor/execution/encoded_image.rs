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

pub(super) fn prepare_encoded_ocr_image(
    bytes: &[u8],
    limits: &SecurityLimits,
    config: &TesseractConfig,
    images: Option<&ImageExtractionConfig>,
    ci_debug_enabled: bool,
) -> crate::Result<PreparedOcrImage> {
    let preparation = Preparation {
        bytes,
        limits,
        config,
        images,
        ci_debug_enabled,
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
        let planned = self.config.preprocessing.clone().unwrap_or_default();
        let dpi_config = ocr_dpi_config(&planned, self.images);
        load_image_for_ocr_with_processing(
            self.bytes,
            self.limits,
            |scale| self.scale_fits(scale, assessment),
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
        )
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

    fn known_native_fits(
        &self,
        dimensions: (u32, u32),
        preprocessing: Option<&ImagePreprocessingConfig>,
    ) -> crate::Result<bool> {
        Ok(
            native_image_budget::known_phase_peak(dimensions.0, dimensions.1, self.bytes.len(), preprocessing)?
                .is_none_or(|peak| peak <= self.limits.max_content_size as u64),
        )
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
        let dpi = self.candidate_dpi(&decoded, policy)?;
        let native_dimensions = if decoded.scale_denominator == 1
            && let Some(preprocessing) = policy
        {
            self.normalized_dimensions((width, height), dpi, preprocessing)
                .unwrap_or((width, height))
        } else {
            (width, height)
        };
        if self.native_needs_retry(native_dimensions, policy)? {
            return Ok(retry_candidate(decoded, policy));
        }
        self.check_reduced_dpi(&decoded, dpi, policy)?;
        if self.resize_needs_retry(&decoded, dpi, policy)? {
            return Ok(retry_candidate(decoded, policy));
        }
        self.finish_candidate(decoded, dpi, policy).map(Candidate::Ready)
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

    fn native_needs_retry(
        &self,
        dimensions: (u32, u32),
        preprocessing: Option<&ImagePreprocessingConfig>,
    ) -> crate::Result<bool> {
        if self.known_native_fits(dimensions, preprocessing)? {
            return Ok(false);
        }
        if !self.is_jpeg() {
            return Err(self.native_error("OCR native image workspace"));
        }
        Ok(true)
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
            return Err(self.native_error("OCR preprocessing and native image workspace"));
        }
        Ok(prepared)
    }

    fn native_error(&self, phase: &str) -> crate::XbergError {
        crate::XbergError::validation(format!(
            "{phase} exceeds security_limits.max_content_size ({} bytes). \
             Raise security_limits.max_content_size or lower target_dpi",
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
