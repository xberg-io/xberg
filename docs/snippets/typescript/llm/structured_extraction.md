```typescript title="TypeScript"
import { extract } from "@xberg-io/xberg";

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

const result = extract({ kind: "uri", uri: "paper.pdf" }, config);
console.log(result.structuredOutput);
```
