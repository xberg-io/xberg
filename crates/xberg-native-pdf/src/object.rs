//! PDF object types.

use crate::error::{Error, Result};

/// PDF object representation.
#[derive(Debug, Clone, PartialEq)]
pub enum Object {
    /// Null object
    Null,
    /// Boolean value
    Boolean(bool),
    /// Integer value
    Integer(i64),
    /// Real (floating-point) value
    Real(f64),
    /// String (byte array)
    String(Vec<u8>),
    /// Name (starting with /)
    Name(String),
    /// Array of objects
    Array(Vec<Object>),
    /// Dictionary (key-value pairs)
    Dictionary(std::collections::HashMap<String, Object>),
    /// Stream (dictionary + data)
    Stream {
        /// Stream dictionary
        dict: std::collections::HashMap<String, Object>,
        /// Stream data
        data: bytes::Bytes,
    },
    /// Indirect object reference
    Reference(ObjectRef),
}

/// Reference to an indirect object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize)]
pub struct ObjectRef {
    /// Object number
    pub id: u32,
    /// Generation number
    pub generation: u16,
}

impl ObjectRef {
    /// Create a new object reference.
    pub fn new(id: u32, generation: u16) -> Self {
        Self { id, generation }
    }
}

impl std::fmt::Display for ObjectRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {} R", self.id, self.generation)
    }
}

/// Encode a Rust `&str` as a PDF text string byte sequence.
///
/// Uses PDFDocEncoding (direct code-point byte) for strings whose characters
/// all fall within U+0000–U+00FF.  Falls back to UTF-16BE with leading BOM
/// (`0xFE 0xFF`) for any string that contains a code point above U+00FF.
///
/// This is the encoding required by ISO 32000-2 §7.9.2 for all PDF text
/// strings (metadata fields, annotation text, bookmark titles, form field
/// values, page-label prefixes, etc.).
pub fn encode_pdf_text_string(s: &str) -> Vec<u8> {
    if s.chars().all(|c| (c as u32) <= 0x00FF) {
        s.chars().map(|c| c as u8).collect()
    } else {
        let mut buf = vec![0xFE_u8, 0xFF];
        for unit in s.encode_utf16() {
            buf.push((unit >> 8) as u8);
            buf.push((unit & 0xFF) as u8);
        }
        buf
    }
}

impl Object {
    /// Create a PDF text string object from a Rust string.
    ///
    /// Accepts `&str`, `String`, or any type that implements `AsRef<str>`.
    /// Encodes using PDFDocEncoding for characters within U+0000–U+00FF,
    /// or UTF-16BE with BOM for strings that contain characters above U+00FF.
    pub fn text_string(s: impl AsRef<str>) -> Self {
        Object::String(encode_pdf_text_string(s.as_ref()))
    }

