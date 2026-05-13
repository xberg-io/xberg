// Setup WASI polyfills before importing kreuzberg
import { createWasiPreview1 } from "jco";

// This will be executed before any tests
const wasiImports = createWasiPreview1({});
(global as any).__WASI__ = wasiImports;
