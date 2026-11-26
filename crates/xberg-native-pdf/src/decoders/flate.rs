//! FlateDecode (zlib/deflate) implementation.
//!
//! This is the most common PDF compression filter, used in ~90% of PDFs.
//! Uses the flate2 crate for zlib decompression.

#![forbid(unsafe_code)]

use crate::decoders::StreamDecoder;
use crate::error::{Error, Result};
use flate2::read::{DeflateDecoder, ZlibDecoder};
use std::io::Read;

/// Default cap for [`FlateDecoder`]: 256 MB per stream.
///
/// Prevents zip-bomb / flate-bomb attacks where a tiny compressed stream
/// expands to an arbitrarily large output, exhausting virtual memory and
/// triggering an allocator abort (SIGABRT / exit 134).
///
/// 256 MB accommodates A4 @ 600 DPI RGB (~99 MB) with headroom.
///
/// Override via:
/// - `XBERG_NATIVE_PDF_MAX_DECOMPRESS_MB` environment variable (e.g. `64` for
///   64 MB), or the pre-rename `PDF_OXIDE_MAX_DECOMPRESS_MB` if the new name
///   is unset — this is a security control, so an upgrade must never silently
///   revert a deployment to the 256 MB default because only the old name was
///   ever set.
/// - [`FlateDecoder::with_limit`] for programmatic control
pub const DEFAULT_MAX_DECOMPRESSED_BYTES: u64 = 256 * 1024 * 1024;

fn effective_limit_from_str(val: Option<&str>) -> u64 {
    val.and_then(|v| v.parse::<u64>().ok())
        .map(|mb| mb.saturating_mul(1024 * 1024))
        // A zero here would be read downstream as `StreamDecoder::decode`'s "cap disabled"
        // sentinel, so setting this security control to what looks like its STRICTEST value
        // would silently switch off the ratio check, the absolute-size check and the
        // RunLength/LZW mid-decode caps together. Treated as an invalid override and ignored,
        // loudly: a deployment that meant to tighten the limit must not end up with none. ~keep
        .filter(|bytes| {
            if *bytes == 0 {
                tracing::warn!(
                    "ignoring a decompression limit of 0 MB: that would disable bomb protection \
                     entirely, so the {} byte default applies instead",
                    DEFAULT_MAX_DECOMPRESSED_BYTES
                );
                return false;
            }
            true
        })
        .unwrap_or(DEFAULT_MAX_DECOMPRESSED_BYTES)
}

/// Resolve the limit from the two possible environment values, new name
/// first. Pulled apart from [`effective_limit`] so the fallback precedence
/// is testable without mutating real process environment variables.
fn effective_limit_from_env_pair(new_val: Option<&str>, old_val: Option<&str>) -> u64 {
    effective_limit_from_str(new_val.or(old_val))
}

/// Read the decompression limit from the environment, falling back to the
/// compile-time default.
///
/// Three names are accepted, newest first: `XBERG_NATIVE_PDF_MAX_DECOMPRESS_MB`, then the
/// pre-vendoring `XBERG_PDF_OXIDE_MAX_DECOMPRESS_MB`, then upstream's original
/// `PDF_OXIDE_MAX_DECOMPRESS_MB`. Each rename kept the older spellings working because this
/// is a deployment knob someone has already set in a Dockerfile or a systemd unit; dropping a
/// name silently reverts their limit to the compile-time default with no error. ~keep
pub(crate) fn effective_limit() -> u64 {
    let new_val = std::env::var("XBERG_NATIVE_PDF_MAX_DECOMPRESS_MB").ok();
    let previous_val = std::env::var("XBERG_PDF_OXIDE_MAX_DECOMPRESS_MB").ok();
    let old_val = std::env::var("PDF_OXIDE_MAX_DECOMPRESS_MB").ok();
    effective_limit_from_env_pair(new_val.or(previous_val).as_deref(), old_val.as_deref())
}

