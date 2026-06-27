```typescript title="TypeScript"
import { ExtractInputKind, extract } from "@xberg-io/xberg";

const config = {
  ocr: {
    backend: "tesseract",
    language: ["eng", "fra"],
    tesseractConfig: {
      psm: 3,
    },
  },
};

const output = await extract(
  {
    kind: "uri",
    uri: "document.pdf",
  },
  config,
);

console.log(output.results[0].content);
```
