```typescript title="TypeScript"
import { extract } from "xberg";

const result = await extract("document.pdf", {
  outputFormat: "html",
  htmlOutput: {
    theme: "github",
    embedCss: true,
  },
});
console.log(result.content); // HTML with kb-* classes
```