/// Heuristic validator for partial-recovery output from a failing decompress.
///
/// Returns `true` if the decoded bytes look like a plausible PDF stream — any
/// of: a content-stream operator (BT/ET/Tj/TJ/Tm/Td), a common PDF object
/// marker (<</>>/stream/obj/endobj), or a `%PDF-` prefix. The set is kept
/// intentionally broad because FlateDecode also wraps object streams, xref
/// streams, font programs, and image data, none of which carry text operators.
///
/// The guard exists to distinguish genuine partially-decoded output from
/// `deflate` "success" on a misaligned input — the latter produces short runs
/// of pseudo-random bytes (128 bytes of `P\xffj!}` repeating) that contain
/// none of these markers.
///
/// Conservative fallback: if the decoded bytes are mostly ASCII (printable
/// plus whitespace), the output is also treated as plausible, because stream
/// contents in the wild include ASCII-only data (hex-encoded images, small
/// object streams) that do not hit any specific marker.
fn looks_like_real_stream(output: &[u8]) -> bool {
    if output.is_empty() {
        return false;
    }
    const MARKERS: &[&[u8]] = &[b"BT", b"ET", b"Tj", b"TJ", b"Tm", b"Td", b"stream", b"endobj", b"%PDF-"];
    for m in MARKERS {
        if output.windows(m.len()).any(|w| w == *m) {
            return true;
        }
    }
    let printable = output
        .iter()
        .filter(|&&b| (0x20..=0x7E).contains(&b) || b == b'\t' || b == b'\n' || b == b'\r')
        .count();
    printable * 5 >= output.len() * 4
}

/// Returns `Err` if `output` reached the decompression cap, indicating that the
/// stream was truncated rather than fully decoded.
#[inline]
fn check_limit(output: &[u8], limit: u64) -> Result<()> {
    if output.len() as u64 >= limit {
        return Err(Error::Decode(format!(
            "FlateDecode output reached the {limit} byte safety limit; \
             stream may be a flate bomb or an unusually large image"
        )));
    }
    Ok(())
}

/// True if `output` is non-empty and plausibly real content per
/// [`looks_like_real_stream`], after validating it against the decompression
/// cap. Shared by every recovery strategy's "accept this partial output?" check.
fn accept_as_partial(output: &[u8], limit: u64) -> Result<bool> {
    check_limit(output, limit)?;
    Ok(!output.is_empty() && looks_like_real_stream(output))
}

/// A deflate stream that stops without its final block marker, as opposed to
/// one with corrupt bits. flate2 >= 1.1.10 rejects these at EOF, but the
/// decoded prefix is valid and binary xref/object streams fail the
/// content-stream heuristic in [`looks_like_real_stream`]. ~keep
fn is_truncation(e: &std::io::Error) -> bool {
    if e.kind() == std::io::ErrorKind::UnexpectedEof {
        return true;
    }
    let msg = e.to_string();
    msg.contains("incomplete deflate stream") || msg.contains("unexpected end")
}

fn accept_truncated_or_plausible(output: &[u8], limit: u64, e: &std::io::Error) -> Result<bool> {
    check_limit(output, limit)?;
    Ok(!output.is_empty() && (is_truncation(e) || looks_like_real_stream(output)))
}

/// Outcome of a primary (non-fallback) decode attempt: accepted output — a
/// clean decode or an accepted partial recovery — or a failure whose error is
/// kept for the final "all strategies failed" message.
enum PrimaryAttempt {
    Recovered(Vec<u8>),
    Failed(std::io::Error),
}

/// Return the part of a `reserved`-byte output buffer the stream did not fill, which a dictionary
/// overstating its raster or a truncated stream leaves behind, once it exceeds the filled part:
/// smaller slack is not worth the copy a shrink makes. An unreserved buffer is kept as the growth
/// left it.
fn release_unfilled(output: &mut Vec<u8>, reserved: usize) {
    if reserved > 0 && output.capacity() > output.len().saturating_mul(2) {
        output.shrink_to_fit();
    }
}

