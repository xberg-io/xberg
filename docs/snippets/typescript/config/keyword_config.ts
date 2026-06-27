import {
  type ExtractionConfig,
  extract,
  type RakeParams,
  type YakeParams,
} from "@xberg-io/xberg";

// Example 1: Basic YAKE configuration
// Uses YAKE algorithm with default parameters and English stopword filtering
async function basicYake(): Promise<void> {
  const config: ExtractionConfig = {
    keywords: {
      algorithm: "yake",
      maxKeywords: 10,
      minScore: 0.0,
      language: "en",
      yakeParams: null,
      rakeParams: null,
    },
  };

  const output = await extract({ kind: "uri", uri: "document.pdf" }, config);
  const result = output.results![0];
  console.log("Keywords:", result.extractedKeywords ?? []);
}

// Example 2: Advanced YAKE with custom parameters
// Fine-tunes YAKE with custom window size for co-occurrence analysis
async function _advancedYake(): Promise<void> {
  const config: ExtractionConfig = {
    keywords: {
      algorithm: "yake",
      maxKeywords: 15,
      minScore: 0.1,
      language: "en",
      yakeParams: {
        windowSize: 1,
      } as YakeParams,
      rakeParams: null,
    },
  };

  const output = await extract({ kind: "uri", uri: "document.pdf" }, config);
  const result = output.results![0];
  console.log("Keywords:", result.extractedKeywords ?? []);
}

// Example 3: RAKE configuration
// Uses RAKE algorithm for rapid keyword extraction with phrase constraints
async function _rakeConfig(): Promise<void> {
  const config: ExtractionConfig = {
    keywords: {
      algorithm: "rake",
      maxKeywords: 10,
      minScore: 5.0,
      language: "en",
      yakeParams: null,
      rakeParams: {
        minWordLength: 1,
        maxWordsPerPhrase: 3,
      } as RakeParams,
    },
  };

  const output = await extract({ kind: "uri", uri: "document.pdf" }, config);
  const result = output.results![0];
  console.log("Keywords:", result.extractedKeywords ?? []);
}

basicYake();
