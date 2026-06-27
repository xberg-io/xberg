using Xberg.Config;
using Xberg;

public class HierarchyConfigExample
{
    public static void Main()
    {
        // Example 1: Basic hierarchy extraction
        // Enabled with default KClusters=6 for standard H1-H6 heading hierarchy.
        // Extract bounding box information for spatial layout awareness.
        var hierarchyConfigBasic = new HierarchyConfig
        {
            Enabled = true,
            KClusters = 6,  // Default: creates 6 font size clusters (H1-H6 structure)
            IncludeBbox = true,  // Include bounding box coordinates
            OcrCoverageThreshold = null  // No OCR coverage threshold
        };

        var pdfConfigBasic = new PdfConfig
        {
            Hierarchy = hierarchyConfigBasic
        };

        var extractionConfigBasic = new ExtractionConfig
        {
            PdfOptions = pdfConfigBasic
        };

        var xberg = new Xberg(extractionConfigBasic);
        // var result = xberg.ExtractSync("document.pdf");


        // Example 2: Custom KClusters for minimal structure
        // Use 3 clusters for simpler hierarchy with minimal structure.
        // Useful when you only need major section divisions (Main, Subsection, Detail).
        var hierarchyConfigMinimal = new HierarchyConfig
        {
            Enabled = true,
            KClusters = 3,  // Minimal clustering: just 3 levels
            IncludeBbox = true,
            OcrCoverageThreshold = null
        };

        var pdfConfigMinimal = new PdfConfig
        {
            Hierarchy = hierarchyConfigMinimal
        };

        var extractionConfigMinimal = new ExtractionConfig
        {
            PdfOptions = pdfConfigMinimal
        };


        // Example 3: With OCR coverage threshold
        // Trigger OCR if less than 50% of text has font data.
        // Useful for documents with mixed digital and scanned content.
        var hierarchyConfigOcr = new HierarchyConfig
        {
            Enabled = true,
            KClusters = 6,
            IncludeBbox = true,
            OcrCoverageThreshold = 0.5f  // Trigger OCR if text coverage < 50%
        };

        var pdfConfigOcr = new PdfConfig
        {
            Hierarchy = hierarchyConfigOcr
        };

        var extractionConfigOcr = new ExtractionConfig
        {
            PdfOptions = pdfConfigOcr
        };
    }
}

// Field descriptions:
//
// Enabled: bool (default: true)
//   - Enable or disable hierarchy extraction
//   - When false, hierarchy structure is not analyzed
//
// KClusters: int (default: 6, valid: 1-7)
//   - Number of font size clusters for hierarchy levels
//   - 6 provides H1-H6 heading levels with body text
//   - Higher values create more fine-grained hierarchy
//   - Lower values create simpler structure
//
// IncludeBbox: bool (default: true)
//   - Include bounding box coordinates in hierarchy blocks
//   - Required for spatial layout awareness and document structure
//   - Set to false only if space optimization is critical
//
// OcrCoverageThreshold: float? (default: null)
//   - Range: 0.0 to 1.0
//   - Triggers OCR when text block coverage falls below this fraction
//   - Example: 0.5f means "run OCR if less than 50% of page has text data"
//   - null means no OCR coverage-based triggering
