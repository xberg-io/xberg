```typescript title="TypeScript"
import { extractFileSync } from "@kreuzberg/node";

const config = {
  structuredExtraction: {
    schema: {
      type: "object",
      properties: {
        title: { type: "string" },
        authors: { type: "array", items: { type: "string" } },
        date: { type: "string" },
      },
      required: ["title", "authors", "date"],
      additionalProperties: false,
    },
    llm: {
      model: "openai/gpt-4o-mini",
    },
    strict: true,
  },
};

const result = extractFileSync("paper.pdf", null, config);
console.log(result.structuredOutput);
```
