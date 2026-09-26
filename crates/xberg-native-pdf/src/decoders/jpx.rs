//! JPEG 2000 (`/JPXDecode`) image decoding via hayro-jpeg2000.
//!
//! ISO 32000-1 §7.4.9: a `/JPXDecode` stream is a JPEG 2000 codestream — either a
//! raw J2K codestream or a JP2-boxed file. hayro-jpeg2000 handles both. This decodes
//! the codestream to interleaved 8-bit-per-component samples; the caller maps the
//! component count to a colour space and applies `/Decode`, `/SMask`, etc.

use crate::error::{Error, Result};

/// Pass-through filter for `/JPXDecode`.
///
/// Like `DCTDecode`/`JBIG2Decode`, the JPEG 2000 codestream is not decompressed
/// by the generic filter pipeline — it is handed to the image extractor, which
/// decodes it with hayro-jpeg2000 (`decode_jpx`). So this decoder returns its input
/// unchanged, so the pipeline can surface the codestream; the extractor then
/// decodes it.
pub struct JpxDecoder;

impl super::StreamDecoder for JpxDecoder {
    // `max_output_bytes` (GH#1764) is ignored: this is a pass-through, so output
    // never exceeds input length. ~keep
    fn decode(&self, input: &[u8], _max_output_bytes: usize) -> Result<Vec<u8>> {
        Ok(input.to_vec())
    }

    fn name(&self) -> &str {
        "JPXDecode"
    }
}

/// A decoded JPEG 2000 image: interleaved 8-bit samples plus component count.
pub struct JpxImage {
    /// `width * height * num_components` bytes, component-interleaved (row-major).
    pub samples: Vec<u8>,
    pub num_components: u8,
}

/// Decode a JP2/J2K codestream to interleaved 8-bit-per-component samples.
///
/// hayro-jpeg2000 yields one f32 plane per component (normalized to the component's
/// bit depth); `DecodedImage::data_u8()` interleaves these to 8-bit samples.
/// Components are assumed to share the image dimensions (no chroma subsampling) —
/// the common case for PDF image XObjects; a subsampled component is rejected with a
/// typed error rather than producing misaligned output.
pub fn decode_jpx(bytes: &[u8]) -> Result<JpxImage> {
    use hayro_jpeg2000::{DecodeSettings, DecoderContext, Image};

    let image = Image::new(bytes, &DecodeSettings::default())
        .map_err(|e| Error::UnsupportedFilter(format!("JPXDecode: JPEG 2000 decode failed: {e:?}")))?;

    let width = image.width();
    let height = image.height();
    let npix = width as usize * height as usize;

    let mut ctx = DecoderContext::default();
    let decoded = image
        .decode(&mut ctx)
        .map_err(|e| Error::UnsupportedFilter(format!("JPXDecode: JPEG 2000 decode failed: {e:?}")))?;

    let comps = decoded.components();
    if comps.is_empty() {
        return Err(Error::UnsupportedFilter(
            "JPXDecode: JPEG 2000 image has no components".to_string(),
        ));
    }
    let num_components = comps.len();

    // ~keep An alpha channel is a component like any other in the codestream, so counting
    // components raw reports a colour space the image does not have: RGBA reads as four
    // components and was mapped to DeviceCMYK, and grey-with-alpha reads as two and was
    // rejected outright, dropping the image (GH#1850). `Image::has_alpha` is the codestream's
    // own answer, so no channel-count guessing is needed. The alpha is dropped rather than
    // returned as a soft mask: the caller maps `colour_components` straight onto a
    // `PixelFormat`, and carrying transparency through to an `/SMask` is a separate feature.
    let has_alpha = image.has_alpha() && num_components > 1;
    let colour_components = if has_alpha { num_components - 1 } else { num_components };

    // Fast path: every component is full-resolution (the common case) → use the
    // decoder's own interleave. ~keep
    if comps.iter().all(|c| c.samples().len() == npix) {
        let mut samples = decoded.data_u8();
        if has_alpha {
            samples = drop_last_channel(&samples, num_components);
        }
        return Ok(JpxImage {
            samples,
            num_components: colour_components as u8,
        });
    }

    // Chroma-subsampled path (WS1.7). hayro-jpeg2000 0.4 does not expose
    // per-component dimensions, so only the unambiguous 2×2 (4:2:0) case is
    // recovered: a component with ⌈w/2⌉·⌈h/2⌉ samples is nearest-upsampled to
    // full resolution; any other ratio (or non-8-bit depth, where the f32→u8
    // scaling would differ) stays unsupported rather than guessing. Components
    // are then interleaved manually since `data_u8` assumes equal plane sizes. ~keep
    let (w, h) = (width as usize, height as usize);
    let (sw, sh) = (width.div_ceil(2) as usize, height.div_ceil(2) as usize);
    let mut planes: Vec<Vec<u8>> = Vec::with_capacity(num_components);
    for (ci, comp) in comps.iter().enumerate() {
        if comp.bit_depth() != 8 {
            return Err(Error::UnsupportedFilter(format!(
                "JPXDecode: subsampled component {ci} with {}-bit depth not supported",
                comp.bit_depth()
            )));
        }
        let s = comp.samples();
        let plane = if s.len() == npix {
            s.iter().map(|&v| v.round().clamp(0.0, 255.0) as u8).collect()
        } else if s.len() == sw * sh {
            upsample_nearest_u8(s, sw, sh, w, h)
        } else {
            return Err(Error::UnsupportedFilter(format!(
                "JPXDecode: subsampled component {ci} ({} samples) — only 2×2 (4:2:0) \
                 subsampling of a {width}×{height} image is supported",
                s.len()
            )));
        };
        planes.push(plane);
    }

    // The alpha plane, when present, is the last one and is simply not interleaved. ~keep
    let mut samples = vec![0u8; npix * colour_components];
    for (ci, plane) in planes.iter().take(colour_components).enumerate() {
        for (i, &px) in plane.iter().enumerate() {
            samples[i * colour_components + ci] = px;
        }
    }
    Ok(JpxImage {
        samples,
        num_components: colour_components as u8,
    })
}

