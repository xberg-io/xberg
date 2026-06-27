import { type ExtractionConfig, extract } from "@xberg-io/xberg-wasm";

// Example 1: Basic YAKE configuration
// Uses YAKE algorithm with default parameters and English stopword filtering
async function basicYake(): Promise<void> {
  const config: ExtractionConfig = {
    keywords: {
      algorithm: "yake",
      maxKeywords: 10,
      minScore: 0.0,
      ngramRange: [1, 3],
      language: "en",
      yakeParams: null,
      rakeParams: null,
    },
  };

  const result = await extract("document.pdf", null, config);
  console.log("Keywords:", result.keywords);
}

// Example 2: Advanced YAKE with custom parameters
// Fine-tunes YAKE with custom window size for co-occurrence analysis
async function _advancedYake(): Promise<void> {
  const config: ExtractionConfig = {
    keywords: {
      algorithm: "yake",
      maxKeywords: 15,
      minScore: 0.1,
      ngramRange: [1, 2],
      language: "en",
      yakeParams: {
        windowSize: 1,
      },
      rakeParams: null,
    },
  };

  const result = await extract("document.pdf", null, config);
  console.log("Keywords:", result.keywords);
}

// Example 3: RAKE configuration
// Uses RAKE algorithm for rapid keyword extraction with phrase constraints
async function _rakeConfig(): Promise<void> {
  const config: ExtractionConfig = {
    keywords: {
      algorithm: "rake",
      maxKeywords: 10,
      minScore: 5.0,
      ngramRange: [1, 3],
      language: "en",
      yakeParams: null,
      rakeParams: {
        minWordLength: 1,
        maxWordsPerPhrase: 3,
      },
    },
  };

  const result = await extract("document.pdf", null, config);
  console.log("Keywords:", result.keywords);
}

basicYake();
