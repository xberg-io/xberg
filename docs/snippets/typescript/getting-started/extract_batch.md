```typescript title="TypeScript"
import { extractBatch } from "@xberg-io/xberg";

const output = await extractBatch([
  { kind: "uri", uri: "document.pdf" },
  {
    kind: "bytes",
    bytes: Buffer.from("Hello from memory"),
    mimeType: "text/plain",
    filename: "note.txt",
  },
]);

for (const result of output.results) {
  console.log(result.content.slice(0, 200));
}
```
