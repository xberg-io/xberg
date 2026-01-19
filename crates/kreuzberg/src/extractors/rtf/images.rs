//! Image metadata extraction from RTF documents.

use crate::extractors::rtf::encoding::parse_rtf_control_word;

/// Extract image metadata from within a \pict group.
///
/// Looks for image type (jpegblip, pngblip, etc.) and dimensions.
pub fn extract_image_metadata(chars: &mut std::iter::Peekable<std::str::Chars>) -> String {
    let mut metadata = String::new();
    let mut image_type: Option<&str> = None;
    let mut width_goal: Option<i32> = None;
    let mut height_goal: Option<i32> = None;
    let mut depth = 0;

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
                    "jpegblip" => image_type = Some("jpg"),
                    "pngblip" => image_type = Some("png"),
                    "wmetafile" => image_type = Some("wmf"),
                    "dibitmap" => image_type = Some("bmp"),
                    "picwgoal" => width_goal = value,
                    "pichgoal" => height_goal = value,
                    "bin" => break,
                    _ => {}
                }
            }
            ' ' => {
                chars.next();
            }
            _ => {
                chars.next();
            }
        }
    }

    if let Some(itype) = image_type {
        metadata.push_str("image.");
        metadata.push_str(itype);
    }

    if let Some(width) = width_goal {
        let width_inches = f64::from(width) / 1440.0;
        metadata.push_str(&format!(" width=\"{:.1}in\"", width_inches));
    }

    if let Some(height) = height_goal {
        let height_inches = f64::from(height) / 1440.0;
        metadata.push_str(&format!(" height=\"{:.1}in\"", height_inches));
    }

    if metadata.is_empty() {
        metadata.push_str("image.jpg");
    }

    metadata
}
