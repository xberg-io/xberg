```typescript title="TypeScript"
import {
  registerPostProcessor,
  type ExtractionResult,
} from "@kreuzberg/node";

class PdfMetadataExtractor {
  private processedCount: number = 0;

  name(): string {
    return "pdf-metadata-extractor";
  }

  processingStage(): "early" | "middle" | "late" {
    return "early";
  }

  shouldProcess(result: ExtractionResult): boolean {
    return result.mimeType === "application/pdf";
  }

  process(result: ExtractionResult): ExtractionResult {
    this.processedCount += 1;

    return {
      ...result,
      metadata: {
        ...result.metadata,
        pdfProcessingIndex: this.processedCount,
        pdfMetadataEnriched: true,
      },
    };
  }

  getStats(): { processedCount: number } {
    return { processedCount: this.processedCount };
  }
}

registerPostProcessor(new PdfMetadataExtractor());
```