/// Strategy 1: decode `input` as zlib-wrapped deflate. Partial recovery:
/// return only if the output *looks like* a plausible stream. The pre-fix
/// behaviour accepted any non-empty buffer, which let strategies 2 and 3
/// return misaligned-deflate garbage (`P\xffj!}` × 16) that the text
/// extractor then emitted as zero bytes of output. ~keep
fn try_zlib_decode(input: &[u8], limit: u64, capacity: usize) -> Result<PrimaryAttempt> {
    let mut decoder = ZlibDecoder::new(input).take(limit);
    let mut output = Vec::with_capacity(capacity);

    match decoder.read_to_end(&mut output) {
        Ok(_) => {
            check_limit(&output, limit)?;
            release_unfilled(&mut output, capacity);
            Ok(PrimaryAttempt::Recovered(output))
        }
        Err(e) => {
            if accept_truncated_or_plausible(&output, limit, &e)? {
                release_unfilled(&mut output, capacity);
                tracing::warn!(
                    filter = "FlateDecode",
                    bytes = output.len(),
                    error = %e,
                    "partial recovery: extracted bytes before corruption"
                );
                return Ok(PrimaryAttempt::Recovered(output));
            }
            Ok(PrimaryAttempt::Failed(e))
        }
    }
}

/// Strategy 2: try raw deflate (no zlib wrapper).
/// Some PDFs have corrupt zlib headers but valid deflate data. ~keep
fn try_raw_deflate(input: &[u8], limit: u64) -> Result<PrimaryAttempt> {
    tracing::debug!(filter = "FlateDecode", "zlib decode failed, trying raw deflate");
    let mut deflate_decoder = DeflateDecoder::new(input).take(limit);
    let mut output = Vec::new();

    match deflate_decoder.read_to_end(&mut output) {
        Ok(_) => {
            check_limit(&output, limit)?;
            tracing::warn!(
                filter = "FlateDecode",
                bytes = output.len(),
                "recovered via raw deflate fallback after corrupt zlib header"
            );
            Ok(PrimaryAttempt::Recovered(output))
        }
        Err(deflate_err) => {
            if accept_truncated_or_plausible(&output, limit, &deflate_err)? {
                tracing::warn!(
                    filter = "FlateDecode",
                    bytes = output.len(),
                    "raw deflate partial recovery: extracted bytes before error"
                );
                return Ok(PrimaryAttempt::Recovered(output));
            }
            Ok(PrimaryAttempt::Failed(deflate_err))
        }
    }
}

/// FlateDecode filter implementation.
///
/// Decompresses data using the zlib/deflate algorithm. The decompression cap
/// defaults to `DEFAULT_MAX_DECOMPRESSED_BYTES` and can be overridden with
/// [`FlateDecoder::with_limit`].
pub struct FlateDecoder {
    /// Maximum number of decompressed bytes accepted per stream.
    pub max_decompressed_bytes: u64,
}

impl Default for FlateDecoder {
    fn default() -> Self {
        Self {
            max_decompressed_bytes: effective_limit(),
        }
    }
}

impl FlateDecoder {
    /// Creates a decoder that rejects any stream decompressing to more than
    /// `limit` bytes. Use this to tighten or relax the default 256 MB cap.
    pub fn with_limit(limit: u64) -> Self {
        Self {
            max_decompressed_bytes: limit,
        }
    }
}

impl FlateDecoder {
    pub(super) fn decode_complete(&self, input: &[u8], max_output_bytes: usize) -> Result<Vec<u8>> {
        use flate2::{Decompress, FlushDecompress, Status};

        let limit = self
            .max_decompressed_bytes
            .min((max_output_bytes as u64).saturating_add(1));
        let mut decoder = Decompress::new(true);
        let mut output = Vec::new();
        let mut chunk = [0; 8192];
        loop {
            let offset = decoder.total_in() as usize;
            let chunk_len = (chunk.len() as u64).min(limit - output.len() as u64) as usize;
            let status = decoder
                .decompress(&input[offset..], &mut chunk[..chunk_len], FlushDecompress::None)
                .map_err(|error| Error::Decode(format!("FlateDecode: {error}")))?;
            let count = decoder.total_out() as usize - output.len();
            output.extend_from_slice(&chunk[..count]);
            check_limit(&output, limit)?;
            if status == Status::StreamEnd && decoder.total_in() as usize == input.len() {
                return Ok(output);
            }
            if status == Status::StreamEnd || (count == 0 && decoder.total_in() as usize == offset) {
                return Err(Error::Decode(
                    "FlateDecode: incomplete zlib stream or trailing data".into(),
                ));
            }
        }
    }
}

