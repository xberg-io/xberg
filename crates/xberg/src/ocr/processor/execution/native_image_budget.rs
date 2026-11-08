use crate::types::ImagePreprocessingConfig;

mod binarized;

const RGB_CHANNELS: u64 = 3;
const PIX_WORD_BITS: u64 = 32;
const PIX_WORD_BYTES: u64 = 4;
const PIX_HEADER_BYTES: u64 = (10 * 4 + 3 * size_of::<usize>() as u64).next_multiple_of(size_of::<usize>() as u64);
const BACKGROUND_REDUCTION: u32 = 4;
const MORPH_BORDER: u32 = 32;

/// Known live image-buffer phases in the pinned Tesseract/Leptonica ingestion path.
/// This excludes recognizer/model allocations and aggregate process RSS. Unmodelled
/// policies retain full-resolution behavior but cannot qualify a reduced JPEG candidate.
pub(super) fn known_phase_peak(
    width: u32,
    height: u32,
    encoded_bytes: usize,
    preprocessing: Option<&ImagePreprocessingConfig>,
) -> crate::Result<Option<u64>> {
    phase_peak(width, height, encoded_bytes, preprocessing, false)
}

/// [`known_phase_peak`] for a raster whose channels are equal in every pixel: the contrast path then
/// takes the 8-bit route (`preprocess_gray_contrast`), which frees the RGB. Other policies are unchanged.
pub(super) fn known_gray_phase_peak(
    width: u32,
    height: u32,
    encoded_bytes: usize,
    preprocessing: Option<&ImagePreprocessingConfig>,
) -> crate::Result<Option<u64>> {
    phase_peak(width, height, encoded_bytes, preprocessing, true)
}

fn phase_peak(
    width: u32,
    height: u32,
    encoded_bytes: usize,
    preprocessing: Option<&ImagePreprocessingConfig>,
    gray_input: bool,
) -> crate::Result<Option<u64>> {
    if preprocessing.is_some_and(|options| !no_binarization_contrast(options) && !plain_binarization(options)) {
        return Ok(None);
    }
    let gray_route = gray_input && preprocessing.is_some_and(releases_source_rgb);
    calculate_phase_peak(width, height, encoded_bytes, preprocessing, gray_route)
        .map(Some)
        .ok_or_else(|| crate::XbergError::validation("OCR native image workspace calculation overflowed"))
}

fn calculate_phase_peak(
    width: u32,
    height: u32,
    encoded_bytes: usize,
    preprocessing: Option<&ImagePreprocessingConfig>,
    gray_route: bool,
) -> Option<u64> {
    let rgba = pix_bytes(width, height, 32)?;
    // SetImage(raw) creates a 32-bpp Pix and SetImage(Pix) copies it. ~keep
    let raw_native = rgba.checked_mul(2)?;
    let rgb = u64::from(width)
        .checked_mul(u64::from(height))?
        .checked_mul(RGB_CHANNELS)?;
    if gray_route {
        return gray_route_peak(width, height, rgb, rgba, encoded_bytes);
    }
    let native = match preprocessing {
        None => raw_native,
        Some(options) if no_binarization_contrast(options) => {
            let gray = pix_bytes(width, height, 8)?.max(pix_bytes(height, width, 8)?);
            // enhance_non_binarized keeps source32 and normalized32. The third-channel
            // pixUnsharpMaskingGray peak adds two prior sharpened8 outputs, input8,
            // convolution8, result8 and a 32-bpp accumulator (enhance.c).
            let sharpening = rgba.checked_mul(3)?.checked_add(gray.checked_mul(5)?)?;
            let map_width = width.div_ceil(BACKGROUND_REDUCTION).checked_add(MORPH_BORDER)?;
            let map_height = height.div_ceil(BACKGROUND_REDUCTION).checked_add(MORPH_BORDER)?;
            let maps = pix_bytes(map_width, map_height, 16)?
                .max(pix_bytes(map_height, map_width, 16)?)
                .checked_mul(8)?;
            sharpening.checked_add(maps)?.max(raw_native)
        }
        Some(options) if plain_binarization(options) => binarized::plain_binarization_peak(width, height, options)?,
        Some(_) => return None,
    };
    // Retain the encoded source and allow another encoded-size allocation, matching
    // decoder admission. Pix headers are included separately from row-aligned pixels.
    native
        .checked_add(rgb)?
        .checked_add((encoded_bytes as u64).checked_mul(2)?)
}

