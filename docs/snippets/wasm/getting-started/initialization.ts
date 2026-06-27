import { getVersion, getWasmCapabilities, initWasm, isInitialized } from "@xberg-io/xberg-wasm";

async function initializeXberg() {
  const caps = getWasmCapabilities();

  if (!caps.hasWasm) {
    console.error("WebAssembly not supported");
    return;
  }

  try {
    if (!isInitialized()) {
      await initWasm();
    }

    const version = getVersion();
    console.log(`Xberg ${version} initialized successfully`);
    console.log("Workers available:", caps.hasWorkers);
    console.log("SharedArrayBuffer available:", caps.hasSharedArrayBuffer);
  } catch (error) {
    console.error("Initialization failed:", error);
  }
}

initializeXberg();
