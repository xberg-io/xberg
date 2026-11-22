//! CCITT Group 4 decompression for bilevel images.
//!
//! This module handles decompression of CCITT Group 4 encoded bilevel (1-bit) images
//! extracted from PDF documents, and converts them to 8-bit grayscale for OCR processing.
//!
//! PDF Spec: ISO 32000-1:2008, Section 7.4.6 - CCITTFaxDecode Filter
//! CCITT Spec: ITU-T Recommendation T.6 - Facsimile coding schemes and coding control functions

use crate::decoders::CcittParams;
use crate::error::{Error, Result};

/// Decompresses CCITT encoded data (Group 3 or Group 4).
///
/// CCITT (Consultative Committee for International Telegraphy and Telephony) is a binary
/// compression format used in TIFF and PDF for bilevel (1-bit) images. This is the standard
/// compression for scanned documents.
///
/// # Arguments
///
/// * `data` - CCITT compressed data
/// * `params` - CCITT decompression parameters from PDF /DecodeParms dictionary
///
/// # Returns
///
/// A vector of bytes representing the decompressed bilevel image.
/// Each byte contains 8 pixels (MSB = leftmost pixel, LSB = rightmost pixel).
/// Pixels are encoded as: 0 = white, 1 = black (unless /BlackIs1=true, then inverted).
pub fn decompress_ccitt(data: &[u8], params: &CcittParams) -> Result<Vec<u8>> {
    decompress_ccitt_reporting(data, params).map(|(out, _)| out)
}

/// As [`decompress_ccitt`], but also returns `rows_valid`, the number of leading
/// rows that came from the stream.
///
/// The blank fallback is a substitute, not a decode result: for an `/ImageMask`
/// the all-zero buffer reads as "paint every pixel" under `/Decode [1 0]`, so a
/// caller that must not paint what it could not read overwrites rows at and
/// beyond `rows_valid` with a value that draws nothing under the mask's own
/// `/Decode`.
pub fn decompress_ccitt_reporting(data: &[u8], params: &CcittParams) -> Result<(Vec<u8>, usize)> {
    if params.columns == 0 {
        return Err(Error::Decode(
            "CCITT decompression requires /Columns parameter".to_string(),
        ));
    }

    // `fax` 0.3 takes u32 dimensions, which is what CcittParams already holds:
    // the previous `as u16` narrowing silently truncated /Columns > 65535. ~keep
    let width = params.columns;

    log_ccitt_decompress_start(data, params);

    let fax_result = decode_ccitt_with_fallback(data, width, params);

    match fax_result {
        Ok((mut output, rows_valid)) => {
            if params.black_is_1 {
                invert_bilevel_pixels(&mut output);
            }
            Ok((output, rows_valid))
        }
        Err(e) => {
            // Both decoders failed. Do NOT silently return an all-white page
            // (the old behavior that produced the blank-page bug) — warn loudly
            // and surface a controlled white fallback only as a last resort so
            // the failure is visible in logs rather than masked as success. ~keep
            tracing::warn!(
                "CCITT decompression failed ({}x{}, {} bytes, K={}, EncodedByteAlign={}): {} — substituting blank image (DECODE FAILED, not a blank scan)",
                params.columns,
                params.rows.unwrap_or(0),
                data.len(),
                params.k,
                params.encoded_byte_align,
                e
            );
            build_blank_fallback(width, params).map(|fallback| (fallback, 0))
        }
    }
}

/// Logs the CCITT decompression parameters at the start of a decode attempt. ~keep
fn log_ccitt_decompress_start(data: &[u8], params: &CcittParams) {
    tracing::debug!(
        "CCITT decompression: {} bytes, {}x{} pixels, K={}, BlackIs1={}",
        data.len(),
        params.columns,
        params.rows.unwrap_or(0),
        params.k,
        params.black_is_1
    );

    if params.is_group_3() {
        tracing::debug!("CCITT Group 3 decompression requested (K={})", params.k);
    } else {
        tracing::debug!("CCITT Group 4 decompression requested");
    }
}

