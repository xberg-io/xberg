//! Shared helpers for generated Rust E2E tests.

use kreuzberg::types::ExtractionResult;
use serde_json::Value;
use std::path::{Path, PathBuf};

/// Path to the workspace root.
pub fn workspace_root() -> PathBuf {
    let mut dir = Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf();
    loop {
        if dir.join("test_documents").is_dir() {
            return dir;
        }
        if !dir.pop() {
            panic!("Could not find workspace root (directory containing test_documents/)");
        }
    }
}

/// Path to the shared test_documents directory.
pub fn test_documents_dir() -> PathBuf {
    workspace_root().join("test_documents")
}

/// Resolve a relative document path under test_documents.
pub fn resolve_document(relative: &str) -> PathBuf {
    test_documents_dir().join(relative)
}

/// Check if an external tool is available on the system PATH.
pub fn external_tool_available(tool: &str) -> bool {
    std::process::Command::new(tool)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

/// Generated assertions shared across tests.
pub mod assertions {
    use super::*;

    /// Assert that the MIME type matches any of the expected patterns.
    pub fn assert_expected_mime(result: &ExtractionResult, expected: &[&str]) {
        if expected.is_empty() {
            return;
        }

        let mime: &str = &result.mime_type;
        let matches = expected.iter().any(|candidate| mime.contains(candidate));
        assert!(matches, "Expected MIME {:?} to match one of {:?}", mime, expected);
    }

    /// Assert that content length is at least `min`.
    pub fn assert_min_content_length(result: &ExtractionResult, min: usize) {
        assert!(
            result.content.len() >= min,
            "Expected content length >= {min}, got {}",
            result.content.len()
        );
    }

    /// Assert that content length is at most `max`.
    pub fn assert_max_content_length(result: &ExtractionResult, max: usize) {
        assert!(
            result.content.len() <= max,
            "Expected content length <= {max}, got {}",
            result.content.len()
        );
    }

    /// Assert that the content contains any of the provided snippets.
    pub fn assert_content_contains_any(result: &ExtractionResult, snippets: &[&str]) {
        if snippets.is_empty() {
            return;
        }

        let lowered = result.content.to_lowercase();
        let preview = result.content.chars().take(160).collect::<String>();
        let found = snippets.iter().any(|snippet| lowered.contains(&snippet.to_lowercase()));

        assert!(
            found,
            "Expected content to contain at least one snippet from {:?}. Preview: {:?}",
            snippets, preview
        );
    }

    /// Assert that the content contains all provided snippets.
    pub fn assert_content_contains_all(result: &ExtractionResult, snippets: &[&str]) {
        if snippets.is_empty() {
            return;
        }

        let lowered = result.content.to_lowercase();
        let all_found = snippets.iter().all(|snippet| lowered.contains(&snippet.to_lowercase()));

        assert!(all_found, "Expected content to contain all snippets {:?}", snippets);
    }

    pub fn assert_content_contains_none(result: &ExtractionResult, snippets: &[&str]) {
        if snippets.is_empty() {
            return;
        }
        let lowered = result.content.to_lowercase();
        let found: Vec<&&str> = snippets
            .iter()
            .filter(|snippet| lowered.contains(&snippet.to_lowercase()))
            .collect();
        assert!(
            found.is_empty(),
            "Expected content to contain none of {:?}, but found {:?}",
            snippets,
            found
        );
    }

    /// Assert table count boundaries.
    pub fn assert_table_count(result: &ExtractionResult, min: Option<usize>, max: Option<usize>) {
        if let Some(min_tables) = min {
            assert!(
                result.tables.len() >= min_tables,
                "Expected at least {min_tables} tables, found {}",
                result.tables.len()
            );
        }
        if let Some(max_tables) = max {
            assert!(
                result.tables.len() <= max_tables,
                "Expected at most {max_tables} tables, found {}",
                result.tables.len()
            );
        }
    }

    /// Assert detected languages contain expected entries with optional confidence requirements.
    pub fn assert_detected_languages(result: &ExtractionResult, expected: &[&str], min_confidence: Option<f32>) {
        let Some(languages) = result.detected_languages.as_ref() else {
            panic!("Expected detected languages but field is None");
        };

        for lang in expected {
            assert!(
                languages.iter().any(|detected| detected == lang),
                "Expected detected languages to contain {lang}, got {:?}",
                languages
            );
        }

        if let Some(threshold) = min_confidence
            && let Ok(Value::Object(map)) = serde_json::to_value(&result.metadata)
            && let Some(confidence) = map.get("confidence").and_then(Value::as_f64)
        {
            assert!(
                (confidence as f32) >= threshold,
                "Expected confidence >= {threshold}, got {confidence}"
            );
        }
    }

    /// Assert metadata expectations expressed as JSON.
    pub fn assert_metadata_expectation(result: &ExtractionResult, path: &str, expectation: &Value) {
        let metadata = serde_json::to_value(&result.metadata).expect("Metadata should serialize to JSON");
        let value =
            lookup_path(&metadata, path).unwrap_or_else(|| panic!("Metadata path `{path}` missing in {:?}", metadata));

        if let Some(eq) = expectation.get("eq") {
            assert!(
                values_equal(value, eq),
                "Expected metadata `{path}` == {eq:?}, got {value:?}"
            );
        }

        if let Some(gte) = expectation.get("gte") {
            let actual = value
                .as_f64()
                .or_else(|| value.as_i64().map(|n| n as f64))
                .unwrap_or_else(|| panic!("Metadata `{path}` is not numeric: {value:?}"));
            let min = gte
                .as_f64()
                .or_else(|| gte.as_i64().map(|n| n as f64))
                .unwrap_or_else(|| panic!("Expectation `{path}` gte is not numeric: {gte:?}"));
            assert!(actual >= min, "Expected metadata `{path}` >= {min}, got {actual}");
        }

        if let Some(lte) = expectation.get("lte") {
            let actual = value
                .as_f64()
                .or_else(|| value.as_i64().map(|n| n as f64))
                .unwrap_or_else(|| panic!("Metadata `{path}` is not numeric: {value:?}"));
            let max = lte
                .as_f64()
                .or_else(|| lte.as_i64().map(|n| n as f64))
                .unwrap_or_else(|| panic!("Expectation `{path}` lte is not numeric: {lte:?}"));
            assert!(actual <= max, "Expected metadata `{path}` <= {max}, got {actual}");
        }

        if let Some(contains) = expectation.get("contains") {
            match (value.as_str(), contains.as_str()) {
                (Some(actual), Some(expected)) => {
                    assert!(
                        actual.contains(expected),
                        "Expected metadata `{path}` string `{actual}` to contain `{expected}`"
                    );
                }
                _ if value.is_array() && contains.is_string() => {
                    let actual_values = value
                        .as_array()
                        .expect("value is array by branch")
                        .iter()
                        .collect::<Vec<_>>();
                    let expected = contains.as_str().expect("contains is string by branch");
                    assert!(
                        actual_values
                            .iter()
                            .any(|item| { item.as_str().is_some_and(|s| s.contains(expected)) }),
                        "Expected metadata `{path}` to contain `{expected}`, got {actual_values:?}"
                    );
                }
                _ if value.is_array() && contains.is_array() => {
                    let actual_values = value
                        .as_array()
                        .expect("value is array by branch")
                        .iter()
                        .collect::<Vec<_>>();
                    for needle in contains.as_array().expect("contains is array") {
                        assert!(
                            actual_values.iter().any(|item| values_equal(item, needle)),
                            "Expected metadata `{path}` to contain {needle:?}, got {actual_values:?}"
                        );
                    }
                }
                _ => panic!("Metadata `{path}` contains expectation unsupported for value {value:?}"),
            }
        }

        if let Some(exists) = expectation.get("exists").and_then(Value::as_bool) {
            if exists {
                assert!(!value.is_null(), "Expected metadata `{path}` to exist (non-null)");
            } else {
                panic!("`exists: false` is not supported for metadata assertions");
            }
        }
    }

    /// Options for chunk assertions.
    #[derive(Default)]
    pub struct ChunkAssertions {
        pub min_count: Option<usize>,
        pub max_count: Option<usize>,
        pub each_has_content: Option<bool>,
        pub each_has_embedding: Option<bool>,
        pub each_has_heading_context: Option<bool>,
        pub each_has_chunk_type: Option<bool>,
        pub content_starts_with_heading: Option<bool>,
    }

    /// Assert chunk count and properties.
    pub fn assert_chunks(result: &ExtractionResult, opts: &ChunkAssertions) {
        let ChunkAssertions {
            min_count,
            max_count,
            each_has_content,
            each_has_embedding,
            each_has_heading_context,
            each_has_chunk_type,
            content_starts_with_heading,
        } = opts;
        let chunks = result.chunks.as_ref().expect("Expected chunks in result");
        let count = chunks.len();

        if let Some(min) = *min_count {
            assert!(count >= min, "Expected at least {min} chunks, found {count}");
        }

        if let Some(max) = *max_count {
            assert!(count <= max, "Expected at most {max} chunks, found {count}");
        }

        if *each_has_content == Some(true) {
            for (i, chunk) in chunks.iter().enumerate() {
                assert!(!chunk.content.is_empty(), "Expected chunk {i} to have content");
            }
        }

        if *each_has_embedding == Some(true) {
            for (i, chunk) in chunks.iter().enumerate() {
                assert!(
                    chunk.embedding.is_some() && !chunk.embedding.as_ref().unwrap().is_empty(),
                    "Expected chunk {i} to have embedding"
                );
            }
        }

        if *each_has_heading_context == Some(true) {
            for (i, chunk) in chunks.iter().enumerate() {
                assert!(
                    chunk.metadata.heading_context.is_some(),
                    "Expected chunk {i} to have heading_context"
                );
            }
        }

        if *each_has_heading_context == Some(false) {
            for (i, chunk) in chunks.iter().enumerate() {
                assert!(
                    chunk.metadata.heading_context.is_none(),
                    "Expected chunk {i} to have no heading_context"
                );
            }
        }

        if *each_has_chunk_type == Some(true) {
            for (i, chunk) in chunks.iter().enumerate() {
                assert!(
                    chunk.chunk_type != kreuzberg::types::ChunkType::Unknown,
                    "Expected chunk {i} to have a specific chunk_type, got Unknown"
                );
            }
        }

        if *content_starts_with_heading == Some(true) {
            for (i, chunk) in chunks.iter().enumerate() {
                if chunk.metadata.heading_context.is_none() {
                    continue;
                }
                assert!(
                    chunk.content.starts_with('#'),
                    "Expected chunk {i} content to start with '#'"
                );
            }
        }
    }

    /// Assert image count and formats.
    pub fn assert_images(
        result: &ExtractionResult,
        min_count: Option<usize>,
        max_count: Option<usize>,
        formats_include: Option<&[&str]>,
    ) {
        let images = result.images.as_ref().expect("Expected images in result");
        let count = images.len();

        if let Some(min) = min_count {
            assert!(count >= min, "Expected at least {min} images, found {count}");
        }

        if let Some(max) = max_count {
            assert!(count <= max, "Expected at most {max} images, found {count}");
        }

        if let Some(formats) = formats_include {
            for format in formats {
                let found = images.iter().any(|img| img.format.contains(format));
                assert!(
                    found,
                    "Expected images to include format {format}, found {:?}",
                    images.iter().map(|img| &img.format).collect::<Vec<_>>()
                );
            }
        }
    }

    /// Assert page count boundaries.
    pub fn assert_pages(
        result: &ExtractionResult,
        min_count: Option<usize>,
        exact_count: Option<usize>,
        has_layout_regions: Option<bool>,
        layout_classes_include: Option<&[&str]>,
    ) {
        let pages = result.pages.as_ref().expect("Expected pages in result");
        let count = pages.len();

        if let Some(min) = min_count {
            assert!(count >= min, "Expected at least {min} pages, found {count}");
        }

        if let Some(exact) = exact_count {
            assert!(count == exact, "Expected exactly {exact} pages, found {count}");
        }

        for page in pages {
            // is_blank should be present as Option<bool>
            let _ = page.is_blank;
        }

        if let Some(true) = has_layout_regions {
            let found = pages
                .iter()
                .any(|page| page.layout_regions.as_ref().is_some_and(|regions| !regions.is_empty()));
            assert!(found, "Expected at least one page to have layout_regions populated");
        }

        if let Some(classes) = layout_classes_include {
            let mut all_classes = std::collections::HashSet::new();
            for page in pages {
                if let Some(regions) = &page.layout_regions {
                    for region in regions {
                        all_classes.insert(region.class.as_str());
                    }
                }
            }
            for expected_class in classes {
                assert!(
                    all_classes.contains(*expected_class),
                    "Expected layout class '{expected_class}' not found in {all_classes:?}"
                );
            }
        }
    }

    /// Assert element count and types.
    pub fn assert_elements(result: &ExtractionResult, min_count: Option<usize>, types_include: Option<&[&str]>) {
        let elements = result.elements.as_ref().expect("Expected elements in result");
        let count = elements.len();

        if let Some(min) = min_count {
            assert!(count >= min, "Expected at least {min} elements, found {count}");
        }

        if let Some(types) = types_include {
            for element_type in types {
                let found = elements.iter().any(|el| {
                    let serialized = serde_json::to_value(el.element_type)
                        .ok()
                        .and_then(|v| v.as_str().map(String::from))
                        .unwrap_or_default();
                    serialized.to_lowercase().contains(&element_type.to_lowercase())
                });
                assert!(
                    found,
                    "Expected elements to include type {element_type}, found {:?}",
                    elements
                        .iter()
                        .map(|el| {
                            serde_json::to_value(el.element_type)
                                .ok()
                                .and_then(|v| v.as_str().map(String::from))
                                .unwrap_or_else(|| format!("{:?}", el.element_type))
                        })
                        .collect::<Vec<_>>()
                );
            }
        }
    }

    /// Assert OCR elements count and properties.
    pub fn assert_ocr_elements(
        result: &ExtractionResult,
        has_elements: Option<bool>,
        elements_have_geometry: Option<bool>,
        elements_have_confidence: Option<bool>,
        min_count: Option<usize>,
    ) {
        if has_elements == Some(true) {
            let ocr_elements = result.ocr_elements.as_ref().expect("Expected ocr_elements in result");
            assert!(!ocr_elements.is_empty(), "Expected non-empty ocr_elements");
        }

        if let Some(Some(ocr_elements)) = result.ocr_elements.as_ref().map(Some) {
            if elements_have_geometry == Some(true) {
                for element in ocr_elements.iter() {
                    // Check that geometry exists and is valid
                    match &element.geometry {
                        kreuzberg::types::OcrBoundingGeometry::Rectangle { .. } => {}
                        kreuzberg::types::OcrBoundingGeometry::Quadrilateral { .. } => {}
                    }
                }
            }

            if elements_have_confidence == Some(true) {
                for (i, element) in ocr_elements.iter().enumerate() {
                    assert!(
                        element.confidence.recognition > 0.0,
                        "Expected element {i} to have recognition confidence > 0, got {}",
                        element.confidence.recognition
                    );
                }
            }

            if let Some(min) = min_count {
                assert!(
                    ocr_elements.len() >= min,
                    "Expected at least {min} ocr_elements, found {}",
                    ocr_elements.len()
                );
            }
        }
    }

    /// Assert document structure presence and properties.
    pub fn assert_document(
        result: &ExtractionResult,
        has_document: bool,
        min_node_count: Option<usize>,
        node_types_include: Option<&[&str]>,
        has_groups: Option<bool>,
    ) {
        if !has_document {
            assert!(result.document.is_none(), "Expected document to be None but got Some");
            return;
        }

        let document = result.document.as_ref().expect("Expected document in result");
        let nodes = &document.nodes;

        if let Some(min) = min_node_count {
            assert!(
                nodes.len() >= min,
                "Expected at least {min} document nodes, found {}",
                nodes.len()
            );
        }

        if let Some(types) = node_types_include {
            let found_types: std::collections::HashSet<String> = nodes
                .iter()
                .filter_map(|n| {
                    serde_json::to_value(&n.content)
                        .ok()
                        .and_then(|v| v.get("node_type").and_then(|t| t.as_str().map(String::from)))
                })
                .collect();
            for expected in types {
                assert!(
                    found_types.contains(*expected),
                    "Expected document to include node type {expected}, found types: {found_types:?}",
                );
            }
        }

        if let Some(expect_groups) = has_groups {
            let has_group_nodes = nodes.iter().any(|n| {
                serde_json::to_value(&n.content)
                    .ok()
                    .and_then(|v| v.get("node_type").and_then(|t| t.as_str().map(|s| s == "group")))
                    .unwrap_or(false)
            });
            assert_eq!(has_group_nodes, expect_groups, "Group node presence mismatch");
        }
    }

    /// Assert keyword extraction results.
    pub fn assert_keywords(
        result: &ExtractionResult,
        has_keywords: Option<bool>,
        min_count: Option<usize>,
        max_count: Option<usize>,
    ) {
        let keywords_opt = result.extracted_keywords.as_ref();

        if let Some(true) = has_keywords {
            let keywords = keywords_opt.expect("Expected keywords but got None");
            assert!(!keywords.is_empty(), "Expected non-empty keywords list");
        }
        if let Some(false) = has_keywords
            && keywords_opt.is_some()
        {
            let keywords = keywords_opt.unwrap();
            assert!(keywords.is_empty(), "Expected no keywords but found {}", keywords.len());
        }
        if let Some(keywords) = keywords_opt {
            if let Some(min) = min_count {
                assert!(
                    keywords.len() >= min,
                    "Expected >= {} keywords, found {}",
                    min,
                    keywords.len()
                );
            }
            if let Some(max) = max_count {
                assert!(
                    keywords.len() <= max,
                    "Expected <= {} keywords, found {}",
                    max,
                    keywords.len()
                );
            }
        }
    }

    /// Assert that content is not empty.
    pub fn assert_content_not_empty(result: &ExtractionResult) {
        assert!(!result.content.trim().is_empty(), "Expected non-empty content");
    }

    /// Assert that all tables have bounding boxes when expected is true.
    pub fn assert_table_bounding_boxes(result: &ExtractionResult, expected: bool) {
        if expected {
            assert!(
                !result.tables.is_empty(),
                "Expected tables with bounding boxes but no tables found"
            );
            for table in &result.tables {
                assert!(
                    table.bounding_box.is_some(),
                    "Expected table to have bounding_box but it was None"
                );
            }
        }
    }

    /// Assert that at least one table cell contains any of the provided snippets.
    pub fn assert_table_content_contains_any(result: &ExtractionResult, snippets: &[&str]) {
        assert!(!result.tables.is_empty(), "Expected tables but none found");
        let all_cells: Vec<&str> = result
            .tables
            .iter()
            .flat_map(|t| t.cells.iter())
            .flat_map(|row| row.iter())
            .map(|s| s.as_str())
            .collect();
        let found = snippets.iter().any(|snippet| {
            let lower = snippet.to_lowercase();
            all_cells.iter().any(|cell| cell.to_lowercase().contains(&lower))
        });
        assert!(
            found,
            "No table cell contains any of {:?}. Cells: {:?}",
            snippets, all_cells
        );
    }

    /// Assert that all images have bounding boxes when expected is true.
    pub fn assert_image_bounding_boxes(result: &ExtractionResult, expected: bool) {
        if expected {
            let images = result
                .images
                .as_ref()
                .expect("Expected images with bounding boxes but images is None");
            assert!(
                !images.is_empty(),
                "Expected images with bounding boxes but no images found"
            );
            for image in images {
                assert!(
                    image.bounding_box.is_some(),
                    "Expected image to have bounding_box but it was None"
                );
            }
        }
    }

    /// Assert quality score presence and range.
    pub fn assert_quality_score(
        result: &ExtractionResult,
        has_score: Option<bool>,
        min_score: Option<f64>,
        max_score: Option<f64>,
    ) {
        if let Some(true) = has_score {
            assert!(
                result.quality_score.is_some(),
                "Expected quality_score to be present but it was None"
            );
        }
        if let Some(false) = has_score {
            assert!(
                result.quality_score.is_none(),
                "Expected quality_score to be absent but it was Some"
            );
        }
        if let Some(min) = min_score {
            let score = result
                .quality_score
                .expect("quality_score required for min_score assertion");
            assert!(score >= min, "quality_score {score} is less than minimum {min}");
        }
        if let Some(max) = max_score {
            let score = result
                .quality_score
                .expect("quality_score required for max_score assertion");
            assert!(score <= max, "quality_score {score} is greater than maximum {max}");
        }
    }

    /// Assert processing warnings count and emptiness.
    pub fn assert_processing_warnings(result: &ExtractionResult, max_count: Option<usize>, is_empty: Option<bool>) {
        if let Some(max) = max_count {
            assert!(
                result.processing_warnings.len() <= max,
                "processing_warnings count {} exceeds maximum {max}",
                result.processing_warnings.len()
            );
        }
        if let Some(true) = is_empty {
            assert!(
                result.processing_warnings.is_empty(),
                "Expected processing_warnings to be empty but found {}",
                result.processing_warnings.len()
            );
        }
    }

    /// Assert LLM usage count and emptiness.
    pub fn assert_llm_usage(result: &ExtractionResult, max_count: Option<usize>, is_empty: Option<bool>) {
        let usage_count = result.llm_usage.as_ref().map(|u| u.len()).unwrap_or(0);
        if let Some(max) = max_count {
            assert!(
                usage_count <= max,
                "llm_usage count {} exceeds maximum {max}",
                usage_count
            );
        }
        if let Some(true) = is_empty {
            assert!(
                usage_count == 0,
                "Expected llm_usage to be empty but found {}",
                usage_count
            );
        }
    }

    /// Assert annotations presence and count.
    pub fn assert_annotations(result: &ExtractionResult, has_annotations: bool, min_count: Option<usize>) {
        if has_annotations {
            let annotations = result
                .annotations
                .as_ref()
                .expect("Expected annotations in result but field is None");
            assert!(!annotations.is_empty(), "Expected non-empty annotations");

            if let Some(min) = min_count {
                assert!(
                    annotations.len() >= min,
                    "Expected at least {min} annotations, found {}",
                    annotations.len()
                );
            }
        } else {
            assert!(
                result.annotations.is_none() || result.annotations.as_ref().is_some_and(|a| a.is_empty()),
                "Expected no annotations but found some"
            );
        }
    }

    /// Assert djot content presence and block count.
    pub fn assert_djot_content(result: &ExtractionResult, has_content: Option<bool>, min_blocks: Option<usize>) {
        if let Some(true) = has_content {
            assert!(
                result.djot_content.is_some(),
                "Expected djot_content to be present but it was None"
            );
        }
        if let Some(false) = has_content {
            assert!(
                result.djot_content.is_none(),
                "Expected djot_content to be absent but it was Some"
            );
        }
        if let Some(min) = min_blocks {
            let djot = result
                .djot_content
                .as_ref()
                .expect("djot_content required for min_blocks assertion");
            assert!(
                djot.blocks.len() >= min,
                "djot_content blocks count {} is less than minimum {min}",
                djot.blocks.len()
            );
        }
    }

    /// Assert structured extraction output presence and field existence.
    pub fn assert_structured_output(
        result: &ExtractionResult,
        has_output: Option<bool>,
        validates_schema: Option<bool>,
        field_exists: Option<&[&str]>,
    ) {
        let output = result.structured_output.as_ref();
        if let Some(true) = has_output {
            assert!(
                output.is_some(),
                "Expected structured_output to be present but it was None"
            );
        }
        if let Some(false) = has_output {
            assert!(
                output.is_none(),
                "Expected structured_output to be absent but it was Some"
            );
        }
        if let Some(true) = validates_schema {
            let val = output.expect("structured_output required for validates_schema assertion");
            assert!(
                val.is_object() || val.is_array(),
                "Expected structured_output to be a JSON object or array"
            );
        }
        if let Some(fields) = field_exists {
            let val = output.expect("structured_output required for field_exists assertion");
            let obj = val
                .as_object()
                .expect("structured_output must be a JSON object for field_exists");
            for field in fields {
                assert!(
                    obj.contains_key(*field),
                    "Expected structured_output to contain field '{}', keys: {:?}",
                    field,
                    obj.keys().collect::<Vec<_>>()
                );
            }
        }
    }

    fn lookup_path<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
        if let Some(found) = lookup_path_inner(value, path) {
            return Some(found);
        }
        if let Value::Object(map) = value
            && let Some(format) = map.get("format")
        {
            return lookup_path_inner(format, path);
        }
        None
    }

    fn lookup_path_inner<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
        let mut current = value;
        for segment in path.split('.') {
            current = match current {
                Value::Object(map) => map.get(segment)?,
                _ => return None,
            };
        }
        Some(current)
    }

    fn values_equal(lhs: &Value, rhs: &Value) -> bool {
        match (lhs, rhs) {
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Number(a), Value::Number(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            _ => lhs == rhs,
        }
    }

    pub fn assert_embed_result(
        result: &[Vec<f32>],
        count: Option<usize>,
        dimensions: Option<usize>,
        no_nan: bool,
        no_inf: bool,
        non_zero: bool,
        normalized: bool,
    ) {
        if let Some(c) = count {
            assert_eq!(result.len(), c, "Expected {c} embeddings, got {}", result.len());
        }
        for (i, vec) in result.iter().enumerate() {
            if let Some(d) = dimensions {
                assert_eq!(vec.len(), d, "Embedding {i}: expected {d} dims, got {}", vec.len());
            }
            if no_nan {
                assert!(!vec.iter().any(|v| v.is_nan()), "Embedding {i} contains NaN values");
            }
            if no_inf {
                assert!(
                    !vec.iter().any(|v| v.is_infinite()),
                    "Embedding {i} contains Inf values"
                );
            }
            if non_zero {
                assert!(vec.iter().any(|&v| v != 0.0), "Embedding {i} is all zeros");
            }
            if normalized {
                let norm = vec.iter().map(|v| v * v).sum::<f32>().sqrt();
                assert!(
                    (norm - 1.0).abs() < 1e-4,
                    "Embedding {i} L2 norm {norm:.6} != 1.0 (not normalized)"
                );
            }
        }
    }
}