    /// Get the type name of this object (without data).
    ///
    /// Returns a human-readable type name like "String", "Array", "Dictionary", etc.
    /// without including the actual data content.
    pub fn type_name(&self) -> &'static str {
        match self {
            Object::Null => "Null",
            Object::Boolean(_) => "Boolean",
            Object::Integer(_) => "Integer",
            Object::Real(_) => "Real",
            Object::String(_) => "String",
            Object::Name(_) => "Name",
            Object::Array(_) => "Array",
            Object::Dictionary(_) => "Dictionary",
            Object::Stream { .. } => "Stream",
            Object::Reference(_) => "Reference",
        }
    }

    /// Try to cast to integer.
    pub fn as_integer(&self) -> Option<i64> {
        match self {
            Object::Integer(i) => Some(*i),
            _ => None,
        }
    }

    /// Try to cast to name.
    pub fn as_name(&self) -> Option<&str> {
        match self {
            Object::Name(s) => Some(s),
            _ => None,
        }
    }

    /// Try to cast to dictionary. Works for both Dictionary and Stream objects.
    pub fn as_dict(&self) -> Option<&std::collections::HashMap<String, Object>> {
        match self {
            Object::Dictionary(d) => Some(d),
            Object::Stream { dict, .. } => Some(dict),
            _ => None,
        }
    }

    /// Try to cast to array.
    pub fn as_array(&self) -> Option<&Vec<Object>> {
        match self {
            Object::Array(arr) => Some(arr),
            _ => None,
        }
    }

    /// Try to cast to reference.
    pub fn as_reference(&self) -> Option<ObjectRef> {
        match self {
            Object::Reference(r) => Some(*r),
            _ => None,
        }
    }

    /// Try to cast to boolean.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Object::Boolean(b) => Some(*b),
            _ => None,
        }
    }

    /// Try to cast to real number.
    pub fn as_real(&self) -> Option<f64> {
        match self {
            Object::Real(r) => Some(*r),
            _ => None,
        }
    }

    /// Try to cast to string (bytes).
    pub fn as_string(&self) -> Option<&[u8]> {
        match self {
            Object::String(s) => Some(s),
            _ => None,
        }
    }

    /// Check if object is null.
    pub fn is_null(&self) -> bool {
        matches!(self, Object::Null)
    }

    /// Decode stream data using filters specified in the stream dictionary.
    ///
    /// This is a convenience method that calls `decode_stream_data_with_decryption`
    /// with no encryption parameters.
    ///
    /// # Returns
    ///
    /// The decoded stream data, or an error if this is not a stream object
    /// or if decoding fails.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use xberg_native_pdf::object::Object;
    ///
    /// # fn example(stream_obj: Object) -> Result<(), Box<dyn std::error::Error>> {
    /// let decoded_data = stream_obj.decode_stream_data()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn decode_stream_data(&self) -> Result<Vec<u8>> {
        self.decode_stream_data_with_decryption(None, 0, 0)
    }

    pub(crate) fn decode_image_stream_data(&self, expected_filter_output_size: Option<usize>) -> Result<Vec<u8>> {
        self.decode_stream_data_with_decryption_and_expected_size(None, 0, 0, expected_filter_output_size, true)
    }

    /// Decode stream data with optional decryption.
    ///
    /// PDF Spec: Section 7.6.2 - General Encryption Algorithm states that streams
    /// must be decrypted BEFORE applying filters (decompression).
    ///
    /// # Arguments
    ///
    /// * `decryption_fn` - Optional decryption function (from EncryptionHandler)
    /// * `obj_num` - Object number (for encryption key derivation)
    /// * `gen_num` - Generation number (for encryption key derivation)
    ///
    /// # Returns
    ///
    /// The decoded stream data, or an error if decoding/decryption fails.
    pub fn decode_stream_data_with_decryption(
        &self,
        decryption_fn: Option<&dyn Fn(&[u8]) -> Result<Vec<u8>>>,
        obj_num: u32,
        gen_num: u32,
    ) -> Result<Vec<u8>> {
        self.decode_stream_data_with_decryption_and_expected_size(decryption_fn, obj_num, gen_num, None, false)
    }

    pub(crate) fn decode_stream_data_with_decryption_and_expected_size(
        &self,
        decryption_fn: Option<&dyn Fn(&[u8]) -> Result<Vec<u8>>>,
        obj_num: u32,
        gen_num: u32,
        expected_filter_output_size: Option<usize>,
        image_stream: bool,
    ) -> Result<Vec<u8>> {
        match self {
            Object::Stream { dict, data } => {
                // Step 1: Decrypt stream data BEFORE applying filters
                // PDF Spec: Section 7.6.2 - Encryption must be applied before compression ~keep
                let decrypted_data = decrypt_stream_data(data, decryption_fn, obj_num, gen_num)?;

                // Step 2: Apply filters (decompression)
                // PDF Spec: ISO 32000-1:2008, Section 7.3.8.2 - Stream Objects ~keep
                let filters = dict.get("Filter").map(extract_filter_names).unwrap_or_default();

                if filters.is_empty() {
                    Ok(decrypted_data)
                } else {
                    // Get decode parameters if present
                    // PDF Spec: ISO 32000-1:2008, Section 7.4.2 - DecodeParms ~keep
                    if image_stream
                        && let Some(predictors) = filter_predictor_params(dict.get("Filter"), dict.get("DecodeParms"))
                    {
                        return crate::decoders::decode_stream_with_filter_predictors(
                            &decrypted_data,
                            &filters,
                            &predictors,
                            expected_filter_output_size,
                        );
                    }
                    let decode_params = extract_decode_params(dict.get("DecodeParms"));

                    if let Some(expected_size) = expected_filter_output_size {
                        crate::decoders::decode_stream_with_params_and_expected_size(
                            &decrypted_data,
                            &filters,
                            decode_params.as_ref(),
                            expected_size,
                        )
                    } else {
                        crate::decoders::decode_stream_with_params(&decrypted_data, &filters, decode_params.as_ref())
                    }
                }
            }
            Object::Dictionary(dict) => {
                // Per ISO 32000, every stream is a dictionary. Some PDFs (e.g.,
                // SafeDocs Dialect-StreamIsDict.pdf) store objects as plain
                // dictionaries where a stream is expected. Treat as empty stream. ~keep
                tracing::warn!("dictionary used where stream expected; treating as empty stream");
                let filters = dict.get("Filter").map(extract_filter_names).unwrap_or_default();
                if filters.is_empty() {
                    Ok(Vec::new())
                } else {
                    let decode_params = extract_decode_params(dict.get("DecodeParms"));
                    crate::decoders::decode_stream_with_params(&[], &filters, decode_params.as_ref())
                }
            }
            _ => Err(Error::InvalidObjectType {
                expected: "Stream".to_string(),
                found: self.type_name().to_string(),
            }),
        }
    }
}

