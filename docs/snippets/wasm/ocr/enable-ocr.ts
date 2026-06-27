import { enableOcr, extract, initWasm } from "@xberg-io/xberg-wasm";

async function extractWithOcr() {
  await initWasm();

  try {
    await enableOcr();
    console.log("OCR enabled successfully");
  } catch (error) {
    console.error("Failed to enable OCR:", error);
    return;
  }

  const bytes = new Uint8Array(await fetch("scanned-page.png").then((r) => r.arrayBuffer()));

  const result = await extract(bytes, "image/png", {
    ocr: {
      backend: "tesseract-wasm",
      language: "eng",
    },
  });

  console.log("Extracted text:");
  console.log(result.content);
}

extractWithOcr().catch(console.error);
