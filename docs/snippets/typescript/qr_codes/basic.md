```typescript title="TypeScript"
import { extract } from '@xberg-io/xberg';

const result = await extract({ kind: "uri", uri: "ticket.pdf" }, { qrCodes: true });
for (const image of result.images ?? []) {
    for (const qr of image.qrCodes ?? []) {
        console.log(qr.payload);
    }
}
```
