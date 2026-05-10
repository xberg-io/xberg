# Word Count Post-Processor

Register a post-processor that computes word count and other text statistics.

```typescript title="WASM"
import init, { registerPostProcessor, extractBytes } from "kreuzberg-wasm";

await init();

// Define a word count post-processor
const wordCountProcessor = {
  processingStage: () => "post-extraction",
  process: (extractionResult) => {
    const text = extractionResult.text || "";
    
    // Compute statistics
    const words = text.trim().split(/\s+/).filter(w => w.length > 0);
    const lines = text.split(/\n/).filter(l => l.trim().length > 0);
    const paragraphs = text.split(/\n{2,}/).filter(p => p.trim().length > 0);
    const sentences = text.split(/[.!?]+/).filter(s => s.trim().length > 0);
    
    // Calculate reading time (average 200 words per minute)
    const readingTimeMinutes = Math.ceil(words.length / 200);
    
    // Compute character statistics
    const chars = text.length;
    const charsNoSpaces = text.replace(/\s/g, "").length;
    
    // Enrich metadata with text statistics
    const enriched = {
      ...extractionResult,
      metadata: {
        ...extractionResult.metadata,
        statistics: {
          wordCount: words.length,
          lineCount: lines.length,
          paragraphCount: paragraphs.length,
          sentenceCount: sentences.length,
          charCount: chars,
          charsNoSpaces: charsNoSpaces,
          averageWordLength: words.length > 0 ? Math.round(charsNoSpaces / words.length) : 0,
          averageLineLength: lines.length > 0 ? Math.round(words.length / lines.length) : 0,
          readingTimeMinutes: readingTimeMinutes
        }
      }
    };
    
    return enriched;
  }
};

try {
  registerPostProcessor(wordCountProcessor);
  console.log("Word count post-processor registered");
} catch (error) {
  console.error("Failed to register post-processor:", error);
}

// Extract with word counting
async function extractAndAnalyze(fileBytes, mimeType) {
  const result = await extractBytes(fileBytes, mimeType, {});
  const stats = result.metadata?.statistics;
  
  console.log("Text Analysis:", {
    words: stats?.wordCount,
    lines: stats?.lineCount,
    paragraphs: stats?.paragraphCount,
    sentences: stats?.sentenceCount,
    readingTime: `${stats?.readingTimeMinutes} min`
  });
  
  return result;
}

const pdfBytes = new Uint8Array([/* PDF content */]);
await extractAndAnalyze(pdfBytes, "application/pdf");
```

This processor analyzes text and provides readability metrics.
