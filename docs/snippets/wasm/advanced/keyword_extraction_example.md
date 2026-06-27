```typescript title="WASM - Extract and Score Keywords"
import init, { extract } from "xberg-wasm";

await init();

const config = {
  outputFormat: "markdown",
};

const bytes = new Uint8Array(buffer);
const result = await extract(bytes, "application/pdf", config);

// TF-IDF style keyword extraction (simplified)
interface Keyword {
  term: string;
  frequency: number;
  uniquePositions: number[];
  score: number;
}

const text = result.content.toLowerCase();
const words = text.split(/[\s\n\t]+/);
const tokenMap = new Map<string, number[]>();

// Record word positions
words.forEach((word, idx) => {
  const cleaned = word.replace(/[^\w]/g, "");
  if (cleaned.length > 3) {
    if (!tokenMap.has(cleaned)) {
      tokenMap.set(cleaned, []);
    }
    tokenMap.get(cleaned)!.push(idx);
  }
});

// Calculate keyword scores
const keywords: Keyword[] = Array.from(tokenMap.entries()).map(([term, positions]) => ({
  term,
  frequency: positions.length,
  uniquePositions: positions,
  score: positions.length * Math.log(words.length / positions.length),
}));

// Sort by score (TF-IDF approximation)
keywords.sort((a, b) => b.score - a.score);

// Top 15 keywords
const topKeywords = keywords.slice(0, 15);
console.log("Top Keywords:");
topKeywords.forEach((kw) => {
  console.log(`  ${kw.term}: ${kw.frequency} occurrences (score: ${kw.score.toFixed(2)})`);
});
```

```typescript title="WASM - Keyword Context Window"
import init, { extract } from "xberg-wasm";

await init();

const config = {
  outputFormat: "markdown",
  chunking: {
    maxChars: 1000,
    chunkOverlap: 200,
  },
};

const bytes = new Uint8Array(buffer);
const result = await extract(bytes, "application/pdf", config);

interface KeywordContext {
  keyword: string;
  contexts: string[];
}

// Find keyword occurrences with surrounding context
function extractKeywordContexts(
  text: string,
  keyword: string,
  contextWindow: number = 50,
): string[] {
  const contexts: string[] = [];
  const regex = new RegExp(keyword, "gi");
  let match;

  while ((match = regex.exec(text)) !== null) {
    const start = Math.max(0, match.index - contextWindow);
    const end = Math.min(text.length, match.index + keyword.length + contextWindow);
    contexts.push(text.substring(start, end));
  }

  return contexts;
}

// Extract context for top keywords
const topKeywords = ["document", "analysis", "results"];
const keywordContexts: KeywordContext[] = topKeywords.map((kw) => ({
  keyword: kw,
  contexts: extractKeywordContexts(result.content, kw, 40),
}));

keywordContexts.forEach((kc) => {
  console.log(`\n"${kc.keyword}" appears ${kc.contexts.length} times:`);
  kc.contexts.slice(0, 2).forEach((ctx, idx) => {
    console.log(`  [${idx + 1}] ...${ctx}...`);
  });
});
```

<!-- snippet:syntax-only --> - Keyword extraction without native YAKE/RAKE requires manual text processing.
