use super::{PreparedOcrImage, SecurityLimits, dpi_workspace, native_image_budget, ocr_dpi_config};
use crate::core::config::ImageExtractionConfig;
use crate::image::preprocessing::inferred_raster_dpi;
use crate::image::text_height::glyph_workspace_bound;
use crate::types::{ImagePreprocessingConfig, ImagePreprocessingMetadata};

pub(super) fn recover(
    image: image::RgbImage,
    encoded: &[u8],
    limits: &SecurityLimits,
    policy: Option<&ImagePreprocessingConfig>,
    images: Option<&ImageExtractionConfig>,
) -> crate::Result<Option<PreparedOcrImage>> {
    let Some(policy) = policy else {
        return Ok(None);
    };
    if !matches!(
        image::guess_format(encoded),
        Ok(image::ImageFormat::Png | image::ImageFormat::Tiff)
    ) {
        return Ok(None);
    }
    let dpi = ocr_dpi_config(policy, images);
    if !dpi.auto_adjust_dpi {
        return Ok(None);
    }

    let source = image.dimensions();
    let limit = limits.max_content_size as u64;
    let budget = DownscaleBudget {
        source,
        source_extra: image.as_raw().capacity().saturating_sub(image.as_raw().len()) as u64,
        encoded: encoded.len(),
        limit,
        policy,
    };
    let Some(destination) = budget.finest_frame(dpi.max_image_dimension)? else {
        return Ok(None);
    };

    let scale = f64::from(destination.0.max(destination.1)) / f64::from(source.0.max(source.1));
    let data =
        crate::image::resize::resize_rgb(image.as_raw(), source.0, source.1, destination.0, destination.1, scale)?;
    drop(image);
    let extra = data.capacity().saturating_sub(data.len()) as u64;
    if !budget.admits_after_resize(destination, data.capacity() as u64, extra)? {
        return Ok(None);
    }
    let Some(final_dpi) = inferred_raster_dpi(&data, destination.0 as usize, destination.1 as usize, policy.target_dpi)
    else {
        return Ok(None);
    };
    let dimension_clamped = budget.dimension_clamps(destination, dpi.max_image_dimension)?;
    Ok(Some(prepared(
        data,
        source,
        destination,
        policy,
        final_dpi,
        dimension_clamped,
    )))
}

struct DownscaleBudget<'a> {
    source: (u32, u32),
    source_extra: u64,
    encoded: usize,
    limit: u64,
    policy: &'a ImagePreprocessingConfig,
}

impl DownscaleBudget<'_> {
    fn finest_frame(&self, max_dimension: i32) -> crate::Result<Option<(u32, u32)>> {
        let long = self.source.0.max(self.source.1);
        let short = self.source.0.min(self.source.1);
        if short < 2 || max_dimension <= 0 {
            return Ok(None);
        }

        let mut low = long.div_ceil(short);
        let mut high = (long - 1).min(max_dimension as u32);
        let mut best = None;
        while low <= high {
            let target = low + (high - low) / 2;
            let frame = self.proportional_frame(target);
            if self.admits(frame)? {
                best = Some(frame);
                low = target + 1;
            } else {
                high = target - 1;
            }
        }
        Ok(best)
    }

    fn proportional_frame(&self, long_side: u32) -> (u32, u32) {
        let (width, height) = self.source;
        let short = (u64::from(width.min(height)) * u64::from(long_side) / u64::from(width.max(height))) as u32;
        if width >= height {
            (long_side, short)
        } else {
            (short, long_side)
        }
    }

    fn dimension_clamps(&self, frame: (u32, u32), max_dimension: i32) -> crate::Result<bool> {
        let longest = frame.0.max(frame.1);
        let next = longest + 1;
        if longest != max_dimension as u32 || next >= self.source.0.max(self.source.1) {
            return Ok(false);
        }
        self.admits(self.proportional_frame(next))
    }

    fn admits(&self, (width, height): (u32, u32)) -> crate::Result<bool> {
        let resize = dpi_workspace::downscale_peak(self.source.0, self.source.1, width, height, self.encoded)
            .and_then(|peak| peak.checked_add(self.source_extra));
        if resize.is_none_or(|peak| peak > self.limit) {
            return Ok(false);
        }
        let Some(rgb_bytes) = u64::from(width)
            .checked_mul(u64::from(height))
            .and_then(|pixels| pixels.checked_mul(3))
        else {
            return Ok(false);
        };
        self.admits_after_resize((width, height), rgb_bytes, 0)
    }

    fn admits_after_resize(
        &self,
        (width, height): (u32, u32),
        rgb_capacity: u64,
        extra_capacity: u64,
    ) -> crate::Result<bool> {
        let retained = (self.encoded as u64)
            .checked_mul(2)
            .and_then(|bytes| bytes.checked_add(rgb_capacity));
        let Some(available) = retained.and_then(|retained| self.limit.checked_sub(retained)) else {
            return Ok(false);
        };
        Ok(
            glyph_workspace_bound(width as usize, height as usize).is_some_and(|peak| peak > 0 && peak <= available)
                && native_image_budget::known_phase_peak(width, height, self.encoded, Some(self.policy))?
                    .and_then(|peak| peak.checked_add(extra_capacity))
                    .is_some_and(|peak| peak <= self.limit),
        )
    }
}

