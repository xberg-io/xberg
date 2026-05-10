```typescript title="TypeScript"
import { extractFile } from "kreuzberg";

async function main() {
  const result = await extractFile("document.pdf");

  console.log(result.content);
  console.log(`Tables: ${result.tables?.length ?? 0}`);
}

main();
```
