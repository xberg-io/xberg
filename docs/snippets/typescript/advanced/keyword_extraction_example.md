```typescript title="TypeScript"
import { extract } from "@xberg-io/xberg";

const config = {
  keywords: {
    algorithm: "yake",
    maxKeywords: 10,
    minScore: 0.3,
  },
};

const output = await extract({ kind: "uri", uri: "research_paper.pdf" }, config);
const result = output.results![0];

for (const keyword of result.extractedKeywords ?? []) {
  console.log(`${keyword.text}: ${keyword.score.toFixed(3)}`);
}
```
