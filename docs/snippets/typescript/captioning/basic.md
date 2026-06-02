```typescript title="TypeScript"
import { extractFile } from "@kreuzberg/node";

const result = await extractFile("report.pdf", {
    captioning: {
        llm: { model: "openai/gpt-4o-mini" },
    },
});

for (const image of result.images ?? []) {
    if (image.caption) {
        console.log(image.caption);
    }
}
```