/// Tries the in-house CCITT decoder first, falling back to the `fax` crate
/// when it cannot make progress. Split out of [`decompress_ccitt`] to keep
/// that function focused on the overall decode-then-fallback flow. ~keep
fn decode_ccitt_with_fallback(data: &[u8], width: u32, params: &CcittParams) -> Result<(Vec<u8>, usize)> {
    let height_opt = params.rows;

    // Primary: the in-house decoder (Group 4 T.6, and Group 3 T.4 for K >= 0).
    // It honors /EncodedByteAlign (which the fax crate cannot — its bit reader
    // is private) and recovers partial content from truncated/damaged streams
    // instead of blanking the page. ~keep
    let in_house = crate::decoders::ccitt::decode(data, params);
    match in_house {
        Ok(decoded) => {
            if decoded.recovered_partial {
                tracing::warn!(
                    "CCITT: recovered {} rows then padded white (truncated/damaged stream, {}x{}, {} bytes)",
                    decoded.rows_decoded,
                    params.columns,
                    params.rows.unwrap_or(0),
                    data.len()
                );
            }
            Ok((decoded.data, decoded.rows_decoded))
        }
        Err(in_house_err) => {
            // A stream the in-house decoder couldn't make progress on: fall
            // back to the legacy fax crate before giving up. ~keep
            tracing::debug!("CCITT in-house decode declined ({in_house_err}); trying fax crate");
            // The fax crate reports no row count; it decodes the whole image or errors.
            decompress_with_fax(data, width, height_opt, params).map(|out| (out, usize::MAX))
        }
    }
}

/// Builds a bounded all-white bilevel buffer to substitute when both CCITT
/// decoders fail, so a controlled blank page is returned instead of panicking
/// or propagating an unrecoverable error. ~keep
fn build_blank_fallback(width: u32, params: &CcittParams) -> Result<Vec<u8>> {
    let bytes_per_row = (width as usize).div_ceil(8);
    let rows = usize::try_from(params.rows.unwrap_or(1))
        .map_err(|_| Error::Decode("CCITT row count exceeds platform limits".to_string()))?;
    let expected_bytes = rows
        .checked_mul(bytes_per_row)
        .ok_or_else(|| Error::Decode("CCITT fallback size overflow".to_string()))?;
    let fallback_len = expected_bytes.max(bytes_per_row);
    let mut fallback = Vec::new();
    fallback
        .try_reserve_exact(fallback_len)
        .map_err(|_| Error::Decode(format!("Unable to allocate {fallback_len} bytes for CCITT fallback")))?;
    fallback.resize(fallback_len, 0);
    Ok(fallback)
}

