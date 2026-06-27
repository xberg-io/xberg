```typescript title="TypeScript"
import { ExtractInputKind, extractBatch } from "@xberg-io/xberg";

const output = await extractBatch([
  { kind: ExtractInputKind.Uri, uri: "document.pdf" },
  {
    kind: ExtractInputKind.Bytes,
    bytes: Buffer.from("Hello from memory"),
    mimeType: "text/plain",
    filename: "note.txt",
  },
]);

for (const result of output.results) {
  console.log(result.content.slice(0, 200));
}
```
