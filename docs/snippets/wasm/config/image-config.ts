import type { ExtractionConfig } from "@xberg-io/xberg-wasm";
import { extract, initWasm } from "@xberg-io/xberg-wasm";

async function extractImagesWithConfig() {
  await initWasm();

  const bytes = new Uint8Array(await fetch("document.pdf").then((r) => r.arrayBuffer()));

  const config: ExtractionConfig = {
    images: {
      extractImages: true,
      targetDpi: 300,
      maxDimension: 2048,
      preserveAspectRatio: true,
    },
  };

  const result = await extract({ kind: "bytes", bytes, mimeType: "application/pdf" }, config);

  if (result.images) {
    console.log(`Extracted ${result.images.length} images`);

    result.images.forEach((image) => {
      console.log(
        `Image: ${image.width}x${image.height}, Format: ${image.format}, DPI: ${image.description}`,
      );
    });
  }
}

extractImagesWithConfig().catch(console.error);
