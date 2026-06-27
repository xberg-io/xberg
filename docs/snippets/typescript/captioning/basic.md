```typescript title="TypeScript"
import { extract } from "@xberg-io/xberg";

const result = await extract("report.pdf", {
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
