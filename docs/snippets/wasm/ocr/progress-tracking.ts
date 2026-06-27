import { extract, initWasm, registerOcrBackend, TesseractWasmBackend } from "@xberg-io/xberg-wasm";

async function extractWithProgressTracking() {
  await initWasm();

  const backend = new TesseractWasmBackend();

  backend.setProgressCallback((progress: number) => {
    const progressBar = document.getElementById("progress");
    if (progressBar) {
      progressBar.style.width = `${progress}%`;
      progressBar.textContent = `${progress}%`;
    }
  });

  await backend.initialize();
  registerOcrBackend(backend);

  const bytes = new Uint8Array(await fetch("document.png").then((r) => r.arrayBuffer()));

  const result = await extract(
    { kind: "bytes", bytes, mimeType: "image/png" },
    {
      ocr: {
        backend: "tesseract-wasm",
        language: "eng",
      },
    },
  );

  console.log("OCR complete");
  console.log(result.content);
}

extractWithProgressTracking().catch(console.error);
