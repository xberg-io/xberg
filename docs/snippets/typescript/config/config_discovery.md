# Configuration Discovery Example

Use `ExtractionConfig.discover()` to automatically find and load configuration files from the current directory or parent directories:

```typescript title="config_discovery.ts"
import { ExtractionConfig, extract } from "@xberg-io/xberg";

const config = ExtractionConfig.discover();
if (config) {
  console.log("Found configuration file");
  const result = await extract("document.pdf", null, config);
  console.log(result.content);
} else {
  console.log("No configuration file found, using defaults");
  const result = await extract("document.pdf");
  console.log(result.content);
}
```

The discovery method looks for `xberg.toml`, `xberg.yaml`, or `xberg.json` files starting in the current directory and searching parent directories up to the filesystem root.