/// The peak of `preprocess_gray_contrast`: the RGB is read into a gray plane and an 8-bpp Pix, then freed;
/// the Pix is background-normalized (maps beside input and output) and sharpened, which keeps its input,
/// first-pass output and convolution beside a 32-bpp block-convolution accumulator (enhance.c).
fn gray_route_peak(width: u32, height: u32, rgb: u64, rgba: u64, encoded_bytes: usize) -> Option<u64> {
    let gray = pix_bytes(width, height, 8)?.max(pix_bytes(height, width, 8)?);
    let creation = rgb.checked_add(gray.checked_mul(2)?)?;
    let map_width = width.div_ceil(BACKGROUND_REDUCTION).checked_add(MORPH_BORDER)?;
    let map_height = height.div_ceil(BACKGROUND_REDUCTION).checked_add(MORPH_BORDER)?;
    let maps = pix_bytes(map_width, map_height, 16)?
        .max(pix_bytes(map_height, map_width, 16)?)
        .checked_mul(8)?;
    let normalization = gray.checked_mul(2)?.checked_add(maps)?;
    let sharpening = gray.checked_mul(3)?.checked_add(rgba)?;
    creation
        .max(normalization)
        .max(sharpening)
        .checked_add((encoded_bytes as u64).checked_mul(2)?)
}

/// The native (Leptonica) part of [`gray_route_peak`], for the measurement test.
#[cfg(test)]
pub(super) fn gray_route_native_peak(width: u32, height: u32) -> Option<u64> {
    let gray = pix_bytes(width, height, 8)?.max(pix_bytes(height, width, 8)?);
    let rgba = pix_bytes(width, height, 32)?;
    let map_width = width.div_ceil(BACKGROUND_REDUCTION).checked_add(MORPH_BORDER)?;
    let map_height = height.div_ceil(BACKGROUND_REDUCTION).checked_add(MORPH_BORDER)?;
    let maps = pix_bytes(map_width, map_height, 16)?
        .max(pix_bytes(map_height, map_width, 16)?)
        .checked_mul(8)?;
    gray.max(gray.checked_mul(2)?.checked_add(maps)?)
        .max(gray.checked_mul(3)?.checked_add(rgba)?)
        .into()
}

/// Whether the non-binarized contrast policy has a gray route. The route applies only to a raster whose
/// channels are equal in every pixel; the caller checks that.
pub(super) fn releases_source_rgb(options: &ImagePreprocessingConfig) -> bool {
    no_binarization_contrast(options)
}

fn plain_binarization(options: &ImagePreprocessingConfig) -> bool {
    matches!(
        options.binarization_method.to_ascii_lowercase().as_str(),
        "otsu" | "adaptive"
    ) && !options.auto_rotate
        && !options.denoise
        && !options.contrast_enhance
        && !options.normalize_shaded_rows
}

fn no_binarization_contrast(options: &ImagePreprocessingConfig) -> bool {
    matches!(
        options.binarization_method.to_ascii_lowercase().as_str(),
        "none" | "off"
    ) && options.contrast_enhance
        && !options.denoise
        && !options.normalize_shaded_rows
}