/// Decompress CCITT data using the fax crate.
///
/// The fax crate is more lenient with malformed EOFB markers compared to ccitt-t4-t6,
/// which makes it better suited for handling real-world PDF files that don't strictly
/// comply with the CCITT specification.
fn decompress_with_fax(data: &[u8], width: u32, height: Option<u32>, params: &CcittParams) -> Result<Vec<u8>> {
    let width_usize = width as usize;

    tracing::debug!(
        "Attempting CCITT decompression with fax crate: width={}, height={:?}, data_len={}, K={}",
        width,
        height,
        data.len(),
        params.k
    );

    match try_decode_with_fax(data, width_usize, height, params) {
        Ok(output) if !output.is_empty() => {
            return Ok(output);
        }
        Ok(_empty) => {
            tracing::debug!("First attempt returned no data, trying with leading zeros stripped");
        }
        Err(e) => {
            tracing::debug!("First attempt failed: {}, trying with leading zeros stripped", e);
        }
    }

    // If that failed, try stripping leading zeros (common in some PDFs) ~keep
    let first_nonzero = data.iter().position(|byte| *byte != 0).unwrap_or(data.len());
    let trimmed_len = data.len() - first_nonzero;
    let mut trimmed_data = Vec::new();
    trimmed_data.try_reserve_exact(trimmed_len).map_err(|_| {
        Error::Decode(format!(
            "Unable to allocate {trimmed_len} bytes for trimmed CCITT input"
        ))
    })?;
    trimmed_data.extend_from_slice(&data[first_nonzero..]);

    if trimmed_data.len() < data.len() && !trimmed_data.is_empty() {
        tracing::debug!(
            "Stripped {} leading zero bytes ({} -> {}), attempting decompression",
            data.len() - trimmed_data.len(),
            data.len(),
            trimmed_data.len()
        );

        tracing::debug!(
            "Data after stripping zeros, first 32 bytes: {}",
            trimmed_data
                .iter()
                .take(32)
                .map(|b| format!("{:02x}", b))
                .collect::<Vec<_>>()
                .join(" ")
        );

        match try_decode_with_fax(&trimmed_data, width_usize, height, params) {
            Ok(output) if !output.is_empty() => {
                tracing::trace!("Successfully decompressed after stripping leading zeros!");
                return Ok(output);
            }
            Ok(_) => {
                tracing::debug!("Strip attempt also returned no data");
            }
            Err(e) => {
                tracing::debug!("Strip attempt also failed: {}", e);
            }
        }
    }

    Err(Error::Decode(
        "CCITT decompression failed: fax decoder returned no output".to_string(),
    ))
}

fn try_decode_with_fax(data: &[u8], width: usize, height: Option<u32>, params: &CcittParams) -> Result<Vec<u8>> {
    use fax::decoder;

    let bytes_per_row = width.div_ceil(8);
    let max_rows = height.map(|rows| rows as usize);
    let mut output = Vec::new();
    if let Some(rows) = max_rows {
        let capacity = bytes_per_row
            .checked_mul(rows)
            .ok_or_else(|| Error::Decode("CCITT fax output size overflow".to_string()))?;
        output
            .try_reserve_exact(capacity)
            .map_err(|_| Error::Decode(format!("Unable to allocate {capacity} bytes for CCITT fax output")))?;
    }
    let mut rows_decoded = 0usize;

    let bytes_iter = data.iter().copied();

    let success = if params.is_group_4() {
        tracing::debug!("Using Group 4 (T.6) decoder");
        let mut callback_error = None;
        let success = decoder::decode_g4(bytes_iter, width as u32, height, |transitions: &[u32]| {
            if callback_error.is_none() && max_rows.is_none_or(|rows| rows_decoded < rows) {
                if let Err(error) = append_transition_row(&mut output, transitions, width) {
                    callback_error = Some(error);
                    return;
                }
                rows_decoded += 1;
            }
        });
        if let Some(error) = callback_error {
            return Err(error);
        }
        success
    } else {
        tracing::debug!("Using Group 3 (T.4) decoder");
        // Group 3 has a different signature - no width/height params in callback ~keep
        let mut callback_error = None;
        let success = decoder::decode_g3(bytes_iter, |transitions: &[u32]| {
            if callback_error.is_none() && max_rows.is_none_or(|rows| rows_decoded < rows) {
                if let Err(error) = append_transition_row(&mut output, transitions, width) {
                    callback_error = Some(error);
                    return;
                }
                rows_decoded += 1;
            }
        });
        if let Some(error) = callback_error {
            return Err(error);
        }
        success
    };

    if success.is_some() && !output.is_empty() {
        tracing::debug!(
            "CCITT decompression successful: {} bytes input -> {} bytes output ({} rows)",
            data.len(),
            output.len(),
            rows_decoded
        );
        Ok(output)
    } else if success.is_some() {
        // Decoder succeeded but produced no output - unusual but valid ~keep
        tracing::debug!("CCITT decoder returned success but no rows produced");
        Ok(Vec::new())
    } else {
        tracing::warn!("CCITT fax decoder returned None");
        Err(Error::Decode("CCITT fax decoder failed".to_string()))
    }
}

