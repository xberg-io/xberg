```typescript title="TypeScript"
import { extract } from "@xberg-io/xberg";

const config = {
  keywords: {
    algorithm: "yake",
    maxKeywords: 10,
    minScore: 0.3,
  },
};

const result = await extract("research_paper.pdf", null, config);

for (const keyword of result.extractedKeywords ?? []) {
  console.log(`${keyword.text}: ${keyword.score.toFixed(3)}`);
}
```
