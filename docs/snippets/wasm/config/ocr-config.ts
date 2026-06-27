import type { ExtractionConfig } from "@xberg-io/xberg-wasm";
import { extract, initWasm } from "@xberg-io/xberg-wasm";

async function extractWithOcr() {
  await initWasm();

  const bytes = new Uint8Array(await fetch("scanned.pdf").then((r) => r.arrayBuffer()));

  const config: ExtractionConfig = {
    ocr: {
      backend: "tesseract-wasm",
      language: "eng",
    },
  };

  const result = await extract({ kind: "bytes", bytes, mimeType: "application/pdf" }, config);

  console.log("Extracted text from scanned document:");
  console.log(result.content);

  if (result.detectedLanguages) {
    console.log("Detected languages:", result.detectedLanguages);
  }
}

extractWithOcr().catch(console.error);
