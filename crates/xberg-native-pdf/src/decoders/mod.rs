//! Stream decoder implementations for PDF filters.
//!
//! This module provides decoders for various PDF compression and encoding filters:
//! - FlateDecode (zlib/deflate) - most common
//! - ASCIIHexDecode - hexadecimal encoding
//! - ASCII85Decode - base85 encoding
//! - LZWDecode - LZW compression
//! - RunLengthDecode - run-length encoding
//! - DCTDecode - JPEG (pass-through)
//! - CCITTFaxDecode - CCITT Fax compression (pass-through)
//! - JBIG2Decode - JBIG2 compression (pass-through)
//!
//! Decoders can be chained together in a filter pipeline.

use crate::error::{Error, Result};
use crate::parser_config::ParserOptions;

mod ascii85;
mod ascii_hex;
mod brotli;
pub(crate) mod ccitt;
mod ccitt_tables;
mod dct;
mod flate;
mod jbig2;
pub(crate) mod jpx;
mod lzw;
mod predictor;
mod runlength;

pub use ascii_hex::AsciiHexDecoder;
pub use ascii85::Ascii85Decoder;
pub use brotli::BrotliDecoder;
pub use ccitt::CcittFaxDecoder;
pub use dct::DctDecoder;
pub use flate::FlateDecoder;
pub use jbig2::Jbig2Decoder;
pub use lzw::LzwDecoder;
pub use predictor::{CcittParams, DecodeParams, PngPredictor, decode_predictor};
pub use runlength::RunLengthDecoder;

/// Security limits for decompression (decompression bomb protection).
///
/// PDF Spec: ISO 32000-1:2008 does not specify decompression limits, but these
/// are necessary security measures to prevent memory exhaustion attacks.
///
/// Default values:
/// - Max decompression ratio: 100:1 (compressed:decompressed)
/// - Max decompressed size: [`flate::DEFAULT_MAX_DECOMPRESSED_BYTES`] (256 MB),
///   overridable by the same environment variable
const DEFAULT_MAX_DECOMPRESSION_RATIO: u32 = 100;

/// Absolute output cap applied when a caller passes no [`ParserOptions`].
///
/// This is deliberately the SAME number `FlateDecoder` already enforces on its own
/// output rather than a second, lower one. A separate 100 MB cap here rejected
/// streams the flate stage had just produced and accepted, which discarded a real
/// page's entire text layer with nothing but a warning (GH#1754): one page of a
/// statistics textbook is a 24 MB compressed content stream that decodes to 194 MB
/// at a ratio of 8:1 — nothing like a bomb, and well inside the flate stage's 256 MB
/// ceiling. Deriving it from `flate::effective_limit()` also makes
/// `XBERG_NATIVE_PDF_MAX_DECOMPRESS_MB` mean what it says: one knob for the crate's
/// decompression ceiling, not one that governs the flate stage while a hardcoded
/// second limit silently overrides it downward.
///
/// Bomb protection is unchanged in kind, but be exact about which guard covers what.
/// The 100:1 ratio check below is what separates a bomb from a merely large stream
/// for flate. It cannot fire on a pure RunLength stream at all: the densest encoding
/// that filter permits is a `[129, byte]` pair — 2 input bytes for 128 output bytes —
/// so 64:1 is the most it can achieve and it never reaches the threshold. This
/// absolute cap is therefore RunLength's only guard. The pipeline passes it to
/// `StreamDecoder::decode_bounded`, which RunLength and LZW override to stop as soon
/// as their output crosses it, rather than building the whole output first (GH#1764). ~keep
pub(crate) fn default_max_decompressed_size() -> usize {
    usize::try_from(flate::effective_limit()).unwrap_or(usize::MAX)
}

