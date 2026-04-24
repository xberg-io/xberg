//! Image metadata and data extraction from RTF documents.

use crate::extractors::rtf::encoding::parse_rtf_control_word;

/// Parsed image data from a `\pict` group.
pub struct RtfImage {
    /// Image format string (e.g., "jpeg", "png", "wmf", "bmp").
    pub format: &'static str,
    /// Decoded binary image data.
    pub data: Vec<u8>,
}

/// Extract image metadata and binary data from within a `\pict` group.
///
/// Parses the image type (`\jpegblip`, `\pngblip`, etc.), dimensions, and
/// collects the hex-encoded image data that follows the control words.
/// Returns the parsed image and a metadata string for text representation.
pub(crate) fn extract_pict_image(chars: &mut std::iter::Peekable<std::str::Chars>) -> (String, Option<RtfImage>) {
    let mut metadata = String::new();
    let mut image_type: Option<&str> = None;
    let mut format: &str = "jpeg"; // default
    let mut depth = 0;
    let mut hex_chars = String::new();
    let mut _has_bin = false;

    while let Some(&ch) = chars.peek() {
        match ch {
            '{' => {
                depth += 1;
                chars.next();
            }
            '}' => {
                if depth == 0 {
                    break;
                }
                depth -= 1;
                chars.next();
            }
            '\\' => {
                chars.next();
                let (control_word, value) = parse_rtf_control_word(chars);

                match control_word.as_str() {
                    "jpegblip" => {
                        image_type = Some("jpg");
                        format = "jpeg";
                    }
                    "pngblip" => {
                        image_type = Some("png");
                        format = "png";
                    }
                    "wmetafile" => {
                        image_type = Some("wmf");
                        format = "wmf";
                    }
                    "dibitmap" => {
                        image_type = Some("bmp");
                        format = "bmp";
                    }
                    "picwgoal" | "pichgoal" => {}
                    "bin" => {
                        // \binN means N raw binary bytes follow. Skip them.
                        if let Some(count) = value {
                            let count = count.max(0) as usize;
                            for _ in 0..count {
                                chars.next();
                            }
                            _has_bin = true;
                        }
                        // Without a count parameter, \bin is non-standard.
                        // Continue parsing — hex data that follows will be
                        // collected normally.
                    }
                    _ => {}
                }
            }
            ' ' | '\r' | '\n' => {
                chars.next();
            }
            _ => {
                // Hex data characters
                if ch.is_ascii_hexdigit() {
                    hex_chars.push(ch);
                }
                chars.next();
            }
        }
    }

    // Build metadata string for text representation
    if let Some(itype) = image_type {
        metadata.push_str("image.");
        metadata.push_str(itype);
    }

    if metadata.is_empty() {
        metadata.push_str("image.jpg");
    }

    // Decode hex data to binary. When \bin was used with a count,
    // the binary data was already skipped; hex_chars may still contain
    // hex-encoded image data collected from the group.
    let image = if !hex_chars.is_empty() {
        match hex::decode(&hex_chars) {
            Ok(data) if !data.is_empty() => Some(RtfImage { format, data }),
            _ => None,
        }
    } else {
        None
    };

    (metadata, image)
}