/// Most output reserved up front per deflate input byte. Deflate can reach 1032 bytes per input
/// byte, so a bound at that ratio let a hostile dictionary over a 1 MB stream reserve 1 GB before
/// decoding a byte; a stream that inflates further grows past the reservation as it decodes.
const MAX_PRESIZED_EXPANSION: usize = 64;

/// The output capacity to reserve for a stream expected to decode to `expected_size` bytes: no
/// more than the read `limit` allows or than [`MAX_PRESIZED_EXPANSION`] times `input`, so an
/// image dictionary that overstates its size cannot reserve memory its data is unlikely to fill.
fn presized_capacity(input: &[u8], limit: u64, expected_size: usize) -> usize {
    let readable = usize::try_from(limit).unwrap_or(usize::MAX);
    expected_size
        .min(readable)
        .min(input.len().saturating_mul(MAX_PRESIZED_EXPANSION))
}

impl FlateDecoder {
    /// [`StreamDecoder::decode`] for a stream expected to decode to `expected_size` bytes, such as an
    /// image raster whose size its dictionary implies: the output is reserved at that size, capped
    /// by [`presized_capacity`], instead of grown by doubling, which holds the old and new buffers
    /// at once and can leave nearly twice the raster allocated.
    pub(crate) fn decode_presized(
        &self,
        input: &[u8],
        max_output_bytes: usize,
        expected_size: usize,
    ) -> Result<Vec<u8>> {
        self.decode_with_capacity(input, max_output_bytes, Some(expected_size))
    }

    fn decode_with_capacity(
        &self,
        input: &[u8],
        max_output_bytes: usize,
        expected_size: Option<usize>,
    ) -> Result<Vec<u8>> {
        if input.is_empty() {
            return Ok(Vec::new());
        }

        let limit = if max_output_bytes == 0 {
            self.max_decompressed_bytes
        } else {
            // Read one byte past the inclusive caller budget to distinguish exact fit from truncation.
            self.max_decompressed_bytes
                .min((max_output_bytes as u64).saturating_add(1))
        };
        let capacity = expected_size.map_or(0, |expected| presized_capacity(input, limit, expected));

        let zlib_err = match try_zlib_decode(input, limit, capacity)? {
            PrimaryAttempt::Recovered(output) => return Ok(output),
            PrimaryAttempt::Failed(e) => e,
        };
        let deflate_err = match try_raw_deflate(input, limit)? {
            PrimaryAttempt::Recovered(output) => return Ok(output),
            PrimaryAttempt::Failed(e) => e,
        };

        if let Some(output) = try_header_skip_deflate(input, limit)? {
            return Ok(output);
        }
        if let Some(output) = try_corrected_header(input, limit)? {
            return Ok(output);
        }
        if let Some(output) = try_brute_force_scan(input, limit)? {
            return Ok(output);
        }

        // SPEC COMPLIANCE FIX: Removed strategies 8-9 that violated PDF spec
        //
        // Previous strategies 8-9 would return raw uncompressed data for streams
        // labeled as /FlateDecode. This violates PDF Spec ISO 32000-1:2008,
        // Section 7.3.8.2 which states that if a stream has /Filter /FlateDecode,
        // it MUST be compressed with the FlateDecode algorithm.
        //
        // Returning raw data creates security risks:
        // 1. Malicious PDFs could bypass compression validation
        // 2. Type confusion attacks (treating compressed data as raw)
        // 3. Inconsistent behavior across PDF processors
        //
        // Correct behavior: If all decompression strategies fail, return an error.
        // The stream is either corrupted or malicious, and should not be processed.
        // ~keep
        Err(Error::Decode(format!(
            "FlateDecode decompression failed: stream is labeled as compressed but all decompression attempts failed. \
            This violates PDF Spec ISO 32000-1:2008, Section 7.3.8.2. \
            Zlib error: {}, Deflate error: {}. Compressed size: {} bytes.",
            zlib_err,
            deflate_err,
            input.len()
        )))
    }
}

