//! PNG predictor implementations for PDF stream decoding.
//!
//! PDF streams can use PNG predictors (algorithms 10-15) to improve compression.
//! These predictors encode differences between adjacent pixels, which are then
//! reversed during decoding.

use crate::error::{Error, Result};

/// PNG predictor algorithms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PngPredictor {
    /// No prediction (predictor 10)
    None = 10,
    /// Sub: each byte is the difference from the byte to its left (predictor 11)
    Sub = 11,
    /// Up: each byte is the difference from the byte above (predictor 12)
    Up = 12,
    /// Average: each byte is the difference from the average of left and above (predictor 13)
    Average = 13,
    /// Paeth: uses a complex predictor function (predictor 14)
    Paeth = 14,
    /// Optimum: PNG allows different predictor per row (predictor 15)
    Optimum = 15,
}

/// Decode parameters for stream decoders.
#[derive(Debug, Clone)]
pub struct DecodeParams {
    /// Predictor algorithm (1 = none, 2 = TIFF, 10-15 = PNG)
    pub predictor: i64,
    /// Number of columns (width in samples)
    pub columns: usize,
    /// Number of color components per sample (default 1)
    pub colors: usize,
    /// Bits per component (default 8)
    pub bits_per_component: usize,
}

impl Default for DecodeParams {
    fn default() -> Self {
        Self {
            predictor: 1,
            columns: 1,
            colors: 1,
            bits_per_component: 8,
        }
    }
}

impl DecodeParams {
    /// Calculate the number of bytes per row.
    pub fn bytes_per_row(&self) -> usize {
        // Each row has: 1 byte for predictor tag + (columns * colors * bits_per_component) / 8
        // For PNG predictors (10-15), we need to add 1 for the predictor byte ~keep
        let pixel_bytes = (self.columns * self.colors * self.bits_per_component).div_ceil(8);

        if self.predictor >= 10 {
            pixel_bytes + 1
        } else {
            pixel_bytes
        }
    }

    /// Calculate the number of bytes of actual pixel data per row (without predictor tag).
    pub fn pixel_bytes_per_row(&self) -> usize {
        (self.columns * self.colors * self.bits_per_component).div_ceil(8)
    }

    /// Calculate the number of bytes of actual pixel data per row, validating
    /// the dimensions first.
    ///
    /// `Columns`, `Colors`, and `BitsPerComponent` come straight from an
    /// attacker-controlled `/DecodeParms` dictionary (see
    /// `object.rs::extract_decode_params` and
    /// `xref.rs::extract_decode_params`), which cast `as_integer().unwrap_or(n)
    /// as usize` with no range checks — a declared `-1` silently becomes
    /// `usize::MAX`. This is the single point both construction sites funnel
    /// through before any row-sized `Vec` indexing or `chunks()` call, so it
    /// is where the guard belongs.
    ///
    /// Rejects:
    /// - any of the three fields being zero (a zero row size makes
    ///   `data.chunks(0)` panic downstream, since `usize::is_multiple_of(0)`
    ///   is vacuously true for empty data and so does not catch it)
    /// - a product that overflows `usize` (the `-1 as usize` case above, and
    ///   any other combination large enough to wrap)
    pub fn checked_pixel_bytes_per_row(&self) -> Result<usize> {
        if self.columns == 0 {
            return Err(Error::Decode("Predictor /Columns must be non-zero".to_string()));
        }
        if self.colors == 0 {
            return Err(Error::Decode("Predictor /Colors must be non-zero".to_string()));
        }
        if self.bits_per_component == 0 {
            return Err(Error::Decode(
                "Predictor /BitsPerComponent must be non-zero".to_string(),
            ));
        }

        let sample_bits = self
            .columns
            .checked_mul(self.colors)
            .and_then(|v| v.checked_mul(self.bits_per_component))
            .ok_or_else(|| {
                Error::Decode(format!(
                    "Predictor row size overflow: Columns={} * Colors={} * BitsPerComponent={} exceeds platform limits",
                    self.columns, self.colors, self.bits_per_component
                ))
            })?;

        Ok(sample_bits.div_ceil(8))
    }

    /// Calculate the total bytes per row (including the PNG predictor tag byte
    /// when `predictor >= 10`), validating the dimensions first.
    ///
    /// See [`DecodeParams::checked_pixel_bytes_per_row`] for what is rejected
    /// and why.
    pub fn checked_bytes_per_row(&self) -> Result<usize> {
        let pixel_bytes = self.checked_pixel_bytes_per_row()?;

        if self.predictor >= 10 {
            pixel_bytes
                .checked_add(1)
                .ok_or_else(|| Error::Decode("Predictor row size overflow: PNG tag byte".to_string()))
        } else {
            Ok(pixel_bytes)
        }
    }
}

