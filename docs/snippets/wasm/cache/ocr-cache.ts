import { extract, initWasm, TesseractWasmBackend } from "@xberg-io/xberg-wasm";

async function demonstrateOcrCaching() {
  await initWasm();

  const backend = new TesseractWasmBackend();
  await backend.initialize();

  console.log("Tesseract WASM backend loaded - models cached");

  const imageBytes = new Uint8Array(await fetch("page1.png").then((r) => r.arrayBuffer()));

  console.time("First OCR (with model load)");
  const _result1 = await extract(imageBytes, "image/png", {
    ocr: { backend: "tesseract-wasm", language: "eng" },
  });
  console.timeEnd("First OCR (with model load)");

  console.log("Model cached in memory");

  const imageBytes2 = new Uint8Array(await fetch("page2.png").then((r) => r.arrayBuffer()));

  console.time("Second OCR (model cached)");
  const _result2 = await extract(imageBytes2, "image/png", {
    ocr: { backend: "tesseract-wasm", language: "eng" },
  });
  console.timeEnd("Second OCR (model cached)");
}

demonstrateOcrCaching().catch(console.error);
