```typescript title="TypeScript"
import { extract } from '@xberg-io/xberg';

const result = await extract("contract.pdf", {
    redaction: {
        categories: ["email", "phone", "ssn", "credit_card", "iban"],
        strategy: "mask",
    },
});
console.log(result.content);
console.log(`Redacted ${result.redactionReport?.totalRedacted ?? 0} spans`);
```
