```typescript title="TypeScript"
import { extractFile } from '@kreuzberg/node';

const result = await extractFile("packet.pdf", {
    pageClassification: {
        labels: ["invoice", "contract", "id_document", "receipt"],
        llm: { model: "openai/gpt-4o-mini" },
    },
});

for (const page of result.pageClassifications ?? []) {
    console.log(`page ${page.pageNumber}: ${page.labels[0]?.label}`);
}
```