/// CCITT Group 3/4 Fax decode parameters.
///
/// PDF Spec: ISO 32000-1:2008, Section 7.4.6 - CCITTFaxDecode Filter Parameters
#[derive(Debug, Clone, PartialEq)]
pub struct CcittParams {
    /// Group indicator:
    ///  <0 = Group 4 (pure 2D)
    ///  0 = Group 3 (1-D)
    ///  >0 = Group 3 (2-D with specified K)
    pub k: i64,
    /// Image width in pixels (must match /Columns in DecodeParms)
    pub columns: u32,
    /// Image height in pixels (optional)
    pub rows: Option<u32>,
    /// Pixel interpretation:
    /// false = white is 0, black is 1 (PDF default)
    /// true = white is 1, black is 0 (inverted)
    pub black_is_1: bool,
    /// Include End-of-Line code
    pub end_of_line: bool,
    /// Align compressed data to byte boundaries
    pub encoded_byte_align: bool,
    /// Include Return-to-Control (RTC) code
    /// true = RTC code at end (default)
    /// false = no RTC code
    pub end_of_block: bool,
}

impl Default for CcittParams {
    fn default() -> Self {
        Self {
            k: 0,
            columns: 1,
            rows: None,
            black_is_1: false,
            end_of_line: false,
            encoded_byte_align: false,
            end_of_block: true,
        }
    }
}

impl CcittParams {
    /// Check if this is Group 4 encoding (`K < 0`).
    pub fn is_group_4(&self) -> bool {
        self.k < 0
    }

    /// Check if this is Group 3 encoding
    pub fn is_group_3(&self) -> bool {
        self.k >= 0
    }
}

/// Apply PNG predictor decoding to data.
///
/// PNG predictors encode differences between pixels. This function reverses
/// the prediction to restore the original data.
///
/// # Arguments
///
/// * `data` - The predictor-encoded data
/// * `params` - Decode parameters specifying predictor type and dimensions
///
/// # Returns
///
/// The decoded data with predictors reversed, or an error if decoding fails.
pub fn decode_predictor(data: &[u8], params: &DecodeParams) -> Result<Vec<u8>> {
    match params.predictor {
        1 => Ok(data.to_vec()),
        2 => decode_tiff_predictor(data, params),
        10..=15 => decode_png_predictor(data, params),
        _ => Err(Error::Decode(format!("Unsupported predictor: {}", params.predictor))),
    }
}

/// Decode TIFF Predictor 2.
///
/// TIFF Predictor 2 encodes the difference between adjacent samples in the same row.
fn decode_tiff_predictor(data: &[u8], params: &DecodeParams) -> Result<Vec<u8>> {
    let bytes_per_row = params.checked_pixel_bytes_per_row()?;
    let colors = params.colors;

    // Each row starts with `colors` unchanged bytes (see the push loop below),
    // so a row that is narrower than the declared component count would index
    // past `row_data`'s end. E.g. `/Colors 4 /BitsPerComponent 1 /Columns 1`
    // gives bytes_per_row = 1 but colors = 4. ~keep
    if colors > bytes_per_row {
        return Err(Error::Decode(format!(
            "Predictor /Colors {colors} exceeds the {bytes_per_row}-byte row implied by /Columns and \
             /BitsPerComponent"
        )));
    }

    if !data.len().is_multiple_of(bytes_per_row) {
        return Err(Error::Decode(format!(
            "Data length {} is not a multiple of row size {}",
            data.len(),
            bytes_per_row
        )));
    }

    let mut output = Vec::with_capacity(data.len());

    for row_data in data.chunks(bytes_per_row) {
        // First pixel in row is unchanged ~keep
        for i in 0..colors {
            output.push(row_data[i]);
        }

        // Subsequent pixels: add left neighbor ~keep
        for i in colors..row_data.len() {
            let left = output[output.len() - colors];
            output.push(row_data[i].wrapping_add(left));
        }
    }

    Ok(output)
}