/// Decrypts a stream's raw bytes before any filter is applied, or returns them unchanged
/// when there is no decryption function (per ISO 32000-1:2008 §7.6.2, decryption happens
/// before decompression).
///
/// IMPORTANT: for encrypted streams, whitespace must NOT be trimmed before decryption —
/// encrypted data is binary and trimming could corrupt it (especially for AES, where the
/// first 16 bytes are the IV). Unencrypted `data` is likewise returned untrimmed: the
/// parser's `parse_stream_data` already consumes exactly one EOL after the `stream`
/// keyword per §7.3.8.1, so `data` begins with the first byte of actual stream content —
/// re-trimming CR/LF here would corrupt binary streams that legitimately start with 0x0A
/// or 0x0D (e.g. an Indexed palette whose first CMYK byte is 0x0D). ~keep
///
/// Split out of [`Object::decode_stream_data_with_decryption_and_expected_size`] purely
/// to shorten that method.
fn decrypt_stream_data(
    data: &[u8],
    decryption_fn: Option<&dyn Fn(&[u8]) -> Result<Vec<u8>>>,
    obj_num: u32,
    gen_num: u32,
) -> Result<Vec<u8>> {
    let Some(decrypt) = decryption_fn else {
        return Ok(data.to_vec());
    };

    tracing::debug!(
        object_id = obj_num,
        generation = gen_num,
        bytes = data.len(),
        "decrypting stream"
    );
    match decrypt(data) {
        Ok(data) => {
            tracing::debug!(bytes = data.len(), "decryption successful");
            Ok(data)
        }
        Err(error) => {
            tracing::error!(
                object_id = obj_num,
                generation = gen_num,
                error_code = error.telemetry_code(),
                error_offset = ?error.telemetry_offset(),
                "decryption failed"
            );
            Err(error)
        }
    }
}

/// Extract filter names from a Filter object.
///
/// The Filter entry can be either:
/// - A single Name (e.g., /FlateDecode)
/// - An Array of Names (e.g., [/ASCII85Decode /FlateDecode])
fn extract_filter_names(filter_obj: &Object) -> Vec<String> {
    match filter_obj {
        Object::Name(name) => vec![name.clone()],
        Object::Array(arr) => arr
            .iter()
            .filter_map(|obj| obj.as_name().map(|s| s.to_string()))
            .collect(),
        _ => vec![],
    }
}

pub(crate) fn filter_predictor_params(
    filters: Option<&Object>,
    params: Option<&Object>,
) -> Option<Vec<Option<crate::decoders::DecodeParams>>> {
    let (Object::Array(filters), Object::Array(params)) = (filters?, params?) else {
        return None;
    };
    if filters.iter().any(|filter| filter.as_name().is_none()) {
        return None;
    }
    Some(
        filters
            .iter()
            .enumerate()
            .map(|(index, filter)| {
                let name = filter.as_name()?;
                if !crate::decoders::supports_filter_predictor(name) {
                    return None;
                }
                let Object::Dictionary(dict) = params.get(index)? else {
                    return None;
                };
                Some(parse_predictor_params(dict))
            })
            .collect(),
    )
}

/// Extract decode parameters from a DecodeParms object.
///
/// PDF Spec: ISO 32000-1:2008, Section 7.4.2 - LZWDecode and FlateDecode Parameters
///
/// The DecodeParms entry can be:
/// - A dictionary (for single filter)
/// - An array of dictionaries (for multiple filters)
/// - Null or absent (no parameters)
///
/// This function extracts predictor parameters used for PNG/TIFF encoding.
pub(crate) fn extract_decode_params(params_obj: Option<&Object>) -> Option<crate::decoders::DecodeParams> {
    let dict = match params_obj? {
        Object::Dictionary(d) => d,
        Object::Array(arr) => arr.iter().filter_map(|obj| obj.as_dict()).next()?,
        _ => return None,
    };

    Some(parse_predictor_params(dict))
}