/// PDF stream filter types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Filter {
    /// FlateDecode (deflate/zlib compression)
    FlateDecode,
    /// ASCIIHexDecode (hexadecimal encoding)
    ASCIIHexDecode,
    /// ASCII85Decode (base-85 encoding)
    ASCII85Decode,
    /// LZWDecode (Lempel-Ziv-Welch compression)
    LZWDecode,
    /// RunLengthDecode (run-length encoding)
    RunLengthDecode,
    /// DCTDecode (JPEG compression)
    DCTDecode,
    /// CCITTFaxDecode (CCITT Fax compression)
    CCITTFaxDecode,
    /// JBIG2Decode (JBIG2 compression)
    JBIG2Decode,
    /// JPXDecode (JPEG 2000 compression)
    JPXDecode,
    /// BrotliDecode (Brotli compression, PDF 2.0)
    BrotliDecode,
}

/// Trait for PDF stream decoders.
///
/// Each decoder implements a specific PDF filter algorithm and can decode
/// compressed or encoded stream data.
pub trait StreamDecoder {
    /// Decode the input data.
    ///
    /// # Arguments
    ///
    /// * `input` - The encoded/compressed data
    ///
    /// # Returns
    ///
    /// The decoded data or an error if decoding fails.
    fn decode(&self, input: &[u8]) -> Result<Vec<u8>>;

    /// Decode the input data, failing if the output would exceed `max_output` bytes.
    ///
    /// The default decodes in full and then checks, so the allocation has already
    /// happened when it rejects. Decoders whose output is not bounded by their input
    /// override it to stop as soon as the limit is crossed.
    fn decode_bounded(&self, input: &[u8], max_output: usize) -> Result<Vec<u8>> {
        let output = self.decode(input)?;
        check_output_limit(self.name(), output.len(), max_output)?;
        Ok(output)
    }

    /// Get the name of this decoder (e.g., "FlateDecode").
    fn name(&self) -> &str;
}

pub(crate) fn check_output_limit(filter: &str, output_len: usize, max_output: usize) -> Result<()> {
    if output_len > max_output {
        return Err(Error::Decode(format!(
            "Decompression bomb detected: {filter} output exceeds limit {max_output} bytes"
        )));
    }
    Ok(())
}

/// Normalize a PDF filter name, handling spec abbreviations and case variations.
///
/// PDF Spec: ISO 32000-1:2008, Table 6 — Standard filter abbreviations.
fn normalize_filter_name(name: &str) -> Result<&'static str> {
    match name {
        "FlateDecode" => return Ok("FlateDecode"),
        "ASCIIHexDecode" => return Ok("ASCIIHexDecode"),
        "ASCII85Decode" => return Ok("ASCII85Decode"),
        "LZWDecode" => return Ok("LZWDecode"),
        "RunLengthDecode" => return Ok("RunLengthDecode"),
        "DCTDecode" => return Ok("DCTDecode"),
        "CCITTFaxDecode" => return Ok("CCITTFaxDecode"),
        "JBIG2Decode" => return Ok("JBIG2Decode"),
        "JPXDecode" => return Ok("JPXDecode"),
        "BrotliDecode" => return Ok("BrotliDecode"),
        _ => {}
    }

    // PDF spec abbreviations (Table 6) ~keep
    match name {
        "Fl" => return Ok("FlateDecode"),
        "AHx" => return Ok("ASCIIHexDecode"),
        "A85" => return Ok("ASCII85Decode"),
        "LZW" => return Ok("LZWDecode"),
        "RL" => return Ok("RunLengthDecode"),
        "DCT" => return Ok("DCTDecode"),
        "CCF" => return Ok("CCITTFaxDecode"),
        _ => {}
    }

    let lower = name.to_ascii_lowercase();
    match lower.as_str() {
        "flatedecode" => Ok("FlateDecode"),
        "asciihexdecode" => Ok("ASCIIHexDecode"),
        "ascii85decode" => Ok("ASCII85Decode"),
        "lzwdecode" => Ok("LZWDecode"),
        "runlengthdecode" => Ok("RunLengthDecode"),
        "dctdecode" => Ok("DCTDecode"),
        "ccittfaxdecode" => Ok("CCITTFaxDecode"),
        "jbig2decode" => Ok("JBIG2Decode"),
        "jpxdecode" => Ok("JPXDecode"),
        "brotlidecode" => Ok("BrotliDecode"),
        _ => Err(Error::UnsupportedFilter(name.to_string())),
    }
}

