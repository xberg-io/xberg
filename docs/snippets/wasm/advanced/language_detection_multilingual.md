```typescript title="WASM - Detect and Process Multilingual Content"
import init, { extract } from "xberg-wasm";

await init();

const config = {
  languageDetection: {
    enabled: true,
    minConfidence: 0.7,
    detectMultiple: true,
  },
  chunking: {
    maxChars: 800,
    chunkOverlap: 200,
  },
};

const bytes = new Uint8Array(buffer);
const result = await extract(bytes, "application/pdf", config);

interface MultilingualChunk {
  index: number;
  text: string;
  language: string;
  confidence: number;
}

// Detect language for each chunk
const multilingualChunks: MultilingualChunk[] =
  result.chunks?.map((chunk, idx) => {
    // Simple language detection based on character ranges
    const text = chunk.content;
    let detectedLang = result.metadata?.language || "en";
    let confidence = result.metadata?.languageConfidence || 0.5;

    // Check for specific character patterns
    if (/[一-鿿]/.test(text)) detectedLang = "zh"; // Chinese
    if (/[぀-ゟ゠-ヿ]/.test(text)) detectedLang = "ja"; // Japanese
    if (/[가-힯]/.test(text)) detectedLang = "ko"; // Korean
    if (/[؀-ۿ]/.test(text)) detectedLang = "ar"; // Arabic
    if (/[Ѐ-ӿ]/.test(text)) detectedLang = "ru"; // Russian

    return {
      index: idx,
      text: text.substring(0, 50),
      language: detectedLang,
      confidence: confidence,
    };
  }) || [];

// Group chunks by language
const chunksByLanguage = new Map<string, MultilingualChunk[]>();
multilingualChunks.forEach((chunk) => {
  if (!chunksByLanguage.has(chunk.language)) {
    chunksByLanguage.set(chunk.language, []);
  }
  chunksByLanguage.get(chunk.language)!.push(chunk);
});

console.log("Chunks by detected language:");
chunksByLanguage.forEach((chunks, lang) => {
  console.log(`  ${lang}: ${chunks.length} chunks`);
  chunks.slice(0, 2).forEach((c) => {
    console.log(`    Chunk ${c.index}: "${c.text}..."`);
  });
});
```

```typescript title="WASM - Language-Specific Text Processing"
import init, { extract } from "xberg-wasm";

await init();

const config = {
  languageDetection: {
    enabled: true,
    detectMultiple: true,
  },
  outputFormat: "markdown",
};

const bytes = new Uint8Array(buffer);
const result = await extract(bytes, "text/plain", config);

// Language-specific text normalization
interface LanguageProcessor {
  normalize: (text: string) => string;
  splitSentences: (text: string) => string[];
}

const processors: Record<string, LanguageProcessor> = {
  en: {
    normalize: (t) => t.replace(/\s+/g, " ").trim(),
    splitSentences: (t) => t.split(/[.!?]+/).filter((s) => s.length > 0),
  },
  zh: {
    normalize: (t) => t.replace(/\s+/g, "").trim(), // CJK: no word spacing
    splitSentences: (t) => t.split(/[。！？]+/).filter((s) => s.length > 0),
  },
  ja: {
    normalize: (t) => t.replace(/\s+/g, "").trim(),
    splitSentences: (t) => t.split(/[。！？]+/).filter((s) => s.length > 0),
  },
  ar: {
    normalize: (t) => t.replace(/\s+/g, " ").trim(),
    splitSentences: (t) => t.split(/[.!?،؟]+/).filter((s) => s.length > 0),
  },
};

const language = result.metadata?.language || "en";
const processor = processors[language] || processors["en"];

const normalized = processor.normalize(result.content);
const sentences = processor.splitSentences(result.content);

console.log(`Language: ${language}`);
console.log(`Normalized length: ${normalized.length}`);
console.log(`Detected sentences: ${sentences.length}`);
sentences.slice(0, 3).forEach((sent, idx) => {
  console.log(`  [${idx + 1}] ${sent.substring(0, 60)}...`);
});
```

```typescript title="WASM - Multilingual Chunking Strategy"
import init, { extract } from "xberg-wasm";

await init();

interface LanguageChunkConfig {
  maxChars: number;
  overlap: number;
}

// Different chunking strategies per language
const chunkConfigByLanguage: Record<string, LanguageChunkConfig> = {
  en: { maxChars: 512, overlap: 128 }, // English: word-based chunking
  zh: { maxChars: 256, overlap: 64 }, // Chinese: smaller chunks due to character density
  ja: { maxChars: 300, overlap: 75 }, // Japanese: medium chunks
  ar: { maxChars: 400, overlap: 100 }, // Arabic: larger chunks for context
  default: { maxChars: 512, overlap: 128 },
};

// Detect language first
const languageDetectConfig = {
  languageDetection: {
    enabled: true,
    minConfidence: 0.8,
  },
};

const bytes = new Uint8Array(buffer);
const langResult = await extract(bytes, "text/plain", languageDetectConfig);
const detectedLang = langResult.metadata?.language || "en";

// Re-extract with language-specific chunking
const chunkConfig = chunkConfigByLanguage[detectedLang] || chunkConfigByLanguage["default"];
const finalConfig = {
  languageDetection: {
    enabled: true,
  },
  chunking: {
    maxChars: chunkConfig.maxChars,
    chunkOverlap: chunkConfig.overlap,
  },
};

const finalResult = await extract(bytes, "text/plain", finalConfig);
console.log(`Language: ${detectedLang}`);
console.log(`Chunking strategy: maxChars=${chunkConfig.maxChars}, overlap=${chunkConfig.overlap}`);
console.log(`Generated ${finalResult.chunks?.length} chunks`);
```
