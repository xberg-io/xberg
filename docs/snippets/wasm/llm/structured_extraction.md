```typescript title="WASM"
import init, { extract } from "xberg-wasm";

await init();

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

const result = await extract("paper.pdf", undefined, config);
console.log(result.structuredOutput);
```

<!-- snippet:syntax-only --> Requires network access to the configured LLM provider and a valid API key in the host environment. The WASM crate accepts `structuredExtraction` configuration; the LLM call is dispatched through liter-llm's `wasm-http` transport.