fn create_decoder(filter_name: &str) -> Result<Box<dyn StreamDecoder>> {
    let canonical = normalize_filter_name(filter_name)?;
    Ok(match canonical {
        "FlateDecode" => Box::new(FlateDecoder::default()),
        "ASCIIHexDecode" => Box::new(AsciiHexDecoder),
        "ASCII85Decode" => Box::new(Ascii85Decoder),
        "LZWDecode" => Box::new(LzwDecoder),
        "RunLengthDecode" => Box::new(RunLengthDecoder),
        "DCTDecode" => Box::new(DctDecoder),
        "CCITTFaxDecode" => Box::new(CcittFaxDecoder),
        "JBIG2Decode" => Box::new(Jbig2Decoder),
        "JPXDecode" => Box::new(jpx::JpxDecoder),
        "BrotliDecode" => Box::new(BrotliDecoder),
        // normalize_filter_name already returns Err for unknown filters ~keep
        _ => unreachable!(),
    })
}

/// Decode stream data using a filter pipeline.
///
/// PDF streams can have multiple filters applied in sequence. This function
/// applies each filter in order to decode the data.
///
/// # Arguments
///
/// * `data` - The raw stream data
/// * `filters` - List of filter names to apply in order
///
/// # Returns
///
/// The fully decoded data or an error if any filter fails.
///
/// # Examples
///
/// ```rust,no_run
/// use xberg_native_pdf::decoders::decode_stream;
///
/// let compressed_data = vec![/* compressed bytes */];
/// let filters = vec!["FlateDecode".to_string()];
/// let decoded = decode_stream(&compressed_data, &filters).unwrap();
/// ```
pub fn decode_stream(data: &[u8], filters: &[String]) -> Result<Vec<u8>> {
    decode_stream_with_params(data, filters, None)
}

/// Decode stream data with parser options (includes decompression bomb protection).
///
/// This function extends `decode_stream` by supporting parser options for
/// security limits and strict mode behavior.
///
/// # Arguments
///
/// * `data` - The raw stream data
/// * `filters` - List of filter names to apply in order
/// * `params` - Optional decode parameters (for predictors, etc.)
/// * `options` - Parser options for security limits
///
/// # Returns
///
/// The fully decoded data or an error if any filter fails or security limits are exceeded.
///
/// # Security
///
/// This function includes decompression bomb protection:
/// - Checks decompression ratio after each filter
/// - Checks the absolute output size limit after decompression, unconditionally
/// - Uses limits from `options` or defaults if None
pub fn decode_stream_with_options(
    data: &[u8],
    filters: &[String],
    params: Option<&DecodeParams>,
    options: Option<&ParserOptions>,
) -> Result<Vec<u8>> {
    decode_stream_with_options_and_expected_size(data, filters, params, options, None)
}

