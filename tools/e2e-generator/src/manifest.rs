//! Parity manifest generator.
//!
//! Reflects on kreuzberg's core types via serde serialization to produce
//! a `parity-manifest.json` describing every public type, field, and variant
//! that language bindings must match.

use std::collections::BTreeMap;

use anyhow::Result;
use camino::Utf8Path;
use serde_json::Value;

use crate::parity::{FieldDef, ParityManifest, TypeDef, VariantDef};

// ---------------------------------------------------------------------------
// Feature-gate annotations (hardcoded lookup)
// ---------------------------------------------------------------------------

const FEATURE_GATES: &[(&str, &str, &str)] = &[
    ("ExtractionConfig", "pdf_options", "pdf"),
    ("ExtractionConfig", "html_options", "html"),
    ("ExtractionConfig", "html_output", "html"),
    ("ExtractionConfig", "keywords", "keywords"),
    ("ExtractionConfig", "security_limits", "archives"),
    ("ExtractionConfig", "layout", "layout-detection"),
    ("ExtractionConfig", "tree_sitter", "tree-sitter"),
    ("ExtractionResult", "extracted_keywords", "keywords"),
];

/// Fields that use `skip_serializing_if = "Option::is_none"` or similar.
/// These are considered optional (required = false).
/// We hardcode these because serde doesn't expose skip conditions at runtime.
const OPTIONAL_FIELDS: &[(&str, &str)] = &[
    // ExtractionResult
    ("ExtractionResult", "detected_languages"),
    ("ExtractionResult", "chunks"),
    ("ExtractionResult", "images"),
    ("ExtractionResult", "pages"),
    ("ExtractionResult", "elements"),
    ("ExtractionResult", "djot_content"),
    ("ExtractionResult", "ocr_elements"),
    ("ExtractionResult", "document"),
    ("ExtractionResult", "extracted_keywords"),
    ("ExtractionResult", "quality_score"),
    ("ExtractionResult", "annotations"),
    ("ExtractionResult", "children"),
    ("ExtractionResult", "uris"),
    ("ExtractionResult", "processing_warnings"), // Vec with skip_serializing_if = "Vec::is_empty"
    // ExtractionConfig
    ("ExtractionConfig", "ocr"),
    ("ExtractionConfig", "force_ocr_pages"),
    ("ExtractionConfig", "chunking"),
    ("ExtractionConfig", "images"),
    ("ExtractionConfig", "pdf_options"),
    ("ExtractionConfig", "token_reduction"),
    ("ExtractionConfig", "language_detection"),
    ("ExtractionConfig", "pages"),
    ("ExtractionConfig", "keywords"),
    ("ExtractionConfig", "postprocessor"),
    ("ExtractionConfig", "html_options"),
    ("ExtractionConfig", "html_output"),
    ("ExtractionConfig", "extraction_timeout_secs"),
    ("ExtractionConfig", "max_concurrent_extractions"),
    ("ExtractionConfig", "security_limits"),
    ("ExtractionConfig", "layout"),
    ("ExtractionConfig", "acceleration"),
    ("ExtractionConfig", "cache_namespace"),
    ("ExtractionConfig", "cache_ttl_secs"),
    ("ExtractionConfig", "email"),
    ("ExtractionConfig", "concurrency"),
    ("ExtractionConfig", "tree_sitter"),
    // Metadata
    ("Metadata", "title"),
    ("Metadata", "subject"),
    ("Metadata", "authors"),
    ("Metadata", "keywords"),
    ("Metadata", "language"),
    ("Metadata", "created_at"),
    ("Metadata", "modified_at"),
    ("Metadata", "created_by"),
    ("Metadata", "modified_by"),
    ("Metadata", "pages"),
    ("Metadata", "format"),
    ("Metadata", "image_preprocessing"),
    ("Metadata", "json_schema"),
    ("Metadata", "error"),
    ("Metadata", "extraction_duration_ms"),
    ("Metadata", "category"),
    ("Metadata", "tags"),
    ("Metadata", "document_version"),
    ("Metadata", "abstract_text"),
    ("Metadata", "output_format"),
    // Uri
    ("Uri", "label"),
    ("Uri", "page"),
    // Table
    ("Table", "bounding_box"),
    // ExtractedImage
    ("ExtractedImage", "page_number"),
    ("ExtractedImage", "width"),
    ("ExtractedImage", "height"),
    ("ExtractedImage", "colorspace"),
    ("ExtractedImage", "bits_per_component"),
    ("ExtractedImage", "description"),
    ("ExtractedImage", "ocr_result"),
    ("ExtractedImage", "bounding_box"),
    ("ExtractedImage", "source_path"),
    // Chunk
    ("Chunk", "embedding"),
    // ChunkMetadata
    ("ChunkMetadata", "token_count"),
    ("ChunkMetadata", "first_page"),
    ("ChunkMetadata", "last_page"),
    ("ChunkMetadata", "heading_context"),
    // Element / ElementMetadata
    ("ElementMetadata", "page_number"),
    ("ElementMetadata", "filename"),
    ("ElementMetadata", "coordinates"),
    ("ElementMetadata", "element_index"),
    // OcrElement
    ("OcrElement", "rotation"),
    ("OcrElement", "parent_id"),
    // OcrConfidence
    ("OcrConfidence", "detection"),
    // OcrRotation
    ("OcrRotation", "confidence"),
    // PdfAnnotation
    ("PdfAnnotation", "content"),
    ("PdfAnnotation", "bounding_box"),
    // DocumentStructure
    ("DocumentStructure", "source_format"),
    // DocumentNode
    ("DocumentNode", "parent"),
    ("DocumentNode", "bbox"),
    ("DocumentNode", "attributes"),
    ("DocumentNode", "page"),
    ("DocumentNode", "page_end"),
    // NodeContent variants with optional fields
    ("NodeContent::Image", "description"),
    ("NodeContent::Image", "image_index"),
    ("NodeContent::Image", "src"),
    ("NodeContent::Code", "language"),
    ("NodeContent::Group", "label"),
    ("NodeContent::Group", "heading_level"),
    ("NodeContent::Group", "heading_text"),
    ("NodeContent::Slide", "title"),
    ("NodeContent::Admonition", "title"),
    // GridCell
    ("GridCell", "bbox"),
    // PageContent
    ("PageContent", "hierarchy"),
    ("PageContent", "is_blank"),
    ("PageContent", "layout_regions"),
    // PageInfo
    ("PageInfo", "title"),
    ("PageInfo", "dimensions"),
    ("PageInfo", "image_count"),
    ("PageInfo", "table_count"),
    ("PageInfo", "hidden"),
    ("PageInfo", "is_blank"),
    // PageStructure
    ("PageStructure", "boundaries"),
    ("PageStructure", "pages"),
    // ProcessingWarning — all required
    // ArchiveEntry — all required
    // Keyword
    ("Keyword", "positions"),
];

