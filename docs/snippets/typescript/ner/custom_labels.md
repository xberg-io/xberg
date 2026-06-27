```typescript title="TypeScript"
const result = await extract("contract.pdf", {
    ner: {
        backend: "llm",
        llm: { model: "openai/gpt-4o-mini" },
        customLabels: ["Treatment", "Vessel", "Product"],
    },
});
```