fn decode_stream_with_options_and_expected_size(
    data: &[u8],
    filters: &[String],
    params: Option<&DecodeParams>,
    options: Option<&ParserOptions>,
    expected_filter_output_size: Option<usize>,
) -> Result<Vec<u8>> {
    let max_ratio = options
        .map(|o| o.max_decompression_ratio)
        .unwrap_or(DEFAULT_MAX_DECOMPRESSION_RATIO);
    let max_size = options
        .map(|o| o.max_decompressed_size)
        .unwrap_or_else(default_max_decompressed_size);
    let output_limit = if max_size > 0 { max_size } else { usize::MAX };

    let compressed_size = data.len();
    let mut current = data.to_vec();

    for (filter_index, filter_name) in filters.iter().enumerate() {
        let decoder = create_decoder(filter_name)?;

        current = decoder.decode_bounded(&current, output_limit)?;

        // SECURITY: Check decompression ratio after each filter. Image callers may
        // provide the exact byte count implied by Width x Height x components x bpc
        // (including predictor framing). A matching raster expansion is bounded by
        // document dimensions and the absolute cap below, so ratio alone is not a
        // useful bomb signal for it. Arbitrary streams receive no exception.
        // PDF Spec: ISO 32000-1:2008 does not specify limits, but this is a
        // critical security measure to prevent decompression bomb attacks. ~keep
        let is_final_filter = filter_index + 1 == filters.len();
        let matches_expected_size = is_final_filter && expected_filter_output_size == Some(current.len());
        if max_ratio > 0 && compressed_size > 0 && !matches_expected_size {
            let ratio = current.len() as u64 / compressed_size.max(1) as u64;
            if ratio > max_ratio as u64 {
                return Err(Error::Decode(format!(
                    "Decompression bomb detected: ratio {}:1 exceeds limit {}:1 (compressed: {} bytes, decompressed: {} bytes)",
                    ratio,
                    max_ratio,
                    compressed_size,
                    current.len()
                )));
            }
        }

        // SECURITY: Check maximum decompressed size ~keep
        if max_size > 0 && current.len() > max_size {
            return Err(Error::Decode(format!(
                "Decompression bomb detected: decompressed size {} bytes exceeds limit {} bytes",
                current.len(),
                max_size
            )));
        }
    }

    if let Some(params) = params
        && params.predictor != 1
    {
        current = decode_predictor(&current, params)?;
    }

    Ok(current)
}

/// Decode stream data using a filter pipeline with optional decode parameters.
///
/// This function extends `decode_stream` by supporting decode parameters
/// (e.g., PNG predictors) that are applied after the main filters.
///
/// This is the entry point used by every production call site (object and
/// xref-stream decoding), neither of which has a `ParserOptions` to thread
/// through. It delegates to [`decode_stream_with_options`] with `options:
/// None` so those callers still get the default decompression-bomb guard
/// (100:1 ratio, 256 MB output cap by default — overridable via
/// `XBERG_NATIVE_PDF_MAX_DECOMPRESS_MB`) rather than none at all — without that,
/// chained filters like `RunLengthDecode` or `LZWDecode` have no cap of their
/// own and a few KB of input can expand by orders of magnitude before this
/// function ever returns.
///
/// # Arguments
///
/// * `data` - The raw stream data
/// * `filters` - List of filter names to apply in order
/// * `params` - Optional decode parameters (for predictors, etc.)
///
/// # Returns
///
/// The fully decoded data or an error if any filter fails or the default
/// decompression-bomb limits are exceeded.
pub fn decode_stream_with_params(data: &[u8], filters: &[String], params: Option<&DecodeParams>) -> Result<Vec<u8>> {
    decode_stream_with_options(data, filters, params, None)
}

