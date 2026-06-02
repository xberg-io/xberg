```typescript title="TypeScript"
import { extractFile } from '@kreuzberg/node';

const result = await extractFile("report.pdf", {
    summarization: {
        strategy: "extractive",
        maxTokens: 200,
    },
});
if (result.summary) {
    console.log(result.summary.text);
}
```
