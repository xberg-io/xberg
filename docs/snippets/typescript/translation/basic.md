```typescript title="TypeScript"
import { extract } from '@xberg-io/xberg';

const result = await extract("contract.pdf", {
    translation: {
        targetLang: "de",
        llm: { model: "openai/gpt-4o-mini" },
    },
});
if (result.translation) {
    console.log(result.translation.content);
}
```