/// Decode PNG predictors (10-15).
///
/// PNG predictors can vary per row (when using predictor 15).
/// Each row starts with a predictor tag byte indicating which algorithm to use.
fn decode_png_predictor(data: &[u8], params: &DecodeParams) -> Result<Vec<u8>> {
    let bytes_per_row = params.checked_bytes_per_row()?;
    let pixel_bytes = params.checked_pixel_bytes_per_row()?;

    if !data.len().is_multiple_of(bytes_per_row) {
        return Err(Error::Decode(format!(
            "Data length {} is not a multiple of row size {}",
            data.len(),
            bytes_per_row
        )));
    }

    let row_count = data.len() / bytes_per_row;
    let output_size = row_count
        .checked_mul(pixel_bytes)
        .ok_or_else(|| Error::Decode("PNG predictor output size overflow".into()))?;
    let mut output = Vec::new();
    output.try_reserve_exact(output_size).map_err(|error| {
        Error::Decode(format!(
            "Failed to reserve {output_size} bytes for PNG predictor output: {error}"
        ))
    })?;
    let bpp = params.colors;

    for row_idx in 0..row_count {
        let row_start = row_idx * bytes_per_row;
        let row_data = &data[row_start..row_start + bytes_per_row];

        // PDF 32000-1:2008 §7.4.4.4: when PNG predictors (10-15) are used on
        // encoding, every row of the filtered data carries an explicit tag
        // byte identifying the per-row algorithm. For decoding, that per-row
        // tag is authoritative regardless of whether /Predictor is 10, 11,
        // 12, 13, 14, or 15 — the numeric /Predictor only signals encoder
        // intent. Honouring the declared predictor instead of the per-row
        // tag byte produces cascade noise on producers that emit tag 0
        // (None) under a non-10 /Predictor value, which the spec permits. ~keep
        let predictor_tag = row_data[0];

        let encoded_pixels = &row_data[1..];

        match predictor_tag {
            0 => {
                output.extend_from_slice(encoded_pixels);
            }
            1 => {
                decode_png_sub(encoded_pixels, &mut output, bpp);
            }
            2 => {
                decode_png_up(encoded_pixels, &mut output, row_idx, pixel_bytes);
            }
            3 => {
                decode_png_average(encoded_pixels, &mut output, row_idx, pixel_bytes, bpp);
            }
            4 => {
                decode_png_paeth(encoded_pixels, &mut output, row_idx, pixel_bytes, bpp);
            }
            _ => {
                return Err(Error::Decode(format!("Invalid PNG predictor tag: {}", predictor_tag)));
            }
        }
    }

    Ok(output)
}

/// PNG Sub predictor: each byte is the difference from the left neighbor.
fn decode_png_sub(encoded: &[u8], output: &mut Vec<u8>, bpp: usize) {
    let start_pos = output.len();

    for (i, &byte) in encoded.iter().enumerate() {
        let left = if i >= bpp { output[start_pos + i - bpp] } else { 0 };
        output.push(byte.wrapping_add(left));
    }
}

/// PNG Up predictor: each byte is the difference from the byte above.
fn decode_png_up(encoded: &[u8], output: &mut Vec<u8>, row_idx: usize, pixel_bytes: usize) {
    for (i, &byte) in encoded.iter().enumerate() {
        let up = if row_idx > 0 {
            output[(row_idx - 1) * pixel_bytes + i]
        } else {
            0
        };
        output.push(byte.wrapping_add(up));
    }
}

/// PNG Average predictor: each byte is the difference from the average of left and above.
fn decode_png_average(encoded: &[u8], output: &mut Vec<u8>, row_idx: usize, pixel_bytes: usize, bpp: usize) {
    let start_pos = output.len();

    for (i, &byte) in encoded.iter().enumerate() {
        let left = if i >= bpp {
            output[start_pos + i - bpp] as u16
        } else {
            0
        };

        let up = if row_idx > 0 {
            output[(row_idx - 1) * pixel_bytes + i] as u16
        } else {
            0
        };

        let avg = ((left + up) / 2) as u8;
        output.push(byte.wrapping_add(avg));
    }
}

/// PNG Paeth predictor: uses the Paeth filter function.
fn decode_png_paeth(encoded: &[u8], output: &mut Vec<u8>, row_idx: usize, pixel_bytes: usize, bpp: usize) {
    let start_pos = output.len();

    for (i, &byte) in encoded.iter().enumerate() {
        let left = if i >= bpp {
            output[start_pos + i - bpp] as i16
        } else {
            0
        };

        let up = if row_idx > 0 {
            output[(row_idx - 1) * pixel_bytes + i] as i16
        } else {
            0
        };

        let up_left = if row_idx > 0 && i >= bpp {
            output[(row_idx - 1) * pixel_bytes + i - bpp] as i16
        } else {
            0
        };

        let paeth = paeth_predictor(left, up, up_left) as u8;
        output.push(byte.wrapping_add(paeth));
    }
}