fn is_optional(type_name: &str, field_name: &str) -> bool {
    OPTIONAL_FIELDS.iter().any(|(t, f)| *t == type_name && *f == field_name)
}

fn feature_gate_for(type_name: &str, field_name: &str) -> Option<String> {
    FEATURE_GATES
        .iter()
        .find(|(t, f, _)| *t == type_name && *f == field_name)
        .map(|(_, _, gate)| (*gate).to_string())
}

// ---------------------------------------------------------------------------
// JSON type classification
// ---------------------------------------------------------------------------

fn json_type_str(value: &Value) -> &'static str {
    match value {
        Value::String(_) => "string",
        Value::Number(_) => "number",
        Value::Bool(_) => "boolean",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
        Value::Null => "null",
    }
}

// ---------------------------------------------------------------------------
// Struct reflection helpers
// ---------------------------------------------------------------------------

fn reflect_struct_fields(type_name: &str, json: &Value) -> BTreeMap<String, FieldDef> {
    let mut fields = BTreeMap::new();
    if let Value::Object(map) = json {
        for (key, val) in map {
            fields.insert(
                key.clone(),
                FieldDef {
                    json_type: json_type_str(val).to_string(),
                    required: !is_optional(type_name, key),
                    feature_gate: feature_gate_for(type_name, key),
                },
            );
        }
    }
    fields
}

// ---------------------------------------------------------------------------
// Fully-populated sample instances
// ---------------------------------------------------------------------------

