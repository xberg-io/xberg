```typescript title="WASM - Keyword Extraction Setup"
import init, { extract } from "xberg-wasm";

await init();

// Note: Keyword extraction requires the 'keywords' feature,
// which may not be available in all WASM builds.
// This example shows the configuration structure.

const config = {
  // Extraction configuration
  outputFormat: "markdown",
};

const bytes = new Uint8Array(buffer);
const result = await extract(bytes, "application/pdf", config);

// Keyword extraction would be performed on the extracted text
// using external libraries or post-processing
console.log(`Extracted text: ${result.content.substring(0, 100)}...`);

// Example post-processing to extract keywords
// (requires external keyword extraction library)
const keywords = new Set<string>();
const words = result.content
  .toLowerCase()
  .split(/\s+/)
  .filter((w) => w.length > 4); // Simple heuristic: words > 4 chars

words.forEach((word) => {
  keywords.add(word);
});

console.log(`Extracted keywords: ${Array.from(keywords).slice(0, 10).join(", ")}`);
```

```typescript title="WASM - Keyword Filtering"
import init, { extract } from "xberg-wasm";

await init();

const config = {
  outputFormat: "markdown",
};

const bytes = new Uint8Array(buffer);
const result = await extract(bytes, "application/pdf", config);

// Common stopwords to exclude
const stopwords = new Set([
  "the",
  "a",
  "an",
  "and",
  "or",
  "but",
  "in",
  "on",
  "at",
  "to",
  "for",
  "of",
  "with",
  "from",
  "by",
  "is",
  "are",
  "was",
  "were",
  "be",
  "been",
  "have",
  "has",
  "had",
  "do",
  "does",
  "did",
  "will",
  "would",
  "could",
  "should",
]);

// Extract and filter keywords
const text = result.content.toLowerCase();
const words = text.split(/\s+/);
const keywordCounts = new Map<string, number>();

words.forEach((word) => {
  const cleaned = word.replace(/[^\w]/g, "");
  if (cleaned.length > 4 && !stopwords.has(cleaned)) {
    keywordCounts.set(cleaned, (keywordCounts.get(cleaned) || 0) + 1);
  }
});

// Get top keywords by frequency
const topKeywords = Array.from(keywordCounts.entries())
  .sort((a, b) => b[1] - a[1])
  .slice(0, 10)
  .map(([word, count]) => `${word} (${count})`);

console.log(`Top keywords: ${topKeywords.join(", ")}`);
```

<!-- snippet:syntax-only --> - Native keyword extraction requires the `keywords` feature which may not be compiled into WASM builds.
