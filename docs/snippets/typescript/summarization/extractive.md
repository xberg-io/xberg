```typescript title="TypeScript"
import { extract } from '@xberg-io/xberg';

const result = await extract("report.pdf", {
    summarization: {
        strategy: "extractive",
        maxTokens: 200,
    },
});
if (result.summary) {
    console.log(result.summary.text);
}
```
