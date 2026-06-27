package main

import (
	"xberg"
)

func main() {
	// Example 1: Basic hierarchy extraction
	// Enabled with default KClusters=6 for standard H1-H6 heading hierarchy.
	// Extract bounding box information for spatial layout awareness.
	hierarchyConfigBasic := xberg.HierarchyConfig{
		Enabled:               func(b bool) *bool { return &b }(true),
		KClusters:             func(i int) *int { return &i }(6),  // Default: creates 6 font size clusters (H1-H6 structure)
		IncludeBbox:           func(b bool) *bool { return &b }(true),  // Include bounding box coordinates
		OcrCoverageThreshold:  nil,  // No OCR coverage threshold
	}

	pdfConfigBasic := xberg.PdfConfig{
		Hierarchy: &hierarchyConfigBasic,
	}

	extractionConfigBasic := xberg.ExtractionConfig{
		PdfOptions: &pdfConfigBasic,
	}

	// Use with ExtractSync or ExtractSync
	// result, err := xberg.ExtractSync("document.pdf", extractionConfigBasic)


	// Example 2: Custom KClusters for minimal structure
	// Use 3 clusters for simpler hierarchy with minimal structure.
	// Useful when you only need major section divisions (Main, Subsection, Detail).
	hierarchyConfigMinimal := xberg.HierarchyConfig{
		Enabled:               func(b bool) *bool { return &b }(true),
		KClusters:             func(i int) *int { return &i }(3),  // Minimal clustering: just 3 levels
		IncludeBbox:           func(b bool) *bool { return &b }(true),
		OcrCoverageThreshold:  nil,
	}

	pdfConfigMinimal := xberg.PdfConfig{
		Hierarchy: &hierarchyConfigMinimal,
	}

	extractionConfigMinimal := xberg.ExtractionConfig{
		PdfOptions: &pdfConfigMinimal,
	}

	_ = extractionConfigMinimal


	// Example 3: With OCR coverage threshold
	// Trigger OCR if less than 50% of text has font data.
	// Useful for documents with mixed digital and scanned content.
	ocrThreshold := 0.5
	hierarchyConfigOcr := xberg.HierarchyConfig{
		Enabled:               func(b bool) *bool { return &b }(true),
		KClusters:             func(i int) *int { return &i }(6),
		IncludeBbox:           func(b bool) *bool { return &b }(true),
		OcrCoverageThreshold:  &ocrThreshold,  // Trigger OCR if text coverage < 50%
	}

	pdfConfigOcr := xberg.PdfConfig{
		Hierarchy: &hierarchyConfigOcr,
	}

	extractionConfigOcr := xberg.ExtractionConfig{
		PdfOptions: &pdfConfigOcr,
	}

	_ = extractionConfigOcr
}

// Field descriptions:
//
// Enabled: *bool (default: true)
//   - Enable or disable hierarchy extraction
//   - When false, hierarchy structure is not analyzed
//
// KClusters: *int (default: 6, valid: 1-7)
//   - Number of font size clusters for hierarchy levels
//   - 6 provides H1-H6 heading levels with body text
//   - Higher values create more fine-grained hierarchy
//   - Lower values create simpler structure
//
// IncludeBbox: *bool (default: true)
//   - Include bounding box coordinates in hierarchy blocks
//   - Required for spatial layout awareness and document structure
//   - Set to false only if space optimization is critical
//
// OcrCoverageThreshold: *float64 (default: nil)
//   - Range: 0.0 to 1.0
//   - Triggers OCR when text block coverage falls below this fraction
//   - Example: 0.5 means "run OCR if less than 50% of page has text data"
//   - nil means no OCR coverage-based triggering
