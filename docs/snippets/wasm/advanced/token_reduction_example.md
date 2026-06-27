```typescript title="WASM - Token Counting and Cost Estimation"
import init, { extract } from "xberg-wasm";

await init();

const config = {
  tokenReduction: {
    mode: "balanced",
    preserveImportantWords: true,
  },
};

const bytes = new Uint8Array(buffer);
const result = await extract(bytes, "application/pdf", config);

// Simple token counting (approximation: 1 token ≈ 4 chars)
function estimateTokenCount(text: string): number {
  return Math.ceil(text.length / 4);
}

// LLM pricing (example: GPT-4 Turbo)
interface PricingEstimate {
  tokenCount: number;
  inputCost: number;
  outputCostEstimate: number;
  totalEstimate: number;
}

const tokenCount = estimateTokenCount(result.content);
const inputPricePerToken = 0.00001; // $0.01/1K tokens
const outputPricePerToken = 0.00003; // $0.03/1K tokens

const costEstimate: PricingEstimate = {
  tokenCount,
  inputCost: tokenCount * inputPricePerToken,
  outputCostEstimate: tokenCount * outputPricePerToken * 0.5, // Assume output is ~50% of input
  totalEstimate: tokenCount * inputPricePerToken + tokenCount * outputPricePerToken * 0.5,
};

console.log("Token and Cost Analysis:");
console.log(`  Estimated tokens: ${costEstimate.tokenCount}`);
console.log(`  Input cost: $${costEstimate.inputCost.toFixed(6)}`);
console.log(`  Output cost (est.): $${costEstimate.outputCostEstimate.toFixed(6)}`);
console.log(`  Total cost (est.): $${costEstimate.totalEstimate.toFixed(6)}`);
```

```typescript title="WASM - Token Reduction for Context Windows"
import init, { extract } from "xberg-wasm";

await init();

interface ContextWindowFit {
  contentLength: number;
  estimatedTokens: number;
  fitsInWindow: boolean;
  utilization: number;
}

function checkContextWindowFit(
  content: string,
  contextWindowSize: number = 4096,
): ContextWindowFit {
  const estimatedTokens = Math.ceil(content.length / 4);
  const fitsInWindow = estimatedTokens < contextWindowSize;
  const utilization = estimatedTokens / contextWindowSize;

  return {
    contentLength: content.length,
    estimatedTokens,
    fitsInWindow,
    utilization,
  };
}

const config = {
  tokenReduction: {
    mode: "aggressive", // Use aggressive mode for large documents
    preserveImportantWords: true,
  },
};

const bytes = new Uint8Array(buffer);
const result = await extract(bytes, "application/pdf", config);

const contextFit = checkContextWindowFit(result.content, 4096);

console.log("Context Window Analysis:");
console.log(`  Content: ${contextFit.contentLength} characters`);
console.log(`  Tokens (est.): ${contextFit.estimatedTokens}`);
console.log(`  Fits in 4K context: ${contextFit.fitsInWindow ? "YES" : "NO"}`);
console.log(`  Utilization: ${(contextFit.utilization * 100).toFixed(1)}%`);

if (!contextFit.fitsInWindow) {
  console.log("  Note: Consider chunking or more aggressive token reduction");
}
```

```typescript title="WASM - Selective Token Preservation"
import init, { extract } from "xberg-wasm";

await init();

const config = {
  tokenReduction: {
    mode: "balanced",
    preserveImportantWords: true,
  },
};

const bytes = new Uint8Array(buffer);
const result = await extract(bytes, "application/pdf", config);

// Extract important terms manually
interface ImportantTerm {
  term: string;
  frequency: number;
  importance: number;
}

function extractImportantTerms(content: string, threshold: number = 3): ImportantTerm[] {
  const words = content.toLowerCase().split(/\s+/);
  const frequencyMap = new Map<string, number>();

  words.forEach((word) => {
    const cleaned = word.replace(/[^\w]/g, "");
    if (cleaned.length > 5) {
      // Only consider longer words
      frequencyMap.set(cleaned, (frequencyMap.get(cleaned) || 0) + 1);
    }
  });

  return Array.from(frequencyMap.entries())
    .filter(([_, freq]) => freq >= threshold)
    .map(([term, freq]) => ({
      term,
      frequency: freq,
      importance: Math.log(freq) * (term.length / 10),
    }))
    .sort((a, b) => b.importance - a.importance)
    .slice(0, 20);
}

const importantTerms = extractImportantTerms(result.content);

console.log("Important Terms (likely preserved by token reduction):");
importantTerms.forEach((t) => {
  console.log(`  "${t.term}": ${t.frequency} occurrences (importance: ${t.importance.toFixed(2)})`);
});
```