impl StreamDecoder for FlateDecoder {
    fn decode(&self, input: &[u8], max_output_bytes: usize) -> Result<Vec<u8>> {
        self.decode_with_capacity(input, max_output_bytes, None)
    }

    fn name(&self) -> &str {
        "FlateDecode"
    }
}

/// Strategy 3: retry raw deflate after skipping the codestream's first two
/// bytes — some PDFs pair a corrupt zlib header with otherwise-valid deflate
/// data starting right after it. Returns `Ok(None)` to fall through to the
/// next strategy.
fn try_header_skip_deflate(input: &[u8], limit: u64) -> Result<Option<Vec<u8>>> {
    if input.len() <= 2 {
        return Ok(None);
    }
    tracing::debug!(
        filter = "FlateDecode",
        "trying deflate after skipping potential corrupt zlib header"
    );
    let mut output = Vec::new();
    let mut deflate_decoder = DeflateDecoder::new(&input[2..]).take(limit);

    match deflate_decoder.read_to_end(&mut output) {
        Ok(_) => {
            check_limit(&output, limit)?;
            tracing::warn!(
                filter = "FlateDecode",
                bytes = output.len(),
                "recovered by skipping corrupt zlib header bytes"
            );
            Ok(Some(output))
        }
        Err(_) => {
            if accept_as_partial(&output, limit)? {
                tracing::warn!(
                    filter = "FlateDecode",
                    bytes = output.len(),
                    "header-skip partial recovery"
                );
                return Ok(Some(output));
            }
            Ok(None)
        }
    }
}

/// Strategy 4: try fixing a corrupt zlib header byte. If the first byte has
/// an invalid compression method, replace it with a standard deflate CMF and
/// retry. ~keep
fn try_corrected_header(input: &[u8], limit: u64) -> Result<Option<Vec<u8>>> {
    if input.len() < 2 {
        return Ok(None);
    }
    let first_byte = input[0];
    let compression_method = first_byte & 0x0F;
    if compression_method == 8 {
        return Ok(None);
    }
    tracing::debug!(
        filter = "FlateDecode",
        compression_method,
        header_byte = format_args!("0x{:02x}", first_byte),
        "invalid compression method in header byte, trying corrected header"
    );
    let mut corrected = input.to_vec();
    // Replace CM bits (0-3) with 8 (deflate), keep CINFO bits (4-7) ~keep
    corrected[0] = (first_byte & 0xF0) | 0x08;

    let mut output = Vec::new();
    let mut decoder = ZlibDecoder::new(&corrected[..]).take(limit);
    let read_ok = decoder.read_to_end(&mut output).is_ok();
    if read_ok && !output.is_empty() {
        check_limit(&output, limit)?;
        tracing::warn!(
            filter = "FlateDecode",
            bytes = output.len(),
            "recovered via corrected zlib header byte"
        );
        return Ok(Some(output));
    }
    if accept_as_partial(&output, limit)? {
        tracing::warn!(
            filter = "FlateDecode",
            bytes = output.len(),
            "header-correction partial recovery"
        );
        return Ok(Some(output));
    }
    tracing::debug!(filter = "FlateDecode", "header correction failed");
    Ok(None)
}

/// Strategy 5: brute-force scan for valid deflate data. Tries starting
/// deflate decompression from offsets 0-20, but validates the output
/// contains valid PDF operators before accepting it. ~keep
fn try_brute_force_scan(input: &[u8], limit: u64) -> Result<Option<Vec<u8>>> {
    tracing::debug!(filter = "FlateDecode", "trying brute-force scan for valid deflate data");
    let max_offset = std::cmp::min(20, input.len());
    for offset in 0..max_offset {
        if offset == 0 || offset == 2 {
            continue;
        }
        if let Some(output) = try_offset_deflate(input, offset, limit)? {
            return Ok(Some(output));
        }
    }
    Ok(None)
}