/// Paeth predictor function from PNG specification.
fn paeth_predictor(a: i16, b: i16, c: i16) -> i16 {
    let p = a + b - c;
    let pa = (p - a).abs();
    let pb = (p - b).abs();
    let pc = (p - c).abs();

    if pa <= pb && pa <= pc {
        a
    } else if pb <= pc {
        b
    } else {
        c
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_predictor() {
        let data = b"Hello, World!";
        let params = DecodeParams {
            predictor: 1,
            ..Default::default()
        };

        let result = decode_predictor(data, &params).unwrap();
        assert_eq!(result, data);
    }

    #[test]
    fn test_png_up_predictor() {
        let params = DecodeParams {
            predictor: 12,
            columns: 5,
            colors: 1,
            bits_per_component: 8,
        };

        // Encoded data: predictor tag (2 for Up) + encoded bytes ~keep
        let encoded = vec![2, 10, 20, 30, 40, 50, 2, 5, 5, 5, 5, 5];

        let result = decode_predictor(&encoded, &params).unwrap();

        assert_eq!(result, vec![10, 20, 30, 40, 50, 15, 25, 35, 45, 55]);
    }

    #[test]
    fn test_bytes_per_row_calculation() {
        let params = DecodeParams {
            predictor: 12,
            columns: 5,
            colors: 1,
            bits_per_component: 8,
        };

        assert_eq!(params.bytes_per_row(), 6);
        assert_eq!(params.pixel_bytes_per_row(), 5);
    }

    #[test]
    fn test_decode_params_default() {
        let params = DecodeParams::default();
        assert_eq!(params.predictor, 1);
        assert_eq!(params.columns, 1);
        assert_eq!(params.colors, 1);
        assert_eq!(params.bits_per_component, 8);
    }

    #[test]
    fn negative_k_values_select_group_4() {
        let params = CcittParams {
            k: -2,
            ..Default::default()
        };
        assert!(params.is_group_4());
        assert!(!params.is_group_3());
    }

    /// A producer may declare `/Predictor 12` and still write tag 0 (None)
    /// on every row. The per-row tag is authoritative for decoding, so
    /// rows must be copied verbatim even when the declared /Predictor
    /// nominally says Up.
    #[test]
    fn test_png_predictor_12_respects_per_row_tag_none() {
        let params = DecodeParams {
            predictor: 12,
            columns: 5,
            colors: 1,
            bits_per_component: 8,
        };
        let encoded = vec![0, 10, 20, 30, 40, 50, 0, 11, 21, 31, 41, 51];
        let result = decode_predictor(&encoded, &params).unwrap();
        assert_eq!(result, vec![10, 20, 30, 40, 50, 11, 21, 31, 41, 51]);
    }

    /// Mixed per-row tags (None on row 0, Up on row 1) under a single
    /// declared /Predictor 12 must still decode correctly. The PDF spec
    /// allows the encoder to pick a different PNG filter per row.
    #[test]
    fn test_png_predictor_12_mixed_per_row_tags() {
        let params = DecodeParams {
            predictor: 12,
            columns: 5,
            colors: 1,
            bits_per_component: 8,
        };
        let encoded = vec![0, 10, 20, 30, 40, 50, 2, 5, 5, 5, 5, 5];
        let result = decode_predictor(&encoded, &params).unwrap();
        assert_eq!(result, vec![10, 20, 30, 40, 50, 15, 25, 35, 45, 55]);
    }

    /// 4-bit-per-component indexed image row decodes by byte; predictor
    /// framing operates at byte granularity regardless of sub-byte sample
    /// packing. Two 710-pixel 4bpc rows → 355 bytes each + 1 tag byte.
    #[test]
    fn test_png_predictor_12_4bpc_tag_none() {
        let params = DecodeParams {
            predictor: 12,
            columns: 710,
            colors: 1,
            bits_per_component: 4,
        };
        assert_eq!(params.pixel_bytes_per_row(), 355);
        assert_eq!(params.bytes_per_row(), 356);

        let mut encoded = Vec::with_capacity(356 * 2);
        encoded.push(0);
        encoded.extend(std::iter::repeat_n(0xFFu8, 355));
        encoded.push(0);
        encoded.extend(std::iter::repeat_n(0xFFu8, 355));

        let result = decode_predictor(&encoded, &params).unwrap();
        // Both rows are pure 0xFF; an Up-cascade on row 1 would wrap to
        // 0xFE (0xFF + 0xFF). The per-row tag=None must suppress that. ~keep
        assert_eq!(result.len(), 710);
        assert!(result.iter().all(|&b| b == 0xFF));
    }
}
