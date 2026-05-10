```typescript title="TypeScript"
import { extractBytes } from "kreuzberg";
import { readFileSync } from "fs";

async function main() {
  const content = readFileSync("document.pdf");
  const result = await extractBytes(content, "application/pdf");

  console.log(result.content);
  console.log(`Tables: ${result.tables?.length ?? 0}`);
}

main();
```
