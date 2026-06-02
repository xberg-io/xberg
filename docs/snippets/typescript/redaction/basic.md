```typescript title="TypeScript"
import { extractFile } from '@kreuzberg/node';

const result = await extractFile("contract.pdf", {
    redaction: {
        categories: ["email", "phone", "ssn", "credit_card", "iban"],
        strategy: "mask",
    },
});
console.log(result.content);
console.log(`Redacted ${result.redactionReport?.totalRedacted ?? 0} spans`);
```
