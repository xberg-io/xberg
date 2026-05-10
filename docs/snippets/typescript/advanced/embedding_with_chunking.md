```typescript title="TypeScript"
const config = {
  chunking: {
    maxChars: 1024,
    maxOverlap: 100,
    embedding: {
      model: { type: "preset", name: "balanced" },
      normalize: true,
      batchSize: 32,
      showDownloadProgress: false,
    },
  },
};
```