/// Convert run-length transition positions to byte-packed pixels.
///
/// The transitions array contains positions where the color changes from white to black
/// or black to white, starting with white. For example, [3, 5, 8] means:
/// - Pixels 0-2: white
/// - Pixels 3-4: black
/// - Pixels 5-7: white
///
/// Generic over the transition scalar so both transition producers can share it
/// without converting a row: the in-house decoder emits `u16`, while the `fax`
/// crate emits `u32` (widened in fax 0.3). Positions are compared in `usize`,
/// so neither width is truncated.
pub(crate) fn transitions_to_bytes<T: Copy + Into<u32>>(transitions: &[T], width: usize) -> Result<Vec<u8>> {
    let bytes_per_row = width.div_ceil(8);
    let mut row_bytes = Vec::new();
    row_bytes
        .try_reserve_exact(bytes_per_row)
        .map_err(|_| Error::Decode(format!("Unable to allocate {bytes_per_row} bytes for a CCITT row")))?;
    row_bytes.resize(bytes_per_row, 0);

    let mut is_black = false;
    let mut start_pos: usize = 0;

    for &transition_pos in transitions {
        let transition_pos = Into::<u32>::into(transition_pos) as usize;
        if is_black {
            for pixel_idx in start_pos..transition_pos.min(width) {
                let byte_idx = pixel_idx / 8;
                let bit_idx = 7 - (pixel_idx % 8);
                row_bytes[byte_idx] |= 1 << bit_idx;
            }
        }
        is_black = !is_black;
        start_pos = transition_pos;
    }

    if is_black && start_pos < width {
        for pixel_idx in start_pos..width {
            let byte_idx = pixel_idx / 8;
            let bit_idx = 7 - (pixel_idx % 8);
            row_bytes[byte_idx] |= 1 << bit_idx;
        }
    }

    Ok(row_bytes)
}

pub(crate) fn append_transition_row<T: Copy + Into<u32>>(
    output: &mut Vec<u8>,
    transitions: &[T],
    width: usize,
) -> Result<()> {
    let row = transitions_to_bytes(transitions, width)?;
    output
        .try_reserve(row.len())
        .map_err(|_| Error::Decode(format!("Unable to grow CCITT output by {} bytes", row.len())))?;
    output.extend_from_slice(&row);
    Ok(())
}

/// Decompresses CCITT Group 4 encoded data (legacy API for backwards compatibility).
///
/// This is a convenience function that uses default CCITT parameters.
#[deprecated(since = "0.1.5", note = "Use decompress_ccitt with CcittParams instead")]
pub fn decompress_ccitt_group4(data: &[u8], width: u32, height: u32) -> Result<Vec<u8>> {
    let params = CcittParams {
        columns: width,
        rows: Some(height),
        // Table 11: K < 0 is Group 4; the default K = 0 is Group 3 one-dimensional.
        k: -1,
        ..Default::default()
    };
    decompress_ccitt(data, &params)
}

/// Invert all bits in a bilevel image.
///
/// This is used when /BlackIs1=true to convert from:
/// - white=1, black=0 (inverted representation)
///
/// to standard PDF representation:
/// - white=0, black=1
fn invert_bilevel_pixels(data: &mut [u8]) {
    for byte in data.iter_mut() {
        *byte = !*byte;
    }
}