fn parse_predictor_params(dict: &std::collections::HashMap<String, Object>) -> crate::decoders::DecodeParams {
    // Extract predictor parameters per PDF Spec Table 3.7 ~keep
    let predictor = dict.get("Predictor").and_then(|obj| obj.as_integer()).unwrap_or(1); // Default: no prediction ~keep

    let columns = dict.get("Columns").and_then(|obj| obj.as_integer()).unwrap_or(1) as usize;

    let colors = dict.get("Colors").and_then(|obj| obj.as_integer()).unwrap_or(1) as usize;

    let bits_per_component = dict
        .get("BitsPerComponent")
        .and_then(|obj| obj.as_integer())
        .unwrap_or(8) as usize;

    crate::decoders::DecodeParams {
        predictor,
        columns,
        colors,
        bits_per_component,
    }
}

/// Extract CCITT-specific decode parameters from a DecodeParms object.
///
/// PDF Spec: ISO 32000-1:2008, Section 7.4.6 - CCITTFaxDecode Filter Parameters
///
/// The DecodeParms entry can be:
/// - A dictionary (for single filter)
/// - An array of dictionaries (for multiple filters)
/// - Null or absent (no parameters, use defaults)
///
/// CCITT parameters:
/// - /K: Group indicator (-1=Group 4, 0=Group 3 1-D, >0=Group 3 2-D)
/// - /Columns: Image width in pixels
/// - /Rows: Image height in pixels (optional)
/// - /BlackIs1: Pixel interpretation (false=white is 0, true=white is 1)
/// - /EndOfLine: Include EOL code (default false)
/// - /EncodedByteAlign: Byte-aligned encoding (default false)
/// - /EndOfBlock: Include RTC code (default true)
pub fn extract_ccitt_params(params_obj: Option<&Object>) -> Option<crate::decoders::CcittParams> {
    extract_ccitt_params_with_width(params_obj, None)
}

/// Extract CCITT decompression parameters from a PDF object with optional width override.
///
/// This function extracts CCITT Group 3 or Group 4 decompression parameters from a PDF
/// /DecodeParms dictionary. If image_width is provided, it will be used as the /Columns
/// parameter, overriding any value in the dictionary.
///
/// # Arguments
/// * `params_obj` - Optional PDF object containing CCITT parameters (Dictionary or Array)
/// * `image_width` - Optional width override to use as /Columns parameter
///
/// # Returns
/// Some(CcittParams) if valid parameters are found, None otherwise
pub fn extract_ccitt_params_with_width(
    params_obj: Option<&Object>,
    image_width: Option<u32>,
) -> Option<crate::decoders::CcittParams> {
    let dict = match params_obj? {
        Object::Dictionary(dict) => dict,
        Object::Array(params) => params.iter().find_map(Object::as_dict)?,
        _ => return None,
    };
    ccitt_params_from_dict(dict, image_width)
}

pub(crate) fn extract_ccitt_params_with_width_at_index(
    params_obj: Option<&Object>,
    filter_index: usize,
    image_width: Option<u32>,
) -> Option<crate::decoders::CcittParams> {
    let dict = match params_obj? {
        Object::Dictionary(d) => d,
        Object::Array(arr) => arr.get(filter_index)?.as_dict()?,
        _ => return None,
    };
    ccitt_params_from_dict(dict, image_width)
}

fn ccitt_params_from_dict(
    dict: &std::collections::HashMap<String, Object>,
    image_width: Option<u32>,
) -> Option<crate::decoders::CcittParams> {
    let k = dict.get("K").and_then(|obj| obj.as_integer()).unwrap_or(0);

    let columns = dict
        .get("Columns")
        .and_then(|obj| obj.as_integer())
        .map(|v| v as u32)
        .or(image_width)
        .unwrap_or(1);

    let rows = dict.get("Rows").and_then(|obj| obj.as_integer()).map(|v| v as u32);

    let black_is_1 = dict.get("BlackIs1").and_then(|obj| obj.as_bool()).unwrap_or(false);

    let end_of_line = dict.get("EndOfLine").and_then(|obj| obj.as_bool()).unwrap_or(false);

    let encoded_byte_align = dict
        .get("EncodedByteAlign")
        .and_then(|obj| obj.as_bool())
        .unwrap_or(false);

    let end_of_block = dict.get("EndOfBlock").and_then(|obj| obj.as_bool()).unwrap_or(true);

    Some(crate::decoders::CcittParams {
        k,
        columns,
        rows,
        black_is_1,
        end_of_line,
        encoded_byte_align,
        end_of_block,
    })
}

#[cfg(test)]
mod tests;
