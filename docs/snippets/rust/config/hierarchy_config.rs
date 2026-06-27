use xberg::{ExtractionConfig, HierarchyConfig, PdfConfig};

// Example 1: Basic hierarchy extraction
// Enabled with default k_clusters=6 for standard H1-H6 heading hierarchy.
// Extract bounding box information for spatial layout awareness.
let hierarchy_config_basic = HierarchyConfig {
    enabled: true,
    k_clusters: 6,  // Default: creates 6 font size clusters (H1-H6 structure)
    include_bbox: true,  // Include bounding box coordinates
    ocr_coverage_threshold: None,  // No OCR coverage threshold
};

let pdf_config_basic = PdfConfig {
    hierarchy: Some(hierarchy_config_basic),
    ..Default::default()
};

let extraction_config_basic = ExtractionConfig {
    pdf_options: Some(pdf_config_basic),
    ..Default::default()
};

// Use with extract_sync or extract_sync
// let result = extract_sync("document.pdf", extraction_config_basic)?;


// Example 2: Custom k_clusters for minimal structure
// Use 3 clusters for simpler hierarchy with minimal structure.
// Useful when you only need major section divisions (Main, Subsection, Detail).
let hierarchy_config_minimal = HierarchyConfig {
    enabled: true,
    k_clusters: 3,  // Minimal clustering: just 3 levels
    include_bbox: true,
    ocr_coverage_threshold: None,
};

let pdf_config_minimal = PdfConfig {
    hierarchy: Some(hierarchy_config_minimal),
    ..Default::default()
};

let extraction_config_minimal = ExtractionConfig {
    pdf_options: Some(pdf_config_minimal),
    ..Default::default()
};


// Example 3: With OCR coverage threshold
// Trigger OCR if less than 50% of text has font data.
// Useful for documents with mixed digital and scanned content.
let hierarchy_config_ocr = HierarchyConfig {
    enabled: true,
    k_clusters: 6,
    include_bbox: true,
    ocr_coverage_threshold: Some(0.5),  // Trigger OCR if text coverage < 50%
};

let pdf_config_ocr = PdfConfig {
    hierarchy: Some(hierarchy_config_ocr),
    ..Default::default()
};

let extraction_config_ocr = ExtractionConfig {
    pdf_options: Some(pdf_config_ocr),
    ..Default::default()
};


// Field descriptions:
//
// enabled: bool (default: true)
//   - Enable or disable hierarchy extraction
//   - When false, hierarchy structure is not analyzed
//
// k_clusters: usize (default: 6, valid: 1-7)
//   - Number of font size clusters for hierarchy levels
//   - 6 provides H1-H6 heading levels with body text
//   - Higher values create more fine-grained hierarchy
//   - Lower values create simpler structure
//
// include_bbox: bool (default: true)
//   - Include bounding box coordinates in hierarchy blocks
//   - Required for spatial layout awareness and document structure
//   - Set to false only if space optimization is critical
//
// ocr_coverage_threshold: Option<f32> (default: None)
//   - Range: 0.0 to 1.0
//   - Triggers OCR when text block coverage falls below this fraction
//   - Example: Some(0.5) means "run OCR if less than 50% of page has text data"
//   - None means no OCR coverage-based triggering