/// Convert 1-bit bilevel image to 8-bit grayscale.
///
/// Each bit in the input is expanded to a full byte where:
/// - 0 (white) -> 0xFF (white in 8-bit)
/// - 1 (black) -> 0x00 (black in 8-bit)
///
/// # Arguments
///
/// * `bilevel_data` - Packed bilevel image data (1 bit per pixel)
/// * `width` - Image width in pixels
/// * `height` - Image height in pixels
///
/// # Returns
///
/// A vector of 8-bit grayscale pixels suitable for image processing and OCR.
/// Returns an empty vector when `width` x `height` would exceed the output
/// size guard (see below) — the caller (`to_dynamic_image`) already treats a
/// length mismatch against the declared dimensions as `Error::Decode`
/// ("Invalid image dimensions"), so this keeps the public signature
/// unchanged while refusing the allocation.
pub fn bilevel_to_grayscale(bilevel_data: &[u8], width: u32, height: u32) -> Vec<u8> {
    let width = width as usize;
    let height = height as usize;

    // `width`/`height` come from a PDF's /Width and /Height (or CCITT
    // /Columns and /Rows), which a hostile file controls: a declared
    // 100 000 x 100 000 image would demand a ~10 GB allocation here (one
    // byte per pixel, expanded from 1-bit-packed input). Cap it the same
    // way the other image expanders in this crate do (see
    // MAX_UNPACKED_OUTPUT_BYTES / MAX_INDEXED_OUTPUT_BYTES in
    // extractors/images.rs). ~keep
    const MAX_GRAYSCALE_OUTPUT_BYTES: usize = 256 * 1024 * 1024;
    let Some(output_bytes) = width.checked_mul(height) else {
        tracing::warn!(
            "CCITT bilevel-to-grayscale size overflow: {width} x {height} exceeds usize; refusing to expand"
        );
        return Vec::new();
    };
    if output_bytes > MAX_GRAYSCALE_OUTPUT_BYTES {
        tracing::warn!(
            "Refusing to expand a {output_bytes}-byte CCITT grayscale buffer (cap {MAX_GRAYSCALE_OUTPUT_BYTES}); \
             width={width}, height={height}"
        );
        return Vec::new();
    }

    let mut grayscale = Vec::with_capacity(output_bytes);

    for row_idx in 0..height {
        // Each row in bilevel data is padded to byte boundary ~keep
        let row_start = row_idx * width.div_ceil(8);

        for col_idx in 0..width {
            let byte_idx = row_start + (col_idx / 8);
            if byte_idx < bilevel_data.len() {
                let bit_pos = 7 - (col_idx % 8);
                let bit = (bilevel_data[byte_idx] >> bit_pos) & 1;
                grayscale.push(if bit == 0 { 0xFF } else { 0x00 });
            } else {
                grayscale.push(0xFF);
            }
        }
    }

    grayscale
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bilevel_to_grayscale() {
        let bilevel = vec![0b10000001];
        let grayscale = bilevel_to_grayscale(&bilevel, 8, 1);

        assert_eq!(grayscale.len(), 8);
        assert_eq!(grayscale[0], 0x00, "Pixel 0 should be black");
        assert_eq!(grayscale[1], 0xFF, "Pixel 1 should be white");
        assert_eq!(grayscale[7], 0x00, "Pixel 7 should be black");
    }

    #[test]
    fn test_bilevel_to_grayscale_padding() {
        let bilevel = vec![0b10000001];
        let grayscale = bilevel_to_grayscale(&bilevel, 5, 1);

        assert_eq!(grayscale.len(), 5);
        assert_eq!(grayscale[0], 0x00);
        assert_eq!(grayscale[1], 0xFF);
        assert_eq!(grayscale[4], 0xFF);
    }

    /// `width`/`height` come straight from a PDF's /Width and /Height (or
    /// CCITT /Columns and /Rows); `bilevel_to_grayscale` used to allocate
    /// `Vec::with_capacity(width * height)` with no size cap. `u32::MAX x
    /// u32::MAX` doesn't overflow the `usize` multiplication on a 64-bit
    /// target, so it reached `Vec::with_capacity` with a byte count
    /// exceeding `isize::MAX`.
    ///
    /// Before the fix: this panics with "capacity overflow" (raised by
    /// `Vec`'s internal allocation-size check) instead of returning a
    /// bounded, empty result. After the fix, the checked-multiply + 256 MiB
    /// cap returns an empty `Vec` instead of allocating.
    #[test]
    fn huge_declared_dimensions_do_not_allocate() {
        let grayscale = bilevel_to_grayscale(&[], u32::MAX, u32::MAX);
        assert!(
            grayscale.is_empty(),
            "huge declared width x height must not be allocated"
        );
    }

    #[test]
    fn test_transitions_to_bytes() {
        // Test transitions to build pattern: WW|BBB|WW|B
        // Transitions at positions [2, 5, 7]:
        // - White from 0-2 (2 pixels)
        // - Black from 2-5 (3 pixels)
        // - White from 5-7 (2 pixels)
        // - Black from 7-8 (1 pixel)
        // Should produce: 0b00111001 = 57 (0x39)
        // Exercised for both transition scalars: the in-house decoder emits
        // u16, the fax crate (0.3+) emits u32, and both share this packer. ~keep
        let row_u16 = transitions_to_bytes(&[2u16, 5, 7], 8).expect("pack u16 row");
        assert_eq!(row_u16.len(), 1);
        assert_eq!(row_u16[0], 0b00111001);

        let row_u32 = transitions_to_bytes(&[2u32, 5, 7], 8).expect("pack u32 row");
        assert_eq!(row_u32, row_u16, "u32 and u16 transitions must pack identically");
    }

    /// A transition beyond u16::MAX must not wrap: fax 0.3 widened transitions
    /// to u32, and positions are compared in usize.
    #[test]
    fn test_transitions_to_bytes_beyond_u16() {
        let width = 70_000usize;
        let row = transitions_to_bytes(&[65_536u32, 65_544], width).expect("pack wide row");
        assert_eq!(row.len(), width.div_ceil(8));
        assert_eq!(row[65_536 / 8], 0xFF, "the 8 pixels at 65536.. must be black");
        assert!(
            row[..65_536 / 8].iter().all(|&b| b == 0),
            "everything before must stay white"
        );
    }
}

