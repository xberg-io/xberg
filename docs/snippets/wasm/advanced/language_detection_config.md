```typescript title="WASM - Basic Language Detection"
import init, { extract } from "xberg-wasm";

await init();

const config = {
  languageDetection: {
    enabled: true,
    minConfidence: 0.75,
  },
};

const bytes = new Uint8Array(buffer);
const result = await extract(bytes, "application/pdf", config);

console.log(`Primary language: ${result.metadata?.language}`);
console.log(`Language confidence: ${result.metadata?.languageConfidence}`);
console.log(`Detected languages: ${result.detectedLanguages?.join(", ")}`);
```

```typescript title="WASM - Multi-Language Detection"
import init, { extract } from "xberg-wasm";

await init();

const config = {
  languageDetection: {
    enabled: true,
    minConfidence: 0.6,
    detectMultiple: true, // Enable detection of multiple languages
  },
};

const bytes = new Uint8Array(buffer);
const result = await extract(bytes, "application/pdf", config);

console.log(`Primary language: ${result.metadata?.language}`);
console.log(`Confidence score: ${result.metadata?.languageConfidence?.toFixed(3)}`);
console.log(`All detected languages: ${result.detectedLanguages?.join(", ")}`);

// Use detected language for downstream processing
if (result.detectedLanguages && result.detectedLanguages.length > 1) {
  console.log("Document contains multiple languages - enable multilingual NLP processing");
}
```

```typescript title="WASM - Language-Specific Extraction"
import init, { extract } from "xberg-wasm";

await init();

const config = {
  languageDetection: {
    enabled: true,
    minConfidence: 0.8,
    detectMultiple: false,
  },
  // Adjust extraction parameters based on detected language
  quality: {
    enableQualityProcessing: true,
  },
};

const bytes = new Uint8Array(buffer);
const result = await extract(bytes, "text/plain", config);

// Process result based on detected language
const language = result.metadata?.language;
console.log(`Processing document in language: ${language}`);

// Example: Apply language-specific rules
const languageConfig: Record<string, { cleanWhitespace: boolean; normalizeText: boolean }> = {
  en: { cleanWhitespace: true, normalizeText: true },
  zh: { cleanWhitespace: false, normalizeText: true }, // Chinese: preserve whitespace patterns
  ja: { cleanWhitespace: false, normalizeText: false }, // Japanese: preserve as-is
  ar: { cleanWhitespace: true, normalizeText: true }, // Arabic
};

const langConfig = languageConfig[language as string] || {
  cleanWhitespace: true,
  normalizeText: true,
};
console.log(`Language config: ${JSON.stringify(langConfig)}`);
```
