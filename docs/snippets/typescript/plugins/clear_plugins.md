```typescript title="TypeScript"
import {
  clearOcrBackends,
  clearPostProcessors,
  clearValidators,
} from "@kreuzberg/node";

clearOcrBackends();
clearPostProcessors();
clearValidators();

console.log("All plugins cleared");
```
