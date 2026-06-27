```typescript title="TypeScript"
import { extract } from "@xberg-io/xberg";

const config = {
  chunking: {
    maxChars: 1000,
    maxOverlap: 200,
  },
};

const result = await extract("document.pdf", null, config);
console.log(`Total chunks: ${result.chunks?.length ?? 0}`);
```

```typescript title="TypeScript - Semantic"
import { extract } from "@xberg-io/xberg";

const config = {
  chunking: {
    chunkerType: "semantic",
  },
};

const result = await extract("document.pdf", null, config);
for (const chunk of result.chunks ?? []) {
  console.log(`Content: ${chunk.content.slice(0, 100)}...`);
}
```

```typescript title="TypeScript - Prepend Heading Context"
import { extract } from "@xberg-io/xberg";

const config = {
  chunking: {
    chunkerType: "markdown",
    maxChars: 500,
    maxOverlap: 50,
    prependHeadingContext: true,
  },
};

const result = await extract("document.md", null, config);
for (const chunk of result.chunks ?? []) {
  // Each chunk's content is prefixed with its heading breadcrumb
  console.log(`Content: ${chunk.content.slice(0, 100)}...`);
}
```
