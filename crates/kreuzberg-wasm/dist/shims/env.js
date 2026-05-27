// Browser stubs for C runtime imports required by the WASM binary.
// These are only called by tesseract's C code for operations that are
// unreachable in a browser extraction path (shell exec, temp files).
export function system() { return -1; }
export function mkstemp() { return -1; }
