```typescript title="WASM"
import init, { extract } from "xberg-wasm";
import { LanguageDetectionConfig, ExtractionConfig } from "xberg-wasm";

await init();

const fileBuffer = new Uint8Array(/* your file bytes */);
const mimeType = "text/plain";

const config = new ExtractionConfig({
  language_detection: new LanguageDetectionConfig({
    enable_detection: true,
    target_languages: ["en", "de", "fr", "es", "it", "ja", "zh"],
    confidence_threshold: 0.5,
  }),
});

const result = await extract(fileBuffer, mimeType, config);

if (result.detected_languages && result.detected_languages.length > 0) {
  console.log("Document languages:", result.detected_languages.join(", "));

  // Process multi-language content
  result.detected_languages.forEach((lang) => {
    console.log(`Language detected: ${lang}`);
  });

  // Access metadata for language info
  if (result.metadata && result.metadata.language) {
    console.log(`Primary metadata language: ${result.metadata.language}`);
  }
} else {
  console.log("No languages detected");
}
```
