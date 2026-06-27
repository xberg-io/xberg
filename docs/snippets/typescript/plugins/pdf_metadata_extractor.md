```typescript title="TypeScript"
import { registerPostProcessor, type ExtractedDocument } from "@xberg-io/xberg";

class PdfMetadataExtractor {
  private processedCount: number = 0;

  name(): string {
    return "pdf-metadata-extractor";
  }

  processingStage(): "early" | "middle" | "late" {
    return "early";
  }

  shouldProcess(result: ExtractedDocument): boolean {
    return result.mimeType === "application/pdf";
  }

  process(result: ExtractedDocument): ExtractedDocument {
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