fn sample_extraction_result() -> Value {
    use bytes::Bytes;
    use std::borrow::Cow;
    use std::collections::HashMap;
    use std::sync::Arc;

    let table = kreuzberg::types::tables::Table {
        cells: vec![vec!["A".to_string()]],
        markdown: "| A |".to_string(),
        page_number: 1,
        bounding_box: Some(kreuzberg::types::extraction::BoundingBox {
            x0: 0.0,
            y0: 0.0,
            x1: 100.0,
            y1: 50.0,
        }),
    };

    let image = kreuzberg::types::extraction::ExtractedImage {
        data: Bytes::from_static(&[0xFF, 0xD8]),
        format: Cow::Borrowed("jpeg"),
        image_index: 0,
        page_number: Some(1),
        width: Some(640),
        height: Some(480),
        colorspace: Some("RGB".to_string()),
        bits_per_component: Some(8),
        is_mask: false,
        description: Some("sample".to_string()),
        ocr_result: None,
        bounding_box: Some(kreuzberg::types::extraction::BoundingBox {
            x0: 10.0,
            y0: 20.0,
            x1: 100.0,
            y1: 200.0,
        }),
        source_path: Some("img.png".to_string()),
    };

    let chunk = kreuzberg::types::extraction::Chunk {
        content: "chunk text".to_string(),
        chunk_type: Default::default(),
        embedding: Some(vec![0.1, 0.2]),
        metadata: kreuzberg::types::extraction::ChunkMetadata {
            byte_start: 0,
            byte_end: 10,
            token_count: Some(3),
            chunk_index: 0,
            total_chunks: 1,
            first_page: Some(1),
            last_page: Some(1),
            heading_context: Some(kreuzberg::types::extraction::HeadingContext {
                headings: vec![kreuzberg::types::extraction::HeadingLevel {
                    level: 1,
                    text: "Title".to_string(),
                }],
            }),
        },
    };

    let page_content = kreuzberg::types::page::PageContent {
        page_number: 1,
        content: "page text".to_string(),
        tables: vec![Arc::new(table.clone())],
        images: vec![Arc::new(image.clone())],
        hierarchy: Some(kreuzberg::types::page::PageHierarchy {
            block_count: 1,
            blocks: vec![kreuzberg::types::page::HierarchicalBlock {
                text: "heading".to_string(),
                font_size: 14.0,
                level: "h1".to_string(),
                bbox: Some((0.0, 0.0, 100.0, 20.0)),
            }],
        }),
        is_blank: Some(false),
        layout_regions: None,
    };

    let element = kreuzberg::types::extraction::Element {
        element_id: kreuzberg::types::extraction::ElementId::new("abc123").unwrap(),
        element_type: kreuzberg::types::extraction::ElementType::Title,
        text: "Title".to_string(),
        metadata: kreuzberg::types::extraction::ElementMetadata {
            page_number: Some(1),
            filename: Some("doc.pdf".to_string()),
            coordinates: Some(kreuzberg::types::extraction::BoundingBox {
                x0: 0.0,
                y0: 0.0,
                x1: 100.0,
                y1: 20.0,
            }),
            element_index: Some(0),
            additional: HashMap::new(),
        },
    };

    let ocr_element = kreuzberg::types::ocr_elements::OcrElement {
        text: "Hello".to_string(),
        geometry: kreuzberg::types::ocr_elements::OcrBoundingGeometry::Rectangle {
            left: 0,
            top: 0,
            width: 100,
            height: 20,
        },
        confidence: kreuzberg::types::ocr_elements::OcrConfidence {
            detection: Some(0.95),
            recognition: 0.88,
        },
        level: kreuzberg::types::ocr_elements::OcrElementLevel::Line,
        rotation: Some(kreuzberg::types::ocr_elements::OcrRotation {
            angle_degrees: 0.0,
            confidence: Some(0.99),
        }),
        page_number: 1,
        parent_id: Some("parent-1".to_string()),
        backend_metadata: {
            let mut m = HashMap::new();
            m.insert("backend".to_string(), serde_json::json!("tesseract"));
            m
        },
    };

    let annotation = kreuzberg::types::annotations::PdfAnnotation {
        annotation_type: kreuzberg::types::annotations::PdfAnnotationType::Text,
        content: Some("note".to_string()),
        page_number: 1,
        bounding_box: Some(kreuzberg::types::extraction::BoundingBox {
            x0: 0.0,
            y0: 0.0,
            x1: 50.0,
            y1: 50.0,
        }),
    };

    let doc_structure = kreuzberg::types::document_structure::DocumentStructure {
        nodes: vec![kreuzberg::types::document_structure::DocumentNode {
            id: kreuzberg::types::document_structure::NodeId::new("node-abc"),
            content: kreuzberg::types::document_structure::NodeContent::Paragraph {
                text: "Hello".to_string(),
            },
            parent: None,
            children: vec![],
            content_layer: kreuzberg::types::document_structure::ContentLayer::Body,
            page: Some(1),
            page_end: Some(1),
            bbox: Some(kreuzberg::types::extraction::BoundingBox {
                x0: 0.0,
                y0: 0.0,
                x1: 100.0,
                y1: 20.0,
            }),
            annotations: vec![kreuzberg::types::document_structure::TextAnnotation {
                start: 0,
                end: 5,
                kind: kreuzberg::types::document_structure::AnnotationKind::Bold,
            }],
            attributes: Some({
                let mut m = HashMap::new();
                m.insert("class".to_string(), "intro".to_string());
                m
            }),
        }],
        source_format: Some("pdf".to_string()),
        relationships: vec![kreuzberg::types::document_structure::DocumentRelationship {
            source: kreuzberg::types::document_structure::NodeIndex(0),
            target: kreuzberg::types::document_structure::NodeIndex(0),
            kind: kreuzberg::types::document_structure::RelationshipKind::FootnoteReference,
        }],
    };

    let uri = kreuzberg::types::uri::Uri {
        url: "https://example.com".to_string(),
        label: Some("Example".to_string()),
        page: Some(1),
        kind: kreuzberg::types::uri::UriKind::Hyperlink,
    };

    let archive_entry = kreuzberg::types::extraction::ArchiveEntry {
        path: "inner/doc.txt".to_string(),
        mime_type: "text/plain".to_string(),
        result: Box::new(kreuzberg::ExtractionResult::default()),
    };

    let keyword = kreuzberg::keywords::Keyword {
        text: "rust".to_string(),
        score: 0.95,
        algorithm: kreuzberg::keywords::KeywordAlgorithm::default(),
        positions: Some(vec![0, 42]),
    };

    let warning = kreuzberg::types::extraction::ProcessingWarning {
        source: Cow::Borrowed("chunking"),
        message: Cow::Borrowed("fallback used"),
    };

    let result = kreuzberg::ExtractionResult {
        content: "Hello world".to_string(),
        mime_type: Cow::Borrowed("text/plain"),
        metadata: kreuzberg::types::metadata::Metadata::default(),
        tables: vec![table],
        detected_languages: Some(vec!["en".to_string()]),
        chunks: Some(vec![chunk]),
        images: Some(vec![image]),
        pages: Some(vec![page_content]),
        elements: Some(vec![element]),
        djot_content: Some(kreuzberg::types::djot::DjotContent {
            plain_text: "djot".to_string(),
            blocks: vec![],
            metadata: kreuzberg::types::metadata::Metadata::default(),
            tables: vec![],
            images: vec![],
            links: vec![],
            footnotes: vec![],
            attributes: vec![],
        }),
        ocr_elements: Some(vec![ocr_element]),
        document: Some(doc_structure),
        extracted_keywords: Some(vec![keyword]),
        quality_score: Some(0.95),
        processing_warnings: vec![warning],
        annotations: Some(vec![annotation]),
        children: Some(vec![archive_entry]),
        uris: Some(vec![uri]),
        llm_usage: None,
        formatted_content: None,
        code_intelligence: None,
        ocr_internal_document: None,
        structured_output: None,
    };

    serde_json::to_value(&result).expect("ExtractionResult serialization")
}

