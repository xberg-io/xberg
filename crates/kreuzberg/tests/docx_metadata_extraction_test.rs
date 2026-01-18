//! End-to-end integration test for DOCX metadata extraction

#![cfg(feature = "office")]

use kreuzberg::{ExtractionConfig, extract_file};

#[tokio::test]
async fn test_docx_full_metadata_extraction() {
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let test_file = workspace_root.join("test_documents/documents/word_sample.docx");

    if !test_file.exists() {
        println!("Skipping test: Test file not found at {:?}", test_file);
        return;
    }

    let result = extract_file(&test_file, None, &ExtractionConfig::default())
        .await
        .expect("Should extract DOCX successfully");

    assert!(!result.content.is_empty(), "Content should not be empty");
    assert!(
        result.content.to_lowercase().contains("swim"),
        "Content should contain 'swim'"
    );

    assert_eq!(
        result.metadata.additional.get("created_by").and_then(|v| v.as_str()),
        Some("Christoph Auer"),
        "Should have correct creator"
    );
    assert_eq!(
        result.metadata.additional.get("modified_by").and_then(|v| v.as_str()),
        Some("Maxim Lysak"),
        "Should have correct last modified by"
    );
    assert_eq!(
        result.metadata.additional.get("created_at").and_then(|v| v.as_str()),
        Some("2024-10-09T12:43:00Z"),
        "Should have correct creation date"
    );
    assert_eq!(
        result.metadata.additional.get("revision").and_then(|v| v.as_str()),
        Some("7"),
        "Should have revision number"
    );

    assert_eq!(
        result.metadata.additional.get("page_count").and_then(|v| v.as_i64()),
        Some(2),
        "Should have 2 pages"
    );
    assert_eq!(
        result.metadata.additional.get("word_count").and_then(|v| v.as_i64()),
        Some(108),
        "Should have 108 words"
    );
    assert_eq!(
        result
            .metadata
            .additional
            .get("character_count")
            .and_then(|v| v.as_i64()),
        Some(620),
        "Should have 620 characters"
    );
    assert_eq!(
        result.metadata.additional.get("line_count").and_then(|v| v.as_i64()),
        Some(5),
        "Should have 5 lines"
    );
    assert_eq!(
        result
            .metadata
            .additional
            .get("paragraph_count")
            .and_then(|v| v.as_i64()),
        Some(1),
        "Should have 1 paragraph"
    );

    println!("✅ DOCX metadata extraction test passed!");
    println!("   Found {} metadata fields", result.metadata.additional.len());
}

#[tokio::test]
async fn test_docx_minimal_metadata_extraction() {
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let test_file = workspace_root.join("test_documents/documents/lorem_ipsum.docx");

    if !test_file.exists() {
        println!("Skipping test: Test file not found at {:?}", test_file);
        return;
    }

    let result = extract_file(&test_file, None, &ExtractionConfig::default())
        .await
        .expect("Should extract DOCX successfully");

    assert!(!result.content.is_empty(), "Content should not be empty");

    assert_eq!(
        result.metadata.additional.get("page_count").and_then(|v| v.as_i64()),
        Some(1),
        "Should have 1 page"
    );
    assert_eq!(
        result.metadata.additional.get("word_count").and_then(|v| v.as_i64()),
        Some(520),
        "Should have 520 words"
    );

    println!("✅ DOCX minimal metadata extraction test passed!");
}

#[tokio::test]
async fn test_docx_keywords_extraction() {
    // This test verifies that DOCX keywords metadata is properly parsed
    // from comma-separated strings into Vec<String> in Metadata.keywords
    //
    // Addresses GitHub issue #309: DOCX keyword extraction was returning
    // strings instead of parsed keyword lists, causing FunctionClauseError
    // in the Elixir binding.

    use std::io::Write;
    use tempfile::NamedTempFile;
    use zip::CompressionMethod;
    use zip::write::{FileOptions, ZipWriter};

    // Create a minimal DOCX with keywords metadata
    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");

    {
        let mut zip = ZipWriter::new(&mut temp_file);
        let options: FileOptions<()> = FileOptions::default().compression_method(CompressionMethod::Stored);

        // Add [Content_Types].xml
        zip.start_file("[Content_Types].xml", options).unwrap();
        zip.write_all(br#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
  <Override PartName="/docProps/core.xml" ContentType="application/vnd.openxmlformats-package.core-properties+xml"/>
</Types>"#).unwrap();

        // Add _rels/.rels
        zip.start_file("_rels/.rels", options).unwrap();
        zip.write_all(br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties" Target="docProps/core.xml"/>
</Relationships>"#).unwrap();

        // Add word/document.xml with simple content
        zip.start_file("word/document.xml", options).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:r>
        <w:t>Test document for keyword extraction</w:t>
      </w:r>
    </w:p>
  </w:body>
</w:document>"#,
        )
        .unwrap();

        // Add docProps/core.xml with keywords (comma-separated string)
        zip.start_file("docProps/core.xml", options).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8"?>
<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties"
                   xmlns:dc="http://purl.org/dc/elements/1.1/"
                   xmlns:dcterms="http://purl.org/dc/terms/">
  <dc:title>Test Document</dc:title>
  <dc:creator>Test Author</dc:creator>
  <cp:keywords>rust, docx, extraction, metadata, test</cp:keywords>
  <dc:subject>Testing keyword extraction</dc:subject>
</cp:coreProperties>"#,
        )
        .unwrap();

        zip.finish().unwrap();
    }

    // Extract the DOCX file
    let result = extract_file(
        temp_file.path(),
        Some("application/vnd.openxmlformats-officedocument.wordprocessingml.document"),
        &ExtractionConfig::default(),
    )
    .await
    .expect("Should extract DOCX with keywords successfully");

    // Verify content was extracted
    assert!(!result.content.is_empty(), "Content should not be empty");
    assert!(
        result.content.contains("Test document for keyword extraction"),
        "Content should match document text"
    );

    // Verify keywords were parsed into Vec<String> in Metadata.keywords
    assert!(
        result.metadata.keywords.is_some(),
        "Keywords should be present in metadata.keywords"
    );

    let keywords = result.metadata.keywords.as_ref().unwrap();
    assert_eq!(
        keywords.len(),
        5,
        "Should have 5 keywords parsed from comma-separated string"
    );

    // Verify individual keywords were trimmed and parsed correctly
    assert_eq!(keywords[0], "rust", "First keyword should be 'rust'");
    assert_eq!(keywords[1], "docx", "Second keyword should be 'docx'");
    assert_eq!(keywords[2], "extraction", "Third keyword should be 'extraction'");
    assert_eq!(keywords[3], "metadata", "Fourth keyword should be 'metadata'");
    assert_eq!(keywords[4], "test", "Fifth keyword should be 'test'");

    // Verify other metadata was also extracted
    assert_eq!(
        result.metadata.additional.get("created_by").and_then(|v| v.as_str()),
        Some("Test Author"),
        "Should have correct creator"
    );
    assert_eq!(
        result.metadata.additional.get("title").and_then(|v| v.as_str()),
        Some("Test Document"),
        "Should have correct title"
    );
    assert_eq!(
        result.metadata.additional.get("subject").and_then(|v| v.as_str()),
        Some("Testing keyword extraction"),
        "Should have correct subject"
    );

    println!("✅ DOCX keywords extraction test passed!");
    println!("   Extracted keywords: {:?}", keywords);
}
