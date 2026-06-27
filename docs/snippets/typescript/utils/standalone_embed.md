```typescript title="TypeScript"
import { embed, embedSync } from "@xberg-io/xberg";
import type { EmbeddingConfig } from "@xberg-io/xberg";

const config: EmbeddingConfig = {
  model: { type: "preset", name: "balanced" },
  normalize: true,
};

// Synchronous
const embeddings = embedSync(["Hello, world!", "Xberg is fast"], config);
console.log(embeddings.length); // 2
console.log(embeddings[0].length); // 768

// Asynchronous (preferred)
const asyncEmbeddings = await embed(["Hello, world!", "Xberg is fast"], config);
console.log(asyncEmbeddings[0].length); // 768
```
