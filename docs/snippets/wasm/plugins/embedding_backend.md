# Register Custom Embedding Backend

Register a custom embedding backend that provides vector embeddings for text.

```typescript title="WASM"
import init, { registerEmbeddingBackend } from "kreuzberg-wasm";

await init();

// Define a custom embedding backend
const customEmbedding = {
  dimensions: () => 384,
  embed: (texts) => {
    // Return embeddings for each text
    return texts.map(text => {
      // Generate a dummy 384-dimensional vector
      const vector = new Array(384).fill(0).map((_, i) => 
        Math.sin((text.charCodeAt(0) + i) / 384)
      );
      return vector;
    });
  }
};

try {
  registerEmbeddingBackend(customEmbedding);
  console.log("Custom embedding backend registered");
} catch (error) {
  console.error("Failed to register embedding backend:", error);
}
```

The embedding backend must implement:
- `dimensions()`: Returns the dimensionality of the embeddings
- `embed(texts: string[])`: Computes vector embeddings for the given texts
