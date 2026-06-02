```typescript title="TypeScript"
import { extractFile } from '@kreuzberg/node';

const result = await extractFile("contract.pdf", {
    translation: {
        targetLang: "de",
        llm: { model: "openai/gpt-4o-mini" },
    },
});
if (result.translation) {
    console.log(result.translation.content);
}
```
