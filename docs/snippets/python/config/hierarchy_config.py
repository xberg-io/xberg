from xberg import ExtractInput, PdfConfig, HierarchyConfig, ExtractionConfig, extract

# Example 1: Basic hierarchy extraction
# Enabled with default k_clusters=6 for standard H1-H6 heading hierarchy.
# Extract bounding box information for spatial layout awareness.
hierarchy_config_basic = HierarchyConfig(
    enabled=True,
    k_clusters=6,  # Default: creates 6 font size clusters (H1-H6 structure)
    include_bbox=True,  # Include bounding box coordinates
    ocr_coverage_threshold=None  # No OCR coverage threshold
)

pdf_config_basic = PdfConfig(hierarchy=hierarchy_config_basic)
extraction_config_basic = ExtractionConfig(pdf_options=pdf_config_basic)

result = extract(ExtractInput.from_uri("document.pdf"), extraction_config_basic)


# Example 2: Custom k_clusters for minimal structure
# Use 3 clusters for simpler hierarchy with minimal structure.
# Useful when you only need major section divisions (Main, Subsection, Detail).
hierarchy_config_minimal = HierarchyConfig(
    enabled=True,
    k_clusters=3,  # Minimal clustering: just 3 levels
    include_bbox=True,
    ocr_coverage_threshold=None
)

pdf_config_minimal = PdfConfig(hierarchy=hierarchy_config_minimal)
extraction_config_minimal = ExtractionConfig(pdf_options=pdf_config_minimal)

result = extract(ExtractInput.from_uri("document.pdf"), extraction_config_minimal)


# Example 3: With OCR coverage threshold
# Trigger OCR if less than 50% of text has font data.
# Useful for documents with mixed digital and scanned content.
hierarchy_config_ocr = HierarchyConfig(
    enabled=True,
    k_clusters=6,
    include_bbox=True,
    ocr_coverage_threshold=0.5  # Trigger OCR if text coverage < 50%
)

pdf_config_ocr = PdfConfig(hierarchy=hierarchy_config_ocr)
extraction_config_ocr = ExtractionConfig(pdf_options=pdf_config_ocr)

result = extract(ExtractInput.from_uri("document.pdf"), extraction_config_ocr)


# Field descriptions:
#
# enabled: bool (default: True)
#   - Enable or disable hierarchy extraction
#   - When False, hierarchy structure is not analyzed
#
# k_clusters: int (default: 6, valid: 1-7)
#   - Number of font size clusters for hierarchy levels
#   - 6 provides H1-H6 heading levels with body text
#   - Higher values create more fine-grained hierarchy
#   - Lower values create simpler structure
#
# include_bbox: bool (default: True)
#   - Include bounding box coordinates in hierarchy blocks
#   - Required for spatial layout awareness and document structure
#   - Set to False only if space optimization is critical
#
# ocr_coverage_threshold: float | None (default: None)
#   - Range: 0.0 to 1.0
#   - Triggers OCR when text block coverage falls below this fraction
#   - Example: 0.5 means "run OCR if less than 50% of page has text data"
#   - None means no OCR coverage-based triggering