fn sample_extraction_config() -> Value {
    let config = kreuzberg::ExtractionConfig {
        use_cache: true,
        enable_quality_processing: true,
        ocr: Some(kreuzberg::OcrConfig::default()),
        force_ocr: false,
        force_ocr_pages: Some(vec![1]),
        disable_ocr: false,
        chunking: Some(kreuzberg::ChunkingConfig::default()),
        content_filter: None,
        images: Some(kreuzberg::ImageExtractionConfig::default()),
        pdf_options: Some(kreuzberg::PdfConfig::default()),
        token_reduction: Some(kreuzberg::TokenReductionConfig {
            mode: "moderate".to_string(),
            preserve_important_words: true,
        }),
        language_detection: Some(kreuzberg::LanguageDetectionConfig {
            enabled: true,
            min_confidence: 0.5,
            detect_multiple: false,
        }),
        pages: Some(kreuzberg::PageConfig::default()),
        keywords: Some(kreuzberg::keywords::KeywordConfig::default()),
        postprocessor: Some(kreuzberg::PostProcessorConfig::default()),
        html_options: Some(kreuzberg::extraction::html::ConversionOptions::default()),
        extraction_timeout_secs: Some(60),
        max_concurrent_extractions: Some(4),
        result_format: kreuzberg::types::OutputFormat::Unified,
        security_limits: Some(kreuzberg::extractors::security::SecurityLimits::default()),
        output_format: kreuzberg::core::config::OutputFormat::Plain,
        layout: Some(kreuzberg::LayoutDetectionConfig::default()),
        include_document_structure: true,
        acceleration: Some(kreuzberg::AccelerationConfig::default()),
        cache_namespace: Some("test".to_string()),
        cache_ttl_secs: Some(3600),
        email: Some(kreuzberg::EmailConfig::default()),
        concurrency: Some(kreuzberg::core::config::ConcurrencyConfig::default()),
        max_archive_depth: 3,
        tree_sitter: Some(kreuzberg::TreeSitterConfig::default()),
        structured_extraction: None,
        html_output: None,
        cancel_token: None,
    };

    serde_json::to_value(&config).expect("ExtractionConfig serialization")
}

