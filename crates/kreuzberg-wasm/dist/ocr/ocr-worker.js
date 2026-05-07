// Hand-authored ESM; not compiled from TypeScript. Type declarations live in ocr-worker.d.ts.
let wasm = null;
let postFn = () => {
  throw new Error("Worker message handler not initialized");
};
async function initWasm(wasmGluePath, wasmBinary) {
  const glue = await import(
    /* @vite-ignore */
    wasmGluePath
  );
  if (typeof glue.default === "function") {
    if (wasmBinary) {
      await glue.default(wasmBinary);
    } else {
      await glue.default();
    }
  }
  wasm = glue;
  postFn({ type: "ready" });
}
function onMessage(msg) {
  switch (msg["type"]) {
    case "init":
      initWasm(msg["wasmGluePath"], msg["wasmBinary"]).catch((e) => {
        postFn({ type: "init-error", error: e instanceof Error ? e.message : String(e) });
      });
      break;
    case "ocr": {
      const id = msg["id"];
      if (!wasm) {
        postFn({ type: "error", id, error: "WASM OCR not initialized" });
        return;
      }
      try {
        let imageData = msg["imageData"];
        
        // Decode base64 in the worker if it was passed as a string
        if (typeof imageData === "string") {
          const binaryString = atob(imageData);
          const bytes = new Uint8Array(binaryString.length);
          for (let i = 0; i < binaryString.length; i++) {
            bytes[i] = binaryString.charCodeAt(i);
          }
          imageData = bytes;
        }

        const text = wasm.ocrRecognize(
          imageData,
          msg["tessdata"],
          msg["language"]
        );
        postFn({ type: "result", id, text });
      } catch (e) {
        postFn({ type: "error", id, error: e instanceof Error ? e.message : String(e) });
      }
      break;
    }
  }
}
async function bootstrap() {
  const isNodeEnv = typeof process !== "undefined" && !!process.versions?.node && typeof globalThis.Deno === "undefined";
  if (isNodeEnv) {
    const { parentPort, workerData } = await import(
      /* @vite-ignore */
      "node:worker_threads"
    );
    if (!parentPort) throw new Error("ocr-worker must be run as a worker thread");
    postFn = (data) => parentPort.postMessage(data);
    parentPort.on("message", (msg) => onMessage(msg));
    const wd = workerData;
    if (wd?.wasmGluePath) {
      await initWasm(wd.wasmGluePath, wd.wasmBinary);
    }
  } else {
    const self_ = globalThis;
    postFn = (data) => self_.postMessage(data);
    self_.onmessage = (e) => onMessage(e.data);
  }
}
bootstrap().catch((e) => {
  try {
    if (typeof process !== "undefined" && process.stderr) {
      process.stderr.write(`[ocr-worker] bootstrap failed: ${e}
`);
      process.exit(1);
    }
  } catch {
  }
});