/// Drop the last channel of a component-interleaved buffer, narrowing it from `stride`
/// channels per pixel to `stride - 1`.
fn drop_last_channel(samples: &[u8], stride: usize) -> Vec<u8> {
    debug_assert!(stride > 1, "narrowing a single-channel buffer would leave nothing");
    let kept = stride - 1;
    let mut out = Vec::with_capacity(samples.len() / stride * kept);
    for pixel in samples.chunks_exact(stride) {
        out.extend_from_slice(&pixel[..kept]);
    }
    out
}

/// Nearest-neighbour upsample of an `sw×sh` f32 sample plane to `fw×fh` u8.
fn upsample_nearest_u8(sub: &[f32], sw: usize, sh: usize, fw: usize, fh: usize) -> Vec<u8> {
    let mut out = vec![0u8; fw * fh];
    for y in 0..fh {
        let sy = (y * sh / fh).min(sh.saturating_sub(1));
        for x in 0..fw {
            let sx = (x * sw / fw).min(sw.saturating_sub(1));
            out[y * fw + x] = sub[sy * sw + sx].round().clamp(0.0, 255.0) as u8;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{decode_jpx, upsample_nearest_u8};

    /// Grayscale JP2 codestream from the minimal repro (816x1056 DeviceGray).
    const SAMPLE_JP2: &[u8] = include_bytes!("../../tests/fixtures/jpx/sample_gray.jp2");

    /// GH#1850 fixtures, 16x16 lossless, generated with Pillow's OpenJPEG encoder. The left
    /// half is opaque `(200, 100, 50)` and the right half is `(10, 220, 90)` at alpha 128, so a
    /// test can tell a dropped alpha channel from a shifted one. ~keep
    const RGBA_JP2: &[u8] = include_bytes!("../../tests/fixtures/jpx/gh1850_rgba.jp2");
    /// The same image as a bare codestream (no JP2 container), which reaches a different
    /// header path in the decoder.
    const RGBA_J2K: &[u8] = include_bytes!("../../tests/fixtures/jpx/gh1850_rgba.j2k");
    /// Greyscale plus alpha: two components, which the unfixed decoder rejected outright.
    const GREY_ALPHA_JP2: &[u8] = include_bytes!("../../tests/fixtures/jpx/gh1850_grey_alpha.jp2");

    /// GH#1850: an alpha channel is a component like any other in the codestream, so counting
    /// components raw described a colour space the image does not have. Four components were
    /// mapped to DeviceCMYK by the caller, and two were rejected outright, dropping the image.
    #[test]
    fn rgba_codestream_reports_three_colour_components() {
        for (label, bytes) in [("jp2", RGBA_JP2), ("j2k", RGBA_J2K)] {
            let img = decode_jpx(bytes).unwrap_or_else(|e| panic!("{label} must decode: {e:?}"));
            assert_eq!(
                img.num_components, 3,
                "{label}: the alpha channel must not be counted as a colour component"
            );
            assert_eq!(
                img.samples.len(),
                16 * 16 * 3,
                "{label}: samples must be RGB-interleaved"
            );
            // Pixel (0,0) is the opaque half; alpha must be gone, not shifted into a channel.
            assert_eq!(&img.samples[..3], &[200, 100, 50], "{label}: first pixel must stay RGB");
        }
    }

    /// The two-component case the decoder rejected, so the image never reached the page at all.
    #[test]
    fn grey_plus_alpha_codestream_decodes_as_single_channel_grey() {
        let img = decode_jpx(GREY_ALPHA_JP2).expect("grey+alpha must decode rather than be dropped");
        assert_eq!(img.num_components, 1, "alpha must not be counted as a colour component");
        assert_eq!(img.samples.len(), 16 * 16, "samples must be one channel per pixel");
        assert_eq!(img.samples[0], 180, "the left half's grey value must survive");
    }

    /// The narrowing helper on its own, so a failure above points at the decoder rather than
    /// at the interleave arithmetic.
    #[test]
    fn drop_last_channel_narrows_each_pixel() {
        let rgba = [1u8, 2, 3, 4, 11, 12, 13, 14];
        assert_eq!(super::drop_last_channel(&rgba, 4), vec![1, 2, 3, 11, 12, 13]);
        let la = [7u8, 255, 9, 128];
        assert_eq!(super::drop_last_channel(&la, 2), vec![7, 9]);
    }

    /// WS1.7: nearest-neighbour upsample of a 2×2 subsampled plane to 4×4 —
    /// each source sample fills its 2×2 output block.
    #[test]
    fn upsample_nearest_2x2_to_4x4() {
        let sub = [10.0f32, 20.0, 30.0, 40.0];
        let out = upsample_nearest_u8(&sub, 2, 2, 4, 4);
        assert_eq!(
            out,
            vec![10, 10, 20, 20, 10, 10, 20, 20, 30, 30, 40, 40, 30, 30, 40, 40,]
        );
    }

    /// Odd full dimensions (⌈w/2⌉ source): upsample 2×2 → 3×3 clamps at edges.
    #[test]
    fn upsample_nearest_2x2_to_3x3() {
        let sub = [1.0f32, 2.0, 3.0, 4.0];
        let out = upsample_nearest_u8(&sub, 2, 2, 3, 3);
        assert_eq!(out.len(), 9);
        assert_eq!(out[0], 1);
        assert_eq!(out[8], 4);
    }

    #[test]
    fn decode_jpx_grayscale() {
        let img = decode_jpx(SAMPLE_JP2).expect("decode JP2 codestream");

        assert_eq!(img.num_components, 1);
        assert_eq!(img.samples.len(), 816 * 1056);

        // A scanned page is not one flat value. ~keep
        let first = img.samples[0];
        assert!(
            img.samples.iter().any(|&b| b != first),
            "decoded image is uniformly flat — decode likely failed"
        );
    }
}
