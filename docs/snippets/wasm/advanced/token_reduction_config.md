```typescript title="WASM - Token Reduction Configuration"
import init, { extract } from "xberg-wasm";

await init();

const config = {
  tokenReduction: {
    mode: "aggressive",
    preserveImportantWords: true,
  },
};

const bytes = new Uint8Array(buffer);
const result = await extract(bytes, "application/pdf", config);

console.log(`Original content: ${result.content.length} characters`);
console.log(`Preview: ${result.content.substring(0, 100)}...`);

// Token reduction modes:
// - "aggressive": maximum reduction
// - "balanced": moderate reduction
// - "conservative": minimal reduction
```

```typescript title="WASM - Token Reduction Modes"
import init, { extract } from "xberg-wasm";

await init();

interface TokenReductionResult {
  mode: string;
  originalSize: number;
  reducedSize: number;
  reductionRatio: number;
  preview: string;
}

async function compareTokenReductionModes(bytes: Uint8Array): Promise<TokenReductionResult[]> {
  const modes = ["conservative", "balanced", "aggressive"];
  const results: TokenReductionResult[] = [];

  for (const mode of modes) {
    const config = {
      tokenReduction: {
        mode,
        preserveImportantWords: true,
      },
    };

    const result = await extract(bytes, "application/pdf", config);
    const originalSize = result.content.length;
    const reducedSize = result.content.split(/\s+/).length;

    results.push({
      mode,
      originalSize,
      reducedSize,
      reductionRatio: 1 - reducedSize / originalSize,
      preview: result.content.substring(0, 80),
    });
  }

  return results;
}

const bytes = new Uint8Array(buffer);
const modeComparison = await compareTokenReductionModes(bytes);

console.log("Token Reduction Mode Comparison:");
modeComparison.forEach((r) => {
  console.log(`  ${r.mode}:`);
  console.log(`    Original: ${r.originalSize} chars`);
  console.log(`    Reduction: ${(r.reductionRatio * 100).toFixed(1)}%`);
});
```
