//! Email extraction functions.
//!
//! Parses .eml (RFC822) and .msg (Outlook) email files using `mail-parser`.
//! Extracts message content, headers, and attachment information.
//!
//! # Features
//!
//! - **EML support**: RFC822 format parsing
//! - **HTML to text**: Strips HTML tags from HTML email bodies
//! - **Metadata extraction**: Sender, recipients, subject, message ID
//! - **Attachment list**: Names of all attachments (content not extracted)
//!
//! # Example
//!
//! ```rust,no_run
//! use kreuzberg::extraction::email::parse_eml_content;
//!
//! # fn example() -> kreuzberg::Result<()> {
//! let eml_bytes = std::fs::read("message.eml")?;
//! let result = parse_eml_content(&eml_bytes)?;
//!
//! println!("From: {:?}", result.from_email);
//! println!("Subject: {:?}", result.subject);
//! # Ok(())
//! # }
//! ```
use bytes::Bytes;
use encoding_rs::Encoding;

use crate::error::{KreuzbergError, Result};
use crate::text::utf8_validation;
use crate::types::{EmailAttachment, EmailExtractionResult};
use mail_parser::MimeHeaders;
use regex::Regex;
use std::collections::HashMap;
use std::sync::OnceLock;

static HTML_TAG_RE: OnceLock<Regex> = OnceLock::new();
static SCRIPT_RE: OnceLock<Regex> = OnceLock::new();
static STYLE_RE: OnceLock<Regex> = OnceLock::new();
static WHITESPACE_RE: OnceLock<Regex> = OnceLock::new();

fn html_tag_regex() -> &'static Regex {
    HTML_TAG_RE.get_or_init(|| Regex::new(r"<[^>]+>").unwrap())
}

fn script_regex() -> &'static Regex {
    SCRIPT_RE.get_or_init(|| Regex::new(r"(?is)<script[^>]*>.*?</script>").unwrap())
}

fn style_regex() -> &'static Regex {
    STYLE_RE.get_or_init(|| Regex::new(r"(?is)<style[^>]*>.*?</style>").unwrap())
}

fn whitespace_regex() -> &'static Regex {
    WHITESPACE_RE.get_or_init(|| Regex::new(r"\s+").unwrap())
}

/// Detect UTF-16 encoding (with or without BOM) and transcode to UTF-8 if needed.
///
/// `mail_parser` expects ASCII/UTF-8 input. If the EML file is encoded as
/// UTF-16, we transcode it to UTF-8 first.
///
/// Detection strategy:
/// 1. Check for BOM (`FF FE` = LE, `FE FF` = BE)
/// 2. If no BOM, use heuristic: EML files start with ASCII headers, so
///    alternating zero bytes indicate UTF-16 encoding.
fn maybe_transcode_utf16(data: &[u8]) -> Option<Vec<u8>> {
    if data.len() < 4 {
        return None;
    }

    let (is_le, skip) = if data[0] == 0xFF && data[1] == 0xFE {
        (true, 2)
    } else if data[0] == 0xFE && data[1] == 0xFF {
        (false, 2)
    } else if data[1] == 0x00 && data[3] == 0x00 && data[0] != 0x00 && data[2] != 0x00 {
        // No BOM, but looks like UTF-16 LE (e.g. "M\0I\0M\0E\0")
        (true, 0)
    } else if data[0] == 0x00 && data[2] == 0x00 && data[1] != 0x00 && data[3] != 0x00 {
        // No BOM, but looks like UTF-16 BE (e.g. "\0M\0I\0M\0E")
        (false, 0)
    } else {
        return None;
    };

    let payload = &data[skip..];
    let even_len = payload.len() & !1;
    let u16_iter = (0..even_len).step_by(2).map(|i| {
        if is_le {
            u16::from_le_bytes([payload[i], payload[i + 1]])
        } else {
            u16::from_be_bytes([payload[i], payload[i + 1]])
        }
    });

    match String::from_utf16(&u16_iter.collect::<Vec<u16>>()) {
        Ok(s) => Some(s.into_bytes()),
        Err(_) => None,
    }
}