// ---------------------------------------------------------------------------
// NodeContent variant reflection
// ---------------------------------------------------------------------------

fn node_content_variants() -> BTreeMap<String, VariantDef> {
    use kreuzberg::types::document_structure::{GridCell, NodeContent, TableGrid};

    let variants: Vec<(&str, NodeContent)> = vec![
        ("title", NodeContent::Title { text: "t".into() }),
        (
            "heading",
            NodeContent::Heading {
                level: 1,
                text: "h".into(),
            },
        ),
        ("paragraph", NodeContent::Paragraph { text: "p".into() }),
        ("list", NodeContent::List { ordered: true }),
        ("list_item", NodeContent::ListItem { text: "li".into() }),
        (
            "table",
            NodeContent::Table {
                grid: TableGrid {
                    rows: 1,
                    cols: 1,
                    cells: vec![GridCell {
                        content: "c".into(),
                        row: 0,
                        col: 0,
                        row_span: 1,
                        col_span: 1,
                        is_header: false,
                        bbox: Some(kreuzberg::types::extraction::BoundingBox {
                            x0: 0.0,
                            y0: 0.0,
                            x1: 10.0,
                            y1: 10.0,
                        }),
                    }],
                },
            },
        ),
        (
            "image",
            NodeContent::Image {
                description: Some("desc".into()),
                image_index: Some(0),
                src: Some("img.png".into()),
            },
        ),
        (
            "code",
            NodeContent::Code {
                text: "fn main()".into(),
                language: Some("rust".into()),
            },
        ),
        ("quote", NodeContent::Quote),
        ("formula", NodeContent::Formula { text: "E=mc^2".into() }),
        ("footnote", NodeContent::Footnote { text: "note".into() }),
        (
            "group",
            NodeContent::Group {
                label: Some("section".into()),
                heading_level: Some(1),
                heading_text: Some("Introduction".into()),
            },
        ),
        ("page_break", NodeContent::PageBreak),
        (
            "slide",
            NodeContent::Slide {
                number: 1,
                title: Some("Slide 1".into()),
            },
        ),
        ("definition_list", NodeContent::DefinitionList),
        (
            "definition_item",
            NodeContent::DefinitionItem {
                term: "term".into(),
                definition: "definition".into(),
            },
        ),
        (
            "citation",
            NodeContent::Citation {
                key: "ref1".into(),
                text: "Author 2024".into(),
            },
        ),
        (
            "admonition",
            NodeContent::Admonition {
                kind: "note".into(),
                title: Some("Note".into()),
            },
        ),
        (
            "raw_block",
            NodeContent::RawBlock {
                format: "html".into(),
                content: "<div>hi</div>".into(),
            },
        ),
        (
            "metadata_block",
            NodeContent::MetadataBlock {
                entries: vec![("key".into(), "value".into())],
            },
        ),
    ];

    let mut result = BTreeMap::new();
    for (variant_name, variant) in variants {
        let json = serde_json::to_value(&variant).expect("NodeContent variant serialization");
        let mut fields = BTreeMap::new();
        if let Value::Object(map) = &json {
            for (key, val) in map {
                // Skip the tag field itself
                if key == "node_type" {
                    continue;
                }
                let qualified = format!("NodeContent::{}", pascal_from_snake(variant_name));
                fields.insert(
                    key.clone(),
                    FieldDef {
                        json_type: json_type_str(val).to_string(),
                        required: !is_optional(&qualified, key),
                        feature_gate: None,
                    },
                );
            }
        }
        result.insert(variant_name.to_string(), VariantDef { fields });
    }
    result
}

