import xberg.config.HierarchyConfig;
import xberg.config.PdfConfig;
import xberg.config.ExtractionConfig;
import xberg.Xberg;

public class HierarchyConfigExample {
    public static void main(String[] args) throws Exception {
        // Example 1: Basic hierarchy extraction
        // Enabled with default kClusters=6 for standard H1-H6 heading hierarchy.
        // Extract bounding box information for spatial layout awareness.
        HierarchyConfig hierarchyConfigBasic = HierarchyConfig.builder()
            .enabled(true)
            .kClusters(6)  // Default: creates 6 font size clusters (H1-H6 structure)
            .includeBbox(true)  // Include bounding box coordinates
            .ocrCoverageThreshold(null)  // No OCR coverage threshold
            .build();

        PdfConfig pdfConfigBasic = PdfConfig.builder()
            .hierarchy(hierarchyConfigBasic)
            .build();

        ExtractionConfig extractionConfigBasic = ExtractionConfig.builder()
            .pdfOptions(pdfConfigBasic)
            .build();

        Xberg xberg = new Xberg(extractionConfigBasic);
        // var result = xberg.extractSync("document.pdf");


        // Example 2: Custom kClusters for minimal structure
        // Use 3 clusters for simpler hierarchy with minimal structure.
        // Useful when you only need major section divisions (Main, Subsection, Detail).
        HierarchyConfig hierarchyConfigMinimal = HierarchyConfig.builder()
            .enabled(true)
            .kClusters(3)  // Minimal clustering: just 3 levels
            .includeBbox(true)
            .ocrCoverageThreshold(null)
            .build();

        PdfConfig pdfConfigMinimal = PdfConfig.builder()
            .hierarchy(hierarchyConfigMinimal)
            .build();

        ExtractionConfig extractionConfigMinimal = ExtractionConfig.builder()
            .pdfOptions(pdfConfigMinimal)
            .build();


        // Example 3: With OCR coverage threshold
        // Trigger OCR if less than 50% of text has font data.
        // Useful for documents with mixed digital and scanned content.
        HierarchyConfig hierarchyConfigOcr = HierarchyConfig.builder()
            .enabled(true)
            .kClusters(6)
            .includeBbox(true)
            .ocrCoverageThreshold(0.5f)  // Trigger OCR if text coverage < 50%
            .build();

        PdfConfig pdfConfigOcr = PdfConfig.builder()
            .hierarchy(hierarchyConfigOcr)
            .build();

        ExtractionConfig extractionConfigOcr = ExtractionConfig.builder()
            .pdfOptions(pdfConfigOcr)
            .build();
    }
}

// Field descriptions:
//
// enabled: boolean (default: true)
//   - Enable or disable hierarchy extraction
//   - When false, hierarchy structure is not analyzed
//
// kClusters: int (default: 6, valid: 1-7)
//   - Number of font size clusters for hierarchy levels
//   - 6 provides H1-H6 heading levels with body text
//   - Higher values create more fine-grained hierarchy
//   - Lower values create simpler structure
//
// includeBbox: boolean (default: true)
//   - Include bounding box coordinates in hierarchy blocks
//   - Required for spatial layout awareness and document structure
//   - Set to false only if space optimization is critical
//
// ocrCoverageThreshold: Float (default: null)
//   - Range: 0.0 to 1.0
//   - Triggers OCR when text block coverage falls below this fraction
//   - Example: 0.5f means "run OCR if less than 50% of page has text data"
//   - null means no OCR coverage-based triggering