/// Parse .eml file content (RFC822 format)
pub fn parse_eml_content(data: &[u8]) -> Result<EmailExtractionResult> {
    // Transcode UTF-16 to UTF-8 if a BOM is detected
    let data = if let Some(transcoded) = maybe_transcode_utf16(data) {
        std::borrow::Cow::Owned(transcoded)
    } else {
        std::borrow::Cow::Borrowed(data)
    };

    let message = mail_parser::MessageParser::default()
        .parse(&data)
        .ok_or_else(|| KreuzbergError::parsing("Failed to parse EML file: invalid email format".to_string()))?;

    let subject = message.subject().map(|s| s.to_string());

    let from_email = message.from().and_then(|from| from.first()).and_then(|addr| {
        let email = addr.address()?;
        Some(match addr.name() {
            Some(name) if !name.is_empty() => format!("\"{}\" <{}>", name, email),
            _ => email.to_string(),
        })
    });

    let to_emails: Vec<String> = message
        .to()
        .map(|to| {
            to.iter()
                .filter_map(|addr| {
                    let email = addr.address()?;
                    Some(match addr.name() {
                        Some(name) if !name.is_empty() => format!("\"{}\" <{}>", name, email),
                        _ => email.to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_else(Vec::new);

    let cc_emails: Vec<String> = message
        .cc()
        .map(|cc| {
            cc.iter()
                .filter_map(|addr| {
                    let email = addr.address()?;
                    Some(match addr.name() {
                        Some(name) if !name.is_empty() => format!("\"{}\" <{}>", name, email),
                        _ => email.to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_else(Vec::new);

    let bcc_emails: Vec<String> = message
        .bcc()
        .map(|bcc| {
            bcc.iter()
                .filter_map(|addr| {
                    let email = addr.address()?;
                    Some(match addr.name() {
                        Some(name) if !name.is_empty() => format!("\"{}\" <{}>", name, email),
                        _ => email.to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_else(Vec::new);

    // Extract date: prefer the raw Date header text (preserves original format),
    // falling back to mail_parser's parsed DateTime → RFC 3339.
    // mail_parser parses standard RFC 2822 dates into HeaderValue::DateTime,
    // losing the original string. For non-standard dates (ISO 8601, invalid strings),
    // it may produce garbled output. We extract the raw header from the email bytes.
    let date = extract_raw_date_header(&data).or_else(|| {
        message.date().and_then(|d| {
            let rfc3339 = d.to_rfc3339();
            // Reject obviously garbled dates (year 2000, month 0)
            if rfc3339.starts_with("2000-00") || rfc3339.starts_with("0000-") {
                None
            } else {
                Some(rfc3339)
            }
        })
    });

    let message_id = message.message_id().map(|id| id.to_string());

    // Extract threading and additional headers
    let reply_to: Vec<String> = message
        .reply_to()
        .map(|addrs| {
            addrs
                .iter()
                .filter_map(|addr| {
                    let email = addr.address()?;
                    Some(match addr.name() {
                        Some(name) if !name.is_empty() => format!("\"{}\" <{}>", name, email),
                        _ => email.to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    let in_reply_to: Vec<String> = message
        .in_reply_to()
        .as_text_list()
        .map(|list| list.iter().map(|s| s.to_string()).collect())
        .unwrap_or_default();

    let references: Vec<String> = message
        .references()
        .as_text_list()
        .map(|list| list.iter().map(|s| s.to_string()).collect())
        .unwrap_or_default();

    // Extract additional raw headers from email bytes
    let raw_headers = extract_raw_headers(&data);

    // Iterate over all body parts to capture content from multipart messages.
    // Also recurse into nested message/rfc822 parts (multipart/digest emails).
    //
    // Important: mail-parser's `body_text()` auto-converts HTML to plain text
    // using a naive tag-stripping approach that does NOT remove <script> or
    // <style> content. We only trust `body_text()` when the message has at
    // least one genuine `text/plain` part (PartType::Text). For HTML-only
    // emails we fall through to `clean_html_content()` which uses
    // html-to-markdown-rs or regex-based stripping that properly handles scripts.
    let has_genuine_text_part = has_genuine_text_body(&message);
    let plain_text = if has_genuine_text_part {
        let mut all_text = Vec::new();
        let mut i = 0;
        while let Some(text) = message.body_text(i) {
            all_text.push(text.to_string());
            i += 1;
        }
        // Extract text from nested message/rfc822 sub-messages (e.g. multipart/digest).
        collect_nested_message_text(&message, &mut all_text);
        if all_text.is_empty() {
            None
        } else {
            Some(all_text.join("\n\n"))
        }
    } else {
        None
    };

    let html_content = {
        let mut all_html = Vec::new();
        let mut i = 0;
        while let Some(html) = message.body_html(i) {
            all_html.push(html.to_string());
            i += 1;
        }
        // Extract HTML from nested message/rfc822 sub-messages.
        collect_nested_message_html(&message, &mut all_html);
        if all_html.is_empty() {
            None
        } else {
            Some(all_html.join("\n\n"))
        }
    };

    let cleaned_text = if let Some(ref plain) = plain_text {
        plain.clone()
    } else if let Some(html) = &html_content {
        clean_html_content(html)
    } else {
        String::new()
    };

    let mut attachments = Vec::with_capacity(message.attachments().count().min(20));
    for attachment in message.attachments() {
        let filename = attachment.attachment_name().map(|s| s.to_string());

        let mime_type = attachment
            .content_type()
            .map(|ct| {
                let content_type_str = format!("{}/{}", ct.ctype(), ct.subtype().unwrap_or("octet-stream"));
                parse_content_type(&content_type_str)
            })
            .unwrap_or_else(|| "application/octet-stream".to_string());

        let data = attachment.contents();
        let size = data.len();

        let is_image = is_image_mime_type(&mime_type);

        attachments.push(EmailAttachment {
            name: filename.clone(),
            filename,
            mime_type: Some(mime_type),
            size: Some(size),
            is_image,
            data: Some(Bytes::copy_from_slice(data)),
        });
    }

    let mut metadata = build_metadata(
        &subject,
        &from_email,
        &to_emails,
        &cc_emails,
        &bcc_emails,
        &date,
        &message_id,
        &attachments,
    );

    // Add threading headers to metadata
    if !reply_to.is_empty() {
        metadata.insert("reply_to".to_string(), reply_to.join(", "));
    }
    if !in_reply_to.is_empty() {
        metadata.insert("in_reply_to".to_string(), in_reply_to.join(", "));
    }
    if !references.is_empty() {
        metadata.insert("references".to_string(), references.join(", "));
    }

    // Add raw headers (Content-Type, MIME-Version, X-Mailer, List-Id, List-Unsubscribe)
    for (key, value) in &raw_headers {
        metadata.insert(key.clone(), value.clone());
    }

    // Add structured attachment metadata (filename, size, mime_type per attachment)
    if !attachments.is_empty() {
        let attachment_details: Vec<String> = attachments
            .iter()
            .map(|att| {
                let name = att.filename.as_deref().or(att.name.as_deref()).unwrap_or("unnamed");
                let mime = att.mime_type.as_deref().unwrap_or("application/octet-stream");
                let size = att.size.unwrap_or(0);
                format!("{}|{}|{}", name, mime, size)
            })
            .collect();
        metadata.insert("attachment_details".to_string(), attachment_details.join("; "));
    }

    Ok(EmailExtractionResult {
        subject,
        from_email,
        to_emails,
        cc_emails,
        bcc_emails,
        date,
        message_id,
        plain_text,
        html_content,
        cleaned_text,
        attachments,
        metadata,
    })
}

/// Check whether a message has at least one genuine `text/plain` body part.
///
/// `mail-parser`'s `body_text()` auto-converts HTML to plain text using a naive
/// tag-stripper that doesn't remove `<script>` or `<style>` content. This helper
/// inspects the actual `PartType` of each `text_body` entry to determine if a
/// real `text/plain` part exists. For HTML-only messages (all text_body entries
/// are `PartType::Html`), callers should use `clean_html_content()` instead.
fn has_genuine_text_body(message: &mail_parser::Message<'_>) -> bool {
    use mail_parser::PartType;
    for &part_id in &message.text_body {
        if let Some(part) = message.parts.get(part_id as usize)
            && matches!(&part.body, PartType::Text(_))
        {
            return true;
        }
    }
    false
}

/// Recursively collect plain text from nested `message/rfc822` sub-messages.
///
/// In `multipart/digest` emails, each part is itself an RFC822 message stored as
/// `PartType::Message`. The top-level `body_text()` won't return these; we must
/// recurse into the nested `Message` to extract their text bodies.
fn collect_nested_message_text(message: &mail_parser::Message<'_>, out: &mut Vec<String>) {
    use mail_parser::PartType;
    for part in &message.parts {
        if let PartType::Message(sub_msg) = &part.body {
            // Collect direct text bodies from the sub-message.
            let mut i = 0;
            while let Some(text) = sub_msg.body_text(i) {
                out.push(text.to_string());
                i += 1;
            }
            // Recurse further in case of deeply nested messages.
            collect_nested_message_text(sub_msg, out);
        }
    }
}

/// Recursively collect HTML from nested `message/rfc822` sub-messages.
fn collect_nested_message_html(message: &mail_parser::Message<'_>, out: &mut Vec<String>) {
    use mail_parser::PartType;
    for part in &message.parts {
        if let PartType::Message(sub_msg) = &part.body {
            let mut i = 0;
            while let Some(html) = sub_msg.body_html(i) {
                out.push(html.to_string());
                i += 1;
            }
            collect_nested_message_html(sub_msg, out);
        }
    }
}

/// Parse .msg file content (Outlook format).
///
/// Reads MSG files directly via the CFB (OLE Compound Document) format,
/// extracting text properties and attachment metadata without the overhead
/// of hex-encoding attachment binary data (which caused hangs on large files
/// with the previous `msg_parser` dependency).
///
/// Some MSG files have FAT headers declaring more sectors than the file
/// actually contains.  The strict `cfb` crate rejects these.  When that
/// happens we pad the data with zero bytes so the sector count matches
/// the FAT and retry – the real streams are still within the original
/// data range and parse correctly.
///
pub fn parse_msg_content(data: &[u8], fallback_codepage: Option<u32>) -> Result<EmailExtractionResult> {
    use std::borrow::Cow;
    use std::io::Cursor;

    // Try strict CFB parsing first; fall back to lenient padding.
    let padded: Cow<'_, [u8]>;
    let data_ref: &[u8] = match cfb::CompoundFile::open(Cursor::new(data)) {
        Ok(_) => data,
        Err(_first_err) => {
            padded = pad_cfb_to_fat_size(data);
            if std::ptr::eq(padded.as_ref(), data) {
                // Padding didn't help – propagate original error.
                return Err(KreuzbergError::parsing(format!(
                    "Failed to parse MSG file: {_first_err}"
                )));
            }
            &padded
        }
    };

    let mut comp = cfb::CompoundFile::open(Cursor::new(data_ref))
        .map_err(|e| KreuzbergError::parsing(format!("Failed to parse MSG file: {e}")))?;

    extract_msg_from_cfb(&mut comp, fallback_codepage)
}

/// Pad an OLE/CFB file so the sector count matches the FAT header.
///
/// Some MSG writers emit FAT tables that reference sectors beyond the
/// physical end of the file.  The `cfb` crate rightfully rejects these
/// as "Malformed FAT".  By zero-padding to the declared size we let cfb
/// open the file; streams within the original range parse normally while
/// the padded area is treated as free sectors.
fn pad_cfb_to_fat_size(data: &[u8]) -> std::borrow::Cow<'_, [u8]> {
    use std::borrow::Cow;

    // OLE header is at least 76 bytes; magic D0 CF 11 E0 A1 B1 1A E1.
    if data.len() < 76 || data[..4] != [0xD0, 0xCF, 0x11, 0xE0] {
        return Cow::Borrowed(data);
    }

    let sector_power = u16::from_le_bytes([data[30], data[31]]) as u32;
    if !(9..=16).contains(&sector_power) {
        return Cow::Borrowed(data);
    }
    let sector_size = 1u64 << sector_power;

    // Number of FAT sectors is at header offset 44 (LE u32).
    let fat_sectors = u32::from_le_bytes([data[44], data[45], data[46], data[47]]) as u64;
    // Each FAT sector holds sector_size/4 entries; each entry maps one sector.
    let fat_entries = fat_sectors * (sector_size / 4);
    // File must be at least: 1 header sector + fat_entries data sectors.
    let needed = (1 + fat_entries) * sector_size;

    // Cap at 256 MB to avoid pathological headers causing huge allocations.
    if needed > 256 * 1024 * 1024 || (data.len() as u64) >= needed {
        return Cow::Borrowed(data);
    }

    let mut padded = data.to_vec();
    padded.resize(needed as usize, 0);
    Cow::Owned(padded)
}

/// Pre-filled dictionary used by the MS-OXRTFCP compressed RTF format.
///
/// See [MS-OXRTFCP] section 2.1.3.1.
const COMPRESSED_RTF_PREBUF: &[u8] = b"{\\rtf1\\ansi\\mac\\deff0\\deftab720{\\fonttbl;}\
{\\f0\\fnil \\froman \\fswiss \\fmodern \\fscript \\fdecor MS Sans SerifSymbolArialTimes New Roman\
Courier{\\colortbl\\red0\\green0\\blue0\r\n\\par \\pard\\plain\\f0\\fs20\\b\\i\\ul\\ob\\strike\
\\scaps\\outline\\shadow\\imprint\\emboss\\lang1024\\sbasedon1033\\fcharset0 {\\*\\cs10 \\additive \
Default Paragraph Font}";

/// Decompress a PR_RTF_COMPRESSED stream per the MS-OXRTFCP specification.
///
/// Returns `None` when the data is too short, has a bad magic number, or
/// the decompression runs past declared bounds.
fn decompress_rtf_compressed(data: &[u8]) -> Option<Vec<u8>> {
    if data.len() < 16 {
        return None;
    }

    let comp_size = u32::from_le_bytes(data[0..4].try_into().ok()?) as usize;
    let raw_size = u32::from_le_bytes(data[4..8].try_into().ok()?);
    let magic = u32::from_le_bytes(data[8..12].try_into().ok()?);
    // _crc at 12..16 — we skip validation, matching common implementations.

    // "LZFu" (0x75465a4c) = compressed, "MELA" (0x414c454d) = uncompressed.
    if magic == 0x414c_454d {
        // Uncompressed: raw RTF follows the 16-byte header.
        return Some(data.get(16..16 + comp_size.saturating_sub(12))?.to_vec());
    }
    if magic != 0x75465a4c {
        return None;
    }

    let mut dict = [0u8; 4096];
    let prebuf_len = COMPRESSED_RTF_PREBUF.len();
    dict[..prebuf_len].copy_from_slice(COMPRESSED_RTF_PREBUF);
    let mut dict_write = prebuf_len;

    let input = data.get(16..)?;
    // comp_size includes the 12 bytes after the first u32, so input length should be comp_size - 12.
    let end = (comp_size.saturating_sub(12)).min(input.len());

    let mut output = Vec::with_capacity(raw_size as usize);
    let mut pos = 0usize;

    while pos < end {
        let control = *input.get(pos)?;
        pos += 1;

        for bit in (0..8).rev() {
            if pos >= end {
                return Some(output);
            }

            if control & (1 << bit) != 0 {
                // Dictionary reference: 2 bytes, big-endian style.
                let hi = *input.get(pos)? as u16;
                let lo = *input.get(pos + 1)? as u16;
                pos += 2;

                let offset = ((hi << 4) | (lo >> 4)) as usize;
                let length = (lo & 0x0F) as usize + 2;

                for i in 0..length {
                    let byte = dict[(offset + i) & 0xFFF];
                    output.push(byte);
                    dict[dict_write & 0xFFF] = byte;
                    dict_write += 1;
                }
            } else {
                // Literal byte.
                let byte = *input.get(pos)?;
                pos += 1;
                output.push(byte);
                dict[dict_write & 0xFFF] = byte;
                dict_write += 1;
            }
        }
    }

    Some(output)
}

/// Strip RTF control sequences and extract plain text.
///
/// Handles `\par` → newline, `\uN` unicode escapes, `{` `}` grouping,
/// and discards other `\command` sequences.  This is intentionally
/// simplified — it covers the typical content produced by Outlook.
fn strip_rtf_to_plain_text(rtf: &[u8]) -> String {
    let text = String::from_utf8_lossy(rtf);
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut output = String::with_capacity(len / 2);
    let mut i = 0;
    // Track brace depth; skip content inside `{\*...}` destinations.
    let mut skip_depth: Option<usize> = None;
    let mut depth: usize = 0;
    // Skip the `{\rtfN ...}` header group preamble up to the first \par or body text.

    while i < len {
        match bytes[i] {
            b'{' => {
                depth += 1;
                i += 1;
                // Detect `{\*` destinations — these are metadata groups we skip.
                if i + 1 < len && bytes[i] == b'\\' && bytes.get(i + 1) == Some(&b'*') && skip_depth.is_none() {
                    skip_depth = Some(depth);
                }
                // Detect `{\fonttbl`, `{\colortbl`, `{\stylesheet`, `{\info` — skip these.
                if skip_depth.is_none() {
                    let rest = &text[i..];
                    if rest.starts_with("\\fonttbl")
                        || rest.starts_with("\\colortbl")
                        || rest.starts_with("\\stylesheet")
                        || rest.starts_with("\\info")
                    {
                        skip_depth = Some(depth);
                    }
                }
            }
            b'}' => {
                if let Some(sd) = skip_depth
                    && depth <= sd
                {
                    skip_depth = None;
                }
                depth = depth.saturating_sub(1);
                i += 1;
            }
            b'\\' if skip_depth.is_none() => {
                i += 1;
                if i >= len {
                    break;
                }
                match bytes[i] {
                    // Escaped literal characters.
                    b'\\' => {
                        output.push('\\');
                        i += 1;
                    }
                    b'{' => {
                        output.push('{');
                        i += 1;
                    }
                    b'}' => {
                        output.push('}');
                        i += 1;
                    }
                    b'\'' => {
                        // \'XX hex escape — decode as windows-1252.
                        i += 1;
                        if i + 2 <= len {
                            if let Ok(hex_str) = std::str::from_utf8(&bytes[i..i + 2])
                                && let Ok(byte_val) = u8::from_str_radix(hex_str, 16)
                            {
                                let byte_arr = [byte_val];
                                let (decoded, _, _) = encoding_rs::WINDOWS_1252.decode(&byte_arr);
                                output.push_str(&decoded);
                            }
                            i += 2;
                        }
                    }
                    b'u' if i + 1 < len && (bytes[i + 1].is_ascii_digit() || bytes[i + 1] == b'-') => {
                        // \uN — unicode escape followed by an ANSI substitution char.
                        i += 1; // skip 'u'
                        let start = i;
                        if i < len && bytes[i] == b'-' {
                            i += 1;
                        }
                        while i < len && bytes[i].is_ascii_digit() {
                            i += 1;
                        }
                        if let Ok(num_str) = std::str::from_utf8(&bytes[start..i])
                            && let Ok(code) = num_str.parse::<i32>()
                        {
                            let cp = if code < 0 { (code + 65536) as u32 } else { code as u32 };
                            if let Some(ch) = char::from_u32(cp) {
                                output.push(ch);
                            }
                        }
                        // Skip optional space delimiter after the number.
                        if i < len && bytes[i] == b' ' {
                            i += 1;
                        }
                        // Skip the ANSI substitution character (usually `?`).
                        if i < len && bytes[i] != b'\\' && bytes[i] != b'{' && bytes[i] != b'}' {
                            i += 1;
                        }
                    }
                    _ => {
                        // Read the control word.
                        let word_start = i;
                        while i < len && bytes[i].is_ascii_alphabetic() {
                            i += 1;
                        }
                        let word = &text[word_start..i];

                        // Skip optional numeric parameter.
                        // Skip optional numeric parameter position tracking (position advanced below).
                        if i < len && (bytes[i] == b'-' || bytes[i].is_ascii_digit()) {
                            if bytes[i] == b'-' {
                                i += 1;
                            }
                            while i < len && bytes[i].is_ascii_digit() {
                                i += 1;
                            }
                        }

                        // Consume optional space delimiter.
                        if i < len && bytes[i] == b' ' {
                            i += 1;
                        }

                        match word {
                            "par" | "line" => output.push('\n'),
                            "tab" => output.push('\t'),
                            // `\pard`, `\plain`, `\b`, `\i`, etc. — formatting, skip.
                            _ => {}
                        }
                    }
                }
            }
            b'\r' | b'\n' if skip_depth.is_none() => {
                // RTF uses \par for line breaks; literal newlines are ignored.
                i += 1;
            }
            _ if skip_depth.is_some() => {
                i += 1;
            }
            _ => {
                output.push(bytes[i] as char);
                i += 1;
            }
        }
    }

    // Collapse multiple blank lines and trim.
    let mut result = String::with_capacity(output.len());
    let mut prev_newline_count = 0u32;
    for ch in output.chars() {
        if ch == '\n' {
            prev_newline_count += 1;
            if prev_newline_count <= 2 {
                result.push('\n');
            }
        } else {
            prev_newline_count = 0;
            result.push(ch);
        }
    }

    result.trim().to_string()
}

/// Internal: extract email fields from an already-opened CFB compound file.
fn extract_msg_from_cfb<F: std::io::Read + std::io::Seek>(
    comp: &mut cfb::CompoundFile<F>,
    fallback_codepage: Option<u32>,
) -> Result<EmailExtractionResult> {
    // --- message-level properties ------------------------------------------

    // Read the message code page. File-specified codepage always wins;
    // fall back to user-configured fallback, then windows-1252.
    let codepage = read_msg_int_prop(comp, "", 0x3FFD) // PR_MESSAGE_CODEPAGE
        .or_else(|| read_msg_int_prop(comp, "", 0x3FDE)) // PR_INTERNET_CPID
        .or(fallback_codepage);

    let subject = read_msg_string_prop(comp, "", 0x0037, codepage); // PR_SUBJECT
    let sender_name = read_msg_string_prop(comp, "", 0x0C1A, codepage); // PR_SENDER_NAME
    let sender_email = read_msg_string_prop(comp, "", 0x0C1F, codepage) // PR_SENDER_EMAIL_ADDRESS
        .or_else(|| read_msg_string_prop(comp, "", 0x0065, codepage)) // PR_SENT_REPRESENTING_EMAIL
        .filter(|s| !s.is_empty());
    let from_email = sender_email.as_ref().map(|email| match sender_name.as_deref() {
        Some(name) if !name.is_empty() => format!("\"{}\" <{}>", name, email),
        _ => email.clone(),
    });
    let body = read_msg_string_prop(comp, "", 0x1000, codepage); // PR_BODY
    let html_body = read_msg_string_prop(comp, "", 0x1013, codepage); // PR_BODY_HTML
    let message_id = read_msg_string_prop(comp, "", 0x1035, codepage) // PR_INTERNET_MESSAGE_ID
        .filter(|s| !s.is_empty());

    // --- date: prefer PR_CLIENT_SUBMIT_TIME, fall back to transport headers ---
    let date = read_msg_filetime_prop(comp, "", 0x0039) // PR_CLIENT_SUBMIT_TIME
        .or_else(|| read_msg_filetime_prop(comp, "", 0x0E06)) // PR_MESSAGE_DELIVERY_TIME
        .or_else(|| {
            let headers = read_msg_string_prop(comp, "", 0x007D, codepage); // PR_TRANSPORT_MESSAGE_HEADERS
            headers.as_ref().and_then(|h| {
                h.lines()
                    .find(|line| line.starts_with("Date:"))
                    .map(|line| line.trim_start_matches("Date:").trim().to_string())
            })
        });

    // --- recipients: read from substorages for full email addresses -----------
    let (to_emails, cc_emails, bcc_emails) = read_msg_recipients(comp, codepage);

    // PR_RTF_COMPRESSED (0x1009) — binary stream, PT_BINARY (0102).
    let rtf_body = read_msg_stream(comp, "__substg1.0_10090102")
        .and_then(|data| decompress_rtf_compressed(&data))
        .map(|rtf| strip_rtf_to_plain_text(&rtf))
        .filter(|s| !s.is_empty());

    let plain_text = body.filter(|s| !s.is_empty());
    let html_content = html_body.filter(|s| !s.is_empty());

    let cleaned_text = if let Some(ref plain) = plain_text {
        plain.clone()
    } else if let Some(ref html) = html_content {
        clean_html_content(html)
    } else if let Some(ref rtf) = rtf_body {
        rtf.clone()
    } else {
        String::new()
    };

    // --- attachment storages -----------------------------------------------

    let attach_paths: Vec<String> = comp
        .walk()
        .filter(|e| e.is_storage() && e.name().starts_with("__attach_"))
        .map(|e| e.path().to_string_lossy().into_owned())
        .collect();

    let mut attachments = Vec::with_capacity(attach_paths.len());
    for path in &attach_paths {
        let long_name = read_msg_string_prop(comp, path, 0x3707, codepage); // PR_ATTACH_LONG_FILENAME
        let short_name = read_msg_string_prop(comp, path, 0x3704, codepage); // PR_ATTACH_FILENAME
        let display_name = read_msg_string_prop(comp, path, 0x3001, codepage); // PR_DISPLAY_NAME
        let extension = read_msg_string_prop(comp, path, 0x3703, codepage); // PR_ATTACH_EXTENSION
        let mime_tag = read_msg_string_prop(comp, path, 0x370E, codepage); // PR_ATTACH_MIME_TAG

        let filename = long_name
            .or(short_name)
            .or_else(|| display_name.clone())
            .or_else(|| extension.map(|ext| format!("attachment{ext}")));

        // Read binary attachment data directly — no hex encoding.
        let bin_path = format!("{path}/__substg1.0_37010102");
        let binary_data = read_msg_stream(comp, &bin_path);
        let size = binary_data.as_ref().map(Vec::len);
        let att_data = binary_data.map(Bytes::from);

        let mime_type = mime_tag
            .filter(|s| !s.is_empty())
            .or_else(|| Some("application/octet-stream".to_string()));
        let is_image = mime_type.as_ref().map(|m| is_image_mime_type(m)).unwrap_or(false);

        attachments.push(EmailAttachment {
            name: filename.clone(),
            filename,
            mime_type,
            size,
            is_image,
            data: att_data,
        });
    }

    // --- metadata ----------------------------------------------------------

    let mut metadata = HashMap::new();
    if let Some(ref subj) = subject {
        metadata.insert("subject".to_string(), subj.to_string());
    }
    if let Some(ref from) = from_email {
        metadata.insert("email_from".to_string(), from.to_string());
    }
    if let Some(ref name) = sender_name
        && !name.is_empty()
    {
        metadata.insert("from_name".to_string(), name.to_string());
    }
    if !to_emails.is_empty() {
        metadata.insert("email_to".to_string(), to_emails.join(", "));
    }
    if !cc_emails.is_empty() {
        metadata.insert("email_cc".to_string(), cc_emails.join(", "));
    }
    if !bcc_emails.is_empty() {
        metadata.insert("email_bcc".to_string(), bcc_emails.join(", "));
    }
    if let Some(ref dt) = date {
        metadata.insert("date".to_string(), dt.to_string());
    }
    if let Some(ref msg_id) = message_id {
        metadata.insert("message_id".to_string(), msg_id.to_string());
    }

    Ok(EmailExtractionResult {
        subject,
        from_email,
        to_emails,
        cc_emails,
        bcc_emails,
        date,
        message_id,
        plain_text,
        html_content,
        cleaned_text,
        attachments,
        metadata,
    })
}

// --- MSG / CFB helper functions --------------------------------------------

/// Read a raw CFB stream by path; returns `None` for missing or empty streams.
fn read_msg_stream<F: std::io::Read + std::io::Seek>(comp: &mut cfb::CompoundFile<F>, path: &str) -> Option<Vec<u8>> {
    use std::io::Read;
    let mut stream = comp.open_stream(path).ok()?;
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).ok()?;
    if buf.is_empty() { None } else { Some(buf) }
}

/// Map a Windows code page number to an `encoding_rs` `Encoding`.
///
/// Falls back to windows-1252 (the most common legacy ANSI code page) for unknown values.
fn encoding_for_windows_codepage(cp: u32) -> &'static Encoding {
    let label: &[u8] = match cp {
        65001 => b"utf-8",
        20127 => b"us-ascii",
        1250 => b"windows-1250",
        1251 => b"windows-1251",
        1252 => b"windows-1252",
        1253 => b"windows-1253",
        1254 => b"windows-1254",
        1255 => b"windows-1255",
        1256 => b"windows-1256",
        1257 => b"windows-1257",
        1258 => b"windows-1258",
        932 | 10001 => b"shift_jis",
        936 | 10008 => b"gbk",
        949 | 10003 => b"euc-kr",
        950 | 10002 => b"big5",
        28591 => b"iso-8859-1",
        28592 => b"iso-8859-2",
        28595 => b"iso-8859-5",
        28597 => b"iso-8859-7",
        28599 => b"iso-8859-9",
        _ => b"windows-1252",
    };
    Encoding::for_label(label).unwrap_or(encoding_rs::WINDOWS_1252)
}

/// Read a PT_LONG (0x0003) integer property from the `__properties_version1.0` stream.
fn read_msg_int_prop<F: std::io::Read + std::io::Seek>(
    comp: &mut cfb::CompoundFile<F>,
    base: &str,
    prop_id: u16,
) -> Option<u32> {
    use std::io::Read;

    let props_path = format!("{base}/__properties_version1.0");
    let mut stream = comp.open_stream(&props_path).ok()?;
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).ok()?;

    // Message-level properties have a 32-byte header; recipient/attachment have 8-byte.
    let header_size: usize = if base.is_empty() { 32 } else { 8 };
    let mut offset = header_size;

    while offset + 16 <= buf.len() {
        let ptype = u16::from_le_bytes([buf[offset], buf[offset + 1]]);
        let pid = u16::from_le_bytes([buf[offset + 2], buf[offset + 3]]);

        if pid == prop_id && ptype == 0x0003 {
            // PT_LONG — value sits in the first 4 bytes of the 8-byte value field
            return Some(u32::from_le_bytes(buf[offset + 8..offset + 12].try_into().ok()?));
        }
        offset += 16;
    }
    None
}

/// Read a MAPI string property (tries PT_UNICODE then PT_STRING8).
///
/// `codepage` is the Windows code page to use when decoding PT_STRING8 bytes.
/// Pass `None` to fall back to windows-1252 (safe default for legacy MSG files).
fn read_msg_string_prop<F: std::io::Read + std::io::Seek>(
    comp: &mut cfb::CompoundFile<F>,
    base: &str,
    prop_id: u16,
    codepage: Option<u32>,
) -> Option<String> {
    // Try PT_UNICODE (001F) first.
    let unicode_path = format!("{base}/__substg1.0_{prop_id:04X}001F");
    if let Some(buf) = read_msg_stream(comp, &unicode_path) {
        return Some(decode_utf16le_bytes(&buf));
    }
    // Fallback to PT_STRING8 (001E), decoded with the message code page.
    let ansi_path = format!("{base}/__substg1.0_{prop_id:04X}001E");
    read_msg_stream(comp, &ansi_path).map(|buf| {
        let encoding = codepage
            .map(encoding_for_windows_codepage)
            .unwrap_or(encoding_rs::WINDOWS_1252);
        let (decoded, _, _) = encoding.decode(&buf);
        decoded.trim_end_matches('\0').to_string()
    })
}

/// Decode UTF-16LE bytes to a String, stripping trailing NUL chars.
fn decode_utf16le_bytes(data: &[u8]) -> String {
    let u16s: Vec<u16> = data.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
    String::from_utf16_lossy(&u16s).trim_end_matches('\0').to_string()
}

/// Read a PT_SYSTIME (FILETIME) property from the __properties_version1.0 stream
/// and convert it to an ISO 8601 date string.
///
/// FILETIME is a 64-bit value representing 100-nanosecond intervals since 1601-01-01.
fn read_msg_filetime_prop<F: std::io::Read + std::io::Seek>(
    comp: &mut cfb::CompoundFile<F>,
    base: &str,
    prop_id: u16,
) -> Option<String> {
    use std::io::Read;

    let props_path = format!("{base}/__properties_version1.0");
    let mut stream = comp.open_stream(&props_path).ok()?;
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).ok()?;

    // Message-level properties have a 32-byte header; recipient/attachment have 8-byte.
    let header_size: usize = if base.is_empty() { 32 } else { 8 };
    let mut offset = header_size;

    while offset + 16 <= buf.len() {
        // MAPI property entry: prop_type (2) + prop_id (2) + flags (4) + value (8)
        let ptype = u16::from_le_bytes([buf[offset], buf[offset + 1]]);
        let pid = u16::from_le_bytes([buf[offset + 2], buf[offset + 3]]);

        if pid == prop_id && ptype == 0x0040 {
            // PT_SYSTIME
            let filetime = u64::from_le_bytes(buf[offset + 8..offset + 16].try_into().ok()?);
            return filetime_to_iso8601(filetime);
        }
        offset += 16;
    }
    None
}

/// Convert a Windows FILETIME (100-ns intervals since 1601-01-01) to ISO 8601.
fn filetime_to_iso8601(filetime: u64) -> Option<String> {
    // Epoch offset: difference between 1601-01-01 and 1970-01-01 in 100-ns intervals
    const EPOCH_DIFF: u64 = 116_444_736_000_000_000;
    if filetime < EPOCH_DIFF {
        return None;
    }
    let hundred_ns = filetime - EPOCH_DIFF;
    let secs = (hundred_ns / 10_000_000) as i64;
    let nanos = ((hundred_ns % 10_000_000) * 100) as u32;

    // Format manually to avoid pulling in chrono
    let days_since_epoch = secs / 86400;
    let time_of_day = secs % 86400;
    let (hour, min, sec) = (time_of_day / 3600, (time_of_day % 3600) / 60, time_of_day % 60);

    // Civil date calculation from days since 1970-01-01 (algorithm from Howard Hinnant)
    let z = days_since_epoch + 719468;
    let era = (if z >= 0 { z } else { z - 146096 }) / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    if nanos == 0 {
        Some(format!("{y:04}-{m:02}-{d:02}T{hour:02}:{min:02}:{sec:02}+00:00"))
    } else {
        // Include sub-second precision
        let frac = nanos / 1_000_000; // milliseconds
        Some(format!(
            "{y:04}-{m:02}-{d:02}T{hour:02}:{min:02}:{sec:02}.{frac:03}+00:00"
        ))
    }
}

/// Read recipients from MSG __recip_version1.0_#XXXXXXXX substorages.
///
/// Returns (to, cc, bcc) vectors. Each entry is formatted as `"Name" <email>` or just `email`.
fn read_msg_recipients<F: std::io::Read + std::io::Seek>(
    comp: &mut cfb::CompoundFile<F>,
    codepage: Option<u32>,
) -> (Vec<String>, Vec<String>, Vec<String>) {
    // Collect recipient storage paths
    let recip_paths: Vec<String> = comp
        .walk()
        .filter(|e| e.is_storage() && e.name().starts_with("__recip_version1.0_"))
        .map(|e| e.path().to_string_lossy().into_owned())
        .collect();

    let mut to_emails = Vec::new();
    let mut cc_emails = Vec::new();
    let mut bcc_emails = Vec::new();

    for path in &recip_paths {
        let display_name = read_msg_string_prop(comp, path, 0x3001, codepage); // PR_DISPLAY_NAME
        let email_addr = read_msg_string_prop(comp, path, 0x39FE, codepage) // PR_SMTP_ADDRESS
            .or_else(|| read_msg_string_prop(comp, path, 0x3003, codepage)) // PR_EMAIL_ADDRESS
            .filter(|s| !s.is_empty());

        let formatted = match (&display_name, &email_addr) {
            (Some(name), Some(email)) if !name.is_empty() && name != email => {
                format!("\"{}\" <{}>", name, email)
            }
            (_, Some(email)) => email.clone(),
            (Some(name), None) if !name.is_empty() => name.clone(),
            _ => continue,
        };

        // Read PR_RECIPIENT_TYPE from properties stream
        let recip_type = read_msg_recip_type(comp, path);
        match recip_type {
            1 => to_emails.push(formatted),  // MAPI_TO
            2 => cc_emails.push(formatted),  // MAPI_CC
            3 => bcc_emails.push(formatted), // MAPI_BCC
            _ => to_emails.push(formatted),  // Default to To
        }
    }

    (to_emails, cc_emails, bcc_emails)
}

/// Read PR_RECIPIENT_TYPE (0x0C15) from a recipient's __properties_version1.0 stream.
/// Returns 1 (To), 2 (CC), 3 (BCC), or 0 if not found.
fn read_msg_recip_type<F: std::io::Read + std::io::Seek>(comp: &mut cfb::CompoundFile<F>, base: &str) -> u32 {
    use std::io::Read;

    let props_path = format!("{base}/__properties_version1.0");
    let mut stream = match comp.open_stream(&props_path) {
        Ok(s) => s,
        Err(_) => return 0,
    };
    let mut buf = Vec::new();
    if stream.read_to_end(&mut buf).is_err() {
        return 0;
    }

    // Recipient properties have 8-byte header
    let mut offset = 8;
    while offset + 16 <= buf.len() {
        // MAPI property entry: prop_type (2) + prop_id (2) + flags (4) + value (8)
        let ptype = u16::from_le_bytes([buf[offset], buf[offset + 1]]);
        let pid = u16::from_le_bytes([buf[offset + 2], buf[offset + 3]]);

        if pid == 0x0C15 && ptype == 0x0003 {
            // PT_LONG
            return u32::from_le_bytes([buf[offset + 8], buf[offset + 9], buf[offset + 10], buf[offset + 11]]);
        }
        offset += 16;
    }
    0
}

/// Extract the raw Date header value from email bytes.
///
/// Scans for `Date:` in the header section (before the blank line that separates
/// headers from body) and returns the raw value, handling continuation lines.
fn extract_raw_date_header(data: &[u8]) -> Option<String> {
    let text = utf8_validation::from_utf8(data).ok()?;

    // Find the end of headers (blank line)
    let header_end = text
        .find("\r\n\r\n")
        .or_else(|| text.find("\n\n"))
        .unwrap_or(text.len().min(8192)); // Cap scan to 8KB

    let headers = &text[..header_end];

    // Find Date: header (case-insensitive start, then exact field name)
    let mut date_value = None;
    for line in headers.lines() {
        if let Some(val) = line.strip_prefix("Date:").or_else(|| line.strip_prefix("date:")) {
            date_value = Some(val.trim().to_string());
        } else if date_value.is_some() && (line.starts_with(' ') || line.starts_with('\t')) {
            // Continuation line (folded header)
            if let Some(ref mut dv) = date_value {
                dv.push(' ');
                dv.push_str(line.trim());
            }
        } else if date_value.is_some() {
            break; // Next header field
        }
    }

    date_value.filter(|s| !s.is_empty())
}

/// Extract additional raw headers from email bytes.
///
/// Scans for Content-Type, MIME-Version, X-Mailer, User-Agent, List-Id,
/// and List-Unsubscribe headers in the header section.
fn extract_raw_headers(data: &[u8]) -> HashMap<String, String> {
    let mut headers = HashMap::new();
    let text = match utf8_validation::from_utf8(data) {
        Ok(s) => s,
        Err(_) => return headers,
    };

    // Find the end of headers (blank line)
    let header_end = text
        .find("\r\n\r\n")
        .or_else(|| text.find("\n\n"))
        .unwrap_or(text.len().min(16384)); // Cap scan to 16KB

    let header_section = &text[..header_end];

    // Headers to extract (case-insensitive key, metadata key)
    let target_headers: &[(&str, &str)] = &[
        ("content-type:", "content_type"),
        ("mime-version:", "mime_version"),
        ("x-mailer:", "x_mailer"),
        ("user-agent:", "user_agent"),
        ("list-id:", "list_id"),
        ("list-unsubscribe:", "list_unsubscribe"),
    ];

    let mut current_key: Option<&str> = None;
    let mut current_value = String::new();

    for line in header_section.lines() {
        if line.starts_with(' ') || line.starts_with('\t') {
            // Continuation line (folded header)
            if current_key.is_some() {
                current_value.push(' ');
                current_value.push_str(line.trim());
            }
            continue;
        }

        // Flush previous header if matched
        if let Some(key) = current_key {
            if !current_value.is_empty() {
                headers.insert(key.to_string(), current_value.clone());
            }
            current_key = None;
            current_value.clear();
        }

        // Check if current line matches any target header
        let line_lower = line.to_lowercase();
        for &(prefix, meta_key) in target_headers {
            if line_lower.starts_with(prefix) {
                current_key = Some(meta_key);
                current_value = line[prefix.len()..].trim().to_string();
                break;
            }
        }
    }

    // Flush last header
    if let Some(key) = current_key
        && !current_value.is_empty()
    {
        headers.insert(key.to_string(), current_value);
    }

    headers
}

/// Extract email content from either .eml or .msg format
pub fn extract_email_content(
    data: &[u8],
    mime_type: &str,
    fallback_codepage: Option<u32>,
) -> Result<EmailExtractionResult> {
    if data.is_empty() {
        return Err(KreuzbergError::validation("Email content is empty".to_string()));
    }

    match mime_type {
        "message/rfc822" | "text/plain" => parse_eml_content(data),
        "application/vnd.ms-outlook" => parse_msg_content(data, fallback_codepage),
        _ => Err(KreuzbergError::validation(format!(
            "Unsupported email MIME type: {}",
            mime_type
        ))),
    }
}

/// Build text output from email extraction result
pub fn build_email_text_output(result: &EmailExtractionResult) -> String {
    let mut text_parts = Vec::with_capacity(10);

    if let Some(ref subject) = result.subject {
        text_parts.push(format!("Subject: {}", subject));
    }

    if let Some(ref from) = result.from_email {
        text_parts.push(format!("From: {}", from));
    }

    if !result.to_emails.is_empty() {
        text_parts.push(format!("To: {}", result.to_emails.join(", ")));
    }

    if !result.cc_emails.is_empty() {
        text_parts.push(format!("CC: {}", result.cc_emails.join(", ")));
    }

    if !result.bcc_emails.is_empty() {
        text_parts.push(format!("BCC: {}", result.bcc_emails.join(", ")));
    }

    if let Some(ref date) = result.date {
        text_parts.push(format!("Date: {}", date));
    }

    text_parts.push(result.cleaned_text.clone());

    // Attachment names are stored in metadata but not included in the text output.
    // This keeps the text output focused on message content.

    text_parts.join("\n")
}

fn clean_html_content(html: &str) -> String {
    if html.is_empty() {
        return String::new();
    }

    // Use html-to-markdown converter in plain text mode when available
    #[cfg(feature = "html")]
    {
        if let Ok(text) = crate::extraction::html::convert_html_to_markdown(
            html,
            None,
            Some(crate::core::config::OutputFormat::Plain),
        ) {
            let trimmed = text.trim().to_string();
            if !trimmed.is_empty() {
                return trimmed;
            }
        }
    }

    // Fallback: regex-based HTML stripping
    let cleaned = script_regex().replace_all(html, "");
    let cleaned = style_regex().replace_all(&cleaned, "");
    let cleaned = html_tag_regex().replace_all(&cleaned, "");
    let cleaned = whitespace_regex().replace_all(&cleaned, " ");

    cleaned.trim().to_string()
}

fn is_image_mime_type(mime_type: &str) -> bool {
    mime_type.starts_with("image/")
}

fn parse_content_type(content_type: &str) -> String {
    let trimmed = content_type.trim();
    if trimmed.is_empty() {
        return "application/octet-stream".to_string();
    }
    trimmed
        .split(';')
        .next()
        .unwrap_or("application/octet-stream")
        .trim()
        .to_lowercase()
}

#[allow(clippy::too_many_arguments)]
fn build_metadata(
    subject: &Option<String>,
    from_email: &Option<String>,
    to_emails: &[String],
    cc_emails: &[String],
    bcc_emails: &[String],
    date: &Option<String>,
    message_id: &Option<String>,
    attachments: &[EmailAttachment],
) -> HashMap<String, String> {
    let mut metadata = HashMap::new();

    if let Some(subj) = subject {
        metadata.insert("subject".to_string(), subj.clone());
    }
    if let Some(from) = from_email {
        metadata.insert("email_from".to_string(), from.clone());
    }
    if !to_emails.is_empty() {
        metadata.insert("email_to".to_string(), to_emails.join(", "));
    }
    if !cc_emails.is_empty() {
        metadata.insert("email_cc".to_string(), cc_emails.join(", "));
    }
    if !bcc_emails.is_empty() {
        metadata.insert("email_bcc".to_string(), bcc_emails.join(", "));
    }
    if let Some(dt) = date {
        metadata.insert("date".to_string(), dt.clone());
    }
    if let Some(msg_id) = message_id {
        metadata.insert("message_id".to_string(), msg_id.clone());
    }

    if !attachments.is_empty() {
        let attachment_names: Vec<String> = attachments
            .iter()
            .filter_map(|att| att.name.as_ref().or(att.filename.as_ref()))
            .cloned()
            .collect();
        if !attachment_names.is_empty() {
            metadata.insert("attachments".to_string(), attachment_names.join(", "));
        }
    }

    metadata
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_html_content() {
        let html = "<p>Hello <b>World</b></p>";
        let cleaned = clean_html_content(html);
        assert_eq!(cleaned, "Hello World");
    }

    #[test]
    fn test_clean_html_with_whitespace() {
        let html = "<div>  Multiple   \n  spaces  </div>";
        let cleaned = clean_html_content(html);
        assert!(
            cleaned.contains("Multiple") && cleaned.contains("spaces"),
            "Should contain text: {}",
            cleaned
        );
        // Whitespace may be collapsed or converted to newlines depending on
        // whether the html feature is enabled (block-level elements → newlines).
        assert!(
            !cleaned.contains("  "),
            "Should not have consecutive spaces: {}",
            cleaned
        );
    }

    #[test]
    fn test_clean_html_with_script_and_style() {
        let html = r#"
            <html>
                <head><style>body { color: red; }</style></head>
                <body>
                    <script>alert('test');</script>
                    <p>Hello World</p>
                </body>
            </html>
        "#;
        let cleaned = clean_html_content(html);
        assert!(!cleaned.contains("<script>"));
        assert!(!cleaned.contains("<style>"));
        assert!(cleaned.contains("Hello World"));
    }

    #[test]
    fn test_is_image_mime_type() {
        assert!(is_image_mime_type("image/png"));
        assert!(is_image_mime_type("image/jpeg"));
        assert!(!is_image_mime_type("text/plain"));
        assert!(!is_image_mime_type("application/pdf"));
    }

    #[test]
    fn test_parse_content_type() {
        assert_eq!(parse_content_type("text/plain"), "text/plain");
        assert_eq!(parse_content_type("text/plain; charset=utf-8"), "text/plain");
        assert_eq!(parse_content_type("image/jpeg; name=test.jpg"), "image/jpeg");
        assert_eq!(parse_content_type(""), "application/octet-stream");
    }

    #[test]
    fn test_extract_email_content_empty_data() {
        let result = extract_email_content(b"", "message/rfc822", None);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KreuzbergError::Validation { .. }));
    }

    #[test]
    fn test_extract_email_content_invalid_mime_type() {
        let result = extract_email_content(b"test", "application/pdf", None);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KreuzbergError::Validation { .. }));
    }

    #[test]
    fn test_parse_eml_content_invalid() {
        let result = parse_eml_content(b"not an email");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_msg_content_invalid() {
        let result = parse_msg_content(b"not a msg file", None);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KreuzbergError::Parsing { .. }));
    }

    #[test]
    fn test_simple_eml_parsing() {
        let eml_content =
            b"From: test@example.com\r\nTo: recipient@example.com\r\nSubject: Test Email\r\n\r\nThis is a test email body.";

        let result = parse_eml_content(eml_content).unwrap();
        assert_eq!(result.subject, Some("Test Email".to_string()));
        assert_eq!(result.from_email, Some("test@example.com".to_string()));
        assert_eq!(result.to_emails, vec!["recipient@example.com".to_string()]);
        assert_eq!(result.cleaned_text, "This is a test email body.");
    }

    #[test]
    fn test_build_email_text_output_minimal() {
        let result = EmailExtractionResult {
            subject: Some("Test".to_string()),
            from_email: Some("sender@example.com".to_string()),
            to_emails: vec!["recipient@example.com".to_string()],
            cc_emails: vec![],
            bcc_emails: vec![],
            date: None,
            message_id: None,
            plain_text: None,
            html_content: None,
            cleaned_text: "Hello World".to_string(),
            attachments: vec![],
            metadata: HashMap::new(),
        };

        let output = build_email_text_output(&result);
        assert!(output.contains("Subject: Test"));
        assert!(output.contains("From: sender@example.com"));
        assert!(output.contains("To: recipient@example.com"));
        assert!(output.contains("Hello World"));
    }

    #[test]
    fn test_build_email_text_output_with_attachments() {
        let result = EmailExtractionResult {
            subject: Some("Test".to_string()),
            from_email: Some("sender@example.com".to_string()),
            to_emails: vec!["recipient@example.com".to_string()],
            cc_emails: vec![],
            bcc_emails: vec![],
            date: None,
            message_id: None,
            plain_text: None,
            html_content: None,
            cleaned_text: "Hello World".to_string(),
            attachments: vec![EmailAttachment {
                name: Some("file.txt".to_string()),
                filename: Some("file.txt".to_string()),
                mime_type: Some("text/plain".to_string()),
                size: Some(1024),
                is_image: false,
                data: None,
            }],
            metadata: HashMap::new(),
        };

        let output = build_email_text_output(&result);
        // Attachment names are stored in metadata, not in text output
        assert!(!output.contains("Attachments:"));
        assert!(output.contains("Hello World"));
    }

    #[test]
    fn test_build_metadata() {
        let subject = Some("Test Subject".to_string());
        let from_email = Some("sender@example.com".to_string());
        let to_emails = vec!["recipient@example.com".to_string()];
        let cc_emails = vec!["cc@example.com".to_string()];
        let bcc_emails = vec!["bcc@example.com".to_string()];
        let date = Some("2024-01-01T12:00:00Z".to_string());
        let message_id = Some("<abc123@example.com>".to_string());
        let attachments = vec![];

        let metadata = build_metadata(
            &subject,
            &from_email,
            &to_emails,
            &cc_emails,
            &bcc_emails,
            &date,
            &message_id,
            &attachments,
        );

        assert_eq!(metadata.get("subject"), Some(&"Test Subject".to_string()));
        assert_eq!(metadata.get("email_from"), Some(&"sender@example.com".to_string()));
        assert_eq!(metadata.get("email_to"), Some(&"recipient@example.com".to_string()));
        assert_eq!(metadata.get("email_cc"), Some(&"cc@example.com".to_string()));
        assert_eq!(metadata.get("email_bcc"), Some(&"bcc@example.com".to_string()));
        assert_eq!(metadata.get("date"), Some(&"2024-01-01T12:00:00Z".to_string()));
        assert_eq!(metadata.get("message_id"), Some(&"<abc123@example.com>".to_string()));
    }

    #[test]
    fn test_build_metadata_with_attachments() {
        let attachments = vec![
            EmailAttachment {
                name: Some("file1.pdf".to_string()),
                filename: Some("file1.pdf".to_string()),
                mime_type: Some("application/pdf".to_string()),
                size: Some(1024),
                is_image: false,
                data: None,
            },
            EmailAttachment {
                name: Some("image.png".to_string()),
                filename: Some("image.png".to_string()),
                mime_type: Some("image/png".to_string()),
                size: Some(2048),
                is_image: true,
                data: None,
            },
        ];

        let metadata = build_metadata(&None, &None, &[], &[], &[], &None, &None, &attachments);

        assert_eq!(metadata.get("attachments"), Some(&"file1.pdf, image.png".to_string()));
    }

    #[test]
    fn test_clean_html_content_empty() {
        let cleaned = clean_html_content("");
        assert_eq!(cleaned, "");
    }

    #[test]
    fn test_clean_html_content_only_tags() {
        let html = "<div><span><p></p></span></div>";
        let cleaned = clean_html_content(html);
        assert_eq!(cleaned, "");
    }

    #[test]
    fn test_clean_html_content_nested_tags() {
        let html = "<div><p>Outer <span>Inner <b>Bold</b></span> Text</p></div>";
        let cleaned = clean_html_content(html);
        assert_eq!(cleaned, "Outer Inner Bold Text");
    }

    #[test]
    fn test_clean_html_content_multiple_scripts() {
        let html = r#"
            <script>function a() {}</script>
            <p>Content</p>
            <script>function b() {}</script>
        "#;
        let cleaned = clean_html_content(html);
        assert!(!cleaned.contains("function"));
        assert!(cleaned.contains("Content"));
    }

    #[test]
    fn test_is_image_mime_type_variants() {
        assert!(is_image_mime_type("image/gif"));
        assert!(is_image_mime_type("image/webp"));
        assert!(is_image_mime_type("image/svg+xml"));
        assert!(!is_image_mime_type("video/mp4"));
        assert!(!is_image_mime_type("audio/mp3"));
    }

    #[test]
    fn test_parse_content_type_with_parameters() {
        assert_eq!(parse_content_type("multipart/mixed; boundary=xyz"), "multipart/mixed");
        assert_eq!(parse_content_type("text/html; charset=UTF-8"), "text/html");
    }

    #[test]
    fn test_parse_content_type_whitespace() {
        assert_eq!(parse_content_type("  text/plain  "), "text/plain");
        assert_eq!(parse_content_type(" text/plain ; charset=utf-8 "), "text/plain");
    }

    #[test]
    fn test_parse_content_type_case_insensitive() {
        assert_eq!(parse_content_type("TEXT/PLAIN"), "text/plain");
        assert_eq!(parse_content_type("Image/JPEG"), "image/jpeg");
    }

    #[test]
    fn test_extract_email_content_mime_variants() {
        let eml_content = b"From: test@example.com\r\n\r\nBody";

        assert!(extract_email_content(eml_content, "message/rfc822", None).is_ok());
        assert!(extract_email_content(eml_content, "text/plain", None).is_ok());
    }

    #[test]
    fn test_simple_eml_with_multiple_recipients() {
        let eml_content = b"From: sender@example.com\r\nTo: r1@example.com, r2@example.com\r\nCc: cc@example.com\r\nBcc: bcc@example.com\r\nSubject: Multi-recipient\r\n\r\nBody";

        let result = parse_eml_content(eml_content).unwrap();
        assert_eq!(result.to_emails.len(), 2);
        assert!(result.to_emails.contains(&"r1@example.com".to_string()));
        assert!(result.to_emails.contains(&"r2@example.com".to_string()));
    }

    #[test]
    fn test_simple_eml_with_html_body() {
        let eml_content = b"From: sender@example.com\r\nTo: recipient@example.com\r\nSubject: HTML Email\r\nContent-Type: text/html\r\n\r\n<html><body><p>HTML Body</p></body></html>";

        let result = parse_eml_content(eml_content).unwrap();
        assert!(!result.cleaned_text.is_empty());
    }

    #[test]
    fn test_build_email_text_output_with_all_fields() {
        let result = EmailExtractionResult {
            subject: Some("Complete Email".to_string()),
            from_email: Some("sender@example.com".to_string()),
            to_emails: vec!["recipient@example.com".to_string()],
            cc_emails: vec!["cc@example.com".to_string()],
            bcc_emails: vec!["bcc@example.com".to_string()],
            date: Some("2024-01-01T12:00:00Z".to_string()),
            message_id: Some("<msg123@example.com>".to_string()),
            plain_text: Some("Plain text body".to_string()),
            html_content: Some("<html><body>HTML body</body></html>".to_string()),
            cleaned_text: "Cleaned body text".to_string(),
            attachments: vec![],
            metadata: HashMap::new(),
        };

        let output = build_email_text_output(&result);
        assert!(output.contains("Subject: Complete Email"));
        assert!(output.contains("From: sender@example.com"));
        assert!(output.contains("To: recipient@example.com"));
        assert!(output.contains("CC: cc@example.com"));
        assert!(output.contains("BCC: bcc@example.com"));
        assert!(output.contains("Date: 2024-01-01T12:00:00Z"));
        assert!(output.contains("Cleaned body text"));
    }

    #[test]
    fn test_build_email_text_output_empty_attachments() {
        let result = EmailExtractionResult {
            subject: Some("Test".to_string()),
            from_email: Some("sender@example.com".to_string()),
            to_emails: vec!["recipient@example.com".to_string()],
            cc_emails: vec![],
            bcc_emails: vec![],
            date: None,
            message_id: None,
            plain_text: None,
            html_content: None,
            cleaned_text: "Body".to_string(),
            attachments: vec![EmailAttachment {
                name: None,
                filename: None,
                mime_type: Some("application/octet-stream".to_string()),
                size: Some(100),
                is_image: false,
                data: None,
            }],
            metadata: HashMap::new(),
        };

        let output = build_email_text_output(&result);
        assert!(output.contains("Body"));
    }

    #[test]
    fn test_build_metadata_empty_fields() {
        let metadata = build_metadata(&None, &None, &[], &[], &[], &None, &None, &[]);
        assert!(metadata.is_empty());
    }

    #[test]
    fn test_build_metadata_partial_fields() {
        let subject = Some("Test".to_string());
        let date = Some("2024-01-01".to_string());

        let metadata = build_metadata(&subject, &None, &[], &[], &[], &date, &None, &[]);

        assert_eq!(metadata.get("subject"), Some(&"Test".to_string()));
        assert_eq!(metadata.get("date"), Some(&"2024-01-01".to_string()));
        assert_eq!(metadata.len(), 2);
    }

    #[test]
    fn test_clean_html_content_case_insensitive_tags() {
        let html = "<SCRIPT>code</SCRIPT><STYLE>css</STYLE><P>Text</P>";
        let cleaned = clean_html_content(html);
        assert!(!cleaned.contains("code"));
        assert!(!cleaned.contains("css"));
        assert!(cleaned.contains("Text"));
    }

    #[test]
    fn test_simple_eml_with_date() {
        let eml_content = b"From: sender@example.com\r\nTo: recipient@example.com\r\nDate: Mon, 1 Jan 2024 12:00:00 +0000\r\nSubject: Test\r\n\r\nBody";

        let result = parse_eml_content(eml_content).unwrap();
        assert!(result.date.is_some());
    }

    #[test]
    fn test_simple_eml_with_message_id() {
        let eml_content = b"From: sender@example.com\r\nTo: recipient@example.com\r\nMessage-ID: <unique@example.com>\r\nSubject: Test\r\n\r\nBody";

        let result = parse_eml_content(eml_content).unwrap();
        assert!(result.message_id.is_some());
    }

    #[test]
    fn test_simple_eml_minimal() {
        let eml_content = b"From: sender@example.com\r\n\r\nMinimal body";

        let result = parse_eml_content(eml_content).unwrap();
        assert_eq!(result.from_email, Some("sender@example.com".to_string()));
        assert_eq!(result.cleaned_text, "Minimal body");
    }

    #[test]
    fn test_regex_initialization() {
        let _ = html_tag_regex();
        let _ = script_regex();
        let _ = style_regex();
        let _ = whitespace_regex();

        let _ = html_tag_regex();
        let _ = script_regex();
        let _ = style_regex();
        let _ = whitespace_regex();
    }

    #[test]
    fn test_clean_html_content_multiline_script() {
        let html = r#"<html>
<head>
    <title>HTML Email</title>
    <style>
        body { font-family: Arial, sans-serif; }
        .header { color: blue; }
        .content { margin: 10px; }
    </style>
</head>
<body>
    <div class="header">
        <h1>Welcome to Our Service</h1>
    </div>

    <div class="content">
        <p>This email contains <strong>only HTML</strong> content.</p>

        <p>It includes:</p>
        <ul>
            <li>HTML entities: &lt;, &gt;, &amp;, &quot;</li>
            <li>Special characters: €, ©, ®</li>
            <li>Formatting: <em>italic</em>, <strong>bold</strong></li>
        </ul>

        <p>The Rust implementation should clean this HTML and extract meaningful text.</p>

        <script>
            // This script should be removed during HTML cleaning
            alert('Should not appear in extracted text');
        </script>
    </div>

    <footer>
        <p>&copy; 2024 Example Company</p>
    </footer>
</body>
</html>"#;
        let cleaned = clean_html_content(html);
        assert!(
            !cleaned.contains("script should be removed"),
            "Script content leaked into output: {}",
            cleaned
        );
        assert!(
            !cleaned.contains("alert("),
            "Script content leaked into output: {}",
            cleaned
        );
        assert!(cleaned.contains("Welcome to Our Service"));
    }

    #[test]
    fn test_html_only_eml_script_stripping() {
        // Regression test: HTML-only emails must strip <script> content.
        // mail-parser's body_text() auto-converts HTML to text via a naive
        // tag-stripper that preserves script content. We must detect this
        // case and use clean_html_content() instead.
        let eml_data = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../test_documents/email/html_only.eml"
        ))
        .expect("html_only.eml should exist");
        let result = parse_eml_content(&eml_data).unwrap();
        assert!(
            !result.cleaned_text.contains("script should be removed"),
            "Script content leaked into cleaned_text: {}",
            result.cleaned_text
        );
        assert!(
            !result.cleaned_text.contains("alert("),
            "Script content leaked into cleaned_text: {}",
            result.cleaned_text
        );
        assert!(
            result.cleaned_text.contains("Welcome to Our Service"),
            "Expected content missing from cleaned_text: {}",
            result.cleaned_text
        );
    }

    #[test]
    fn test_parse_msg_content_invalid_with_fallback() {
        // With a user-configured fallback, invalid data still returns an error.
        let result = parse_msg_content(b"not a msg file", Some(1251));
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KreuzbergError::Parsing { .. }));
    }

    #[test]
    fn test_extract_email_content_invalid_codepage_is_silent() {
        // Unknown codepage 99999 should not cause an error on a valid EML file.
        // EML ignores fallback_codepage (it's MSG-only), so this tests no panic.
        let eml = b"From: a@b.com\r\nSubject: Test\r\n\r\nBody";
        let result = extract_email_content(eml, "message/rfc822", Some(99999));
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_msg_default_unchanged_with_real_fixture() {
        // This is a Western English fixture, so windows-1252 default is correct.
        let data = include_bytes!("../../../../test_documents/vendored/unstructured/msg/fake-email.msg");
        let result = parse_msg_content(data, None).unwrap();
        assert!(!result.cleaned_text.is_empty());
    }

    #[test]
    fn test_parse_msg_invalid_codepage_falls_back_silently() {
        // An unrecognized fallback codepage (99999) silently degrades to windows-1252.
        // For a Western English fixture the output is identical to the no-fallback case.
        let data = include_bytes!("../../../../test_documents/vendored/unstructured/msg/fake-email.msg");
        let result_invalid = parse_msg_content(data, Some(99999)).unwrap();
        let result_default = parse_msg_content(data, None).unwrap();
        assert_eq!(result_invalid.subject, result_default.subject);
        assert_eq!(result_invalid.cleaned_text, result_default.cleaned_text);
    }

    #[test]
    fn test_eml_threading_headers() {
        let eml = b"From: alice@example.com\r\n\
            To: bob@example.com\r\n\
            Subject: Re: Thread test\r\n\
            Message-ID: <msg2@example.com>\r\n\
            In-Reply-To: <msg1@example.com>\r\n\
            References: <msg0@example.com> <msg1@example.com>\r\n\
            Reply-To: noreply@example.com\r\n\
            \r\n\
            Reply body";

        let result = parse_eml_content(eml).unwrap();
        assert!(
            result.metadata.contains_key("in_reply_to"),
            "Should extract In-Reply-To header"
        );
        assert!(
            result.metadata.contains_key("references"),
            "Should extract References header"
        );
        assert!(
            result.metadata.contains_key("reply_to"),
            "Should extract Reply-To header"
        );
        assert!(result.metadata.get("reply_to").unwrap().contains("noreply@example.com"));
    }

    #[test]
    fn test_eml_raw_headers() {
        let eml = b"From: alice@example.com\r\n\
            To: bob@example.com\r\n\
            Subject: Header test\r\n\
            Content-Type: text/plain; charset=utf-8\r\n\
            MIME-Version: 1.0\r\n\
            X-Mailer: TestMailer/1.0\r\n\
            List-Id: <test.example.com>\r\n\
            List-Unsubscribe: <mailto:unsub@example.com>\r\n\
            \r\n\
            Body content";

        let result = parse_eml_content(eml).unwrap();
        assert!(
            result.metadata.contains_key("content_type"),
            "Should extract Content-Type"
        );
        assert!(result.metadata.get("content_type").unwrap().contains("text/plain"));
        assert!(
            result.metadata.contains_key("mime_version"),
            "Should extract MIME-Version"
        );
        assert_eq!(result.metadata.get("mime_version").unwrap(), "1.0");
        assert!(result.metadata.contains_key("x_mailer"), "Should extract X-Mailer");
        assert!(result.metadata.get("x_mailer").unwrap().contains("TestMailer"));
        assert!(result.metadata.contains_key("list_id"), "Should extract List-Id");
        assert!(
            result.metadata.contains_key("list_unsubscribe"),
            "Should extract List-Unsubscribe"
        );
    }

    #[test]
    fn test_eml_attachment_details_metadata() {
        let eml = b"From: alice@example.com\r\n\
            To: bob@example.com\r\n\
            Subject: With attachment\r\n\
            MIME-Version: 1.0\r\n\
            Content-Type: multipart/mixed; boundary=\"BOUNDARY\"\r\n\
            \r\n\
            --BOUNDARY\r\n\
            Content-Type: text/plain\r\n\
            \r\n\
            Body text\r\n\
            --BOUNDARY\r\n\
            Content-Type: application/pdf\r\n\
            Content-Disposition: attachment; filename=\"report.pdf\"\r\n\
            \r\n\
            FAKEPDFDATA\r\n\
            --BOUNDARY--";

        let result = parse_eml_content(eml).unwrap();
        assert!(!result.attachments.is_empty(), "Should have attachment");
        assert!(
            result.metadata.contains_key("attachment_details"),
            "Should have attachment_details"
        );
        let details = result.metadata.get("attachment_details").unwrap();
        assert!(details.contains("report.pdf"), "Should contain filename");
        assert!(details.contains("application/pdf"), "Should contain mime type");
    }

    #[test]
    fn test_extract_raw_headers_function() {
        let data = b"From: alice@example.com\r\n\
            Content-Type: multipart/mixed; boundary=foo\r\n\
            MIME-Version: 1.0\r\n\
            X-Mailer: MyApp/2.0\r\n\
            User-Agent: MyAgent/1.0\r\n\
            \r\n\
            Body";

        let headers = extract_raw_headers(data);
        assert_eq!(headers.get("content_type").unwrap(), "multipart/mixed; boundary=foo");
        assert_eq!(headers.get("mime_version").unwrap(), "1.0");
        assert_eq!(headers.get("x_mailer").unwrap(), "MyApp/2.0");
        assert_eq!(headers.get("user_agent").unwrap(), "MyAgent/1.0");
    }

    #[test]
    fn test_decompress_rtf_compressed_too_short() {
        assert!(decompress_rtf_compressed(&[0u8; 10]).is_none());
    }

    #[test]
    fn test_decompress_rtf_compressed_bad_magic() {
        let mut data = [0u8; 16];
        // Set a bogus magic value.
        data[8..12].copy_from_slice(&0xDEAD_BEEFu32.to_le_bytes());
        assert!(decompress_rtf_compressed(&data).is_none());
    }

    #[test]
    fn test_decompress_rtf_compressed_uncompressed_magic() {
        // MELA magic (0x414c454d) means the payload is uncompressed RTF.
        let rtf_payload = b"{\\rtf1 Hello}";
        let comp_size = (rtf_payload.len() + 12) as u32;
        let raw_size = rtf_payload.len() as u32;
        let mut data = Vec::new();
        data.extend_from_slice(&comp_size.to_le_bytes());
        data.extend_from_slice(&raw_size.to_le_bytes());
        data.extend_from_slice(&0x414c_454du32.to_le_bytes()); // MELA
        data.extend_from_slice(&0u32.to_le_bytes()); // CRC
        data.extend_from_slice(rtf_payload);

        let result = decompress_rtf_compressed(&data).unwrap();
        assert_eq!(result, rtf_payload);
    }

    #[test]
    fn test_strip_rtf_to_plain_text_basic() {
        let rtf = b"{\\rtf1\\ansi\\deff0 Hello World}";
        let result = strip_rtf_to_plain_text(rtf);
        assert_eq!(result, "Hello World");
    }

    #[test]
    fn test_strip_rtf_to_plain_text_par() {
        let rtf = b"{\\rtf1\\ansi Line 1\\par Line 2}";
        let result = strip_rtf_to_plain_text(rtf);
        assert!(result.contains("Line 1"));
        assert!(result.contains("Line 2"));
        assert!(result.contains('\n'));
    }

    #[test]
    fn test_strip_rtf_to_plain_text_unicode() {
        // \u8364? is the euro sign, with `?` as ANSI substitution.
        let rtf = b"{\\rtf1 Price: \\u8364?100}";
        let result = strip_rtf_to_plain_text(rtf);
        assert!(result.contains('\u{20AC}')); // Euro sign
        assert!(result.contains("100"));
    }

    #[test]
    fn test_strip_rtf_to_plain_text_hex_escape() {
        // \'e9 is e-acute in windows-1252.
        let rtf = b"{\\rtf1 caf\\'e9}";
        let result = strip_rtf_to_plain_text(rtf);
        assert!(result.contains("caf\u{00e9}")); // cafe with accent
    }

    #[test]
    fn test_strip_rtf_to_plain_text_skips_fonttbl() {
        let rtf = b"{\\rtf1{\\fonttbl{\\f0 Arial;}}{\\f0 Visible text}}";
        let result = strip_rtf_to_plain_text(rtf);
        assert!(!result.contains("Arial"));
        assert!(result.contains("Visible text"));
    }

    #[test]
    fn test_strip_rtf_to_plain_text_escaped_braces() {
        let rtf = b"{\\rtf1 Open \\{ and close \\}}";
        let result = strip_rtf_to_plain_text(rtf);
        assert!(result.contains("Open {"));
        assert!(result.contains("close }"));
    }

    #[test]
    fn test_strip_rtf_to_plain_text_empty() {
        let rtf = b"{\\rtf1}";
        let result = strip_rtf_to_plain_text(rtf);
        assert!(result.is_empty());
    }
}
