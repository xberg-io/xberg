```typescript title="WASM - Enable Quality Processing"
import init, { extract } from "xberg-wasm";

await init();

const config = {
  enableQualityProcessing: true,
};

const bytes = new Uint8Array(buffer);
const result = await extract(bytes, "application/pdf", config);

console.log(`Quality score: ${result.qualityScore?.toFixed(3) || "N/A"}`);
console.log(`Content: ${result.content.substring(0, 100)}...`);

// Quality score indicates text extraction quality (0.0-1.0)
if (result.qualityScore && result.qualityScore < 0.5) {
  console.warn("Low quality extraction detected - consider OCR or alternative processing");
}
```

```typescript title="WASM - Quality Monitoring"
import init, { extract } from "xberg-wasm";

await init();

interface ExtractionQuality {
  contentLength: number;
  qualityScore: number | null;
  assessedAs: string;
}

const config = {
  enableQualityProcessing: true,
};

const bytes = new Uint8Array(buffer);
const result = await extract(bytes, "application/pdf", config);

const quality: ExtractionQuality = {
  contentLength: result.content.length,
  qualityScore: result.qualityScore || null,
  assessedAs: result.qualityScore
    ? result.qualityScore > 0.8
      ? "high"
      : result.qualityScore > 0.5
        ? "medium"
        : "low"
    : "unknown",
};

console.log("Extraction Quality Report:");
console.log(`  Content size: ${quality.contentLength} bytes`);
console.log(`  Quality score: ${quality.qualityScore?.toFixed(3) || "N/A"}`);
console.log(`  Assessment: ${quality.assessedAs}`);

if (quality.assessedAs === "low") {
  console.log(
    "  Recommendation: Review raw text for encoding issues or consider alternative extraction",
  );
}
```

```typescript title="WASM - Quality with OCR Fallback"
import init, { extract } from "xberg-wasm";

await init();

async function extractWithQualityCheck(
  bytes: Uint8Array,
  mimeType: string,
): Promise<{ content: string; quality: number | null; method: string }> {
  const config = {
    enableQualityProcessing: true,
  };

  const result = await extract(bytes, mimeType, config);
  const qualityScore = result.qualityScore || 0;

  // If quality is low, consider text extraction failed or use OCR
  if (qualityScore < 0.5) {
    console.warn("Low quality text extraction - alternative processing recommended");
    return {
      content: result.content,
      quality: qualityScore,
      method: "degraded-text-extraction",
    };
  }

  return {
    content: result.content,
    quality: qualityScore,
    method: "text-extraction",
  };
}

const bytes = new Uint8Array(buffer);
const extracted = await extractWithQualityCheck(bytes, "application/pdf");

console.log(`Extraction method: ${extracted.method}`);
console.log(`Quality score: ${extracted.quality?.toFixed(3)}`);
console.log(`Content preview: ${extracted.content.substring(0, 80)}...`);
```
