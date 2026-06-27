# Configuration Discovery Example

Use `ExtractionConfig.discover()` to automatically find and load configuration files from the current directory or parent directories:

```typescript title="config_discovery.ts"
import { ExtractInputKind, ExtractionConfig, extract } from "@xberg-io/xberg";

const config = ExtractionConfig.discover();
const input = {
  kind: "uri",
  uri: "document.pdf",
};

if (config) {
  console.log("Found configuration file");
  const output = await extract(input, config);
  console.log(output.results[0].content);
} else {
  console.log("No configuration file found, using defaults");
  const output = await extract(input);
  console.log(output.results[0].content);
}
```

The discovery method looks for `xberg.toml`, `xberg.yaml`, or `xberg.json` files starting in the current directory and searching parent directories up to the filesystem root.
