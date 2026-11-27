//! Regression tests for xberg-io/xberg#3416: a DOCX carrying a large EMF drawing was rejected
//! as a ZIP bomb. Office embeds charts and diagrams as EMF/WMF, which are long runs of
//! repeated drawing records: real ones are several MiB and deflate at 100-300:1, past the
//! default `max_compression_ratio` for the member and, in a short document, for the whole
//! archive. The ratio cap now applies only once the member or the archive total reaches
//! 32 MiB; a member at or above that size is still rejected.

#![cfg(feature = "office")]

mod helpers;
use helpers::extract_bytes_document_blocking;

use std::io::{Cursor, Write};
use xberg::ExtractionConfig;
use zip::CompressionMethod;
use zip::write::{FileOptions, ZipWriter};

const DOCX_MIME: &str = "application/vnd.openxmlformats-officedocument.wordprocessingml.document";
const MEDIA_PART: &str = "word/media/image1.emf";
const PARAGRAPH: &str = "Chart summary paragraph.";

/// An EMF-shaped payload: a header record followed by one drawing record repeated, which
/// deflates the way real Office chart drawings do.
fn emf_like_drawing(len: usize) -> Vec<u8> {
    const RECORD: [u8; 32] = [
        0x56, 0x00, 0x00, 0x00, 0x20, 0x00, 0x00, 0x00, 0x10, 0x00, 0x20, 0x00, 0x30, 0x00, 0x40, 0x00, 0x02, 0x00,
        0x00, 0x00, 0x11, 0x00, 0x22, 0x00, 0x33, 0x00, 0x44, 0x00, 0x55, 0x00, 0x66, 0x00,
    ];
    let mut data = b"\x01\x00\x00\x00\x6c\x00\x00\x00 EMF".to_vec();
    data.extend(RECORD.iter().copied().cycle().take(len - data.len()));
    data
}

fn build_docx(media: &[u8]) -> Vec<u8> {
    let document = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>{PARAGRAPH}</w:t></w:r></w:p></w:body></w:document>"#
    );
    let parts: [(&str, &[u8]); 4] = [
        (
            "[Content_Types].xml",
            br#"<?xml version="1.0" encoding="UTF-8"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Default Extension="emf" ContentType="image/x-emf"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>"#,
        ),
        (
            "_rels/.rels",
            br#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#,
        ),
        ("word/document.xml", document.as_bytes()),
        (MEDIA_PART, media),
    ];

    let mut cursor = Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut cursor);
        let options: FileOptions<()> = FileOptions::default().compression_method(CompressionMethod::Deflated);
        for (name, data) in parts {
            zip.start_file(name, options).expect("zip entry");
            zip.write_all(data).expect("zip write");
        }
        zip.finish().expect("zip finish");
    }
    cursor.into_inner()
}

/// `(member ratio, whole-archive ratio)` as the central directory declares them.
fn declared_ratios(bytes: &[u8]) -> (f64, f64) {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).expect("open DOCX");
    let (mut total_compressed, mut total_uncompressed) = (0u64, 0u64);
    let mut member_ratio = 0.0;
    for index in 0..archive.len() {
        let entry = archive.by_index_raw(index).expect("entry header");
        total_compressed += entry.compressed_size();
        total_uncompressed += entry.size();
        if entry.name().expect("entry name") == MEDIA_PART {
            member_ratio = entry.size() as f64 / entry.compressed_size() as f64;
        }
    }
    (member_ratio, total_uncompressed as f64 / total_compressed as f64)
}

fn config() -> ExtractionConfig {
    ExtractionConfig {
        use_cache: false,
        ..Default::default()
    }
}

#[test]
fn docx_with_a_highly_compressible_2_mib_emf_drawing_extracts() {
    let bytes = build_docx(&emf_like_drawing(2 * 1024 * 1024));

    let max_ratio = xberg::SecurityLimits::default().max_compression_ratio as f64;
    let (member_ratio, archive_ratio) = declared_ratios(&bytes);
    assert!(
        member_ratio > max_ratio && archive_ratio > max_ratio,
        "the fixture must exceed the default ratio for the member and the archive, got {member_ratio:.1}:1 and \
         {archive_ratio:.1}:1"
    );

    let result = extract_bytes_document_blocking(&bytes, DOCX_MIME, &config())
        .expect("a 2 MiB EMF drawing cannot exhaust memory and must not reject the document");
    assert!(
        result.content.contains(PARAGRAPH),
        "the document text must survive, got: {:?}",
        result.content
    );
}

#[test]
fn docx_with_a_media_member_past_the_ratio_floor_is_still_rejected() {
    let bytes = build_docx(&emf_like_drawing(40 * 1024 * 1024));

    let error = extract_bytes_document_blocking(&bytes, DOCX_MIME, &config())
        .expect_err("a 40 MiB member far past the ratio cap is a bomb, media part or not");
    let message = error.to_string();
    assert!(
        message.contains("Potential ZIP bomb detected") && message.contains("uncompressed 41943040B"),
        "expected the 40 MiB media member to be named as the bomb, got: {message}"
    );
}
