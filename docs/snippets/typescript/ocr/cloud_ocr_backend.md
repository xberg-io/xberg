```typescript title="TypeScript"
import { registerOcrBackend, extractSync } from "@xberg-io/xberg";

const supportedLangs = ["eng", "deu", "fra"];

const cloudBackend = {
  name: () => "cloud-ocr",
  version: () => "1.0.0",
  initialize: () => {},
  shutdown: () => {},
  process_image: async (imageBytes: Uint8Array, config: { language?: string }) => {
    // Call your cloud OCR API with imageBytes and config.language.
    return { content: "Extracted text", mime_type: "text/plain" };
  },
  supports_language: (lang: string) => supportedLangs.includes(lang),
  backend_type: () => "Custom",
  supported_languages: () => supportedLangs,
};

registerOcrBackend(cloudBackend);

const result = extractSync("scanned.pdf", {
  ocr: {
    backend: "cloud-ocr",
    language: "eng",
  },
});
console.log(result.content);
```
