```typescript title="TypeScript"
import { extract } from '@xberg-io/xberg';

const result = await extract("contract.pdf", {
    ner: {
        backend: "llm",
        llm: { model: "openai/gpt-4o-mini" },
    },
});

for (const entity of result.entities ?? []) {
    console.log(`${entity.category}: ${entity.text}`);
}
```
