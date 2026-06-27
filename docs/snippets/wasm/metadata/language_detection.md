```typescript title="WASM"
import init, { extract } from "xberg-wasm";
import { LanguageDetectionConfig, ExtractionConfig } from "xberg-wasm";

await init();

const fileBuffer = new Uint8Array(/* your file bytes */);
const mimeType = "text/plain";

const config = new ExtractionConfig({
  language_detection: new LanguageDetectionConfig({
    enable_detection: true,
    target_languages: ["en", "de", "fr"],
  }),
});

const result = await extract(fileBuffer, mimeType, config);

if (result.detected_languages) {
  console.log("Detected languages:", result.detected_languages);

  for (const language of result.detected_languages) {
    console.log(`Language: ${language}`);
  }
}
```