/// One offset attempt for [`try_brute_force_scan`]: decode from `offset` and
/// accept the output only if it contains a recognizable PDF content-stream
/// operator, whether the read completed cleanly or errored partway through.
fn try_offset_deflate(input: &[u8], offset: usize, limit: u64) -> Result<Option<Vec<u8>>> {
    let mut output = Vec::new();
    let mut deflate_decoder = DeflateDecoder::new(&input[offset..]).take(limit);
    let read_ok = deflate_decoder.read_to_end(&mut output).is_ok();
    if output.is_empty() {
        return Ok(None);
    }
    check_limit(&output, limit)?;
    let decoded_str = String::from_utf8_lossy(&output);
    let has_pdf_operators = decoded_str.contains("BT")
        || decoded_str.contains("ET")
        || decoded_str.contains("Tj")
        || decoded_str.contains("TJ")
        || decoded_str.contains("Tm")
        || decoded_str.contains("Td");

    if !has_pdf_operators {
        if read_ok {
            tracing::trace!(
                filter = "FlateDecode",
                offset,
                bytes = output.len(),
                "brute-force offset produced no valid PDF operators, trying next offset"
            );
        } else {
            tracing::trace!(
                filter = "FlateDecode",
                offset,
                "brute-force partial recovery has no valid PDF operators, trying next offset"
            );
        }
        return Ok(None);
    }

    if read_ok {
        tracing::warn!(
            filter = "FlateDecode",
            offset,
            bytes = output.len(),
            "brute-force deflate recovery succeeded (validated PDF content)"
        );
    } else {
        tracing::warn!(
            filter = "FlateDecode",
            offset,
            bytes = output.len(),
            "brute-force partial recovery (validated PDF content)"
        );
    }
    Ok(Some(output))
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::Compression;
    use flate2::write::ZlibEncoder;
    use std::io::Write;

    // When a strategy's partial recovery emits garbage (valid-looking
    // deflate bits that decode to pseudo-random bytes on a misaligned input),
    // the decoder must not accept it. Strategy 5 already validates via PDF
    // content-stream operators; strategies 1–4 now validate via
    // `looks_like_real_stream`. This test pins that guard. ~keep
    #[test]
    fn looks_like_real_stream_rejects_repeating_garbage() {
        // Actual symptom before the fix: 128 bytes of
        // `P\xffj!}\xef\xbd\xbd\xef\xbd\xbd...` high-bit-heavy repetition. ~keep
        let garbage =
            b"P\xffj!}\xef\xbd\xbd\xef\xbd\xbd\xef\xbd\xbd\xef\xbd\xbd\xef\xbd\xbd\xef\xbd\xbd\xef\xbd\xbd\xef\xbd\xbd"
                .repeat(4);
        assert!(
            !looks_like_real_stream(&garbage),
            "misaligned-deflate garbage must be rejected as a partial recovery"
        );
    }

    #[test]
    fn looks_like_real_stream_accepts_content_stream_operators() {
        let real = b"BT /F1 12 Tf 100 700 Td (hello) Tj ET";
        assert!(looks_like_real_stream(real));
    }

    #[test]
    fn looks_like_real_stream_accepts_ascii_only_object_stream() {
        // Object-stream-like payload: ASCII, no content-stream operators. ~keep
        let object_stream = b"1 0 obj\n<< /Length 42 >>\nstream\nhello world\nendstream\nendobj\n";
        assert!(looks_like_real_stream(object_stream));
    }

    #[test]
    fn looks_like_real_stream_rejects_empty() {
        assert!(!looks_like_real_stream(&[]));
    }

    #[test]
    fn test_flate_decode_simple() {
        let decoder = FlateDecoder::default();

        let original = b"Hello, FlateDecode!";
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(original).unwrap();
        let compressed = encoder.finish().unwrap();

        let decoded = decoder.decode(&compressed, 0).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_empty_stream_decodes_to_no_bytes() {
        let decoded = FlateDecoder::default()
            .decode(&[], 0)
            .expect("an empty stream must decode to zero bytes, not fail");
        assert!(decoded.is_empty());
    }

    #[test]
    fn test_truncated_deflate_stream_yields_its_decoded_prefix() {
        let original: Vec<u8> = (0u16..600).map(|i| (i % 251) as u8).collect();
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&original).unwrap();
        let complete = encoder.finish().unwrap();

        let truncated = &complete[..complete.len() * 3 / 4];
        assert!(truncated.len() < complete.len());

        let decoded = FlateDecoder::default()
            .decode(truncated, 0)
            .expect("a truncated deflate stream must still yield its decoded prefix");

        assert!(!decoded.is_empty(), "recovery returned nothing for a truncated stream");
        assert!(original.starts_with(&decoded));
    }

    #[test]
    fn test_flate_decode_empty() {
        let decoder = FlateDecoder::default();

        let original = b"";
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(original).unwrap();
        let compressed = encoder.finish().unwrap();

        let decoded = decoder.decode(&compressed, 0).unwrap();
        assert_eq!(decoded, original);
    }

    /// A raster of noise, which deflate cannot shrink below [`MAX_PRESIZED_EXPANSION`] times, so
    /// the reservation covers it.
    fn raster(length: usize) -> (Vec<u8>, Vec<u8>) {
        let mut state: u32 = 0x9e37_79b9;
        let original: Vec<u8> = (0..length)
            .map(|_| {
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                (state >> 24) as u8
            })
            .collect();
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&original).unwrap();
        (original, encoder.finish().unwrap())
    }

    #[test]
    fn a_presized_decode_holds_exactly_the_expected_raster() {
        let (original, compressed) = raster(100_000);
        let decoded = FlateDecoder::default()
            .decode_presized(&compressed, 0, original.len())
            .unwrap();
        assert_eq!(decoded, original);
        assert_eq!(decoded.capacity(), original.len(), "reserved once, never grown");
        // The image pipeline hands the final filter the raster size its dictionary implies.
        let piped = crate::decoders::decode_stream_with_params_and_expected_size(
            &compressed,
            &["FlateDecode".to_string()],
            None,
            original.len(),
        )
        .unwrap();
        assert_eq!(piped, original);
        assert_eq!(piped.capacity(), original.len());
    }

    #[test]
    fn a_presized_decode_survives_a_dictionary_that_misstates_the_raster() {
        let (original, compressed) = raster(100_000);
        let overstated = FlateDecoder::default()
            .decode_presized(&compressed, 0, original.len() * 3)
            .unwrap();
        assert_eq!(overstated, original);
        assert_eq!(
            overstated.capacity(),
            original.len(),
            "the unfilled reservation is released"
        );
        let understated = FlateDecoder::default()
            .decode_presized(&compressed, 0, original.len() / 3)
            .unwrap();
        assert_eq!(understated, original);
    }

    #[test]
    fn a_presized_reservation_never_exceeds_the_read_limit_or_a_small_multiple_of_the_input() {
        assert_eq!(
            presized_capacity(&[0; 10], u64::MAX, usize::MAX),
            10 * MAX_PRESIZED_EXPANSION
        );
        assert_eq!(presized_capacity(&[0; 10_000], 100, 5_000), 100);
        assert_eq!(presized_capacity(&[0; 10_000], u64::MAX, 5_000), 5_000);
        // A dictionary claiming 1 GB over a 1 MB stream once reserved all of it up front.
        assert_eq!(presized_capacity(&vec![0; 1 << 20], u64::MAX, 1 << 30), 64 << 20);
    }

    /// A stream that inflates past the reservation grows to its full size, and a buffer whose
    /// slack is no larger than its data is not copied by a shrink.
    #[test]
    fn a_presized_decode_grows_past_a_capped_reservation() {
        let original = vec![0u8; 1 << 20];
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&original).unwrap();
        let compressed = encoder.finish().unwrap();
        assert!(
            compressed.len() * MAX_PRESIZED_EXPANSION < original.len(),
            "the fixture must outgrow the cap"
        );

        let decoded = FlateDecoder::default()
            .decode_presized(&compressed, 0, original.len())
            .unwrap();

        assert_eq!(decoded, original);
        assert!(decoded.capacity() <= 2 * original.len(), "{}", decoded.capacity());
    }

    #[test]
    fn test_flate_decode_large_data() {
        let decoder = FlateDecoder::default();

        let original = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ".repeat(1000);
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&original).unwrap();
        let compressed = encoder.finish().unwrap();

        let decoded = decoder.decode(&compressed, 0).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_flate_decode_invalid_data() {
        let decoder = FlateDecoder::default();

        // Invalid zlib data - should fail decompression
        // SPEC COMPLIANCE: We now correctly reject invalid compressed data
        // instead of returning it as raw data (which violated PDF spec) ~keep
        let invalid = b"This is not zlib compressed data";
        let result = decoder.decode(invalid, 0);
        assert!(result.is_err());

        if let Err(e) = result {
            let error_msg = format!("{}", e);
            assert!(error_msg.contains("FlateDecode decompression failed"));
        }
    }

    #[test]
    fn test_flate_decoder_name() {
        let decoder = FlateDecoder::default();
        assert_eq!(decoder.name(), "FlateDecode");
    }

    #[test]
    fn test_flate_bomb_rejected() {
        let large = vec![0u8; DEFAULT_MAX_DECOMPRESSED_BYTES as usize];
        let result = check_limit(&large, DEFAULT_MAX_DECOMPRESSED_BYTES);
        assert!(result.is_err());
        let msg = format!("{}", result.unwrap_err());
        assert!(msg.contains("safety limit"));
    }

    #[test]
    fn test_check_limit_below_threshold() {
        let small = vec![0u8; 1024];
        assert!(check_limit(&small, DEFAULT_MAX_DECOMPRESSED_BYTES).is_ok());
    }

    #[test]
    fn test_custom_limit_accepts_data_within_limit() {
        let original = b"x".repeat(512);
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&original).unwrap();
        let compressed = encoder.finish().unwrap();

        let decoder = FlateDecoder::with_limit(1024);
        let decoded = decoder.decode(&compressed, 0).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_custom_limit_rejects_data_over_limit() {
        let original = b"x".repeat(100);
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&original).unwrap();
        let compressed = encoder.finish().unwrap();

        let decoder = FlateDecoder::with_limit(10);
        let result = decoder.decode(&compressed, 0);
        assert!(result.is_err(), "expected rejection when output exceeds custom limit");
    }

    #[test]
    fn test_bomb_error_does_not_expose_internal_symbol_name() {
        let large = vec![0u8; DEFAULT_MAX_DECOMPRESSED_BYTES as usize];
        let result = check_limit(&large, DEFAULT_MAX_DECOMPRESSED_BYTES);
        assert!(result.is_err());
        let msg = format!("{}", result.unwrap_err());
        assert!(
            !msg.contains("MAX_DECOMPRESSED_BYTES"),
            "error message must not reference internal symbol names: {msg}"
        );
    }

    #[test]
    fn test_effective_limit_env_variable() {
        assert_eq!(effective_limit_from_str(None), DEFAULT_MAX_DECOMPRESSED_BYTES);
        assert_eq!(effective_limit_from_str(Some("64")), 64 * 1024 * 1024);
        assert_eq!(
            effective_limit_from_str(Some("not_a_number")),
            DEFAULT_MAX_DECOMPRESSED_BYTES
        );
    }

    /// A limit of `0` MB must NOT be honoured: downstream, `0` is `StreamDecoder::decode`'s
    /// "cap disabled" sentinel, so obeying it would turn the strictest-looking setting of this
    /// security control into no protection at all.
    #[test]
    fn test_effective_limit_rejects_zero_instead_of_disabling_the_cap() {
        assert_eq!(
            effective_limit_from_str(Some("0")),
            DEFAULT_MAX_DECOMPRESSED_BYTES,
            "0 MB must fall back to the default, never to the cap-disabled sentinel"
        );
        assert_ne!(
            effective_limit_from_str(Some("0")),
            0,
            "the resolved limit must never be 0"
        );
    }

    #[test]
    fn test_effective_limit_prefers_new_env_var_and_falls_back_to_old() {
        assert_eq!(effective_limit_from_env_pair(Some("64"), Some("128")), 64 * 1024 * 1024);
        // New name unset: the pre-rename name still works, so an upgrade
        // never silently reverts a deployment to the compile-time default. ~keep
        assert_eq!(effective_limit_from_env_pair(None, Some("64")), 64 * 1024 * 1024);
        assert_eq!(
            effective_limit_from_env_pair(None, None),
            DEFAULT_MAX_DECOMPRESSED_BYTES
        );
    }
}
