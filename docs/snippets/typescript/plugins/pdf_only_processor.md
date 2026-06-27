```typescript title="TypeScript"
import { registerPostProcessor, type ExtractedDocument } from "@xberg-io/xberg";

class PdfOnlyProcessor {
  name(): string {
    return "pdf-only-processor";
  }

  processingStage(): "early" | "middle" | "late" {
    return "middle";
  }

  // Gate the processor so it only runs for PDF documents.
  shouldProcess(result: ExtractedDocument): boolean {
    return result.mimeType === "application/pdf";
  }

  process(result: ExtractedDocument): ExtractedDocument {
    return result;
  }
}

registerPostProcessor(new PdfOnlyProcessor());
```