/// Decode an image stream while allowing a high-ratio expansion only when it
/// exactly matches the byte count implied by the image dictionary.
pub(crate) fn decode_stream_with_params_and_expected_size(
    data: &[u8],
    filters: &[String],
    params: Option<&DecodeParams>,
    expected_filter_output_size: usize,
) -> Result<Vec<u8>> {
    decode_stream_with_options_and_expected_size(data, filters, params, None, Some(expected_filter_output_size))
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::Compression;
    use flate2::write::ZlibEncoder;
    use std::io::Write;

    #[test]
    fn test_decode_stream_no_filters() {
        let data = b"Hello, World!";
        let result = decode_stream(data, &[]).unwrap();
        assert_eq!(result, data);
    }

    #[test]
    fn test_decode_stream_unsupported_filter() {
        let data = b"test";
        let filters = vec!["UnsupportedFilter".to_string()];
        let result = decode_stream(data, &filters);
        assert!(result.is_err());
        match result {
            Err(crate::error::Error::UnsupportedFilter(name)) => {
                assert_eq!(name, "UnsupportedFilter");
            }
            _ => panic!("Expected UnsupportedFilter error"),
        }
    }

    #[test]
    fn test_decode_stream_pipeline() {
        let data = b"48656C6C6F";
        let filters = vec!["ASCIIHexDecode".to_string()];
        let result = decode_stream(data, &filters).unwrap();
        assert_eq!(result, b"Hello");
    }

    #[test]
    fn test_normalize_filter_abbreviations() {
        assert_eq!(normalize_filter_name("A85").unwrap(), "ASCII85Decode");
        assert_eq!(normalize_filter_name("AHx").unwrap(), "ASCIIHexDecode");
        assert_eq!(normalize_filter_name("LZW").unwrap(), "LZWDecode");
        assert_eq!(normalize_filter_name("Fl").unwrap(), "FlateDecode");
        assert_eq!(normalize_filter_name("RL").unwrap(), "RunLengthDecode");
        assert_eq!(normalize_filter_name("CCF").unwrap(), "CCITTFaxDecode");
        assert_eq!(normalize_filter_name("DCT").unwrap(), "DCTDecode");
    }

    #[test]
    fn test_normalize_filter_case_insensitive() {
        assert_eq!(normalize_filter_name("Flatedecode").unwrap(), "FlateDecode");
        assert_eq!(normalize_filter_name("FLATEDECODE").unwrap(), "FlateDecode");
        assert_eq!(normalize_filter_name("flatedecode").unwrap(), "FlateDecode");
        assert_eq!(normalize_filter_name("ascii85decode").unwrap(), "ASCII85Decode");
        assert_eq!(normalize_filter_name("ASCIIHEXDECODE").unwrap(), "ASCIIHexDecode");
    }

    #[test]
    fn test_normalize_filter_unknown() {
        let result = normalize_filter_name("BogusFilter");
        assert!(result.is_err());
        match result {
            Err(crate::error::Error::UnsupportedFilter(name)) => {
                assert_eq!(name, "BogusFilter");
            }
            _ => panic!("Expected UnsupportedFilter error"),
        }
    }

    #[test]
    fn test_decode_stream_with_abbreviation() {
        let data = b"48656C6C6F";
        let filters = vec!["AHx".to_string()];
        let result = decode_stream(data, &filters).unwrap();
        assert_eq!(result, b"Hello");
    }

    /// GH: this crate's `crates/xberg/tests/fixtures/ocr/scanned_hello.pdf` is a plain,
    /// mostly-white scanned page whose ASCII85+Flate image stream decodes to exactly
    /// 4,200,000 bytes — a 1000x1400 RGB raster — at a cumulative ratio of 170:1 and a
    /// Flate-stage ratio of 182:1, both well past the 100:1 `DEFAULT_MAX_DECOMPRESSION_RATIO`.
    /// Before gating the ratio check on `MIN_SIZE_FOR_RATIO_CHECK`, this
    /// stream was rejected outright as a "decompression bomb", silently
    /// blanking the rendered page (`render_image` returns `Err`, the page
    /// renders all-white, OCR correctly finds no text on a blank page). A
    /// solid-colour 4.2 MB payload compresses to a tiny fraction of its size
    /// too, so this reproduces the same "small absolute output, high ratio"
    /// shape without needing the real fixture bytes.
    #[test]
    fn small_high_ratio_image_stream_decodes_despite_ratio_over_limit() {
        const DECOMPRESSED_LEN: usize = 4_200_000; // 1000 * 1400 * 3, matching the RGB fixture

        let original = vec![0xFFu8; DECOMPRESSED_LEN];
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&original).unwrap();
        let compressed = encoder.finish().unwrap();

        // Sanity-check the test setup: a solid-colour payload must compress well past the
        // 100:1 default ratio limit, otherwise this test would not exercise the guard at all.
        let ratio = DECOMPRESSED_LEN as u64 / compressed.len().max(1) as u64;
        assert!(
            ratio > DEFAULT_MAX_DECOMPRESSION_RATIO as u64,
            "test setup must produce a ratio over the default limit to be meaningful, got {ratio}:1"
        );

        let filters = vec!["FlateDecode".to_string()];
        let result = decode_stream_with_params_and_expected_size(&compressed, &filters, None, DECOMPRESSED_LEN)
            .expect("a 4.2 MB image stream must decode despite exceeding the ratio limit");
        assert_eq!(
            result.len(),
            DECOMPRESSED_LEN,
            "decoded output must be the full 4,200,000-byte raster, not truncated or rejected"
        );
    }

    #[test]
    fn high_ratio_stream_with_mismatched_image_size_is_rejected() {
        const DECOMPRESSED_LEN: usize = 4_200_000;

        let original = vec![0xFFu8; DECOMPRESSED_LEN];
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&original).unwrap();
        let compressed = encoder.finish().unwrap();
        let filters = vec!["FlateDecode".to_string()];

        let error = decode_stream_with_params_and_expected_size(&compressed, &filters, None, DECOMPRESSED_LEN + 1)
            .expect_err("a high-ratio stream that does not match the raster dimensions must be rejected");
        assert!(error.to_string().contains("Decompression bomb detected: ratio"));
    }

    /// A 2480 x 3506 RGB raster is an A4 page at roughly 300 dpi. Mostly-white
    /// scans of this size legitimately exceed the default 100:1 ratio, but the
    /// decoded allocation is bounded and must not be mistaken for a bomb.
    #[test]
    fn a4_rgb_scan_decodes_despite_ratio_over_limit() {
        const WIDTH: usize = 2480;
        const HEIGHT: usize = 3506;
        const RGB_COMPONENTS: usize = 3;
        const DECOMPRESSED_LEN: usize = WIDTH * HEIGHT * RGB_COMPONENTS;
        let original = vec![0xFFu8; DECOMPRESSED_LEN];
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&original).unwrap();
        let compressed = encoder.finish().unwrap();
        let ratio = DECOMPRESSED_LEN as u64 / compressed.len().max(1) as u64;
        assert!(ratio > DEFAULT_MAX_DECOMPRESSION_RATIO as u64);

        let filters = vec!["FlateDecode".to_string()];
        let result = decode_stream_with_params_and_expected_size(&compressed, &filters, None, DECOMPRESSED_LEN)
            .expect("a bounded 300-dpi RGB scan must not be rejected as a decompression bomb");
        assert_eq!(result.len(), DECOMPRESSED_LEN);
    }

    /// A genuine decompression bomb — high ratio AND an absolute output large enough to
    /// matter — must still be rejected. This is the guardrail test: a fix that merely
    /// raised or removed the ratio limit would pass the test above while leaving actual
    /// bombs unguarded. The output here (64 MB) sits below the absolute cap
    /// [`default_max_decompressed_size`] resolves to, so rejection can only come from the
    /// ratio check — proving the ratio guard itself still fires once output clears the
    /// floor, rather than having been disabled altogether.
    #[test]
    fn generic_bomb_below_absolute_cap_is_rejected_by_ratio_check() {
        const DECOMPRESSED_LEN: usize = 64 * 1024 * 1024;
        assert!(DECOMPRESSED_LEN < default_max_decompressed_size());

        let original = vec![0u8; DECOMPRESSED_LEN];
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&original).unwrap();
        let compressed = encoder.finish().unwrap();

        let filters = vec!["FlateDecode".to_string()];
        let result = decode_stream_with_options(&compressed, &filters, None, None);

        match result {
            Err(Error::Decode(message)) => {
                assert!(
                    message.contains("Decompression bomb detected: ratio"),
                    "a bomb above the floor and below the absolute cap must be caught by the \
                     ratio check specifically, got: {message}"
                );
            }
            other => panic!("expected a ratio-based Decode error, got: {other:?}"),
        }
    }

    /// GH#1754. The pipeline's implicit absolute cap (the one used when a caller passes
    /// no `ParserOptions`, which every page-content decode does) was a hardcoded 100 MB
    /// while `FlateDecoder`'s own ceiling is 256 MB. A stream the flate stage had just
    /// produced and accepted was therefore rejected one line later, and the caller —
    /// per-page text extraction — turned that `Err` into an empty page with nothing but
    /// a warning. The carrier was page 574 of a statistics textbook: a 24 MB content
    /// stream decoding to 194 MB at a ratio of 8:1, which is nothing like a bomb.
    ///
    /// This reproduces the shape at the smallest size that crosses the old boundary:
    /// output just past 100 MB at a ratio far under the 100:1 bomb threshold.
    /// `Compression::none()` gives that ratio for free — stored deflate blocks, so
    /// both the encode and the decode run at memcpy speed — and the payload is fed in
    /// chunks so no third copy of it is ever materialised.
    #[test]
    fn should_accept_a_large_low_ratio_stream_the_flate_stage_itself_allows() {
        const OLD_PIPELINE_CAP: usize = 100 * 1024 * 1024;
        const CHUNK: usize = 1024 * 1024;
        const CHUNKS: usize = OLD_PIPELINE_CAP / CHUNK + 1;
        const DECOMPRESSED_LEN: usize = CHUNK * CHUNKS;

        const {
            assert!(
                DECOMPRESSED_LEN > OLD_PIPELINE_CAP,
                "the fixture must cross the cap it exists to test"
            )
        };
        assert!(
            DECOMPRESSED_LEN < default_max_decompressed_size(),
            "the default cap must be the flate stage's own ceiling, not a second lower one"
        );

        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::none());
        let chunk = vec![0u8; CHUNK];
        for _ in 0..CHUNKS {
            encoder.write_all(&chunk).unwrap();
        }
        drop(chunk);
        let compressed = encoder.finish().unwrap();

        let ratio = DECOMPRESSED_LEN as u64 / compressed.len().max(1) as u64;
        assert!(
            ratio <= DEFAULT_MAX_DECOMPRESSION_RATIO as u64,
            "the fixture must be rejectable only by the absolute cap, not the ratio check; ratio was {ratio}:1"
        );

        let filters = vec!["FlateDecode".to_string()];
        let decoded = decode_stream_with_options(&compressed, &filters, None, None)
            .expect("a large, low-ratio stream inside the flate ceiling must not be read as a bomb");
        assert_eq!(decoded.len(), DECOMPRESSED_LEN);
    }

    /// GH#1764. 64:1 is under the ratio threshold, so the only thing that can reject
    /// this stream is the size cap, and the message names the decoder that enforced it.
    #[test]
    fn should_pass_the_size_cap_into_the_decoder() {
        let data = [129, b'A'].repeat(16);
        let filters = vec!["RunLengthDecode".to_string()];
        let options = ParserOptions {
            max_decompressed_size: 1_000,
            ..ParserOptions::default()
        };

        let error = decode_stream_with_options(&data, &filters, None, Some(&options)).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("RunLengthDecode output exceeds limit 1000 bytes"),
            "got: {error}"
        );
    }

    #[test]
    fn should_treat_a_zero_size_cap_as_no_limit() {
        let data = [129, b'A'].repeat(16);
        let filters = vec!["RunLengthDecode".to_string()];
        let options = ParserOptions {
            max_decompressed_size: 0,
            ..ParserOptions::default()
        };

        let decoded = decode_stream_with_options(&data, &filters, None, Some(&options)).unwrap();
        assert_eq!(decoded.len(), 16 * 128);
    }
}
