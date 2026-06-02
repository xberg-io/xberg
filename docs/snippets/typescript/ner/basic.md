```typescript title="TypeScript"
import { extractFile } from '@kreuzberg/node';

const result = await extractFile("contract.pdf", {
    ner: {
        backend: "llm",
        llm: { model: "openai/gpt-4o-mini" },
    },
});

for (const entity of result.entities ?? []) {
    console.log(`${entity.category}: ${entity.text}`);
}
```