#[cfg(test)]
mod group4_entry_point_tests {
    use super::*;

    /// A Group 4 stream is two-dimensional only; an EOFB alone decodes to zero
    /// rows under Group 4 and is rejected by the Group 3 decoder, so the
    /// reported row count tells which decoder the legacy entry point asked for.
    #[test]
    #[allow(deprecated)]
    fn test_group4_entry_point_requests_group4() {
        let params = CcittParams {
            columns: 8,
            rows: Some(1),
            k: -1,
            ..Default::default()
        };
        assert!(params.is_group_4());
        assert!(!params.is_group_3());

        let defaulted = CcittParams {
            columns: 8,
            rows: Some(1),
            ..Default::default()
        };
        assert!(defaulted.is_group_3(), "the filter default is Group 3");

        // All-white 8x1 row in T.6: vertical mode V0 (bit "1") then EOFB.
        let g4 = [0b1000_0000, 0b0001_0000, 0b0000_0001];
        let (_, rows_valid) = decompress_ccitt_reporting(&g4, &params).expect("group 4 decode");
        assert_eq!(rows_valid, 1, "one all-white row decodes under Group 4");
        let legacy = decompress_ccitt_group4(&g4, 8, 1).expect("legacy entry point");
        assert_eq!(legacy, vec![0x00], "legacy entry point must decode as Group 4");

        // An all-white row is also what a failed decode falls back to, so a row with ink is what
        // tells decoding from blank substitution. Horizontal mode: white run 2, black run 6, EOFB.
        let g4_ink = [0x2E, 0x40, 0x04, 0x00, 0x80];
        let (reported, rows_valid) = decompress_ccitt_reporting(&g4_ink, &params).expect("group 4 decode");
        assert_eq!(
            (reported.as_slice(), rows_valid),
            (&[0x3F][..], 1),
            "test premise: the row carries ink"
        );
        let legacy = decompress_ccitt_group4(&g4_ink, 8, 1).expect("legacy entry point");
        assert_eq!(
            legacy, reported,
            "legacy entry point must decode a row with ink as Group 4"
        );
    }
}
