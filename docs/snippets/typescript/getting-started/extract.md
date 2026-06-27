```typescript title="TypeScript"
import { ExtractInputKind, extract } from "@xberg-io/xberg";

const output = await extract({
  kind: ExtractInputKind.Uri,
  uri: "document.pdf",
});

console.log(output.results[0].content);
console.log(`Results: ${output.summary.results}`);
```