fn pascal_from_snake(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Simple enum reflection
// ---------------------------------------------------------------------------

fn simple_enum_values<T: serde::Serialize>(variants: &[T]) -> Vec<String> {
    variants
        .iter()
        .filter_map(|v| {
            let json = serde_json::to_value(v).ok()?;
            json.as_str().map(|s| s.to_string())
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Generate the manifest as a `serde_json::Value` (for use by freshness tests).
pub fn generate_manifest_value() -> Value {
    let mut types = BTreeMap::new();

    // -- ExtractionResult --
    let er_json = sample_extraction_result();
    types.insert(
        "ExtractionResult".to_string(),
        TypeDef::Struct {
            fields: reflect_struct_fields("ExtractionResult", &er_json),
        },
    );

    // -- ExtractionConfig --
    let ec_json = sample_extraction_config();
    types.insert(
        "ExtractionConfig".to_string(),
        TypeDef::Struct {
            fields: reflect_struct_fields("ExtractionConfig", &ec_json),
        },
    );

    // -- NodeContent (tagged enum) --
    types.insert(
        "NodeContent".to_string(),
        TypeDef::TaggedEnum {
            tag_field: "node_type".to_string(),
            variants: node_content_variants(),
        },
    );

    // -- Simple enums --
    types.insert(
        "OutputFormat".to_string(),
        TypeDef::SimpleEnum {
            values: simple_enum_values(&[
                kreuzberg::core::config::OutputFormat::Plain,
                kreuzberg::core::config::OutputFormat::Markdown,
                kreuzberg::core::config::OutputFormat::Djot,
                kreuzberg::core::config::OutputFormat::Html,
                kreuzberg::core::config::OutputFormat::Structured,
            ]),
        },
    );

    types.insert(
        "ResultFormat".to_string(),
        TypeDef::SimpleEnum {
            values: simple_enum_values(&[
                kreuzberg::types::OutputFormat::Unified,
                kreuzberg::types::OutputFormat::ElementBased,
            ]),
        },
    );

    types.insert(
        "ElementType".to_string(),
        TypeDef::SimpleEnum {
            values: simple_enum_values(&[
                kreuzberg::types::extraction::ElementType::Title,
                kreuzberg::types::extraction::ElementType::NarrativeText,
                kreuzberg::types::extraction::ElementType::Heading,
                kreuzberg::types::extraction::ElementType::ListItem,
                kreuzberg::types::extraction::ElementType::Table,
                kreuzberg::types::extraction::ElementType::Image,
                kreuzberg::types::extraction::ElementType::PageBreak,
                kreuzberg::types::extraction::ElementType::CodeBlock,
                kreuzberg::types::extraction::ElementType::BlockQuote,
                kreuzberg::types::extraction::ElementType::Footer,
                kreuzberg::types::extraction::ElementType::Header,
            ]),
        },
    );

    types.insert(
        "UriKind".to_string(),
        TypeDef::SimpleEnum {
            values: simple_enum_values(&[
                kreuzberg::types::uri::UriKind::Hyperlink,
                kreuzberg::types::uri::UriKind::Image,
                kreuzberg::types::uri::UriKind::Anchor,
                kreuzberg::types::uri::UriKind::Citation,
                kreuzberg::types::uri::UriKind::Reference,
                kreuzberg::types::uri::UriKind::Email,
            ]),
        },
    );

    types.insert(
        "PdfAnnotationType".to_string(),
        TypeDef::SimpleEnum {
            values: simple_enum_values(&[
                kreuzberg::types::annotations::PdfAnnotationType::Text,
                kreuzberg::types::annotations::PdfAnnotationType::Highlight,
                kreuzberg::types::annotations::PdfAnnotationType::Link,
                kreuzberg::types::annotations::PdfAnnotationType::Stamp,
                kreuzberg::types::annotations::PdfAnnotationType::Underline,
                kreuzberg::types::annotations::PdfAnnotationType::StrikeOut,
                kreuzberg::types::annotations::PdfAnnotationType::Other,
            ]),
        },
    );

    types.insert(
        "ContentLayer".to_string(),
        TypeDef::SimpleEnum {
            values: simple_enum_values(&[
                kreuzberg::types::document_structure::ContentLayer::Body,
                kreuzberg::types::document_structure::ContentLayer::Header,
                kreuzberg::types::document_structure::ContentLayer::Footer,
                kreuzberg::types::document_structure::ContentLayer::Footnote,
            ]),
        },
    );

    types.insert(
        "RelationshipKind".to_string(),
        TypeDef::SimpleEnum {
            values: simple_enum_values(&[
                kreuzberg::types::document_structure::RelationshipKind::FootnoteReference,
                kreuzberg::types::document_structure::RelationshipKind::CitationReference,
                kreuzberg::types::document_structure::RelationshipKind::InternalLink,
                kreuzberg::types::document_structure::RelationshipKind::Caption,
                kreuzberg::types::document_structure::RelationshipKind::Label,
                kreuzberg::types::document_structure::RelationshipKind::TocEntry,
                kreuzberg::types::document_structure::RelationshipKind::CrossReference,
            ]),
        },
    );

    types.insert(
        "OcrElementLevel".to_string(),
        TypeDef::SimpleEnum {
            values: simple_enum_values(&[
                kreuzberg::types::ocr_elements::OcrElementLevel::Word,
                kreuzberg::types::ocr_elements::OcrElementLevel::Line,
                kreuzberg::types::ocr_elements::OcrElementLevel::Block,
                kreuzberg::types::ocr_elements::OcrElementLevel::Page,
            ]),
        },
    );

    types.insert(
        "KeywordAlgorithm".to_string(),
        TypeDef::SimpleEnum {
            values: simple_enum_values(&[
                kreuzberg::keywords::KeywordAlgorithm::Yake,
                kreuzberg::keywords::KeywordAlgorithm::Rake,
            ]),
        },
    );

    types.insert(
        "PageUnitType".to_string(),
        TypeDef::SimpleEnum {
            values: simple_enum_values(&[
                kreuzberg::types::page::PageUnitType::Page,
                kreuzberg::types::page::PageUnitType::Slide,
                kreuzberg::types::page::PageUnitType::Sheet,
            ]),
        },
    );

    // -- Nested struct types (reflected from the JSON of their parent) --
    reflect_nested_struct(
        &mut types,
        "Uri",
        &serde_json::to_value(&kreuzberg::types::uri::Uri {
            url: "https://example.com".to_string(),
            label: Some("label".to_string()),
            page: Some(1),
            kind: kreuzberg::types::uri::UriKind::Hyperlink,
        })
        .unwrap(),
    );

    reflect_nested_struct(
        &mut types,
        "Table",
        &serde_json::to_value(&kreuzberg::types::tables::Table {
            cells: vec![vec!["A".to_string()]],
            markdown: "| A |".to_string(),
            page_number: 1,
            bounding_box: Some(kreuzberg::types::extraction::BoundingBox {
                x0: 0.0,
                y0: 0.0,
                x1: 100.0,
                y1: 50.0,
            }),
        })
        .unwrap(),
    );

    reflect_nested_struct(
        &mut types,
        "BoundingBox",
        &serde_json::to_value(kreuzberg::types::extraction::BoundingBox {
            x0: 0.0,
            y0: 0.0,
            x1: 100.0,
            y1: 50.0,
        })
        .unwrap(),
    );

    reflect_nested_struct(
        &mut types,
        "ProcessingWarning",
        &serde_json::to_value(&kreuzberg::types::extraction::ProcessingWarning {
            source: std::borrow::Cow::Borrowed("test"),
            message: std::borrow::Cow::Borrowed("msg"),
        })
        .unwrap(),
    );

    reflect_nested_struct(
        &mut types,
        "ArchiveEntry",
        &serde_json::to_value(&kreuzberg::types::extraction::ArchiveEntry {
            path: "test.txt".to_string(),
            mime_type: "text/plain".to_string(),
            result: Box::new(kreuzberg::ExtractionResult::default()),
        })
        .unwrap(),
    );

    reflect_nested_struct(
        &mut types,
        "PdfAnnotation",
        &serde_json::to_value(&kreuzberg::types::annotations::PdfAnnotation {
            annotation_type: kreuzberg::types::annotations::PdfAnnotationType::Text,
            content: Some("note".to_string()),
            page_number: 1,
            bounding_box: Some(kreuzberg::types::extraction::BoundingBox {
                x0: 0.0,
                y0: 0.0,
                x1: 50.0,
                y1: 50.0,
            }),
        })
        .unwrap(),
    );

    reflect_nested_struct(
        &mut types,
        "Keyword",
        &serde_json::to_value(&kreuzberg::keywords::Keyword {
            text: "rust".to_string(),
            score: 0.95,
            algorithm: kreuzberg::keywords::KeywordAlgorithm::default(),
            positions: Some(vec![0]),
        })
        .unwrap(),
    );

    // -- OcrBoundingGeometry (tagged enum) --
    {
        use kreuzberg::types::ocr_elements::OcrBoundingGeometry;
        let variants_data: Vec<(&str, OcrBoundingGeometry)> = vec![
            (
                "rectangle",
                OcrBoundingGeometry::Rectangle {
                    left: 0,
                    top: 0,
                    width: 100,
                    height: 20,
                },
            ),
            (
                "quadrilateral",
                OcrBoundingGeometry::Quadrilateral {
                    points: [(0, 0), (100, 0), (100, 20), (0, 20)],
                },
            ),
        ];
        let mut variants = BTreeMap::new();
        for (name, variant) in variants_data {
            let json = serde_json::to_value(&variant).unwrap();
            let mut fields = BTreeMap::new();
            if let Value::Object(map) = &json {
                for (key, val) in map {
                    if key == "type" {
                        continue;
                    }
                    fields.insert(
                        key.clone(),
                        FieldDef {
                            json_type: json_type_str(val).to_string(),
                            required: true,
                            feature_gate: None,
                        },
                    );
                }
            }
            variants.insert(name.to_string(), VariantDef { fields });
        }
        types.insert(
            "OcrBoundingGeometry".to_string(),
            TypeDef::TaggedEnum {
                tag_field: "type".to_string(),
                variants,
            },
        );
    }

    // -- AnnotationKind (tagged enum) --
    {
        use kreuzberg::types::document_structure::AnnotationKind;
        let variants_data: Vec<(&str, AnnotationKind)> = vec![
            ("bold", AnnotationKind::Bold),
            ("italic", AnnotationKind::Italic),
            ("underline", AnnotationKind::Underline),
            ("strikethrough", AnnotationKind::Strikethrough),
            ("code", AnnotationKind::Code),
            ("subscript", AnnotationKind::Subscript),
            ("superscript", AnnotationKind::Superscript),
            (
                "link",
                AnnotationKind::Link {
                    url: "https://example.com".into(),
                    title: Some("title".into()),
                },
            ),
            ("highlight", AnnotationKind::Highlight),
            (
                "color",
                AnnotationKind::Color {
                    value: "#ff0000".into(),
                },
            ),
            ("font_size", AnnotationKind::FontSize { value: "12pt".into() }),
            (
                "custom",
                AnnotationKind::Custom {
                    name: "custom".into(),
                    value: Some("val".into()),
                },
            ),
        ];
        let mut variants = BTreeMap::new();
        for (name, variant) in variants_data {
            let json = serde_json::to_value(&variant).unwrap();
            let mut fields = BTreeMap::new();
            if let Value::Object(map) = &json {
                for (key, val) in map {
                    if key == "annotation_type" {
                        continue;
                    }
                    let qualified = format!("AnnotationKind::{}", pascal_from_snake(name));
                    fields.insert(
                        key.clone(),
                        FieldDef {
                            json_type: json_type_str(val).to_string(),
                            required: !is_optional(&qualified, key),
                            feature_gate: None,
                        },
                    );
                }
            }
            variants.insert(name.to_string(), VariantDef { fields });
        }
        types.insert(
            "AnnotationKind".to_string(),
            TypeDef::TaggedEnum {
                tag_field: "annotation_type".to_string(),
                variants,
            },
        );
    }

    // Feature profiles
    let mut feature_profiles = BTreeMap::new();
    feature_profiles.insert(
        "full".to_string(),
        vec![
            "pdf".to_string(),
            "html".to_string(),
            "archives".to_string(),
            "keywords".to_string(),
            "layout-detection".to_string(),
            "tree-sitter".to_string(),
            "embeddings".to_string(),
            "chunking-tokenizers".to_string(),
        ],
    );
    feature_profiles.insert(
        "wasm".to_string(),
        vec![
            "pdf".to_string(),
            "html".to_string(),
            "archives".to_string(),
            "keywords".to_string(),
            "tree-sitter".to_string(),
        ],
    );
    feature_profiles.insert(
        "ffi".to_string(),
        vec![
            "pdf".to_string(),
            "html".to_string(),
            "archives".to_string(),
            "keywords".to_string(),
            "layout-detection".to_string(),
            "tree-sitter".to_string(),
        ],
    );

    let manifest = ParityManifest {
        version: 1,
        types,
        feature_profiles,
    };

    serde_json::to_value(&manifest).expect("manifest serialization")
}

fn reflect_nested_struct(types: &mut BTreeMap<String, TypeDef>, type_name: &str, json: &Value) {
    types.insert(
        type_name.to_string(),
        TypeDef::Struct {
            fields: reflect_struct_fields(type_name, json),
        },
    );
}

/// Generate the parity manifest and write it to disk.
pub fn generate(output_path: &Utf8Path) -> Result<()> {
    let manifest_value = generate_manifest_value();
    let json_str = serde_json::to_string_pretty(&manifest_value)?;

    // Ensure parent directory exists
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    std::fs::write(output_path, json_str)?;
    Ok(())
}
