// Hand-authored ESM; not compiled from TypeScript. Type declarations live in worker-bridge.d.ts.
import { isNode } from "../runtime.js";
let workerHandle = null;
const pendingRequests = /* @__PURE__ */ new Map();
let nextRequestId = 0;
let workerReady = false;
let readyResolve = null;
let readyReject = null;
let useFallback = false;
let fallbackFn = null;
async function cleanupWorker() {
  if (workerHandle) {
    await workerHandle.terminate();
    workerHandle = null;
  }
  workerReady = false;
}
function handleWorkerMessage(msg) {
  switch (msg["type"]) {
    case "ready":
      workerReady = true;
      readyResolve?.();
      readyResolve = null;
      readyReject = null;
      break;
    case "init-error":
      readyReject?.(new Error(msg["error"]));
      readyResolve = null;
      readyReject = null;
      break;
    case "result": {
      const id = msg["id"];
      const pending = pendingRequests.get(id);
      if (pending) {
        pendingRequests.delete(id);
        pending.resolve(msg["text"]);
      }
      break;
    }
    case "error": {
      const id = msg["id"];
      const pending = pendingRequests.get(id);
      if (pending) {
        pendingRequests.delete(id);
        pending.reject(new Error(msg["error"]));
      }
      break;
    }
  }
}
async function createOcrWorker(wasmGluePath, wasmBinary, directFallback) {
  fallbackFn = directFallback;
  if (workerHandle) return;
  const readyPromise = new Promise((resolve, reject) => {
    readyResolve = resolve;
    readyReject = reject;
  });
  try {
    if (isNode()) {
      await createNodeWorker(wasmGluePath, wasmBinary);
    } else if (typeof Worker !== "undefined") {
      await createBrowserWorker(wasmGluePath, wasmBinary);
    } else {
      useFallback = true;
      return;
    }
    const timeoutMs = 3e4;
    const timeout = new Promise((_, reject) => {
      setTimeout(() => reject(new Error("OCR worker initialization timed out")), timeoutMs);
    });
    await Promise.race([readyPromise, timeout]);
  } catch (e) {
    await cleanupWorker();
    console.warn(`[kreuzberg/wasm] OCR Worker creation failed, falling back to synchronous OCR: ${e instanceof Error ? e.message : String(e)}`);
    useFallback = true;
  }
}
async function createNodeWorker(wasmGluePath, wasmBinary) {
  const { Worker: Worker2 } = await import(
    /* @vite-ignore */
    "node:worker_threads"
  );
  const nodePath = await import(
    /* @vite-ignore */
    "node:path"
  );
  const nodeUrl = await import(
    /* @vite-ignore */
    "node:url"
  );
  const __dirname = nodePath.dirname(nodeUrl.fileURLToPath(import.meta.url));
  const workerPath = nodePath.join(__dirname, "ocr-worker.js");
  const worker = new Worker2(workerPath, {
    workerData: { wasmGluePath, wasmBinary }
  });
  worker.on("message", (msg) => handleWorkerMessage(msg));
  worker.on("error", (err) => {
    for (const pending of pendingRequests.values()) {
      pending.reject(err);
    }
    pendingRequests.clear();
    readyReject?.(err);
  });
  workerHandle = {
    postMessage: (data) => worker.postMessage(data),
    terminate: () => {
      worker.terminate();
      return void 0;
    }
  };
}
async function createBrowserWorker(wasmGluePath, wasmBinary) {
  const workerScriptUrl = new URL("./ocr-worker.js", import.meta.url);
  let worker;
  try {
    const response = await fetch(workerScriptUrl.href);
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    const scriptText = await response.text();
    const blob = new Blob([scriptText], { type: "application/javascript" });
    const blobUrl = URL.createObjectURL(blob);
    worker = new Worker(blobUrl, { type: "module" });
    URL.revokeObjectURL(blobUrl);
  } catch (e) {
    const blob = new Blob([`import "${workerScriptUrl.href}";`], { type: "application/javascript" });
    const blobUrl = URL.createObjectURL(blob);
    worker = new Worker(blobUrl, { type: "module" });
    URL.revokeObjectURL(blobUrl);
  }
  worker.onmessage = (e) => handleWorkerMessage(e.data);
  worker.onerror = (e) => {
    const err = new Error(e.message);
    for (const pending of pendingRequests.values()) {
      pending.reject(err);
    }
    pendingRequests.clear();
    readyReject?.(err);
  };
  workerHandle = {
    postMessage: (data) => worker.postMessage(data),
    terminate: () => {
      worker.terminate();
      return void 0;
    }
  };
  worker.postMessage({
    type: "init",
    wasmGluePath,
    wasmBinary
  });
}
async function runOcrInWorker(imageData, tessdata, language) {
  if (useFallback || !workerHandle || !workerReady) {
    if (fallbackFn) {
      try {
        const text = fallbackFn(imageData, tessdata, language);
        return Promise.resolve(text);
      } catch (e) {
        return Promise.reject(e instanceof Error ? e : new Error(String(e)));
      }
    }
    return Promise.reject(new Error("OCR worker not initialized and no fallback available"));
  }
  const id = nextRequestId++;
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => {
      if (pendingRequests.has(id)) {
        pendingRequests.delete(id);
        const err = new Error(
          "OCR timed out after 20s \u2014 Tesseract hung on this image. Try a different image or a text-based file format."
        );
        for (const pending of pendingRequests.values()) {
          pending.reject(err);
        }
        pendingRequests.clear();
        terminateOcrWorker().catch(() => void 0);
        reject(err);
      }
    }, 2e4);
    pendingRequests.set(id, {
      resolve: (text) => {
        clearTimeout(timer);
        resolve(text);
      },
      reject: (errVal) => {
        clearTimeout(timer);
        reject(errVal);
      }
    });
    const transfer = [];
    if (imageData instanceof Uint8Array || imageData instanceof ArrayBuffer) {
      transfer.push(imageData instanceof Uint8Array ? imageData.buffer : imageData);
    }
    if (tessdata instanceof Uint8Array || tessdata instanceof ArrayBuffer) {
      transfer.push(tessdata instanceof Uint8Array ? tessdata.buffer : tessdata);
    }

    workerHandle?.postMessage({
      type: "ocr",
      id,
      imageData,
      tessdata,
      language
    }, transfer);
  });
}
function isUsingWorker() {
  return workerHandle !== null && workerReady && !useFallback;
}
async function terminateOcrWorker() {
  if (workerHandle) {
    await workerHandle.terminate();
    workerHandle = null;
  }
  workerReady = false;
  useFallback = false;
  fallbackFn = null;
  for (const pending of pendingRequests.values()) {
    pending.reject(new Error("OCR worker terminated"));
  }
  pendingRequests.clear();
}
export {
  createOcrWorker,
  isUsingWorker,
  runOcrInWorker,
  terminateOcrWorker
};