fn prepared(
    data: Vec<u8>,
    source: (u32, u32),
    destination: (u32, u32),
    policy: &ImagePreprocessingConfig,
    estimated_dpi: f64,
    dimension_clamped: bool,
) -> PreparedOcrImage {
    let (width, height) = destination;
    let final_dpi = estimated_dpi.round() as i32;
    let original_dpi = (
        estimated_dpi * f64::from(source.0) / f64::from(width),
        estimated_dpi * f64::from(source.1) / f64::from(height),
    );
    PreparedOcrImage {
        data,
        width,
        height,
        source_dpi: final_dpi,
        apply_pix_preprocessing: true,
        preprocessing: Some(policy.clone()),
        image_preprocessing: Some(ImagePreprocessingMetadata {
            original_dimensions: (source.0 as usize, source.1 as usize).into(),
            original_dpi: original_dpi.into(),
            target_dpi: policy.target_dpi,
            scale_factor: f64::from(width.max(height)) / f64::from(source.0.max(source.1)),
            auto_adjusted: true,
            final_dpi,
            new_dimensions: Some((width as usize, height as usize).into()),
            resample_method: "LANCZOS3".into(),
            dimension_clamped,
            calculated_dpi: Some(final_dpi),
            skipped_resize: false,
            resize_error: None,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admits_the_finest_proven_frame_and_rejects_the_next_larger_frame() {
        let policy = ImagePreprocessingConfig::default();
        let budget = DownscaleBudget {
            source: (4000, 3000),
            source_extra: 0,
            encoded: 483_482,
            limit: 100 * 1024 * 1024,
            policy: &policy,
        };
        assert_eq!(budget.finest_frame(4096).unwrap(), Some((3649, 2736)));
        assert!(budget.admits((3649, 2736)).unwrap());
        assert!(!budget.admits((3650, 2737)).unwrap());
        assert_eq!(budget.finest_frame(3200).unwrap(), Some((3200, 2400)));
        assert!(budget.dimension_clamps((3200, 2400), 3200).unwrap());
        assert!(!budget.dimension_clamps((3649, 2736), 3649).unwrap());
    }

    #[test]
    fn charges_original_rgb_capacity_and_rejects_before_any_resize_at_a_low_cap() {
        let policy = ImagePreprocessingConfig::default();
        let frame = (500, 375);
        let resize = dpi_workspace::downscale_peak(1000, 750, frame.0, frame.1, 1024).unwrap();
        let budget = DownscaleBudget {
            source: (1000, 750),
            source_extra: 8_192,
            encoded: 1024,
            limit: resize + 8_191,
            policy: &policy,
        };
        assert!(!budget.admits(frame).unwrap());
        assert!(
            DownscaleBudget {
                source_extra: 0,
                ..budget
            }
            .admits(frame)
            .unwrap()
        );
        assert!(
            DownscaleBudget {
                limit: 2_250_000,
                ..budget
            }
            .finest_frame(4096)
            .unwrap()
            .is_none()
        );
    }

    #[test]
    fn skips_recovery_if_reduced_pixels_do_not_supply_a_glyph_estimate() {
        let policy = ImagePreprocessingConfig::default();
        let image = image::RgbImage::from_pixel(100, 100, image::Rgb([245, 245, 245]));
        let limits = SecurityLimits {
            max_content_size: 80_000,
            ..Default::default()
        };
        let png = b"\x89PNG\r\n\x1a\n";
        let budget = DownscaleBudget {
            source: (100, 100),
            source_extra: 0,
            encoded: png.len(),
            limit: limits.max_content_size as u64,
            policy: &policy,
        };
        assert!(
            budget.finest_frame(4096).unwrap().is_some(),
            "a smaller blank raster fits"
        );
        assert!(recover(image, png, &limits, Some(&policy), None).unwrap().is_none());
    }
    #[test]
    fn explicit_preprocessing_still_reports_the_original_workspace_error() {
        let bytes = include_bytes!("../../../../tests/fixtures/ocr_unknown_dpi_2530/records.png");
        let config = super::super::TesseractConfig {
            preprocessing: Some(ImagePreprocessingConfig::default()),
            ..Default::default()
        };
        let error = super::super::prepare_encoded_ocr_image(bytes, &SecurityLimits::default(), &config, None, false)
            .err()
            .expect("an explicit preprocessing policy must not be replaced by implicit recovery");
        assert!(
            error.to_string().contains("OCR image DPI estimation workspace"),
            "{error}"
        );
    }
}