fn pix_bytes(width: u32, height: u32, depth: u64) -> Option<u64> {
    let row_words = u64::from(width).checked_mul(depth)?.div_ceil(PIX_WORD_BITS);
    row_words
        .checked_mul(PIX_WORD_BYTES)?
        .checked_mul(u64::from(height))?
        .checked_add(PIX_HEADER_BYTES)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_phase_uses_word_aligned_native_rows_and_retains_the_encoded_input() {
        assert_eq!(
            known_phase_peak(1, 3, 7, None).expect("addressable dimensions"),
            Some(2 * (12 + PIX_HEADER_BYTES) + 9 + 14),
        );
    }

    #[test]
    fn raw_phase_boundary_keeps_the_finest_safe_full_or_reduced_raster() {
        let (width, height) = (512, 256);
        let rgb = vec![0; width as usize * height as usize * 3];
        let bytes = crate::extraction::image::jpeg_fixture::baseline_rgb_jpeg(&rgb, width, height);
        let peak = known_phase_peak(u32::from(width), u32::from(height), bytes.len(), None)
            .unwrap()
            .unwrap();
        for (budget, expected_width) in [(peak, u32::from(width)), (peak - 1, u32::from(width) / 2)] {
            let prepared = super::super::prepare_encoded_ocr_image(
                &bytes,
                &super::super::SecurityLimits {
                    max_content_size: budget as usize,
                    ..Default::default()
                },
                &super::super::TesseractConfig::default(),
                None,
                false,
            )
            .expect("a phase-safe candidate exists");
            assert_eq!(prepared.width, expected_width);
            assert!(!prepared.apply_pix_preprocessing);
        }
    }

    #[test]
    fn implicit_bright_scan_keeps_its_distinct_preprocessing_policy() {
        let dimension = 4096;
        let rgb = vec![245; dimension as usize * dimension as usize * 3];
        let bytes = crate::extraction::image::jpeg_fixture::baseline_rgb_jpeg(&rgb, dimension, dimension);
        let config = super::super::TesseractConfig {
            source_dpi: Some(600.0),
            known_full_page_scan: true,
            ..Default::default()
        };
        let prepared = super::super::prepare_encoded_ocr_image(
            &bytes,
            &super::super::SecurityLimits {
                max_content_size: 40 * 1024 * 1024,
                ..Default::default()
            },
            &config,
            None,
            false,
        )
        .expect("implicit bright-page preprocessing must not inherit raw-ingestion selection");
        assert_eq!(
            (prepared.width, prepared.height, prepared.source_dpi),
            (2048, 2048, 300)
        );
        assert!(prepared.apply_pix_preprocessing);
        assert!(prepared.image_preprocessing.expect("DPI metadata").skipped_resize);
    }

    #[test]
    fn unsupported_policy_and_arithmetic_overflow_are_distinct() {
        assert!(
            known_phase_peak(
                1,
                1,
                0,
                Some(&ImagePreprocessingConfig {
                    denoise: true,
                    ..Default::default()
                })
            )
            .expect("unsupported policy")
            .is_none()
        );
        assert!(known_phase_peak(u32::MAX, u32::MAX, 0, None).is_err());
    }

    #[test]
    fn default_otsu_phase_retains_its_40_mib_result_but_exceeds_28_mib() {
        let options = ImagePreprocessingConfig::default();
        let peak = known_phase_peak(2048, 2048, 655360, Some(&options))
            .expect("addressable default phase")
            .expect("global Otsu and default deskew are modelled");
        assert!(peak > 28 * 1024 * 1024);
        assert!(peak <= 40 * 1024 * 1024);
        let inverted = ImagePreprocessingConfig {
            invert_colors: true,
            ..options
        };
        assert_eq!(
            known_phase_peak(2048, 2048, 655360, Some(&inverted)).unwrap(),
            Some(peak)
        );
    }

    #[test]
    fn gray_contrast_route_frees_the_rgb_and_is_charged_for_the_eight_bit_raster() {
        let (width, height) = (3000, 4000);
        let options = ImagePreprocessingConfig {
            contrast_enhance: true,
            binarization_method: "none".to_owned(),
            denoise: false,
            normalize_shaded_rows: false,
            ..Default::default()
        };
        assert!(releases_source_rgb(&options));
        let gray = pix_bytes(width, height, 8).unwrap();
        let rgba = pix_bytes(width, height, 32).unwrap();
        let rgb = u64::from(width) * u64::from(height) * RGB_CHANNELS;
        let colour = known_phase_peak(width, height, 1000, Some(&options)).unwrap().unwrap();
        let on_gray = known_gray_phase_peak(width, height, 1000, Some(&options))
            .unwrap()
            .unwrap();
        assert_eq!(on_gray, (rgb + 2 * gray).max(3 * gray + rgba) + 2000);
        assert!(on_gray < colour / 2, "{on_gray} vs {colour}");
        let binarized = ImagePreprocessingConfig::default();
        assert_eq!(
            known_gray_phase_peak(width, height, 1000, Some(&binarized)).unwrap(),
            known_phase_peak(width, height, 1000, Some(&binarized)).unwrap(),
            "only the contrast policy has a gray route",
        );
    }

    #[test]
    fn contrast_phase_exceeds_40_mib_but_fits_96_mib_at_the_required_dimensions() {
        let options = ImagePreprocessingConfig {
            contrast_enhance: true,
            binarization_method: "none".to_owned(),
            denoise: false,
            normalize_shaded_rows: false,
            ..Default::default()
        };
        let peak = known_phase_peak(2048, 2048, 655360, Some(&options))
            .expect("addressable phase")
            .expect("known contrast phase");
        assert!(peak > 40 * 1024 * 1024);
        assert!(peak <= 96 * 1024 * 1024);
    }
}
