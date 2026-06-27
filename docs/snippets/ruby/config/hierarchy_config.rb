require 'xberg'

# Example 1: Basic hierarchy extraction
# Enabled with default k_clusters=6 for standard H1-H6 heading hierarchy.
# Extract bounding box information for spatial layout awareness.
hierarchy_config_basic = Xberg::HierarchyConfig.new(
  enabled: true,
  k_clusters: 6,  # Default: creates 6 font size clusters (H1-H6 structure)
  include_bbox: true,  # Include bounding box coordinates
  ocr_coverage_threshold: nil  # No OCR coverage threshold
)

pdf_config_basic = Xberg::PdfConfig.new(
  hierarchy: hierarchy_config_basic
)

extraction_config_basic = Xberg::ExtractionConfig.new(
  pdf_options: pdf_config_basic
)

# input = Xberg::ExtractInput.new(uri: "document.pdf")
# result = Xberg.extract(input, extraction_config_basic)


# Example 2: Custom k_clusters for minimal structure
# Use 3 clusters for simpler hierarchy with minimal structure.
# Useful when you only need major section divisions (Main, Subsection, Detail).
hierarchy_config_minimal = Xberg::HierarchyConfig.new(
  enabled: true,
  k_clusters: 3,  # Minimal clustering: just 3 levels
  include_bbox: true,
  ocr_coverage_threshold: nil
)

pdf_config_minimal = Xberg::PdfConfig.new(
  hierarchy: hierarchy_config_minimal
)

extraction_config_minimal = Xberg::ExtractionConfig.new(
  pdf_options: pdf_config_minimal
)

# input = Xberg::ExtractInput.new(uri: "document.pdf")
# result = Xberg.extract(input, extraction_config_minimal)


# Example 3: With OCR coverage threshold
# Trigger OCR if less than 50% of text has font data.
# Useful for documents with mixed digital and scanned content.
hierarchy_config_ocr = Xberg::HierarchyConfig.new(
  enabled: true,
  k_clusters: 6,
  include_bbox: true,
  ocr_coverage_threshold: 0.5  # Trigger OCR if text coverage < 50%
)

pdf_config_ocr = Xberg::PdfConfig.new(
  hierarchy: hierarchy_config_ocr
)

extraction_config_ocr = Xberg::ExtractionConfig.new(
  pdf_options: pdf_config_ocr
)

# input = Xberg::ExtractInput.new(uri: "document.pdf")
# result = Xberg.extract(input, extraction_config_ocr)


# Field descriptions:
#
# enabled: boolean (default: true)
#   - Enable or disable hierarchy extraction
#   - When false, hierarchy structure is not analyzed
#
# k_clusters: integer (default: 6, valid: 1-7)
#   - Number of font size clusters for hierarchy levels
#   - 6 provides H1-H6 heading levels with body text
#   - Higher values create more fine-grained hierarchy
#   - Lower values create simpler structure
#
# include_bbox: boolean (default: true)
#   - Include bounding box coordinates in hierarchy blocks
#   - Required for spatial layout awareness and document structure
#   - Set to false only if space optimization is critical
#
# ocr_coverage_threshold: float | nil (default: nil)
#   - Range: 0.0 to 1.0
#   - Triggers OCR when text block coverage falls below this fraction
#   - Example: 0.5 means "run OCR if less than 50% of page has text data"
#   - nil means no OCR coverage-based triggering
