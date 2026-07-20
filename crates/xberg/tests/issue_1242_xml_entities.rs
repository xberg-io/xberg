//! Regression tests for #1242: XML entities (`&amp;`, `&lt;`, `&gt;`) dropped from extracted text.
//!
//! Since quick-xml 0.38, entity and character references arrive as `Event::GeneralRef` instead
//! of inside `Event::Text`. Readers that only handle `Event::Text` silently drop the
//! referenced characters, so `Falafel &amp; Hummus` extracts as `Falafel  Hummus`.
//!
//! Covers every streaming quick-xml reader that assembles document text: the DOCX body /
//! header / footnote parsers and the DocBook, JATS, and generic XML extractors.

mod helpers;

#[cfg(feature = "office")]
mod docx {
    use std::io::{Cursor, Write};
    use zip::CompressionMethod;
    use zip::write::{FileOptions, ZipWriter};

    use crate::helpers::extract_bytes_document;
    use xberg::ExtractionConfig;

    const DOCX_MIME: &str = "application/vnd.openxmlformats-officedocument.wordprocessingml.document";

    /// Build an in-memory DOCX whose body, table, header, and footnote all contain
    /// XML entities and character references.
    fn build_entity_docx() -> Vec<u8> {
        let mut cursor = Cursor::new(Vec::new());
        {
            let mut zip = ZipWriter::new(&mut cursor);
            let options: FileOptions<()> = FileOptions::default().compression_method(CompressionMethod::Stored);

            zip.start_file("[Content_Types].xml", options).expect("zip write");
            zip.write_all(br#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
  <Override PartName="/word/header1.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml"/>
  <Override PartName="/word/footnotes.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.footnotes+xml"/>
</Types>"#).expect("zip write");

            zip.start_file("_rels/.rels", options).expect("zip write");
            zip.write_all(br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
</Relationships>"#).expect("zip write");

            zip.start_file("word/_rels/document.xml.rels", options)
                .expect("zip write");
            zip.write_all(br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/header" Target="header1.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/footnotes" Target="footnotes.xml"/>
</Relationships>"#).expect("zip write");

            zip.start_file("word/document.xml", options).expect("zip write");
            zip.write_all(
                br#"<?xml version="1.0" encoding="UTF-8"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
            xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <w:body>
    <w:p><w:r><w:t>Falafel &amp; Hummus &lt;combo&gt; 5&gt;3</w:t></w:r></w:p>
    <w:p><w:r><w:t>em&#8212;dash and hex&#x2014;dash</w:t></w:r></w:p>
    <w:tbl>
      <w:tr><w:tc><w:p><w:r><w:t>Fish &amp; Chips</w:t></w:r></w:p></w:tc></w:tr>
    </w:tbl>
    <w:p><w:r><w:footnoteReference w:id="2"/></w:r></w:p>
    <w:sectPr><w:headerReference w:type="default" r:id="rId1"/></w:sectPr>
  </w:body>
</w:document>"#,
            )
            .expect("zip write");

            zip.start_file("word/header1.xml", options).expect("zip write");
            zip.write_all(
                br#"<?xml version="1.0" encoding="UTF-8"?>
<w:hdr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:p><w:r><w:t>Header: Q&amp;A session</w:t></w:r></w:p>
</w:hdr>"#,
            )
            .expect("zip write");

            zip.start_file("word/footnotes.xml", options).expect("zip write");
            zip.write_all(
                br#"<?xml version="1.0" encoding="UTF-8"?>
<w:footnotes xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:footnote w:id="2">
    <w:p><w:r><w:t>Footnote: salt &amp; pepper</w:t></w:r></w:p>
  </w:footnote>
</w:footnotes>"#,
            )
            .expect("zip write");

            zip.finish().expect("zip finish");
        }
        cursor.into_inner()
    }

    #[tokio::test]
    async fn test_docx_body_preserves_xml_entities() {
        let bytes = build_entity_docx();
        let result = extract_bytes_document(&bytes, DOCX_MIME, &ExtractionConfig::default())
            .await
            .expect("extraction should succeed");

        assert!(
            result.content.contains("Falafel & Hummus <combo> 5>3"),
            "body text must keep &, <, > entities; got: {:?}",
            result.content
        );
    }

    #[tokio::test]
    async fn test_docx_body_preserves_character_references() {
        let bytes = build_entity_docx();
        let result = extract_bytes_document(&bytes, DOCX_MIME, &ExtractionConfig::default())
            .await
            .expect("extraction should succeed");

        assert!(
            result.content.contains("em\u{2014}dash and hex\u{2014}dash"),
            "decimal and hex character references must be decoded; got: {:?}",
            result.content
        );
    }

    #[tokio::test]
    async fn test_docx_table_cell_preserves_xml_entities() {
        let bytes = build_entity_docx();
        let result = extract_bytes_document(&bytes, DOCX_MIME, &ExtractionConfig::default())
            .await
            .expect("extraction should succeed");

        assert!(
            result.content.contains("Fish & Chips"),
            "table cell text must keep the & entity; got: {:?}",
            result.content
        );
    }

    #[tokio::test]
    async fn test_docx_header_and_footnote_preserve_xml_entities() {
        let bytes = build_entity_docx();
        let config = ExtractionConfig {
            include_document_structure: true,
            ..Default::default()
        };
        let result = extract_bytes_document(&bytes, DOCX_MIME, &config)
            .await
            .expect("extraction should succeed");

        // Footnote definitions render into plain content as a trailing section. ~keep
        assert!(
            result.content.contains("salt & pepper"),
            "footnote text must keep the & entity; got: {:?}",
            result.content
        );

        // Header content lives on the Header layer, which plain output excludes
        // by design; assert on the document structure instead. ~keep
        let structure = result.document.expect("document structure requested");
        let header_texts: Vec<String> = structure
            .nodes
            .iter()
            .map(|node| format!("{:?}", node.content))
            .collect();
        assert!(
            header_texts.iter().any(|text| text.contains("Header: Q&A session")),
            "header node must keep the & entity; got nodes: {:?}",
            header_texts
        );
    }
}

#[cfg(feature = "xml")]
mod xml_family {
    use crate::helpers::extract_bytes_document;
    use xberg::ExtractionConfig;

    #[tokio::test]
    async fn test_generic_xml_preserves_entities() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
<root><item>Cats &amp; Dogs &lt;tag&gt; 5&gt;3 em&#8212;dash</item></root>"#;
        let result = extract_bytes_document(xml, "application/xml", &ExtractionConfig::default())
            .await
            .expect("extraction should succeed");

        assert!(
            result.content.contains("Cats & Dogs <tag> 5>3 em\u{2014}dash"),
            "generic XML text must keep entities and char refs; got: {:?}",
            result.content
        );
    }

    #[tokio::test]
    async fn test_docbook_preserves_entities() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
<article xmlns="http://docbook.org/ns/docbook" version="5.0">
  <title>R&amp;D report</title>
  <para>Profits &amp; losses &lt;audited&gt; 5&gt;3</para>
</article>"#;
        let result = extract_bytes_document(xml, "application/docbook+xml", &ExtractionConfig::default())
            .await
            .expect("extraction should succeed");

        assert!(
            result.content.contains("Profits & losses <audited> 5>3"),
            "DocBook para text must keep entities; got: {:?}",
            result.content
        );
    }

    #[tokio::test]
    async fn test_jats_preserves_entities() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
<article>
  <front><article-meta><title-group>
    <article-title>Genes &amp; Development</article-title>
  </title-group></article-meta></front>
  <body><p>Expression &lt;0.05 &amp; significant, 5&gt;3</p></body>
</article>"#;
        let result = extract_bytes_document(xml, "application/x-jats+xml", &ExtractionConfig::default())
            .await
            .expect("extraction should succeed");

        assert!(
            result.content.contains("Expression <0.05 & significant, 5>3"),
            "JATS body text must keep entities; got: {:?}",
            result.content
        );
    }
}
